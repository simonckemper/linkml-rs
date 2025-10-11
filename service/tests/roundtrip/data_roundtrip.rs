//! Data round-trip tests
//!
//! Tests: Data → Excel → Data conversion with value preservation validation
//!
//! Note: This tests data loading preservation. Full data generation (Data → Excel)
//! is not yet implemented in the generator, so these tests focus on schema-driven
//! data loading round-trips.

use linkml_core::prelude::*;
use linkml_service::loader::excel::ExcelLoader;
use linkml_service::loader::{DataLoader, LoadOptions};
use logger_service::wiring::wire_logger;
use rust_xlsxwriter::Workbook;
use serde_json::json;
use tempfile::TempDir;
use timestamp_service::wiring::wire_timestamp;

/// Helper to create test services
fn create_test_services() -> (
    std::sync::Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    std::sync::Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = wire_timestamp().into_arc();
    let logger = wire_logger(timestamp.clone(), logger_core::types::LoggerConfig::default())
        .expect("Failed to wire logger")
        .into_arc();
    (logger, timestamp)
}

/// Helper to create temporary directory for test files
fn setup_test_directory() -> std::result::Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    Ok(temp_dir)
}

/// Test simple data round-trip: Excel → Data → Validation
#[tokio::test]
async fn test_simple_data_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema
    let schema = create_simple_data_schema();

    // Create Excel file with data
    let excel_path = temp_dir.path().join("simple_data.xlsx");
    create_simple_data_excel(&excel_path)?;

    // Load data from Excel
    let loader = ExcelLoader::new(logger.clone(), timestamp.clone());
    let options = LoadOptions {
        target_class: Some("Person".to_string()),
        ..Default::default()
    };

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    // Validate loaded data
    assert_eq!(instances.len(), 3, "Should load 3 person records");

    // Check first person
    let person1 = &instances[0];
    assert_eq!(person1.class_name, "Person");
    assert_eq!(person1.data.get("id").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        person1.data.get("name").and_then(|v| v.as_str()),
        Some("Alice")
    );
    assert_eq!(person1.data.get("age").and_then(|v| v.as_i64()), Some(30));

    // Check second person
    let person2 = &instances[1];
    assert_eq!(person2.data.get("id").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(
        person2.data.get("name").and_then(|v| v.as_str()),
        Some("Bob")
    );
    assert_eq!(person2.data.get("age").and_then(|v| v.as_i64()), Some(25));

    // Check third person
    let person3 = &instances[2];
    assert_eq!(person3.data.get("id").and_then(|v| v.as_i64()), Some(3));
    assert_eq!(
        person3.data.get("name").and_then(|v| v.as_str()),
        Some("Carol")
    );
    assert_eq!(person3.data.get("age").and_then(|v| v.as_i64()), Some(35));

    Ok(())
}

/// Test data round-trip with all data types
#[tokio::test]
async fn test_all_types_data_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with all types
    let schema = create_all_types_schema();

    // Create Excel file with all data types
    let excel_path = temp_dir.path().join("all_types_data.xlsx");
    create_all_types_excel(&excel_path)?;

    // Load data
    let loader = ExcelLoader::new(logger, timestamp);
    let options = LoadOptions {
        target_class: Some("AllTypes".to_string()),
        ..Default::default()
    };

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    assert_eq!(instances.len(), 2, "Should load 2 records");

    // Validate first record
    let record1 = &instances[0];
    assert_eq!(record1.data.get("id").and_then(|v| v.as_i64()), Some(1));
    assert_eq!(
        record1.data.get("text_field").and_then(|v| v.as_str()),
        Some("Hello")
    );
    assert_eq!(
        record1.data.get("int_field").and_then(|v| v.as_i64()),
        Some(42)
    );
    assert_eq!(
        record1.data.get("float_field").and_then(|v| v.as_f64()),
        Some(std::f64::consts::PI)
    );
    assert_eq!(
        record1.data.get("bool_field").and_then(|v| v.as_bool()),
        Some(true)
    );

    Ok(())
}

