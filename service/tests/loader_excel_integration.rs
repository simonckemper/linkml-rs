//! Integration tests for Excel data loader
//!
//! Tests the complete workflow of loading data from Excel files
//! with schema validation and type conversion.

use linkml_core::prelude::*;
use linkml_service::loader::{DataLoader, ExcelLoader, LoadOptions};
use logger_service::wiring::wire_logger;
use rust_xlsxwriter::{Format, Workbook};
use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;
use timestamp_service::wiring::wire_timestamp;

/// Helper function to create test services
fn create_test_services() -> (
    Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = wire_timestamp().into_arc();
    let logger = wire_logger(timestamp.clone()).into_arc();
    (logger, timestamp)
}

/// Helper function to setup test directory
async fn setup_test_directory() -> Result<TempDir, Box<dyn std::error::Error>> {
    Ok(tempfile::tempdir()?)
}

/// Helper function to create a basic test schema
fn create_employee_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("employee_schema");
    schema.id = "employee_schema".to_string();
    schema.name = Some("Employee Schema".to_string());

    // Create Employee class
    let mut employee_class = ClassDefinition::new("Employee");
    employee_class.attributes.insert(
        "id".to_string(),
        "employee_id".to_string(),
    );
    employee_class.attributes.insert(
        "name".to_string(),
        "employee_name".to_string(),
    );
    employee_class.attributes.insert(
        "age".to_string(),
        "employee_age".to_string(),
    );
    employee_class.attributes.insert(
        "email".to_string(),
        "employee_email".to_string(),
    );
    employee_class.attributes.insert(
        "salary".to_string(),
        "employee_salary".to_string(),
    );
    employee_class.attributes.insert(
        "active".to_string(),
        "employee_active".to_string(),
    );

    // Create slots
    let mut id_slot = SlotDefinition::new("employee_id");
    id_slot.range = Some("integer".to_string());
    id_slot.required = Some(true);
    id_slot.identifier = Some(true);

    let mut name_slot = SlotDefinition::new("employee_name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);

    let mut age_slot = SlotDefinition::new("employee_age");
    age_slot.range = Some("integer".to_string());
    age_slot.required = Some(false);

    let mut email_slot = SlotDefinition::new("employee_email");
    email_slot.range = Some("string".to_string());
    email_slot.required = Some(false);

    let mut salary_slot = SlotDefinition::new("employee_salary");
    salary_slot.range = Some("float".to_string());
    salary_slot.required = Some(false);

    let mut active_slot = SlotDefinition::new("employee_active");
    active_slot.range = Some("boolean".to_string());
    active_slot.required = Some(false);

    schema.classes.insert("Employee".to_string(), employee_class);
    schema.slots.insert("employee_id".to_string(), id_slot);
    schema.slots.insert("employee_name".to_string(), name_slot);
    schema.slots.insert("employee_age".to_string(), age_slot);
    schema.slots.insert("employee_email".to_string(), email_slot);
    schema.slots.insert("employee_salary".to_string(), salary_slot);
    schema.slots.insert("employee_active".to_string(), active_slot);

    schema
}

