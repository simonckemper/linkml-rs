//! JSON Schema generation for `LinkML` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{Value as JsonValue, json};
use std::collections::HashMap;

/// JSON Schema generator for `LinkML` schemas
pub struct JsonSchemaGenerator {
    /// Generator name
    name: String,
}

impl JsonSchemaGenerator {
    /// Create a new JSON Schema generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "json-schema".to_string(),
        }
    }

    /// Generate JSON Schema for a class
    fn generate_class_schema(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        definitions: &mut HashMap<String, JsonValue>,
    ) -> GeneratorResult<JsonValue> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Collect all slots including inherited ones
        let slots = self.collect_all_slots(class, schema)?;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let property = self.generate_property_schema(slot, schema)?;
                properties.insert(slot_name.clone(), property);

                if slot.required == Some(true) {
                    required.push(slot_name.clone());
                }
            }
        }

        let mut schema_obj = json!({
            "type": "object",
            "properties": properties,
        });

        // Add title and description
        schema_obj["title"] = json!(class_name);

        if let Some(desc) = &class.description {
            schema_obj["description"] = json!(desc);
        }

        // Add required array if not empty
        if !required.is_empty() {
            schema_obj["required"] = json!(required);
        }

        // Handle inheritance using allOf
        if let Some(parent) = &class.is_a {
            let parent_ref = json!({
                "$ref": format!("#/definitions/{}", parent)
            });

            schema_obj = json!({
                "allOf": [parent_ref, schema_obj]
            });
        }

        // Store in definitions
        definitions.insert(class_name.to_string(), schema_obj.clone());

        Ok(schema_obj)
    }

    /// Generate JSON Schema for a property (slot)
    fn generate_property_schema(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<JsonValue> {
        let base_schema = self.get_base_type_schema(&slot.range, schema)?;

        let mut property = if slot.multivalued == Some(true) {
            json!({
                "type": "array",
                "items": base_schema
            })
        } else {
            base_schema
        };

        // Add constraints
        if let Some(desc) = &slot.description {
            property["description"] = json!(desc);
        }

        if let Some(pattern) = &slot.pattern {
            property["pattern"] = json!(pattern);
        }

        if let Some(min) = &slot.minimum_value {
            property["minimum"] = json!(min);
        }

        if let Some(max) = &slot.maximum_value {
            property["maximum"] = json!(max);
        }

        Ok(property)
    }

    /// Get base JSON Schema type from `LinkML` range
    fn get_base_type_schema(
        &self,
        range: &Option<String>,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<JsonValue> {
        match range.as_deref() {
            Some("string" | "str") => Ok(json!({"type": "string"})),
            Some("integer" | "int") => Ok(json!({"type": "integer"})),
            Some("float" | "double" | "decimal") => Ok(json!({"type": "number"})),
            Some("boolean" | "bool") => Ok(json!({"type": "boolean"})),
            Some("date") => Ok(json!({
                "type": "string",
                "format": "date"
            })),
            Some("datetime") => Ok(json!({
                "type": "string",
                "format": "date-time"
            })),
            Some("uri" | "url") => Ok(json!({
                "type": "string",
                "format": "uri"
            })),
            Some(other) => {
                // Check if it's an enum
                if schema.enums.contains_key(other) {
                    Ok(json!({
                        "$ref": format!("#/definitions/{}", other)
                    }))
                } else if schema.classes.contains_key(other) {
                    // Reference to another class
                    Ok(json!({
                        "$ref": format!("#/definitions/{}", other)
                    }))
                } else if schema.types.contains_key(other) {
                    // Custom type
                    Ok(json!({
                        "$ref": format!("#/definitions/{}", other)
                    }))
                } else {
                    // Default to string
                    Ok(json!({"type": "string"}))
                }
            }
            None => Ok(json!({"type": "string"})),
        }
    }

    /// Generate enum schema
    fn generate_enum_schema(
        &self,
        enum_name: &str,
        enum_def: &EnumDefinition,
        definitions: &mut HashMap<String, JsonValue>,
    ) -> GeneratorResult<()> {
        let values: Vec<String> = enum_def
            .permissible_values
            .iter()
            .map(|v| match v {
                PermissibleValue::Simple(text) => text.clone(),
                PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();

        let mut schema = json!({
            "type": "string",
            "enum": values
        });

        if let Some(desc) = &enum_def.description {
            schema["description"] = json!(desc);
        }

        definitions.insert(enum_name.to_string(), schema);
        Ok(())
    }

    /// Generate type schema
    fn generate_type_schema(
        &self,
        type_name: &str,
        type_def: &TypeDefinition,
        schema: &SchemaDefinition,
        definitions: &mut HashMap<String, JsonValue>,
    ) -> GeneratorResult<()> {
        let base_schema = self.get_base_type_schema(&type_def.base_type, schema)?;

        let mut schema = base_schema;

        // Add constraints
        if let Some(desc) = &type_def.description {
            schema["description"] = json!(desc);
        }

        if let Some(pattern) = &type_def.pattern {
            schema["pattern"] = json!(pattern);
        }

        if let Some(min) = &type_def.minimum_value {
            schema["minimum"] = json!(min);
        }

        if let Some(max) = &type_def.maximum_value {
            schema["maximum"] = json!(max);
        }

        definitions.insert(type_name.to_string(), schema);
        Ok(())
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<String>> {
        let mut all_slots = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Add direct slots
        for slot in &class.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add inherited slots
        if let Some(parent) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent) {
                let parent_slots = self.collect_all_slots(parent_class, schema)?;
                for slot in parent_slots {
                    if seen.insert(slot.clone()) {
                        all_slots.push(slot);
                    }
                }
            }
        }

        Ok(all_slots)
    }
}

impl Default for JsonSchemaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for JsonSchemaGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate JSON Schema from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".json", ".schema.json"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        self.validate_schema(schema).await?;

        let mut definitions = HashMap::new();

        // Generate enum definitions
        for (enum_name, enum_def) in &schema.enums {
            self.generate_enum_schema(enum_name, enum_def, &mut definitions)?;
        }

        // Generate type definitions
        for (type_name, type_def) in &schema.types {
            self.generate_type_schema(type_name, type_def, schema, &mut definitions)?;
        }

        // Generate class definitions
        let mut root_classes = Vec::new();
        for (class_name, class) in &schema.classes {
            self.generate_class_schema(class_name, class, schema, &mut definitions)?;

            // Track root classes
            if class.tree_root == Some(true)
                || (class.is_a.is_none() && class.abstract_ != Some(true))
            {
                root_classes.push(class_name.clone());
            }
        }

        // Build the main schema
        let mut json_schema = json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "$id": schema.id.clone(),
            "title": schema.name.clone(),
            "definitions": definitions
        });

        if let Some(desc) = &schema.description {
            json_schema["description"] = json!(desc);
        }

        // If there's exactly one root class, make it the main schema
        if root_classes.len() == 1 {
            json_schema["$ref"] = json!(format!("#/definitions/{}", root_classes[0]));
        } else if !root_classes.is_empty() {
            // Multiple root classes - use oneOf
            let refs: Vec<JsonValue> = root_classes
                .iter()
                .map(|name| json!({"$ref": format!("#/definitions/{}", name)}))
                .collect();
            json_schema["oneOf"] = json!(refs);
        }

        // Format output
        let content = if options.get_custom("pretty_print") == Some("true") {
            serde_json::to_string_pretty(&json_schema)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        } else {
            serde_json::to_string(&json_schema)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        };

        // Create output
        let filename = format!(
            "{}.schema.json",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("schema_name".to_string(), schema.name.clone());
        metadata.insert("json_schema_version".to_string(), "draft-07".to_string());

        Ok(vec![GeneratedOutput {
            content,
            filename,
            metadata,
        }])
    }
}

