//! `schema2sheets` command implementation
//!
//! Converts LinkML schema to Excel SchemaSheets template with data validation,
//! formatting, and examples.

use crate::generator::excel::ExcelGenerator;
use indicatif::{ProgressBar, ProgressStyle};
use linkml_core::error::{LinkMLError, Result};
use linkml_core::prelude::*;
use std::path::{Path, PathBuf};

/// Command for converting LinkML schema to Excel SchemaSheets
pub struct Schema2SheetsCommand {
    /// Input schema file path
    pub schema: PathBuf,
    /// Output Excel file path
    pub output: PathBuf,
    /// Add data validation
    pub validation: bool,
    /// Include example data
    pub examples: bool,
    /// Freeze header rows
    pub freeze_headers: bool,
    /// Add auto-filters
    pub filters: bool,
    /// Show progress indicators
    pub progress: bool,
    /// Verbose output
    pub verbose: bool,
}

impl Schema2SheetsCommand {
    /// Create a new schema2sheets command
    #[must_use]
    pub fn new(schema: PathBuf, output: PathBuf) -> Self {
        Self {
            schema,
            output,
            validation: false,
            examples: false,
            freeze_headers: true,
            filters: true,
            progress: true,
            verbose: false,
        }
    }

    /// Set data validation option
    #[must_use]
    pub fn with_validation(mut self, validation: bool) -> Self {
        self.validation = validation;
        self
    }

    /// Set examples option
    #[must_use]
    pub fn with_examples(mut self, examples: bool) -> Self {
        self.examples = examples;
        self
    }

    /// Set freeze headers option
    #[must_use]
    pub fn with_freeze_headers(mut self, freeze_headers: bool) -> Self {
        self.freeze_headers = freeze_headers;
        self
    }

    /// Set filters option
    #[must_use]
    pub fn with_filters(mut self, filters: bool) -> Self {
        self.filters = filters;
        self
    }

    /// Set progress indicator visibility
    #[must_use]
    pub fn with_progress(mut self, progress: bool) -> Self {
        self.progress = progress;
        self
    }

    /// Set verbose output
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Execute the command
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Schema file doesn't exist or can't be read
    /// - Schema is invalid or malformed
    /// - Excel generation fails
    /// - Output file can't be written
    pub async fn execute(&self) -> Result<()> {
        // Validate input file exists
        if !self.schema.exists() {
            return Err(LinkMLError::io_error(format!(
                "Schema file not found: {}",
                self.schema.display()
            )));
        }

        // Create progress bar if enabled
        let progress = if self.progress {
            let pb = ProgressBar::new(3);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
                    .expect("Invalid progress bar template")
                    .progress_chars("#>-"),
            );
            Some(pb)
        } else {
            None
        };

        // Step 1: Load schema
        if let Some(ref pb) = progress {
            pb.set_message(format!("Loading schema from {}...", self.schema.display()));
        } else if self.verbose {
            eprintln!("Loading schema from: {}", self.schema.display());
        }

        let schema = self.load_schema(&self.schema)?;

        if let Some(ref pb) = progress {
            pb.inc(1);
        }

        // Step 2: Generate Excel workbook
        if let Some(ref pb) = progress {
            pb.set_message("Generating Excel workbook...");
        } else if self.verbose {
            eprintln!("Generating Excel workbook...");
        }

        let mut generator = ExcelGenerator::new()
            .with_frozen_headers(self.freeze_headers)
            .with_filters(self.filters);

        if self.validation {
            generator = generator.with_validation(true);
        }

        if self.examples {
            generator = generator.with_examples(true);
        }

        generator
            .generate_file(&schema, self.output.to_str().ok_or_else(|| {
                LinkMLError::io_error("Invalid output path")
            })?)
            .map_err(|e| LinkMLError::service(format!("Failed to generate Excel file: {e}")))?;

        if let Some(ref pb) = progress {
            pb.inc(1);
        }

        // Step 3: Complete
        if let Some(ref pb) = progress {
            pb.inc(1);
            pb.finish_with_message(format!("✓ Excel template generated: {}", self.output.display()));
        } else {
            println!("Excel template generated: {}", self.output.display());
        }

        // Print summary if verbose
        if self.verbose {
            eprintln!("\nTemplate Summary:");
            eprintln!("  Schema: {}", schema.name);
            eprintln!("  Classes: {}", schema.classes.len());
            eprintln!("  Sheets: {}", schema.classes.len());
            eprintln!("  Validation: {}", if self.validation { "enabled" } else { "disabled" });
            eprintln!("  Examples: {}", if self.examples { "included" } else { "not included" });
        }

        Ok(())
    }

    /// Load schema from file (YAML or JSON)
    fn load_schema(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LinkMLError::io_error(format!("Failed to read schema file: {e}")))?;

        // Try YAML first, then JSON
        if let Ok(schema) = serde_yaml::from_str::<SchemaDefinition>(&content) {
            return Ok(schema);
        }

        if let Ok(schema) = serde_json::from_str::<SchemaDefinition>(&content) {
            return Ok(schema);
        }

        Err(LinkMLError::deserialization(
            "Failed to parse schema as YAML or JSON",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_builder() {
        let cmd = Schema2SheetsCommand::new(
            PathBuf::from("schema.yaml"),
            PathBuf::from("template.xlsx"),
        )
        .with_validation(true)
        .with_examples(true)
        .with_freeze_headers(false)
        .with_filters(false)
        .with_verbose(true);

        assert_eq!(cmd.schema, PathBuf::from("schema.yaml"));
        assert_eq!(cmd.output, PathBuf::from("template.xlsx"));
        assert!(cmd.validation);
        assert!(cmd.examples);
        assert!(!cmd.freeze_headers);
        assert!(!cmd.filters);
        assert!(cmd.verbose);
    }

    #[test]
    fn test_default_options() {
        let cmd = Schema2SheetsCommand::new(
            PathBuf::from("schema.yaml"),
            PathBuf::from("template.xlsx"),
        );

        assert!(!cmd.validation);
        assert!(!cmd.examples);
        assert!(cmd.freeze_headers);
        assert!(cmd.filters);
        assert!(cmd.progress);
        assert!(!cmd.verbose);
    }
}

