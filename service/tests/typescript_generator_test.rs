//! Tests for the TypeScript generator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, PermissibleValue, SlotDefinition};
use linkml_service::generator::{Generator, TypeScriptGenerator};
use serde_json::json;
fn generate_typescript(schema: SchemaDefinition) -> String {
    let generator = TypeScriptGenerator::new();

    generator.generate(&schema).expect("Test operation failed")
}

#[tokio::test]
async fn test_simple_interface_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        description: Some("Test schema for TypeScript generation".to_string()),
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

    let output = generate_typescript(schema).await;

    // Check header
    assert!(output.contains("Generated from LinkML schema"));
    assert!(output.contains("Test schema for TypeScript generation"));

    // Check validation types
    assert!(output.contains("export interface ValidationError"));
    assert!(output.contains("export type ValidationResult<T>"));

    // Check interface definition
    assert!(output.contains("export interface Person {"));
    assert!(output.contains("Represents a person"));
    assert!(output.contains("@generated from LinkML schema"));

    // Check fields with JSDoc
    assert!(output.contains("/**
   * Unique identifier
   */
  id: string;"));
    assert!(output.contains("/**
   * Person's full name
   */
  name: string;"));
    assert!(output.contains("/**
   * Age in years
   */
  age?: number;"));

    // Check type guard
    assert!(output.contains("export function isPerson(obj: unknown): obj is Person"));
    assert!(output.contains("typeof obj === 'object'"));
    assert!(output.contains("'id' in obj"));
    assert!(output.contains("typeof (obj as any).id === 'string'"));

    // Check validator
    assert!(
        output.contains("export function validatePerson(obj: unknown): ValidationResult<Person>")
    );
    assert!(output.contains("const errors: ValidationError[] = []"));
    assert!(output.contains("if (obj.age !== undefined && obj.age < 0)"));
    assert!(output.contains("if (obj.age !== undefined && obj.age > 150)"));
}

#[tokio::test]
async fn test_array_fields() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_arrays".to_string(),
        ..Default::default()
    };

    let group = ClassDefinition {
        name: "Group".to_string(),
        description: Some("A group of items".to_string()),
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

    let output = generate_typescript(schema).await;

    // Check array types
    assert!(output.contains("members: string[];"));
    assert!(output.contains("tags?: string[];"));
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
                PermissibleValue::Simple("pending".to_string()),
            ],
            ..Default::default()
        },
    );

    let output = generate_typescript(schema).await;

    // Check enum generation
    assert!(output.contains("export enum Status {"));
    assert!(output.contains("ACTIVE = \"active\","));
    assert!(output.contains("INACTIVE = \"inactive\","));
    assert!(output.contains("PENDING = \"pending\""));

    // Check field uses enum
    assert!(output.contains("status?: Status;"));
}

#[tokio::test]
async fn test_inheritance() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_inheritance".to_string(),
        ..Default::default()
    };

    // Base interface
    let entity = ClassDefinition {
        name: "NamedEntity".to_string(),
        description: Some("Base entity with name".to_string()),
        slots: vec!["id".to_string(), "name".to_string()],
        ..Default::default()
    };

    // Derived interface
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

    let output = generate_typescript(schema).await;

    // Check both interfaces
    assert!(output.contains("export interface NamedEntity {"));
    assert!(output.contains("export interface Person extends NamedEntity {"));

    // Check Person only has its own fields (not inherited ones)
    let person_section = output
        .split("export interface Person")
        .nth(1)
        .expect("Test operation failed");
    let person_end = person_section.find("}").expect("Test operation failed");
    let person_body = &person_section[..person_end];

    // Should have age and email but not id and name
    assert!(person_body.contains("age?: number;"));
    assert!(person_body.contains("email?: string;"));
    assert!(!person_body.contains("id:"));
    assert!(!person_body.contains("name:"));
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

    let output = generate_typescript(schema).await;

    // Check pattern validation in validator
    assert!(output.contains("export function validateContact"));
    assert!(output.contains(r"if (obj.email && !/^[\w\.-]+@[\w\.-]+\.\w+$/u.test(obj.email))"));
    assert!(output.contains(r"if (obj.phone && !/^\+?[\d\s\-()]+$/u.test(obj.phone))"));
    assert!(output.contains("Does not match pattern"));
}

#[tokio::test]
async fn test_type_guard_without_validators() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_guard".to_string(),
        ..Default::default()
    };

    let simple = ClassDefinition {
        name: "SimpleType".to_string(),
        slots: vec!["value".to_string()],
        ..Default::default()
    };

    schema.classes.insert("SimpleType".to_string(), simple);

    schema.slots.insert(
        "value".to_string(),
        SlotDefinition {
            name: "value".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    let generator = TypeScriptGenerator::new();
    let options = GeneratorOptions::new()
        .set_custom("generate_validators", "false")
        .set_custom("generate_type_guards", "true");

    let outputs = generator.generate(&schema).expect("Test operation failed");
    let output = &outputs[0].content;

    // Should have type guard but no validator
    assert!(output.contains("export function isSimpleType"));
    assert!(!output.contains("export function validateSimpleType"));
    assert!(!output.contains("ValidationError"));
    assert!(!output.contains("ValidationResult"));
}
