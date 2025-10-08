//! SchemaSheets format generator - simplified version

use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::{
    PermissibleValue, PrefixDefinition, SchemaDefinition,
};
use rust_xlsxwriter::{Color, DataValidation, Format, FormatAlign, FormatBorder, Workbook, XlsxError};
use std::path::Path;

/// Helper trait to convert XlsxError to LinkMLError with context
trait XlsxResultExt<T> {
    fn with_context(self, context: impl Into<String>) -> Result<T>;
}

impl<T> XlsxResultExt<T> for std::result::Result<T, XlsxError> {
    fn with_context(self, context: impl Into<String>) -> Result<T> {
        self.map_err(|e| {
            let message = format!("{}: {}", context.into(), e);
            LinkMLError::other(message)
        })
    }
}

/// Generator for SchemaSheets format Excel files
///
/// This generator creates Excel files in the SchemaSheets format from LinkML schemas,
/// enabling bidirectional conversion and lossless roundtrip transformation.
#[derive(Debug, Clone)]
pub struct SchemaSheetsGenerator {
    /// Whether to include all metadata columns (mappings, constraints, etc.)
    ///
    /// When `true`, the generator includes mapping columns (exact_mappings, close_mappings, etc.)
    /// and other extended metadata. When `false`, only basic columns are included.
    pub include_all_metadata: bool,

    /// Whether to generate metadata sheets (prefixes, settings)
    ///
    /// When `true`, the generator creates separate sheets for prefixes and settings.
    /// When `false`, only the main schema sheet is generated.
    pub generate_metadata_sheets: bool,

    /// Whether to freeze header rows for easier scrolling
    ///
    /// When `true`, the first row (headers) is frozen so it remains visible when scrolling.
    pub freeze_headers: bool,

    /// Whether to add auto-filters to column headers
    ///
    /// When `true`, auto-filter dropdowns are added to all column headers.
    pub add_filters: bool,

    /// Whether to use alternating row colors for better readability
    ///
    /// When `true`, alternating rows have different background colors.
    pub alternating_row_colors: bool,

    /// Whether to auto-size columns based on content
    ///
    /// When `true`, column widths are automatically adjusted to fit content.
    pub auto_size_columns: bool,

    /// Whether to add data validation dropdowns
    ///
    /// When `true`, adds dropdown lists for enum fields, element_type, multiplicity, and boolean fields.
    /// This improves data integrity and user experience.
    pub add_data_validation: bool,
}

impl SchemaSheetsGenerator {
    /// Create a new generator with default settings
    ///
    /// Default settings include all metadata columns, metadata sheets, and advanced formatting.
    ///
    /// # Examples
    ///
    /// ```
    /// use linkml_service::schemasheets::SchemaSheetsGenerator;
    ///
    /// let generator = SchemaSheetsGenerator::new();
    /// assert!(generator.include_all_metadata);
    /// assert!(generator.generate_metadata_sheets);
    /// assert!(generator.freeze_headers);
    /// assert!(generator.add_filters);
    /// ```
    pub fn new() -> Self {
        Self {
            include_all_metadata: true,
            generate_metadata_sheets: true,
            freeze_headers: true,
            add_filters: true,
            alternating_row_colors: true,
            auto_size_columns: true,
            add_data_validation: true,
        }
    }

    /// Generate a SchemaSheets format Excel file from a schema
    ///
    /// This method creates an Excel file in the SchemaSheets format, which can be
    /// parsed back into a schema using `SchemaSheetsParser`.
    ///
    /// # Arguments
    ///
    /// * `schema` - The LinkML schema to convert to Excel format
    /// * `output_path` - The path where the Excel file should be saved
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the file was successfully generated, or an error if
    /// the operation failed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use linkml_core::types::SchemaDefinition;
    /// use linkml_service::schemasheets::SchemaSheetsGenerator;
    /// use std::path::Path;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = SchemaDefinition::default();
    /// let generator = SchemaSheetsGenerator::new();
    /// generator.generate_file(&schema, Path::new("schema.xlsx")).await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The output path is invalid or inaccessible
    /// - The Excel file cannot be created or written
    /// - The schema contains invalid data
    pub async fn generate_file(&self, schema: &SchemaDefinition, output_path: &Path) -> Result<()> {
        let mut workbook = Workbook::new();

        // Generate main schema sheet
        let sheet = workbook.add_worksheet();
        sheet.set_name("Schema")
            .with_context("Failed to set worksheet name")?;

        // Create formats
        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(0x4472C4))
            .set_font_color(Color::White)
            .set_align(FormatAlign::Center)
            .set_border(FormatBorder::Thin);

