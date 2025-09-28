//! Comprehensive tests for the Rust code generator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, PermissibleValue, SlotDefinition};
use linkml_service::generator::{Generator, GeneratorOptions, RustGenerator};
use serde_json::json;

async fn generate_rust(schema: SchemaDefinition) -> String {
    let generator = RustGenerator::new();
    let options = GeneratorOptions::new().with_docs(true);

    generator.generate(&schema).expect("Test operation failed")
}

#[tokio::test]
async fn test_simple_struct_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        description: Some("Test schema for Rust generation".to_string()),
        ..Default::default()
    };

    // Create Person class
    let person = ClassDefinition {
        name: "Person".to_string(),
        description: Some("Represents a person".to_string()),
        slots: vec!["id".to_string(), "name".to_string(), "age".to_string()],
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person);

    // Define slots
    schema.slots.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            description: Some("Unique identifier".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            description: Some("Person's full name".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            description: Some("Age in years".to_string()),
            range: Some("integer".to_string()),
            minimum_value: Some(json!(0)),
            maximum_value: Some(json!(150)),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check imports
    assert!(output.contains("use serde::{Deserialize, Serialize};"));

    // Check struct definition
    assert!(output.contains("/// Represents a person"));
    assert!(output.contains("pub struct Person {"));
    assert!(output.contains("#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]"));

    // Check fields
    assert!(output.contains("/// Unique identifier"));
    assert!(output.contains("pub id: String,"));
    assert!(output.contains("/// Person's full name"));
    assert!(output.contains("pub name: String,"));
    assert!(output.contains("/// Age in years"));
    assert!(output.contains("pub age: Option<i64>,"));

    // Check constructor
    assert!(output.contains("impl Person {"));
    assert!(output.contains("pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self"));

    // Check validation
    assert!(output.contains("pub fn validate(&self) -> Result<(), Vec<ValidationError>>"));
    assert!(output.contains("if self.id.is_empty()"));
    assert!(output.contains("if self.name.is_empty()"));
}

#[tokio::test]
async fn test_enum_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_enum".to_string(),
        ..Default::default()
    };

    let person = ClassDefinition {
        name: "Person".to_string(),
        slots: vec!["name".to_string(), "status".to_string()],
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person);

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "status".to_string(),
        SlotDefinition {
            name: "status".to_string(),
            description: Some("Current status".to_string()),
            permissible_values: vec![
                PermissibleValue::Simple("active".to_string()),
                PermissibleValue::Simple("inactive".to_string()),
                PermissibleValue::Complex {
                    text: "pending-review".to_string(),
                    description: Some("Awaiting review".to_string()),
                    meaning: None,
                },
            ],
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check enum generation
    assert!(output.contains("/// Current status"));
    assert!(output.contains("pub enum Status {"));
    assert!(output.contains("Active,"));
    assert!(output.contains("Inactive,"));
    assert!(output.contains("/// Awaiting review"));
    assert!(output.contains("PendingReview,"));

    // Check Display implementation
    assert!(output.contains("impl std::fmt::Display for Status"));
    assert!(output.contains("Status::Active => write!(f, \"active\")"));
    assert!(output.contains("Status::PendingReview => write!(f, \"pending-review\")"));

    // Check FromStr implementation
    assert!(output.contains("impl std::str::FromStr for Status"));
    assert!(output.contains("\"active\" => Ok(Status::Active)"));
    assert!(output.contains("\"pending-review\" => Ok(Status::PendingReview)"));

    // Check struct uses the enum
    assert!(output.contains("pub status: Option<Status>,"));
}

