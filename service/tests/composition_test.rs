//! Tests for schema composition and inheritance

use linkml_service::parser::Parser;
use linkml_service::validator::{SchemaComposer, validate_as_class};
use serde_json::json;

#[tokio::test]
async fn test_class_inheritance() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for inheritance

classes:
  NamedThing:
    name: NamedThing
    abstract: true
    description: Root class for all named entities
    slots:
      - id
      - name
      - description

  LivingThing:
    name: LivingThing
    is_a: NamedThing
    abstract: true
    description: A living organism
    slots:
      - birth_date
      - death_date

  Person:
    name: Person
    is_a: LivingThing
    tree_root: true
    description: A human being
    slots:
      - age
      - email
    slot_usage:
      name:
        name: name
        required: true
        description: Full name of the person
      email:
        name: email
        pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"

slots:
  id:
    name: id
    range: string
    identifier: true
    required: true

  name:
    name: name
    range: string

  description:
    name: description
    range: string

  birth_date:
    name: birth_date
    range: date

  death_date:
    name: death_date
    range: date

  age:
    name: age
    range: integer
    minimum_value: 0
    maximum_value: 150

  email:
    name: email
    range: string
"#;

    // Parse schema
    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Test schema composition
    let mut composer = SchemaComposer::new(schema.clone());
    let person_resolved = composer
        .resolve_class("Person")
        .expect("Test operation failed");

    // Check all inherited slots
    assert_eq!(person_resolved.effective_slots.len(), 7);
    assert!(person_resolved.effective_slots.contains_key("id"));
    assert!(person_resolved.effective_slots.contains_key("name"));
    assert!(person_resolved.effective_slots.contains_key("description"));
    assert!(person_resolved.effective_slots.contains_key("birth_date"));
    assert!(person_resolved.effective_slots.contains_key("death_date"));
    assert!(person_resolved.effective_slots.contains_key("age"));
    assert!(person_resolved.effective_slots.contains_key("email"));

    // Check ancestors
    assert_eq!(person_resolved.ancestors, vec!["NamedThing", "LivingThing"]);

    // Check slot usage override
    let name_slot = &person_resolved.effective_slots["name"];
    assert_eq!(name_slot.required, Some(true));
    assert_eq!(
        name_slot.description,
        Some("Full name of the person".to_string())
    );

    // Validate data against composed schema
    let valid_person = json!({
        "id": "person-123",
        "name": "John Doe",
        "age": 30,
        "email": "john@example.com"
    });

    let report = validate_as_class(&schema, &valid_person, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Missing required inherited field
    let invalid_person = json!({
        "name": "Jane Doe",
        "age": 25,
        "email": "jane@example.com"
    });

    let report = validate_as_class(&schema, &invalid_person, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.errors().any(|e| e.path.contains("id")));
}

#[tokio::test]
async fn test_mixin_composition() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  Timestamped:
    name: Timestamped
    mixin: true
    description: Mixin for timestamped entities
    slots:
      - created_at
      - updated_at

  Versioned:
    name: Versioned
    mixin: true
    description: Mixin for versioned entities
    slots:
      - version
      - version_date

  Document:
    name: Document
    mixins:
      - Timestamped
      - Versioned
    slots:
      - title
      - content

slots:
  created_at:
    name: created_at
    range: datetime
    required: true

  updated_at:
    name: updated_at
    range: datetime

  version:
    name: version
    range: string
    pattern: "^v\\d+\\.\\d+\\.\\d+$"

  version_date:
    name: version_date
    range: date

  title:
    name: title
    range: string
    required: true

  content:
    name: content
    range: string
"#;

    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Test mixin composition
    let mut composer = SchemaComposer::new(schema.clone());
    let doc_resolved = composer
        .resolve_class("Document")
        .expect("Test operation failed");

    // Check all mixin slots are included
    assert_eq!(doc_resolved.effective_slots.len(), 6);
    assert!(doc_resolved.effective_slots.contains_key("created_at"));
    assert!(doc_resolved.effective_slots.contains_key("updated_at"));
    assert!(doc_resolved.effective_slots.contains_key("version"));
    assert!(doc_resolved.effective_slots.contains_key("version_date"));
    assert!(doc_resolved.effective_slots.contains_key("title"));
    assert!(doc_resolved.effective_slots.contains_key("content"));

    // Check mixins
    assert_eq!(doc_resolved.mixins, vec!["Timestamped", "Versioned"]);

    // Validate data with mixin fields
    let doc_data = json!({
        "title": "Test Document",
        "content": "This is a test",
        "created_at": "2025-01-31T12:00:00Z",
        "version": "v1.0.0"
    });

    let report = validate_as_class(&schema, &doc_data, "Document", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_abstract_class_detection() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  AbstractBase:
    name: AbstractBase
    abstract: true
    slots:
      - id

  ConcreteChild:
    name: ConcreteChild
    is_a: AbstractBase
    slots:
      - name

  AnotherAbstract:
    name: AnotherAbstract
    abstract: true
    is_a: AbstractBase

slots:
  id:
    name: id
    range: string

  name:
    name: name
    range: string
"#;

    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    let mut composer = SchemaComposer::new(schema);

    // Get concrete classes
    let concrete = composer
        .get_concrete_classes()
        .expect("Test operation failed");
    println!("Concrete classes: {:?}", concrete);

    // Only ConcreteChild should be concrete (not abstract)
    assert!(concrete.contains(&"ConcreteChild".to_string()));
    assert!(!concrete.contains(&"AbstractBase".to_string()));
    assert!(!concrete.contains(&"AnotherAbstract".to_string()));

    // Check subclass relationships
    assert!(
        composer
            .is_subclass_of("ConcreteChild", "AbstractBase")
            .expect("Test operation failed")
    );
    assert!(
        composer
            .is_subclass_of("AnotherAbstract", "AbstractBase")
            .expect("Test operation failed")
    );
    assert!(
        !composer
            .is_subclass_of("AbstractBase", "ConcreteChild")
            .expect("Test operation failed")
    );

    // Get all subclasses
    let subclasses = composer
        .get_subclasses("AbstractBase")
        .expect("Test operation failed");
    assert_eq!(subclasses.len(), 2);
    assert!(subclasses.contains(&"ConcreteChild".to_string()));
    assert!(subclasses.contains(&"AnotherAbstract".to_string()));
}

#[tokio::test]
async fn test_tree_root_detection() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  RootClass1:
    name: RootClass1
    tree_root: true
    slots:
      - id

  RootClass2:
    name: RootClass2
    tree_root: true
    slots:
      - id

  NonRoot:
    name: NonRoot
    is_a: RootClass1

slots:
  id:
    name: id
    range: string
"#;

    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("Test operation failed");

    let mut composer = SchemaComposer::new(schema);

    // Get tree roots
    let roots = composer.get_tree_roots().expect("Test operation failed");
    assert_eq!(roots.len(), 2);
    assert!(roots.contains(&"RootClass1".to_string()));
    assert!(roots.contains(&"RootClass2".to_string()));

    // Check that NonRoot inherits from a tree root but is not itself a tree root
    let non_root = composer
        .resolve_class("NonRoot")
        .expect("Test operation failed");
    assert!(!non_root.is_tree_root);
    assert!(
        composer
            .is_subclass_of("NonRoot", "RootClass1")
            .expect("Test operation failed")
    );
}