        let class_format = Format::new()
            .set_background_color(Color::RGB(0xE7E6E6))
            .set_border(FormatBorder::Thin);

        let enum_format = Format::new()
            .set_background_color(Color::RGB(0xFFF2CC))
            .set_border(FormatBorder::Thin);

        let type_format = Format::new()
            .set_background_color(Color::RGB(0xD9E1F2))
            .set_border(FormatBorder::Thin);

        let subset_format = Format::new()
            .set_background_color(Color::RGB(0xE2EFDA))
            .set_border(FormatBorder::Thin);

        let alt_row_format = Format::new()
            .set_background_color(Color::RGB(0xF2F2F2))
            .set_border(FormatBorder::Thin);

        let normal_format = Format::new()
            .set_border(FormatBorder::Thin);

        // Write header
        let mut headers = vec![">", "element_type", "field", "key", "multiplicity", "range", "desc", "is_a", "pattern"];

        // Add mapping columns if metadata is enabled
        if self.include_all_metadata {
            headers.extend_from_slice(&[
                "schema.org:exactMatch",
                "skos:closeMatch",
                "skos:relatedMatch",
                "skos:narrowMatch",
                "skos:broadMatch",
            ]);
        }

        for (col, header) in headers.iter().enumerate() {
            sheet.write_with_format(0, col as u16, *header, &header_format)
                .with_context(format!("Failed to write header column {}", col))?;
        }
        
        let mut row = 1;

