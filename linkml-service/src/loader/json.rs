//! JSON loader and dumper for `LinkML`
//!
//! This module provides functionality to load and dump `LinkML` data in JSON format.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{Map, Value};
use std::collections::HashMap;

/// `JSON` loader for `LinkML` data
pub struct JsonLoader {
    /// Input file path
    file_path: Option<String>,
}

impl JsonLoader {
    /// Create a new `JSON` loader
    #[must_use] pub fn new() -> Self {
        Self { file_path: None }
    }

    /// Set the input file path
    #[must_use] pub fn with_file(mut self, path: &str) -> Self {
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
    fn name(&self) -> &'static str {
        "json"
    }

    fn description(&self) -> &'static str {
        "Load data from JSON files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["json", "jsonld"]
    }

    async fn load_file(
        &self,
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = std::fs::read_to_string(path).map_err(LoaderError::Io)?;
        self.load_string(&content, schema, options).await
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let json: Value =
            serde_json::from_str(content).map_err(|e| LoaderError::Parse(e.to_string()))?;

        // Apply options for validation and filtering
        if options.validate {
            self.validate_schema(schema)?;
        }

        let instances = match json {
            Value::Array(arr) => {
                // Array of instances
                let mut instances = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    if let Value::Object(obj) = item {
                        let instance = self.object_to_instance(obj.clone(), schema)?;

                        // Apply class filtering if specified in options
                        if let Some(ref target_class) = options.target_class
                            && instance.class_name != *target_class {
                                continue;
                            }

                        // Apply limit if specified
                        if let Some(limit) = options.limit
                            && instances.len() >= limit {
                                break;
                            }

                        instances.push(instance);
                    } else if !options.skip_invalid {
                        return Err(LoaderError::InvalidFormat(
                            format!("Array item {index} is not an object")
                        ));
                    }
                }
                instances
            }
            Value::Object(obj) => {
                // Single instance
                let instance = self.object_to_instance(obj, schema)?;

                // Apply class filtering if specified in options
                if let Some(ref target_class) = options.target_class
                    && instance.class_name != *target_class {
                        return Ok(vec![]);
                    }

                vec![instance]
            }
            _ => {
                if !options.skip_invalid {
                    return Err(LoaderError::InvalidFormat(
                        "JSON must be an object or array of objects".to_string(),
                    ));
                }
                // In skip_invalid mode, return empty result
                vec![]
            }
        };

        Ok(instances)
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content =
            String::from_utf8(data.to_vec()).map_err(|e| LoaderError::Parse(e.to_string()))?;
        self.load_string(&content, schema, options).await
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> LoaderResult<()> {
        Ok(())
    }
}

impl JsonLoader {
    /// Convert `JSON` object to `DataInstance`
    fn object_to_instance(
        &self,
        obj: Map<String, Value>,
        schema: &SchemaDefinition,
    ) -> LoaderResult<DataInstance> {
        // Try to determine class from @type field or structure
        let class_name = if let Some(Value::String(type_val)) = obj.get("@type") {
            type_val.clone()
        } else {
            // Try to infer from structure
            self.infer_class(&obj, schema)?
        };

        Ok(DataInstance {
            class_name,
            data: obj.into_iter().collect(),
            id: None,
            metadata: HashMap::new(),
        })
    }

    /// Infer class from object structure
    fn infer_class(
        &self,
        obj: &Map<String, Value>,
        schema: &SchemaDefinition,
    ) -> LoaderResult<String> {
        let obj_keys: std::collections::HashSet<_> = obj.keys().cloned().collect();

        // Find best matching class
        let mut best_match = None;
        let mut best_score = 0;

        for (class_name, class_def) in &schema.classes {
            let class_slots: std::collections::HashSet<_> =
                class_def.slots.iter().cloned().collect();
            let intersection = obj_keys.intersection(&class_slots).count();

            if intersection > best_score {
                best_score = intersection;
                best_match = Some(class_name.clone());
            }
        }

        best_match.ok_or_else(|| {
            LoaderError::SchemaValidation("Could not infer class from object structure".to_string())
        })
    }
}

/// `JSON` dumper for `LinkML` data
pub struct JsonDumper {
    /// Pretty print output
    pretty: bool,
}

impl JsonDumper {
    /// Create a new `JSON` dumper
    #[must_use] pub fn new(pretty: bool) -> Self {
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
    fn name(&self) -> &'static str {
        "json"
    }

    fn description(&self) -> &'static str {
        "Dump data to JSON format"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["json", "jsonld"]
    }

    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()> {
        let content = self.dump_string(instances, schema, options).await?;
        std::fs::write(path, content).map_err(DumperError::Io)?;
        Ok(())
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        _schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String> {
        let json_instances: Vec<Value> = instances
            .iter()
            .map(|instance| {
                let mut obj = Map::new();
                // Convert HashMap to Map
                for (k, v) in &instance.data {
                    obj.insert(k.clone(), v.clone());
                }
                obj.insert(
                    "@type".to_string(),
                    Value::String(instance.class_name.clone()),
                );
                Value::Object(obj)
            })
            .collect();

        let json_str = if options.pretty_print || self.pretty {
            serde_json::to_string_pretty(&json_instances)
        } else {
            serde_json::to_string(&json_instances)
        }
        .map_err(|e| DumperError::Serialization(e.to_string()))?;

        Ok(json_str)
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let result = self.dump_string(instances, schema, options).await?;
        Ok(result.into_bytes())
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> DumperResult<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_loader() -> std::result::Result<(), anyhow::Error> {
        let json_content = r#"
        {
            "@type": "Person",
            "name": "John Doe",
            "age": 30
        }
        "#;

        // Create temp file
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), json_content)?;

        let mut schema = SchemaDefinition::default();
        let mut class = ClassDefinition::default();
        class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), class);

        let loader = JsonLoader::new();
        let options = LoadOptions::default();
        let instances = loader
            .load_file(temp_file.path(), &schema, &options)
            .await?;

        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].class_name, "Person");
        assert_eq!(
            instances[0].data.get("name"),
            Some(&Value::String("John Doe".to_string()))
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_json_dumper() -> std::result::Result<(), anyhow::Error> {
        let mut alice_data = HashMap::new();
        alice_data.insert("name".to_string(), Value::String("Alice".to_string()));
        alice_data.insert(
            "age".to_string(),
            Value::Number(serde_json::Number::from(25)),
        );

        let mut bob_data = HashMap::new();
        bob_data.insert("name".to_string(), Value::String("Bob".to_string()));
        bob_data.insert(
            "age".to_string(),
            Value::Number(serde_json::Number::from(30)),
        );

        let instances = vec![
            DataInstance {
                class_name: "Person".to_string(),
                data: alice_data,
                id: None,
                metadata: HashMap::new(),
            },
            DataInstance {
                class_name: "Person".to_string(),
                data: bob_data,
                id: None,
                metadata: HashMap::new(),
            },
        ];

        let schema = SchemaDefinition::default();
        let dumper = JsonDumper::new(false);
        let options = DumpOptions::default();
        let json_str = dumper
            .dump_string(&instances, &schema, &options)
            .await?;

        let parsed: Vec<Value> = serde_json::from_str(&json_str)?;

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["@type"], "Person");
        assert_eq!(parsed[0]["name"], "Alice");
        Ok(())
    }
}