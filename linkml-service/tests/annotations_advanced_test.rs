//! Advanced tests for annotation inheritance, merging, and edge cases

use linkml_core::{
    annotations::{Annotatable, Annotation, AnnotationValue, Annotations},
    types::{ClassDefinition, SchemaDefinition, SlotDefinition, TypeDefinition, EnumDefinition},
};
use linkml_service::transform::InheritanceResolver;
use linkml_service::parser::{SchemaParser, YamlParser};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_annotation_inheritance_in_slots() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    // Base slot with annotations
    let mut base_slot = SlotDefinition::new("base_slot");
    base_slot.annotations = Some(Annotations::new());
    base_slot.set_annotation("ui:widget", "text".into());
    base_slot.set_annotation("db:indexed", true.into());
    base_slot.set_annotation("validation:pattern", "[A-Z]+".into());
    schema.slots.insert("base_slot".to_string(), base_slot);
    
    // Class that uses slot with slot_usage override
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string()];
    
    // Slot usage that inherits from base_slot
    let mut name_usage = SlotDefinition::new("name");
    name_usage.is_a = Some("base_slot".to_string());
    name_usage.annotations = Some(Annotations::new());
    name_usage.set_annotation("ui:widget", "textarea".into()); // Override
    name_usage.set_annotation("ui:rows", 5.into()); // New annotation
    
    person_class.slot_usage.insert("name".to_string(), name_usage);
    schema.classes.insert("Person".to_string(), person_class);
    
    // Resolve inheritance
    let mut resolver = InheritanceResolver::new(&schema);
    let resolved = resolver.resolve().unwrap();
    
    // Check that annotations were properly merged
    let resolved_class = resolved.classes.get("Person").unwrap();
    let name_usage = resolved_class.slot_usage.get("name").unwrap();
    
    // Should have overridden annotation
    assert_eq!(
        name_usage.get_annotation("ui:widget"),
        Some(&AnnotationValue::String("textarea".to_string()))
    );
    
    // Should have inherited annotation
    assert_eq!(
        name_usage.get_annotation("db:indexed"),
        Some(&AnnotationValue::Bool(true))
    );
    
    // Should have new annotation
    assert_eq!(
        name_usage.get_annotation("ui:rows"),
        Some(&AnnotationValue::Number(5.into()))
    );
}

#[test]
fn test_complex_annotation_values() {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.annotations = Some(Annotations::new());
    
    // Test array annotation
    let array_value = AnnotationValue::Array(vec![
        AnnotationValue::String("option1".to_string()),
        AnnotationValue::String("option2".to_string()),
        AnnotationValue::Number(42.into()),
    ]);
    schema.set_annotation("allowed_values", array_value);
    
    // Test nested object annotation
    let mut nested_map = HashMap::new();
    nested_map.insert("host".to_string(), AnnotationValue::String("localhost".to_string()));
    nested_map.insert("port".to_string(), AnnotationValue::Number(5432.into()));
    nested_map.insert("ssl".to_string(), AnnotationValue::Bool(true));
    
    let object_value = AnnotationValue::Object(nested_map);
    schema.set_annotation("database", object_value);
    
    // Test deeply nested annotation
    let mut deep_map = HashMap::new();
    let mut inner_map = HashMap::new();
    inner_map.insert("level".to_string(), AnnotationValue::String("debug".to_string()));
    inner_map.insert("format".to_string(), AnnotationValue::String("json".to_string()));
    deep_map.insert("logging".to_string(), AnnotationValue::Object(inner_map));
    
    schema.set_annotation("config", AnnotationValue::Object(deep_map));
    
    // Verify complex values
    if let Some(AnnotationValue::Array(arr)) = schema.get_annotation("allowed_values") {
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], AnnotationValue::String("option1".to_string()));
    } else {
        panic!("Expected array annotation");
    }
    
    if let Some(AnnotationValue::Object(obj)) = schema.get_annotation("database") {
        assert_eq!(obj.get("port"), Some(&AnnotationValue::Number(5432.into())));
    } else {
        panic!("Expected object annotation");
    }
}

