//! SchemaSheets format generator
//!
//! Generates Excel files in SchemaSheets format from LinkML schemas,
//! enabling lossless roundtrip conversion.

use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::SchemaDefinition;
use std::path::Path;

/// Generator for SchemaSheets format Excel files
pub struct SchemaSheetsGenerator {
    /// Whether to include metadata sheets (prefixes, types, settings)
    include_metadata: bool,
}

impl SchemaSheetsGenerator {
    /// Create a new SchemaSheets generator
    pub fn new() -> Self {
        Self {
            include_metadata: true,
        }
    }

    /// Set whether to include metadata sheets
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Generate a SchemaSheets Excel file from a LinkML schema
    ///
    /// # Arguments
    ///
    /// * `schema` - The LinkML schema to convert
    /// * `output_path` - Path where the Excel file should be written
    ///
    /// # Returns
    ///
    /// Ok(()) on success
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Excel file cannot be created
    /// - Schema contains unsupported features
    pub async fn generate_file(&self, schema: &SchemaDefinition, output_path: &Path) -> Result<()> {
        // TODO: Implement SchemaSheets format generation
        // This will be implemented in the next phase
        Err(LinkMLError::not_implemented(
            "SchemaSheets format generation not yet implemented",
        ))
    }
}

impl Default for SchemaSheetsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

