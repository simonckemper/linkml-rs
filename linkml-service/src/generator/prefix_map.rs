//! Prefix map generator for LinkML schemas
//!
//! This module generates prefix expansion/contraction mappings from LinkML schemas,
//! enabling efficient namespace management in semantic web applications.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::Schema;
use serde_json::{json, Map, Value};
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
    /// Simple JSON object mapping prefixes to URIs
    Simple,
    /// Extended format with metadata
    Extended,
    /// Turtle/SPARQL prefix format
    Turtle,
    /// YAML format
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
}

impl PrefixMapGenerator {
    /// Create a new prefix map generator
    pub fn new(config: PrefixMapGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate prefix map in the configured format
    fn generate_prefix_map(&self, schema: &Schema) -> Result<String, LinkMLError> {
        match self.config.format {
            PrefixMapFormat::Simple => self.generate_simple_json(schema),
            PrefixMapFormat::Extended => self.generate_extended_json(schema),
            PrefixMapFormat::Turtle => self.generate_turtle(schema),
            PrefixMapFormat::Yaml => self.generate_yaml(schema),
            PrefixMapFormat::Csv => self.generate_csv(schema),
        }
    }
    
    /// Generate simple JSON format
    fn generate_simple_json(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut map = Map::new();
        
        // Add schema prefixes
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                if self.config.validate_prefixes {
                    self.validate_prefix(prefix, &expansion.prefix_reference)?;
                }
                map.insert(prefix.clone(), json!(expansion.prefix_reference));
            }
        }
        
