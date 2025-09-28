//! JSON-LD generator for `LinkML` schemas
//!
//! This module generates JSON-LD (JSON for Linked Data) contexts and schemas from `LinkML` schemas.
//! JSON-LD is a W3C standard for representing linked data in JSON format.

use linkml_core::{
    error::LinkMLError,
    types::{ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition},
};
use serde_json::{Value as JsonValue, json};

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult};

/// `JSON`-LD generator for linked data contexts
pub struct JsonLdGenerator {
    options: GeneratorOptions,
}

impl JsonLdGenerator {
    /// Create a new `JSON`-LD generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: GeneratorOptions::default(),
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        Self { options }
    }

    /// Generate `JSON`-LD context
    fn generate_context(&self, schema: &SchemaDefinition) -> JsonValue {
        let mut context = serde_json::Map::new();

        // Standard prefixes (skip if format is compact)
        let use_compact = self
            .options
            .custom
            .get("compact")
            .is_some_and(|v| v == "true");

        if use_compact {
            // Compact output - only required prefixes
            context.insert("@vocab".to_string(), json!(format!("{}#", schema.id)));
        } else {
            context.insert("@vocab".to_string(), json!(format!("{}#", schema.id)));
            context.insert(
                "xsd".to_string(),
                json!("http://www.w3.org/2001/XMLSchema#"),
            );
            context.insert(
                "rdf".to_string(),
                json!("http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
            );
            context.insert(
                "rdfs".to_string(),
                json!("http://www.w3.org/2000/01/rdf-schema#"),
            );
            context.insert("owl".to_string(), json!("http://www.w3.org/2002/07/owl#"));
            context.insert(
                "skos".to_string(),
                json!("http://www.w3.org/2004/02/skos/core#"),
            );
            context.insert("dcterms".to_string(), json!("http://purl.org/dc/terms/"));
        }

        // Add schema name as prefix
        let schema_prefix = Self::to_snake_case(&schema.name);
        context.insert(schema_prefix.clone(), json!(format!("{}#", schema.id)));

        // Add classes
        for (class_name, _) in &schema.classes {
            let class_id = Self::to_pascal_case(class_name);
            context.insert(
                class_id.clone(),
                json!(format!("{}:{}", schema_prefix, class_id)),
            );
        }

        // Add properties (slots)
        for (slot_name, slot) in &schema.slots {
            // Skip internal slots if configured to exclude them
            let exclude_internal = self
                .options
                .custom
                .get("exclude_internal")
                .is_some_and(|v| v == "true");
            if exclude_internal && slot_name.starts_with('_') {
                continue;
            }

            let mut slot_def = serde_json::Map::new();

            let property_id = Self::to_snake_case(slot_name);
            slot_def.insert(
                "@id".to_string(),
                json!(format!("{}:{}", schema_prefix, property_id)),
            );

            // Add type mapping
            if let Some(range) = &slot.range {
                if let Some(json_ld_type) = self.get_json_ld_type(range, schema) {
                    slot_def.insert("@type".to_string(), json!(json_ld_type));
                }

                // Handle object references
                if schema.classes.contains_key(range) || schema.enums.contains_key(range) {
                    slot_def.insert("@type".to_string(), json!("@id"));
                }
            }

            // Handle multivalued fields
            if slot.multivalued == Some(true) {
                slot_def.insert("@container".to_string(), json!("@set"));
            }

            context.insert(property_id, json!(slot_def));
        }

        // Add enums
        for (enum_name, _) in &schema.enums {
            let enum_id = Self::to_pascal_case(enum_name);
            context.insert(
                enum_id.clone(),
                json!(format!("{}:{}", schema_prefix, enum_id)),
            );
        }

        json!(context)
    }

    /// Generate frame for a class
    fn generate_frame(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> JsonValue {
        let mut frame = serde_json::Map::new();

        // Add type
        let schema_prefix = Self::to_snake_case(&schema.name);
        frame.insert(
            "@type".to_string(),
            json!(format!(
                "{}:{}",
                schema_prefix,
                Self::to_pascal_case(class_name)
            )),
        );

        // Collect all slots (including inherited)
        let all_slots = self.collect_all_slots(class, schema);

        // Add slot defaults/examples
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let property_id = Self::to_snake_case(slot_name);

                // Add frame hints
                let mut slot_frame = serde_json::Map::new();

                if slot.required == Some(true) {
                    slot_frame.insert("@default".to_string(), json!(null));
                }

                if !slot_frame.is_empty() {
                    frame.insert(property_id, json!(slot_frame));
                }
            }
        }

        json!(frame)
    }

    /// Generate `JSON`-LD schema document
    fn generate_schema_document(&self, schema: &SchemaDefinition) -> JsonValue {
        let mut doc = serde_json::Map::new();

        // Add context
        doc.insert("@context".to_string(), self.generate_context(schema));

        // Add graph with schema metadata
        let mut graph = Vec::new();

        // Schema metadata
        let mut schema_meta = serde_json::Map::new();
        schema_meta.insert("@id".to_string(), json!(schema.id));
        schema_meta.insert("@type".to_string(), json!("owl:Ontology"));
        schema_meta.insert("rdfs:label".to_string(), json!(schema.name));

        if let Some(version) = &schema.version {
            schema_meta.insert("owl:versionInfo".to_string(), json!(version));
        }

        if let Some(desc) = &schema.description {
            schema_meta.insert("dcterms:description".to_string(), json!(desc));
        }

        graph.push(json!(schema_meta));

        // Add classes
        for (class_name, class) in &schema.classes {
            let class_doc = Self::generate_class_document(class_name, class, schema);
            graph.push(class_doc);
        }

        // Add properties
        for (slot_name, slot) in &schema.slots {
            let property_doc = Self::generate_property_document(slot_name, slot, schema);
            graph.push(property_doc);
        }

        // Add enums
        for (enum_name, enum_def) in &schema.enums {
            let enum_doc = Self::generate_enum_document(enum_name, enum_def, schema);
            graph.push(enum_doc);
        }

        doc.insert("@graph".to_string(), json!(graph));

        json!(doc)
    }

    /// Generate class document
    fn generate_class_document(
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = Self::to_snake_case(&schema.name);

        doc.insert(
            "@id".to_string(),
            json!(format!("{}:{}", schema_prefix, Self::to_pascal_case(name))),
        );
        doc.insert("@type".to_string(), json!("owl:Class"));
        doc.insert("rdfs:label".to_string(), json!(name));

        if let Some(desc) = &class.description {
            doc.insert("skos:definition".to_string(), json!(desc));
        }

        // Superclass
        if let Some(parent) = &class.is_a {
            doc.insert(
                "rdfs:subClassOf".to_string(),
                json!(format!(
                    "{}:{}",
                    schema_prefix,
                    Self::to_pascal_case(parent)
                )),
            );
        }

        // Mixins as additional superclasses
        if !class.mixins.is_empty() {
            let mixin_refs: Vec<JsonValue> = class
                .mixins
                .iter()
                .map(|m| json!(format!("{}:{}", schema_prefix, Self::to_pascal_case(m))))
                .collect();

            if let Some(parent) = doc.get("rdfs:subClassOf") {
                let mut all_parents = vec![parent.clone()];
                all_parents.extend(mixin_refs);
                doc.insert("rdfs:subClassOf".to_string(), json!(all_parents));
            } else {
                doc.insert("rdfs:subClassOf".to_string(), json!(mixin_refs));
            }
        }

        json!(doc)
    }

    /// Generate property document
    fn generate_property_document(
        name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = Self::to_snake_case(&schema.name);

        doc.insert(
            "@id".to_string(),
            json!(format!("{}:{}", schema_prefix, Self::to_snake_case(name))),
        );

        // Determine property type
        let property_type = if let Some(range) = &slot.range {
            if schema.classes.contains_key(range) {
                "owl:ObjectProperty"
            } else {
                "owl:DatatypeProperty"
            }
        } else {
            "owl:DatatypeProperty"
        };

        doc.insert("@type".to_string(), json!(property_type));
        doc.insert("rdfs:label".to_string(), json!(name));

        if let Some(desc) = &slot.description {
            doc.insert("skos:definition".to_string(), json!(desc));
        }

        // Range
        if let Some(range) = &slot.range {
            if let Some(xsd_type) = Self::get_xsd_datatype(range) {
                doc.insert("rdfs:range".to_string(), json!(xsd_type));
            } else if schema.classes.contains_key(range) || schema.enums.contains_key(range) {
                doc.insert(
                    "rdfs:range".to_string(),
                    json!(format!("{}:{}", schema_prefix, Self::to_pascal_case(range))),
                );
            }
        }

        json!(doc)
    }

    /// Generate enum document
    fn generate_enum_document(
        name: &str,
        enum_def: &EnumDefinition,
        schema: &SchemaDefinition,
    ) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = Self::to_snake_case(&schema.name);

        doc.insert(
            "@id".to_string(),
            json!(format!("{}:{}", schema_prefix, Self::to_pascal_case(name))),
        );
        doc.insert("@type".to_string(), json!("owl:Class"));
        doc.insert("rdfs:label".to_string(), json!(name));

        if let Some(desc) = &enum_def.description {
            doc.insert("skos:definition".to_string(), json!(desc));
        }

        // Create individuals
        let individuals: Vec<JsonValue> = enum_def
            .permissible_values
            .iter()
            .map(|pv| {
                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                json!(format!(
                    "{}:{}_{}",
                    schema_prefix,
                    Self::to_pascal_case(name),
                    Self::to_pascal_case(value)
                ))
            })
            .collect();

        let mut equiv_class = serde_json::Map::new();
        equiv_class.insert("@type".to_string(), json!("owl:Class"));
        equiv_class.insert("owl:oneOf".to_string(), json!(individuals));

        doc.insert("owl:equivalentClass".to_string(), json!(equiv_class));

        json!(doc)
    }

    /// Generate example instance
    ///
    /// # Errors
    /// Returns `GeneratorError::Generation` if the class name is not found in the schema
    pub fn generate_example_instance(
        &self,
        class_name: &str,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<JsonValue> {
        let class = schema.classes.get(class_name).ok_or_else(|| {
            GeneratorError::Generation(format!("example: Class {class_name} not found"))
        })?;

        let mut instance = serde_json::Map::new();
        let schema_prefix = Self::to_snake_case(&schema.name);

        // Add context reference
        instance.insert(
            "@context".to_string(),
            json!(format!("{schema_prefix}.context.jsonld")),
        );

        // Add type
        instance.insert("@type".to_string(), json!(Self::to_pascal_case(class_name)));

        // Add ID
        instance.insert(
            "@id".to_string(),
            json!(format!(
                "{}#example-{}",
                schema.id,
                Self::to_snake_case(class_name)
            )),
        );

        // Collect all slots
        let all_slots = self.collect_all_slots(class, schema);

        // Add example values for required slots
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name)
                && slot.required == Some(true)
            {
                let property_id = Self::to_snake_case(slot_name);
                let example_value = Self::get_example_value(slot, schema);
                instance.insert(property_id, example_value);
            }
        }

        Ok(json!(instance))
    }

    /// Get example value for a slot
    fn get_example_value(slot: &SlotDefinition, schema: &SchemaDefinition) -> JsonValue {
        if let Some(range) = &slot.range {
            match range.as_str() {
                "string" | "str" => json!("example string"),
                "integer" | "int" => json!(42),
                "float" | "double" => json!(std::f64::consts::PI),
                "boolean" | "bool" => json!(true),
                "date" => json!("2024-01-15"),
                "datetime" => json!("2024-01-15T10:30:00Z"),
                "uri" => json!("https://example.org/resource"),
                _ => {
                    if schema.enums.contains_key(range) {
                        // Return first enum value
                        if let Some(enum_def) = schema.enums.get(range) {
                            if let Some(first_value) = enum_def.permissible_values.first() {
                                let value = match first_value {
                                    PermissibleValue::Simple(s) => s,
                                    PermissibleValue::Complex { text, .. } => text,
                                };
                                json!(value)
                            } else {
                                json!(null)
                            }
                        } else {
                            json!(null)
                        }
                    } else if schema.classes.contains_key(range) {
                        // Return reference to another object
                        json!({"@id": format!("#example-{}", Self::to_snake_case(range))})
                    } else {
                        json!("example")
                    }
                }
            }
        } else {
            json!("example")
        }
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();

        // First, get slots from parent if any
        if let Some(parent_name) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            all_slots.extend(self.collect_all_slots(parent_class, schema));
        }

        // Then add direct slots
        all_slots.extend(class.slots.clone());

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        all_slots.retain(|slot| seen.insert(slot.clone()));

        all_slots
    }

    /// Get `JSON`-LD type for `LinkML` range
    fn get_json_ld_type(&self, range: &str, schema: &SchemaDefinition) -> Option<String> {
        // Check if it's a custom type first
        if let Some(type_def) = schema.types.get(range)
            && let Some(base_type) = &type_def.base_type
        {
            return self.get_json_ld_type(base_type, schema);
        }

        match range {
            "string" | "str" => Some("xsd:string".to_string()),
            "integer" | "int" => Some("xsd:integer".to_string()),
            "float" | "double" => Some("xsd:double".to_string()),
            "decimal" => Some("xsd:decimal".to_string()),
            "boolean" | "bool" => Some("xsd:boolean".to_string()),
            "date" => Some("xsd:date".to_string()),
            "datetime" => Some("xsd:dateTime".to_string()),
            "time" => Some("xsd:time".to_string()),
            "uri" => Some("xsd:anyURI".to_string()),
            _ => None,
        }
    }

    /// Get XSD datatype
    fn get_xsd_datatype(range: &str) -> Option<String> {
        match range {
            "string" | "str" => Some("xsd:string".to_string()),
            "integer" | "int" => Some("xsd:integer".to_string()),
            "float" | "double" => Some("xsd:double".to_string()),
            "decimal" => Some("xsd:decimal".to_string()),
            "boolean" | "bool" => Some("xsd:boolean".to_string()),
            "date" => Some("xsd:date".to_string()),
            "datetime" => Some("xsd:dateTime".to_string()),
            "time" => Some("xsd:time".to_string()),
            "uri" => Some("xsd:anyURI".to_string()),
            _ => None,
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
        s.split(['_', '-'])
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
}

impl Default for JsonLdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for JsonLdGenerator {
    fn name(&self) -> &'static str {
        "json-ld"
    }

    fn description(&self) -> &'static str {
        "Generates JSON-LD context and schema documents from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Validate schema has required fields
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for JSON-LD generation",
            ));
        }

        // JSON-LD requires at least one class or type to generate meaningful output
        if schema.classes.is_empty() && schema.types.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have at least one class or type for JSON-LD generation",
            ));
        }

        Ok(())
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".jsonld", ".context.jsonld"]
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Choose generation mode based on options
        let generate_full = self
            .options
            .custom
            .get("full_schema")
            .is_some_and(|v| v == "true");

        let mut doc = if generate_full {
            // Use the comprehensive schema document generator
            match self.generate_schema_document(schema) {
                serde_json::Value::Object(obj) => obj,
                _ => serde_json::Map::new(),
            }
        } else {
            // Generate simple context document
            serde_json::Map::new()
        };

        // If not using full schema, generate basic structure
        if !generate_full {
            // Add context
            let context = self.generate_context(schema);
            doc.insert("@context".to_string(), context);

            // Add schema metadata
            doc.insert(
                "@id".to_string(),
                serde_json::Value::String(format!("https://example.org/schemas/{}", schema.name)),
            );
            doc.insert(
                "@type".to_string(),
                serde_json::Value::String("linkml:Schema".to_string()),
            );

            // Include documentation if enabled in options
            if self.options.include_docs
                && let Some(desc) = &schema.description
            {
                doc.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }

            // Add classes
            if !schema.classes.is_empty() {
                let classes_array: Vec<serde_json::Value> = schema
                    .classes
                    .iter()
                    .map(|(name, class_def)| {
                        // Use the class document generator for rich output
                        Self::generate_class_document(name, class_def, schema)
                    })
                    .collect();
                doc.insert(
                    "classes".to_string(),
                    serde_json::Value::Array(classes_array),
                );
            }

            // Add properties
            if !schema.slots.is_empty() {
                let properties_array: Vec<serde_json::Value> = schema
                    .slots
                    .iter()
                    .map(|(name, slot_def)| {
                        Self::generate_property_document(name, slot_def, schema)
                    })
                    .collect();
                doc.insert(
                    "properties".to_string(),
                    serde_json::Value::Array(properties_array),
                );
            }

            // Add enums
            if !schema.enums.is_empty() {
                let enums_array: Vec<serde_json::Value> = schema
                    .enums
                    .iter()
                    .map(|(name, enum_def)| Self::generate_enum_document(name, enum_def, schema))
                    .collect();
                doc.insert("enums".to_string(), serde_json::Value::Array(enums_array));
            }
        }

        // Generate example instances if requested
        if self
            .options
            .get_custom("include_examples")
            .is_some_and(|v| v == "true")
        {
            // Generate frame documents for each class
            let mut frames_map = serde_json::Map::new();
            for (class_name, class_def) in &schema.classes {
                let frame = self.generate_frame(class_name, class_def, schema);
                frames_map.insert(class_name.clone(), frame);
            }

            // Add frames to document
            if !frames_map.is_empty() {
                doc.insert("frames".to_string(), json!(frames_map));
            }

            // Also generate example instances
            let mut examples = Vec::new();
            for class_name in schema.classes.keys() {
                if let Ok(example) = self.generate_example_instance(class_name, schema) {
                    examples.push(example);
                }
            }
            if !examples.is_empty() {
                doc.insert("examples".to_string(), json!(examples));
            }
        }

        // Format based on options
        let use_compact = self
            .options
            .custom
            .get("compact")
            .is_some_and(|v| v == "true");
        let result = if use_compact {
            serde_json::to_string(&doc)
                .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?
        } else {
            serde_json::to_string_pretty(&doc)
                .map_err(|e| LinkMLError::service(format!("JSON formatting error: {e}")))?
        };

        Ok(result)
    }

    fn get_file_extension(&self) -> &'static str {
        "jsonld"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}
