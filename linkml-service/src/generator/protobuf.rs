//! Protocol Buffers code generator for `LinkML` schemas
//!
//! This module generates Protocol Buffers (.proto) files from `LinkML` schemas,
//! enabling cross-language serialization and RPC support.

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult};
use linkml_core::error::LinkMLError;

/// Protocol Buffers generator
pub struct ProtobufGenerator {
    /// Generator options
    options: GeneratorOptions,
    /// Type mapping from `LinkML` to Proto
    type_map: HashMap<String, String>,
}

impl ProtobufGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new Protocol Buffers generator
    #[must_use]
    pub fn new() -> Self {
        let mut type_map = HashMap::new();

        // Basic type mappings
        type_map.insert("string".to_string(), "string".to_string());
        type_map.insert("str".to_string(), "string".to_string());
        type_map.insert("integer".to_string(), "int64".to_string());
        type_map.insert("int".to_string(), "int64".to_string());
        type_map.insert("float".to_string(), "double".to_string());
        type_map.insert("double".to_string(), "double".to_string());
        type_map.insert("decimal".to_string(), "double".to_string());
        type_map.insert("boolean".to_string(), "bool".to_string());
        type_map.insert("bool".to_string(), "bool".to_string());
        type_map.insert("date".to_string(), "string".to_string()); // ISO 8601 string
        type_map.insert("datetime".to_string(), "string".to_string()); // ISO 8601 string
        type_map.insert("time".to_string(), "string".to_string()); // ISO 8601 string
        type_map.insert("uri".to_string(), "string".to_string());
        type_map.insert("uriorcurie".to_string(), "string".to_string());
        type_map.insert("curie".to_string(), "string".to_string());
        type_map.insert("ncname".to_string(), "string".to_string());

        Self {
            options: GeneratorOptions::default(),
            type_map,
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Generate proto file header
    fn generate_header(schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "// Generated from LinkML schema: {}",
            schema.name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "// Schema ID: {}", schema.id)
            .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(version) = &schema.version {
            writeln!(&mut output, "// Version: {version}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "syntax = \"proto3\";")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Package name from schema name
        let package_name = Self::to_snake_case(&schema.name);
        writeln!(&mut output, "package {package_name};")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Import well-known types if needed
        if Self::needs_timestamp_import(schema) {
            writeln!(&mut output, "import \"google/protobuf/timestamp.proto\";")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Check if schema needs timestamp import
    fn needs_timestamp_import(schema: &SchemaDefinition) -> bool {
        // Check if any slot uses datetime type
        schema
            .slots
            .values()
            .any(|slot| matches!(slot.range.as_deref(), Some("datetime" | "timestamp")))
    }

    /// Generate enum definition
    fn generate_enum(name: &str, enum_def: &EnumDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add description as comment
        if let Some(desc) = &enum_def.description {
            writeln!(&mut output, "// {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "enum {} {{", Self::to_pascal_case(name))
            .map_err(Self::fmt_error_to_generator_error)?;

        // Proto3 requires first enum value to be 0
        writeln!(
            &mut output,
            "  {}_UNSPECIFIED = 0;",
            Self::to_screaming_snake_case(name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate enum values
        for (index, pv) in enum_def.permissible_values.iter().enumerate() {
            let text = match pv {
                PermissibleValue::Simple(s) => s,
                PermissibleValue::Complex { text, .. } => text,
            };
            // For enum values, keep them uppercase if they already are
            let enum_value = if text.chars().all(|c| c.is_uppercase() || !c.is_alphabetic()) {
                text.to_string()
            } else {
                Self::to_screaming_snake_case(text)
            };
            writeln!(&mut output, "  {} = {};", enum_value, index + 1)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate message (class) definition
    fn generate_message(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add description as comment
        if let Some(desc) = &class.description {
            writeln!(&mut output, "// {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "message {} {{", Self::to_pascal_case(name))
            .map_err(Self::fmt_error_to_generator_error)?;

        // Collect all slots (including inherited)
        let all_slots = self.collect_all_slots(class, schema);

        // Generate fields with proper numbering
        let mut field_number = 1;
        let mut seen_slots = HashSet::new();

        for slot_name in &all_slots {
            if seen_slots.contains(slot_name) {
                continue;
            }
            seen_slots.insert(slot_name);

            if let Some(slot) = schema.slots.get(slot_name) {
                let field = self.generate_field(slot, field_number, schema)?;
                write!(&mut output, "{field}").map_err(Self::fmt_error_to_generator_error)?;
                field_number += 1;
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Recursively collect all slots from a class and its parents
    fn collect_all_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();

        // First, recursively get slots from parent
        if let Some(parent_name) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            all_slots.extend(self.collect_all_slots(parent_class, schema));
        }

        // Then add direct slots
        all_slots.extend(class.slots.clone());

        all_slots
    }

    /// Generate a proto field from a slot
    fn generate_field(
        &self,
        slot: &SlotDefinition,
        field_number: u32,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add description as comment
        if let Some(desc) = &slot.description {
            writeln!(&mut output, "  // {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine proto type
        let proto_type = self.get_proto_type(slot.range.as_ref(), schema)?;

        // Handle repeated fields
        let repeated = if slot.multivalued.unwrap_or(false) {
            "repeated "
        } else {
            ""
        };

        // Generate field
        let field_name = Self::to_snake_case(&slot.name);
        writeln!(
            &mut output,
            "  {repeated}{proto_type} {field_name} = {field_number};"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Get proto type for a `LinkML` range
    fn get_proto_type(
        &self,
        range: Option<&String>,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        match range {
            Some(r) => {
                // Check if it's a built-in type
                if let Some(proto_type) = self.type_map.get(r) {
                    Ok(proto_type.clone())
                } else if let Some(type_def) = schema.types.get(r) {
                    // It's a custom type, resolve to its base type
                    self.get_proto_type(type_def.base_type.as_ref(), schema)
                } else {
                    // Assume it's a message or enum type
                    Ok(Self::to_pascal_case(r))
                }
            }
            None => Ok("string".to_string()), // Default to string
        }
    }

    /// Convert to `snake_case`
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_upper = ch.is_uppercase();
        }

        result
    }

    /// Convert to `PascalCase`
    fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }

    /// Convert to `SCREAMING_SNAKE_CASE`
    fn to_screaming_snake_case(s: &str) -> String {
        // Handle hyphens and underscores
        let with_underscores = s.replace('-', "_");

        // Simple approach: insert underscore before uppercase letters that follow lowercase
        let mut result = String::new();
        let mut prev_lowercase = false;

        for ch in with_underscores.chars() {
            if ch == '_' {
                // Keep existing underscores
                result.push('_');
                prev_lowercase = false;
            } else if ch.is_uppercase() {
                // Add underscore before uppercase if previous was lowercase
                if prev_lowercase && !result.is_empty() {
                    result.push('_');
                }
                result.push(ch);
                prev_lowercase = false;
            } else {
                // Convert lowercase to uppercase
                result.push(ch.to_ascii_uppercase());
                prev_lowercase = ch.is_lowercase();
            }
        }

        result
    }
}

impl Default for ProtobufGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for ProtobufGenerator {
    fn name(&self) -> &'static str {
        "protobuf"
    }

    fn description(&self) -> &'static str {
        "Generates Protocol Buffers (.proto) files from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".proto"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Protobuf generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let mut output = String::new();

        // Generate header
        output.push_str(&Self::generate_header(schema)?);

        // Generate enums first
        let mut enum_output = String::new();
        for (name, enum_def) in &schema.enums {
            let enum_code = Self::generate_enum(name, enum_def)
                .map_err(|e| LinkMLError::service(format!("Error generating enum {name}: {e}")))?;
            writeln!(&mut enum_output, "{enum_code}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if !enum_output.is_empty() {
            output.push_str(&enum_output);
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate messages
        for (name, class) in &schema.classes {
            let message_code = self
                .generate_message(name, class, schema)
                .map_err(|e| LinkMLError::service(format!("Error generating class {name}: {e}")))?;
            writeln!(&mut output, "{message_code}").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    fn get_file_extension(&self) -> &'static str {
        "proto"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_protobuf_generation() -> anyhow::Result<()> {
        let mut schema = SchemaDefinition::new("test_schema");
        schema.id = "https://example.org/test".to_string();
        schema.version = Some("1.0.0".to_string());

        // Add an enum
        let status_enum = EnumDefinition {
            name: "Status".to_string(),
            description: Some("Order status".to_string()),
            permissible_values: vec![
                PermissibleValue::Complex {
                    text: "pending".to_string(),
                    description: Some("Pending status".to_string()),
                    meaning: None,
                },
                PermissibleValue::Complex {
                    text: "approved".to_string(),
                    description: Some("Approved status".to_string()),
                    meaning: None,
                },
            ],
            ..Default::default()
        };
        schema.enums.insert("Status".to_string(), status_enum);

        // Add slots
        let mut name_slot = SlotDefinition::new("name");
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut age_slot = SlotDefinition::new("age");
        age_slot.range = Some("integer".to_string());
        schema.slots.insert("age".to_string(), age_slot);

        let mut tags_slot = SlotDefinition::new("tags");
        tags_slot.range = Some("string".to_string());
        tags_slot.multivalued = Some(true);
        schema.slots.insert("tags".to_string(), tags_slot);

        let mut status_slot = SlotDefinition::new("status");
        status_slot.range = Some("Status".to_string());
        schema.slots.insert("status".to_string(), status_slot);

        // Add a class
        let mut person_class = ClassDefinition::new("Person");
        person_class.slots = vec![
            "name".to_string(),
            "age".to_string(),
            "tags".to_string(),
            "status".to_string(),
        ];
        schema.classes.insert("Person".to_string(), person_class);

        // Generate protobuf
        let generator = ProtobufGenerator::new();
        let proto_content = generator
            .generate(&schema)
            .expect("should generate protobuf: {}");

        assert!(proto_content.contains("syntax = \"proto3\""));
        assert!(proto_content.contains("package test_schema"));
        assert!(proto_content.contains("enum Status"));
        assert!(proto_content.contains("STATUS_UNSPECIFIED = 0"));
        assert!(proto_content.contains("PENDING = 1"));
        assert!(proto_content.contains("APPROVED = 2"));
        assert!(proto_content.contains("message Person"));
        assert!(proto_content.contains("string name = 1"));
        assert!(proto_content.contains("int64 age = 2"));
        assert!(proto_content.contains("repeated string tags = 3"));
        assert!(proto_content.contains("Status status = 4"));
        Ok(())
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(
            ProtobufGenerator::to_snake_case("PersonName"),
            "person_name"
        );
        assert_eq!(
            ProtobufGenerator::to_snake_case("HTTPRequest"),
            "httprequest"
        );
        assert_eq!(
            ProtobufGenerator::to_snake_case("already_snake"),
            "already_snake"
        );

        assert_eq!(
            ProtobufGenerator::to_pascal_case("person_name"),
            "PersonName"
        );
        assert_eq!(
            ProtobufGenerator::to_pascal_case("http_request"),
            "HttpRequest"
        );
        assert_eq!(
            ProtobufGenerator::to_pascal_case("AlreadyPascal"),
            "AlreadyPascal"
        );

        assert_eq!(
            ProtobufGenerator::to_screaming_snake_case("personName"),
            "PERSON_NAME"
        );
        assert_eq!(
            ProtobufGenerator::to_screaming_snake_case("STATUS"),
            "STATUS"
        );
    }

    #[test]
    fn test_type_mapping() -> anyhow::Result<()> {
        let generator = ProtobufGenerator::new();
        let schema = SchemaDefinition::new("test");

        assert_eq!(
            generator
                .get_proto_type(Some(&"string".to_string()), &schema)
                .expect("should get proto type: {}"),
            "string"
        );
        assert_eq!(
            generator
                .get_proto_type(Some(&"integer".to_string()), &schema)
                .expect("should get proto type: {}"),
            "int64"
        );
        assert_eq!(
            generator
                .get_proto_type(Some(&"boolean".to_string()), &schema)
                .expect("should get proto type: {}"),
            "bool"
        );
        assert_eq!(
            generator
                .get_proto_type(Some(&"CustomType".to_string()), &schema)
                .expect("should get proto type: {}"),
            "CustomType"
        );
        assert_eq!(
            generator
                .get_proto_type(None, &schema)
                .expect("should get proto type: {}"),
            "string"
        );
        Ok(())
    }
}
