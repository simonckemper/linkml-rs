//! YAML loader and dumper v2 with file system adapter support
//!
//! This module provides YAML loading/dumping that uses `FileSystemOperations`
//! instead of direct file system access.

use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::traits::{DataInstance, DumperError, LoaderError};
use super::traits_v2::{DataDumperV2, DataLoaderV2, DumperResult, LoaderResult};
use crate::file_system_adapter::FileSystemOperations;

/// `YAML` loader v2 with file system adapter support
#[derive(Default)]
pub struct YamlLoaderV2 {
    /// Options for loading
    validate: bool,
    strict: bool,
}

impl YamlLoaderV2 {
    /// Create a new `YAML` loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            validate: true,
            strict: false,
        }
    }

    /// Set validation enabled
    #[must_use]
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Set strict mode
    #[must_use]
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Infer class name from object structure
    fn infer_class_from_object(&self, obj: &serde_json::Map<String, Value>) -> String {
        // Check for explicit type field
        if let Some(type_val) = obj
            .get("@type")
            .or_else(|| obj.get("type"))
            .or_else(|| obj.get("_type"))
            .or_else(|| obj.get("class"))
            .or_else(|| obj.get("class_name"))
            && let Some(type_str) = type_val.as_str()
        {
            return type_str.to_string();
        }

        // Infer from field patterns
        let fields: Vec<_> = obj.keys().map(std::string::String::as_str).collect();

        // Common patterns for different types
        if fields.contains(&"name") && fields.contains(&"slots") {
            return "ClassDefinition".to_string();
        }

        if fields.contains(&"range") && fields.contains(&"required") {
            return "SlotDefinition".to_string();
        }

        if fields.contains(&"permissible_values") {
            return "EnumDefinition".to_string();
        }

        if fields.contains(&"person_id")
            || fields.contains(&"first_name")
            || fields.contains(&"last_name")
        {
            return "Person".to_string();
        }

        if fields.contains(&"organization_id") || fields.contains(&"organization_name") {
            return "Organization".to_string();
        }

        // Default fallback based on presence of common fields
        if fields.contains(&"id") && fields.contains(&"name") {
            return "Entity".to_string();
        }

        "DataObject".to_string()
    }
}

#[async_trait]
impl DataLoaderV2 for YamlLoaderV2 {
    async fn load_file<F: FileSystemOperations>(
        &mut self,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = fs
            .read_to_string(path)
            .await
            .map_err(|e| LoaderError::Io(std::io::Error::other(e.to_string())))?;

        self.load_str(&content, schema).await
    }

    async fn load_str(
        &mut self,
        content: &str,
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| LoaderError::Parse(e.to_string()))?;

        // Convert YAML value to JSON value
        let json_str =
            serde_json::to_string(&yaml_value).map_err(|e| LoaderError::Parse(e.to_string()))?;
        let json_value: Value =
            serde_json::from_str(&json_str).map_err(|e| LoaderError::Parse(e.to_string()))?;

