//! JSON-LD generator for LinkML schemas
//!
//! This module generates JSON-LD (JSON for Linked Data) contexts and schemas from LinkML schemas.
//! JSON-LD is a W3C standard for representing linked data in JSON format.

use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, PermissibleValue};
use std::collections::HashMap;
use serde_json::{json, Value as JsonValue};

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

/// JSON-LD generator for linked data contexts
pub struct JsonLdGenerator {}

impl JsonLdGenerator {
    /// Create a new JSON-LD generator
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
    
    /// Create with custom options
    #[must_use]
    pub fn with_options(_options: GeneratorOptions) -> Self {
        Self {}
    }
    
    /// Generate JSON-LD context
    fn generate_context(&self, schema: &SchemaDefinition) -> JsonValue {
        let mut context = serde_json::Map::new();
        
        // Standard prefixes
        context.insert("@vocab".to_string(), json!(format!("{}#", schema.id)));
        context.insert("xsd".to_string(), json!("http://www.w3.org/2001/XMLSchema#"));
        context.insert("rdf".to_string(), json!("http://www.w3.org/1999/02/22-rdf-syntax-ns#"));
        context.insert("rdfs".to_string(), json!("http://www.w3.org/2000/01/rdf-schema#"));
        context.insert("owl".to_string(), json!("http://www.w3.org/2002/07/owl#"));
        context.insert("skos".to_string(), json!("http://www.w3.org/2004/02/skos/core#"));
        context.insert("dcterms".to_string(), json!("http://purl.org/dc/terms/"));
        
        // Add schema name as prefix
        let schema_prefix = self.to_snake_case(&schema.name);
        context.insert(schema_prefix.clone(), json!(format!("{}#", schema.id)));
        
        // Add classes
        for (class_name, _) in &schema.classes {
            let class_id = self.to_pascal_case(class_name);
            context.insert(class_id.clone(), json!(format!("{}:{}", schema_prefix, class_id)));
        }
        
        // Add properties (slots)
        for (slot_name, slot) in &schema.slots {
            let mut slot_def = serde_json::Map::new();
            
            let property_id = self.to_snake_case(slot_name);
            slot_def.insert("@id".to_string(), json!(format!("{}:{}", schema_prefix, property_id)));
            
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
            let enum_id = self.to_pascal_case(enum_name);
            context.insert(enum_id.clone(), json!(format!("{}:{}", schema_prefix, enum_id)));
        }
        
        json!(context)
    }
    
