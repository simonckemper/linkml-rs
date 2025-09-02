//! JSON loader and dumper v2 with file system adapter support
//!
//! This module provides JSON loading/dumping that uses FileSystemOperations
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

/// JSON loader v2 with file system adapter support
#[derive(Default)]
pub struct JsonLoaderV2 {
    /// Options for loading
    validate: bool,
    strict: bool,
}

impl JsonLoaderV2 {
    /// Create a new JSON loader
    pub fn new() -> Self {
        Self {
            validate: true,
            strict: false,
        }
    }

    /// Set validation enabled
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Set strict mode
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

#[async_trait]
impl DataLoaderV2 for JsonLoaderV2 {
    async fn load_file<F: FileSystemOperations>(
        &mut self,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = fs.read_to_string(path).await.map_err(|e| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        self.load_str(&content, schema).await
    }

    async fn load_str(
        &mut self,
        content: &str,
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let json_value: Value =
            serde_json::from_str(content).map_err(|e| LoaderError::Parse(e.to_string()))?;

        // Handle both single objects and arrays
        let instances = match json_value {
            Value::Array(items) => {
                items
                    .into_iter()
                    .filter_map(|item| {
                        if let Value::Object(obj) = item {
                            Some(DataInstance {
                                class_name: "UnknownClass".to_string(), // TODO: infer from structure
                                data: obj.into_iter().collect(),
                                id: None,
                                metadata: HashMap::new(),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Value::Object(obj) => {
                vec![DataInstance {
                    class_name: "UnknownClass".to_string(), // TODO: infer from structure
                    data: obj.into_iter().collect(),
                    id: None,
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
        "JSONLoaderV2"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        vec!["json", "jsonl"]
    }
}

/// JSON dumper v2 with file system adapter support
#[derive(Default)]
pub struct JsonDumperV2 {
    /// Pretty print output
    pretty: bool,
    /// Use JSON Lines format
    jsonl: bool,
}

impl JsonDumperV2 {
    /// Create a new JSON dumper
    pub fn new() -> Self {
        Self {
            pretty: true,
            jsonl: false,
        }
    }

    /// Set pretty printing
    pub fn with_pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Set JSON Lines format
    pub fn with_jsonl(mut self, jsonl: bool) -> Self {
        self.jsonl = jsonl;
        self
    }
}

#[async_trait]
impl DataDumperV2 for JsonDumperV2 {
    async fn dump_file<F: FileSystemOperations>(
        &mut self,
        instances: Vec<DataInstance>,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> DumperResult<()> {
        let content = self.dump_str(instances, schema).await?;

        fs.write(path, &content).await.map_err(|e| {
            DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        Ok(())
    }

    async fn dump_str(
        &mut self,
        instances: Vec<DataInstance>,
        _schema: &SchemaDefinition,
    ) -> DumperResult<String> {
        if self.jsonl {
            // JSON Lines format - one object per line
            let lines: Vec<String> = instances
                .into_iter()
                .map(|instance| {
                    let mut obj = instance.data;
                    obj.insert(
                        "@type".to_string(),
                        serde_json::Value::String(instance.class_name),
                    );
                    let json_obj = serde_json::Value::Object(serde_json::Map::from_iter(obj));
                    serde_json::to_string(&json_obj)
                })
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(|e| DumperError::Serialization(e.to_string()))?;

            Ok(lines.join("\n"))
        } else {
            // Regular JSON format
            let json_output = if instances.len() == 1 {
                // Single instance - output as object
                let instance = instances
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("should have at least one instance after length check"))?;
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

            if self.pretty {
                serde_json::to_string_pretty(&json_output)
                    .map_err(|e| DumperError::Serialization(e.to_string()))
            } else {
                serde_json::to_string(&json_output)
                    .map_err(|e| DumperError::Serialization(e.to_string()))
            }
        }
    }

    fn name(&self) -> &'static str {
        "JSONDumperV2"
    }

    fn supported_extensions(&self) -> Vec<&'static str> {
        if self.jsonl {
            vec!["jsonl", "ndjson"]
        } else {
            vec!["json"]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system_adapter::TokioFileSystemAdapter;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_json_loader_v2() {
        let temp_dir = TempDir::new().map_err(|e| anyhow::anyhow!("should create temporary directory: {}", e))?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
            temp_dir.path().to_path_buf(),
        ));

        let json_content = r#"[
  {"name": "Alice", "age": 30},
  {"name": "Bob", "age": 25}
]"#;

        let file_path = Path::new("data.json");
        fs.write(file_path, json_content)
            .await
            .map_err(|e| anyhow::anyhow!("should write JSON file: {}", e))?;

        let mut loader = JsonLoaderV2::new();
        let schema = SchemaDefinition::default();
        let instances = loader
            .load_file(&file_path, &schema, fs)
            .await
            .map_err(|e| anyhow::anyhow!("should load JSON file: {}", e))?;

        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].data["name"], "Alice");
        assert_eq!(instances[1].data["name"], "Bob");
    }

    #[tokio::test]
    async fn test_json_dumper_v2() {
        let temp_dir = TempDir::new().map_err(|e| anyhow::anyhow!("should create temporary directory: {}", e))?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
            temp_dir.path().to_path_buf(),
        ));

        let instances = vec![
            DataInstance {
                class_name: "Person".to_string(),
                data: HashMap::from([
                    ("name".to_string(), serde_json::json!("Alice")),
                    ("age".to_string(), serde_json::json!(30)),
                ]),
                id: Some("person1".to_string()),
                metadata: HashMap::new(),
            },
            DataInstance {
                class_name: "Person".to_string(),
                data: HashMap::from([
                    ("name".to_string(), serde_json::json!("Bob")),
                    ("age".to_string(), serde_json::json!(25)),
                ]),
                id: Some("person2".to_string()),
                metadata: HashMap::new(),
            },
        ];

        // Test regular JSON
        let file_path = Path::new("output.json");
        let mut dumper = JsonDumperV2::new();
        let schema = SchemaDefinition::default();

        dumper
            .dump_file(instances.clone(), file_path, &schema, fs.clone())
            .await
            .map_err(|e| anyhow::anyhow!("should dump instances to JSON: {}", e))?;

        let content = fs
            .read_to_string(file_path)
            .await
            .map_err(|e| anyhow::anyhow!("should read JSON file: {}", e))?;
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));

        // Test JSON Lines
        let jsonl_path = Path::new("output.jsonl");
        let mut jsonl_dumper = JsonDumperV2::new().with_jsonl(true);

        jsonl_dumper
            .dump_file(instances, jsonl_path, &schema, fs.clone())
            .await
            .map_err(|e| anyhow::anyhow!("should dump instances to JSONL: {}", e))?;

        let jsonl_content = fs
            .read_to_string(jsonl_path)
            .await
            .map_err(|e| anyhow::anyhow!("should read JSONL file: {}", e))?;
        let lines: Vec<&str> = jsonl_content.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);
    }
}
