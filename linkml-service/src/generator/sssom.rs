//! SSSOM (Simple Standard for Sharing Ontological Mappings) generator for LinkML schemas
//!
//! This module generates SSSOM-compliant mapping files from LinkML schemas,
//! enabling interoperability between different ontologies and vocabularies.

use crate::generator::traits::{Generator, GeneratorConfig};
use anyhow::anyhow;
use chrono::Local;
use linkml_core::error::LinkMLError;
use linkml_core::types::{ClassDefinition, PrefixDefinition, SchemaDefinition, SlotDefinition};

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
    fn generate_sssom(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mappings = self.extract_mappings(schema)?;

        match self.config.format {
            SssomFormat::Tsv => self.generate_tsv(&mappings, schema),
            SssomFormat::Json => self.generate_json(&mappings, schema),
        }
    }

    /// Extract mappings from schema
    fn extract_mappings(
        &self,
        schema: &SchemaDefinition,
    ) -> Result<Vec<SssomMapping>, LinkMLError> {
        let mappings = Vec::new();
        let _mapping_date = Local::now().format("%Y-%m-%d").to_string();

        // Extract class mappings
        for (_class_name, _class_def) in &schema.classes {
            // TODO: Add mapping extraction when mapping fields are added to ClassDefinition
            // The following fields would be checked:
            // - exact_mappings
            // - close_mappings
            // - broad_mappings
            // - narrow_mappings
            // - related_mappings
        }

        // Extract slot mappings
        for (_slot_name, _slot_def) in &schema.slots {
            // TODO: Add mapping extraction when mapping fields are added to SlotDefinition
            // The following fields would be checked:
            // - exact_mappings
            // - close_mappings
        }

        // TODO: Add schema-level mapping extraction when source_file_mappings field is added

        Ok(mappings)
    }

    /// Create a mapping
    fn _create_mapping(
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
            subject_type: Some(format!(
                "owl:{}",
                match subject_type {
                    "class" => "Class",
                    "property" => "ObjectProperty",
                    _ => "Thing",
                }
            )),
            object_type: None, // Would need external lookup
            mapping_date: date.to_string(),
            creator_id: self.config.creator.clone(),
            comment,
        }
    }

    /// Get URI for a class
    fn _get_class_uri(
        &self,
        name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> String {
        if let Some(uri) = &class_def.class_uri {
            uri.clone()
        } else {
            // TODO: id_prefixes not yet implemented
            self._construct_uri(name, &None, schema)
        }
    }

    /// Get URI for a slot
    fn _get_slot_uri(
        &self,
        name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> String {
        if let Some(uri) = &slot_def.slot_uri {
            uri.clone()
        } else {
            // TODO: id_prefixes not yet implemented in SlotDefinition
            self._construct_uri(name, &None, schema)
        }
    }

    /// Construct URI from name and prefixes
    fn _construct_uri(
        &self,
        name: &str,
        id_prefixes: &Option<Vec<String>>,
        schema: &SchemaDefinition,
    ) -> String {
        if let Some(prefixes) = id_prefixes {
            if let Some(prefix) = prefixes.first() {
                if let Some(expansion) = schema.prefixes.get(prefix) {
                    let reference = match expansion {
                        PrefixDefinition::Simple(url) => url.clone(),
                        PrefixDefinition::Complex {
                            prefix_reference, ..
                        } => prefix_reference.as_ref().cloned().unwrap_or_default(),
                    };
                    return format!("{}{}", reference, name);
                }
                return format!("{}:{}", prefix, name);
            }
        }

        // Use default prefix if available
        if let Some(default_prefix) = &schema.default_prefix {
            if let Some(expansion) = schema.prefixes.get(default_prefix) {
                let reference = match expansion {
                    PrefixDefinition::Simple(url) => url.clone(),
                    PrefixDefinition::Complex {
                        prefix_reference, ..
                    } => prefix_reference.as_ref().cloned().unwrap_or_default(),
                };
                return format!("{}{}", reference, name);
            }
            return format!("{}:{}", default_prefix, name);
        }

        name.to_string()
    }

    /// Generate TSV format
    fn generate_tsv(
        &self,
        mappings: &[SssomMapping],
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        let mut output = String::new();

        // Add metadata header if requested
        if self.config.include_metadata {
            output.push_str("# SSSOM Metadata\n");
            output.push_str("# mapping_set_id: ");
            if !schema.id.is_empty() {
                output.push_str(&schema.id);
            } else if !schema.name.is_empty() {
                output.push_str(&format!("https://w3id.org/sssom/mappings/{}", schema.name));
            } else {
                output.push_str("https://w3id.org/sssom/mappings/unknown");
            }
            output.push('\n');

            if let Some(license) = &self.config.license {
                output.push_str(&format!("# license: {}\n", license));
            }

            if let Some(creator) = &self.config.creator {
                output.push_str(&format!("# creator_id: {}\n", creator));
            }

            output.push_str(&format!(
                "# mapping_date: {}\n",
                Local::now().format("%Y-%m-%d")
            ));

            if let Some(description) = &schema.description {
                output.push_str(&format!(
                    "# comment: Generated from LinkML schema - {}\n",
                    description
                ));
            }

            // Add prefix declarations
            output.push_str("# curie_map:\n");
            if !schema.prefixes.is_empty() {
                for (prefix, expansion) in &schema.prefixes {
                    let reference = match expansion {
                        PrefixDefinition::Simple(url) => url.clone(),
                        PrefixDefinition::Complex {
                            prefix_reference, ..
                        } => prefix_reference.as_ref().cloned().unwrap_or_default(),
                    };
                    output.push_str(&format!("#   {}: {}\n", prefix, reference));
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
    fn generate_json(
        &self,
        mappings: &[SssomMapping],
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        use serde_json::{Map, Value, json};

        let mut root = Map::new();

        // Add metadata
        let mut metadata = Map::new();

        if !schema.id.is_empty() {
            metadata.insert("mapping_set_id".to_string(), json!(&schema.id));
        } else if !schema.name.is_empty() {
            metadata.insert(
                "mapping_set_id".to_string(),
                json!(format!("https://w3id.org/sssom/mappings/{}", schema.name)),
            );
        } else {
            metadata.insert(
                "mapping_set_id".to_string(),
                json!("https://w3id.org/sssom/mappings/unknown"),
            );
        }

        if let Some(license) = &self.config.license {
            metadata.insert("license".to_string(), json!(license));
        }

        if let Some(creator) = &self.config.creator {
            metadata.insert("creator_id".to_string(), json!(creator));
        }

        metadata.insert(
            "mapping_date".to_string(),
            json!(Local::now().format("%Y-%m-%d").to_string()),
        );

        if let Some(description) = &schema.description {
            metadata.insert(
                "comment".to_string(),
                json!(format!("Generated from LinkML schema - {}", description)),
            );
        }

        // Add curie map
        let mut curie_map = Map::new();
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                let reference = match expansion {
                    PrefixDefinition::Simple(url) => url.clone(),
                    PrefixDefinition::Complex {
                        prefix_reference, ..
                    } => prefix_reference.as_ref().cloned().unwrap_or_default(),
                };
                curie_map.insert(prefix.clone(), json!(reference));
            }
        }
        curie_map.insert(
            "skos".to_string(),
            json!("http://www.w3.org/2004/02/skos/core#"),
        );
        curie_map.insert("owl".to_string(), json!("http://www.w3.org/2002/07/owl#"));
        curie_map.insert(
            "semapv".to_string(),
            json!("https://w3id.org/semapv/vocab/"),
        );

        metadata.insert("curie_map".to_string(), Value::Object(curie_map));

        root.insert("metadata".to_string(), Value::Object(metadata));

        // Add mappings
        let mapping_objects: Vec<Value> = mappings
            .iter()
            .map(|m| {
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
                obj.insert(
                    "mapping_justification".to_string(),
                    json!(m.mapping_justification),
                );
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
            })
            .collect();

        root.insert("mappings".to_string(), json!(mapping_objects));

        serde_json::to_string_pretty(&root).map_err(|e| {
            LinkMLError::ServiceError(format!("Failed to serialize SSSOM JSON: {}", e))
        })
    }
}

impl Generator for SssomGenerator {
    fn generate(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
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
    use linkml_core::types::SchemaDefinition;

    #[test]
    fn test_sssom_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();
        schema.id = "https://example.com/test-schema".to_string();

        // Add a class with mappings
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        // TODO: Add test mappings when mapping fields are added to ClassDefinition

        let mut classes = indexmap::IndexMap::new();
        classes.insert("Person".to_string(), person_class);
        schema.classes = classes;

        // Test TSV generation
        let config = SssomGeneratorConfig::default();
        let generator = SssomGenerator::new(config);
        let result = generator.generate(&schema).map_err(|e| anyhow::anyhow!("should generate SSSOM": {}, e))?;

        // Should contain header even with no mappings
        assert!(result.contains("subject_id\tsubject_label\tpredicate_id"));
        // TODO: Add assertions for actual mappings when mapping fields are added
    }
}
