//! Tests for annotation support

use linkml_core::{
    annotations::{Annotatable, AnnotationValue, Annotations},
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::parser::{SchemaParser, YamlParser};
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};

#[test]
fn test_annotations_api() {
    let mut schema = SchemaDefinition::new("test_schema");

    // Initially no annotations
    assert!(schema.annotations().is_none());

    // Add annotations
    schema.annotations = Some(Annotations::new());
    schema.set_annotation("author", "John Doe".into());
    schema.set_annotation("version", 2.into());
    schema.set_annotation("deprecated", false.into());

    // Check annotations
    assert!(schema.has_annotation("author"));
    assert_eq!(
        schema.get_annotation("author"),
        Some(&AnnotationValue::String("John Doe".to_string()))
    );

    // Update annotation
    schema.set_annotation("version", 3.into());
    assert_eq!(
        schema.get_annotation("version"),
        Some(&AnnotationValue::Number(3.into()))
    );

    // Remove annotation
    let removed = schema.remove_annotation("deprecated");
    assert_eq!(removed, Some(AnnotationValue::Bool(false)));
    assert!(!schema.has_annotation("deprecated"));
}

#[test]
fn test_class_annotations() {
    let mut class = ClassDefinition::new("Person");

    // Add annotations
    class.annotations = Some(Annotations::new());
    class.set_annotation("ui:hidden", false.into());
    class.set_annotation("db:table", "persons".into());

    assert_eq!(
        class.get_annotation("db:table"),
        Some(&AnnotationValue::String("persons".to_string()))
    );
}

#[test]
fn test_slot_annotations() {
    let mut slot = SlotDefinition::new("email");

    // Add annotations
    slot.annotations = Some(Annotations::new());
    slot.set_annotation("pattern:type", "email".into());
    slot.set_annotation("ui:widget", "email-input".into());

    assert!(slot.has_annotation("pattern:type"));
    assert!(slot.has_annotation("ui:widget"));
}

#[test]
fn test_yaml_parsing_with_annotations() {
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
annotations:
  author: Jane Smith
  version: 1.0.0
  tags:
    - experimental
    - internal
classes:
  Person:
    name: Person
    annotations:
      db:table: persons
      ui:icon: user
    slots:
      - name
      - email
slots:
  name:
    name: name
    range: string
    annotations:
      ui:placeholder: Enter your full name
  email:
    name: email
    range: string
    annotations:
      pattern:type: email
      ui:widget: email-input
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Test operation failed");

    // Check schema annotations
    assert!(schema.annotations.is_some());
    assert_eq!(
        schema.get_annotation("author"),
        Some(&AnnotationValue::String("Jane Smith".to_string()))
    );
    assert_eq!(
        schema.get_annotation("version"),
        Some(&AnnotationValue::String("1.0.0".to_string()))
    );

    // Check array annotation
    if let Some(AnnotationValue::Array(tags)) = schema.get_annotation("tags") {
        assert_eq!(tags.len(), 2);
        assert_eq!(tags[0], AnnotationValue::String("experimental".to_string());
        assert_eq!(tags[1], AnnotationValue::String("internal".to_string());
    } else {
        panic!("Expected array annotation");
    }

    // Check class annotations
    let person_class = schema.classes.get("Person").expect("Test operation failed");
    assert_eq!(
        person_class.get_annotation("db:table"),
        Some(&AnnotationValue::String("persons".to_string()))
    );
    assert_eq!(
        person_class.get_annotation("ui:icon"),
        Some(&AnnotationValue::String("user".to_string()))
    );

    // Check slot annotations
    let email_slot = schema.slots.get("email").expect("Test operation failed");
    assert_eq!(
        email_slot.get_annotation("pattern:type"),
        Some(&AnnotationValue::String("email".to_string()))
    );
    assert_eq!(
        email_slot.get_annotation("ui:widget"),
        Some(&AnnotationValue::String("email-input".to_string()))
    );
}

#[test]
fn test_complex_annotation_values() {
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
classes:
  Dataset:
    name: Dataset
    annotations:
      metadata:
        created_by: "Data Team"
        created_date: "2024-01-01"
        tags:
          - research
          - public
        config:
          export_enabled: true
          max_size: 1000000
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Test operation failed");

    let dataset_class = schema
        .classes
        .get("Dataset")
        .expect("Test operation failed");

    // Check nested object annotation
    if let Some(AnnotationValue::Object(metadata)) = dataset_class.get_annotation("metadata") {
        assert_eq!(
            metadata.get("created_by"),
            Some(&AnnotationValue::String("Data Team".to_string()))
        );

        // Check nested array
        if let Some(AnnotationValue::Array(tags)) = metadata.get("tags") {
            assert_eq!(tags.len(), 2);
        } else {
            panic!("Expected tags array");
        }

        // Check nested object
        if let Some(AnnotationValue::Object(config)) = metadata.get("config") {
            assert_eq!(
                config.get("export_enabled"),
                Some(&AnnotationValue::Bool(true))
            );
            assert_eq!(
                config.get("max_size"),
                Some(&AnnotationValue::Number(1000000.into()))
            );
        } else {
            panic!("Expected config object");
        }
    } else {
        panic!("Expected metadata object");
    }
}

#[test]
fn test_annotation_serialization() {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.annotations = Some(Annotations::new());
    schema.set_annotation("version", "1.0.0".into());
    schema.set_annotation("experimental", true.into());

    let json = serde_json::to_string_pretty(&schema).expect("Test operation failed");
    assert!(json.contains(r#""version": "1.0.0""#));
    assert!(json.contains(r#""experimental": true"#));

    // Deserialize back
    let parsed: SchemaDefinition = serde_json::from_str(&json).expect("Test operation failed");
    assert_eq!(
        parsed.get_annotation("version"),
        Some(&AnnotationValue::String("1.0.0".to_string()))
    );
}
