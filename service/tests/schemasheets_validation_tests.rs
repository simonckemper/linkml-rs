//! Tests for SchemaSheets data validation features

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
};
use linkml_service::schemasheets::SchemaSheetsGenerator;
use tempfile::TempDir;

/// Test that data validation is enabled by default
#[tokio::test]
async fn test_data_validation_enabled_by_default() {
    let generator = SchemaSheetsGenerator::new();
    assert!(
        generator.add_data_validation,
        "Data validation should be enabled by default"
    );
}

/// Test generator with data validation enabled
#[tokio::test]
async fn test_generate_with_data_validation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add a class with various field types
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person".to_string()),
        ..Default::default()
    };

    // Add an identifier field (boolean validation)
    let id_slot = SlotDefinition {
        name: "id".to_string(),
        identifier: Some(true),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };

    // Add a regular field
    let name_slot = SlotDefinition {
        name: "name".to_string(),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };

    // Add a field with enum range
    let status_slot = SlotDefinition {
        name: "status".to_string(),
        required: Some(false),
        range: Some("Status".to_string()),
        ..Default::default()
    };

    person_class.attributes.insert("id".to_string(), id_slot);
    person_class
        .attributes
        .insert("name".to_string(), name_slot);
    person_class
        .attributes
        .insert("status".to_string(), status_slot);

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

    // Generate with data validation
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("validated_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    assert!(generator.add_data_validation);

    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test generator with data validation disabled
#[tokio::test]
async fn test_generate_without_data_validation() {
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

    person_class
        .attributes
        .insert("name".to_string(), name_slot);
    schema.classes.insert("Person".to_string(), person_class);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("unvalidated_schema.xlsx");

    let mut generator = SchemaSheetsGenerator::new();
    generator.add_data_validation = false;

    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test validation with multiple enums
#[tokio::test]
async fn test_validation_with_multiple_enums() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add multiple enums
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ],
        ..Default::default()
    };

    let priority_enum = EnumDefinition {
        name: "Priority".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("HIGH".to_string()),
            PermissibleValue::Simple("MEDIUM".to_string()),
            PermissibleValue::Simple("LOW".to_string()),
        ],
        ..Default::default()
    };

    schema.enums.insert("Status".to_string(), status_enum);
    schema.enums.insert("Priority".to_string(), priority_enum);

    // Add a class using these enums
    let mut task_class = ClassDefinition {
        name: "Task".to_string(),
        ..Default::default()
    };

    let status_slot = SlotDefinition {
        name: "status".to_string(),
        range: Some("Status".to_string()),
        ..Default::default()
    };

    let priority_slot = SlotDefinition {
        name: "priority".to_string(),
        range: Some("Priority".to_string()),
        ..Default::default()
    };

    task_class
        .attributes
        .insert("status".to_string(), status_slot);
    task_class
        .attributes
        .insert("priority".to_string(), priority_slot);

    schema.classes.insert("Task".to_string(), task_class);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("multi_enum_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test validation with complex permissible values
#[tokio::test]
async fn test_validation_with_complex_permissible_values() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add enum with complex permissible values
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        permissible_values: vec![
            PermissibleValue::Complex {
                text: "ACTIVE".to_string(),
                description: Some("Active status".to_string()),
                meaning: None,
            },
            PermissibleValue::Complex {
                text: "INACTIVE".to_string(),
                description: Some("Inactive status".to_string()),
                meaning: None,
            },
        ],
        ..Default::default()
    };

    schema.enums.insert("Status".to_string(), status_enum);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("complex_enum_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}

/// Test that validation includes common data types
#[tokio::test]
async fn test_validation_includes_common_types() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add a class with various data types
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        ..Default::default()
    };

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };

    let age_slot = SlotDefinition {
        name: "age".to_string(),
        range: Some("integer".to_string()),
        ..Default::default()
    };

    let active_slot = SlotDefinition {
        name: "active".to_string(),
        range: Some("boolean".to_string()),
        ..Default::default()
    };

    person_class
        .attributes
        .insert("name".to_string(), name_slot);
    person_class.attributes.insert("age".to_string(), age_slot);
    person_class
        .attributes
        .insert("active".to_string(), active_slot);

    schema.classes.insert("Person".to_string(), person_class);

    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("typed_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator
        .generate_file(&schema, &output_path)
        .await
        .unwrap();

    // Verify file was created
    assert!(output_path.exists());
}
