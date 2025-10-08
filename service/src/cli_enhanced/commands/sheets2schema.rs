//! `sheets2schema` command implementation
//!
//! Converts Excel SchemaSheets to LinkML schema.
//!
//! Supports two modes:
//! 1. **SchemaSheets Format**: Parses metadata from SchemaSheets-formatted Excel files
//!    for lossless roundtrip conversion (preserves all schema metadata)
//! 2. **Data Introspection**: Analyzes data and infers schema structure
//!    (generates new schema from data patterns)

use crate::inference::introspectors::excel::ExcelIntrospector;
use crate::inference::DataIntrospector;
use crate::schemasheets::SchemaSheetsParser;
use indicatif::{ProgressBar, ProgressStyle};
use linkml_core::error::{LinkMLError, Result};
use linkml_core::prelude::*;
use logger_service::wiring::wire_logger;
use std::path::{Path, PathBuf};
use timestamp_service::wiring::wire_timestamp;

/// Command for converting Excel SchemaSheets to LinkML schema
pub struct Sheets2SchemaCommand {
    /// Input Excel file path
    pub input: PathBuf,
    /// Output schema file path
    pub output: Option<PathBuf>,
    /// Schema ID
    pub schema_id: Option<String>,
    /// Schema name
    pub schema_name: Option<String>,
    /// Output format
    pub format: SchemaFormat,
    /// Force SchemaSheets format parsing (default: auto-detect)
    pub force_schemasheets: bool,
    /// Force data introspection mode (default: auto-detect)
    pub force_introspection: bool,
    /// Show progress indicators
    pub progress: bool,
    /// Verbose output
    pub verbose: bool,
}

/// Schema output format
#[derive(Debug, Clone, Copy)]
pub enum SchemaFormat {
    /// YAML format
    Yaml,
    /// JSON format
    Json,
}

impl Sheets2SchemaCommand {
    /// Create a new sheets2schema command
    #[must_use]
    pub fn new(input: PathBuf, output: Option<PathBuf>) -> Self {
        Self {
            input,
            output,
            schema_id: None,
            schema_name: None,
            format: SchemaFormat::Yaml,
            force_schemasheets: false,
            force_introspection: false,
            progress: true,
            verbose: false,
        }
    }

    /// Force SchemaSheets format parsing
    #[must_use]
    pub fn with_schemasheets_format(mut self, force: bool) -> Self {
        self.force_schemasheets = force;
        self
    }

    /// Force data introspection mode
    #[must_use]
    pub fn with_introspection_mode(mut self, force: bool) -> Self {
        self.force_introspection = force;
        self
    }

    /// Set schema ID
    #[must_use]
    pub fn with_schema_id(mut self, schema_id: String) -> Self {
        self.schema_id = Some(schema_id);
        self
    }

    /// Set schema name
    #[must_use]
    pub fn with_schema_name(mut self, schema_name: String) -> Self {
        self.schema_name = Some(schema_name);
        self
    }

