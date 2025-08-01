//! JSON-LD Context generator for LinkML schemas
//!
//! This module generates JSON-LD @context definitions from LinkML schemas,
//! enabling semantic web integration and linked data capabilities.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::{ClassDefinition, Schema, SlotDefinition};
use serde_json::{json, Map, Value};
use std::collections::HashMap;

/// JSON-LD Context generator configuration
#[derive(Debug, Clone)]
pub struct JsonLdContextGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Base URI for the schema
    pub base_uri: Option<String>,
    /// Whether to include type coercion
    pub include_type_coercion: bool,
    /// Whether to include language maps
    pub include_language_maps: bool,
    /// Default language for string values
    pub default_language: Option<String>,
    /// Whether to use compact IRIs
    pub use_curies: bool,
    /// Whether to include container mappings
    pub include_containers: bool,
}

impl Default for JsonLdContextGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            base_uri: None,
            include_type_coercion: true,
            include_language_maps: false,
            default_language: None,
            use_curies: true,
            include_containers: true,
        }
    }
}

/// JSON-LD Context generator
pub struct JsonLdContextGenerator {
    config: JsonLdContextGeneratorConfig,
}

impl JsonLdContextGenerator {
    /// Create a new JSON-LD Context generator
    pub fn new(config: JsonLdContextGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate the context object
    fn generate_context(&self, schema: &Schema) -> Result<Value, LinkMLError> {
        let mut context = Map::new();
        
        // Add base URI if provided
        if let Some(base) = &self.config.base_uri {
            context.insert("@base".to_string(), json!(base));
        }
        
        // Add default language if specified
        if let Some(lang) = &self.config.default_language {
            context.insert("@language".to_string(), json!(lang));
        }
        
        // Add prefix mappings
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                if prefix != "@base" && prefix != "@language" {
                    context.insert(prefix.clone(), json!(expansion.prefix_reference));
                }
            }
        }
        
        // Add default prefix if available
        if let Some(default_prefix) = &schema.default_prefix {
            if let Some(prefixes) = &schema.prefixes {
                if let Some(expansion) = prefixes.get(default_prefix) {
                    context.insert("@vocab".to_string(), json!(expansion.prefix_reference));
                }
            }
        }
        
        // Add class mappings
        if let Some(classes) = &schema.classes {
            for (class_name, class_def) in classes {
                self.add_class_to_context(class_name, class_def, &mut context, schema)?;
            }
        }
        
        // Add slot mappings
        if let Some(slots) = &schema.slots {
            for (slot_name, slot_def) in slots {
                self.add_slot_to_context(slot_name, slot_def, &mut context, schema)?;
            }
        }
        