        // Write classes
        for (class_name, class_def) in &schema.classes {
            // Determine row format (alternating colors if enabled)
            let row_format = if self.alternating_row_colors && row % 2 == 0 {
                &alt_row_format
            } else {
                &normal_format
            };

            sheet.write_with_format(row, 0, class_name, &class_format)
                .with_context(format!("Failed to write class name '{}' at row {}", class_name, row))?;
            sheet.write_with_format(row, 1, "class", &class_format)
                .with_context(format!("Failed to write element_type for class '{}' at row {}", class_name, row))?;
            if let Some(ref desc) = class_def.description {
                sheet.write_with_format(row, 6, desc, row_format)
                    .with_context(format!("Failed to write description for class '{}' at row {}", class_name, row))?;
            }
            if let Some(ref parent) = class_def.is_a {
                sheet.write_with_format(row, 7, parent, row_format)
                    .with_context(format!("Failed to write is_a for class '{}' at row {}", class_name, row))?;
            }

            // Write mappings if metadata is enabled
            if self.include_all_metadata {
                let mut col = 9;
                if !class_def.exact_mappings.is_empty() {
                    sheet.write_with_format(row, col, class_def.exact_mappings.join(", "), row_format)
                        .with_context(format!("Failed to write exact_mappings for class '{}' at row {}", class_name, row))?;
                }
                col += 1;
                if !class_def.close_mappings.is_empty() {
                    sheet.write_with_format(row, col, class_def.close_mappings.join(", "), row_format)
                        .with_context(format!("Failed to write close_mappings for class '{}' at row {}", class_name, row))?;
                }
                col += 1;
                if !class_def.related_mappings.is_empty() {
                    sheet.write_with_format(row, col, class_def.related_mappings.join(", "), row_format)
                        .with_context(format!("Failed to write related_mappings for class '{}' at row {}", class_name, row))?;
                }
                col += 1;
                if !class_def.narrow_mappings.is_empty() {
                    sheet.write_with_format(row, col, class_def.narrow_mappings.join(", "), row_format)
                        .with_context(format!("Failed to write narrow_mappings for class '{}' at row {}", class_name, row))?;
                }
                col += 1;
                if !class_def.broad_mappings.is_empty() {
                    sheet.write_with_format(row, col, class_def.broad_mappings.join(", "), row_format)
                        .with_context(format!("Failed to write broad_mappings for class '{}' at row {}", class_name, row))?;
                }
            }

            row += 1;
            
            // Write attributes
            for (attr_name, attr_def) in &class_def.attributes {
                let row_format = if self.alternating_row_colors && row % 2 == 0 {
                    &alt_row_format
                } else {
                    &normal_format
                };

                sheet.write_with_format(row, 2, attr_name, row_format)
                    .with_context(format!("Failed to write attribute name '{}' for class '{}' at row {}", attr_name, class_name, row))?;
                if attr_def.identifier == Some(true) {
                    sheet.write_with_format(row, 3, "true", row_format)
                        .with_context(format!("Failed to write identifier flag for attribute '{}' at row {}", attr_name, row))?;
                }
                let mult = match (attr_def.required.unwrap_or(false), attr_def.multivalued.unwrap_or(false)) {
                    (true, false) => "1",
                    (false, false) => "0..1",
                    (true, true) => "1..*",
                    (false, true) => "0..*",
                };
                sheet.write_with_format(row, 4, mult, row_format)
                    .with_context(format!("Failed to write multiplicity for attribute '{}' at row {}", attr_name, row))?;
                if let Some(ref range) = attr_def.range {
                    sheet.write_with_format(row, 5, range, row_format)
                        .with_context(format!("Failed to write range for attribute '{}' at row {}", attr_name, row))?;
                }
                if let Some(ref desc) = attr_def.description {
                    sheet.write_with_format(row, 6, desc, row_format)
                        .with_context(format!("Failed to write description for attribute '{}' at row {}", attr_name, row))?;
                }
                if let Some(ref pattern) = attr_def.pattern {
                    sheet.write_with_format(row, 8, pattern, row_format)
                        .with_context(format!("Failed to write pattern for attribute '{}' at row {}", attr_name, row))?;
                }

                // Write mappings if metadata is enabled
                if self.include_all_metadata {
                    let mut col = 9;
                    if !attr_def.exact_mappings.is_empty() {
                        sheet.write_with_format(row, col, attr_def.exact_mappings.join(", "), row_format)
                            .with_context(format!("Failed to write exact_mappings for attribute '{}' at row {}", attr_name, row))?;
                    }
                    col += 1;
                    if !attr_def.close_mappings.is_empty() {
                        sheet.write_with_format(row, col, attr_def.close_mappings.join(", "), row_format)
                            .with_context(format!("Failed to write close_mappings for attribute '{}' at row {}", attr_name, row))?;
                    }
                    col += 1;
                    if !attr_def.related_mappings.is_empty() {
                        sheet.write_with_format(row, col, attr_def.related_mappings.join(", "), row_format)
                            .with_context(format!("Failed to write related_mappings for attribute '{}' at row {}", attr_name, row))?;
                    }
                    col += 1;
                    if !attr_def.narrow_mappings.is_empty() {
                        sheet.write_with_format(row, col, attr_def.narrow_mappings.join(", "), row_format)
                            .with_context(format!("Failed to write narrow_mappings for attribute '{}' at row {}", attr_name, row))?;
                    }
                    col += 1;
                    if !attr_def.broad_mappings.is_empty() {
                        sheet.write_with_format(row, col, attr_def.broad_mappings.join(", "), row_format)
                            .with_context(format!("Failed to write broad_mappings for attribute '{}' at row {}", attr_name, row))?;
                    }
                }

                row += 1;
            }
        }
        
