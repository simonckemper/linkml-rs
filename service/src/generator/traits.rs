//! Core generator traits and types
//!
//! This module defines the fundamental traits and types used by all code generators.

use async_trait::async_trait;
use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Result type for generator operations
pub type GeneratorResult<T> = std::result::Result<T, GeneratorError>;

/// Errors that can occur during code generation
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Schema validation error
    #[error("Schema validation failed: {0}")]
    Validation(String),

    /// Code generation error
    #[error("Code generation failed: {0}")]
    Generation(String),

    /// Schema validation error (alternative name)
    #[error("Schema validation failed: {0}")]
    SchemaValidation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// `LinkML` core error
    #[error("LinkML error: {0}")]
    LinkML(#[from] LinkMLError),

    /// Template error
    #[error("Template error: {0}")]
    Template(String),

    /// Configuration error (alternative name)
    #[error("Configuration error: {0}")]
    Config(String),
}

impl From<anyhow::Error> for GeneratorError {
    fn from(err: anyhow::Error) -> Self {
        GeneratorError::Generation(err.to_string())
    }
}

impl From<GeneratorError> for LinkMLError {
    fn from(err: GeneratorError) -> Self {
        LinkMLError::data_validation(err.to_string())
    }
}

/// Options for code generation
#[derive(Debug, Clone, Default)]
pub struct GeneratorOptions {
    /// Include documentation in generated code
    pub include_docs: bool,

    /// Generate tests
    pub generate_tests: bool,

    /// Indentation style
    pub indent: IndentStyle,

    /// Output format
    pub output_format: OutputFormat,

    /// Custom options for specific generators
    pub custom: HashMap<String, String>,
}

/// Configuration for generators
#[derive(Debug, Clone, Default)]
pub struct GeneratorConfig {
    /// Output directory
    pub output_dir: Option<String>,

    /// Generator options
    pub options: GeneratorOptions,
}

impl GeneratorOptions {
    /// Create new generator options
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set whether to include documentation
    #[must_use]
    pub fn with_docs(mut self, include_docs: bool) -> Self {
        self.include_docs = include_docs;
        self
    }

    /// Set whether to generate tests
    #[must_use]
    pub fn with_tests(mut self, generate_tests: bool) -> Self {
        self.generate_tests = generate_tests;
        self
    }

    /// Set indentation style
    #[must_use]
    pub fn with_indent(mut self, indent: IndentStyle) -> Self {
        self.indent = indent;
        self
    }

    /// Set output format
    #[must_use]
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.output_format = format;
        self
    }

    /// Set a custom option
    #[must_use]
    pub fn set_custom(mut self, key: &str, value: &str) -> Self {
        self.custom.insert(key.to_string(), value.to_string());
        self
    }

    /// Get a custom option
    #[must_use]
    pub fn get_custom(&self, key: &str) -> Option<&String> {
        self.custom.get(key)
    }
}

/// Indentation style for generated code
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndentStyle {
    /// Use spaces for indentation
    Spaces(usize),
    /// Use tabs for indentation
    Tabs,
}

impl Default for IndentStyle {
    fn default() -> Self {
        Self::Spaces(4)
    }
}

impl IndentStyle {
    /// Get single indentation string
    #[must_use]
    pub fn single(&self) -> String {
        match self {
            Self::Spaces(n) => " ".repeat(*n),
            Self::Tabs => "\t".to_string(),
        }
    }

    /// Get indentation string for given level
    #[must_use]
    pub fn to_string(&self, level: usize) -> String {
        match self {
            Self::Spaces(n) => " ".repeat(n * level),
            Self::Tabs => "\t".repeat(level),
        }
    }
}

/// Output format for generated code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputFormat {
    /// Rust code
    Rust,
    /// `TypeQL` schema
    TypeQL,
    /// GraphQL schema
    GraphQL,
    /// `SQL` DDL
    SQL,
    /// TypeScript code
    TypeScript,
    /// Python code
    Python,
    /// `JSON` schema
    JSON,
    /// `YAML`
    YAML,
    /// Markdown documentation
    Markdown,
    /// HTML documentation
    HTML,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Rust
    }
}

/// Generated output from a code generator
#[derive(Debug, Clone)]
pub struct GeneratedOutput {
    /// Generated content
    pub content: String,
    /// Suggested filename
    pub filename: String,
    /// Metadata about the generation
    pub metadata: HashMap<String, String>,
}

/// Core trait for synchronous code generators
pub trait Generator: Send + Sync {
    /// Get generator name
    fn name(&self) -> &str;

    /// Get generator description
    fn description(&self) -> &str;

    /// Generate code from a schema
    ///
    /// # Errors
    /// Returns an error if the schema is invalid or code generation fails
    fn generate(&self, schema: &SchemaDefinition) -> Result<String>;

    /// Get the file extension for generated files
    fn get_file_extension(&self) -> &str;

    /// Get supported file extensions
    fn file_extensions(&self) -> Vec<&str> {
        vec![self.get_file_extension()]
    }

    /// Get the default filename for generated files
    fn get_default_filename(&self) -> &str;

    /// Validate schema before generation
    ///
    /// # Errors
    /// Returns an error if the schema validation fails
    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()>;
}

/// Core trait for asynchronous code generators
#[async_trait]
pub trait AsyncGenerator: Send + Sync {
    /// Get generator name
    fn name(&self) -> &str;

    /// Get generator description
    fn description(&self) -> &str;

    /// Get supported file extensions
    fn file_extensions(&self) -> Vec<&str>;

    /// Validate schema before generation
    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()>;

    /// Generate code from schema
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>>;
}

/// Trait for code formatting utilities
pub trait CodeFormatter {
    /// Get formatter name
    fn name(&self) -> &str;

    /// Get formatter description
    fn description(&self) -> &str;

    /// Get supported file extensions
    fn file_extensions(&self) -> Vec<&str>;

    /// Format code
    ///
    /// # Errors
    /// Returns an error if code formatting fails
    fn format_code(&self, code: &str) -> GeneratorResult<String>;

    /// Format documentation
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        doc.lines()
            .map(|line| format!("{prefix}{line}"))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    }

    /// Format list of items
    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        indent: &IndentStyle,
        level: usize,
        separator: &str,
    ) -> String {
        items
            .iter()
            .map(|item| format!("{}{}", indent.to_string(level), item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    /// Escape string for the target language
    fn escape_string(&self, s: &str) -> String {
        // Default implementation - no escaping
        s.to_string()
    }

    /// Convert identifier to target language conventions
    fn convert_identifier(&self, id: &str) -> String {
        // Default implementation - no conversion
        id.to_string()
    }
}
