//! YAML loader and dumper v2 with file system adapter support
//!
//! This module provides YAML loading/dumping that uses FileSystemOperations
//! instead of direct file system access.

use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use crate::file_system_adapter::FileSystemOperations;
use super::traits::{DataInstance, LoaderError, DumperError};
use super::traits_v2::{DataLoaderV2, DataDumperV2, LoaderResult, DumperResult};

/// YAML loader v2 with file system adapter support
#[derive(Default)]
pub struct YamlLoaderV2 {
    /// Options for loading
    validate: bool,
    strict: bool,
}

impl YamlLoaderV2 {
    /// Create a new YAML loader
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
impl DataLoaderV2 for YamlLoaderV2 {
    async fn load_file<F: FileSystemOperations>(
        &mut self,
        path: &Path,
        _schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = fs.read_to_string(path).await
            .map_err(|e| LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))?;
        
        self.load_str(&content, _schema).await
    }
    
    async fn load_str(
        &mut self,
        content: &str,
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(content)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        
        // Convert YAML value to JSON value
        let json_str = serde_json::to_string(&yaml_value)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        let json_value: Value = serde_json::from_str(&json_str)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        
        // Handle both single objects and arrays
        let instances = match json_value {
            Value::Array(items) => {
                items.into_iter()
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
                    "Expected object or array at root".to_string()
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

/// YAML dumper v2 with file system adapter support
#[derive(Default)]
pub struct YamlDumperV2 {
    /// Pretty print output
    pretty: bool,
}

impl YamlDumperV2 {
    /// Create a new YAML dumper
    pub fn new() -> Self {
        Self {
            pretty: true,
        }
    }
    
    /// Set pretty printing
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
        
        fs.write(path, &content).await
            .map_err(|e| DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string()
            )))?;
        
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
            let instance = instances.into_iter().next().expect("should have at least one instance after length check");
            let mut obj = instance.data;
            obj.insert("@type".to_string(), serde_json::Value::String(instance.class_name));
            serde_json::Value::Object(serde_json::Map::from_iter(obj))
        } else {
            // Multiple instances - output as array
            let json_instances: Vec<serde_json::Value> = instances.into_iter().map(|instance| {
                let mut obj = instance.data;
                obj.insert("@type".to_string(), serde_json::Value::String(instance.class_name));
                serde_json::Value::Object(serde_json::Map::from_iter(obj))
            }).collect();
            serde_json::Value::Array(json_instances)
        };
        
        serde_yaml::to_string(&json_output)
            .map_err(|e| DumperError::Serialization(e.to_string()))
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
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_yaml_loader_v2() {
        let temp_dir = TempDir::new().expect("should create temporary directory");
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(temp_dir.path().to_path_buf()));
        
        let yaml_content = r#"
- name: Alice
  age: 30
- name: Bob
  age: 25
"#;
        
        let file_path = temp_dir.path().join("data.yaml");
        fs.write(&file_path, yaml_content).await.expect("should write YAML file");
        
        let mut loader = YamlLoaderV2::new();
        let schema = SchemaDefinition::default();
        let instances = loader.load_file(&file_path, &schema, fs).await.expect("should load YAML file");
        
        assert_eq!(instances.len(), 2);
        assert_eq!(instances[0].data["name"], "Alice");
        assert_eq!(instances[1].data["name"], "Bob");
    }
    
    #[tokio::test]
    async fn test_yaml_dumper_v2() {
        let temp_dir = TempDir::new().expect("should create temporary directory");
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(temp_dir.path().to_path_buf()));
        
        let instances = vec![
            DataInstance {
                data: serde_json::json!({
                    "name": "Alice",
                    "age": 30
                }),
                class_name: None,
                metadata: HashMap::new(),
            },
            DataInstance {
                data: serde_json::json!({
                    "name": "Bob",
                    "age": 25
                }),
                class_name: None,
                metadata: HashMap::new(),
            },
        ];
        
        let file_path = temp_dir.path().join("output.yaml");
        let mut dumper = YamlDumperV2::new();
        let schema = SchemaDefinition::default();
        
        dumper.dump_file(instances, &file_path, &schema, fs.clone()).await.expect("should dump instances to YAML");
        
        let content = fs.read_to_string(&file_path).await.expect("should read YAML file");
        assert!(content.contains("Alice"));
        assert!(content.contains("Bob"));
    }
}