//! SSSOM (Simple Standard for Sharing Ontological Mappings) generator for `LinkML` schemas
//!
//! This module generates SSSOM-compliant mapping files from `LinkML` schemas,
//! enabling interoperability between different ontologies and vocabularies.

use crate::generator::traits::{Generator, GeneratorConfig};
use crate::utils::timestamp::TimestampUtils;
use linkml_core::annotations::AnnotationValue;
use linkml_core::error::LinkMLError;
use linkml_core::types::{ClassDefinition, PrefixDefinition, SchemaDefinition, SlotDefinition};
use std::fmt::Write;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// SSSOM generator configuration
#[derive(Debug, Clone)]
pub struct SssomGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Output format (TSV or `JSON`)
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
    /// `JSON` representation
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
    /// Additional generator options for customization
    options: super::traits::GeneratorOptions,
    /// Timestamp utilities for date generation
    timestamp_utils: Arc<TimestampUtils>,
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
    #[must_use]
    pub fn new(config: SssomGeneratorConfig) -> Self {
        let timestamp_service = timestamp_service::wiring::wire_timestamp();
        let timestamp_utils = Arc::new(TimestampUtils::new(timestamp_service));
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
            timestamp_utils,
        }
    }

    /// Create generator with custom options
    #[must_use]
    pub fn with_options(
        config: SssomGeneratorConfig,
        options: super::traits::GeneratorOptions,
    ) -> Self {
        let timestamp_service = timestamp_service::wiring::wire_timestamp();
        let timestamp_utils = Arc::new(TimestampUtils::new(timestamp_service));
        Self {
            config,
            options,
            timestamp_utils,
        }
    }

    /// Create generator with custom timestamp service
    #[must_use]
    pub fn with_timestamp_service(
        config: SssomGeneratorConfig,
        timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        let timestamp_utils = Arc::new(TimestampUtils::new(timestamp_service));
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
            timestamp_utils,
        }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Generate SSSOM mappings from schema
    async fn generate_sssom(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mappings = self.extract_mappings(schema).await?;

        match self.config.format {
            SssomFormat::Tsv => self.generate_tsv(&mappings, schema).await,
            SssomFormat::Json => self.generate_json(&mappings, schema).await,
        }
    }

    /// Extract mappings from schema
    async fn extract_mappings(
        &self,
        schema: &SchemaDefinition,
    ) -> Result<Vec<SssomMapping>, LinkMLError> {
        let mut mappings = Vec::new();
        let mapping_date = self.timestamp_utils.today_string().await?;

        // Extract class mappings
        for (class_name, class_def) in &schema.classes {
            // DESIGN NOTE: LinkML Core defines mapping fields in ElementMetadata but they are not yet
            // integrated into ClassDefinition. Using annotations is the OFFICIAL approach for mapping
            // metadata until the core types are updated. This follows LinkML's extensibility pattern
            // where annotations provide forward-compatible metadata storage.

            // Extract mapping metadata from annotations (standard LinkML pattern)
            if let Some(annotations) = class_def.annotations.as_ref() {
                for (key, value) in annotations {
                    let predicate = match key.as_str() {
                        "exact_mapping" | "exact_mappings" => "skos:exactMatch",
                        "close_mapping" | "close_mappings" => "skos:closeMatch",
                        "broad_mapping" | "broad_mappings" => "skos:broadMatch",
                        "narrow_mapping" | "narrow_mappings" => "skos:narrowMatch",
                        "related_mapping" | "related_mappings" => "skos:relatedMatch",
                        _ => continue,
                    };

                    // Parse value which might be a list or single value
                    let value_str = match value {
                        AnnotationValue::String(s) => s.clone(),
                        AnnotationValue::Number(n) => n.to_string(),
                        AnnotationValue::Bool(b) => b.to_string(),
                        _ => format!("{value:?}"), // Fallback for complex types
                    };
                    let targets = if value_str.contains(',') {
                        value_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect::<Vec<_>>()
                    } else {
                        vec![value_str]
                    };

                    for target in targets {
                        if !target.is_empty() {
                            mappings.push(self._create_mapping(
                                &self._get_class_uri(class_name, class_def, schema),
                                &target,
                                predicate,
                                self.config.default_confidence,
                                &mapping_date,
                                class_def.description.as_deref(),
                                "class",
                                Some(format!("Mapping from LinkML class '{class_name}'")),
                            ));
                        }
                    }
                }
            }
        }

        // Extract slot mappings
        for (slot_name, slot_def) in &schema.slots {
            // DESIGN NOTE: Following LinkML's extensibility pattern, slot mappings are stored
            // in annotations until ElementMetadata is integrated into SlotDefinition.

            // Check for mapping annotations
            if let Some(annotations) = slot_def.annotations.as_ref() {
                for (key, value) in annotations {
                    let predicate = match key.as_str() {
                        "exact_mapping" | "exact_mappings" => "skos:exactMatch",
                        "close_mapping" | "close_mappings" => "skos:closeMatch",
                        _ => continue,
                    };

                    // Parse value which might be a list or single value
                    let value_str = match value {
                        AnnotationValue::String(s) => s.clone(),
                        AnnotationValue::Number(n) => n.to_string(),
                        AnnotationValue::Bool(b) => b.to_string(),
                        _ => format!("{value:?}"), // Fallback for complex types
                    };
                    let targets = if value_str.contains(',') {
                        value_str
                            .split(',')
                            .map(|s| s.trim().to_string())
                            .collect::<Vec<_>>()
                    } else {
                        vec![value_str]
                    };

                    for target in targets {
                        if !target.is_empty() {
                            mappings.push(self._create_mapping(
                                &self._get_slot_uri(slot_name, slot_def, schema),
                                &target,
                                predicate,
                                self.config.default_confidence,
                                &mapping_date,
                                slot_def.description.as_deref(),
                                "property",
                                Some(format!("Mapping from LinkML slot '{slot_name}'")),
                            ));
                        }
                    }
                }
            }
        }

        // Extract schema-level mappings from annotations
        // DESIGN NOTE: Schema-level mappings follow the same annotation-based pattern.
        if let Some(annotations) = schema.annotations.as_ref() {
            for (key, value) in annotations {
                if key == "source_file_mappings" || key == "schema_mappings" {
                    // Parse schema-level mappings from annotation value
                    // Expected format: "source:target:predicate:confidence"
                    let value_str = match value {
                        AnnotationValue::String(s) => s.clone(),
                        AnnotationValue::Number(n) => n.to_string(),
                        AnnotationValue::Bool(b) => b.to_string(),
                        _ => format!("{value:?}"), // Fallback for complex types
                    };
                    for mapping_str in value_str.split(';') {
                        let parts: Vec<&str> = mapping_str.trim().split(':').collect();
                        if parts.len() >= 3 {
                            let subject = parts[0];
                            let object = parts[1];
                            let predicate = parts.get(2).copied().unwrap_or("skos:exactMatch");
                            let confidence = parts
                                .get(3)
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(self.config.default_confidence);

                            if !subject.is_empty() && !object.is_empty() {
                                mappings.push(self._create_mapping(
                                    subject,
                                    object,
                                    predicate,
                                    confidence,
                                    &mapping_date,
                                    None,
                                    "schema",
                                    Some("Schema-level mapping".to_string()),
                                ));
                            }
                        }
                    }
                }
            }
        }

        Ok(mappings)
    }

    /// Create a mapping
    #[allow(clippy::too_many_arguments)]
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
            subject_label: description.map(std::string::ToString::to_string),
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
            creator_id: self
                .get_custom_option("creator")
                .cloned()
                .or_else(|| self.config.creator.clone()),
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
            // Check for id_prefixes in annotations (standard extension pattern)
            let id_prefixes = class_def
                .annotations
                .as_ref()
                .and_then(|ann| ann.get("id_prefixes"))
                .map(|v| {
                    let value_str = match v {
                        AnnotationValue::String(s) => s.clone(),
                        AnnotationValue::Number(n) => n.to_string(),
                        AnnotationValue::Bool(b) => b.to_string(),
                        _ => format!("{v:?}"), // Fallback for complex types
                    };
                    value_str.split(',').map(|s| s.trim().to_string()).collect()
                });

            self._construct_uri(name, &id_prefixes, schema)
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
            // Check for id_prefixes in annotations (standard extension pattern)
            let id_prefixes = slot_def
                .annotations
                .as_ref()
                .and_then(|ann| ann.get("id_prefixes"))
                .map(|v| {
                    let value_str = match v {
                        AnnotationValue::String(s) => s.clone(),
                        AnnotationValue::Number(n) => n.to_string(),
                        AnnotationValue::Bool(b) => b.to_string(),
                        _ => format!("{v:?}"), // Fallback for complex types
                    };
                    value_str.split(',').map(|s| s.trim().to_string()).collect()
                });

            self._construct_uri(name, &id_prefixes, schema)
        }
    }

    /// Construct URI from name and prefixes
    fn _construct_uri(
        &self,
        name: &str,
        id_prefixes: &Option<Vec<String>>,
        schema: &SchemaDefinition,
    ) -> String {
        if let Some(prefixes) = id_prefixes
            && let Some(prefix) = prefixes.first()
        {
            if let Some(expansion) = schema.prefixes.get(prefix) {
                let reference = match expansion {
                    PrefixDefinition::Simple(url) => url.clone(),
                    PrefixDefinition::Complex {
                        prefix_reference, ..
                    } => prefix_reference.clone().unwrap_or_default(),
                };
                return format!("{reference}{name}");
            }
            return format!("{prefix}:{name}");
        }

        // Use default prefix if available
        if let Some(default_prefix) = &schema.default_prefix {
            if let Some(expansion) = schema.prefixes.get(default_prefix) {
                let reference = match expansion {
                    PrefixDefinition::Simple(url) => url.clone(),
                    PrefixDefinition::Complex {
                        prefix_reference, ..
                    } => prefix_reference.clone().unwrap_or_default(),
                };
                return format!("{reference}{name}");
            }
            return format!("{default_prefix}:{name}");
        }

        name.to_string()
    }

    /// Generate TSV format
    async fn generate_tsv(
        &self,
        mappings: &[SssomMapping],
        schema: &SchemaDefinition,
    ) -> Result<String, LinkMLError> {
        let mut output = String::new();

        // Add metadata header if requested
        if self.config.include_metadata {
            output.push_str(
                "# SSSOM Metadata
",
            );
            output.push_str("# mapping_set_id: ");
            if !schema.id.is_empty() {
                output.push_str(&schema.id);
            } else if !schema.name.is_empty() {
                write!(output, "https://w3id.org/sssom/mappings/{}", schema.name)
                    .expect("write! to String should never fail");
            } else {
                output.push_str("https://w3id.org/sssom/mappings/unknown");
            }
            output.push('\n');

            if let Some(license) = &self.config.license {
                writeln!(output, "# license: {license}")
                    .expect("writeln! to String should never fail");
            }

            if let Some(creator) = &self.config.creator {
                writeln!(output, "# creator_id: {creator}")
                    .expect("writeln! to String should never fail");
            }

            let today = self.timestamp_utils.today_string().await?;
            writeln!(output, "# mapping_date: {today}")
                .expect("writeln! to String should never fail");

            if let Some(description) = &schema.description {
                writeln!(
                    output,
                    "# comment: Generated from LinkML schema - {description}"
                )
                .expect("LinkML operation should succeed");
            }

            // Add prefix declarations
            output.push_str(
                "# curie_map:
",
            );
            if !schema.prefixes.is_empty() {
                for (prefix, expansion) in &schema.prefixes {
                    let reference = match expansion {
                        PrefixDefinition::Simple(url) => url.clone(),
                        PrefixDefinition::Complex {
                            prefix_reference, ..
                        } => prefix_reference.clone().unwrap_or_default(),
                    };
                    writeln!(output, "#   {prefix}: {reference}")
                        .expect("writeln! to String should never fail");
                }
            }
            // Add standard prefixes used in SSSOM
            output.push_str(
                "#   skos: http://www.w3.org/2004/02/skos/core#
",
            );
            output.push_str(
                "#   owl: http://www.w3.org/2002/07/owl#
",
            );
            output.push_str(
                "#   semapv: https://w3id.org/semapv/vocab/
",
            );
            output.push('\n');
        }

        // Add TSV header
        output.push_str("subject_id\tsubject_label\tpredicate_id\tobject_id\tobject_label\t");
        output.push_str("mapping_justification\tconfidence\tsubject_type\tobject_type\t");
        output.push_str(
            "mapping_date\tcreator_id\tcomment
",
        );

        // Add mappings
        for mapping in mappings {
            write!(
                output,
                "{}\t{}\t{}\t{}\t{}\t{}\t{:.2}\t{}\t{}\t{}\t{}\t{}
",
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
            )
            .unwrap();
        }

        Ok(output)
    }

    /// Generate `JSON` format
    async fn generate_json(
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

        let today = self.timestamp_utils.today_string().await?;
        metadata.insert("mapping_date".to_string(), json!(today));

        if let Some(description) = &schema.description {
            metadata.insert(
                "comment".to_string(),
                json!(format!("Generated from LinkML schema - {description}")),
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
                    } => prefix_reference.clone().unwrap_or_default(),
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

        serde_json::to_string_pretty(&root)
            .map_err(|e| LinkMLError::ServiceError(format!("Failed to serialize SSSOM JSON: {e}")))
    }
}