#[tokio::test]
async fn test_excel_loader_single_sheet() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("employees.xlsx");

    // Create test Excel file
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    worksheet.write_string(0, 2, "age")?;
    worksheet.write_string(0, 3, "email")?;
    worksheet.write_string(0, 4, "salary")?;
    worksheet.write_string(0, 5, "active")?;

    // Write data rows
    let employees = vec![
        (1, "John Doe", 30, "john@example.com", 75000.50, true),
        (2, "Jane Smith", 28, "jane@example.com", 82000.00, true),
        (3, "Bob Johnson", 35, "bob@example.com", 68000.75, false),
    ];

    for (row_idx, (id, name, age, email, salary, active)) in employees.iter().enumerate() {
        let row = (row_idx + 1) as u32;
        worksheet.write_number(row, 0, *id as f64)?;
        worksheet.write_string(row, 1, name)?;
        worksheet.write_number(row, 2, *age as f64)?;
        worksheet.write_string(row, 3, email)?;
        worksheet.write_number(row, 4, *salary)?;
        worksheet.write_boolean(row, 5, *active)?;
    }

    workbook.save(&excel_path)?;

    // Load data with schema
    let schema = create_employee_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Employee".to_string());

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    // Verify results
    assert_eq!(instances.len(), 3, "Should load 3 employees");

    // Check first employee
    let john = &instances[0];
    assert_eq!(john.class_name, "Employee");
    assert!(john.data.contains_key("id"));
    assert!(john.data.contains_key("name"));
    assert!(john.data.contains_key("age"));
    assert!(john.data.contains_key("email"));
    assert!(john.data.contains_key("salary"));
    assert!(john.data.contains_key("active"));

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_type_conversion() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("types.xlsx");

    // Create test Excel file with various data types
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "count")?;
    worksheet.write_string(0, 2, "price")?;
    worksheet.write_string(0, 3, "description")?;
    worksheet.write_string(0, 4, "available")?;

    // Write data with different types
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_number(1, 1, 42.0)?;
    worksheet.write_number(1, 2, 19.99)?;
    worksheet.write_string(1, 3, "Test Product")?;
    worksheet.write_boolean(1, 4, true)?;

    workbook.save(&excel_path)?;

    // Create schema for type testing
    let mut schema = SchemaDefinition::new("type_test_schema");
    let mut product_class = ClassDefinition::new("Product");
    product_class.attributes.insert("id".to_string(), "prod_id".to_string());
    product_class.attributes.insert("count".to_string(), "prod_count".to_string());
    product_class.attributes.insert("price".to_string(), "prod_price".to_string());
    product_class.attributes.insert("description".to_string(), "prod_desc".to_string());
    product_class.attributes.insert("available".to_string(), "prod_avail".to_string());

    let mut id_slot = SlotDefinition::new("prod_id");
    id_slot.range = Some("integer".to_string());
    id_slot.identifier = Some(true);

    let mut count_slot = SlotDefinition::new("prod_count");
    count_slot.range = Some("integer".to_string());

    let mut price_slot = SlotDefinition::new("prod_price");
    price_slot.range = Some("float".to_string());

    let mut desc_slot = SlotDefinition::new("prod_desc");
    desc_slot.range = Some("string".to_string());

    let mut avail_slot = SlotDefinition::new("prod_avail");
    avail_slot.range = Some("boolean".to_string());

    schema.classes.insert("Product".to_string(), product_class);
    schema.slots.insert("prod_id".to_string(), id_slot);
    schema.slots.insert("prod_count".to_string(), count_slot);
    schema.slots.insert("prod_price".to_string(), price_slot);
    schema.slots.insert("prod_desc".to_string(), desc_slot);
    schema.slots.insert("prod_avail".to_string(), avail_slot);

    // Load data
    let mut options = LoadOptions::default();
    options.target_class = Some("Product".to_string());

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    assert_eq!(instances.len(), 1, "Should load 1 product");

    let product = &instances[0];
    assert!(product.data.get("id").is_some());
    assert!(product.data.get("count").is_some());
    assert!(product.data.get("price").is_some());
    assert!(product.data.get("description").is_some());
    assert!(product.data.get("available").is_some());

    // Verify types
    assert!(product.data.get("count").unwrap().is_number());
    assert!(product.data.get("price").unwrap().is_number());
    assert!(product.data.get("description").unwrap().is_string());
    assert!(product.data.get("available").unwrap().is_boolean());

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_multi_sheet() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();

    // Use ExcelOptions to load all sheets
    use linkml_service::loader::ExcelOptions;
    let mut excel_options = ExcelOptions::default();
    excel_options.target_sheet = Some("*".to_string());

    let loader = ExcelLoader::with_options(logger, timestamp, excel_options);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("multi_sheet.xlsx");

    // Create workbook with multiple sheets
    let mut workbook = Workbook::new();

    // Sheet 1: Employees
    let sheet1 = workbook.add_worksheet();
    sheet1.set_name("Employee")?;
    sheet1.write_string(0, 0, "id")?;
    sheet1.write_string(0, 1, "name")?;
    sheet1.write_number(1, 0, 1.0)?;
    sheet1.write_string(1, 1, "John Doe")?;

    // Sheet 2: Departments
    let sheet2 = workbook.add_worksheet();
    sheet2.set_name("Department")?;
    sheet2.write_string(0, 0, "id")?;
    sheet2.write_string(0, 1, "dept_name")?;
    sheet2.write_number(1, 0, 100.0)?;
    sheet2.write_string(1, 1, "Engineering")?;

    workbook.save(&excel_path)?;

    // Create schema with two classes
    let mut schema = SchemaDefinition::new("multi_sheet_schema");

    let mut emp_class = ClassDefinition::new("Employee");
    emp_class.attributes.insert("id".to_string(), "emp_id".to_string());
    emp_class.attributes.insert("name".to_string(), "emp_name".to_string());

    let mut dept_class = ClassDefinition::new("Department");
    dept_class.attributes.insert("id".to_string(), "dept_id".to_string());
    dept_class.attributes.insert("dept_name".to_string(), "dept_name_slot".to_string());

    let mut emp_id_slot = SlotDefinition::new("emp_id");
    emp_id_slot.range = Some("integer".to_string());
    emp_id_slot.identifier = Some(true);

    let mut emp_name_slot = SlotDefinition::new("emp_name");
    emp_name_slot.range = Some("string".to_string());

    let mut dept_id_slot = SlotDefinition::new("dept_id");
    dept_id_slot.range = Some("integer".to_string());
    dept_id_slot.identifier = Some(true);

    let mut dept_name_slot = SlotDefinition::new("dept_name_slot");
    dept_name_slot.range = Some("string".to_string());

    schema.classes.insert("Employee".to_string(), emp_class);
    schema.classes.insert("Department".to_string(), dept_class);
    schema.slots.insert("emp_id".to_string(), emp_id_slot);
    schema.slots.insert("emp_name".to_string(), emp_name_slot);
    schema.slots.insert("dept_id".to_string(), dept_id_slot);
    schema.slots.insert("dept_name_slot".to_string(), dept_name_slot);

    // Load all sheets
    let options = LoadOptions::default();
    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    // Should load instances from both sheets
    assert!(instances.len() >= 2, "Should load instances from multiple sheets");

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_validation_required_fields() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("validation.xlsx");

    // Create Excel file missing required fields
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    // Missing required 'name' field in data
    worksheet.write_number(1, 0, 1.0)?;
    // Intentionally leave name empty

    workbook.save(&excel_path)?;

    // Create schema with required fields
    let schema = create_employee_schema();

    let mut options = LoadOptions::default();
    options.target_class = Some("Employee".to_string());
    options.validate = true; // Enable validation

    // Loading should fail due to missing required field
    let result = loader.load_file(&excel_path, &schema, &options).await;
    assert!(result.is_err(), "Should fail validation for missing required field");

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_skip_invalid() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("skip_invalid.xlsx");

    // Create Excel file with mix of valid and invalid rows
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;

    // Valid row
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "Valid User")?;

    // Invalid row (missing required name)
    worksheet.write_number(2, 0, 2.0)?;
    // Missing name

    // Valid row
    worksheet.write_number(3, 0, 3.0)?;
    worksheet.write_string(3, 1, "Another Valid User")?;

    workbook.save(&excel_path)?;

    let schema = create_employee_schema();

    let mut options = LoadOptions::default();
    options.target_class = Some("Employee".to_string());
    options.validate = true;
    options.skip_invalid = true; // Skip invalid rows

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    // Should load only valid rows
    assert_eq!(instances.len(), 2, "Should load 2 valid rows and skip 1 invalid");

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_load_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("bytes_test.xlsx");

    // Create test Excel file
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "Test User")?;

    workbook.save(&excel_path)?;

    // Read file as bytes
    let bytes = std::fs::read(&excel_path)?;

    let schema = create_employee_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Employee".to_string());

    // Load from bytes
    let instances = loader.load_bytes(&bytes, &schema, &options).await?;

    assert_eq!(instances.len(), 1, "Should load 1 instance from bytes");
    assert_eq!(instances[0].class_name, "Employee");

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_empty_cells() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    let temp_dir = setup_test_directory().await?;
    let excel_path = temp_dir.path().join("empty_cells.xlsx");

    // Create Excel with empty cells
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "name")?;
    worksheet.write_string(0, 2, "age")?;

    // Row with some empty cells
    worksheet.write_number(1, 0, 1.0)?;
    worksheet.write_string(1, 1, "User With No Age")?;
    // age column left empty

    workbook.save(&excel_path)?;

    let schema = create_employee_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Employee".to_string());
    options.validate = false; // Don't validate to allow optional fields

    let instances = loader.load_file(&excel_path, &schema, &options).await?;

    assert_eq!(instances.len(), 1);
    assert!(instances[0].data.contains_key("id"));
    assert!(instances[0].data.contains_key("name"));
    assert!(!instances[0].data.contains_key("age"), "Empty age should not be in data");

    Ok(())
}

#[tokio::test]
async fn test_excel_loader_schema_validation() -> Result<(), Box<dyn std::error::Error>> {
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);

    // Test valid schema
    let valid_schema = create_employee_schema();
    assert!(loader.validate_schema(&valid_schema).is_ok());

    // Test empty schema
    let empty_schema = SchemaDefinition::new("empty");
    assert!(loader.validate_schema(&empty_schema).is_err(),
            "Empty schema should fail validation");

    // Test schema with missing slot reference
    let mut invalid_schema = SchemaDefinition::new("invalid");
    let mut invalid_class = ClassDefinition::new("Invalid");
    invalid_class.attributes.insert(
        "field".to_string(),
        "nonexistent_slot".to_string(),
    );
    invalid_schema.classes.insert("Invalid".to_string(), invalid_class);

    assert!(loader.validate_schema(&invalid_schema).is_err(),
            "Schema with missing slot should fail validation");

    Ok(())
}