#[test]
fn test_annotation_on_all_element_types() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    // Schema annotations
    schema.annotations = Some(Annotations::new());
    schema.set_annotation("schema:version", "1.0".into());
    
    // Class annotations
    let mut class = ClassDefinition::new("TestClass");
    class.annotations = Some(Annotations::new());
    class.set_annotation("class:type", "entity".into());
    schema.classes.insert("TestClass".to_string(), class);
    
    // Slot annotations
    let mut slot = SlotDefinition::new("test_slot");
    slot.annotations = Some(Annotations::new());
    slot.set_annotation("slot:indexed", true.into());
    schema.slots.insert("test_slot".to_string(), slot);
    
    // Type annotations
    let mut type_def = TypeDefinition::new("TestType");
    type_def.annotations = Some(Annotations::new());
    type_def.set_annotation("type:base", "string".into());
    schema.types.insert("TestType".to_string(), type_def);
    
    // Enum annotations
    let mut enum_def = EnumDefinition::new("TestEnum");
    enum_def.annotations = Some(Annotations::new());
    enum_def.set_annotation("enum:source", "database".into());
    schema.enums.insert("TestEnum".to_string(), enum_def);
    
    // Verify all annotations
    assert!(schema.has_annotation("schema:version"));
    assert!(schema.classes.get("TestClass").unwrap().has_annotation("class:type"));
    assert!(schema.slots.get("test_slot").unwrap().has_annotation("slot:indexed"));
    assert!(schema.types.get("TestType").unwrap().has_annotation("type:base"));
    assert!(schema.enums.get("TestEnum").unwrap().has_annotation("enum:source"));
}

#[tokio::test]
async fn test_annotation_yaml_parsing() {
    let yaml = r#"
id: https://example.org/test
name: test_schema
annotations:
  author: John Doe
  version: 2.0
  tags:
    - experimental
    - beta
  metadata:
    created: 2024-01-01
    reviewed: true

classes:
  Person:
    annotations:
      ui:hidden: false
      db:table: persons
      api:endpoints:
        - /api/persons
        - /api/people
    slots:
      - name
      
slots:
  name:
    annotations:
      ui:widget: text
      validation:
        minLength: 2
        maxLength: 100
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(yaml).unwrap();
    
    // Check schema annotations
    assert_eq!(
        schema.get_annotation("author"),
        Some(&AnnotationValue::String("John Doe".to_string()))
    );
    
    if let Some(AnnotationValue::Array(tags)) = schema.get_annotation("tags") {
        assert_eq!(tags.len(), 2);
    } else {
        panic!("Expected tags array");
    }
    
    // Check class annotations
    let person = schema.classes.get("Person").unwrap();
    assert_eq!(
        person.get_annotation("db:table"),
        Some(&AnnotationValue::String("persons".to_string()))
    );
    
    // Check slot annotations  
    let name_slot = schema.slots.get("name").unwrap();
    if let Some(AnnotationValue::Object(validation)) = name_slot.get_annotation("validation") {
        assert_eq!(
            validation.get("minLength"),
            Some(&AnnotationValue::Number(2.into()))
        );
    } else {
        panic!("Expected validation object");
    }
}

#[test]
fn test_annotation_null_and_empty_values() {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.annotations = Some(Annotations::new());
    
    // Test null value
    schema.set_annotation("nullable", AnnotationValue::Null);
    assert_eq!(schema.get_annotation("nullable"), Some(&AnnotationValue::Null));
    
    // Test empty string
    schema.set_annotation("empty_string", "".into());
    assert_eq!(
        schema.get_annotation("empty_string"),
        Some(&AnnotationValue::String("".to_string()))
    );
    
    // Test empty array
    schema.set_annotation("empty_array", AnnotationValue::Array(vec![]));
    if let Some(AnnotationValue::Array(arr)) = schema.get_annotation("empty_array") {
        assert_eq!(arr.len(), 0);
    }
    
    // Test empty object
    schema.set_annotation("empty_object", AnnotationValue::Object(HashMap::new()));
    if let Some(AnnotationValue::Object(obj)) = schema.get_annotation("empty_object") {
        assert_eq!(obj.len(), 0);
    }
}