#[tokio::test]
async fn test_pattern_validation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_pattern".to_string(),
        ..Default::default()
    };

    let contact = ClassDefinition {
        name: "Contact".to_string(),
        slots: vec!["email".to_string(), "phone".to_string()],
        ..Default::default()
    };

    schema.classes.insert("Contact".to_string(), contact);

    schema.slots.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            description: Some("Email address".to_string()),
            range: Some("string".to_string()),
            pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "phone".to_string(),
        SlotDefinition {
            name: "phone".to_string(),
            range: Some("string".to_string()),
            pattern: Some(r"^\+?[\d\s\-()]+$".to_string()),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check regex imports
    assert!(output.contains("use once_cell::sync::Lazy;"));
    assert!(output.contains("use regex::Regex;"));

    // Check pattern constants
    assert!(output.contains("static PATTERN_EMAIL: Lazy<Regex>"));
    assert!(output.contains(r#"Regex::new(r"^[\w\.-]+@[\w\.-]+\.\w+$")"#));
    assert!(output.contains("static PATTERN_PHONE: Lazy<Regex>"));

    // Check validation uses patterns
    assert!(output.contains("if !PATTERN_EMAIL.is_match(&self.email)"));
    assert!(output.contains("if let Some(ref value) = self.phone"));
    assert!(output.contains("if !PATTERN_PHONE.is_match(value)"));
}

#[tokio::test]
async fn test_multivalued_fields() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_arrays".to_string(),
        ..Default::default()
    };

    let group = ClassDefinition {
        name: "Group".to_string(),
        slots: vec![
            "name".to_string(),
            "members".to_string(),
            "tags".to_string(),
        ],
        ..Default::default()
    };

    schema.classes.insert("Group".to_string(), group);

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "members".to_string(),
        SlotDefinition {
            name: "members".to_string(),
            description: Some("Group members".to_string()),
            range: Some("string".to_string()),
            multivalued: Some(true),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "tags".to_string(),
        SlotDefinition {
            name: "tags".to_string(),
            range: Some("string".to_string()),
            multivalued: Some(true),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check multivalued fields
    assert!(output.contains("pub members: Vec<String>,"));
    assert!(output.contains("#[serde(default, skip_serializing_if = \"Vec::is_empty\")]"));
    assert!(output.contains("pub tags: Vec<String>,"));

    // Check constructor - required multivalued field should be a parameter
    assert!(
        output.contains(
            "pub fn new(name: impl Into<String>, members: impl Into<Vec<String>>) -> Self"
        )
    );
    // Optional multivalued fields are initialized as empty
    assert!(output.contains("tags: Vec::new(),"));
}

#[tokio::test]
async fn test_inheritance() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_inheritance".to_string(),
        ..Default::default()
    };

    // Base class
    let entity = ClassDefinition {
        name: "NamedEntity".to_string(),
        description: Some("Base entity with id and name".to_string()),
        slots: vec!["id".to_string(), "name".to_string()],
        ..Default::default()
    };

    // Derived class
    let person = ClassDefinition {
        name: "Person".to_string(),
        description: Some("Person extends NamedEntity".to_string()),
        is_a: Some("NamedEntity".to_string()),
        slots: vec!["age".to_string(), "email".to_string()],
        ..Default::default()
    };

    schema.classes.insert("NamedEntity".to_string(), entity);
    schema.classes.insert("Person".to_string(), person);

    // Define slots
    schema.slots.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check Person has all fields (inherited + own)
    let person_section = output
        .split("pub struct Person")
        .nth(1)
        .expect("Test operation failed");
    assert!(person_section.contains("pub id: String,"));
    assert!(person_section.contains("pub name: String,"));
    assert!(person_section.contains("pub age: Option<i64>,"));
    assert!(person_section.contains("pub email: Option<String>,"));

    // Check constructor accepts all required fields
    assert!(output.contains("pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self"));
}