        Ok(Value::Object(context))
    }
    
    /// Add a class to the context
    fn add_class_to_context(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &mut Map<String, Value>,
        schema: &Schema,
    ) -> Result<(), LinkMLError> {
        let mut class_mapping = Map::new();
        
        // Determine the IRI for the class
        let class_iri = self.get_iri_for_element(class_name, &class_def.id_prefixes, schema);
        class_mapping.insert("@id".to_string(), json!(class_iri));
        
        // Add type if this represents an RDF type
        if class_def.class_uri.is_some() || class_def.is_a.is_none() {
            class_mapping.insert("@type".to_string(), json!("@id"));
        }
        
        // Process class-specific slots
        if let Some(slots) = &class_def.slots {
            for slot_name in slots {
                if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                    self.add_slot_to_context(slot_name, slot_def, context, schema)?;
                }
            }
        }
        
        // Process attributes
        if let Some(attributes) = &class_def.attributes {
            for (attr_name, attr_def) in attributes {
                self.add_slot_to_context(attr_name, attr_def, context, schema)?;
            }
        }
        
        // Only add if there are actual mappings
        if !class_mapping.is_empty() {
            context.insert(class_name.to_string(), Value::Object(class_mapping));
        }
        
        Ok(())
    }
    
    /// Add a slot to the context
    fn add_slot_to_context(
        &self,
        slot_name: &str,
        slot_def: &SlotDefinition,
        context: &mut Map<String, Value>,
        schema: &Schema,
    ) -> Result<(), LinkMLError> {
        // Skip if already added
        if context.contains_key(slot_name) {
            return Ok(());
        }
        
        let mut slot_mapping = Map::new();
        
        // Determine the IRI for the slot
        let slot_iri = if let Some(uri) = &slot_def.slot_uri {
            uri.clone()
        } else {
            self.get_iri_for_element(slot_name, &slot_def.id_prefixes, schema)
        };
        
        // Simple string mapping if no special handling needed
        if !self.needs_complex_mapping(slot_def, schema) {
            context.insert(slot_name.to_string(), json!(slot_iri));
            return Ok(());
        }
        
        // Complex mapping
        slot_mapping.insert("@id".to_string(), json!(slot_iri));
        
        // Add type coercion if enabled
        if self.config.include_type_coercion {
            if let Some(type_value) = self.get_type_coercion(slot_def, schema) {
                slot_mapping.insert("@type".to_string(), type_value);
            }
        }
        
        // Add container mapping for multivalued slots
        if self.config.include_containers && slot_def.multivalued == Some(true) {
            slot_mapping.insert("@container".to_string(), json!("@list"));
        }
        
        // Add language mapping if applicable
        if self.config.include_language_maps && self.is_translatable_slot(slot_def) {
            slot_mapping.insert("@container".to_string(), json!("@language"));
        }
        
        context.insert(slot_name.to_string(), Value::Object(slot_mapping));
        Ok(())
    }
    
    /// Determine if a slot needs complex mapping
    fn needs_complex_mapping(&self, slot_def: &SlotDefinition, schema: &Schema) -> bool {
        // Needs complex mapping if:
        // - Type coercion is enabled and slot has a specific type
        // - It's multivalued and containers are enabled
        // - It's translatable and language maps are enabled
        // - It has a specific slot URI different from default
        
        if self.config.include_type_coercion && self.get_type_coercion(slot_def, schema).is_some() {
            return true;
        }
        
        if self.config.include_containers && slot_def.multivalued == Some(true) {
            return true;
        }
        
        if self.config.include_language_maps && self.is_translatable_slot(slot_def) {
            return true;
        }
        
        slot_def.slot_uri.is_some()
    }
    
    /// Get type coercion for a slot
    fn get_type_coercion(&self, slot_def: &SlotDefinition, schema: &Schema) -> Option<Value> {
        if let Some(range) = &slot_def.range {
            // Check if it's a class reference
            if schema.classes.as_ref().and_then(|c| c.get(range)).is_some() {
                return Some(json!("@id"));
            }
            
            // Check if it's a type
            if let Some(types) = &schema.types {
                if let Some(type_def) = types.get(range) {
                    // Map to XSD types
                    return match type_def.typeof.as_deref() {
                        Some("string") => None, // Default type
                        Some("integer") => Some(json!("xsd:integer")),
                        Some("float") => Some(json!("xsd:float")),
                        Some("double") => Some(json!("xsd:double")),
                        Some("boolean") => Some(json!("xsd:boolean")),
                        Some("date") => Some(json!("xsd:date")),
                        Some("datetime") => Some(json!("xsd:dateTime")),
                        Some("time") => Some(json!("xsd:time")),
                        Some("uri") => Some(json!("@id")),
                        _ => None,
                    };
                }
            }
            
            // Direct type mapping
            match range.as_str() {
                "integer" | "int" => Some(json!("xsd:integer")),
                "float" => Some(json!("xsd:float")),
                "double" => Some(json!("xsd:double")),
                "boolean" | "bool" => Some(json!("xsd:boolean")),
                "date" => Some(json!("xsd:date")),
                "datetime" => Some(json!("xsd:dateTime")),
                "time" => Some(json!("xsd:time")),
                "uri" | "uriorcurie" => Some(json!("@id")),
                _ => None,
            }
        } else {
            None
        }
    }
    
    /// Check if a slot is translatable
    fn is_translatable_slot(&self, slot_def: &SlotDefinition) -> bool {
        // A slot is translatable if it's a string type and marked as translatable
        if let Some(range) = &slot_def.range {
            if range == "string" || range == "text" {
                // In a real implementation, we'd check for a translatable annotation
                return slot_def.description.is_some();
            }
        }
        false
    }
    
    /// Get IRI for an element
    fn get_iri_for_element(
        &self,
        name: &str,
        id_prefixes: &Option<Vec<String>>,
        schema: &Schema,
    ) -> String {
        // Check if there's a specific prefix for this element
        if let Some(prefixes) = id_prefixes {
            if let Some(prefix) = prefixes.first() {
                if let Some(prefix_map) = &schema.prefixes {
                    if let Some(expansion) = prefix_map.get(prefix) {
                        return format!("{}{}", expansion.prefix_reference, name);
                    }
                }
            }
        }
        
        // Use CURIE if enabled and default prefix exists
        if self.config.use_curies {
            if let Some(default_prefix) = &schema.default_prefix {
                return format!("{}:{}", default_prefix, name);
            }
        }
        
        // Use base URI if available
        if let Some(base) = &self.config.base_uri {
            return format!("{}{}", base, name);
        }
        
        // Fallback to just the name
        name.to_string()
    }
}

