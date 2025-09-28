//! Prefix map generator for `LinkML` schemas
//!
//! This module generates prefix expansion/contraction mappings from `LinkML` schemas,
//! enabling efficient namespace management in semantic web applications.

use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::error::LinkMLError;
use linkml_core::types::{PrefixDefinition, SchemaDefinition};
use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Prefix map generator configuration
#[derive(Debug, Clone)]
pub struct PrefixMapGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Output format
    pub format: PrefixMapFormat,
    /// Whether to include schema metadata
    pub include_metadata: bool,
    /// Whether to validate prefixes
    pub validate_prefixes: bool,
    /// Additional prefixes to include
    pub additional_prefixes: HashMap<String, String>,
}

/// Output format for prefix maps
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixMapFormat {
    /// Simple `JSON` object mapping prefixes to URIs
    Simple,
    /// Extended format with metadata
    Extended,
    /// Turtle/SPARQL prefix format
    Turtle,
    /// `YAML` format
    Yaml,
    /// CSV format
    Csv,
}

impl Default for PrefixMapGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            format: PrefixMapFormat::Simple,
            include_metadata: false,
            validate_prefixes: true,
            additional_prefixes: HashMap::new(),
        }
    }
}

/// Prefix map generator
pub struct PrefixMapGenerator {
    config: PrefixMapGeneratorConfig,
    /// Generator options (stub for future configuration)
    options: super::traits::GeneratorOptions,
}

