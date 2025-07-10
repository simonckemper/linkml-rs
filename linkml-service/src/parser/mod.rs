//! Schema parsing module for LinkML service
//!
//! This module handles parsing LinkML schemas from YAML and JSON formats,
//! following the composition-over-inheritance pattern from Rust Book Chapter 17.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::path::Path;

pub mod yaml_parser;
pub mod json_parser;
pub mod import_resolver;

pub use yaml_parser::YamlParser;
pub use json_parser::JsonParser;
pub use import_resolver::ImportResolver;

/// Trait for schema parsers
pub trait SchemaParser: Send + Sync {
    /// Parse schema from string content
    fn parse_str(&self, content: &str) -> Result<SchemaDefinition>;
    
    /// Parse schema from file
    fn parse_file(&self, path: &Path) -> Result<SchemaDefinition>;
}

/// Main parser that delegates to format-specific parsers
pub struct Parser {
    yaml_parser: YamlParser,
    json_parser: JsonParser,
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            yaml_parser: YamlParser::new(),
            json_parser: JsonParser::new(),
        }
    }
    
    /// Parse schema from file, detecting format from extension
    pub fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let extension = path.extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| LinkMLError::parse("No file extension found"))?;
            
        match extension {
            "yaml" | "yml" => self.yaml_parser.parse_file(path),
            "json" => self.json_parser.parse_file(path),
            _ => Err(LinkMLError::parse(format!("Unsupported file format: {}", extension))),
        }
    }
    
    /// Parse schema from string with specified format
    pub fn parse_str(&self, content: &str, format: &str) -> Result<SchemaDefinition> {
        match format {
            "yaml" | "yml" => self.yaml_parser.parse_str(content),
            "json" => self.json_parser.parse_str(content),
            _ => Err(LinkMLError::parse(format!("Unsupported format: {}", format))),
        }
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parser_creation() {
        let parser = Parser::new();
        // Parser should be created successfully
        let _ = parser; // Use to avoid warning
    }
}