impl Generator for JsonLdContextGenerator {
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let context = self.generate_context(schema)?;
        
        let output = json!({
            "@context": context
        });
        
        // Pretty print the JSON
        serde_json::to_string_pretty(&output)
            .map_err(|e| LinkMLError::GeneratorError(format!("Failed to serialize JSON-LD context: {}", e)))
    }
    
    fn get_file_extension(&self) -> &str {
        "jsonld"
    }
    
    fn get_default_filename(&self) -> &str {
        "context"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::{Prefix, SchemaDefinition};
    
    #[test]
    fn test_jsonld_context_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        
        // Add prefixes
        let mut prefixes = HashMap::new();
        prefixes.insert(
            "ex".to_string(),
            Prefix {
                prefix_prefix: "ex".to_string(),
                prefix_reference: "https://example.com/".to_string(),
            },
        );
        prefixes.insert(
            "schema".to_string(),
            Prefix {
                prefix_prefix: "schema".to_string(),
                prefix_reference: "https://schema.org/".to_string(),
            },
        );
        schema.prefixes = Some(prefixes);
        schema.default_prefix = Some("ex".to_string());
        
        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        person_class.slots = Some(vec!["name".to_string(), "age".to_string()]);
        
        schema.classes = Some(HashMap::from([
            ("Person".to_string(), person_class),
        ]));
        
        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.description = Some("The person's name".to_string());
        name_slot.range = Some("string".to_string());
        
        let mut age_slot = SlotDefinition::default();
        age_slot.description = Some("The person's age".to_string());
        age_slot.range = Some("integer".to_string());
        
        schema.slots = Some(HashMap::from([
            ("name".to_string(), name_slot),
            ("age".to_string(), age_slot),
        ]));
        
        let config = JsonLdContextGeneratorConfig {
            base_uri: Some("https://example.com/".to_string()),
            ..Default::default()
        };
        let generator = JsonLdContextGenerator::new(config);
        
        let result = generator.generate(&Schema(schema)).expect("should generate JSON-LD context");
        
        // Verify key elements
        assert!(result.contains("@context"));
        assert!(result.contains("@base"));
        assert!(result.contains("https://example.com/"));
        assert!(result.contains("ex"));
        assert!(result.contains("schema"));
        assert!(result.contains("xsd:integer"));
    }
}