        // Handle both single objects and arrays
        let instances = match json_value {
            Value::Array(items) => {
                items
                    .into_iter()
                    .filter_map(|item| {
                        if let Value::Object(obj) = item {
                            // Infer class name from structure
                            let class_name = self.infer_class_from_object(&obj);

                            // Extract ID if present
                            let id = obj
                                .get("id")
                                .or_else(|| obj.get("@id"))
                                .and_then(|v| v.as_str())
                                .map(std::string::ToString::to_string);

                            Some(DataInstance {
                                class_name,
                                data: obj.into_iter().collect(),
                                id,
                                metadata: HashMap::new(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Value::Object(obj) => {
                // Infer class name from structure
                let class_name = self.infer_class_from_object(&obj);

                // Extract ID if present
                let id = obj
                    .get("id")
                    .or_else(|| obj.get("@id"))
                    .and_then(|v| v.as_str())
                    .map(std::string::ToString::to_string);

                vec![DataInstance {
                    class_name,
                    data: obj.into_iter().collect(),
                    id,
                    metadata: HashMap::new(),
                }]
            }
            _ => {
                return Err(LoaderError::Parse(
                    "Expected object or array at root".to_string(),
                ));
            }
        };

        Ok(instances)
    }

    fn name(&self) -> &'static str {
        "YAMLLoaderV2"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

/// `YAML` dumper v2 with file system adapter support
#[derive(Default)]
pub struct YamlDumperV2 {
    /// Pretty print output
    pretty: bool,
}

impl YamlDumperV2 {
    /// Create a new `YAML` dumper
    #[must_use]
    pub fn new() -> Self {
        Self { pretty: true }
    }

    /// Set pretty printing
    #[must_use]
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }
}

#[async_trait]
impl DataDumperV2 for YamlDumperV2 {
    async fn dump_file<F: FileSystemOperations>(
        &mut self,
        instances: Vec<DataInstance>,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> DumperResult<()> {
        let content = self.dump_str(instances, schema).await?;

        fs.write(path, &content)
            .await
            .map_err(|e| DumperError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    async fn dump_str(
        &mut self,
        instances: Vec<DataInstance>,
        _schema: &SchemaDefinition,
    ) -> DumperResult<String> {
        // Convert instances to appropriate format
        let json_output = if instances.len() == 1 {
            // Single instance - output as object
            let instance = instances.into_iter().next().ok_or_else(|| {
                anyhow::anyhow!("should have at least one instance after length check")
            })?;
            let mut obj = instance.data;
            obj.insert(
                "@type".to_string(),
                serde_json::Value::String(instance.class_name),
            );
            serde_json::Value::Object(serde_json::Map::from_iter(obj))
        } else {
            // Multiple instances - output as array
            let json_instances: Vec<serde_json::Value> = instances
                .into_iter()
                .map(|instance| {
                    let mut obj = instance.data;
                    obj.insert(
                        "@type".to_string(),
                        serde_json::Value::String(instance.class_name),
                    );
                    serde_json::Value::Object(serde_json::Map::from_iter(obj))
                })
                .collect();
            serde_json::Value::Array(json_instances)
        };

        serde_yaml::to_string(&json_output).map_err(|e| DumperError::Serialization(e.to_string()))
    }

    fn name(&self) -> &'static str {
        "YAMLDumperV2"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["yaml", "yml"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system_adapter::TokioFileSystemAdapter;
    use linkml_core::types::SchemaDefinition;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_yaml_loader_v2() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
            temp_dir.path().to_path_buf(),
        ));

        let yaml_content = r"
- name: Alice
  age: 30
- name: Bob
  age: 25
";

        let file_path = Path::new("data.yaml");
        fs.write(file_path, yaml_content).await?;

        let mut loader = YamlLoaderV2::new();
        let schema = SchemaDefinition::default();
        let instances = loader.load_file(&file_path, &schema, fs).await?;

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].data["name"], "Alice");
        assert_eq!(instances[1].data["name"], "Bob");
        Ok(())
    }

    #[tokio::test]
    async fn test_yaml_dumper_v2() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
            temp_dir.path().to_path_buf(),
        ));

        let instances = vec![
            DataInstance {
                data: {
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), serde_json::json!("Alice"));
                    map.insert("age".to_string(), serde_json::json!(30));
                    map
                },
                class_name: "Person".to_string(),
                id: None,
                metadata: HashMap::new(),
            },
            DataInstance {
                data: {
                    let mut map = HashMap::new();
                    map.insert("name".to_string(), serde_json::json!("Bob"));
                    map.insert("age".to_string(), serde_json::json!(25));
                    map
                },
                class_name: "Person".to_string(),
                id: None,
                metadata: HashMap::new(),
            },
        ];

        let file_path = Path::new("output.yaml");
        let mut dumper = YamlDumperV2::new();
        let schema = SchemaDefinition::default();

        dumper
            .dump_file(instances, file_path, &schema, fs.clone())
            .await?;

        let content = fs.read_to_string(file_path).await?;
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));
        Ok(())
    }
}
