//! Roundtrip tests for SchemaSheets format
//!
//! These tests verify that schema → Excel → schema conversion is lossless.

use linkml_core::types::SchemaDefinition;
use linkml_service::parser::SchemaLoader;
use linkml_service::schemasheets::{SchemaSheetsGenerator, SchemaSheetsParser};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to load a YAML schema
async fn load_yaml_schema(path: &str) -> SchemaDefinition {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("schemas")
        .join(path);

    let loader = SchemaLoader::new();
    loader.load_file(&schema_path).await.unwrap()
}

/// Test roundtrip with biolink_minimal schema
///
/// Note: This test requires linkml:types import resolution.
/// Import resolution is supported via ImportResolverV2, but requires
/// proper configuration of import aliases in the schema settings.
#[tokio::test]
#[ignore = "Requires linkml:types import configuration"]
async fn test_roundtrip_biolink_minimal() {
    // Check if the schema file exists
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("schemas")
        .join("biolink_minimal.yaml");

    if !schema_path.exists() {
        println!("Skipping test: biolink_minimal.yaml not found");
        return;
    }

    // Load original schema
    let original_schema = load_yaml_schema("biolink_minimal.yaml").await;

    // Generate Excel file
    let temp_dir = TempDir::new().unwrap();
    let excel_path = temp_dir.path().join("biolink_minimal.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator
        .generate_file(&original_schema, &excel_path)
        .await
        .unwrap();

    // Parse it back
    let parser = SchemaSheetsParser::new();
    let roundtrip_schema = parser
        .parse_file(&excel_path, Some(&original_schema.name))
        .unwrap();

    // Verify basic properties
    assert_eq!(roundtrip_schema.id, original_schema.id);
    assert_eq!(roundtrip_schema.name, original_schema.name);

    // Verify classes are preserved
    for (class_name, original_class) in &original_schema.classes {
        assert!(
            roundtrip_schema.classes.contains_key(class_name),
            "Class {} should be preserved",
            class_name
        );

        let roundtrip_class = &roundtrip_schema.classes[class_name];

        // Verify class properties
        assert_eq!(roundtrip_class.description, original_class.description);
        assert_eq!(roundtrip_class.is_a, original_class.is_a);

        // Verify attributes
        for (attr_name, original_attr) in &original_class.attributes {
            assert!(
                roundtrip_class.attributes.contains_key(attr_name),
                "Attribute {} should be preserved in class {}",
                attr_name,
                class_name
            );

            let roundtrip_attr = &roundtrip_class.attributes[attr_name];
            assert_eq!(roundtrip_attr.range, original_attr.range);
            assert_eq!(roundtrip_attr.required, original_attr.required);
        }
    }
}

/// Test roundtrip with api_models schema
///
/// Note: This test requires linkml:types import resolution.
/// Import resolution is supported via ImportResolverV2, but requires
/// proper configuration of import aliases in the schema settings.
#[tokio::test]
#[ignore = "Requires linkml:types import configuration"]
async fn test_roundtrip_api_models() {
    let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data")
        .join("schemas")
        .join("api_models.yaml");

    if !schema_path.exists() {
        println!("Skipping test: api_models.yaml not found");
        return;
    }

    let original_schema = load_yaml_schema("api_models.yaml").await;

    let temp_dir = TempDir::new().unwrap();
    let excel_path = temp_dir.path().join("api_models.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator
        .generate_file(&original_schema, &excel_path)
        .await
        .unwrap();

    let parser = SchemaSheetsParser::new();
    let roundtrip_schema = parser
        .parse_file(&excel_path, Some(&original_schema.name))
        .unwrap();

    // Verify schema metadata
    assert_eq!(roundtrip_schema.id, original_schema.id);
    assert_eq!(roundtrip_schema.name, original_schema.name);

    // Verify subsets are preserved
    for (subset_name, original_subset) in &original_schema.subsets {
        assert!(
            roundtrip_schema.subsets.contains_key(subset_name),
            "Subset {} should be preserved",
            subset_name
        );

        let roundtrip_subset = &roundtrip_schema.subsets[subset_name];
        assert_eq!(roundtrip_subset.description, original_subset.description);
    }
}

/// Test roundtrip with complex schema containing all element types
#[tokio::test]
async fn test_roundtrip_complex_schema() {
    use linkml_core::types::{
        ClassDefinition, EnumDefinition, PermissibleValue, PrefixDefinition, SlotDefinition,
        SubsetDefinition, TypeDefinition,
    };

    // Create a complex schema with all element types
    let mut schema = SchemaDefinition {
        id: "https://example.org/complex_schema".to_string(),
        name: "complex_schema".to_string(),
        version: Some("1.0.0".to_string()),
        description: Some("A complex test schema".to_string()),
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

    // Add a class with inheritance and mappings
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person entity".to_string()),
        exact_mappings: vec!["schema:Person".to_string()],
        close_mappings: vec!["foaf:Person".to_string()],
        ..Default::default()
    };

    // Add attributes with various constraints
    let id_slot = SlotDefinition {
        name: "id".to_string(),
        identifier: Some(true),
        required: Some(true),
        range: Some("string".to_string()),
        description: Some("Unique identifier".to_string()),
        ..Default::default()
    };

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        required: Some(true),
        range: Some("string".to_string()),
        description: Some("Person's name".to_string()),
        exact_mappings: vec!["schema:name".to_string()],
        ..Default::default()
    };

    let email_slot = SlotDefinition {
        name: "email".to_string(),
        required: Some(false),
        range: Some("EmailType".to_string()),
        description: Some("Email address".to_string()),
        pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
        ..Default::default()
    };

    person_class.attributes.insert("id".to_string(), id_slot);
    person_class
        .attributes
        .insert("name".to_string(), name_slot);
    person_class
        .attributes
        .insert("email".to_string(), email_slot);

    schema.classes.insert("Person".to_string(), person_class);

    // Add a child class
    let employee_class = ClassDefinition {
        name: "Employee".to_string(),
        description: Some("An employee".to_string()),
        is_a: Some("Person".to_string()),
        ..Default::default()
    };

    schema
        .classes
        .insert("Employee".to_string(), employee_class);

    // Add an enum
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        description: Some("Status values".to_string()),
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

    // Add a type
    let email_type = TypeDefinition {
        name: "EmailType".to_string(),
        description: Some("Email address type".to_string()),
        base_type: Some("string".to_string()),
        pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
        ..Default::default()
    };

    schema.types.insert("EmailType".to_string(), email_type);

    // Add a subset
    let required_subset = SubsetDefinition {
        name: "required".to_string(),
        description: Some("Required fields".to_string()),
        ..Default::default()
    };

    schema
        .subsets
        .insert("required".to_string(), required_subset);

    // Generate Excel
    let temp_dir = TempDir::new().unwrap();
    let excel_path = temp_dir.path().join("complex_schema.xlsx");

    let generator = SchemaSheetsGenerator::new();
    generator.generate_file(&schema, &excel_path).await.unwrap();

    // Parse it back
    let parser = SchemaSheetsParser::new();
    let roundtrip_schema = parser
        .parse_file(&excel_path, Some("complex_schema"))
        .unwrap();

    // Verify all elements are preserved
    assert_eq!(roundtrip_schema.id, schema.id);
    assert_eq!(roundtrip_schema.name, schema.name);
    assert_eq!(roundtrip_schema.version, schema.version);
    assert_eq!(roundtrip_schema.description, schema.description);

    // Verify prefixes
    assert_eq!(roundtrip_schema.prefixes.len(), schema.prefixes.len());
    assert!(roundtrip_schema.prefixes.contains_key("schema"));
    assert!(roundtrip_schema.prefixes.contains_key("foaf"));

    // Verify classes
    assert_eq!(roundtrip_schema.classes.len(), schema.classes.len());
    assert!(roundtrip_schema.classes.contains_key("Person"));
    assert!(roundtrip_schema.classes.contains_key("Employee"));

    // Verify Person class details
    let roundtrip_person = &roundtrip_schema.classes["Person"];
    assert_eq!(
        roundtrip_person.description,
        schema.classes["Person"].description
    );
    assert!(!roundtrip_person.exact_mappings.is_empty());
    assert!(!roundtrip_person.close_mappings.is_empty());

    // Verify Employee inheritance
    let roundtrip_employee = &roundtrip_schema.classes["Employee"];
    assert_eq!(roundtrip_employee.is_a, Some("Person".to_string()));

    // Verify attributes
    assert_eq!(roundtrip_person.attributes.len(), 3);
    assert!(roundtrip_person.attributes.contains_key("id"));
    assert!(roundtrip_person.attributes.contains_key("name"));
    assert!(roundtrip_person.attributes.contains_key("email"));

    // Verify attribute details
    let roundtrip_id = &roundtrip_person.attributes["id"];
    assert_eq!(roundtrip_id.identifier, Some(true));
    assert_eq!(roundtrip_id.required, Some(true));

    let roundtrip_name = &roundtrip_person.attributes["name"];
    assert!(!roundtrip_name.exact_mappings.is_empty());

    let roundtrip_email = &roundtrip_person.attributes["email"];
    assert!(roundtrip_email.pattern.is_some());

    // Verify enums
    assert_eq!(roundtrip_schema.enums.len(), schema.enums.len());
    assert!(roundtrip_schema.enums.contains_key("Status"));
    let roundtrip_status = &roundtrip_schema.enums["Status"];
    assert_eq!(roundtrip_status.permissible_values.len(), 2);

    // Verify types
    assert_eq!(roundtrip_schema.types.len(), schema.types.len());
    assert!(roundtrip_schema.types.contains_key("EmailType"));
    let roundtrip_email_type = &roundtrip_schema.types["EmailType"];
    assert_eq!(roundtrip_email_type.base_type, Some("string".to_string()));
    assert!(roundtrip_email_type.pattern.is_some());

    // Verify subsets
    assert_eq!(roundtrip_schema.subsets.len(), schema.subsets.len());
    assert!(roundtrip_schema.subsets.contains_key("required"));
}
