//! Tests for the Pydantic v2 generator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, PermissibleValue, SlotDefinition};
use linkml_service::generator::{Generator, GeneratorOptions, PydanticGenerator};
use serde_json::json;
async fn generate_pydantic(schema: SchemaDefinition) -> String {
    let generator = PydanticGenerator::new();
    let options = GeneratorOptions::new()
        .with_docs(true)
        .set_custom("generate_validators", "true");

    generator.generate(&schema).expect("Test operation failed")
}

#[tokio::test]
async fn test_simple_class_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        description: Some("Test schema for Pydantic generation".to_string()),
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

    let output = generate_pydantic(schema).await;

    // Check imports
    assert!(output.contains("from pydantic import BaseModel"));
    assert!(output.contains("from typing import Optional"));

    // Check class definition
    assert!(output.contains("class Person(BaseModel):"));
    assert!(output.contains("Represents a person"));

    // Check model config
    assert!(output.contains("model_config = {"));
    assert!(output.contains("\"validate_assignment\": True"));
    assert!(output.contains("\"use_enum_values\": True"));

    // Check fields with Field
    assert!(output.contains("id: str = Field(..., description=\"Unique identifier\")"));
    assert!(output.contains("name: str = Field(..., description=\"Person's full name\")"));
    assert!(
        output.contains(
            "age: Optional[int] = Field(None, description=\"Age in years\", ge=0, le=150)"
        )
    );

    // Check examples
    assert!(output.contains("\"examples\":"));
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

    let output = generate_pydantic(schema).await;

    assert!(output.contains("from typing import List"));
    assert!(output.contains("members: List[str] = Field(default_factory=list"));
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

    let output = generate_pydantic(schema).await;

    // Check enum generation
    assert!(output.contains("from enum import Enum"));
    assert!(output.contains("class Status(str, Enum):"));
    assert!(output.contains("ACTIVE = \"active\""));
    assert!(output.contains("INACTIVE = \"inactive\""));
    assert!(output.contains("PENDING = \"pending\""));

    // Check field uses enum
    assert!(output.contains("status: Optional[Status] = Field(None"));
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

    let output = generate_pydantic(schema).await;

    // Check both classes are generated
    assert!(output.contains("class NamedEntity(BaseModel):"));
    assert!(output.contains("class Person(NamedEntity):"));

    // Person inherits from NamedEntity, not BaseModel
    assert!(!output.contains("class Person(BaseModel):"));
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

    let generator = PydanticGenerator::new();
    let options = GeneratorOptions::new();

    let output = generator.generate(&schema).expect("Test operation failed");

    // Check pattern validation in Field
    assert!(output.contains(r#"pattern=r"^[\w\.-]+@[\w\.-]+\.\w+$""#));
    assert!(output.contains(r#"pattern=r"^\+?[\d\s\-()]+$""#));
}

#[tokio::test]
async fn test_datetime_types() {
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
            "datetime".to_string(),
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
            range: Some("date".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "datetime".to_string(),
        SlotDefinition {
            name: "datetime".to_string(),
            range: Some("datetime".to_string()),
            ..Default::default()
        },
    );

    let output = generate_pydantic(schema).await;

    // Check datetime imports - the import manager combines them differently
    assert!(output.contains("from datetime import"));
    assert!(output.contains("datetime"));

    // Check field types
    assert!(output.contains("date: Optional[datetime.date]"));
    assert!(output.contains("datetime: Optional[datetime.datetime]"));
}