    /// Generate frame for a class
    fn generate_frame(&self, class_name: &str, class: &ClassDefinition, schema: &SchemaDefinition) -> JsonValue {
        let mut frame = serde_json::Map::new();
        
        // Add type
        let schema_prefix = self.to_snake_case(&schema.name);
        frame.insert("@type".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(class_name))));
        
        // Collect all slots (including inherited)
        let all_slots = self.collect_all_slots(class, schema);
        
        // Add slot defaults/examples
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let property_id = self.to_snake_case(slot_name);
                
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
    
    /// Generate JSON-LD schema document
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
            let class_doc = self.generate_class_document(class_name, class, schema);
            graph.push(class_doc);
        }
        
        // Add properties
        for (slot_name, slot) in &schema.slots {
            let property_doc = self.generate_property_document(slot_name, slot, schema);
            graph.push(property_doc);
        }
        
        // Add enums
        for (enum_name, enum_def) in &schema.enums {
            let enum_doc = self.generate_enum_document(enum_name, enum_def, schema);
            graph.push(enum_doc);
        }
        
        doc.insert("@graph".to_string(), json!(graph));
        
        json!(doc)
    }
    
    /// Generate class document
    fn generate_class_document(&self, name: &str, class: &ClassDefinition, schema: &SchemaDefinition) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        
        doc.insert("@id".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(name))));
        doc.insert("@type".to_string(), json!("owl:Class"));
        doc.insert("rdfs:label".to_string(), json!(name));
        
        if let Some(desc) = &class.description {
            doc.insert("skos:definition".to_string(), json!(desc));
        }
        
        // Superclass
        if let Some(parent) = &class.is_a {
            doc.insert("rdfs:subClassOf".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(parent))));
        }
        
        // Mixins as additional superclasses
        if !class.mixins.is_empty() {
            let mixin_refs: Vec<JsonValue> = class.mixins.iter()
                .map(|m| json!(format!("{}:{}", schema_prefix, self.to_pascal_case(m))))
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
    fn generate_property_document(&self, name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        
        doc.insert("@id".to_string(), json!(format!("{}:{}", schema_prefix, self.to_snake_case(name))));
        
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
            if let Some(xsd_type) = self.get_xsd_datatype(range) {
                doc.insert("rdfs:range".to_string(), json!(xsd_type));
            } else if schema.classes.contains_key(range) {
                doc.insert("rdfs:range".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(range))));
            } else if schema.enums.contains_key(range) {
                doc.insert("rdfs:range".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(range))));
            }
        }
        
        json!(doc)
    }
    
    /// Generate enum document
    fn generate_enum_document(&self, name: &str, enum_def: &EnumDefinition, schema: &SchemaDefinition) -> JsonValue {
        let mut doc = serde_json::Map::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        
        doc.insert("@id".to_string(), json!(format!("{}:{}", schema_prefix, self.to_pascal_case(name))));
        doc.insert("@type".to_string(), json!("owl:Class"));
        doc.insert("rdfs:label".to_string(), json!(name));
        
        if let Some(desc) = &enum_def.description {
            doc.insert("skos:definition".to_string(), json!(desc));
        }
        
        // Create individuals
        let individuals: Vec<JsonValue> = enum_def.permissible_values.iter()
            .map(|pv| {
                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                json!(format!("{}:{}_{}", schema_prefix, self.to_pascal_case(name), self.to_pascal_case(value)))
            })
            .collect();
        
        let mut equiv_class = serde_json::Map::new();
        equiv_class.insert("@type".to_string(), json!("owl:Class"));
        equiv_class.insert("owl:oneOf".to_string(), json!(individuals));
        
        doc.insert("owl:equivalentClass".to_string(), json!(equiv_class));
        
        json!(doc)
    }
    
    /// Generate example instance
    fn generate_example_instance(&self, class_name: &str, schema: &SchemaDefinition) -> GeneratorResult<JsonValue> {
        let class = schema.classes.get(class_name)
            .ok_or_else(|| GeneratorError::Generation {
                context: "example".to_string(),
                message: format!("Class {} not found", class_name),
            })?;
        
        let mut instance = serde_json::Map::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        
        // Add context reference
        instance.insert("@context".to_string(), json!(format!("{}.context.jsonld", schema_prefix)));
        
        // Add type
        instance.insert("@type".to_string(), json!(self.to_pascal_case(class_name)));
        
        // Add ID
        instance.insert("@id".to_string(), json!(format!("{}#example-{}", schema.id, self.to_snake_case(class_name))));
        
        // Collect all slots
        let all_slots = self.collect_all_slots(class, schema);
        
        // Add example values for required slots
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.required == Some(true) {
                    let property_id = self.to_snake_case(slot_name);
                    let example_value = self.get_example_value(slot, schema)?;
                    instance.insert(property_id, example_value);
                }
            }
        }
        
        Ok(json!(instance))
    }
    
    /// Get example value for a slot
    fn get_example_value(&self, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<JsonValue> {
        if let Some(range) = &slot.range {
            match range.as_str() {
                "string" | "str" => Ok(json!("example string")),
                "integer" | "int" => Ok(json!(42)),
                "float" | "double" => Ok(json!(3.14)),
                "boolean" | "bool" => Ok(json!(true)),
                "date" => Ok(json!("2024-01-15")),
                "datetime" => Ok(json!("2024-01-15T10:30:00Z")),
                "uri" => Ok(json!("https://example.org/resource")),
                _ => {
                    if schema.enums.contains_key(range) {
                        // Return first enum value
                        if let Some(enum_def) = schema.enums.get(range) {
                            if let Some(first_value) = enum_def.permissible_values.first() {
                                let value = match first_value {
                                    PermissibleValue::Simple(s) => s,
                                    PermissibleValue::Complex { text, .. } => text,
                                };
                                Ok(json!(value))
                            } else {
                                Ok(json!(null))
                            }
                        } else {
                            Ok(json!(null))
                        }
                    } else if schema.classes.contains_key(range) {
                        // Return reference to another object
                        Ok(json!({"@id": format!("#example-{}", self.to_snake_case(range))}))
                    } else {
                        Ok(json!("example"))
                    }
                }
            }
        } else {
            Ok(json!("example"))
        }
    }
    
    /// Collect all slots including inherited ones
    fn collect_all_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();
        
        // First, get slots from parent if any
        if let Some(parent_name) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent_name) {
                all_slots.extend(self.collect_all_slots(parent_class, schema));
            }
        }
        
        // Then add direct slots
        all_slots.extend(class.slots.clone());
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        all_slots.retain(|slot| seen.insert(slot.clone()));
        
        all_slots
    }
    
    /// Get JSON-LD type for LinkML range
    fn get_json_ld_type(&self, range: &str, schema: &SchemaDefinition) -> Option<String> {
        // Check if it's a custom type first
        if let Some(type_def) = schema.types.get(range) {
            if let Some(base_type) = &type_def.base_type {
                return self.get_json_ld_type(base_type, schema);
            }
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
    fn get_xsd_datatype(&self, range: &str) -> Option<String> {
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
    
    /// Convert to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;
        
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().expect("lowercase char should exist"));
            prev_upper = ch.is_uppercase();
        }
        
        result
    }
    
    /// Convert to PascalCase
    fn to_pascal_case(&self, s: &str) -> String {
        s.split(|c| c == '_' || c == '-')
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

#[async_trait]
impl Generator for JsonLdGenerator {
    fn name(&self) -> &str {
        "json-ld"
    }
    
    fn description(&self) -> &str {
        "Generates JSON-LD context and schema documents from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".jsonld", ".context.jsonld"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let mut outputs = Vec::new();
        
        // Generate context file
        let context = self.generate_context(schema);
        let context_content = if options.get_custom("pretty_print") == Some("true") {
            serde_json::to_string_pretty(&context)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        } else {
            serde_json::to_string(&context)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        };
        
        let context_filename = format!("{}.context.jsonld", self.to_snake_case(&schema.name));
        let mut context_metadata = HashMap::new();
        context_metadata.insert("type".to_string(), "context".to_string());
        context_metadata.insert("schema".to_string(), schema.name.clone());
        
        outputs.push(GeneratedOutput {
            filename: context_filename,
            content: context_content,
            metadata: context_metadata,
        });
        
        // Generate schema document
        let schema_doc = self.generate_schema_document(schema);
        let schema_content = if options.get_custom("pretty_print") == Some("true") {
            serde_json::to_string_pretty(&schema_doc)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        } else {
            serde_json::to_string(&schema_doc)
                .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
        };
        
        let schema_filename = format!("{}.schema.jsonld", self.to_snake_case(&schema.name));
        let mut schema_metadata = HashMap::new();
        schema_metadata.insert("type".to_string(), "schema".to_string());
        schema_metadata.insert("schema".to_string(), schema.name.clone());
        
        outputs.push(GeneratedOutput {
            filename: schema_filename,
            content: schema_content,
            metadata: schema_metadata,
        });
        
        // Generate frames for each class
        for (class_name, class) in &schema.classes {
            let frame = self.generate_frame(class_name, class, schema);
            let frame_content = if options.get_custom("pretty_print") == Some("true") {
                serde_json::to_string_pretty(&frame)
                    .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
            } else {
                serde_json::to_string(&frame)
                    .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
            };
            
            let frame_filename = format!("{}.{}.frame.jsonld", 
                self.to_snake_case(&schema.name), 
                self.to_snake_case(class_name)
            );
            
            let mut frame_metadata = HashMap::new();
            frame_metadata.insert("type".to_string(), "frame".to_string());
            frame_metadata.insert("class".to_string(), class_name.clone());
            
            outputs.push(GeneratedOutput {
                filename: frame_filename,
                content: frame_content,
                metadata: frame_metadata,
            });
        }
        
        // Generate example instances
        if options.get_custom("generate_examples") == Some("true") {
            for (class_name, _) in &schema.classes {
                if let Ok(example) = self.generate_example_instance(class_name, schema) {
                    let example_content = if options.get_custom("pretty_print") == Some("true") {
                        serde_json::to_string_pretty(&example)
                            .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
                    } else {
                        serde_json::to_string(&example)
                            .map_err(|e| GeneratorError::Template(format!("JSON formatting error: {e}")))?
                    };
                    
                    let example_filename = format!("{}.{}.example.jsonld", 
                        self.to_snake_case(&schema.name), 
                        self.to_snake_case(class_name)
                    );
                    
                    let mut example_metadata = HashMap::new();
                    example_metadata.insert("type".to_string(), "example".to_string());
                    example_metadata.insert("class".to_string(), class_name.clone());
                    
                    outputs.push(GeneratedOutput {
                        filename: example_filename,
                        content: example_content,
                        metadata: example_metadata,
                    });
                }
            }
        }
        
        Ok(outputs)
    }
}