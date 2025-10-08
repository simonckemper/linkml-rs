//! Tests for SchemaSheets advanced formatting features

use linkml_core::types::{ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition};
use linkml_service::schemasheets::SchemaSheetsGenerator;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test that generator creates file with advanced formatting
#[tokio::test]
async fn test_advanced_formatting_enabled() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add a class
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

    person_class.attributes.insert("name".to_string(), name_slot);
    schema.classes.insert("Person".to_string(), person_class);

    // Generate with advanced formatting
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("formatted_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    assert!(generator.freeze_headers);
    assert!(generator.add_filters);
    assert!(generator.alternating_row_colors);
    assert!(generator.auto_size_columns);

    generator.generate_file(&schema, &output_path).await.unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test generator with formatting disabled
#[tokio::test]
async fn test_formatting_disabled() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        ..Default::default()
    };

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        ..Default::default()
    };

    person_class.attributes.insert("name".to_string(), name_slot);
    schema.classes.insert("Person".to_string(), person_class);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unformatted_schema.xlsx");

    let mut generator = SchemaSheetsGenerator::new();
    generator.freeze_headers = false;
    generator.add_filters = false;
    generator.alternating_row_colors = false;
    generator.auto_size_columns = false;

    generator.generate_file(&schema, &output_path).await.unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test formatting with multiple element types
#[tokio::test]
async fn test_formatting_with_all_element_types() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add a class
    let person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person".to_string()),
        ..Default::default()
    };
    schema.classes.insert("Person".to_string(), person_class);

    // Add an enum
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("Status".to_string(), status_enum);

    // Add a type
    let email_type = linkml_core::types::TypeDefinition {
        name: "EmailType".to_string(),
        base_type: Some("string".to_string()),
        ..Default::default()
    };
    schema.types.insert("EmailType".to_string(), email_type);

    // Add a subset
    let required_subset = linkml_core::types::SubsetDefinition {
        name: "required".to_string(),
        ..Default::default()
    };
    schema.subsets.insert("required".to_string(), required_subset);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("all_types_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test that default generator has formatting enabled
#[test]
fn test_default_generator_has_formatting() {
    let generator = SchemaSheetsGenerator::default();
    assert!(generator.freeze_headers);
    assert!(generator.add_filters);
    assert!(generator.alternating_row_colors);
    assert!(generator.auto_size_columns);
}