/// Test data round-trip with constraints validation
#[tokio::test]
async fn test_constraints_data_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with constraints
    let schema = create_constraints_schema();

    // Create Excel file with constrained data
    let excel_path = temp_dir.path().join("constraints_data.xlsx");
    create_constraints_excel(&excel_path)?;

    // Load data
    let loader = ExcelLoader::new(logger, timestamp);
    let options = LoadOptions {
        target_class: Some("Product".to_string()),
        ..Default::default()
    };

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    assert_eq!(instances.len(), 2, "Should load 2 products");

    // Validate constraints are preserved
    let product1 = &instances[0];
    let price1 = product1.data.get("price").and_then(|v| v.as_f64()).unwrap();
    assert!(
        (0.0..=99999.99).contains(&price1),
        "Price should be within range"
    );

    Ok(())
}

/// Test multi-sheet data round-trip
#[tokio::test]
async fn test_multi_sheet_data_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with multiple classes
    let schema = create_multi_class_data_schema();

    // Create Excel file with multiple sheets
    let excel_path = temp_dir.path().join("multi_sheet_data.xlsx");
    create_multi_sheet_excel(&excel_path)?;

    // Load data with wildcard to get all sheets
    use linkml_service::loader::ExcelOptions;
    let mut excel_options = ExcelOptions::default();
    excel_options.target_sheet = Some("*".to_string());

    let loader = ExcelLoader::with_options(logger, timestamp, excel_options);
    let options = LoadOptions::default();

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    // Should load data from both Department and Employee sheets
    let departments: Vec<_> = instances
        .iter()
        .filter(|i| i.class_name == "Department")
        .collect();
    let employees: Vec<_> = instances
        .iter()
        .filter(|i| i.class_name == "Employee")
        .collect();

    assert_eq!(departments.len(), 2, "Should load 2 departments");
    assert_eq!(employees.len(), 3, "Should load 3 employees");

    Ok(())
}

/// Test data round-trip with optional fields
#[tokio::test]
async fn test_optional_fields_data_roundtrip() -> std::result::Result<(), Box<dyn std::error::Error>>
{
    let (logger, timestamp) = create_test_services();
    let temp_dir = setup_test_directory()?;

    // Create schema with optional fields
    let schema = create_optional_fields_schema();

    // Create Excel with some null/empty values
    let excel_path = temp_dir.path().join("optional_fields_data.xlsx");
    create_optional_fields_excel(&excel_path)?;

    // Load data
    let loader = ExcelLoader::new(logger, timestamp);
    let mut options = LoadOptions::default();
    options.target_class = Some("Contact".to_string());

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    assert_eq!(instances.len(), 2, "Should load 2 contacts");

    // First contact has all fields
    let contact1 = &instances[0];
    assert!(contact1.data.contains_key("email"));
    assert!(contact1.data.contains_key("phone"));

    // Second contact has optional field missing
    let contact2 = &instances[1];
    assert!(contact2.data.contains_key("name"));
    // Phone might be null or missing - both are acceptable for optional fields

    Ok(())
}

// =============================================================================
// Test Data Creation Helpers
// =============================================================================

/// Create simple schema for data tests
fn create_simple_data_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("simple_data_schema");
    schema.id = "simple_data_schema".to_string();

    let mut person_class = ClassDefinition::new("Person");
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

/// Create Excel file with simple person data
fn create_simple_data_excel(
    path: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    worksheet.write_string(0, 2, "age")?;

    // Write data rows
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "Alice")?;
    worksheet.write_number(1, 2, 30.0)?;

    worksheet.write_number(2, 0, 2.0)?;
    worksheet.write_string(2, 1, "Bob")?;
    worksheet.write_number(2, 2, 25.0)?;

    worksheet.write_number(3, 0, 3.0)?;
    worksheet.write_string(3, 1, "Carol")?;
    worksheet.write_number(3, 2, 35.0)?;

    workbook.save(path)?;
    Ok(())
}

/// Create schema with all data types
fn create_all_types_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("all_types_schema");
    schema.id = "all_types_schema".to_string();

    let mut class = ClassDefinition::new("AllTypes");
    class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "text_field".to_string(),
        SlotDefinition {
            name: "text_field".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "int_field".to_string(),
        SlotDefinition {
            name: "int_field".to_string(),
            range: Some("integer".to_string()),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "float_field".to_string(),
        SlotDefinition {
            name: "float_field".to_string(),
            range: Some("float".to_string()),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "bool_field".to_string(),
        SlotDefinition {
            name: "bool_field".to_string(),
            range: Some("boolean".to_string()),
            ..Default::default()
        },
    );

    schema.classes.insert("AllTypes".to_string(), class);
    schema
}

/// Create Excel with all data types
fn create_all_types_excel(
    path: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Headers
    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "text_field")?;
    worksheet.write_string(0, 2, "int_field")?;
    worksheet.write_string(0, 3, "float_field")?;
    worksheet.write_string(0, 4, "bool_field")?;

    // Data rows
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "Hello")?;
    worksheet.write_number(1, 2, 42.0)?;
    worksheet.write_number(1, 3, std::f64::consts::PI)?;
    worksheet.write_boolean(1, 4, true)?;

    worksheet.write_number(2, 0, 2.0)?;
    worksheet.write_string(2, 1, "World")?;
    worksheet.write_number(2, 2, 99.0)?;
    worksheet.write_number(2, 3, std::f64::consts::E)?;
    worksheet.write_boolean(2, 4, false)?;

    workbook.save(path)?;
    Ok(())
}