        // Add additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            if self.config.validate_prefixes {
                self.validate_prefix(prefix, uri)?;
            }
            map.insert(prefix.clone(), json!(uri));
        }
        
        // Add common prefixes if not already present
        self.add_common_prefixes(&mut map);
        
        serde_json::to_string_pretty(&map)
            .map_err(|e| LinkMLError::GeneratorError(format!("Failed to serialize prefix map: {}", e)))
    }
    
    /// Generate extended JSON format with metadata
    fn generate_extended_json(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = Map::new();
        
        // Add metadata if requested
        if self.config.include_metadata {
            let mut metadata = Map::new();
            if let Some(name) = &schema.name {
                metadata.insert("schema_name".to_string(), json!(name));
            }
            if let Some(id) = &schema.id {
                metadata.insert("schema_id".to_string(), json!(id));
            }
            metadata.insert("generated_by".to_string(), json!("LinkML Prefix Map Generator"));
            metadata.insert("format_version".to_string(), json!("1.0"));
            
            output.insert("@metadata".to_string(), Value::Object(metadata));
        }
        
        // Add prefixes
        let mut prefixes = Map::new();
        
        if let Some(schema_prefixes) = &schema.prefixes {
            for (prefix, expansion) in schema_prefixes {
                let mut prefix_info = Map::new();
                prefix_info.insert("uri".to_string(), json!(expansion.prefix_reference));
                
                // Add prefix metadata
                if prefix == schema.default_prefix.as_ref().unwrap_or(&String::new()) {
                    prefix_info.insert("default".to_string(), json!(true));
                }
                
                // Check if it's a standard prefix
                if self.is_standard_prefix(prefix) {
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
            let stats = self.calculate_usage_stats(schema);
            output.insert("usage_stats".to_string(), stats);
        }
        
        serde_json::to_string_pretty(&output)
            .map_err(|e| LinkMLError::GeneratorError(format!("Failed to serialize extended prefix map: {}", e)))
    }
    
    /// Generate Turtle/SPARQL prefix format
    fn generate_turtle(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut lines = vec![];
        
        // Add header comment
        lines.push("# Prefix declarations for LinkML schema".to_string());
        if let Some(name) = &schema.name {
            lines.push(format!("# Schema: {}", name));
        }
        lines.push(String::new());
        
        // Add prefixes
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                lines.push(format!("@prefix {}: <{}> .", prefix, expansion.prefix_reference));
            }
        }
        
        // Add additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            lines.push(format!("@prefix {}: <{}> .", prefix, uri));
        }
        
        // Add common prefixes
        lines.push(String::new());
        lines.push("# Common prefixes".to_string());
        for (prefix, uri) in self.get_common_prefixes() {
            if !schema.prefixes.as_ref().map(|p| p.contains_key(prefix)).unwrap_or(false) {
                lines.push(format!("@prefix {}: <{}> .", prefix, uri));
            }
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Generate YAML format
    fn generate_yaml(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut lines = vec![];
        
        // Add header
        lines.push("# Prefix map for LinkML schema".to_string());
        if let Some(name) = &schema.name {
            lines.push(format!("# Schema: {}", name));
        }
        lines.push(String::new());
        
        if self.config.include_metadata {
            lines.push("metadata:".to_string());
            if let Some(name) = &schema.name {
                lines.push(format!("  schema_name: {}", name));
            }
            if let Some(id) = &schema.id {
                lines.push(format!("  schema_id: {}", id));
            }
            lines.push(String::new());
        }
        
        lines.push("prefixes:".to_string());
        
        // Add schema prefixes
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                lines.push(format!("  {}: {}", prefix, expansion.prefix_reference));
            }
        }
        
        // Add additional prefixes
        if !self.config.additional_prefixes.is_empty() {
            lines.push(String::new());
            lines.push("  # Additional prefixes".to_string());
            for (prefix, uri) in &self.config.additional_prefixes {
                lines.push(format!("  {}: {}", prefix, uri));
            }
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Generate CSV format
    fn generate_csv(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut lines = vec![];
        
        // Header
        lines.push("prefix,uri,type,is_default".to_string());
        
        // Schema prefixes
        if let Some(prefixes) = &schema.prefixes {
            for (prefix, expansion) in prefixes {
                let is_default = prefix == schema.default_prefix.as_ref().unwrap_or(&String::new());
                lines.push(format!(
                    "{},{},schema,{}",
                    prefix,
                    expansion.prefix_reference,
                    is_default
                ));
            }
        }
        
        // Additional prefixes
        for (prefix, uri) in &self.config.additional_prefixes {
            lines.push(format!("{},{},custom,false", prefix, uri));
        }
        
        Ok(lines.join("\n"))
    }
    
    /// Add common semantic web prefixes
    fn add_common_prefixes(&self, map: &mut Map<String, Value>) {
        let common = self.get_common_prefixes();
        
        for (prefix, uri) in common {
            if !map.contains_key(prefix) {
                map.insert(prefix.to_string(), json!(uri));
            }
        }
    }
    
    /// Get common semantic web prefixes
    fn get_common_prefixes(&self) -> Vec<(&'static str, &'static str)> {
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
    fn is_standard_prefix(&self, prefix: &str) -> bool {
        self.get_common_prefixes()
            .iter()
            .any(|(p, _)| p == &prefix)
    }
    
    /// Validate a prefix
    fn validate_prefix(&self, prefix: &str, uri: &str) -> Result<(), LinkMLError> {
        // Check prefix format
        if prefix.is_empty() {
            return Err(LinkMLError::GeneratorError("Empty prefix not allowed".to_string()));
        }
        
        if !prefix.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(LinkMLError::GeneratorError(
                format!("Invalid prefix '{}': must contain only alphanumeric characters, underscores, or hyphens", prefix)
            ));
        }
        
        // Check URI format
        if !uri.ends_with('/') && !uri.ends_with('#') {
            return Err(LinkMLError::GeneratorError(
                format!("Invalid URI '{}': should end with '/' or '#'", uri)
            ));
        }
        
        Ok(())
    }
    
    /// Calculate usage statistics
    fn calculate_usage_stats(&self, schema: &Schema) -> Value {
        let mut stats = Map::new();
        
        if let Some(prefixes) = &schema.prefixes {
            stats.insert("total_prefixes".to_string(), json!(prefixes.len()));
            
            // Count usage in classes and slots
            let mut usage_count: HashMap<String, usize> = HashMap::new();
            
            // Check classes
            if let Some(classes) = &schema.classes {
                for (_, class_def) in classes {
                    if let Some(id_prefixes) = &class_def.id_prefixes {
                        for prefix in id_prefixes {
                            *usage_count.entry(prefix.clone()).or_insert(0) += 1;
                        }
                    }
                }
            }
            
            // Check slots
            if let Some(slots) = &schema.slots {
                for (_, slot_def) in slots {
                    if let Some(id_prefixes) = &slot_def.id_prefixes {
                        for prefix in id_prefixes {
                            *usage_count.entry(prefix.clone()).or_insert(0) += 1;
                        }
                    }
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
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
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
    
    fn get_default_filename(&self) -> &str {
        "prefix_map"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::{Prefix, SchemaDefinition};
    
    #[test]
    fn test_prefix_map_generation() {
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
            "test".to_string(),
            Prefix {
                prefix_prefix: "test".to_string(),
                prefix_reference: "https://test.org/vocab#".to_string(),
            },
        );
        schema.prefixes = Some(prefixes);
        schema.default_prefix = Some("ex".to_string());
        
        // Test simple JSON format
        let config = PrefixMapGeneratorConfig::default();
        let generator = PrefixMapGenerator::new(config);
        let result = generator.generate(&Schema(schema.clone())).expect("should generate prefix map");
        
        assert!(result.contains("\"ex\": \"https://example.com/\""));
        assert!(result.contains("\"test\": \"https://test.org/vocab#\""));
        
        // Test Turtle format
        let config_ttl = PrefixMapGeneratorConfig {
            format: PrefixMapFormat::Turtle,
            ..Default::default()
        };
        let generator_ttl = PrefixMapGenerator::new(config_ttl);
        let result_ttl = generator_ttl.generate(&Schema(schema)).expect("should generate turtle prefix map");
        
        assert!(result_ttl.contains("@prefix ex: <https://example.com/> ."));
        assert!(result_ttl.contains("@prefix test: <https://test.org/vocab#> ."));
    }
}