impl Generator for SssomGenerator {
    fn name(&self) -> &'static str {
        "sssom"
    }

    fn description(&self) -> &'static str {
        "Generate SSSOM mapping files from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for sssom generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        // This is synchronous but we need async - use blocking approach
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.generate_sssom(schema))
        })
    }

    fn get_file_extension(&self) -> &str {
        match self.config.format {
            SssomFormat::Tsv => "sssom.tsv",
            SssomFormat::Json => "sssom.json",
        }
    }

    fn get_default_filename(&self) -> &'static str {
        "mappings"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[test]
    fn test_sssom_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();
        schema.id = "https://example.com/test-schema".to_string();

        // Add a class with mappings
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        // Add mapping annotations (temporary until mapping fields are available)
        let mut annotations = indexmap::IndexMap::new();
        annotations.insert(
            "exact_mappings".to_string(),
            linkml_core::annotations::AnnotationValue::String(
                "foaf:Person, schema:Person".to_string(),
            ),
        );
        annotations.insert(
            "close_mappings".to_string(),
            linkml_core::annotations::AnnotationValue::String("dbo:Person".to_string()),
        );
        person_class.annotations = Some(annotations);

        let mut classes = indexmap::IndexMap::new();
        classes.insert("Person".to_string(), person_class);
        schema.classes = classes;

        // Test TSV generation
        let config = SssomGeneratorConfig::default();
        let generator = SssomGenerator::new(config);
        let result = generator
            .generate(&schema)
            .expect("should generate SSSOM: {}");

        // Should contain header
        assert!(result.contains("subject_id\tsubject_label\tpredicate_id"));
        // Should now contain actual mappings from annotations
        assert!(result.contains("skos:exactMatch") || result.contains("skos:closeMatch"));
        assert!(result.lines().count() > 10); // Should have metadata header and mappings
        Ok(())
    }
}