#[test]
fn test_annotation_serialization_edge_cases() {
    use serde_json;
    
    let mut annotations = Annotations::new();
    
    // Add various types of annotations
    annotations.insert(Annotation {
        tag: "unicode".to_string(),
        value: AnnotationValue::String("Hello 世界 🌍".to_string()),
    });
    
    annotations.insert(Annotation {
        tag: "large_number".to_string(),
        value: AnnotationValue::Number(serde_json::Number::from(i64::MAX)),
    });
    
    annotations.insert(Annotation {
        tag: "float".to_string(),
        value: AnnotationValue::Number(serde_json::Number::from_f64(3.14159).unwrap()),
    });
    
    annotations.insert(Annotation {
        tag: "special_chars".to_string(),
        value: AnnotationValue::String("Line1\nLine2\tTab\"Quote'Apostrophe\\Backslash".to_string()),
    });
    
    // Serialize and deserialize
    let json = serde_json::to_string(&annotations).unwrap();
    let deserialized: Annotations = serde_json::from_str(&json).unwrap();
    
    // Verify all values survived
    assert_eq!(annotations.0.len(), deserialized.0.len());
    for annotation in &annotations.0 {
        assert!(deserialized.0.iter().any(|a| a == annotation));
    }
}

#[test]
fn test_annotation_merging_conflicts() {
    let mut base = Annotations::new();
    base.insert(Annotation {
        tag: "priority".to_string(),
        value: AnnotationValue::Number(1.into()),
    });
    base.insert(Annotation {
        tag: "status".to_string(),
        value: AnnotationValue::String("draft".to_string()),
    });
    
    let mut override_annotations = Annotations::new();
    override_annotations.insert(Annotation {
        tag: "priority".to_string(),
        value: AnnotationValue::Number(2.into()),
    });
    override_annotations.insert(Annotation {
        tag: "reviewed".to_string(),
        value: AnnotationValue::Bool(true),
    });
    
    // Merge annotations (override should win)
    let mut merged = base.clone();
    for annotation in override_annotations.0 {
        // Remove existing annotation with same tag
        merged.0.retain(|a| a.tag != annotation.tag);
        merged.0.push(annotation);
    }
    
    // Check merged result
    let priority = merged.0.iter().find(|a| a.tag == "priority").unwrap();
    assert_eq!(priority.value, AnnotationValue::Number(2.into()));
    
    let status = merged.0.iter().find(|a| a.tag == "status").unwrap();
    assert_eq!(status.value, AnnotationValue::String("draft".to_string()));
    
    let reviewed = merged.0.iter().find(|a| a.tag == "reviewed").unwrap();
    assert_eq!(reviewed.value, AnnotationValue::Bool(true));
}

#[test]
fn test_annotation_performance_with_many_annotations() {
    use std::time::Instant;
    
    let mut schema = SchemaDefinition::new("test_schema");
    schema.annotations = Some(Annotations::new());
    
    // Add many annotations
    let start = Instant::now();
    for i in 0..1000 {
        schema.set_annotation(&format!("annotation_{}", i), i.into());
    }
    let insert_time = start.elapsed();
    
    // Lookup annotations
    let start = Instant::now();
    for i in 0..1000 {
        let value = schema.get_annotation(&format!("annotation_{}", i));
        assert_eq!(value, Some(&AnnotationValue::Number(i.into())));
    }
    let lookup_time = start.elapsed();
    
    // Performance should be reasonable
    assert!(insert_time.as_millis() < 100); // Less than 100ms for 1000 inserts
    assert!(lookup_time.as_millis() < 50);  // Less than 50ms for 1000 lookups
}

#[test]
fn test_annotation_special_characters_in_tags() {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.annotations = Some(Annotations::new());
    
    // Test various special characters in tags
    schema.set_annotation("namespace:tag", "value1".into());
    schema.set_annotation("tag.with.dots", "value2".into());
    schema.set_annotation("tag-with-dashes", "value3".into());
    schema.set_annotation("tag_with_underscores", "value4".into());
    schema.set_annotation("tag/with/slashes", "value5".into());
    schema.set_annotation("tag@with@at", "value6".into());
    
    // All should be retrievable
    assert!(schema.has_annotation("namespace:tag"));
    assert!(schema.has_annotation("tag.with.dots"));
    assert!(schema.has_annotation("tag-with-dashes"));
    assert!(schema.has_annotation("tag_with_underscores"));
    assert!(schema.has_annotation("tag/with/slashes"));
    assert!(schema.has_annotation("tag@with@at"));
}