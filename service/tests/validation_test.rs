//! Tests for validation functionality

use linkml_service::validator::validate_as_class;
use serde_json::json;

#[tokio::test]
async fn test_validate_simple_schema() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for validation

classes:
  Person:
    name: Person
    description: A person
    slots:
      - name
      - age
      - email

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
    minimum_value: 0
    maximum_value: 150

  email:
    name: email
    description: Email address
    range: string
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"
"#;

    // Parse schema
    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Valid data
    let valid_data = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john@example.com"
    });

    let report = validate_as_class(&schema, &valid_data, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);

    // Invalid data - missing required field
    let invalid_data = json!({
        "age": 30,
        "email": "john@example.com"
    });

    let report = validate_as_class(&schema, &invalid_data, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.stats.error_count > 0);

    // Find the required field error
    let required_error = report
        .errors()
        .find(|e| e.message.contains("Required"))
        .expect("Should have required field error");
    assert!(required_error.path.contains("name"));
}

#[tokio::test]
async fn test_validate_type_constraints() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  DataTypes:
    name: DataTypes
    slots:
      - str_field
      - int_field
      - float_field
      - bool_field
      - date_field

slots:
  str_field:
    name: str_field
    range: string

  int_field:
    name: int_field
    range: integer

  float_field:
    name: float_field
    range: float

  bool_field:
    name: bool_field
    range: boolean

  date_field:
    name: date_field
    range: date
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Test wrong types
    let wrong_types = json!({
        "str_field": 123,           // Should be string
        "int_field": "not a number", // Should be integer
        "float_field": "abc",        // Should be float
        "bool_field": "yes",         // Should be boolean
        "date_field": "not-a-date"   // Should be date
    });

    let report = validate_as_class(&schema, &wrong_types, "DataTypes", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.stats.error_count >= 5); // At least one error per field
}

#[tokio::test]
async fn test_validate_pattern() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  Contact:
    name: Contact
    slots:
      - email
      - phone

slots:
  email:
    name: email
    range: string
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"

  phone:
    name: phone
    range: string
    pattern: "^\\+?[0-9]{10,15}$"
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Valid patterns
    let valid_data = json!({
        "email": "test@example.com",
        "phone": "+1234567890"
    });

    let report = validate_as_class(&schema, &valid_data, "Contact", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Invalid patterns
    let invalid_data = json!({
        "email": "not-an-email",
        "phone": "abc123"
    });

    let report = validate_as_class(&schema, &invalid_data, "Contact", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.stats.error_count >= 2);
}

#[tokio::test]
async fn test_validate_range_constraints() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  Measurement:
    name: Measurement
    slots:
      - temperature
      - percentage

slots:
  temperature:
    name: temperature
    range: float
    minimum_value: "-273.15"
    maximum_value: "1000"

  percentage:
    name: percentage
    range: integer
    minimum_value: "0"
    maximum_value: "100"
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Out of range values
    let invalid_data = json!({
        "temperature": -300.0,  // Below absolute zero
        "percentage": 150       // Above 100%
    });

    let report = validate_as_class(&schema, &invalid_data, "Measurement", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    let temp_error = report
        .errors()
        .find(|e| e.path.contains("temperature"))
        .expect("Should have temperature range error");
    assert!(temp_error.message.contains("less than minimum"));

    let percent_error = report
        .errors()
        .find(|e| e.path.contains("percentage"))
        .expect("Should have percentage range error");
    assert!(percent_error.message.contains("exceeds maximum"));
}

#[tokio::test]
async fn test_validate_enum_values() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

enums:
  StatusEnum:
    name: StatusEnum
    permissible_values:
      - ACTIVE
      - INACTIVE
      - PENDING

classes:
  Item:
    name: Item
    slots:
      - status

slots:
  status:
    name: status
    range: StatusEnum
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Valid enum value
    let valid_data = json!({
        "status": "ACTIVE"
    });

    let report = validate_as_class(&schema, &valid_data, "Item", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Invalid enum value
    let invalid_data = json!({
        "status": "UNKNOWN"
    });

    let report = validate_as_class(&schema, &invalid_data, "Item", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    let enum_error = report
        .errors()
        .find(|e| e.message.contains("permissible values"))
        .expect("Should have enum validation error");
    assert!(enum_error.path.contains("status"));
}

// TODO: Add cardinality and unique validation when LinkML core types support these fields
/*
#[tokio::test]
async fn test_validate_multivalued() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  Group:
    name: Group
    slots:
      - members
      - tags

slots:
  members:
    name: members
    range: string
    multivalued: true
    minimum_cardinality: 1
    maximum_cardinality: 10

  tags:
    name: tags
    range: string
    multivalued: true
    unique: true
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser.parse(schema_yaml, "yaml").expect("Test operation failed");

    // Test cardinality
    let too_many = json!({
        "members": ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"], // 11 items
        "tags": ["tag1"]
    });

    let report = validate(&schema, &too_many, None).await.expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.errors().any(|e| e.message.contains("maximum cardinality"));

    // Test uniqueness
    let duplicates = json!({
        "members": ["alice"],
        "tags": ["tag1", "tag2", "tag1"] // Duplicate tag1
    });

    let report = validate(&schema, &duplicates, None).await.expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.errors().any(|e| e.message.contains("Duplicate"));
}
*/

#[tokio::test]
async fn test_validate_with_inheritance() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  NamedThing:
    name: NamedThing
    slots:
      - id
      - name

  Person:
    name: Person
    is_a: NamedThing
    slots:
      - age

slots:
  id:
    name: id
    range: string
    required: true
    identifier: true

  name:
    name: name
    range: string
    required: true

  age:
    name: age
    range: integer
"#;

    let parser = linkml_service::parser::Parser::new();
    let schema = parser
        .parse_str(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Person should inherit id and name slots
    let person_missing_inherited = json!({
        "age": 30
    });

    let engine =
        linkml_service::validator::ValidationEngine::new(&schema).expect("Test operation failed");
    let report = engine
        .validate_as_class(&person_missing_inherited, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    // Should have errors for missing id and name
    assert!(report.errors().any(|e| e.path.contains("id")));
    assert!(report.errors().any(|e| e.path.contains("name")));
}
