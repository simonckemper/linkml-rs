//! Tests for the Python dataclass generator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, PermissibleValue, SlotDefinition};
use linkml_service::generator::{Generator, GeneratorOptions, PythonDataclassGenerator};
use serde_json::json;
async fn generate_python(schema: SchemaDefinition) -> String {
    let generator = PythonDataclassGenerator::new();
    let options = GeneratorOptions::new()
        .with_docs(true)
        .with_examples(true)
        .set_custom("generate_validation", "true");

    generator.generate(&schema).expect("Test operation failed")
}

#[tokio::test]
async fn test_simple_class_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        description: Some("Test schema for Python generation".to_string()),
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

    let output = generate_python(schema).await;

    // Check imports
    assert!(output.contains("from dataclasses import dataclass"));
    assert!(output.contains("from typing import Optional"));

    // Check class definition
    assert!(output.contains("@dataclass"));
    assert!(output.contains("class Person:"));
    assert!(output.contains("Represents a person"));

    // Check fields
    assert!(output.contains("id: str"));
    assert!(output.contains("name: str"));
    assert!(output.contains("age: Optional[int] = None"));

    // Check validation
    assert!(output.contains("def __post_init__(self):"));
    assert!(output.contains("if self.age is not None and self.age < 0:"));
    assert!(output.contains("if self.age is not None and self.age > 150:"));
}

#[tokio::test]
async fn test_multivalued_fields() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_multivalued".to_string(),
        ..Default::default()
    };

    let group = ClassDefinition {
        name: "Group".to_string(),
        description: Some("A group of people".to_string()),
        slots: vec!["name".to_string(), "members".to_string()],
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
            ..Default::default()
        },
    );

    let output = generate_python(schema).await;

    assert!(output.contains("from typing import List"));
    assert!(output.contains("members: List[str] = field(default_factory=list)"));
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
            description: Some("Person's status".to_string()),
            permissible_values: vec![
                PermissibleValue::Simple("active".to_string()),
                PermissibleValue::Simple("inactive".to_string()),
                PermissibleValue::Simple("pending".to_string()),
            ],
            ..Default::default()
        },
    );

    let output = generate_python(schema).await;

    // Check enum generation
    assert!(output.contains("from enum import Enum"));
    assert!(output.contains("class Status(Enum):"));
    assert!(output.contains("ACTIVE = \"active\""));
    assert!(output.contains("INACTIVE = \"inactive\""));
    assert!(output.contains("PENDING = \"pending\""));

    // Check field uses enum
    assert!(output.contains("status: Optional[Status] = None"));
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
        description: Some("Base class for named entities".to_string()),
        slots: vec!["id".to_string(), "name".to_string()],
        ..Default::default()
    };

    // Derived class
    let person = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person is a named entity".to_string()),
        is_a: Some("NamedEntity".to_string()),
        slots: vec!["age".to_string()],
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

    let output = generate_python(schema).await;

    // Check both classes are generated
    assert!(output.contains("class NamedEntity:"));
    assert!(output.contains("class Person:"));

    // Check Person has all fields (inherited + own)
    let person_section = output
        .split("class Person:")
        .nth(1)
        .expect("Test operation failed");
    assert!(person_section.contains("id: str"));
    assert!(person_section.contains("name: str"));
    assert!(person_section.contains("age: Optional[int] = None"));
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
            range: Some("string".to_string()),
            pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
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

    let generator = PythonDataclassGenerator::new();
    let options = GeneratorOptions::new().set_custom("generate_validation", "true");

    let output = generator.generate(&schema).expect("Test operation failed");

    // Check pattern validation in __post_init__
    assert!(output.contains("import re"));
    assert!(output.contains("if self.email is not None and not re.match"));
    assert!(output.contains(r#"r"^[\w\.-]+@[\w\.-]+\.\w+$""#));
    assert!(output.contains("if self.phone is not None and not re.match"));
}
