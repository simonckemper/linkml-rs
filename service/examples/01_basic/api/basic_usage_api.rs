//! Basic usage API demonstration for LinkML service
//!
//! This example demonstrates the core LinkML service API:
//! - Loading schemas from YAML/JSON
//! - Basic validation
//! - Working with classes and slots
//! - Understanding validation reports
//!
//! NOTE: This is an API demonstration. In production, you would
//! initialize the service with RootReal service dependencies.

use linkml_core::prelude::*;
use serde_json::json;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Basic Usage API Demonstration");
    println!(
        "===================================
"
    );

    // Show basic schema structure
    demonstrate_schema_basics();

    // Show validation basics
    demonstrate_validation_basics()?;

    // Show validation report structure
    demonstrate_validation_reports()?;

    // Show schema features
    demonstrate_schema_features();
    Ok(())
}

fn demonstrate_schema_basics() {
    println!(
        "1. Basic Schema Structure:
"
    );

    let _schema_yaml = r#"
id: https://example.org/basic-schema
name: BasicSchema
description: A simple schema for demonstration

classes:
  Person:
    description: A human being
    slots:
      - id
      - name
      - email
      - age

  Organization:
    description: A company or institution
    slots:
      - id
      - name
      - founded_year
      - employees

slots:
  id:
    identifier: true
    range: string
    description: Unique identifier

  name:
    range: string
    required: true
    description: Full name

  email:
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[A-Z|a-z]{2,}$"
    description: Email address

  age:
    range: integer
    minimum_value: 0
    maximum_value: 150
    description: Age in years

  founded_year:
    range: integer
    minimum_value: 1800
    description: Year the organization was founded

  employees:
    range: Person
    multivalued: true
    description: List of employees
"#;

    println!("Schema components:");
    println!("- Classes: Define the structure of your data");
    println!("- Slots: Define the fields/properties");
    println!("- Ranges: Specify the data type");
    println!("- Constraints: Add validation rules");
}

fn demonstrate_validation_basics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
2. Basic Validation Examples:
"
    );

    // Valid data
    let valid_person = json!({
        "id": "person-001",
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "age": 30
    });

    println!("Valid Person:");
    println!("{}", serde_json::to_string_pretty(&valid_person)?);
    println!("✓ Passes all validation rules");

    // Invalid data - missing required field
    let missing_name = json!({
        "id": "person-002",
        "email": "bob@example.com",
        "age": 25
    });

    println!(
        "
Invalid - Missing required 'name':"
    );
    println!("{}", serde_json::to_string_pretty(&missing_name)?);
    println!("✗ Error: Required field 'name' is missing");

    // Invalid data - pattern mismatch
    let invalid_email = json!({
        "id": "person-003",
        "name": "Charlie Brown",
        "email": "not-an-email",
        "age": 35
    });

    println!(
        "
Invalid - Bad email format:"
    );
    println!("{}", serde_json::to_string_pretty(&invalid_email)?);
    println!("✗ Error: 'not-an-email' doesn't match email pattern");

    // Invalid data - range violation
    let invalid_age = json!({
        "id": "person-004",
        "name": "David Smith",
        "email": "david@example.com",
        "age": 200
    });

    println!(
        "
Invalid - Age out of range:"
    );
    println!("{}", serde_json::to_string_pretty(&invalid_age)?);
    println!("✗ Error: Age 200 exceeds maximum value 150");
    Ok(())
}

fn demonstrate_validation_reports() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
3. Understanding Validation Reports:
"
    );

    println!("A ValidationReport contains:");
    println!("- valid: boolean indicating overall validity");
    println!("- errors: list of validation errors");
    println!("- warnings: list of validation warnings");
    println!("- info: additional information");

    // Example report structure
    let example_report = ValidationReport {
        valid: false,
        errors: vec![
            ValidationError {
                path: Some("Person.email".to_string()),
                message: "Email 'invalid@' doesn't match required pattern".to_string(),
                expected: Some("Valid email format".to_string()),
                actual: Some("invalid@".to_string()),
                severity: Severity::Error,
            },
            ValidationError {
                path: Some("Person.name".to_string()),
                message: "Required field 'name' is missing".to_string(),
                expected: Some("Non-empty string".to_string()),
                actual: Some("null".to_string()),
                severity: Severity::Error,
            },
        ],
        warnings: vec![],
        timestamp: Some(chrono::Utc::now()),
        schema_id: Some("https://example.org/book-schema".to_string()),
    };

    println!(
        "
Example validation report:"
    );
    println!("Valid: {}", example_report.valid);
    println!("Errors: {}", example_report.errors.len());

    for (i, error) in example_report.errors.iter().enumerate() {
        println!(
            "
Error {}:",
            i + 1
        );
        println!(
            "  Path: {}",
            error.path.as_ref().unwrap_or(&"<unknown>".to_string())
        );
        println!("  Severity: {:?}", error.severity);
        println!("  Message: {}", error.message);
        if let Some(expected) = &error.expected {
            println!("  Expected: {}", expected);
        }
    }
    Ok(())
}

fn demonstrate_schema_features() {
    println!(
        "
4. Advanced Schema Features:
"
    );

    println!("a) Inheritance:");
    let inheritance_example = r#"
classes:
  NamedThing:
    abstract: true
    slots:
      - id
      - name

  Person:
    is_a: NamedThing  # Inherits id and name
    slots:
      - email
      - age
"#;
    println!("{}", inheritance_example);

    println!(
        "
b) Enums:"
    );
    let enum_example = r#"
enums:
  StatusEnum:
    permissible_values:
      - active
      - inactive
      - pending

slots:
  status:
    range: StatusEnum
    required: true
"#;
    println!("{}", enum_example);

    println!(
        "
c) Multivalued slots:"
    );
    let multivalued_example = r#"
slots:
  tags:
    range: string
    multivalued: true
    minimum_cardinality: 1
    maximum_cardinality: 10
"#;
    println!("{}", multivalued_example);

    println!(
        "
d) Complex patterns:"
    );
    let pattern_example = r#"
slots:
  phone:
    range: string
    pattern: "^\\+?[1-9]\\d{1,14}$"  # E.164 format

  postal_code:
    range: string
    pattern: "^[A-Z]{1,2}[0-9][A-Z0-9]? ?[0-9][A-Z]{2}$"  # UK format
"#;
    println!("{}", pattern_example);
}

/// Show how to use the service in production
fn _production_usage_example() {
    println!(
        "
5. Production Usage Pattern:
"
    );

    println!(
        r#"
// In your application code:
async fn validate_person_data(
    linkml: &LinkMLService,
    person_data: &Value,
) -> std::result::Result<(), ValidationError> {{
    // Load schema (typically cached)
    let schema = linkml.load_schema_str(SCHEMA_YAML, SchemaFormat::Yaml).await?;

    // Validate against Person class
    let report = linkml.validate(person_data, &schema, "Person").await?;

    if !report.valid {{
        // Handle validation errors
        for error in &report.errors {{
            log::error!("Validation error at {{}}: {{}}",
                error.path.as_ref().unwrap_or(&"root".to_string()),
                error.message
            );
        }}
        return Err(ValidationError::Invalid(report));
    }}

    Ok(())
}}

// Batch validation for performance:
async fn validate_batch(
    linkml: &LinkMLService,
    documents: Vec<Value>,
) -> Vec<ValidationReport> {{
    futures::future::join_all(
        documents.into_iter().map(|doc| {{
            linkml.validate(&doc, &schema, "Person")
        }})
    ).await
}}
"#
    );
}
