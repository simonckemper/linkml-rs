//! Generator trait definitions and core types

use async_trait::async_trait;
use linkml_core::error::LinkMLError;
use linkml_core::types::SchemaDefinition;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Generator error types
#[derive(Debug, Error)]
pub enum GeneratorError {
    /// Schema validation error
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    /// IO error during generation
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Template rendering error
    #[error("Template error: {0}")]
    Template(String),

    /// Unsupported feature
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generation error with context
    #[error("Generation error in {context}: {message}")]
    Generation {
        /// Context where error occurred
        context: String,
        /// Error message
        message: String,
    },

    /// Plugin error
    #[error("Plugin error: {0}")]
    Plugin(String),
}

/// Result type for generator operations
pub type GeneratorResult<T> = std::result::Result<T, GeneratorError>;

impl From<GeneratorError> for LinkMLError {
    fn from(err: GeneratorError) -> Self {
        match err {
            GeneratorError::SchemaValidation(msg) => LinkMLError::schema_validation(msg),
            GeneratorError::Io(io_err) => LinkMLError::IoError(io_err),
            GeneratorError::Template(msg) => LinkMLError::service(format!("Template error: {}", msg)),
            GeneratorError::UnsupportedFeature(feature) => LinkMLError::not_implemented(feature),
            GeneratorError::Configuration(msg) => LinkMLError::config(msg),
            GeneratorError::Generation { context, message } => {
                LinkMLError::service(format!("Generation error in {}: {}", context, message))
            }
            GeneratorError::Plugin(msg) => LinkMLError::service(format!("Plugin error: {}", msg)),
        }
    }
}

/// Generated output from a generator
#[derive(Debug, Clone)]
pub struct GeneratedOutput {
    /// The generated content
    pub content: String,

    /// The suggested filename
    pub filename: String,

    /// Additional metadata about the generation
    pub metadata: HashMap<String, String>,
}

/// Async generator trait for code generation from LinkML schemas
#[async_trait]
pub trait AsyncGenerator: Send + Sync {
    /// Get the name of this generator
    fn name(&self) -> &str;

    /// Get a description of what this generator produces
    fn description(&self) -> &str;

    /// Get the file extensions this generator produces
    fn file_extensions(&self) -> Vec<&str>;

    /// Generate output from a schema
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>>;

    /// Generate output for a specific class
    async fn generate_class(
        &self,
        schema: &SchemaDefinition,
        class_name: &str,
        options: &GeneratorOptions,
    ) -> GeneratorResult<GeneratedOutput> {
        // Default implementation generates all and filters
        let outputs = self.generate(schema, options).await?;
        outputs
            .into_iter()
            .find(|output| output.metadata.get("class") == Some(&class_name.to_string()))
            .ok_or_else(|| GeneratorError::Generation {
                context: "class generation".to_string(),
                message: format!("Class '{class_name}' not found in schema"),
            })
    }

    /// Validate that the schema can be generated
    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Default implementation - can be overridden
        if schema.name.is_empty() {
            return Err(GeneratorError::SchemaValidation(
                "Schema must have a name".to_string(),
            ));
        }
        Ok(())
    }

    /// Write generated output to files
    async fn write_output(
        &self,
        outputs: &[GeneratedOutput],
        output_dir: &Path,
    ) -> GeneratorResult<()> {
        use tokio::fs;

        // Ensure output directory exists
        fs::create_dir_all(output_dir).await?;

        for output in outputs {
            let path = output_dir.join(&output.filename);

            // Create parent directories if needed
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).await?;
            }

            // Write the file
            fs::write(&path, &output.content).await?;
        }

        Ok(())
    }
}

/// Options for controlling generation
#[derive(Debug, Clone, Default)]
pub struct GeneratorOptions {
    /// Whether to include documentation comments
    pub include_docs: bool,

    /// Whether to include examples
    pub include_examples: bool,

    /// Whether to generate tests
    pub generate_tests: bool,

    /// Output format preferences
    pub format: OutputFormat,

    /// Custom options for specific generators
    pub custom: HashMap<String, String>,

    /// Target version or compatibility level
    pub target_version: Option<String>,

    /// Whether to validate generated output
    pub validate_output: bool,

    /// Indentation style
    pub indent: IndentStyle,
}

