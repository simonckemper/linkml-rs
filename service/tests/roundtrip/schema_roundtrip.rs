//! Schema round-trip tests
//!
//! Tests: Schema → Excel → Schema conversion with semantic equivalence validation

use super::equivalence::{compare_schemas, EquivalenceResult};
use linkml_core::prelude::*;
use linkml_service::generator::excel::ExcelGenerator;
use linkml_service::introspector::excel::ExcelIntrospector;
use linkml_service::introspector::Introspector;
use logger_service::wiring::wire_logger;
use std::path::PathBuf;
use tempfile::TempDir;
use timestamp_service::wiring::wire_timestamp;

/// Helper to create test services
fn create_test_services() -> (
    std::sync::Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    std::sync::Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = wire_timestamp().into_arc();
    let logger = wire_logger(timestamp.clone()).into_arc();
    (logger, timestamp)
}

/// Helper to create temporary directory for test files
fn setup_test_directory() -> Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    Ok(temp_dir)
}

/// Test simple schema round-trip: Schema → Excel → Schema
#[tokio::test]
async fn test_simple_schema_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create original schema
    let original_schema = create_simple_schema();

    // Step 1: Schema → Excel (using generator)
    let generator = ExcelGenerator::new();
    let excel_path = temp_dir.path().join("simple_schema.xlsx");
    generator.generate_file(&original_schema, excel_path.to_str().unwrap())?;

    // Step 2: Excel → Schema (using introspector)
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let reconstructed_schema = introspector.introspect_file(&excel_path).await?;

    // Step 3: Compare schemas for semantic equivalence
    let result = compare_schemas(&original_schema, &reconstructed_schema);

    if !result.is_equivalent {
        eprintln!("Schema round-trip failed:");
        eprintln!("{}", result.report());
    }

    assert!(
        result.is_equivalent,
        "Simple schema round-trip should preserve all semantic information"
    );

    Ok(())
}

/// Test complex schema with inheritance hierarchies
#[tokio::test]
async fn test_complex_schema_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create complex schema with inheritance
    let original_schema = create_complex_schema();

    // Schema → Excel
    let generator = ExcelGenerator::new();
    let excel_path = temp_dir.path().join("complex_schema.xlsx");
    generator.generate_file(&original_schema, excel_path.to_str().unwrap())?;

    // Excel → Schema
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let reconstructed_schema = introspector.introspect_file(&excel_path).await?;

    // Compare
    let result = compare_schemas(&original_schema, &reconstructed_schema);

    if !result.is_equivalent {
        eprintln!("Complex schema round-trip failed:");
        eprintln!("{}", result.report());
    }

    assert!(
        result.is_equivalent,
        "Complex schema with inheritance should preserve all semantic information"
    );

    Ok(())
}

/// Test schema with all LinkML constraint types
#[tokio::test]
async fn test_schema_with_constraints_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with various constraints
    let original_schema = create_schema_with_constraints();

    // Schema → Excel
    let generator = ExcelGenerator::new();
    let excel_path = temp_dir.path().join("constraints_schema.xlsx");
    generator.generate_file(&original_schema, excel_path.to_str().unwrap())?;

    // Excel → Schema
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let reconstructed_schema = introspector.introspect_file(&excel_path).await?;

    // Compare
    let result = compare_schemas(&original_schema, &reconstructed_schema);

    if !result.is_equivalent {
        eprintln!("Schema with constraints round-trip failed:");
        eprintln!("{}", result.report());
    }

    assert!(
        result.is_equivalent,
        "Schema with constraints should preserve all semantic information"
    );

    Ok(())
}

/// Test schema with enums
#[tokio::test]
async fn test_schema_with_enums_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with enums
    let original_schema = create_schema_with_enums();

    // Schema → Excel
    let generator = ExcelGenerator::new();
    let excel_path = temp_dir.path().join("enums_schema.xlsx");
    generator.generate_file(&original_schema, excel_path.to_str().unwrap())?;

    // Excel → Schema
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let reconstructed_schema = introspector.introspect_file(&excel_path).await?;

    // Compare
    let result = compare_schemas(&original_schema, &reconstructed_schema);

    if !result.is_equivalent {
        eprintln!("Schema with enums round-trip failed:");
        eprintln!("{}", result.report());
    }

    assert!(
        result.is_equivalent,
        "Schema with enums should preserve all semantic information"
    );

    Ok(())
}

/// Test multi-class schema with relationships
#[tokio::test]
async fn test_multi_class_schema_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with multiple related classes
    let original_schema = create_multi_class_schema();

    // Schema → Excel
    let generator = ExcelGenerator::new();
    let excel_path = temp_dir.path().join("multi_class_schema.xlsx");
    generator.generate_file(&original_schema, excel_path.to_str().unwrap())?;

    // Excel → Schema
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let reconstructed_schema = introspector.introspect_file(&excel_path).await?;

    // Compare
    let result = compare_schemas(&original_schema, &reconstructed_schema);

    if !result.is_equivalent {
        eprintln!("Multi-class schema round-trip failed:");
        eprintln!("{}", result.report());
    }

    assert!(
        result.is_equivalent,
        "Multi-class schema should preserve all semantic information"
    );

    Ok(())
}

