//! Unit tests for SchemaSheets parser

mod helpers;

use helpers::schemasheets_test_generator::*;
use linkml_service::schemasheets::SchemaSheetsParser;
use tempfile::TempDir;

/// Test parsing enum definitions
#[tokio::test]
async fn test_parse_enum_definition() {
    // Create test file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    // Parse schema
    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify enum was parsed
    assert!(
        schema.enums.contains_key("Status"),
        "Status enum should be present"
    );

    let status_enum = &schema.enums["Status"];
    assert_eq!(status_enum.name, "Status");
    assert_eq!(status_enum.description, Some("Status values".to_string()));

    // Verify enum values
    assert_eq!(
        status_enum.permissible_values.len(),
        2,
        "Should have 2 permissible values"
    );
}

/// Test parsing enum values
#[tokio::test]
async fn test_parse_enum_values() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    let status_enum = &schema.enums["Status"];

    // Check that we have ACTIVE and INACTIVE values
    let values: Vec<String> = status_enum
        .permissible_values
        .iter()
        .map(|pv| match pv {
            linkml_core::types::PermissibleValue::Simple(s) => s.clone(),
            linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
        })
        .collect();

    assert!(
        values.contains(&"ACTIVE".to_string()),
        "Should contain ACTIVE"
    );
    assert!(
        values.contains(&"INACTIVE".to_string()),
        "Should contain INACTIVE"
    );
}

/// Test parsing type definitions
#[tokio::test]
async fn test_parse_type_definition() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify type was parsed
    assert!(
        schema.types.contains_key("EmailType"),
        "EmailType should be present"
    );

    let email_type = &schema.types["EmailType"];
    assert_eq!(email_type.name, "EmailType");
    assert_eq!(
        email_type.description,
        Some("Email address type".to_string())
    );
    assert_eq!(email_type.base_type, Some("string".to_string()));
    assert!(
        email_type.pattern.is_some(),
        "Should have pattern constraint"
    );
}

/// Test parsing type constraints
#[tokio::test]
async fn test_parse_type_constraints() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    let email_type = &schema.types["EmailType"];

    // Verify pattern constraint
    let pattern = email_type.pattern.as_ref().unwrap();
    assert!(pattern.contains("@"), "Email pattern should contain @");
}

/// Test parsing subset definitions
#[tokio::test]
async fn test_parse_subset_definition() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify subset was parsed
    assert!(
        schema.subsets.contains_key("required"),
        "required subset should be present"
    );

    let required_subset = &schema.subsets["required"];
    assert_eq!(required_subset.name, "required");
    assert_eq!(
        required_subset.description,
        Some("Required fields".to_string())
    );
}

/// Test parsing class with inheritance
#[tokio::test]
async fn test_parse_class_with_is_a() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify Employee inherits from Person
    assert!(
        schema.classes.contains_key("Employee"),
        "Employee class should be present"
    );

    let employee = &schema.classes["Employee"];
    assert_eq!(employee.name, "Employee");
    assert_eq!(employee.is_a, Some("Person".to_string()));
    assert_eq!(employee.description, Some("An employee".to_string()));
}

/// Test parsing slot with pattern constraint
#[tokio::test]
async fn test_parse_slot_with_pattern() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    let person = &schema.classes["Person"];

    // Verify email slot has pattern
    assert!(
        person.attributes.contains_key("email"),
        "email attribute should be present"
    );

    let email_slot = &person.attributes["email"];
    assert!(
        email_slot.pattern.is_some(),
        "email should have pattern constraint"
    );

    let pattern = email_slot.pattern.as_ref().unwrap();
    assert!(pattern.contains("@"), "Email pattern should contain @");
}

