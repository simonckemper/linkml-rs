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

        let mut schema = self.load_schema(&self.schema).await?;

        // Post-process schema to populate name fields from map keys
        self.populate_names(&mut schema);

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

    /// Populate name fields in schema from map keys
    ///
    /// LinkML YAML schemas use map keys as names, but the Rust structs
    /// require explicit name fields. This function populates those fields
    /// after deserialization.
    fn populate_names(&self, schema: &mut SchemaDefinition) {
        // Populate class names
        for (class_name, class_def) in &mut schema.classes {
            if class_def.name.is_empty() {
                class_def.name = class_name.clone();
            }

            // Populate attribute names
            for (attr_name, attr_def) in &mut class_def.attributes {
                if attr_def.name.is_empty() {
                    attr_def.name = attr_name.clone();
                }
            }

            // Populate slot_usage names
            for (slot_name, slot_def) in &mut class_def.slot_usage {
                if slot_def.name.is_empty() {
                    slot_def.name = slot_name.clone();
                }
            }
        }

        // Populate slot names
        for (slot_name, slot_def) in &mut schema.slots {
            if slot_def.name.is_empty() {
                slot_def.name = slot_name.clone();
            }
        }

        // Populate enum names
        for (enum_name, enum_def) in &mut schema.enums {
            if enum_def.name.is_empty() {
                enum_def.name = enum_name.clone();
            }
        }

        // Populate type names
        for (type_name, type_def) in &mut schema.types {
            if type_def.name.is_empty() {
                type_def.name = type_name.clone();
            }
        }

        // Populate subset names
        for (subset_name, subset_def) in &mut schema.subsets {
            if subset_def.name.is_empty() {
                subset_def.name = subset_name.clone();
            }
        }
    }

    /// Load schema from file (YAML or JSON) and resolve imports
    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| LinkMLError::io_error(format!("Failed to read schema file: {e}")))?;

        // Try YAML first, capture error
        let yaml_result = serde_yaml::from_str::<SchemaDefinition>(&content);
        let mut schema = if let Ok(schema) = yaml_result {
            schema
        } else {
            let yaml_error = yaml_result.unwrap_err();

            // Try JSON, capture error
            let json_result = serde_json::from_str::<SchemaDefinition>(&content);
            if let Ok(schema) = json_result {
                schema
            } else {
                let json_error = json_result.unwrap_err();

                // Return detailed error with both parsing attempts
                return Err(LinkMLError::deserialization(format!(
                    "Failed to parse schema as YAML or JSON.\n\
                     YAML parsing error: {}\n\
                     JSON parsing error: {}",
                    yaml_error, json_error
                )));
            }
        };

        // Step 2: Resolve imports if present
        if !schema.imports.is_empty() {
            use crate::parser::import_resolver_v2::ImportResolverV2;
            use linkml_core::settings::ImportSettings;

            // Configure import settings with base path
            let mut settings = ImportSettings::default();
            if let Some(parent) = path.parent() {
                settings
                    .search_paths
                    .push(parent.to_string_lossy().to_string());
            }
            // Also add current directory as fallback
            settings.search_paths.push(".".to_string());

            // Add standard LinkML aliases for common imports
            // linkml:types resolves to the standard LinkML types schema
            settings.aliases.insert(
                "linkml:types".to_string(),
                "https://w3id.org/linkml/types.yaml".to_string(),
            );
            settings.aliases.insert(
                "linkml:mappings".to_string(),
                "https://w3id.org/linkml/mappings.yaml".to_string(),
            );
            settings.aliases.insert(
                "linkml:extensions".to_string(),
                "https://w3id.org/linkml/extensions.yaml".to_string(),
            );
            settings.aliases.insert(
                "linkml:annotations".to_string(),
                "https://w3id.org/linkml/annotations.yaml".to_string(),
            );

            // Resolve imports
            let resolver = ImportResolverV2::with_settings(settings);
            schema = resolver.resolve_imports(&schema).await?;
        }

        Ok(schema)
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

