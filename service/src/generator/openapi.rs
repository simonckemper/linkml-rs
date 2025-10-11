//! `OpenAPI` schema generation for `LinkML` schemas

use super::options::IndentStyle;
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};
use linkml_core::{error::LinkMLError, prelude::*};
use regex;
use serde::Serialize;
use serde_json::{Value as JsonValue, json};

/// `OpenAPI` schema generator for `LinkML` schemas
pub struct OpenApiGenerator {
    /// Generator name
    name: String,
    /// Generator options (stub for future configuration)
    options: super::traits::GeneratorOptions,
}

impl OpenApiGenerator {
    /// Create a new `OpenAPI` generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "openapi".to_string(),
            options: super::traits::GeneratorOptions::default(),
        }
    }
    /// Create a new `OpenAPI` generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        Self {
            name: "openapi".to_string(),
            options,
        }
    }

    /// Get indentation string based on options
    fn get_indent(&self) -> String {
        self.options.indent.single()
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Generate `OpenAPI` schema component for a class
    fn generate_class_component(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        schemas: &mut serde_json::Map<String, JsonValue>,
    ) -> GeneratorResult<()> {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Add ID field for non-abstract classes
        if class.abstract_ != Some(true) {
            properties.insert(
                "id".to_string(),
                json!({
                    "type": "string",
                    "format": "uuid",
                    "description": "Unique identifier",
                    "readOnly": true
                }),
            );
        }

        // Collect all slots including inherited ones
        let slots = self.collect_all_slots(class, schema);

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
            "properties": properties});

        // Add description
        if let Some(desc) = &class.description {
            schema_obj["description"] = json!(desc);
        }

        // Add additional metadata if docs are enabled
        if self.options.include_docs {
            if let Some(author) = self.get_custom_option("author") {
                schema_obj["x-author"] = json!(author);
            }
            if let Some(version) = self.get_custom_option("version") {
                schema_obj["x-version"] = json!(version);
            }
        }

        // Add required array if not empty
        if !required.is_empty() {
            schema_obj["required"] = json!(required);
        }

        // Handle inheritance using allOf
        if let Some(parent) = &class.is_a {
            let parent_ref = json!({
                "$ref": format!("#/components/schemas/{parent}")
            });

            schema_obj = json!({
                "allOf": [parent_ref, schema_obj]
            });
        }

        schemas.insert(class_name.to_string(), schema_obj);

        // Generate request/response schemas
        if class.abstract_ != Some(true) {
            // Create request schema (without id)
            let mut create_properties = properties.clone();
            create_properties.remove("id");

            schemas.insert(
                format!("{class_name}CreateRequest"),
                json!({
                    "type": "object",
                    "properties": create_properties,
                    "required": required.clone()
                }),
            );

            // Update request schema (partial update)
            schemas.insert(
                format!("{class_name}UpdateRequest"),
                json!({
                    "type": "object",
                    "properties": create_properties,
                    "minProperties": 1
                }),
            );

            // List response
            schemas.insert(
                format!("{class_name}ListResponse"),
                json!({
                    "type": "object",
                    "properties": {
                        "items": {
                            "type": "array",
                            "items": {
                                "$ref": format!("#/components/schemas/{class_name}")
                            }
                        },
                        "total": {
                            "type": "integer",
                            "description": "Total number of items"
                        },
                        "page": {
                            "type": "integer",
                            "description": "Current page number"
                        },
                        "pageSize": {
                            "type": "integer",
                            "description": "Number of items per page"
                        }
                    },
                    "required": ["items", "total", "page", "pageSize"]
                }),
            );
        }

        Ok(())
    }

    /// Generate property schema for `OpenAPI`
    fn generate_property_schema(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<JsonValue> {
        // Validate slot has a range
        if slot.range.is_none() {
            return Err(GeneratorError::Generation(format!(
                "Slot '{}' must have a range specified",
                slot.name
            )));
        }

        let base_schema = Self::get_base_type_schema(slot.range.as_ref(), schema);

        let mut property = if slot.multivalued == Some(true) {
            json!({
                "type": "array",
                "items": base_schema
            })
        } else {
            base_schema
        };

        // Add constraints and metadata
        if let Some(desc) = &slot.description {
            property["description"] = json!(desc);
        }

        // Add examples if enabled in options
        if self.options.include_docs
            && let Some(example) = self.get_custom_option(&format!("{}_example", slot.name))
        {
            property["example"] = json!(example);
        }

        if let Some(pattern) = &slot.pattern {
            // Validate pattern is a valid regex for OpenAPI
            regex::Regex::new(pattern).map_err(|e| {
                GeneratorError::Generation(format!(
                    "Invalid regex pattern '{pattern}' for OpenAPI: {e}"
                ))
            })?;
            property["pattern"] = json!(pattern);
        }

        if let Some(min) = &slot.minimum_value {
            property["minimum"] = json!(min);
        }

        if let Some(max) = &slot.maximum_value {
            property["maximum"] = json!(max);
        }

        if slot.identifier == Some(true) {
            property["readOnly"] = json!(true);
        }

        Ok(property)
    }

    /// Get base type schema for `OpenAPI`
    fn get_base_type_schema(range: Option<&String>, schema: &SchemaDefinition) -> JsonValue {
        match range.map(String::as_str) {
            Some("string" | "str") | None => json!({"type": "string"}),
            Some("integer" | "int") => json!({"type": "integer", "format": "int64"}),
            Some("float" | "double") => json!({"type": "number", "format": "double"}),
            Some("decimal") => json!({"type": "string", "format": "decimal"}),
            Some("boolean" | "bool") => json!({"type": "boolean"}),
            Some("date") => json!({
                "type": "string",
                "format": "date"
            }),
            Some("datetime") => json!({
                "type": "string",
                "format": "date-time"
            }),
            Some("uri" | "url") => json!({
                "type": "string",
                "format": "uri"
            }),
            Some(other) => {
                // Check if it's an enum or class reference
                if schema.enums.contains_key(other) || schema.classes.contains_key(other) {
                    json!({
                        "$ref": format!("#/components/schemas/{other}")
                    })
                } else {
                    json!({"type": "string"})
                }
            }
        }
    }

    /// Generate enum schema for `OpenAPI`
    fn generate_enum_component(
        enum_name: &str,
        enum_def: &EnumDefinition,
        schemas: &mut serde_json::Map<String, JsonValue>,
    ) {
        let values: Vec<String> = enum_def
            .permissible_values
            .iter()
            .map(|v| match v {
                PermissibleValue::Simple(text) | PermissibleValue::Complex { text, .. } => {
                    text.clone()
                }
            })
            .collect();

        let mut schema = json!({
            "type": "string",
            "enum": values
        });

        if let Some(desc) = &enum_def.description {
            schema["description"] = json!(desc);
        }

        schemas.insert(enum_name.to_string(), schema);
    }

    /// Generate paths for REST `API` operations
    fn generate_paths(schema: &SchemaDefinition) -> serde_json::Map<String, JsonValue> {
        let mut paths = serde_json::Map::new();

        for (class_name, class) in &schema.classes {
            // Skip abstract classes
            if class.abstract_ == Some(true) {
                continue;
            }

            let resource_name = Self::to_plural_kebab_case(class_name);
            let base_path = format!("/{resource_name}");
            let item_path = format!("/{resource_name}/{{id}}");

            // Collection operations
            let mut collection_ops = serde_json::Map::new();

            // GET list
            collection_ops.insert("get".to_string(), json!({
                "summary": format!("List {resource_name}"),
                "operationId": format!("list{class_name}"),
                "tags": [class_name],
                "parameters": [
                    {
                        "name": "page",
                        "in": "query",
                        "schema": {"type": "integer", "default": 1},
                        "description": "Page number"
                    },
                    {
                        "name": "pageSize",
                        "in": "query",
                        "schema": {"type": "integer", "default": 20, "maximum": 100},
                        "description": "Number of items per page"
                    },
                    {
                        "name": "sort",
                        "in": "query",
                        "schema": {"type": "string"},
                        "description": "Sort field"
                    },
                    {
                        "name": "order",
                        "in": "query",
                        "schema": {"type": "string", "enum": ["asc", "desc"], "default": "asc"},
                        "description": "Sort order"
                    }
                ],
                "responses": {
                    "200": {
                        "description": "Successful response",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{class_name}ListResponse")
                                }
                            }
                        }
                    }
                }
            }));

            // POST create
            collection_ops.insert("post".to_string(), json!({
                "summary": format!("Create a new {}", class_name.to_lowercase()),
                "operationId": format!("create{class_name}"),
                "tags": [class_name],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": format!("#/components/schemas/{class_name}CreateRequest")
                            }
                        }
                    }
                },
                "responses": {
                    "201": {
                        "description": "Created",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{class_name}")
                                }
                            }
                        }
                    },
                    "400": {
                        "$ref": "#/components/responses/BadRequest"
                    }
                }
            }));

            paths.insert(base_path, json!(collection_ops));

            // Item operations
            let mut item_ops = serde_json::Map::new();

            // GET by id
            item_ops.insert(
                "get".to_string(),
                json!({
                    "summary": format!("Get {} by ID", class_name.to_lowercase()),
                    "operationId": format!("get{class_name}ById"),
                    "tags": [class_name],
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "string", "format": "uuid"}
                        }
                    ],
                    "responses": {
                        "200": {
                            "description": "Successful response",
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "$ref": format!("#/components/schemas/{class_name}")
                                    }
                                }
                            }
                        },
                        "404": {
                            "$ref": "#/components/responses/NotFound"
                        }
                    }
                }),
            );

            // PUT update
            item_ops.insert("put".to_string(), json!({
                "summary": format!("Update {}", class_name.to_lowercase()),
                "operationId": format!("update{class_name}"),
                "tags": [class_name],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": {"type": "string", "format": "uuid"}
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": format!("#/components/schemas/{class_name}CreateRequest")
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Updated",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{class_name}")
                                }
                            }
                        }
                    },
                    "400": {
                        "$ref": "#/components/responses/BadRequest"
                    },
                    "404": {
                        "$ref": "#/components/responses/NotFound"
                    }
                }
            }));

            // PATCH partial update
            item_ops.insert("patch".to_string(), json!({
                "summary": format!("Partially update {}", class_name.to_lowercase()),
                "operationId": format!("patch{class_name}"),
                "tags": [class_name],
                "parameters": [
                    {
                        "name": "id",
                        "in": "path",
                        "required": true,
                        "schema": {"type": "string", "format": "uuid"}
                    }
                ],
                "requestBody": {
                    "required": true,
                    "content": {
                        "application/json": {
                            "schema": {
                                "$ref": format!("#/components/schemas/{class_name}UpdateRequest")
                            }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": "Updated",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "$ref": format!("#/components/schemas/{class_name}")
                                }
                            }
                        }
                    },
                    "400": {
                        "$ref": "#/components/responses/BadRequest"
                    },
                    "404": {
                        "$ref": "#/components/responses/NotFound"
                    }
                }
            }));

            // DELETE
            item_ops.insert(
                "delete".to_string(),
                json!({
                    "summary": format!("Delete {}", class_name.to_lowercase()),
                    "operationId": format!("delete{class_name}"),
                    "tags": [class_name],
                    "parameters": [
                        {
                            "name": "id",
                            "in": "path",
                            "required": true,
                            "schema": {"type": "string", "format": "uuid"}
                        }
                    ],
                    "responses": {
                        "204": {
                            "description": "Deleted"
                        },
                        "404": {
                            "$ref": "#/components/responses/NotFound"
                        }
                    }
                }),
            );

            paths.insert(item_path, json!(item_ops));
        }

        paths
    }

    /// Convert to plural kebab-case
    fn to_plural_kebab_case(s: &str) -> String {
        let kebab = s
            .chars()
            .enumerate()
            .map(|(i, c)| {
                if c.is_uppercase() && i > 0 {
                    format!("-{}", c.to_lowercase())
                } else {
                    c.to_lowercase().to_string()
                }
            })
            .collect::<String>();

        // Simple pluralization
        if kebab.ends_with('s') {
            format!("{kebab}es")
        } else if kebab.ends_with('y') {
            format!("{}ies", &kebab[..kebab.len() - 1])
        } else {
            format!("{kebab}s")
        }
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Add direct slots
        for slot in &class.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add inherited slots
        if let Some(parent) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            let parent_slots = self.collect_all_slots(parent_class, schema);
            for slot in parent_slots {
                if seen.insert(slot.clone()) {
                    all_slots.push(slot);
                }
            }
        }

        all_slots
    }
}