// =============================================================================
// Test Schema Creation Helpers
// =============================================================================

/// Create a simple schema with one class
fn create_simple_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("simple_schema");
    schema.id = "simple_schema".to_string();
    schema.name = "simple_schema".to_string();
    schema.description = Some("A simple test schema".to_string());

    let mut person_class = ClassDefinition::new("Person");
    person_class.name = "Person".to_string();
    person_class.description = Some("A person entity".to_string());

    // Add basic attributes
    person_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );

    person_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    person_class.attributes.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );

    schema.classes.insert("Person".to_string(), person_class);
    schema
}

/// Create a complex schema with inheritance
fn create_complex_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("complex_schema");
    schema.id = "complex_schema".to_string();
    schema.name = "complex_schema".to_string();
    schema.description = Some("A complex schema with inheritance".to_string());

    // Base class
    let mut entity_class = ClassDefinition::new("Entity");
    entity_class.name = "Entity".to_string();
    entity_class.description = Some("Base entity class".to_string());
    entity_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );

    // Derived class
    let mut person_class = ClassDefinition::new("Person");
    person_class.name = "Person".to_string();
    person_class.description = Some("Person inherits from Entity".to_string());
    person_class.is_a = Some("Entity".to_string());
    person_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    // Another derived class
    let mut organization_class = ClassDefinition::new("Organization");
    organization_class.name = "Organization".to_string();
    organization_class.description = Some("Organization inherits from Entity".to_string());
    organization_class.is_a = Some("Entity".to_string());
    organization_class.attributes.insert(
        "org_name".to_string(),
        SlotDefinition {
            name: "org_name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.classes.insert("Entity".to_string(), entity_class);
    schema.classes.insert("Person".to_string(), person_class);
    schema.classes.insert("Organization".to_string(), organization_class);

    schema
}

/// Create a schema with various constraints
fn create_schema_with_constraints() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("constraints_schema");
    schema.id = "constraints_schema".to_string();
    schema.name = "constraints_schema".to_string();

    let mut product_class = ClassDefinition::new("Product");
    product_class.name = "Product".to_string();

    // Pattern constraint
    product_class.attributes.insert(
        "sku".to_string(),
        SlotDefinition {
            name: "sku".to_string(),
            range: Some("string".to_string()),
            pattern: Some("^[A-Z]{3}-\\d{4}$".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );

    // Range constraints
    product_class.attributes.insert(
        "price".to_string(),
        SlotDefinition {
            name: "price".to_string(),
            range: Some("float".to_string()),
            minimum_value: Some(0.0),
            maximum_value: Some(99999.99),
            required: Some(true),
            ..Default::default()
        },
    );

    // Multivalued
    product_class.attributes.insert(
        "tags".to_string(),
        SlotDefinition {
            name: "tags".to_string(),
            range: Some("string".to_string()),
            multivalued: Some(true),
            required: Some(false),
            ..Default::default()
        },
    );

    schema.classes.insert("Product".to_string(), product_class);
    schema
}

/// Create a schema with enums
fn create_schema_with_enums() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("enums_schema");
    schema.id = "enums_schema".to_string();
    schema.name = "enums_schema".to_string();

    // Define enum
    let mut status_enum = EnumDefinition::new("StatusEnum");
    status_enum.name = "StatusEnum".to_string();
    status_enum.permissible_values.insert(
        "active".to_string(),
        PermissibleValue {
            text: "active".to_string(),
            description: Some("Active status".to_string()),
            ..Default::default()
        },
    );
    status_enum.permissible_values.insert(
        "inactive".to_string(),
        PermissibleValue {
            text: "inactive".to_string(),
            description: Some("Inactive status".to_string()),
            ..Default::default()
        },
    );

    // Class using enum
    let mut user_class = ClassDefinition::new("User");
    user_class.name = "User".to_string();
    user_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    user_class.attributes.insert(
        "status".to_string(),
        SlotDefinition {
            name: "status".to_string(),
            range: Some("StatusEnum".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.enums.insert("StatusEnum".to_string(), status_enum);
    schema.classes.insert("User".to_string(), user_class);

    schema
}

/// Create a multi-class schema with relationships
fn create_multi_class_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("multi_class_schema");
    schema.id = "multi_class_schema".to_string();
    schema.name = "multi_class_schema".to_string();

    // Department class
    let mut department_class = ClassDefinition::new("Department");
    department_class.name = "Department".to_string();
    department_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    department_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    // Employee class with reference to Department
    let mut employee_class = ClassDefinition::new("Employee");
    employee_class.name = "Employee".to_string();
    employee_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    employee_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );
    employee_class.attributes.insert(
        "department_id".to_string(),
        SlotDefinition {
            name: "department_id".to_string(),
            range: Some("Department".to_string()), // Reference to Department
            required: Some(true),
            ..Default::default()
        },
    );

    schema.classes.insert("Department".to_string(), department_class);
    schema.classes.insert("Employee".to_string(), employee_class);

    schema
}
