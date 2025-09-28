//! Tests for the JavaScript ES6 generator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, PermissibleValue, SlotDefinition};
use linkml_service::generator::{Generator, GeneratorOptions, JavaScriptGenerator};
use serde_json::json;
async fn generate_javascript(schema: SchemaDefinition) -> String {
    let generator = JavaScriptGenerator::new();
    let options = GeneratorOptions::new().with_docs(true);

    generator.generate(&schema).expect("Test operation failed")
}

#[tokio::test]
async fn test_simple_class_generation() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        description: Some("Test schema for JavaScript generation".to_string()),
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

    let output = generate_javascript(schema).await;

    // Check header
    assert!(output.contains("Generated from LinkML schema"));
    assert!(output.contains("'use strict';"));

    // Check class definition
    assert!(output.contains("export class Person {"));
    assert!(output.contains("Represents a person"));
    assert!(output.contains("@generated from LinkML schema"));

    // Check constructor JSDoc
    assert!(output.contains("@param {Object} data - Initialization data"));
    assert!(output.contains("@param {string} data.id - Unique identifier"));
    assert!(output.contains("@param {string} data.name - Person's full name"));
    assert!(output.contains("@param {[number]} [data.age] - Age in years"));

    // Check constructor
    assert!(output.contains("constructor(data = {})"));
    assert!(output.contains("this.#validate(data);"));
    assert!(output.contains("this.id = data.id;"));
    assert!(output.contains("this.name = data.name;"));
    assert!(output.contains("this.age = data.age || null;"));

    // Check validation
    assert!(output.contains("#validate(data)"));
    assert!(output.contains("if (!data.id || typeof data.id !== 'string')"));
    assert!(output.contains("throw new TypeError('id must be a non-empty string')"));
    assert!(output.contains("if (typeof data.age === 'number' && data.age < 0)"));
    assert!(output.contains("if (typeof data.age === 'number' && data.age > 150)"));

    // Check static methods
    assert!(output.contains("static fromJSON(json)"));
    assert!(output.contains("return new Person(JSON.parse(json));"));

    // Check instance methods
    assert!(output.contains("toObject()"));
    assert!(output.contains("toJSON()"));
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
            range: Some("string".to_string()),
            multivalued: Some(true),
            ..Default::default()
        },
    );

    let output = generate_javascript(schema).await;

    // Check array initialization
    assert!(output.contains("this.members = data.members || [];"));

    // Check array validation
    assert!(output.contains("if (data.members && !Array.isArray(data.members))"));
    assert!(output.contains("throw new TypeError('members must be an array')"));

    // Check toObject spreads array
    assert!(output.contains("members: [...this.members]"));
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

    let output = generate_javascript(schema).await;

    // Check enum generation
    assert!(output.contains("export const Status = Object.freeze({"));
    assert!(output.contains("ACTIVE: \"active\","));
    assert!(output.contains("INACTIVE: \"inactive\","));
    assert!(output.contains("PENDING: \"pending\","));
    assert!(output.contains("});"));

    // Check JSDoc
    assert!(output.contains("@readonly"));
    assert!(output.contains("@enum {string}"));
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
        description: Some("Base entity".to_string()),
        slots: vec!["id".to_string(), "name".to_string()],
        ..Default::default()
    };

    // Derived class
    let person = ClassDefinition {
        name: "Person".to_string(),
        description: Some("Person extends NamedEntity".to_string()),
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

    let output = generate_javascript(schema).await;

    // Check inheritance
    assert!(output.contains("export class Person extends NamedEntity"));
    assert!(output.contains("super(data);"));

    // Check Person only initializes its own fields
    assert!(output.contains("this.age = data.age || null;"));

    // Check toObject includes parent data
    assert!(output.contains("const parentData = super.toObject();"));
    assert!(output.contains("...parentData,"));
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

    let output = generate_javascript(schema).await;

    // Check pattern validation
    assert!(output.contains(r"if (data.email && !/^[\w\.-]+@[\w\.-]+\.\w+$/u.test(data.email))"));
    assert!(output.contains(r"if (data.phone && !/^\+?[\d\s\-()]+$/u.test(data.phone))"));
    assert!(output.contains("does not match pattern"));
}

#[tokio::test]
async fn test_commonjs_export() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_commonjs".to_string(),
        ..Default::default()
    };

    let simple = ClassDefinition {
        name: "SimpleClass".to_string(),
        slots: vec!["value".to_string()],
        ..Default::default()
    };

    schema.classes.insert("SimpleClass".to_string(), simple);

    schema.slots.insert(
        "value".to_string(),
        SlotDefinition {
            name: "value".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    let generator = JavaScriptGenerator::new();
    let options = GeneratorOptions::new().set_custom("module_type", "commonjs");

    let output = generator.generate(&schema).expect("Test operation failed");

    // Check CommonJS exports
    assert!(output.contains("// CommonJS exports"));
    assert!(output.contains("if (typeof module !== 'undefined' && module.exports)"));
    assert!(output.contains("module.exports.SimpleClass = SimpleClass;"));
}

#[tokio::test]
async fn test_esm_module() {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_esm".to_string(),
        ..Default::default()
    };

    let simple = ClassDefinition {
        name: "SimpleClass".to_string(),
        slots: vec!["value".to_string()],
        ..Default::default()
    };

    schema.classes.insert("SimpleClass".to_string(), simple);

    schema.slots.insert(
        "value".to_string(),
        SlotDefinition {
            name: "value".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    let generator = JavaScriptGenerator::new();
    let options = GeneratorOptions::new(); // Default is ESM

    let output = generator.generate(&schema).expect("Test operation failed");

    // Should not have CommonJS exports
    assert!(!output.contains("module.exports"));
}