impl CodeFormatter for JsonSchemaGenerator {
    fn format_doc(&self, doc: &str, _indent: &IndentStyle, _level: usize) -> String {
        doc.to_string()
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        _indent: &IndentStyle,
        _level: usize,
        separator: &str,
    ) -> String {
        items
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        // JSON escaping is handled by serde_json
        s.to_string()
    }

    fn convert_identifier(&self, id: &str) -> String {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_schema_generation() {
        let generator = JsonSchemaGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "https://example.com/schemas/test".to_string();
        schema.name = "test_schema".to_string();

        // Add a slot
        let mut slot = SlotDefinition::default();
        slot.name = "name".to_string();
        slot.range = Some("string".to_string());
        slot.required = Some(true);
        slot.pattern = Some("^[A-Za-z]+$".to_string());

        schema.slots.insert("name".to_string(), slot);

        // Add an enum
        let mut enum_def = EnumDefinition::default();
        enum_def.permissible_values = vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ];

        schema.enums.insert("Status".to_string(), enum_def);

        // Add a class
        let mut class = ClassDefinition::default();
        class.name = "Person".to_string();
        class.slots = vec!["name".to_string()];

        schema.classes.insert("Person".to_string(), class);

        let options = GeneratorOptions::new().set_custom("pretty_print", "true");

        let outputs = generator.generate(&schema, &options).await.expect("should generate JSON schema");

        assert_eq!(outputs.len(), 1);
        let json_content = &outputs[0].content;

        // Parse to verify it's valid JSON
        let parsed: JsonValue = serde_json::from_str(json_content).expect("should parse as valid JSON");

        // Check basic structure
        assert_eq!(parsed["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(parsed["$id"], "https://example.com/schemas/test");
        assert_eq!(parsed["title"], "test_schema");

        // Check definitions
        assert!(parsed["definitions"]["Person"].is_object());
        assert!(parsed["definitions"]["Status"].is_object());

        // Check enum values
        let status_enum = &parsed["definitions"]["Status"]["enum"];
        assert!(status_enum.as_array().expect("enum should be array").contains(&json!("ACTIVE")));
        assert!(status_enum.as_array().expect("enum should be array").contains(&json!("INACTIVE")));
    }
}
