//! Configuration for SchemaSheets generator
//!
//! This module provides configuration structures for the SchemaSheets Excel generator,
//! allowing customization of column widths, colors, validation rules, and other settings.

use serde::{Deserialize, Serialize};

/// Complete configuration for SchemaSheets generator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
#[derive(Default)]
pub struct SchemaSheetsConfig {
    /// Column width configuration
    pub column_widths: ColumnWidthConfig,

    /// Color scheme configuration
    pub colors: ColorSchemeConfig,

    /// Data validation configuration
    pub validation: ValidationConfig,

    /// Excel limits and constraints
    pub limits: ExcelLimitsConfig,
}

/// Column width configuration for SchemaSheets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnWidthConfig {
    /// Width for element name column (column 0)
    pub element_name: f64,

    /// Width for element type column (column 1)
    pub element_type: f64,

    /// Width for field/slot name column (column 2)
    pub field_name: f64,

    /// Width for key/identifier column (column 3)
    pub key: f64,

    /// Width for multiplicity column (column 4)
    pub multiplicity: f64,

    /// Width for range/type column (column 5)
    pub range: f64,

    /// Width for description column (column 6)
    pub description: f64,

    /// Width for is_a/parent column (column 7)
    pub is_a: f64,

    /// Width for pattern column (column 8)
    pub pattern: f64,

    /// Width for mapping columns (columns 9-13)
    pub mappings: f64,

    /// Default width for any other columns
    pub default: f64,
}

impl Default for ColumnWidthConfig {
    fn default() -> Self {
        Self {
            element_name: 20.0,
            element_type: 15.0,
            field_name: 20.0,
            key: 8.0,
            multiplicity: 12.0,
            range: 15.0,
            description: 40.0,
            is_a: 15.0,
            pattern: 30.0,
            mappings: 25.0,
            default: 15.0,
        }
    }
}

/// Color scheme configuration for SchemaSheets
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorSchemeConfig {
    /// Header row color (RGB hex)
    pub header_background: String,

    /// Header text color (RGB hex)
    pub header_text: String,

    /// Class element background color (RGB hex)
    pub class_background: String,

    /// Enum element background color (RGB hex)
    pub enum_background: String,

    /// Type element background color (RGB hex)
    pub type_background: String,

    /// Subset element background color (RGB hex)
    pub subset_background: String,

    /// Alternating row background color (RGB hex)
    pub alt_row_background: String,
}

impl Default for ColorSchemeConfig {
    fn default() -> Self {
        Self {
            header_background: "4472C4".to_string(),
            header_text: "FFFFFF".to_string(),
            class_background: "E7E6E6".to_string(),
            enum_background: "FFF2CC".to_string(),
            type_background: "D9E1F2".to_string(),
            subset_background: "E2EFDA".to_string(),
            alt_row_background: "F2F2F2".to_string(),
        }
    }
}

impl ColorSchemeConfig {
    /// Parse hex color string to RGB u32
    pub fn parse_hex_color(&self, hex: &str) -> u32 {
        u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0x00FF_FFFF)
    }

    /// Get header background as RGB u32
    pub fn header_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.header_background)
    }

    /// Get header text as RGB u32
    pub fn header_text_rgb(&self) -> u32 {
        self.parse_hex_color(&self.header_text)
    }

    /// Get class background as RGB u32
    pub fn class_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.class_background)
    }

    /// Get enum background as RGB u32
    pub fn enum_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.enum_background)
    }

    /// Get type background as RGB u32
    pub fn type_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.type_background)
    }

    /// Get subset background as RGB u32
    pub fn subset_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.subset_background)
    }

    /// Get alternating row background as RGB u32
    pub fn alt_row_background_rgb(&self) -> u32 {
        self.parse_hex_color(&self.alt_row_background)
    }
}

/// Data validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ValidationConfig {
    /// Valid element types
    pub element_types: Vec<String>,

    /// Valid multiplicity values
    pub multiplicity_values: Vec<String>,

    /// Valid boolean values
    pub boolean_values: Vec<String>,

    /// Common data types for range validation
    pub common_types: Vec<String>,

    /// Error message for invalid element type
    pub element_type_error: String,

    /// Error message for invalid multiplicity
    pub multiplicity_error: String,

    /// Error message for invalid boolean value
    pub boolean_error: String,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            element_types: vec![
                "class".to_string(),
                "enum".to_string(),
                "type".to_string(),
                "subset".to_string(),
            ],
            multiplicity_values: vec![
                "1".to_string(),
                "0..1".to_string(),
                "1..*".to_string(),
                "0..*".to_string(),
            ],
            boolean_values: vec!["true".to_string(), "false".to_string()],
            common_types: vec![
                "string".to_string(),
                "integer".to_string(),
                "float".to_string(),
                "double".to_string(),
                "boolean".to_string(),
                "date".to_string(),
                "datetime".to_string(),
                "uri".to_string(),
            ],
            element_type_error: "Please select one of: class, enum, type, subset".to_string(),
            multiplicity_error: "Please select one of: 1, 0..1, 1..*, 0..*".to_string(),
            boolean_error: "Please select 'true' or 'false'".to_string(),
        }
    }
}

/// Excel limits and constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExcelLimitsConfig {
    /// Maximum number of rows in Excel (Excel 2007+ limit)
    pub max_rows: u32,

    /// Maximum number of columns in Excel
    pub max_columns: u16,
}

impl Default for ExcelLimitsConfig {
    fn default() -> Self {
        Self {
            max_rows: 1_048_575, // Excel 2007+ limit (1,048,576 rows - 1 for header)
            max_columns: 16_384, // Excel 2007+ limit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SchemaSheetsConfig::default();
        assert_eq!(config.column_widths.element_name, 20.0);
        assert_eq!(config.colors.header_background, "4472C4");
        assert_eq!(config.validation.element_types.len(), 4);
        assert_eq!(config.limits.max_rows, 1_048_575);
    }

    #[test]
    fn test_color_parsing() {
        let colors = ColorSchemeConfig::default();
        assert_eq!(colors.header_background_rgb(), 0x4472C4);
        assert_eq!(colors.class_background_rgb(), 0xE7E6E6);
        assert_eq!(colors.enum_background_rgb(), 0xFFF2CC);
    }

    #[test]
    fn test_color_parsing_with_hash() {
        let colors = ColorSchemeConfig {
            header_background: "#4472C4".to_string(),
            ..Default::default()
        };
        assert_eq!(colors.header_background_rgb(), 0x4472C4);
    }

    #[test]
    fn test_validation_config() {
        let validation = ValidationConfig::default();
        assert!(validation.element_types.contains(&"class".to_string()));
        assert!(validation.multiplicity_values.contains(&"1".to_string()));
        assert!(validation.boolean_values.contains(&"true".to_string()));
        assert!(validation.common_types.contains(&"string".to_string()));
    }
}
