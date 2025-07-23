//! JSON loader and dumper for LinkML
//!
//! This module provides functionality to load and dump LinkML data in JSON format.

use super::traits::{DataLoader, DataDumper, LoaderError, LoaderResult, DumperError, DumperResult, DataInstance};
use linkml_core::prelude::*;
use async_trait::async_trait;
use serde_json::{Value, Map};

/// JSON loader for LinkML data
pub struct JsonLoader {
    /// Input file path
    file_path: Option<String>,
}

impl JsonLoader {
    /// Create a new JSON loader
    pub fn new() -> Self {
        Self {
            file_path: None,
        }
    }
    
    /// Set the input file path
    pub fn with_file(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        self
    }
}

impl Default for JsonLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for JsonLoader {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let content = if let Some(path) = &self.file_path {
            std::fs::read_to_string(path)
                .map_err(|e| LoaderError::Io(e))?
        } else {
            return Err(LoaderError::Configuration("No input file specified".to_string()));
        };
        
        let json: Value = serde_json::from_str(&content)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        
        let instances = match json {
            Value::Array(arr) => {
                // Array of instances
                let mut instances = Vec::new();
                for item in arr {
                    if let Value::Object(obj) = item {
                        let instance = self.object_to_instance(obj, schema)?;
                        instances.push(instance);
                    }
                }
                instances
            }
            Value::Object(obj) => {
                // Single instance
                vec![self.object_to_instance(obj, schema)?]
            }
            _ => {
                return Err(LoaderError::InvalidFormat(
                    "JSON must be an object or array of objects".to_string()
                ));
            }
        };
        
        Ok(instances)
    }
}

impl JsonLoader {
    /// Convert JSON object to DataInstance
    fn object_to_instance(&self, obj: Map<String, Value>, schema: &SchemaDefinition) -> LoaderResult<DataInstance> {
        // Try to determine class from @type field or structure
        let class_name = if let Some(Value::String(type_val)) = obj.get("@type") {
            type_val.clone()
        } else {
            // Try to infer from structure
            self.infer_class(&obj, schema)?
        };
        
        Ok(DataInstance {
            class_name,
            data: obj,
        })
    }
    
    /// Infer class from object structure
    fn infer_class(&self, obj: &Map<String, Value>, schema: &SchemaDefinition) -> LoaderResult<String> {
        let obj_keys: std::collections::HashSet<_> = obj.keys().cloned().collect();
        
        // Find best matching class
        let mut best_match = None;
        let mut best_score = 0;
        
        for (class_name, class_def) in &schema.classes {
            let class_slots: std::collections::HashSet<_> = class_def.slots.iter().cloned().collect();
            let intersection = obj_keys.intersection(&class_slots).count();
            
            if intersection > best_score {
                best_score = intersection;
                best_match = Some(class_name.clone());
            }
        }
        
        best_match.ok_or_else(|| LoaderError::ValidationFailed(
            "Could not infer class from object structure".to_string()
        ))
    }
}

/// JSON dumper for LinkML data
pub struct JsonDumper {
    /// Pretty print output
    pretty: bool,
}

impl JsonDumper {
    /// Create a new JSON dumper
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }
}

impl Default for JsonDumper {
    fn default() -> Self {
        Self::new(true)
    }
}

#[async_trait]
impl DataDumper for JsonDumper {
    async fn dump(&mut self, instances: &[DataInstance], _schema: &SchemaDefinition) -> DumperResult<Vec<u8>> {
        let json_instances: Vec<Value> = instances.iter()
            .map(|instance| {
                let mut obj = instance.data.clone();
                obj.insert("@type".to_string(), Value::String(instance.class_name.clone()));
                Value::Object(obj)
            })
            .collect();
        
        let json_str = if self.pretty {
            serde_json::to_string_pretty(&json_instances)
        } else {
            serde_json::to_string(&json_instances)
        }.map_err(|e| DumperError::Serialization(e.to_string()))?;
        
        Ok(json_str.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_json_loader() {
        let json_content = r#"
        {
            "@type": "Person",
            "name": "John Doe",
            "age": 30
        }
        "#;
        
        // Create temp file
        let temp_file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), json_content).unwrap();
        
        let mut schema = SchemaDefinition::default();
        let mut class = ClassDefinition::default();
        class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), class);
        
        let mut loader = JsonLoader::new().with_file(temp_file.path().to_str().unwrap());
        let instances = loader.load(&schema).await.unwrap();
        
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].class_name, "Person");
        assert_eq!(instances[0].data.get("name"), Some(&Value::String("John Doe".to_string())));
    }
    
    #[tokio::test]
    async fn test_json_dumper() {
        let instances = vec![
            DataInstance {
                class_name: "Person".to_string(),
                data: serde_json::from_str(r#"{"name": "Alice", "age": 25}"#).unwrap(),
            },
            DataInstance {
                class_name: "Person".to_string(),
                data: serde_json::from_str(r#"{"name": "Bob", "age": 30}"#).unwrap(),
            },
        ];
        
        let schema = SchemaDefinition::default();
        let mut dumper = JsonDumper::new(false);
        let result = dumper.dump(&instances, &schema).await.unwrap();
        
        let json_str = String::from_utf8(result).unwrap();
        let parsed: Vec<Value> = serde_json::from_str(&json_str).unwrap();
        
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["@type"], "Person");
        assert_eq!(parsed[0]["name"], "Alice");
    }
}