//! Tests for SchemaSheets configuration

use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_service::schemasheets::config::ColorSchemeConfig;
use linkml_service::schemasheets::{SchemaSheetsConfig, SchemaSheetsGenerator};
use tempfile::TempDir;

/// Test default configuration
#[tokio::test]
async fn test_default_configuration() {
    let config = SchemaSheetsConfig::default();

    // Verify default column widths
    assert_eq!(config.column_widths.element_name, 20.0);
    assert_eq!(config.column_widths.description, 40.0);

    // Verify default colors
    assert_eq!(config.colors.header_background, "4472C4");
    assert_eq!(config.colors.class_background, "E7E6E6");

    // Verify default validation
    assert_eq!(config.validation.element_types.len(), 4);
    assert_eq!(config.validation.common_types.len(), 8);

    // Verify default limits
    assert_eq!(config.limits.max_rows, 1_048_575);
}

/// Test custom column widths
#[tokio::test]
async fn test_custom_column_widths() {
    let mut config = SchemaSheetsConfig::default();
    config.column_widths.element_name = 30.0;
    config.column_widths.description = 60.0;

    assert_eq!(config.column_widths.element_name, 30.0);
    assert_eq!(config.column_widths.description, 60.0);
}

/// Test custom color scheme
#[tokio::test]
async fn test_custom_color_scheme() {
    let mut config = SchemaSheetsConfig::default();
    config.colors.header_background = "FF0000".to_string();
    config.colors.class_background = "00FF00".to_string();

    assert_eq!(config.colors.header_background_rgb(), 0xFF0000);
    assert_eq!(config.colors.class_background_rgb(), 0x00FF00);
}

/// Test color parsing with hash prefix
#[tokio::test]
async fn test_color_parsing_with_hash() {
    let mut config = SchemaSheetsConfig::default();
    config.colors.header_background = "#4472C4".to_string();

    assert_eq!(config.colors.header_background_rgb(), 0x4472C4);
}

/// Test custom validation configuration
#[tokio::test]
async fn test_custom_validation_config() {
    let mut config = SchemaSheetsConfig::default();
    config
        .validation
        .common_types
        .push("custom_type".to_string());
    config.validation.element_type_error = "Custom error message".to_string();

    assert!(
        config
            .validation
            .common_types
            .contains(&"custom_type".to_string())
    );
    assert_eq!(config.validation.element_type_error, "Custom error message");
}

/// Test generator with custom configuration
#[tokio::test]
async fn test_generator_with_custom_config() {
    let mut config = SchemaSheetsConfig::default();
    config.column_widths.element_name = 35.0;
    config.colors.header_background = "2E5090".to_string();

    let generator = SchemaSheetsGenerator::with_config(config.clone());

    assert_eq!(generator.config.column_widths.element_name, 35.0);
    assert_eq!(generator.config.colors.header_background, "2E5090");
}

/// Test generating Excel file with custom configuration
#[tokio::test]
async fn test_generate_with_custom_config() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person".to_string()),
        ..Default::default()
    };

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };

    person_class
        .attributes
        .insert("name".to_string(), name_slot);
    schema.classes.insert("Person".to_string(), person_class);

    // Create custom configuration
    let mut config = SchemaSheetsConfig::default();
    config.column_widths.element_name = 40.0;
    config.colors.header_background = "FF5733".to_string();

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("custom_config_schema.xlsx");

    let generator = SchemaSheetsGenerator::with_config(config);
    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test configuration serialization to YAML
#[tokio::test]
async fn test_config_serialization() {
    let config = SchemaSheetsConfig::default();

    let yaml = serde_yaml::to_string(&config).unwrap();
    assert!(yaml.contains("column_widths"));
    assert!(yaml.contains("colors"));
    assert!(yaml.contains("validation"));
    assert!(yaml.contains("limits"));
}

/// Test configuration deserialization from YAML
#[tokio::test]
async fn test_config_deserialization() {
    let yaml = r#"
column_widths:
  element_name: 25.0
  element_type: 18.0
  field_name: 22.0
  key: 10.0
  multiplicity: 14.0
  range: 18.0
  description: 45.0
  is_a: 18.0
  pattern: 32.0
  mappings: 28.0
  default: 16.0

colors:
  header_background: "2E5090"
  header_text: "FFFFFF"
  class_background: "D9D9D9"
  enum_background: "FFE699"
  type_background: "B4C7E7"
  subset_background: "C6E0B4"
  alt_row_background: "E7E6E6"

validation:
  element_types:
    - "class"
    - "enum"
    - "type"
    - "subset"
  multiplicity_values:
    - "1"
    - "0..1"
    - "1..*"
    - "0..*"
  boolean_values:
    - "true"
    - "false"
  common_types:
    - "string"
    - "integer"
  element_type_error: "Custom error"
  multiplicity_error: "Custom multiplicity error"
  boolean_error: "Custom boolean error"

limits:
  max_rows: 1048575
  max_columns: 16384
"#;

    let config: SchemaSheetsConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.column_widths.element_name, 25.0);
    assert_eq!(config.colors.header_background, "2E5090");
    assert_eq!(config.validation.element_type_error, "Custom error");
    assert_eq!(config.limits.max_rows, 1_048_575);
}

/// Test loading configuration from file
#[tokio::test]
async fn test_load_config_from_file() {
    // Create a temporary config file
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test_config.yaml");

    let yaml = r#"
column_widths:
  element_name: 30.0
  description: 50.0
colors:
  header_background: "FF0000"
validation:
  element_types: ["class", "enum"]
limits:
  max_rows: 1048575
"#;

    std::fs::write(&config_path, yaml).unwrap();

    // Load configuration
    let contents = std::fs::read_to_string(&config_path).unwrap();
    let config: SchemaSheetsConfig = serde_yaml::from_str(&contents).unwrap();

    assert_eq!(config.column_widths.element_name, 30.0);
    assert_eq!(config.colors.header_background, "FF0000");
}

/// Test invalid color parsing
#[tokio::test]
async fn test_invalid_color_parsing() {
    let colors = ColorSchemeConfig {
        header_background: "INVALID".to_string(),
        ..Default::default()
    };

    // Invalid colors should default to white (0xFFFFFF)
    assert_eq!(colors.header_background_rgb(), 0xFFFFFF);
}

/// Test configuration with extended types
#[tokio::test]
async fn test_extended_types_config() {
    let mut config = SchemaSheetsConfig::default();
    config.validation.common_types.extend(vec![
        "time".to_string(),
        "uriorcurie".to_string(),
        "ncname".to_string(),
    ]);

    assert!(config.validation.common_types.contains(&"time".to_string()));
    assert!(
        config
            .validation
            .common_types
            .contains(&"uriorcurie".to_string())
    );
    assert!(
        config
            .validation
            .common_types
            .contains(&"ncname".to_string())
    );
}