#[tokio::test]
async fn test_datetime_fields() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_datetime".to_string(),
        ..Default::default()
    };

    let event = ClassDefinition {
        name: "Event".to_string(),
        slots: vec![
            "name".to_string(),
            "date".to_string(),
            "timestamp".to_string(),
        ],
        ..Default::default()
    };

    schema.classes.insert("Event".to_string(), event);

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "date".to_string(),
        SlotDefinition {
            name: "date".to_string(),
            description: Some("Event date".to_string()),
            range: Some("date".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "timestamp".to_string(),
        SlotDefinition {
            name: "timestamp".to_string(),
            description: Some("Event timestamp".to_string()),
            range: Some("datetime".to_string()),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check chrono imports
    assert!(output.contains("use chrono::{DateTime, NaiveDate, NaiveTime, Utc};"));

    // Check field types
    assert!(output.contains("pub date: Option<chrono::NaiveDate>,"));
    assert!(output.contains("pub timestamp: Option<chrono::DateTime<chrono::Utc>>,"));
}

#[tokio::test]
async fn test_builder_pattern() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_builder".to_string(),
        ..Default::default()
    };

    let person = ClassDefinition {
        name: "Person".to_string(),
        slots: vec!["id".to_string(), "name".to_string(), "age".to_string()],
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person);

    schema.slots.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            ..Default::default()
        },
    );

    let generator = RustGenerator::new();
    let options = GeneratorOptions::new().set_custom("generate_builder", "true");

    let output = generator.generate(&schema).expect("Test operation failed");

    // Check builder struct
    assert!(output.contains("pub struct PersonBuilder"));
    assert!(output.contains("#[derive(Default)]"));

    // Check builder methods
    assert!(output.contains("impl PersonBuilder"));
    assert!(output.contains("pub fn id(mut self, value: String) -> Self"));
    assert!(output.contains("pub fn name(mut self, value: String) -> Self"));
    assert!(output.contains("pub fn age(mut self, value: i64) -> Self"));
    assert!(output.contains("pub fn build(self) -> Person"));
}

#[tokio::test]
async fn test_field_name_edge_cases() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_field_names".to_string(),
        ..Default::default()
    };

    let data = ClassDefinition {
        name: "Data".to_string(),
        slots: vec![
            "type".to_string(),
            "firstName".to_string(),
            "XMLData".to_string(),
            "self".to_string(),
        ],
        ..Default::default()
    };

    schema.classes.insert("Data".to_string(), data);

    // Define slots with problematic names
    schema.slots.insert(
        "type".to_string(),
        SlotDefinition {
            name: "type".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "firstName".to_string(),
        SlotDefinition {
            name: "firstName".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "XMLData".to_string(),
        SlotDefinition {
            name: "XMLData".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "self".to_string(),
        SlotDefinition {
            name: "self".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check field name conversions
    assert!(output.contains("pub type_: Option<String>,"));
    assert!(output.contains(r#"#[serde(rename = "type")]"#));

    assert!(output.contains("pub first_name: Option<String>,"));
    assert!(output.contains(r#"#[serde(rename = "firstName")]"#));

    assert!(output.contains("pub xmldata: Option<String>,"));
    assert!(output.contains(r#"#[serde(rename = "XMLData")]"#));

    assert!(output.contains("pub self_: Option<String>,"));
    assert!(output.contains(r#"#[serde(rename = "self")]"#));
}

#[tokio::test]
async fn test_range_validation_comprehensive() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_ranges".to_string(),
        ..Default::default()
    };

    let measurement = ClassDefinition {
        name: "Measurement".to_string(),
        slots: vec![
            "temperature".to_string(),
            "percentage".to_string(),
            "count".to_string(),
        ],
        ..Default::default()
    };

    schema
        .classes
        .insert("Measurement".to_string(), measurement);

    schema.slots.insert(
        "temperature".to_string(),
        SlotDefinition {
            name: "temperature".to_string(),
            description: Some("Temperature in Celsius".to_string()),
            range: Some("float".to_string()),
            minimum_value: Some(json!(-273.15)),
            maximum_value: Some(json!(1000.0)),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "percentage".to_string(),
        SlotDefinition {
            name: "percentage".to_string(),
            range: Some("float".to_string()),
            minimum_value: Some(json!(0.0)),
            maximum_value: Some(json!(100.0)),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "count".to_string(),
        SlotDefinition {
            name: "count".to_string(),
            range: Some("integer".to_string()),
            minimum_value: Some(json!(0)),
            ..Default::default()
        },
    );

    let output = generate_rust(schema).await;

    // Check validation generates range checks
    assert!(output.contains("if temperature < -273.15"));
    assert!(output.contains("if temperature > 1000"));

    assert!(output.contains("if let Some(value) = self.percentage"));
    assert!(output.contains("if value < 0"));
    assert!(output.contains("if value > 100"));

    assert!(output.contains("if let Some(value) = self.count"));
    assert!(output.contains("if value < 0"));
}