impl PrefixMapGenerator {
    /// Create a new prefix map generator
    #[must_use]
    pub fn new(config: PrefixMapGeneratorConfig) -> Self {
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with custom options
    #[must_use]
    pub fn with_options(
        config: PrefixMapGeneratorConfig,
        options: super::traits::GeneratorOptions,
    ) -> Self {
        Self { config, options }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Get prefix reference from `PrefixDefinition`
    fn get_prefix_reference(prefix_def: &PrefixDefinition) -> String {
        match prefix_def {
            PrefixDefinition::Simple(url) => url.clone(),
            PrefixDefinition::Complex {
                prefix_reference, ..
            } => prefix_reference.clone().unwrap_or_default(),
        }
    }

    /// Generate prefix map in the configured format
    fn generate_prefix_map(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        match self.config.format {
            PrefixMapFormat::Simple => self.generate_simple_json(schema),
            PrefixMapFormat::Extended => self.generate_extended_json(schema),
            PrefixMapFormat::Turtle => Ok(self.generate_turtle(schema)),
            PrefixMapFormat::Yaml => Ok(self.generate_yaml(schema)),
            PrefixMapFormat::Csv => Ok(self.generate_csv(schema)),
        }
    }

    /// Generate simple `JSON` format
    fn generate_simple_json(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut map = Map::new();

        // Add schema prefixes
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                let reference = Self::get_prefix_reference(expansion);
                if self.config.validate_prefixes {
                    Self::validate_prefix(prefix, &reference)?;
                }
                map.insert(prefix.clone(), json!(reference));
            }
        }

        // Add additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            if self.config.validate_prefixes {
                Self::validate_prefix(prefix, uri)?;
            }
            map.insert(prefix.clone(), json!(uri));
        }

        // Add custom prefixes from options if enabled
        if self.options.include_docs
            && let Some(author) = self.get_custom_option("author")
        {
            map.insert("_author".to_string(), json!(author));
        }

        // Add common prefixes if not already present
        Self::add_common_prefixes(&mut map);

        serde_json::to_string_pretty(&map)
            .map_err(|e| LinkMLError::ServiceError(format!("Failed to serialize prefix map: {e}")))
    }

    /// Generate extended `JSON` format with metadata
    fn generate_extended_json(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut output = Map::new();

        // Add metadata if requested
        if self.config.include_metadata {
            let mut metadata = Map::new();
            if !schema.name.is_empty() {
                metadata.insert("schema_name".to_string(), json!(&schema.name));
            }
            if !schema.id.is_empty() {
                metadata.insert("schema_id".to_string(), json!(&schema.id));
            }
            metadata.insert(
                "generated_by".to_string(),
                json!("LinkML Prefix Map Generator"),
            );
            metadata.insert("format_version".to_string(), json!("1.0"));

            output.insert("@metadata".to_string(), Value::Object(metadata));
        }

        // Add prefixes
        let mut prefixes = Map::new();

        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                let mut prefix_info = Map::new();
                prefix_info.insert(
                    "uri".to_string(),
                    json!(Self::get_prefix_reference(expansion)),
                );

                // Add prefix metadata
                if prefix == schema.default_prefix.as_ref().unwrap_or(&String::new()) {
                    prefix_info.insert("default".to_string(), json!(true));
                }

                // Check if it's a standard prefix
                if Self::is_standard_prefix(prefix) {
                    prefix_info.insert("standard".to_string(), json!(true));
                }

                prefixes.insert(prefix.clone(), Value::Object(prefix_info));
            }
        }

        // Add additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            let mut prefix_info = Map::new();
            prefix_info.insert("uri".to_string(), json!(uri));
            prefix_info.insert("custom".to_string(), json!(true));

            prefixes.insert(prefix.clone(), Value::Object(prefix_info));
        }

        output.insert("prefixes".to_string(), Value::Object(prefixes));

        // Add usage statistics
        if self.config.include_metadata {
            let stats = Self::calculate_usage_stats(schema);
            output.insert("usage_stats".to_string(), stats);
        }

        serde_json::to_string_pretty(&output).map_err(|e| {
            LinkMLError::ServiceError(format!("Failed to serialize extended prefix map: {e}"))
        })
    }

    /// Generate Turtle/SPARQL prefix format
    fn generate_turtle(&self, schema: &SchemaDefinition) -> String {
        let mut lines = vec![];

        // Add header comment
        lines.push("# Prefix declarations for LinkML schema".to_string());
        if !schema.name.is_empty() {
            lines.push(format!("# SchemaDefinition: {}", &schema.name));
        }
        lines.push(String::new());

        // Add prefixes
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                lines.push(format!(
                    "@prefix {}: <{}> .",
                    prefix,
                    Self::get_prefix_reference(expansion)
                ));
            }
        }

        // Add additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            lines.push(format!("@prefix {prefix}: <{uri}> ."));
        }

        // Add common prefixes
        lines.push(String::new());
        lines.push("# Common prefixes".to_string());
        for (prefix, uri) in Self::get_common_prefixes() {
            if !schema.prefixes.contains_key(prefix) {
                lines.push(format!("@prefix {prefix}: <{uri}> ."));
            }
        }

        lines.join(
            "
",
        )
    }

    /// Generate `YAML` format
    fn generate_yaml(&self, schema: &SchemaDefinition) -> String {
        let mut lines = vec![];

        // Add header
        lines.push("# Prefix map for LinkML schema".to_string());
        if !schema.name.is_empty() {
            lines.push(format!("# SchemaDefinition: {}", &schema.name));
        }
        lines.push(String::new());

        if self.config.include_metadata {
            lines.push("metadata:".to_string());
            if !schema.name.is_empty() {
                lines.push(format!("  schema_name: {}", &schema.name));
            }
            if !schema.id.is_empty() {
                lines.push(format!("  schema_id: {}", &schema.id));
            }
            lines.push(String::new());
        }

        lines.push("prefixes:".to_string());

        // Add schema prefixes
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                lines.push(format!(
                    "  {}: {}",
                    prefix,
                    Self::get_prefix_reference(expansion)
                ));
            }
        }

        // Add additional prefixes
        if !self.config.additional_prefixes.is_empty() {
            lines.push(String::new());
            lines.push("  # Additional prefixes".to_string());
            for (prefix, uri) in &self.config.additional_prefixes {
                lines.push(format!("  {prefix}: {uri}"));
            }
        }

        lines.join(
            "
",
        )
    }

    /// Generate CSV format
    fn generate_csv(&self, schema: &SchemaDefinition) -> String {
        let mut lines = vec![];

        // Header
        lines.push("prefix,uri,type,is_default".to_string());

        // SchemaDefinition prefixes
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                let is_default = prefix == schema.default_prefix.as_ref().unwrap_or(&String::new());
                lines.push(format!(
                    "{},{},schema,{}",
                    prefix,
                    Self::get_prefix_reference(expansion),
                    is_default
                ));
            }
        }

        // Additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            lines.push(format!("{prefix},{uri},custom,false"));
        }

        lines.join(
            "
",
        )
    }

    /// Add common semantic web prefixes
    fn add_common_prefixes(map: &mut Map<String, Value>) {
        let common = Self::get_common_prefixes();

        for (prefix, uri) in common {
            if !map.contains_key(prefix) {
                map.insert(prefix.to_string(), json!(uri));
            }
        }
    }

    /// Get common semantic web prefixes
    fn get_common_prefixes() -> Vec<(&'static str, &'static str)> {
        vec![
            ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
            ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
            ("xsd", "http://www.w3.org/2001/XMLSchema#"),
            ("owl", "http://www.w3.org/2002/07/owl#"),
            ("skos", "http://www.w3.org/2004/02/skos/core#"),
            ("dcterms", "http://purl.org/dc/terms/"),
            ("foaf", "http://xmlns.com/foaf/0.1/"),
            ("schema", "https://schema.org/"),
            ("prov", "http://www.w3.org/ns/prov#"),
            ("linkml", "https://w3id.org/linkml/"),
        ]
    }

    /// Check if a prefix is a standard one
    fn is_standard_prefix(prefix: &str) -> bool {
        Self::get_common_prefixes()
            .iter()
            .any(|(p, _)| p == &prefix)
    }

    /// Validate a prefix
    fn validate_prefix(prefix: &str, uri: &str) -> Result<(), LinkMLError> {
        // Check prefix format
        if prefix.is_empty() {
            return Err(LinkMLError::ServiceError(
                "Empty prefix not allowed".to_string(),
            ));
        }

        if !prefix
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(LinkMLError::ServiceError(format!(
                "Invalid prefix '{prefix}': must contain only alphanumeric characters, underscores, or hyphens"
            )));
        }

        // Check URI format
        if !uri.ends_with('/') && !uri.ends_with('#') {
            return Err(LinkMLError::ServiceError(format!(
                "Invalid URI '{uri}': should end with '/' or '#'"
            )));
        }

        Ok(())
    }

    /// Calculate usage statistics
    fn calculate_usage_stats(schema: &SchemaDefinition) -> Value {
        let mut stats = Map::new();

        if !schema.prefixes.is_empty() {
            stats.insert("total_prefixes".to_string(), json!(schema.prefixes.len()));

            // Count usage in classes and slots
            let usage_count: HashMap<String, usize> = HashMap::new();

            // Check classes
            if !schema.classes.is_empty() {
                for (_, _class_def) in &schema.classes {
                    // This field is not present in the current LinkML specification
                    // if let Some(id_prefixes) = &class_def.id_prefixes {
                    //     for prefix in id_prefixes {
                    //         *usage_count.entry(prefix.clone()).or_insert(0) += 1;
                    //     }
                    // }
                }
            }

            // Check slots
            if !schema.slots.is_empty() {
                for (_, _slot_def) in &schema.slots {
                    // This field is not present in the current LinkML specification
                    // if let Some(id_prefixes) = &slot_def.id_prefixes {
                    //     for prefix in id_prefixes {
                    //         *usage_count.entry(prefix.clone()).or_insert(0) += 1;
                    //     }
                    // }
                }
            }

            let usage_map: Map<String, Value> = usage_count
                .into_iter()
                .map(|(k, v)| (k, json!(v)))
                .collect();

            stats.insert("usage_by_prefix".to_string(), Value::Object(usage_map));
        }

        Value::Object(stats)
    }
}