/// Create schema with constraints
fn create_constraints_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("constraints_schema");
    schema.id = "constraints_schema".to_string();

    let mut class = ClassDefinition::new("Product");
    class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "price".to_string(),
        SlotDefinition {
            name: "price".to_string(),
            range: Some("float".to_string()),
            minimum_value: Some(json!(0.0)),
            maximum_value: Some(json!(99999.99)),
            ..Default::default()
        },
    );

    schema.classes.insert("Product".to_string(), class);
    schema
}

/// Create Excel with constraint-compliant data
fn create_constraints_excel(
    path: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "price")?;

    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_number(1, 1, 19.99)?;

    worksheet.write_number(2, 0, 2.0)?;
    worksheet.write_number(2, 1, 49.99)?;

    workbook.save(path)?;
    Ok(())
}

/// Create multi-class schema for multi-sheet tests
fn create_multi_class_data_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("multi_class_schema");
    schema.id = "multi_class_schema".to_string();

    let mut dept_class = ClassDefinition::new("Department");
    dept_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    dept_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    let mut emp_class = ClassDefinition::new("Employee");
    emp_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    emp_class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.classes.insert("Department".to_string(), dept_class);
    schema.classes.insert("Employee".to_string(), emp_class);

    schema
}

/// Create multi-sheet Excel file
fn create_multi_sheet_excel(
    path: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    // Department sheet
    let dept_sheet = workbook.add_worksheet();
    dept_sheet.set_name("Department")?;
    dept_sheet.write_string(0, 0, "id")?;
    dept_sheet.write_string(0, 1, "name")?;
    dept_sheet.write_number(1, 0, 1.0)?;
    dept_sheet.write_string(1, 1, "Engineering")?;
    dept_sheet.write_number(2, 0, 2.0)?;
    dept_sheet.write_string(2, 1, "Sales")?;

    // Employee sheet
    let emp_sheet = workbook.add_worksheet();
    emp_sheet.set_name("Employee")?;
    emp_sheet.write_string(0, 0, "id")?;
    emp_sheet.write_string(0, 1, "name")?;
    emp_sheet.write_number(1, 0, 1.0)?;
    emp_sheet.write_string(1, 1, "Alice")?;
    emp_sheet.write_number(2, 0, 2.0)?;
    emp_sheet.write_string(2, 1, "Bob")?;
    emp_sheet.write_number(3, 0, 3.0)?;
    emp_sheet.write_string(3, 1, "Carol")?;

    workbook.save(path)?;
    Ok(())
}

/// Create schema with optional fields
fn create_optional_fields_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("optional_fields_schema");
    schema.id = "optional_fields_schema".to_string();

    let mut class = ClassDefinition::new("Contact");
    class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            range: Some("string".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );
    class.attributes.insert(
        "phone".to_string(),
        SlotDefinition {
            name: "phone".to_string(),
            range: Some("string".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );

    schema.classes.insert("Contact".to_string(), class);
    schema
}

/// Create Excel with optional fields (some empty)
fn create_optional_fields_excel(
    path: &std::path::Path,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    worksheet.write_string(0, 2, "email")?;
    worksheet.write_string(0, 3, "phone")?;

    // Contact 1: all fields
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "Alice")?;
    worksheet.write_string(1, 2, "alice@example.com")?;
    worksheet.write_string(1, 3, "555-1234")?;

    // Contact 2: missing phone (leave cell empty)
    worksheet.write_number(2, 0, 2.0)?;
    worksheet.write_string(2, 1, "Bob")?;
    worksheet.write_string(2, 2, "bob@example.com")?;
    // Column 3 (phone) left empty

    workbook.save(path)?;
    Ok(())
}