impl GeneratorOptions {
    /// Create new options with defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_docs: true,
            include_examples: false,
            generate_tests: false,
            format: OutputFormat::default(),
            custom: HashMap::new(),
            target_version: None,
            validate_output: true,
            indent: IndentStyle::default(),
        }
    }

    /// Builder method for including docs
    #[must_use]
    pub fn with_docs(mut self, include: bool) -> Self {
        self.include_docs = include;
        self
    }

    /// Builder method for including examples
    #[must_use]
    pub fn with_examples(mut self, include: bool) -> Self {
        self.include_examples = include;
        self
    }

    /// Builder method for generating tests
    #[must_use]
    pub fn with_tests(mut self, include: bool) -> Self {
        self.generate_tests = include;
        self
    }

    /// Set a custom option
    #[must_use]
    pub fn set_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Get a custom option
    #[must_use]
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom.get(key).map(String::as_str)
    }
}

/// Output format preferences
#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    /// Compact output with minimal whitespace
    Compact,

    /// Pretty-printed output with nice formatting
    Pretty,

    /// Custom format with specific settings
    Custom {
        /// Line width limit
        max_line_width: usize,
        /// Whether to align fields
        align_fields: bool,
        /// Whether to sort imports
        sort_imports: bool,
    },
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Pretty
    }
}

/// Indentation style
#[derive(Debug, Clone, PartialEq)]
pub enum IndentStyle {
    /// Use spaces for indentation
    Spaces(u8),

    /// Use tabs for indentation
    Tabs,
}

impl Default for IndentStyle {
    fn default() -> Self {
        Self::Spaces(4)
    }
}

impl IndentStyle {
    /// Convert to a string for the given indent level
    #[must_use]
    pub fn to_string(&self, level: usize) -> String {
        match self {
            Self::Spaces(n) => " ".repeat(*n as usize * level),
            Self::Tabs => "\t".repeat(level),
        }
    }

    /// Get a single indent
    #[must_use]
    pub fn single(&self) -> &str {
        match self {
            Self::Spaces(n) => {
                // This is a bit hacky but avoids allocation
                match n {
                    2 => "  ",
                    4 => "    ",
                    8 => "        ",
                    _ => "    ", // Default to 4 spaces for all other values
                }
            }
            Self::Tabs => "\t",
        }
    }
}

/// Generator configuration base type
#[derive(Debug, Clone, Default)]
pub struct GeneratorConfig {
    /// Include documentation in generated code
    pub include_docs: bool,
    /// Include examples in generated code  
    pub include_examples: bool,
    /// Target language version
    pub target_version: Option<String>,
    /// Custom configuration options
    pub custom: HashMap<String, String>,
}

/// Simple synchronous generator trait for backward compatibility
pub trait Generator: Send + Sync {
    /// Get the name of this generator
    fn name(&self) -> &str {
        "Unknown Generator"
    }
    
    /// Get a description of what this generator produces
    fn description(&self) -> &str {
        "No description available"
    }
    
    /// Get the file extensions this generator produces
    fn file_extensions(&self) -> Vec<&str> {
        vec![self.get_file_extension()]
    }
    
    /// Generate output from a schema
    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError>;
    
    /// Get the file extension for generated files
    fn get_file_extension(&self) -> &str;
    
    /// Get the default filename for generated files
    fn get_default_filename(&self) -> &str;
    
    /// Validate schema before generation (optional)
    fn validate_schema(&self, _schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        Ok(())
    }
}

/// Helper trait for formatting generated code
pub trait CodeFormatter {
    /// Format a documentation comment
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String;

    /// Format a list of items with proper indentation
    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        indent: &IndentStyle,
        level: usize,
        separator: &str,
    ) -> String;

    /// Escape a string for the target language
    fn escape_string(&self, s: &str) -> String;

    /// Convert a `LinkML` identifier to target language conventions
    fn convert_identifier(&self, id: &str) -> String;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_options() {
        let opts = GeneratorOptions::new()
            .with_docs(false)
            .with_tests(true)
            .set_custom("namespace", "com.example");

        assert!(!opts.include_docs);
        assert!(opts.generate_tests);
        assert_eq!(opts.get_custom("namespace"), Some("com.example"));
    }

    #[test]
    fn test_indent_style() {
        let spaces = IndentStyle::Spaces(4);
        assert_eq!(spaces.to_string(2), "        ");
        assert_eq!(spaces.single(), "    ");

        let tabs = IndentStyle::Tabs;
        assert_eq!(tabs.to_string(2), "\t\t");
        assert_eq!(tabs.single(), "\t");
    }
}