        // Write enums
        for (enum_name, enum_def) in &schema.enums {
            let row_format = if self.alternating_row_colors && row % 2 == 0 {
                &alt_row_format
            } else {
                &normal_format
            };

            sheet.write_with_format(row, 0, enum_name, &enum_format)
                .with_context(format!("Failed to write enum name '{}' at row {}", enum_name, row))?;
            sheet.write_with_format(row, 1, "enum", &enum_format)
                .with_context(format!("Failed to write element_type for enum '{}' at row {}", enum_name, row))?;
            if let Some(ref desc) = enum_def.description {
                sheet.write_with_format(row, 6, desc, row_format)
                    .with_context(format!("Failed to write description for enum '{}' at row {}", enum_name, row))?;
            }
            row += 1;

            for pv in &enum_def.permissible_values {
                let row_format = if self.alternating_row_colors && row % 2 == 0 {
                    &alt_row_format
                } else {
                    &normal_format
                };

                let (value, desc) = match pv {
                    PermissibleValue::Simple(v) => (v.clone(), None),
                    PermissibleValue::Complex { text, description, .. } => (text.clone(), description.clone()),
                };
                sheet.write_with_format(row, 2, &value, row_format)
                    .with_context(format!("Failed to write enum value '{}' for enum '{}' at row {}", value, enum_name, row))?;
                if let Some(ref d) = desc {
                    sheet.write_with_format(row, 6, d, row_format)
                        .with_context(format!("Failed to write description for enum value '{}' at row {}", value, row))?;
                }
                row += 1;
            }
        }

        // Write types
        for (type_name, type_def) in &schema.types {
            let row_format = if self.alternating_row_colors && row % 2 == 0 {
                &alt_row_format
            } else {
                &normal_format
            };

            sheet.write_with_format(row, 0, type_name, &type_format)
                .with_context(format!("Failed to write type name '{}' at row {}", type_name, row))?;
            sheet.write_with_format(row, 1, "type", &type_format)
                .with_context(format!("Failed to write element_type for type '{}' at row {}", type_name, row))?;
            if let Some(ref desc) = type_def.description {
                sheet.write_with_format(row, 6, desc, row_format)
                    .with_context(format!("Failed to write description for type '{}' at row {}", type_name, row))?;
            }
            if let Some(ref base_type) = type_def.base_type {
                sheet.write_with_format(row, 7, base_type, row_format)
                    .with_context(format!("Failed to write base_type for type '{}' at row {}", type_name, row))?;
            }
            if let Some(ref pattern) = type_def.pattern {
                sheet.write_with_format(row, 8, pattern, row_format)
                    .with_context(format!("Failed to write pattern for type '{}' at row {}", type_name, row))?;
            }
            row += 1;
        }

        // Write subsets
        for (subset_name, subset_def) in &schema.subsets {
            let row_format = if self.alternating_row_colors && row % 2 == 0 {
                &alt_row_format
            } else {
                &normal_format
            };

            sheet.write_with_format(row, 0, subset_name, &subset_format)
                .with_context(format!("Failed to write subset name '{}' at row {}", subset_name, row))?;
            sheet.write_with_format(row, 1, "subset", &subset_format)
                .with_context(format!("Failed to write element_type for subset '{}' at row {}", subset_name, row))?;
            if let Some(ref desc) = subset_def.description {
                sheet.write_with_format(row, 6, desc, row_format)
                    .with_context(format!("Failed to write description for subset '{}' at row {}", subset_name, row))?;
            }
            row += 1;
        }

        // Apply formatting to main schema sheet before creating new worksheets
        if self.freeze_headers {
            sheet.set_freeze_panes(1, 0)
                .with_context("Failed to freeze schema sheet headers")?;
        }

        if self.add_filters {
            let last_col = if self.include_all_metadata { 13 } else { 8 };
            sheet.autofilter(0, 0, 0, last_col)
                .with_context("Failed to set autofilter on schema sheet")?;
        }

        if self.auto_size_columns {
            for col in 0..=8 {
                let width = match col {
                    0 => 30.0, // Element name
                    1 => 15.0, // Element type
                    2 => 20.0, // Slot name
                    3 => 15.0, // Range
                    4 => 10.0, // Required
                    5 => 10.0, // Multivalued
                    6 => 40.0, // Description
                    7 => 20.0, // Is_a
                    8 => 20.0, // Pattern
                    _ => 15.0,
                };
                sheet.set_column_width(col, width)
                    .with_context(format!("Failed to set column {} width", col))?;
            }
        }

