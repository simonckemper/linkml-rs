//! YAML validator generator for `LinkML` schemas
//!
//! This module generates YAML validation rules and schemas from `LinkML` schemas,
//! enabling validation of YAML data against LinkML-defined structures.

use crate::generator::traits::{Generator, GeneratorConfig};
use std::fmt::Write;
use linkml_core::error::LinkMLError;
use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
    TypeDefinition};
use serde_json::{Map, Value, json};

/// `YAML` validator generator configuration
#[derive(Debug, Clone)]
pub struct YamlValidatorGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Validation framework to target
    pub framework: ValidationFramework,
    /// Whether to include inline documentation
    pub include_docs: bool,
    /// Whether to generate strict validation rules
    pub strict_mode: bool,
    /// Whether to include custom error messages
    pub custom_error_messages: bool,
    /// Additional validation plugins to include
    pub plugins: Vec<String>}

/// Supported validation frameworks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationFramework {
    /// `JSON` `SchemaDefinition` for `YAML` validation
    JsonSchemaDefinition,
    /// Cerberus (Python) validation rules
    Cerberus,
    /// Joi (JavaScript) validation schema
    Joi,
    /// Yup (JavaScript) validation schema
    Yup,
    /// Open`API`/Swagger specification
    OpenAPI}

impl Default for YamlValidatorGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            framework: ValidationFramework::JsonSchemaDefinition,
            include_docs: true,
            strict_mode: false,
            custom_error_messages: true,
            plugins: Vec::new()}
    }
}

/// `YAML` validator generator
pub struct YamlValidatorGenerator {
    config: YamlValidatorGeneratorConfig,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl YamlValidatorGenerator {
    /// Create a new `YAML` validator generator
    #[must_use] pub fn new(config: YamlValidatorGeneratorConfig) -> Self {
        Self { config }
    }

    /// Generate validation schema for the configured framework
    fn generate_validation(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        match self.config.framework {
            ValidationFramework::JsonSchemaDefinition => self.generate_json_schema(schema),
            ValidationFramework::Cerberus => self.generate_cerberus(schema),
            ValidationFramework::Joi => self.generate_joi(schema),
            ValidationFramework::Yup => Ok(self.generate_yup(schema)),
            ValidationFramework::OpenAPI => self.generate_openapi(schema)}
    }

