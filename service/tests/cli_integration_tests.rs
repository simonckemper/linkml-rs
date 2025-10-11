//! End-to-end CLI integration tests for LinkML SchemaSheets commands
//!
//! These tests execute the actual CLI commands via `std::process::Command`
//! to validate the complete workflow from command-line to output files.

use linkml_core::prelude::*;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Helper to get the path to the linkml binary
fn get_linkml_binary() -> PathBuf {
    // In tests, the binary is in target/debug/linkml
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Remove service
    path.pop(); // Remove linkml
    path.pop(); // Remove symbolic
    path.pop(); // Remove model
    path.pop(); // Remove crates
    path.push("target");
    path.push("debug");
    path.push("linkml");
    path
}

/// Test schema2sheets → sheets2schema round-trip with data
#[test]
#[ignore] // Requires binary to be built first
fn test_schema2sheets_sheets2schema_roundtrip() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary = get_linkml_binary();

    // Create a simple test schema
    let schema = create_test_schema();
    let schema_path = temp_dir.path().join("test_schema.yaml");
    let schema_yaml = serde_yaml::to_string(&schema).expect("Failed to serialize schema");
    fs::write(&schema_path, schema_yaml).expect("Failed to write schema");

    // Step 1: schema2sheets - Generate Excel template
    let excel_path = temp_dir.path().join("template.xlsx");
    let output = Command::new(&binary)
        .args([
            "schema2sheets",
            schema_path.to_str().unwrap(),
            "-o",
            excel_path.to_str().unwrap(),
            "--validation",
            "--freeze-headers",
        ])
        .output()
        .expect("Failed to execute schema2sheets");

    assert!(
        output.status.success(),
        "schema2sheets failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(excel_path.exists(), "Excel file was not created");

    // Step 2: sheets2schema - Generate schema from Excel
    let output_schema_path = temp_dir.path().join("output_schema.yaml");
    let output = Command::new(&binary)
        .args([
            "sheets2schema",
            excel_path.to_str().unwrap(),
            "-o",
            output_schema_path.to_str().unwrap(),
            "--schema-id",
            "test_schema",
        ])
        .output()
        .expect("Failed to execute sheets2schema");

    assert!(
        output.status.success(),
        "sheets2schema failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_schema_path.exists(), "Output schema was not created");

    // Verify the output schema can be parsed
    let output_content =
        fs::read_to_string(&output_schema_path).expect("Failed to read output schema");
    let output_schema: SchemaDefinition =
        serde_yaml::from_str(&output_content).expect("Failed to parse output schema");

    // Basic validation - schema should have the same classes
    assert_eq!(output_schema.id, "test_schema");
    assert!(!output_schema.classes.is_empty());
}

/// Test sheets2schema with invalid Excel file
#[test]
#[ignore] // Requires binary to be built first
fn test_sheets2schema_invalid_file() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary = get_linkml_binary();

    // Create an invalid file (not Excel)
    let invalid_file = temp_dir.path().join("invalid.xlsx");
    fs::write(&invalid_file, "This is not an Excel file").expect("Failed to write invalid file");

    let output = Command::new(&binary)
        .args(["sheets2schema", invalid_file.to_str().unwrap()])
        .output()
        .expect("Failed to execute sheets2schema");

    assert!(
        !output.status.success(),
        "sheets2schema should fail with invalid file"
    );
}

/// Test sheets2schema with missing file
#[test]
#[ignore] // Requires binary to be built first
fn test_sheets2schema_missing_file() {
    let binary = get_linkml_binary();

    let output = Command::new(&binary)
        .args(["sheets2schema", "/nonexistent/file.xlsx"])
        .output()
        .expect("Failed to execute sheets2schema");

    assert!(
        !output.status.success(),
        "sheets2schema should fail with missing file"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not found") || stderr.contains("No such file"),
        "Error message should mention file not found"
    );
}

/// Test schema2sheets with invalid schema
#[test]
#[ignore] // Requires binary to be built first
fn test_schema2sheets_invalid_schema() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary = get_linkml_binary();

    // Create an invalid schema file
    let invalid_schema = temp_dir.path().join("invalid.yaml");
    fs::write(&invalid_schema, "This is not a valid schema")
        .expect("Failed to write invalid schema");

    let excel_path = temp_dir.path().join("output.xlsx");
    let output = Command::new(&binary)
        .args([
            "schema2sheets",
            invalid_schema.to_str().unwrap(),
            "-o",
            excel_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute schema2sheets");

    assert!(
        !output.status.success(),
        "schema2sheets should fail with invalid schema"
    );
}

/// Test schema2sheets help message
#[test]
#[ignore] // Requires binary to be built first
fn test_schema2sheets_help() {
    let binary = get_linkml_binary();

    let output = Command::new(&binary)
        .args(["schema2sheets", "--help"])
        .output()
        .expect("Failed to execute schema2sheets --help");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("schema2sheets"),
        "Help should mention command name"
    );
    assert!(
        stdout.contains("schema"),
        "Help should mention schema option"
    );
    assert!(
        stdout.contains("output"),
        "Help should mention output option"
    );
}

/// Test sheets2schema help message
#[test]
#[ignore] // Requires binary to be built first
fn test_sheets2schema_help() {
    let binary = get_linkml_binary();

    let output = Command::new(&binary)
        .args(["sheets2schema", "--help"])
        .output()
        .expect("Failed to execute sheets2schema --help");

    assert!(output.status.success(), "Help command should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("sheets2schema"),
        "Help should mention command name"
    );
    assert!(stdout.contains("Excel"), "Help should mention Excel");
    assert!(stdout.contains("schema"), "Help should mention schema");
}

/// Test schema2sheets with all options
#[test]
#[ignore] // Requires binary to be built first
fn test_schema2sheets_all_options() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let binary = get_linkml_binary();

    let schema = create_test_schema();
    let schema_path = temp_dir.path().join("schema.yaml");
    let schema_yaml = serde_yaml::to_string(&schema).expect("Failed to serialize schema");
    fs::write(&schema_path, schema_yaml).expect("Failed to write schema");

    let excel_path = temp_dir.path().join("full_template.xlsx");
    let output = Command::new(&binary)
        .args([
            "schema2sheets",
            schema_path.to_str().unwrap(),
            "-o",
            excel_path.to_str().unwrap(),
            "--validation",
            "--examples",
            "--freeze-headers",
            "--filters",
        ])
        .output()
        .expect("Failed to execute schema2sheets");

    assert!(
        output.status.success(),
        "schema2sheets with all options failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(excel_path.exists(), "Excel file was not created");
}

/// Helper function to create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.id = "test_schema".to_string();
    schema.name = "Test Schema".to_string();
    schema.description = Some("A test schema for CLI integration tests".to_string());

    let mut person_class = ClassDefinition::new("Person");
    person_class.name = "Person".to_string();
    person_class.description = Some("A person entity".to_string());

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