impl Generator for PrefixMapGenerator {
    fn name(&self) -> &'static str {
        "prefix_map"
    }

    fn description(&self) -> &'static str {
        "Generates prefix maps from LinkML schemas for namespace management"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "SchemaDefinition must have a name for prefixmap generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        self.generate_prefix_map(schema)
    }

    fn get_file_extension(&self) -> &str {
        match self.config.format {
            PrefixMapFormat::Simple | PrefixMapFormat::Extended => "json",
            PrefixMapFormat::Turtle => "ttl",
            PrefixMapFormat::Yaml => "yaml",
            PrefixMapFormat::Csv => "csv",
        }
    }

    fn get_default_filename(&self) -> &'static str {
        "prefix_map"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_map_generation() -> anyhow::Result<()> {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Add prefixes
        use indexmap::IndexMap;
        let mut prefixes = IndexMap::new();
        prefixes.insert(
            "ex".to_string(),
            PrefixDefinition::Simple("https://example.com/".to_string()),
        );
        prefixes.insert(
            "test".to_string(),
            PrefixDefinition::Simple("https://test.org/vocab#".to_string()),
        );
        schema.prefixes = prefixes;
        schema.default_prefix = Some("ex".to_string());

        // Test simple JSON format
        let config = PrefixMapGeneratorConfig::default();
        let generator = PrefixMapGenerator::new(config);
        let result = generator
            .generate(&schema)
            .expect("should generate prefix map: {}");

        assert!(result.contains("\"ex\": \"https://example.com/\""));
        assert!(result.contains("\"test\": \"https://test.org/vocab#\""));

        // Test Turtle format
        let config_ttl = PrefixMapGeneratorConfig {
            format: PrefixMapFormat::Turtle,
            ..Default::default()
        };
        let generator_ttl = PrefixMapGenerator::new(config_ttl);
        let result_ttl = generator_ttl
            .generate(&schema)
            .expect("should generate turtle prefix map: {}");

        assert!(result_ttl.contains("@prefix ex: <https://example.com/> ."));
        assert!(result_ttl.contains("@prefix test: <https://test.org/vocab#> ."));
        Ok(())
    }
}