    /// Generate `JSON` `SchemaDefinition` for `YAML` validation
    fn generate_json_schema(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut json_schema = Map::new();

        // SchemaDefinition metadata
        json_schema.insert(
            "$schema".to_string(),
            json!("http://json-schema.org/draft-07/schema#"),
        );
        if !schema.id.is_empty() {
            json_schema.insert("$id".to_string(), json!(&schema.id));
        }
        if !schema.name.is_empty() {
            json_schema.insert("title".to_string(), json!(&schema.name));
        }
        if let Some(description) = &schema.description {
            json_schema.insert("description".to_string(), json!(description));
        }

        // Definitions for types, enums, and classes
        let mut definitions = Map::new();

        // Add type definitions
        if !schema.types.is_empty() {
            for (type_name, type_def) in &schema.types {
                definitions.insert(type_name.clone(), self.type_to_json_schema(type_def)?);
            }
        }

        // Add enum definitions
        if !schema.enums.is_empty() {
            for (enum_name, enum_def) in &schema.enums {
                definitions.insert(enum_name.clone(), self.enum_to_json_schema(enum_def)?);
            }
        }

        // Add class definitions
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                definitions.insert(
                    class_name.clone(),
                    self.class_to_json_schema(class_def, schema)?,
                );
            }
        }

        json_schema.insert("definitions".to_string(), Value::Object(definitions));

        // If strict mode, require at least one class instance
        if self.config.strict_mode {
            json_schema.insert("type".to_string(), json!("object"));
            json_schema.insert("additionalProperties".to_string(), json!(false));

            if !schema.classes.is_empty() {
                let class_refs: Vec<Value> = schema
                    .classes
                    .keys()
                    .map(|name| json!({ "$ref": format!("#/definitions/{name}") }))
                    .collect();
                json_schema.insert("oneOf".to_string(), json!(class_refs));
            }
        }

        serde_json::to_string_pretty(&json_schema).map_err(|e| {
            LinkMLError::ServiceError(format!("Failed to serialize JSON SchemaDefinition: {e}"))
        })
    }

    /// Convert `LinkML` type to JSON `SchemaDefinition`
    fn type_to_json_schema(&self, type_def: &TypeDefinition) -> Result<Value, LinkMLError> {
        let mut schema = Map::new();

        if let Some(description) = &type_def.description
            && self.config.include_docs {
                schema.insert("description".to_string(), json!(description));
            }

        // Base type mapping
        match type_def.base_type.as_deref() {
            Some("string") => {
                schema.insert("type".to_string(), json!("string"));
                if let Some(pattern) = &type_def.pattern {
                    schema.insert("pattern".to_string(), json!(pattern));
                }
                if let Some(min_length) = &type_def.minimum_value {
                    schema.insert("minLength".to_string(), json!(min_length));
                }
                if let Some(max_length) = &type_def.maximum_value {
                    schema.insert("maxLength".to_string(), json!(max_length));
                }
            }
            Some("integer") => {
                schema.insert("type".to_string(), json!("integer"));
                if let Some(min) = &type_def.minimum_value {
                    schema.insert("minimum".to_string(), json!(min));
                }
                if let Some(max) = &type_def.maximum_value {
                    schema.insert("maximum".to_string(), json!(max));
                }
            }
            Some("float" | "double") => {
                schema.insert("type".to_string(), json!("number"));
                if let Some(min) = &type_def.minimum_value {
                    schema.insert("minimum".to_string(), json!(min));
                }
                if let Some(max) = &type_def.maximum_value {
                    schema.insert("maximum".to_string(), json!(max));
                }
            }
            Some("boolean") => {
                schema.insert("type".to_string(), json!("boolean"));
            }
            Some("date") => {
                schema.insert("type".to_string(), json!("string"));
                schema.insert("format".to_string(), json!("date"));
            }
            Some("datetime") => {
                schema.insert("type".to_string(), json!("string"));
                schema.insert("format".to_string(), json!("date-time"));
            }
            Some("uri") => {
                schema.insert("type".to_string(), json!("string"));
                schema.insert("format".to_string(), json!("uri"));
            }
            _ => {
                schema.insert("type".to_string(), json!("string"));
            }
        }

        Ok(Value::Object(schema))
    }

    /// Convert `LinkML` enum to JSON `SchemaDefinition`
    fn enum_to_json_schema(&self, enum_def: &EnumDefinition) -> Result<Value, LinkMLError> {
        let mut schema = Map::new();

        if let Some(description) = &enum_def.description
            && self.config.include_docs {
                schema.insert("description".to_string(), json!(description));
            }

        if !enum_def.permissible_values.is_empty() {
            let enum_values: Vec<String> = enum_def
                .permissible_values
                .iter()
                .map(|pv| match pv {
                    PermissibleValue::Simple(s) => s.clone(),
                    PermissibleValue::Complex { text, .. } => text.clone()})
                .collect();
            schema.insert("enum".to_string(), json!(enum_values));

            if self.config.custom_error_messages {
                let error_msg = format!("Must be one of: {}", enum_values.join(", "));
                schema.insert("errorMessage".to_string(), json!(error_msg));
            }
        }

        Ok(Value::Object(schema))
    }

    /// Convert `LinkML` class to JSON `SchemaDefinition`
    fn class_to_json_schema(
        &self,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Result<Value, LinkMLError> {
        let mut json_schema = Map::new();

        json_schema.insert("type".to_string(), json!("object"));

        if let Some(description) = &class_def.description
            && self.config.include_docs {
                json_schema.insert("description".to_string(), json!(description));
            }

        let mut properties = Map::new();
        let mut required = Vec::new();

        // Process slots
        if !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    properties.insert(
                        slot_name.clone(),
                        self.slot_to_json_schema(slot_def, schema)?,
                    );
                    if slot_def.required == Some(true) {
                        required.push(slot_name.clone());
                    }
                }
            }
        }

        // Process attributes
        if !class_def.attributes.is_empty() {
            for (attr_name, attr_def) in &class_def.attributes {
                properties.insert(
                    attr_name.clone(),
                    self.slot_to_json_schema(attr_def, schema)?,
                );
                if attr_def.required == Some(true) {
                    required.push(attr_name.clone());
                }
            }
        }

        json_schema.insert("properties".to_string(), Value::Object(properties));

        if !required.is_empty() {
            json_schema.insert("required".to_string(), json!(required));
        }

        if self.config.strict_mode {
            json_schema.insert("additionalProperties".to_string(), json!(false));
        }

        Ok(Value::Object(json_schema))
    }

    /// Convert `LinkML` slot to JSON `SchemaDefinition`
    fn slot_to_json_schema(
        &self,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> Result<Value, LinkMLError> {
        let mut json_schema = Map::new();

        if let Some(description) = &slot_def.description
            && self.config.include_docs {
                json_schema.insert("description".to_string(), json!(description));
            }

        // Handle multivalued slots
        if slot_def.multivalued == Some(true) {
            let mut array_schema = Map::new();
            array_schema.insert("type".to_string(), json!("array"));

            let item_schema = self.get_range_schema(slot_def, schema)?;
            array_schema.insert("items".to_string(), item_schema);

            // Cardinality constraints are enforced via required/multivalued flags
            // if let Some(min) = slot_def.minimum_cardinality {
            //     array_schema.insert("minItems".to_string(), json!(min));
            // }
            // if let Some(max) = slot_def.maximum_cardinality {
            //     array_schema.insert("maxItems".to_string(), json!(max));
            // }

            return Ok(Value::Object(array_schema));
        }

        // Single-valued slot
        self.get_range_schema(slot_def, schema)
    }

    /// Get `JSON` `SchemaDefinition` for slot range
    fn get_range_schema(
        &self,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> Result<Value, LinkMLError> {
        if let Some(range) = &slot_def.range {
            // Check if it's a type
            if !schema.types.is_empty() && schema.types.contains_key(range) {
                return Ok(json!({ "$ref": format!("#/definitions/{range}") }));
            }

            // Check if it's an enum
            if !schema.enums.is_empty() && schema.enums.contains_key(range) {
                return Ok(json!({ "$ref": format!("#/definitions/{range}") }));
            }

            // Check if it's a class
            if schema.classes.contains_key(range) {
                return Ok(json!({ "$ref": format!("#/definitions/{range}") }));
            }

            // Built-in types
            match range.as_str() {
                "string" => Ok(json!({ "type": "string" })),
                "integer" | "int" => Ok(json!({ "type": "integer" })),
                "float" | "double" => Ok(json!({ "type": "number" })),
                "boolean" | "bool" => Ok(json!({ "type": "boolean" })),
                "date" => Ok(json!({ "type": "string", "format": "date" })),
                "datetime" => Ok(json!({ "type": "string", "format": "date-time" })),
                "uri" => Ok(json!({ "type": "string", "format": "uri" })),
                _ => Ok(json!({ "type": "string" }))}
        } else {
            Ok(json!({ "type": "string" }))
        }
    }

    /// Generate Cerberus validation rules (Python)
    fn generate_cerberus(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut output = String::new();

        output.push_str("# Cerberus validation schema generated from LinkML\n\n");
        output.push_str("from cerberus import Validator\n\n");

        // Generate schemas for each class
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                writeln!(output, "# Validation schema for {class_name}").unwrap();
                output.push_str(&format!("{}_SCHEMA = {{\n", class_name.to_uppercase()));

                // Process slots
                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        output.push_str(&self.slot_to_cerberus(slot_name, slot_def, schema)?);
                    }
                }

                // Process attributes
                for (attr_name, attr_def) in &class_def.attributes {
                    output.push_str(&self.slot_to_cerberus(attr_name, attr_def, schema)?);
                }

                output.push_str("}\n\n");
            }
        }

        // Generate validator functions
        output.push_str("# Validator instances\n");
        for class_name in schema.classes.keys() {
            output.push_str(&format!(
                "{}_validator = Validator({}_SCHEMA)\n",
                class_name.to_lowercase(),
                class_name.to_uppercase()
            ));
        }

        Ok(output)
    }

    /// Convert slot to Cerberus rule
    fn slot_to_cerberus(
        &self,
        name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        let mut rule = format!("    '{name}': {{\n");

        // Type
        if let Some(range) = &slot_def.range {
            let cerberus_type = self.range_to_cerberus_type(range, schema);
            writeln!(rule, "        'type': '{cerberus_type}',").unwrap();
        }

        // Required
        if slot_def.required == Some(true) {
            rule.push_str("        'required': True,\n");
        }

        // Pattern
        if let Some(pattern) = &slot_def.pattern {
            writeln!(rule, "        'regex': r'{pattern}',").unwrap();
        }

        // Min/max values
        if let Some(min) = &slot_def.minimum_value {
            writeln!(rule, "        'min': {min},").unwrap();
        }
        if let Some(max) = &slot_def.maximum_value {
            writeln!(rule, "        'max': {max},").unwrap();
        }

        rule.push_str("    },\n");
        Ok(rule)
    }

    /// Convert range to Cerberus type
    fn range_to_cerberus_type(&self, range: &str, _schema: &SchemaDefinition) -> &'static str {
        match range {
            "string" => "string",
            "integer" | "int" => "integer",
            "float" | "double" => "float",
            "boolean" | "bool" => "boolean",
            "date" | "datetime" => "datetime",
            _ => "string"}
    }

    /// Generate Joi validation schema (JavaScript)
    fn generate_joi(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut output = String::new();

        output.push_str("// Joi validation schema generated from LinkML\n\n");
        output.push_str("const Joi = require('joi');\n\n");

        // Generate schemas for each class
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                writeln!(output, "// Validation schema for {class_name}").unwrap();
                output.push_str(&format!(
                    "const {}SchemaDefinition = Joi.object({{\n",
                    self.to_camel_case(class_name)
                ));

                // Process all slots
                let mut slot_rules = Vec::new();

                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        slot_rules.push(self.slot_to_joi(slot_name, slot_def, schema));
                    }
                }

                for (attr_name, attr_def) in &class_def.attributes {
                    slot_rules.push(self.slot_to_joi(attr_name, attr_def, schema));
                }

                output.push_str(&slot_rules.join(",\n"));
                output.push_str("\n})");

                if self.config.strict_mode {
                    output.push_str(".options({ allowUnknown: false })");
                }

                output.push_str(";\n\n");
            }
        }

        // Export schemas
        output.push_str("module.exports = {\n");
        let exports: Vec<String> = schema
            .classes
            .keys()
            .map(|name| format!("  {}SchemaDefinition", self.to_camel_case(name)))
            .collect();
        output.push_str(&exports.join(",\n"));
        output.push_str("\n};\n");

        Ok(output)
    }

    /// Convert slot to Joi rule
    fn slot_to_joi(
        &self,
        name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> String {

        let mut rule = format!("  {name}: ");

        // Base type
        if let Some(range) = &slot_def.range {
            rule.push_str(&self.range_to_joi_type(range, schema));
        } else {
            rule.push_str("Joi.string()");
        }

        // Required
        if slot_def.required == Some(true) {
            rule.push_str(".required()");
        }

        // Pattern
        if let Some(pattern) = &slot_def.pattern {
            rule.push_str(&format!(".pattern(/{pattern}/)"));
        }

        // Description
        if let Some(description) = &slot_def.description
            && self.config.include_docs {
                rule.push_str(&format!(
                    ".description('{}')",
                    description.replace('\'', "\\'")
                ));
            }

        rule

    }

    /// Convert range to Joi type
    fn range_to_joi_type(&self, range: &str, _schema: &SchemaDefinition) -> String {
        match range {
            "string" => "Joi.string()".to_string(),
            "integer" | "int" => "Joi.number().integer()".to_string(),
            "float" | "double" => "Joi.number()".to_string(),
            "boolean" | "bool" => "Joi.boolean()".to_string(),
            "date" | "datetime" => "Joi.date()".to_string(),
            "uri" => "Joi.string().uri()".to_string(),
            _ => "Joi.any()".to_string()}
    }

    /// Generate Yup validation schema (JavaScript)
    fn generate_yup(&self, schema: &SchemaDefinition) -> String {

        let mut output = String::new();

        output.push_str("// Yup validation schema generated from LinkML\n\n");
        output.push_str("import * as yup from 'yup';\n\n");

        // Generate schemas for each class
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                writeln!(output, "// Validation schema for {class_name}").unwrap();
                output.push_str(&format!(
                    "export const {}SchemaDefinition = yup.object({{\n",
                    self.to_camel_case(class_name)
                ));

                // Process all slots
                let mut slot_rules = Vec::new();

                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        slot_rules.push(self.slot_to_yup(slot_name, slot_def, schema));
                    }
                }

                for (attr_name, attr_def) in &class_def.attributes {
                    slot_rules.push(self.slot_to_yup(attr_name, attr_def, schema));
                }

                output.push_str(&slot_rules.join(",\n"));
                output.push_str("\n})");

                if self.config.strict_mode {
                    output.push_str(".noUnknown()");
                }

                output.push_str(";\n\n");
            }
        }

        output

    }

    /// Convert slot to Yup rule
    fn slot_to_yup(
        &self,
        name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> String {
        let mut rule = format!("  {name}: ");

        // Base type
        if let Some(range) = &slot_def.range {
            rule.push_str(&self.range_to_yup_type(range, schema));
        } else {
            rule.push_str("yup.string()");
        }

        // Required
        if slot_def.required == Some(true) {
            rule.push_str(".required()");
        } else {
            rule.push_str(".nullable()");
        }

        // Pattern
        if let Some(pattern) = &slot_def.pattern {
            rule.push_str(&format!(".matches(/{pattern}/)"));
        }

        // Custom error message
        if self.config.custom_error_messages
            && let Some(_description) = &slot_def.description {
                rule.push_str(&format!(".label('{name}')"));
            }

        rule

    }

    /// Convert range to Yup type
    fn range_to_yup_type(&self, range: &str, _schema: &SchemaDefinition) -> String {
        match range {
            "string" => "yup.string()".to_string(),
            "integer" | "int" => "yup.number().integer()".to_string(),
            "float" | "double" => "yup.number()".to_string(),
            "boolean" | "bool" => "yup.boolean()".to_string(),
            "date" | "datetime" => "yup.date()".to_string(),
            "uri" => "yup.string().url()".to_string(),
            _ => "yup.mixed()".to_string()}
    }

    /// Generate Open`API` validation specification
    fn generate_openapi(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut openapi = Map::new();

        // OpenAPI metadata
        openapi.insert("openapi".to_string(), json!("3.0.3"));

        let mut info = Map::new();
        if !schema.name.is_empty() {
            info.insert("title".to_string(), json!(&schema.name));
        }
        if let Some(description) = &schema.description {
            info.insert("description".to_string(), json!(description));
        }
        info.insert("version".to_string(), json!("1.0.0"));
        openapi.insert("info".to_string(), Value::Object(info));

        // Components with schemas
        let mut components = Map::new();
        let mut schemas = Map::new();

        // Add all classes as schemas
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                schemas.insert(
                    class_name.clone(),
                    self.class_to_openapi(class_def, schema)?,
                );
            }
        }

        components.insert("schemas".to_string(), Value::Object(schemas));
        openapi.insert("components".to_string(), Value::Object(components));

        // Minimal paths (required by OpenAPI)
        openapi.insert("paths".to_string(), json!({}));

        serde_json::to_string_pretty(&openapi)
            .map_err(|e| LinkMLError::ServiceError(format!("Failed to serialize OpenAPI: {e}")))
    }

    /// Convert class to Open`API` schema
    fn class_to_openapi(
        &self,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Result<Value, LinkMLError> {
        // Reuse JSON SchemaDefinition generation
        self.class_to_json_schema(class_def, schema)
    }

    /// Convert to camelCase
    fn to_camel_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for (i, ch) in s.chars().enumerate() {
            if ch == '_' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(
                    ch.to_uppercase()
                        .next()
                        .unwrap_or(ch),
                );
                capitalize_next = false;
            } else if i == 0 {
                result.push(
                    ch.to_lowercase()
                        .next()
                        .unwrap_or(ch),
                );
            } else {
                result.push(ch);
            }
        }

        result
    }
}

