//! Unit tests for SchemaSheets generator

use linkml_core::types::{ClassDefinition, EnumDefinition, PermissibleValue, PrefixDefinition, SchemaDefinition};
use linkml_service::schemasheets::{SchemaSheetsGenerator, SchemaSheetsParser};
use std::collections::HashMap;
use tempfile::TempDir;

/// Test generating a basic schema
#[tokio::test]
async fn test_generate_basic_schema() {
    // Create a simple schema
    let mut schema = SchemaDefinition {
        id: "https://example.org/test_schema".to_string(),
        name: "test_schema".to_string(),
        version: Some("1.0.0".to_string()),
        description: Some("A test schema".to_string()),
        ..Default::default()
    };

    // Add a simple class
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person entity".to_string()),
        ..Default::default()
    };

    // Add an attribute
    let mut name_slot = linkml_core::types::SlotDefinition {
        name: "name".to_string(),
        description: Some("Person's name".to_string()),
        range: Some("string".to_string()),
        required: Some(true),
        ..Default::default()
    };
    person_class.attributes.insert("name".to_string(), name_slot);

    schema.classes.insert("Person".to_string(), person_class);

    // Generate Excel file
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("test_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    // Verify file was created
    assert!(output_path.exists(), "Generated Excel file should exist");

    // Parse it back
    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("test_schema")).await.unwrap();

    // Verify basic properties
    assert_eq!(parsed_schema.id, schema.id);
    assert_eq!(parsed_schema.name, schema.name);
    assert!(parsed_schema.classes.contains_key("Person"));
}

/// Test generating schema with enums
#[tokio::test]
async fn test_generate_with_enums() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/enum_schema".to_string(),
        name: "enum_schema".to_string(),
        ..Default::default()
    };

    // Add an enum
    let mut status_enum = EnumDefinition {
        name: "Status".to_string(),
        description: Some("Status values".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ],
        ..Default::default()
    };

    schema.enums.insert("Status".to_string(), status_enum);

    // Generate and parse
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("enum_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("enum_schema")).await.unwrap();

    // Verify enum was preserved
    assert!(parsed_schema.enums.contains_key("Status"));
    let parsed_enum = &parsed_schema.enums["Status"];
    assert_eq!(parsed_enum.permissible_values.len(), 2);
}

/// Test generating schema with types
#[tokio::test]
async fn test_generate_with_types() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/type_schema".to_string(),
        name: "type_schema".to_string(),
        ..Default::default()
    };

    // Add a type
    let email_type = linkml_core::types::TypeDefinition {
        name: "EmailType".to_string(),
        description: Some("Email address type".to_string()),
        base_type: Some("string".to_string()),
        pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
        ..Default::default()
    };

    schema.types.insert("EmailType".to_string(), email_type);

    // Generate and parse
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("type_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("type_schema")).await.unwrap();

    // Verify type was preserved
    assert!(parsed_schema.types.contains_key("EmailType"));
    let parsed_type = &parsed_schema.types["EmailType"];
    assert_eq!(parsed_type.base_type, Some("string".to_string()));
    assert!(parsed_type.pattern.is_some());
}

/// Test generating metadata sheets
#[tokio::test]
async fn test_generate_metadata_sheets() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/metadata_schema".to_string(),
        name: "metadata_schema".to_string(),
        version: Some("2.0.0".to_string()),
        description: Some("Schema with metadata".to_string()),
        ..Default::default()
    };

    // Add prefixes
    schema.prefixes.insert(
        "schema".to_string(),
        PrefixDefinition::Simple("http://schema.org/".to_string()),
    );
    schema.prefixes.insert(
        "foaf".to_string(),
        PrefixDefinition::Simple("http://xmlns.com/foaf/0.1/".to_string()),
    );

    // Generate and parse
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("metadata_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("metadata_schema")).await.unwrap();

    // Verify metadata was preserved
    assert_eq!(parsed_schema.version, Some("2.0.0".to_string()));
    assert_eq!(parsed_schema.description, Some("Schema with metadata".to_string()));
    assert!(parsed_schema.prefixes.contains_key("schema"));
    assert!(parsed_schema.prefixes.contains_key("foaf"));
}

/// Test generating schema with mappings
#[tokio::test]
async fn test_generate_with_mappings() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/mapping_schema".to_string(),
        name: "mapping_schema".to_string(),
        ..Default::default()
    };

    // Add a class with mappings
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person".to_string()),
        exact_mappings: vec!["schema:Person".to_string()],
        close_mappings: vec!["foaf:Person".to_string()],
        ..Default::default()
    };

    // Add an attribute with mappings
    let mut name_slot = linkml_core::types::SlotDefinition {
        name: "name".to_string(),
        description: Some("Name".to_string()),
        exact_mappings: vec!["schema:name".to_string()],
        ..Default::default()
    };

    person_class.attributes.insert("name".to_string(), name_slot);
    schema.classes.insert("Person".to_string(), person_class);

    // Generate and parse
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("mapping_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("mapping_schema")).await.unwrap();

    // Verify mappings were preserved
    let parsed_person = &parsed_schema.classes["Person"];
    assert!(!parsed_person.exact_mappings.is_empty());
    assert!(parsed_person.exact_mappings.contains(&"schema:Person".to_string()));
    assert!(!parsed_person.close_mappings.is_empty());
    assert!(parsed_person.close_mappings.contains(&"foaf:Person".to_string()));

    let parsed_name = &parsed_person.attributes["name"];
    assert!(!parsed_name.exact_mappings.is_empty());
    assert!(parsed_name.exact_mappings.contains(&"schema:name".to_string()));
}

/// Test roundtrip conversion
#[tokio::test]
async fn test_roundtrip_conversion() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/roundtrip_schema".to_string(),
        name: "roundtrip_schema".to_string(),
        version: Some("1.0.0".to_string()),
        description: Some("Roundtrip test schema".to_string()),
        ..Default::default()
    };

    // Add a class with attributes
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person".to_string()),
        ..Default::default()
    };

    let id_slot = linkml_core::types::SlotDefinition {
        name: "id".to_string(),
        identifier: Some(true),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };

    let name_slot = linkml_core::types::SlotDefinition {
        name: "name".to_string(),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };

    person_class.attributes.insert("id".to_string(), id_slot);
    person_class.attributes.insert("name".to_string(), name_slot);
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

    // Generate Excel
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("roundtrip_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &output_path).await.unwrap();

    // Parse it back
    let parser = SchemaSheetsParser::new();
    let parsed_schema = parser.parse_file(&output_path, Some("roundtrip_schema")).await.unwrap();

    // Verify all elements are preserved
    assert_eq!(parsed_schema.id, schema.id);
    assert_eq!(parsed_schema.name, schema.name);
    assert_eq!(parsed_schema.version, schema.version);
    assert_eq!(parsed_schema.classes.len(), schema.classes.len());
    assert_eq!(parsed_schema.enums.len(), schema.enums.len());

    // Verify class details
    let parsed_person = &parsed_schema.classes["Person"];
    assert_eq!(parsed_person.attributes.len(), 2);
    assert!(parsed_person.attributes.contains_key("id"));
    assert!(parsed_person.attributes.contains_key("name"));

    // Verify attribute details
    let parsed_id = &parsed_person.attributes["id"];
    assert_eq!(parsed_id.identifier, Some(true));
    assert_eq!(parsed_id.required, Some(true));
}

