//! YAML loader and dumper for `LinkML`
//!
//! This module provides functionality to load and dump `LinkML` data in YAML format.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{Map, Value};
use serde_yaml;
use std::collections::HashMap;

/// `YAML` loader for `LinkML` data
pub struct YamlLoader {
    /// Input file path
    file_path: Option<String>,
}

impl YamlLoader {
    /// Create a new `YAML` loader
    #[must_use]
    pub fn new() -> Self {
        Self { file_path: None }
    }

    /// Set the input file path
    #[must_use]
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
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn description(&self) -> &'static str {
        "Load data from YAML files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml"]
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
        let yaml_value: serde_yaml::Value =
            serde_yaml::from_str(content).map_err(|e| LoaderError::Parse(e.to_string()))?;

        // Apply options for validation and filtering
        if options.validate {
            self.validate_schema(schema)?;
        }

        // Convert YAML to JSON for easier processing
        let json_str =
            serde_json::to_string(&yaml_value).map_err(|e| LoaderError::Parse(e.to_string()))?;
        let json: Value =
            serde_json::from_str(&json_str).map_err(|e| LoaderError::Parse(e.to_string()))?;

        let instances = match json {
            Value::Array(arr) => {
                // Array of instances
                let mut instances = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    if let Value::Object(obj) = item {
                        let instance = self.object_to_instance(obj.clone(), schema)?;

                        // Apply class filtering if specified in options
                        if let Some(ref target_class) = options.target_class
                            && instance.class_name != *target_class
                        {
                            continue;
                        }

                        // Apply limit if specified
                        if let Some(limit) = options.limit
                            && instances.len() >= limit
                        {
                            break;
                        }

                        instances.push(instance);
                    } else if !options.skip_invalid {
                        return Err(LoaderError::InvalidFormat(format!(
                            "Array item {index} is not a mapping"
                        )));
                    }
                }
                instances
            }
            Value::Object(obj) => {
                // Single instance
                let instance = self.object_to_instance(obj, schema)?;

                // Apply class filtering if specified in options
                if let Some(ref target_class) = options.target_class
                    && instance.class_name != *target_class
                {
                    return Ok(vec![]);
                }

                vec![instance]
            }
            _ => {
                if !options.skip_invalid {
                    return Err(LoaderError::InvalidFormat(
                        "YAML must be a mapping or sequence of mappings".to_string(),
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

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that schema is compatible with YAML loading

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema name is required for YAML loading".to_string(),
            ));
        }

        // Validate that classes have valid structure for YAML representation
        for (class_name, class_def) in &schema.classes {
            // Validate slots for YAML compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                let slot_def = &schema.slots[slot_name];

                // Validate range constraints
                if let Some(range) = &slot_def.range {
                    if !schema.classes.contains_key(range)
                        && !schema.enums.contains_key(range)
                        && !is_valid_yaml_datatype(range)
                    {
                        return Err(LoaderError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or YAML-compatible datatype"
                        )));
                    }
                }

                // Check domain constraints
                if let Some(domain) = &slot_def.domain {
                    if !schema.classes.contains_key(domain) {
                        return Err(LoaderError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                        )));
                    }
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a {
                if !schema.classes.contains_key(parent_name) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                    )));
                }
            }
        }

        // Validate slot definitions
        for (slot_name, slot_def) in &schema.slots {
            // Check if range is valid
            if let Some(range) = &slot_def.range {
                if !schema.classes.contains_key(range)
                    && !schema.enums.contains_key(range)
                    && !is_valid_yaml_datatype(range)
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or YAML-compatible datatype"
                    )));
                }
            }
        }

        // Validate enums
        for (enum_name, enum_def) in &schema.enums {
            // Check enum values
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Check if a type name represents a valid YAML-compatible datatype
fn is_valid_yaml_datatype(type_name: &str) -> bool {
    matches!(
        type_name,
        "string"
            | "boolean"
            | "integer"
            | "float"
            | "double"
            | "decimal"
            | "date"
            | "datetime"
            | "time"
            | "uri"
            | "uriorcurie"
            | "ncname"
            | "nodeidentifier"
            | "jsonpointer"
            | "jsonpath"
            | "sparqlpath"
            | "curie"
            | "int"
            | "long"
            | "short"
            | "byte"
            | "unsignedInt"
            | "unsignedLong"
            | "unsignedShort"
            | "unsignedByte"
            | "positiveInteger"
            | "nonNegativeInteger"
            | "negativeInteger"
            | "nonPositiveInteger"
    )
}

impl YamlLoader {
    /// Convert object to `DataInstance`
    fn object_to_instance(
        &self,
        obj: Map<String, Value>,
        schema: &SchemaDefinition,
    ) -> LoaderResult<DataInstance> {
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

/// `YAML` dumper for `LinkML` data
pub struct YamlDumper {
    /// Include document markers
    include_markers: bool,
}

impl YamlDumper {
    /// Create a new `YAML` dumper
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_markers: true,
        }
    }

    /// Set whether to include document markers
    #[must_use]
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
    fn name(&self) -> &'static str {
        "yaml"
    }

    fn description(&self) -> &'static str {
        "Dump data to YAML format"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml"]
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
        _options: &DumpOptions,
    ) -> DumperResult<String> {
        let yaml_instances: std::result::Result<Vec<serde_yaml::Value>, DumperError> = instances
            .iter()
            .map(|instance| {
                let mut obj = instance.data.clone();
                if !obj.contains_key("@type") && !obj.contains_key("type") {
                    obj.insert(
                        "@type".to_string(),
                        Value::String(instance.class_name.clone()),
                    );
                }

                // Convert to YAML value
                let json_obj = Value::Object(serde_json::Map::from_iter(obj));
                let json_str = serde_json::to_string(&json_obj).map_err(|e| {
                    DumperError::Serialization(format!("JSON serialization failed: {e}"))
                })?;
                serde_yaml::from_str(&json_str)
                    .map_err(|e| DumperError::Serialization(format!("YAML parsing failed: {e}")))
            })
            .collect::<std::result::Result<Vec<_>, DumperError>>();
        let yaml_instances = yaml_instances?;

        let yaml_str = if yaml_instances.len() == 1 {
            serde_yaml::to_string(&yaml_instances[0])
        } else {
            serde_yaml::to_string(&yaml_instances)
        }
        .map_err(|e| DumperError::Serialization(e.to_string()))?;

        Ok(yaml_str)
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

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate that schema is compatible with YAML dumping

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Schema name is required for YAML dumping".to_string(),
            ));
        }

        // Validate that classes have valid structure for YAML representation
        for (class_name, class_def) in &schema.classes {
            // Validate slots for YAML compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                let slot_def = &schema.slots[slot_name];

                // Validate range constraints
                if let Some(range) = &slot_def.range {
                    if !schema.classes.contains_key(range)
                        && !schema.enums.contains_key(range)
                        && !is_valid_yaml_datatype(range)
                    {
                        return Err(DumperError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or YAML-compatible datatype"
                        )));
                    }
                }

                // Check domain constraints
                if let Some(domain) = &slot_def.domain {
                    if !schema.classes.contains_key(domain) {
                        return Err(DumperError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                        )));
                    }
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a {
                if !schema.classes.contains_key(parent_name) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                    )));
                }
            }
        }

        // Validate enums
        for (enum_name, enum_def) in &schema.enums {
            // Check enum values
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(DumperError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_yaml_loader() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let yaml_content = r#"
"@type": Person
name: John Doe
age: 30
emails:
  - john@example.com
  - johndoe@work.com
"#;

        // Create temp file
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), yaml_content)?;

        let mut schema = SchemaDefinition::default();
        let mut class = ClassDefinition::default();
        class.slots = vec!["name".to_string(), "age".to_string(), "emails".to_string()];
        schema.classes.insert("Person".to_string(), class);

        let loader = YamlLoader::new();
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
        assert!(
            instances[0]
                .data
                .get("emails")
                .ok_or_else(|| anyhow::anyhow!("should have emails field"))?
                .is_array()
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_yaml_dumper() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let instances = vec![DataInstance {
            class_name: "Person".to_string(),
            data: {
                let mut map = HashMap::new();
                map.insert("name".to_string(), serde_json::json!("Alice"));
                map.insert("age".to_string(), serde_json::json!(25));
                map.insert("active".to_string(), serde_json::json!(true));
                map
            },
            id: Some("person_1".to_string()),
            metadata: HashMap::new(),
        }];

        let schema = SchemaDefinition::default();
        let dumper = YamlDumper::new();
        let options = DumpOptions::default();
        let yaml_str = dumper.dump_string(&instances, &schema, &options).await?;
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_str)?;

        assert_eq!(parsed["@type"], "Person");
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["age"], 25);
        assert_eq!(parsed["active"], true);
        Ok(())
    }
}