        // Add data validation if enabled
        if self.add_data_validation {
            self.add_data_validations(sheet, schema)?;
        }

        // Generate metadata sheets
        if self.generate_metadata_sheets {
            // Prefixes sheet - use a block to ensure the borrow ends before creating settings sheet
            {
                let prefixes_sheet = workbook.add_worksheet();
                prefixes_sheet.set_name("prefixes")
                    .with_context("Failed to set prefixes sheet name")?;
                prefixes_sheet.write_with_format(0, 0, "prefix", &header_format)
                    .with_context("Failed to write prefixes header 'prefix'")?;
                prefixes_sheet.write_with_format(0, 1, "uri", &header_format)
                    .with_context("Failed to write prefixes header 'uri'")?;
                let mut row = 1;
                for (prefix, definition) in &schema.prefixes {
                    prefixes_sheet.write(row, 0, prefix)
                        .with_context(format!("Failed to write prefix '{}' at row {}", prefix, row))?;
                    let uri = match definition {
                        PrefixDefinition::Simple(uri) => uri.clone(),
                        PrefixDefinition::Complex { prefix_reference, .. } => prefix_reference.clone().unwrap_or_default(),
                    };
                    prefixes_sheet.write(row, 1, uri)
                        .with_context(format!("Failed to write URI for prefix '{}' at row {}", prefix, row))?;
                    row += 1;
                }

                // Apply formatting to prefixes sheet
                if self.freeze_headers {
                    prefixes_sheet.set_freeze_panes(1, 0)
                        .with_context("Failed to freeze prefixes sheet headers")?;
                }

                if self.add_filters {
                    prefixes_sheet.autofilter(0, 0, 0, 1)
                        .with_context("Failed to add autofilter to prefixes sheet")?;
                }

                if self.auto_size_columns {
                    prefixes_sheet.set_column_width(0, 15)
                        .with_context("Failed to set prefix column width")?;
                    prefixes_sheet.set_column_width(1, 50)
                        .with_context("Failed to set URI column width")?;
                }
            } // prefixes_sheet borrow ends here

            // Settings sheet
            let settings_sheet = workbook.add_worksheet();
            settings_sheet.set_name("settings")
                .with_context("Failed to set settings sheet name")?;
            settings_sheet.write_with_format(0, 0, "setting", &header_format)
                .with_context("Failed to write settings header 'setting'")?;
            settings_sheet.write_with_format(0, 1, "value", &header_format)
                .with_context("Failed to write settings header 'value'")?;
            let mut row = 1;
            settings_sheet.write(row, 0, "id")
                .with_context("Failed to write 'id' setting name")?;
            settings_sheet.write(row, 1, &schema.id)
                .with_context("Failed to write schema ID value")?;
            row += 1;
            settings_sheet.write(row, 0, "name")
                .with_context("Failed to write 'name' setting name")?;
            settings_sheet.write(row, 1, &schema.name)
                .with_context("Failed to write schema name value")?;
            row += 1;
            if let Some(ref version) = schema.version {
                settings_sheet.write(row, 0, "version")
                    .with_context("Failed to write 'version' setting name")?;
                settings_sheet.write(row, 1, version)
                    .with_context("Failed to write schema version value")?;
                row += 1;
            }
            if let Some(ref description) = schema.description {
                settings_sheet.write(row, 0, "description")
                    .with_context("Failed to write 'description' setting name")?;
                settings_sheet.write(row, 1, description)
                    .with_context("Failed to write schema description value")?;
            }

            // Apply formatting to settings sheet
            if self.freeze_headers {
                settings_sheet.set_freeze_panes(1, 0)
                    .with_context("Failed to freeze settings sheet headers")?;
            }

            if self.add_filters {
                settings_sheet.autofilter(0, 0, 0, 1)
                    .with_context("Failed to add autofilter to settings sheet")?;
            }

            if self.auto_size_columns {
                settings_sheet.set_column_width(0, 15)
                    .with_context("Failed to set setting column width")?;
                settings_sheet.set_column_width(1, 50)
                    .with_context("Failed to set value column width")?;
            }
        }