impl Default for OpenApiGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for OpenApiGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate OpenAPI 3.0 specification from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".openapi.json", ".openapi.yaml"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for OpenAPI generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Validate schema
        self.validate_schema(schema)?;

        let mut components = serde_json::Map::new();
        let mut schemas = serde_json::Map::new();

        // Generate enum components
        for (enum_name, enum_def) in &schema.enums {
            Self::generate_enum_component(enum_name, enum_def, &mut schemas);
        }

        // Generate class components
        for (class_name, class) in &schema.classes {
            self.generate_class_component(class_name, class, schema, &mut schemas)?;
        }

        // Add common response schemas
        let responses = json!({
            "BadRequest": {
                "description": "Bad Request",
                "content": {
                    "application/json": {
                        "schema": {
                            "$ref": "#/components/schemas/Error"
                        }
                    }
                }
            },
            "NotFound": {
                "description": "Not Found",
                "content": {
                    "application/json": {
                        "schema": {
                            "$ref": "#/components/schemas/Error"
                        }
                    }
                }
            }
        });

        // Add error schema
        schemas.insert(
            "Error".to_string(),
            json!({
                "type": "object",
                "properties": {
                    "code": {
                        "type": "string",
                        "description": "Error code"
                    },
                    "message": {
                        "type": "string",
                        "description": "Error message"
                    },
                    "details": {
                        "type": "object",
                        "description": "Additional error details"
                    }
                },
                "required": ["code", "message"]
            }),
        );

        components.insert("schemas".to_string(), json!(schemas));
        components.insert("responses".to_string(), responses);

        // Generate paths
        let paths = Self::generate_paths(schema);

        // Build OpenAPI document
        let mut openapi = json!({
            "openapi": "3.0.3",
            "info": {
                "title": schema.name.clone(),
                "version": schema.version.as_deref().unwrap_or("1.0.0")},
            "paths": paths,
            "components": components
        });

        if let Some(desc) = &schema.description {
            openapi["info"]["description"] = json!(desc);
        }

        // Add additional OpenAPI metadata if options are set
        if self.options.include_docs {
            if let Some(contact) = self.get_custom_option("contact") {
                openapi["info"]["contact"] = json!({"name": contact});
            }
            if let Some(license) = self.get_custom_option("license") {
                openapi["info"]["license"] = json!({"name": license});
            }
            if let Some(terms) = self.get_custom_option("terms_of_service") {
                openapi["info"]["termsOfService"] = json!(terms);
            }
        }

        // Format output according to options
        let content = match self.options.output_format {
            super::traits::OutputFormat::JSON => {
                if self.options.indent == super::traits::IndentStyle::Tabs {
                    // Custom formatting for tabs
                    let pretty = serde_json::to_string_pretty(&openapi)
                        .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?;
                    pretty.replace("    ", "\t")
                } else if let super::traits::IndentStyle::Spaces(n) = self.options.indent {
                    // Custom spacing
                    let spaces = vec![b' '; n];
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(&spaces);
                    let mut buf = Vec::new();
                    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                    openapi
                        .serialize(&mut ser)
                        .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?;
                    String::from_utf8(buf)
                        .map_err(|e| LinkMLError::service(format!("Invalid UTF-8 in JSON: {e}")))?
                } else {
                    serde_json::to_string_pretty(&openapi)
                        .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?
                }
            }
            _ => {
                // Default formatting for other output formats
                serde_json::to_string_pretty(&openapi)
                    .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?
            }
        };

        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "json"
    }

    fn get_default_filename(&self) -> &'static str {
        "openapi"
    }
}

