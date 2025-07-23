//! SSSOM (Simple Standard for Sharing Ontological Mappings) generator for LinkML schemas
//!
//! This module generates SSSOM-compliant mapping files from LinkML schemas,
//! enabling interoperability between different ontologies and vocabularies.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::{ClassDefinition, Schema, SlotDefinition};
use std::collections::{HashMap, HashSet};
use chrono::Local;

/// SSSOM generator configuration
#[derive(Debug, Clone)]
pub struct SssomGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Output format (TSV or JSON)
    pub format: SssomFormat,
    /// Include metadata header
    pub include_metadata: bool,
    /// Mapping predicate to use
    pub default_predicate: String,
    /// Confidence score for generated mappings
    pub default_confidence: f64,
    /// License URI for mappings
    pub license: Option<String>,
    /// Creator information
    pub creator: Option<String>,
}

/// SSSOM output format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SssomFormat {
    /// Tab-separated values (standard SSSOM format)
    Tsv,
    /// JSON representation
    Json,
}

impl Default for SssomGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            format: SssomFormat::Tsv,
            include_metadata: true,
            default_predicate: "skos:exactMatch".to_string(),
            default_confidence: 1.0,
            license: Some("https://creativecommons.org/publicdomain/zero/1.0/".to_string()),
            creator: None,
        }
    }
}

/// SSSOM generator
pub struct SssomGenerator {
    config: SssomGeneratorConfig,
}

/// SSSOM mapping structure
#[derive(Debug, Clone)]
struct SssomMapping {
    subject_id: String,
    subject_label: Option<String>,
    predicate_id: String,
    object_id: String,
    object_label: Option<String>,
    mapping_justification: String,
    confidence: f64,
    subject_type: Option<String>,
    object_type: Option<String>,
    mapping_date: String,
    creator_id: Option<String>,
    comment: Option<String>,
}