    /// Set output format
    #[must_use]
    pub fn with_format(mut self, format: SchemaFormat) -> Self {
        self.format = format;
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
    /// - Input file doesn't exist or can't be read
    /// - Excel file is invalid or corrupted
    /// - Schema generation fails
    /// - Output file can't be written
    pub async fn execute(&self) -> Result<()> {
        // Validate input file exists
        if !self.input.exists() {
            return Err(LinkMLError::io_error(format!(
                "Input file not found: {}",
                self.input.display()
            )));
        }

        // Determine output path
        let output_path = self.determine_output_path();

        // Determine schema ID
        let schema_id = self.determine_schema_id();

        // Create progress bar if enabled
        let progress = if self.progress {
            let pb = ProgressBar::new(4);
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

        // Detect format and parse accordingly
        let schema = if self.force_schemasheets {
            // Force SchemaSheets format
            self.parse_schemasheets_format(&schema_id, &progress).await?
        } else if self.force_introspection {
            // Force data introspection
            self.parse_via_introspection(&schema_id, &progress).await?
        } else {
            // Auto-detect format
            self.auto_detect_and_parse(&schema_id, &progress).await?
        };

        if let Some(pb) = progress.as_ref() {
            pb.inc(1);
        }

        // Step 4: Write output
        if let Some(pb) = progress.as_ref() {
            pb.set_message(format!("Writing schema to {}...", output_path.display()));
        } else if self.verbose {
            eprintln!("Writing schema to: {}", output_path.display());
        }

        self.write_schema(&schema, &output_path)?;

        if let Some(pb) = progress.as_ref() {
            pb.inc(1);
            pb.finish_with_message(format!("✓ Schema generated: {}", output_path.display()));
        } else {
            println!("Schema generated: {}", output_path.display());
        }

        // Print summary if verbose
        if self.verbose {
            eprintln!("\nSchema Summary:");
            eprintln!("  ID: {}", schema.id);
            eprintln!("  Name: {}", schema.name);
            eprintln!("  Classes: {}", schema.classes.len());
            eprintln!("  Enums: {}", schema.enums.len());
            eprintln!("  Slots: {}", schema.slots.len());
        }

        Ok(())
    }

    /// Parse using SchemaSheets format parser
    async fn parse_schemasheets_format(
        &self,
        schema_id: &str,
        progress: &Option<ProgressBar>,
    ) -> Result<SchemaDefinition> {
        if let Some(pb) = progress.as_ref() {
            pb.set_message("Parsing SchemaSheets format...");
        } else if self.verbose {
            eprintln!("Parsing SchemaSheets format...");
        }

        let parser = SchemaSheetsParser::new();
        let schema = parser.parse_file(&self.input, Some(schema_id)).await?;

        if let Some(pb) = progress.as_ref() {
            pb.inc(3); // Skip to final step
        }

        Ok(schema)
    }

    /// Parse using data introspection
    async fn parse_via_introspection(
        &self,
        schema_id: &str,
        progress: &Option<ProgressBar>,
    ) -> Result<SchemaDefinition> {
        if let Some(pb) = progress.as_ref() {
            pb.set_message("Reading Excel file...");
        } else if self.verbose {
            eprintln!("Reading Excel file: {}", self.input.display());
        }

        // Wire services
        let timestamp = wire_timestamp().into_arc();
        let logger = wire_logger(timestamp.clone(), logger_core::LoggerConfig::default())
            .map_err(|e| LinkMLError::service(format!("Failed to wire logger: {e}")))?
            .into_arc();

        let introspector = ExcelIntrospector::new(logger, timestamp);

        if let Some(pb) = progress.as_ref() {
            pb.inc(1);
        }

        // Analyze Excel data
        if let Some(pb) = progress.as_ref() {
            pb.set_message("Analyzing data and inferring schema...");
        } else if self.verbose {
            eprintln!("Analyzing data and inferring schema...");
        }

        let stats = introspector
            .analyze_file(&self.input)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to analyze Excel file: {e}")))?;

        if let Some(pb) = progress.as_ref() {
            pb.inc(1);
        }

        // Generate schema
        if let Some(pb) = progress.as_ref() {
            pb.set_message("Generating LinkML schema...");
        } else if self.verbose {
            eprintln!("Generating LinkML schema...");
        }

        let schema = introspector
            .generate_schema(&stats, schema_id)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to generate schema: {e}")))?;

        if let Some(pb) = progress.as_ref() {
            pb.inc(1);
        }

        Ok(schema)
    }

    /// Auto-detect format and parse accordingly
    async fn auto_detect_and_parse(
        &self,
        schema_id: &str,
        progress: &Option<ProgressBar>,
    ) -> Result<SchemaDefinition> {
        // Try SchemaSheets format first
        if let Some(pb) = progress.as_ref() {
            pb.set_message("Detecting format...");
        } else if self.verbose {
            eprintln!("Detecting format...");
        }

        // Attempt to parse as SchemaSheets
        let parser = SchemaSheetsParser::new();
        match parser.parse_file(&self.input, Some(schema_id)).await {
            Ok(schema) => {
                if self.verbose {
                    eprintln!("✓ Detected SchemaSheets format");
                }
                if let Some(pb) = progress.as_ref() {
                    pb.inc(3);
                }
                Ok(schema)
            }
            Err(_) => {
                // Fall back to data introspection
                if self.verbose {
                    eprintln!("✓ Using data introspection mode");
                }
                self.parse_via_introspection(schema_id, progress).await
            }
        }
    }

    /// Determine output path based on input and options
    fn determine_output_path(&self) -> PathBuf {
        if let Some(ref output) = self.output {
            output.clone()
        } else {
            // Default: <input>.yaml or <input>.json
            let mut path = self.input.clone();
            path.set_extension(match self.format {
                SchemaFormat::Yaml => "yaml",
                SchemaFormat::Json => "json",
            });
            path
        }
    }

    /// Determine schema ID from options or filename
    fn determine_schema_id(&self) -> String {
        if let Some(ref id) = self.schema_id {
            id.clone()
        } else {
            // Use filename without extension
            self.input
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("schema")
                .to_string()
        }
    }

    /// Write schema to file in specified format
    fn write_schema(&self, schema: &SchemaDefinition, path: &Path) -> Result<()> {
        let content = match self.format {
            SchemaFormat::Yaml => serde_yaml::to_string(schema)
                .map_err(|e| LinkMLError::serialization(format!("YAML serialization failed: {e}")))?,
            SchemaFormat::Json => serde_json::to_string_pretty(schema)
                .map_err(|e| LinkMLError::serialization(format!("JSON serialization failed: {e}")))?,
        };

        std::fs::write(path, content)
            .map_err(|e| LinkMLError::io_error(format!("Failed to write file: {e}")))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_output_path_default_yaml() {
        let cmd = Sheets2SchemaCommand::new(PathBuf::from("data.xlsx"), None);
        assert_eq!(cmd.determine_output_path(), PathBuf::from("data.yaml"));
    }

    #[test]
    fn test_determine_output_path_default_json() {
        let cmd = Sheets2SchemaCommand::new(PathBuf::from("data.xlsx"), None)
            .with_format(SchemaFormat::Json);
        assert_eq!(cmd.determine_output_path(), PathBuf::from("data.json"));
    }

    #[test]
    fn test_determine_output_path_explicit() {
        let cmd = Sheets2SchemaCommand::new(
            PathBuf::from("data.xlsx"),
            Some(PathBuf::from("schema.yaml")),
        );
        assert_eq!(cmd.determine_output_path(), PathBuf::from("schema.yaml"));
    }

    #[test]
    fn test_determine_schema_id_from_filename() {
        let cmd = Sheets2SchemaCommand::new(PathBuf::from("my_data.xlsx"), None);
        assert_eq!(cmd.determine_schema_id(), "my_data");
    }

    #[test]
    fn test_determine_schema_id_explicit() {
        let cmd = Sheets2SchemaCommand::new(PathBuf::from("data.xlsx"), None)
            .with_schema_id("custom_schema".to_string());
        assert_eq!(cmd.determine_schema_id(), "custom_schema");
    }
}