impl CodeFormatter for OpenApiGenerator {
    fn name(&self) -> &'static str {
        "openapi"
    }

    fn description(&self) -> &'static str {
        "Code formatter for openapi output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml", "json"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // Basic formatting - use options for indentation
        let mut formatted = String::new();
        let indent = self.get_indent();
        let mut indent_level: usize = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add proper indentation
            formatted.push_str(&indent.repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after opening braces
            if trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(') {
                indent_level += 1;
            }
        }

        Ok(formatted)
    }
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
        s.to_string()
    }

    fn convert_identifier(&self, id: &str) -> String {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[test]
    fn test_openapi_generation() -> anyhow::Result<()> {
        let generator = OpenApiGenerator::new();

        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "Test API".to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        // Add a class
        let class = ClassDefinition {
            name: "User".to_string(),
            description: Some("User account".to_string()),
            ..Default::default()
        };

        schema.classes.insert("User".to_string(), class);

        let content = generator
            .generate(&schema)
            .expect("should generate OpenAPI: {}");

        // Parse to verify it's valid JSON
        let parsed: JsonValue =
            serde_json::from_str(&content).expect("should parse as valid JSON: {}");

        // Check basic structure
        assert_eq!(parsed["openapi"], "3.0.3");
        assert_eq!(parsed["info"]["title"], "Test API");
        assert_eq!(parsed["info"]["version"], "1.0.0");

        // Check paths
        assert!(parsed["paths"]["/users"].is_object());
        assert!(parsed["paths"]["/users/{id}"].is_object());

        // Check components
        assert!(parsed["components"]["schemas"]["User"].is_object());
        assert!(parsed["components"]["schemas"]["UserCreateRequest"].is_object());
        Ok(())
    }
}