impl Generator for YamlValidatorGenerator {
    fn name(&self) -> &'static str {
        "yaml-validator"
    }

    fn description(&self) -> &'static str {
        "Generate YAML validators from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for yamlvalidator generation"
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        self.generate_validation(schema)
    }

    fn get_file_extension(&self) -> &str {
        match self.config.framework {
            ValidationFramework::JsonSchemaDefinition => "json",
            ValidationFramework::Cerberus => "py",
            ValidationFramework::Joi | ValidationFramework::Yup => "js",
            ValidationFramework::OpenAPI => "yaml"}
    }

    fn get_default_filename(&self) -> &str {
        match self.config.framework {
            ValidationFramework::JsonSchemaDefinition => "schema",
            ValidationFramework::Cerberus => "validation",
            ValidationFramework::Joi => "joi_schema",
            ValidationFramework::Yup => "yup_schema",
            ValidationFramework::OpenAPI => "openapi"}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};

    #[test]
    fn test_yaml_validator_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchemaDefinition".to_string();

        // Add a simple class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());

        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);

        person_class
            .attributes
            .insert("name".to_string(), name_slot);

        schema.classes.insert("Person".to_string(), person_class);

        // Test JSON SchemaDefinition generation
        let config = YamlValidatorGeneratorConfig::default();
        let generator = YamlValidatorGenerator::new(config);
        let result = generator
            .generate(&schema)
            .expect("should generate YAML validator: {}");

        assert!(result.contains("$schema"));
        assert!(result.contains("definitions"));
        assert!(result.contains("Person"));
        Ok(())
    }
}