impl SssomGenerator {
    /// Create a new SSSOM generator
    pub fn new(config: SssomGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate SSSOM mappings from schema
    fn generate_sssom(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mappings = self.extract_mappings(schema)?;
        
        match self.config.format {
            SssomFormat::Tsv => self.generate_tsv(&mappings, schema),
            SssomFormat::Json => self.generate_json(&mappings, schema),
        }
    }
    
    /// Extract mappings from schema
    fn extract_mappings(&self, schema: &Schema) -> Result<Vec<SssomMapping>, LinkMLError> {
        let mut mappings = Vec::new();
        let mapping_date = Local::now().format("%Y-%m-%d").to_string();
        
        // Extract class mappings
        if let Some(classes) = &schema.classes {
            for (class_name, class_def) in classes {
                // Check for exact matches
                if let Some(exact_mappings) = &class_def.exact_mappings {
                    for target in exact_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_class_uri(class_name, class_def, schema),
                            target,
                            "skos:exactMatch",
                            1.0,
                            &mapping_date,
                            class_def.description.as_deref(),
                            "class",
                            None,
                        ));
                    }
                }
                
                // Check for close matches
                if let Some(close_mappings) = &class_def.close_mappings {
                    for target in close_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_class_uri(class_name, class_def, schema),
                            target,
                            "skos:closeMatch",
                            0.8,
                            &mapping_date,
                            class_def.description.as_deref(),
                            "class",
                            None,
                        ));
                    }
                }
                
                // Check for broad matches
                if let Some(broad_mappings) = &class_def.broad_mappings {
                    for target in broad_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_class_uri(class_name, class_def, schema),
                            target,
                            "skos:broadMatch",
                            0.7,
                            &mapping_date,
                            class_def.description.as_deref(),
                            "class",
                            None,
                        ));
                    }
                }
                
                // Check for narrow matches
                if let Some(narrow_mappings) = &class_def.narrow_mappings {
                    for target in narrow_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_class_uri(class_name, class_def, schema),
                            target,
                            "skos:narrowMatch",
                            0.7,
                            &mapping_date,
                            class_def.description.as_deref(),
                            "class",
                            None,
                        ));
                    }
                }
                
                // Check for related matches
                if let Some(related_mappings) = &class_def.related_mappings {
                    for target in related_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_class_uri(class_name, class_def, schema),
                            target,
                            "skos:relatedMatch",
                            0.6,
                            &mapping_date,
                            class_def.description.as_deref(),
                            "class",
                            None,
                        ));
                    }
                }
            }
        }
        
        // Extract slot mappings
        if let Some(slots) = &schema.slots {
            for (slot_name, slot_def) in slots {
                // Check for exact mappings
                if let Some(exact_mappings) = &slot_def.exact_mappings {
                    for target in exact_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_slot_uri(slot_name, slot_def, schema),
                            target,
                            "skos:exactMatch",
                            1.0,
                            &mapping_date,
                            slot_def.description.as_deref(),
                            "property",
                            None,
                        ));
                    }
                }
                
                // Check for close matches
                if let Some(close_mappings) = &slot_def.close_mappings {
                    for target in close_mappings {
                        mappings.push(self.create_mapping(
                            &self.get_slot_uri(slot_name, slot_def, schema),
                            target,
                            "skos:closeMatch",
                            0.8,
                            &mapping_date,
                            slot_def.description.as_deref(),
                            "property",
                            None,
                        ));
                    }
                }
            }
        }
        
        // Also check for mappings defined in schema metadata
        if let Some(mappings_metadata) = &schema.source_file_mappings {
            for (source, targets) in mappings_metadata {
                for target in targets {
                    mappings.push(self.create_mapping(
                        source,
                        target,
                        &self.config.default_predicate,
                        self.config.default_confidence,
                        &mapping_date,
                        None,
                        "schema",
                        Some("Mapping from schema metadata".to_string()),
                    ));
                }
            }
        }
        
        Ok(mappings)
    }
    
    /// Create a mapping
    fn create_mapping(
        &self,
        subject: &str,
        object: &str,
        predicate: &str,
        confidence: f64,
        date: &str,
        description: Option<&str>,
        subject_type: &str,
        comment: Option<String>,
    ) -> SssomMapping {
        SssomMapping {
            subject_id: subject.to_string(),
            subject_label: description.map(|s| s.to_string()),
            predicate_id: predicate.to_string(),
            object_id: object.to_string(),
            object_label: None, // Would need external lookup
            mapping_justification: "semapv:ManualMappingCuration".to_string(),
            confidence,
            subject_type: Some(format!("owl:{}", match subject_type {
                "class" => "Class",
                "property" => "ObjectProperty",
                _ => "Thing",
            })),
            object_type: None, // Would need external lookup
            mapping_date: date.to_string(),
            creator_id: self.config.creator.clone(),
            comment,
        }
    }
    
    /// Get URI for a class
    fn get_class_uri(&self, name: &str, class_def: &ClassDefinition, schema: &Schema) -> String {
        if let Some(uri) = &class_def.class_uri {
            uri.clone()
        } else {
            self.construct_uri(name, &class_def.id_prefixes, schema)
        }
    }
    
    /// Get URI for a slot
    fn get_slot_uri(&self, name: &str, slot_def: &SlotDefinition, schema: &Schema) -> String {
        if let Some(uri) = &slot_def.slot_uri {
            uri.clone()
        } else {
            self.construct_uri(name, &slot_def.id_prefixes, schema)
        }
    }
    
    /// Construct URI from name and prefixes
    fn construct_uri(&self, name: &str, id_prefixes: &Option<Vec<String>>, schema: &Schema) -> String {
        if let Some(prefixes) = id_prefixes {
            if let Some(prefix) = prefixes.first() {
                if let Some(schema_prefixes) = &schema.prefixes {
                    if let Some(expansion) = schema_prefixes.get(prefix) {
                        return format!("{}{}", expansion.prefix_reference, name);
                    }
                }
                return format!("{}:{}", prefix, name);
            }
        }
        
        // Use default prefix if available
        if let Some(default_prefix) = &schema.default_prefix {
            if let Some(schema_prefixes) = &schema.prefixes {
                if let Some(expansion) = schema_prefixes.get(default_prefix) {
                    return format!("{}{}", expansion.prefix_reference, name);
                }
            }
            return format!("{}:{}", default_prefix, name);
        }
        
        name.to_string()
    }
    
    /// Generate TSV format
    fn generate_tsv(&self, mappings: &[SssomMapping], schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        // Add metadata header if requested
        if self.config.include_metadata {
            output.push_str("# SSSOM Metadata\n");
            output.push_str("# mapping_set_id: ");
            if let Some(id) = &schema.id {
                output.push_str(id);
            } else if let Some(name) = &schema.name {
                output.push_str(&format!("https://w3id.org/sssom/mappings/{}", name));
            }
            output.push('\n');
            
            if let Some(license) = &self.config.license {
                output.push_str(&format!("# license: {}\n", license));
            }
            
            if let Some(creator) = &self.config.creator {
                output.push_str(&format!("# creator_id: {}\n", creator));
            }
            
            output.push_str(&format!("# mapping_date: {}\n", Local::now().format("%Y-%m-%d")));
            
            if let Some(description) = &schema.description {
                output.push_str(&format!("# comment: Generated from LinkML schema - {}\n", description));
            }
            
            // Add prefix declarations
            output.push_str("# curie_map:\n");
            if let Some(prefixes) = &schema.prefixes {
                for (prefix, expansion) in prefixes {
                    output.push_str(&format!("#   {}: {}\n", prefix, expansion.prefix_reference));
                }
            }
            // Add standard prefixes used in SSSOM
            output.push_str("#   skos: http://www.w3.org/2004/02/skos/core#\n");
            output.push_str("#   owl: http://www.w3.org/2002/07/owl#\n");
            output.push_str("#   semapv: https://w3id.org/semapv/vocab/\n");
            output.push('\n');
        }
        
        // Add TSV header
        output.push_str("subject_id\tsubject_label\tpredicate_id\tobject_id\tobject_label\t");
        output.push_str("mapping_justification\tconfidence\tsubject_type\tobject_type\t");
        output.push_str("mapping_date\tcreator_id\tcomment\n");
        
        // Add mappings
        for mapping in mappings {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{}\t{}\t{}\t{}\t{}\n",
                mapping.subject_id,
                mapping.subject_label.as_deref().unwrap_or(""),
                mapping.predicate_id,
                mapping.object_id,
                mapping.object_label.as_deref().unwrap_or(""),
                mapping.mapping_justification,
                mapping.confidence,
                mapping.subject_type.as_deref().unwrap_or(""),
                mapping.object_type.as_deref().unwrap_or(""),
                mapping.mapping_date,
                mapping.creator_id.as_deref().unwrap_or(""),
                mapping.comment.as_deref().unwrap_or(""),
            ));
        }
        
        Ok(output)
    }
    
    /// Generate JSON format
    fn generate_json(&self, mappings: &[SssomMapping], schema: &Schema) -> Result<String, LinkMLError> {
        use serde_json::{json, Map, Value};
        
        let mut root = Map::new();
        
        // Add metadata
        let mut metadata = Map::new();
        
        if let Some(id) = &schema.id {
            metadata.insert("mapping_set_id".to_string(), json!(id));
        } else if let Some(name) = &schema.name {
            metadata.insert("mapping_set_id".to_string(), json!(format!("https://w3id.org/sssom/mappings/{}", name)));
        }
        
        if let Some(license) = &self.config.license {
            metadata.insert("license".to_string(), json!(license));
        }
        
        if let Some(creator) = &self.config.creator {
            metadata.insert("creator_id".to_string(), json!(creator));
        }
        
        metadata.insert("mapping_date".to_string(), json!(Local::now().format("%Y-%m-%d").to_string()));
        
        if let Some(description) = &schema.description {
            metadata.insert("comment".to_string(), json!(format!("Generated from LinkML schema - {}", description)));
        }
        
        // Add curie map
        let mut curie_map = Map::new();
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                curie_map.insert(prefix.clone(), json!(expansion.prefix_reference));
            }
        }
        curie_map.insert("skos".to_string(), json!("http://www.w3.org/2004/02/skos/core#"));
        curie_map.insert("owl".to_string(), json!("http://www.w3.org/2002/07/owl#"));
        curie_map.insert("semapv".to_string(), json!("https://w3id.org/semapv/vocab/"));
        
        metadata.insert("curie_map".to_string(), Value::Object(curie_map));
        
        root.insert("metadata".to_string(), Value::Object(metadata));
        
        // Add mappings
        let mapping_objects: Vec<Value> = mappings.iter().map(|m| {
            let mut obj = Map::new();
            obj.insert("subject_id".to_string(), json!(m.subject_id));
            if let Some(label) = &m.subject_label {
                obj.insert("subject_label".to_string(), json!(label));
            }
            obj.insert("predicate_id".to_string(), json!(m.predicate_id));
            obj.insert("object_id".to_string(), json!(m.object_id));
            if let Some(label) = &m.object_label {
                obj.insert("object_label".to_string(), json!(label));
            }
            obj.insert("mapping_justification".to_string(), json!(m.mapping_justification));
            obj.insert("confidence".to_string(), json!(m.confidence));
            if let Some(t) = &m.subject_type {
                obj.insert("subject_type".to_string(), json!(t));
            }
            if let Some(t) = &m.object_type {
                obj.insert("object_type".to_string(), json!(t));
            }
            obj.insert("mapping_date".to_string(), json!(m.mapping_date));
            if let Some(creator) = &m.creator_id {
                obj.insert("creator_id".to_string(), json!(creator));
            }
            if let Some(comment) = &m.comment {
                obj.insert("comment".to_string(), json!(comment));
            }
            Value::Object(obj)
        }).collect();
        
        root.insert("mappings".to_string(), json!(mapping_objects));
        
        serde_json::to_string_pretty(&root)
            .map_err(|e| LinkMLError::GeneratorError(format!("Failed to serialize SSSOM JSON: {}", e)))
    }
}

impl Generator for SssomGenerator {
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
        self.generate_sssom(schema)
    }
    
    fn get_file_extension(&self) -> &str {
        match self.config.format {
            SssomFormat::Tsv => "sssom.tsv",
            SssomFormat::Json => "sssom.json",
        }
    }
    
    fn get_default_filename(&self) -> &str {
        "mappings"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::SchemaDefinition;
    
    #[test]
    fn test_sssom_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        schema.id = Some("https://example.com/test-schema".to_string());
        
        // Add a class with mappings
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        person_class.exact_mappings = Some(vec!["schema:Person".to_string()]);
        person_class.close_mappings = Some(vec!["foaf:Person".to_string()]);
        
        schema.classes = Some(HashMap::from([
            ("Person".to_string(), person_class),
        ]));
        
        // Test TSV generation
        let config = SssomGeneratorConfig::default();
        let generator = SssomGenerator::new(config);
        let result = generator.generate(&Schema(schema)).unwrap();
        
        assert!(result.contains("subject_id\tsubject_label\tpredicate_id"));
        assert!(result.contains("skos:exactMatch"));
        assert!(result.contains("schema:Person"));
    }
}