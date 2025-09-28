//! Parser tests for LinkML service

use linkml_service::parser::Parser;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parse_simple_yaml_schema() {
    let yaml_content = r#"
id: https://example.org/test
name: test_schema
description: A simple test schema

classes:
  Person:
    name: Person
    description: A person
    slots:
      - name
      - age

slots:
  name:
    name: name
    description: Person's name
    range: string
    required: true
  age:
    name: age
    description: Person's age
    range: integer
"#;

    // Write to temp file
    let temp_dir = TempDir::new().expect("Test operation failed");
    let schema_path = temp_dir.path().join("test_schema.yaml");
    fs::write(&schema_path, yaml_content).expect("Test operation failed");

    // Test parsing
    let parser = Parser::new();
    let schema = parser
        .parse_file(&schema_path)
        .expect("Test operation failed");

    assert_eq!(schema.name, "test_schema");
    assert_eq!(schema.id, "https://example.org/test");
    assert!(schema.classes.contains_key("Person"));
    assert_eq!(schema.slots.len(), 2);
    assert!(schema.slots.contains_key("name"));
    assert!(schema.slots.contains_key("age"));

    // Check slot details
    let name_slot = &schema.slots["name"];
    assert_eq!(
        name_slot.range.as_ref().expect("Test operation failed"),
        "string"
    );
    assert_eq!(name_slot.required, Some(true));

    let age_slot = &schema.slots["age"];
    assert_eq!(
        age_slot.range.as_ref().expect("Test operation failed"),
        "integer"
    );
}

#[test]
fn test_parse_json_schema() {
    let json_content = r#"{
        "id": "https://example.org/test",
        "name": "test_schema",
        "description": "A simple test schema",
        "classes": {
            "Person": {
                "name": "Person",
                "description": "A person",
                "slots": ["name", "age"]
            }
        },
        "slots": {
            "name": {
                "name": "name",
                "description": "Person's name",
                "range": "string",
                "required": true
            },
            "age": {
                "name": "age",
                "description": "Person's age",
                "range": "integer"
            }
        }
    }"#;

    // Write to temp file
    let temp_dir = TempDir::new().expect("Test operation failed");
    let schema_path = temp_dir.path().join("test_schema.json");
    fs::write(&schema_path, json_content).expect("Test operation failed");

    // Test parsing
    let parser = Parser::new();
    let schema = parser
        .parse_file(&schema_path)
        .expect("Test operation failed");

    assert_eq!(schema.name, "test_schema");
    assert_eq!(schema.id, "https://example.org/test");
    assert!(schema.classes.contains_key("Person"));
    assert_eq!(schema.slots.len(), 2);
}

#[test]
fn test_parse_schema_with_types_and_enums() {
    let yaml_content = r#"
id: https://example.org/test
name: test_schema

types:
  EmailAddress:
    name: EmailAddress
    typeof: string
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"

enums:
  StatusEnum:
    name: StatusEnum
    permissible_values:
      - ACTIVE
      - INACTIVE
      - PENDING

classes:
  User:
    name: User
    slots:
      - email
      - status

slots:
  email:
    name: email
    range: EmailAddress
  status:
    name: status
    range: StatusEnum
"#;

    let temp_dir = TempDir::new().expect("Test operation failed");
    let schema_path = temp_dir.path().join("test_schema.yaml");
    fs::write(&schema_path, yaml_content).expect("Test operation failed");

    let parser = Parser::new();
    let schema = parser
        .parse_file(&schema_path)
        .expect("Test operation failed");

    // Check types
    assert!(schema.types.contains_key("EmailAddress"));
    let email_type = &schema.types["EmailAddress"];
    assert_eq!(
        email_type
            .base_type
            .as_ref()
            .expect("Test operation failed"),
        "string"
    );
    assert!(email_type.pattern.is_some());

    // Check enums
    assert!(schema.enums.contains_key("StatusEnum"));
    let status_enum = &schema.enums["StatusEnum"];
    assert_eq!(status_enum.permissible_values.len(), 3);
}

#[test]
fn test_parse_invalid_yaml() {
    let yaml_content = "invalid: yaml: content: bad";

    let temp_dir = TempDir::new().expect("Test operation failed");
    let schema_path = temp_dir.path().join("invalid.yaml");
    fs::write(&schema_path, yaml_content).expect("Test operation failed");

    let parser = Parser::new();
    let result = parser.parse_file(&schema_path);

    assert!(result.is_err());
}

#[test]
fn test_parse_string_directly() {
    let yaml = r#"
id: https://example.org/test
name: direct_parse_test
"#;

    let parser = Parser::new();
    let schema = parser
        .parse_str(yaml, "yaml")
        .expect("Test operation failed");

    assert_eq!(schema.name, "direct_parse_test");
    assert_eq!(schema.id, "https://example.org/test");
}
