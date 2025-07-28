//! YAML loader and dumper for LinkML
//!
//! This module provides functionality to load and dump LinkML data in YAML format.

use super::traits::{DataLoader, DataDumper, LoaderError, LoaderResult, DumperError, DumperResult, DataInstance};
use linkml_core::prelude::*;
use async_trait::async_trait;
use serde_json::{Value, Map};
use serde_yaml;

/// YAML loader for LinkML data
pub struct YamlLoader {
    /// Input file path
    file_path: Option<String>,
}

impl YamlLoader {
    /// Create a new YAML loader
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

impl Default for YamlLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for YamlLoader {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let content = if let Some(path) = &self.file_path {
            std::fs::read_to_string(path)
                .map_err(|e| LoaderError::Io(e))?
        } else {
            return Err(LoaderError::Configuration("No input file specified".to_string()));
        };
        
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        
        // Convert YAML to JSON for easier processing
        let json_str = serde_json::to_string(&yaml_value)
            .map_err(|e| LoaderError::Parse(e.to_string()))?;
        let json: Value = serde_json::from_str(&json_str)
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
                    "YAML must be a mapping or sequence of mappings".to_string()
                ));
            }
        };
        
        Ok(instances)
    }
}

impl YamlLoader {
    /// Convert object to DataInstance
    fn object_to_instance(&self, obj: Map<String, Value>, schema: &SchemaDefinition) -> LoaderResult<DataInstance> {
        // Try to determine class from @type field or structure
        let class_name = if let Some(Value::String(type_val)) = obj.get("@type") {
            type_val.clone()
        } else if let Some(Value::String(type_val)) = obj.get("type") {
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

/// YAML dumper for LinkML data
pub struct YamlDumper {
    /// Include document markers
    include_markers: bool,
}

impl YamlDumper {
    /// Create a new YAML dumper
    pub fn new() -> Self {
        Self {
            include_markers: true,
        }
    }
    
    /// Set whether to include document markers
    pub fn with_markers(mut self, include: bool) -> Self {
        self.include_markers = include;
        self
    }
}

impl Default for YamlDumper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataDumper for YamlDumper {
    async fn dump(&mut self, instances: &[DataInstance], _schema: &SchemaDefinition) -> DumperResult<Vec<u8>> {
        let yaml_instances: Vec<serde_yaml::Value> = instances.iter()
            .map(|instance| {
                let mut obj = instance.data.clone();
                if !obj.contains_key("@type") && !obj.contains_key("type") {
                    obj.insert("@type".to_string(), Value::String(instance.class_name.clone()));
                }
                
                // Convert to YAML value
                let json_str = serde_json::to_string(&Value::Object(obj))
                    .expect("valid JSON object should serialize");
                serde_yaml::from_str(&json_str)
                    .expect("valid JSON should parse as YAML")
            })
            .collect();
        
        let yaml_str = if yaml_instances.len() == 1 {
            serde_yaml::to_string(&yaml_instances[0])
        } else {
            serde_yaml::to_string(&yaml_instances)
        }.map_err(|e| DumperError::Serialization(e.to_string()))?;
        
        Ok(yaml_str.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_yaml_loader() {
        let yaml_content = r#"
@type: Person
name: John Doe
age: 30
emails:
  - john@example.com
  - johndoe@work.com
"#;
        
        // Create temp file
        let temp_file = tempfile::NamedTempFile::new().expect("should create temporary file");
        std::fs::write(temp_file.path(), yaml_content).expect("should write YAML content");
        
        let mut schema = SchemaDefinition::default();
        let mut class = ClassDefinition::default();
        class.slots = vec!["name".to_string(), "age".to_string(), "emails".to_string()];
        schema.classes.insert("Person".to_string(), class);
        
        let mut loader = YamlLoader::new().with_file(temp_file.path().to_str().expect("temp file path should be valid UTF-8"));
        let instances = loader.load(&schema).await.expect("should load YAML instances");
        
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].class_name, "Person");
        assert_eq!(instances[0].data.get("name"), Some(&Value::String("John Doe".to_string())));
        assert!(instances[0].data.get("emails").expect("should have emails field").is_array());
    }
    
    #[tokio::test]
    async fn test_yaml_dumper() {
        let instances = vec![
            DataInstance {
                class_name: "Person".to_string(),
                data: serde_json::from_str(r#"{
                    "name": "Alice",
                    "age": 25,
                    "active": true
                }"#).expect("should parse valid JSON"),
            },
        ];
        
        let schema = SchemaDefinition::default();
        let mut dumper = YamlDumper::new();
        let result = dumper.dump(&instances, &schema).await.expect("should dump instances to YAML");
        
        let yaml_str = String::from_utf8(result).expect("YAML should be valid UTF-8");
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_str).expect("should parse dumped YAML");
        
        assert_eq!(parsed["@type"], "Person");
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 25);
        assert_eq!(parsed["active"], true);
    }
}