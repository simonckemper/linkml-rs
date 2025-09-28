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

type Result<T> = std::result::Result<T, LoaderError>;

/// `JSON` loader for `LinkML` data
pub struct JsonLoader {
    /// Input file path
    file_path: Option<String>,
}

impl JsonLoader {
    /// Create a new `JSON` loader
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
                            "Array item {index} is not an object"
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

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that schema is compatible with JSON loading

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema name is required for JSON loading".to_string(),
            ));
        }

        // Validate that classes have valid structure for JSON representation
        for (class_name, class_def) in &schema.classes {
            // Validate slots for JSON compatibility
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
                        && !is_valid_json_datatype(range)
                    {
                        return Err(LoaderError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or JSON-compatible datatype"
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
                    && !is_valid_json_datatype(range)
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or JSON-compatible datatype"
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

        // Check for circular inheritance that could cause issues in JSON processing
        check_inheritance_cycles(schema)?;

        Ok(())
    }
}

/// Check for inheritance cycles in the schema that could cause infinite recursion
fn check_inheritance_cycles(schema: &linkml_core::types::SchemaDefinition) -> Result<()> {
    let mut visited = std::collections::HashSet::new();
    let mut path = std::collections::HashSet::new();

    for class_name in schema.classes.keys() {
        if !visited.contains(class_name) {
            visit_class_inheritance(schema, class_name, &mut visited, &mut path)?;
        }
    }

    Ok(())
}

/// Depth-first search to detect cycles in inheritance hierarchy
fn visit_class_inheritance(
    schema: &linkml_core::types::SchemaDefinition,
    class_name: &str,
    visited: &mut std::collections::HashSet<String>,
    path: &mut std::collections::HashSet<String>,
) -> Result<()> {
    if path.contains(class_name) {
        return Err(LoaderError::SchemaValidation(format!(
            "Circular inheritance detected involving class: {class_name}"
        )));
    }

    if visited.contains(class_name) {
        return Ok(());
    }

    if let Some(class_def) = schema.classes.get(class_name) {
        path.insert(class_name.to_string());

        // Check parent class
        if let Some(parent) = &class_def.is_a {
            visit_class_inheritance(schema, parent, visited, path)?;
        }

        // Check mixins
        for mixin in &class_def.mixins {
            visit_class_inheritance(schema, mixin, visited, path)?;
        }

        path.remove(class_name);
        visited.insert(class_name.to_string());
    }

    Ok(())
}

/// Check if a type name represents a valid JSON-compatible datatype
fn is_valid_json_datatype(type_name: &str) -> bool {
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
    #[must_use]
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

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate that schema is compatible with JSON dumping

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Schema name is required for JSON dumping".to_string(),
            ));
        }

        // Validate that classes have valid structure for JSON representation
        for (class_name, class_def) in &schema.classes {
            // Validate slots for JSON compatibility
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
                        && !is_valid_json_datatype(range)
                    {
                        return Err(DumperError::SchemaValidation(format!(
                            "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or JSON-compatible datatype"
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

        // Validate slot definitions
        for (slot_name, slot_def) in &schema.slots {
            // Check if range is valid
            if let Some(range) = &slot_def.range {
                if !schema.classes.contains_key(range)
                    && !schema.enums.contains_key(range)
                    && !is_valid_json_datatype(range)
                {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or JSON-compatible datatype"
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
        let json_str = dumper.dump_string(&instances, &schema, &options).await?;

        let parsed: Vec<Value> = serde_json::from_str(&json_str)?;

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["@type"], "Person");
        assert_eq!(parsed[0]["name"], "Alice");
        Ok(())
    }
}