/// Test parsing exact mappings
#[tokio::test]
async fn test_parse_exact_mappings() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    let person = &schema.classes["Person"];

    // Verify class has exact mapping
    assert!(
        !person.exact_mappings.is_empty(),
        "Person should have exact mappings"
    );
    assert!(
        person.exact_mappings.contains(&"schema:Person".to_string()),
        "Should have schema:Person mapping"
    );

    // Verify slot has exact mapping
    let name_slot = &person.attributes["name"];
    assert!(
        !name_slot.exact_mappings.is_empty(),
        "name should have exact mappings"
    );
    assert!(
        name_slot
            .exact_mappings
            .contains(&"schema:name".to_string()),
        "Should have schema:name mapping"
    );
}

/// Test parsing all mapping types
#[tokio::test]
async fn test_parse_all_mapping_types() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    let person = &schema.classes["Person"];

    // Verify exact mappings
    assert!(
        !person.exact_mappings.is_empty(),
        "Should have exact mappings"
    );

    // Verify close mappings
    assert!(
        !person.close_mappings.is_empty(),
        "Should have close mappings"
    );
    assert!(
        person.close_mappings.contains(&"foaf:Person".to_string()),
        "Should have foaf:Person close mapping"
    );
}

/// Test parsing prefixes sheet
#[tokio::test]
async fn test_parse_prefixes_sheet() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify prefixes were parsed
    assert!(
        schema.prefixes.contains_key("schema"),
        "schema prefix should be present"
    );
    assert!(
        schema.prefixes.contains_key("foaf"),
        "foaf prefix should be present"
    );
    assert!(
        schema.prefixes.contains_key("skos"),
        "skos prefix should be present"
    );

    // Verify prefix URIs
    match &schema.prefixes["schema"] {
        linkml_core::types::PrefixDefinition::Simple(uri) => {
            assert_eq!(uri, "http://schema.org/");
        }
        _ => panic!("Expected simple prefix definition"),
    }
}

/// Test parsing settings sheet
#[tokio::test]
async fn test_parse_settings_sheet() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("person_schema.xlsx");
    create_person_schema_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("person_schema"))
        .unwrap();

    // Verify schema metadata
    assert_eq!(schema.id, "https://example.org/person_schema");
    assert_eq!(schema.name, "person_schema");
    assert_eq!(schema.version, Some("1.0.0".to_string()));
    assert_eq!(
        schema.description,
        Some("A schema for person and employee entities".to_string())
    );
}

/// Test parsing biolink minimal schema
#[tokio::test]
async fn test_parse_biolink_minimal() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("biolink_minimal.xlsx");
    create_biolink_minimal_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser
        .parse_file(&test_file, Some("biolink_minimal"))
        .unwrap();

    // Verify classes
    assert!(
        schema.classes.contains_key("NamedThing"),
        "NamedThing should be present"
    );
    assert!(
        schema.classes.contains_key("BiologicalEntity"),
        "BiologicalEntity should be present"
    );
    assert!(
        schema.classes.contains_key("Gene"),
        "Gene should be present"
    );

    // Verify inheritance
    let bio_entity = &schema.classes["BiologicalEntity"];
    assert_eq!(bio_entity.is_a, Some("NamedThing".to_string()));

    let gene = &schema.classes["Gene"];
    assert_eq!(gene.is_a, Some("BiologicalEntity".to_string()));
}

/// Test parsing API models with subsets
#[tokio::test]
async fn test_parse_api_models_with_subsets() {
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("api_models.xlsx");
    create_api_models_excel(&test_file).unwrap();

    let parser = SchemaSheetsParser::new();
    let schema = parser.parse_file(&test_file, Some("api_models")).unwrap();

    // Verify subsets
    assert!(
        schema.subsets.contains_key("required"),
        "required subset should be present"
    );
    assert!(
        schema.subsets.contains_key("public"),
        "public subset should be present"
    );
    assert!(
        schema.subsets.contains_key("internal"),
        "internal subset should be present"
    );

    // Verify class
    assert!(
        schema.classes.contains_key("ApiRequest"),
        "ApiRequest should be present"
    );
}
