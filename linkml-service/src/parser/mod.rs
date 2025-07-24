//! Schema parsing module for LinkML service
//!
//! This module handles parsing LinkML schemas from YAML and JSON formats,
//! following the composition-over-inheritance pattern from Rust Book Chapter 17.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::path::Path;

pub mod import_resolver;
pub mod import_resolver_v2;
pub mod json_parser;
pub mod yaml_parser;
pub mod json_parser_v2;
pub mod yaml_parser_v2;
pub mod schema_loader;

pub use import_resolver::ImportResolver;
pub use import_resolver_v2::{ImportResolverV2, ImportSpec};
pub use json_parser::JsonParser;
pub use yaml_parser::YamlParser;
pub use json_parser_v2::JsonParserV2;
pub use yaml_parser_v2::{YamlParserV2, AsyncSchemaParser};
pub use schema_loader::SchemaLoader;

/// Trait for schema parsers
pub trait SchemaParser: Send + Sync {
    /// Parse schema from string content
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if parsing fails
    fn parse_str(&self, content: &str) -> Result<SchemaDefinition>;

    /// Parse schema from file
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - File cannot be read
    /// - Parsing fails
    fn parse_file(&self, path: &Path) -> Result<SchemaDefinition>;
}

/// Main parser that delegates to format-specific parsers
pub struct Parser {
    yaml_parser: YamlParser,
    json_parser: JsonParser,
    /// Whether to automatically resolve imports
    auto_resolve_imports: bool,
}

impl Parser {
    /// Create a new parser
    #[must_use]
    pub fn new() -> Self {
        Self {
            yaml_parser: YamlParser::new(),
            json_parser: JsonParser::new(),
            auto_resolve_imports: false,
        }
    }
    
    /// Create a parser that automatically resolves imports
    #[must_use]
    pub fn with_import_resolution() -> Self {
        Self {
            yaml_parser: YamlParser::new(),
            json_parser: JsonParser::new(),
            auto_resolve_imports: true,
        }
    }
    
    /// Set whether to automatically resolve imports
    pub fn set_auto_resolve_imports(&mut self, enabled: bool) {
        self.auto_resolve_imports = enabled;
    }

    /// Parse schema from file, detecting format from extension
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - File has no extension
    /// - File format is not supported
    /// - Parsing fails
    pub fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .ok_or_else(|| LinkMLError::parse("No file extension found"))?;

        match extension {
            "yaml" | "yml" => self.yaml_parser.parse_file(path),
            "json" => self.json_parser.parse_file(path),
            _ => Err(LinkMLError::parse(format!(
                "Unsupported file format: {extension}"
            ))),
        }
    }

    /// Parse schema from string with specified format
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Format is not supported
    /// - Parsing fails
    pub fn parse_str(&self, content: &str, format: &str) -> Result<SchemaDefinition> {
        match format {
            "yaml" | "yml" => self.yaml_parser.parse_str(content),
            "json" => self.json_parser.parse_str(content),
            _ => Err(LinkMLError::parse(format!("Unsupported format: {format}"))),
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
