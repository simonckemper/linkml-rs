//! Core Rust generator implementation

use super::base::BaseCodeFormatter;
use super::traits::{
    CodeFormatter, Generator, GeneratorError, GeneratorResult,
};
use super::GeneratorOptions;

use linkml_core::prelude::*;
use std::fmt::Write;

/// Rust code generator for LinkML schemas
pub struct RustGenerator {
    /// Generator name
    pub name: String,
    /// Generator description
    pub description: String,
}

impl RustGenerator {
    /// Create a new Rust generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "rust".to_string(),
            description: "Generate idiomatic Rust code from LinkML schemas with serde support, comprehensive validation, and zero-tolerance for unwrap()".to_string(),
        }
    }

    /// Get the generator name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the generator description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Check if a class has subclasses
    pub(super) fn has_subclasses(&self, class_name: &str, schema: &SchemaDefinition) -> bool {
        schema
            .classes
            .values()
            .any(|c| c.is_a.as_deref() == Some(class_name))
    }

    /// Get all direct subclasses of a class
    pub(super) fn get_subclasses(&self, class_name: &str, schema: &SchemaDefinition) -> Vec<String> {
        schema
            .classes
            .iter()
            .filter(|(_, c)| c.is_a.as_deref() == Some(class_name))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Convert field name to Rust naming convention
    pub(super) fn convert_field_name(&self, name: &str) -> String {
        BaseCodeFormatter::to_snake_case(name)
    }

    /// Convert error from fmt::Error to GeneratorError
    pub(super) fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Generation(format!("Format error: {}", err))
    }

    /// Generate file header with imports
    pub(super) fn generate_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // File header
        writeln!(
            &mut output,
            "//! Generated from LinkML schema: {}",
            if schema.name.is_empty() {
                "unnamed"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &schema.description {
            writeln!(&mut output, "//! {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Standard imports
        writeln!(&mut output, "use serde::{{Deserialize, Serialize}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "use std::collections::HashMap;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "use thiserror::Error;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate validation error enum
    pub(super) fn generate_validation_error(&self) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "/// Validation errors for generated types").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "#[derive(Debug, Error)]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "pub enum ValidationError {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    #[error(\"Required field missing: {{field}}\")]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    RequiredField {{ field: &'static str }},").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    #[error(\"Invalid value for field {{field}}: {{message}}\")]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    InvalidValue {{ field: &'static str, message: String }},").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    #[error(\"Pattern validation failed for field {{field}}\")]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    PatternValidation {{ field: &'static str }},").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RustGenerator {
    fn name(&self) -> &str {
        "rust"
    }

    fn description(&self) -> &str {
        "Generate Rust structs and enums from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Rust generation"
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        let _options = GeneratorOptions::default();
        let mut output = String::new();

        // Generate header
        output.push_str(&self.generate_header(schema).map_err(|e| LinkMLError::data_validation(e.to_string()))?);

        // Generate validation error enum
        output.push_str(&self.generate_validation_error().map_err(|e| LinkMLError::data_validation(e.to_string()))?);

        // Generate basic structs for classes
        for (class_name, class_def) in &schema.classes {
            output.push_str(&format!("\n/// {}\n", class_def.description.as_deref().unwrap_or(class_name)));
            output.push_str(&format!("#[derive(Debug, Clone, Serialize, Deserialize)]\npub struct {} {{\n", class_name));
            output.push_str("    // TODO: Add fields based on slots\n");
            output.push_str("}\n\n");
        }

        Ok(output)
    }

    fn get_file_extension(&self) -> &str {
        "rs"
    }

    fn get_default_filename(&self) -> &str {
        "schema"
    }
}

impl CodeFormatter for RustGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // In a real implementation, this would use rustfmt
        // For now, just return the code as-is
        Ok(code.to_string())
    }
}