        workbook.save(output_path).map_err(|e| LinkMLError::other(format!("Failed to save Excel file: {e}")))?;

        Ok(())
    }

    /// Add data validation dropdowns to the schema sheet
    fn add_data_validations(&self, sheet: &mut rust_xlsxwriter::Worksheet, schema: &SchemaDefinition) -> Result<()> {
        // 1. Add validation for element_type column (column 1)
        let element_types = vec!["class", "enum", "type", "subset"];
        let element_type_validation = DataValidation::new()
            .allow_list_strings(&element_types)
            .map_err(|e| LinkMLError::other(format!("Failed to create element type validation: {}", e)))?
            .set_error_title("Invalid Element Type")
            .map_err(|e| LinkMLError::other(format!("Failed to set error title: {}", e)))?
            .set_error_message("Please select one of: class, enum, type, subset")
            .map_err(|e| LinkMLError::other(format!("Failed to set error message: {}", e)))?;

        sheet.add_data_validation(1, 1, 1_048_575, 1, &element_type_validation)
            .with_context("Failed to add element_type validation")?;

        // 2. Add validation for key column (column 3) - boolean values
        let boolean_values = vec!["true", "false"];
        let key_validation = DataValidation::new()
            .allow_list_strings(&boolean_values)
            .map_err(|e| LinkMLError::other(format!("Failed to create key validation: {}", e)))?
            .set_error_title("Invalid Key Value")
            .map_err(|e| LinkMLError::other(format!("Failed to set error title: {}", e)))?
            .set_error_message("Please select 'true' or 'false'")
            .map_err(|e| LinkMLError::other(format!("Failed to set error message: {}", e)))?;

        sheet.add_data_validation(1, 3, 1_048_575, 3, &key_validation)
            .with_context("Failed to add key validation")?;

        // 3. Add validation for multiplicity column (column 4)
        let multiplicity_values = vec!["1", "0..1", "1..*", "0..*"];
        let multiplicity_validation = DataValidation::new()
            .allow_list_strings(&multiplicity_values)
            .map_err(|e| LinkMLError::other(format!("Failed to create multiplicity validation: {}", e)))?
            .set_error_title("Invalid Multiplicity")
            .map_err(|e| LinkMLError::other(format!("Failed to set error title: {}", e)))?
            .set_error_message("Please select one of: 1, 0..1, 1..*, 0..*")
            .map_err(|e| LinkMLError::other(format!("Failed to set error message: {}", e)))?;

        sheet.add_data_validation(1, 4, 1_048_575, 4, &multiplicity_validation)
            .with_context("Failed to add multiplicity validation")?;

        // 4. Add validation for range column (column 5) - enum types
        // Collect all enum names and common types
        let mut range_values: Vec<String> = schema.enums.keys().cloned().collect();
        range_values.extend_from_slice(&[
            "string".to_string(),
            "integer".to_string(),
            "float".to_string(),
            "double".to_string(),
            "boolean".to_string(),
            "date".to_string(),
            "datetime".to_string(),
            "uri".to_string(),
        ]);
        range_values.sort();

        if !range_values.is_empty() {
            let range_validation = DataValidation::new()
                .allow_list_strings(&range_values.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                .map_err(|e| LinkMLError::other(format!("Failed to create range validation: {}", e)))?
                .set_input_title("Select Range Type")
                .map_err(|e| LinkMLError::other(format!("Failed to set input title: {}", e)))?
                .set_input_message("Select a data type or enum name")
                .map_err(|e| LinkMLError::other(format!("Failed to set input message: {}", e)))?;

            sheet.add_data_validation(1, 5, 1_048_575, 5, &range_validation)
                .with_context("Failed to add range validation")?;
        }

        Ok(())
    }
}

impl Default for SchemaSheetsGenerator {
    fn default() -> Self {
        Self::new()
    }
}
