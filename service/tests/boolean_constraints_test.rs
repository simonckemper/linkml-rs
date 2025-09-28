//! Integration tests for boolean constraint validators

use linkml_core::types::{
    AnonymousSlotExpression, ClassDefinition, SchemaDefinition, SlotDefinition,
};
use linkml_service::validator::ValidationOptions;
use serde_json::json;
#[tokio::test]
async fn test_parse_any_of_constraint() {
    // Test that any_of constraints are properly parsed from YAML
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    any_of:
      - range: string
      - range: integer
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    assert!(slot.any_of.is_some());
    let constraints = slot.any_of.as_ref().expect("Test operation failed");
    assert_eq!(constraints.len(), 2);
    assert_eq!(constraints[0].range, Some("string".to_string()));
    assert_eq!(constraints[1].range, Some("integer".to_string()));
}

#[tokio::test]
async fn test_parse_all_of_constraint() {
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    all_of:
      - minimum_value: 0
        maximum_value: 100
      - pattern: "^\\d+$"
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    assert!(slot.all_of.is_some());
    let constraints = slot.all_of.as_ref().expect("Test operation failed");
    assert_eq!(constraints.len(), 2);
    assert_eq!(constraints[0].minimum_value, Some(json!(0)));
    assert_eq!(constraints[0].maximum_value, Some(json!(100)));
    assert_eq!(constraints[1].pattern, Some("^\\d+$".to_string()));
}

#[tokio::test]
async fn test_parse_exactly_one_of_constraint() {
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    exactly_one_of:
      - range: boolean
      - range: integer
        minimum_value: 0
        maximum_value: 1
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    assert!(slot.exactly_one_of.is_some());
    let constraints = slot.exactly_one_of.as_ref().expect("Test operation failed");
    assert_eq!(constraints.len(), 2);
    assert_eq!(constraints[0].range, Some("boolean".to_string()));
    assert_eq!(constraints[1].range, Some("integer".to_string()));
    assert_eq!(constraints[1].minimum_value, Some(json!(0)));
    assert_eq!(constraints[1].maximum_value, Some(json!(1)));
}

#[tokio::test]
async fn test_parse_none_of_constraint() {
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    none_of:
      - pattern: "^test"
      - pattern: "^demo"
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    assert!(slot.none_of.is_some());
    let constraints = slot.none_of.as_ref().expect("Test operation failed");
    assert_eq!(constraints.len(), 2);
    assert_eq!(constraints[0].pattern, Some("^test".to_string()));
    assert_eq!(constraints[1].pattern, Some("^demo".to_string()));
}

#[tokio::test]
async fn test_nested_boolean_constraints() {
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    any_of:
      - all_of:
          - range: integer
          - minimum_value: 0
      - all_of:
          - range: string
          - pattern: "^[A-Z]"
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    assert!(slot.any_of.is_some());
    let any_constraints = slot.any_of.as_ref().expect("Test operation failed");
    assert_eq!(any_constraints.len(), 2);

    // First any_of constraint has all_of
    assert!(any_constraints[0].all_of.is_some());
    let all_constraints_1 = any_constraints[0]
        .all_of
        .as_ref()
        .expect("Test operation failed");
    assert_eq!(all_constraints_1.len(), 2);
    assert_eq!(all_constraints_1[0].range, Some("integer".to_string()));
    assert_eq!(all_constraints_1[1].minimum_value, Some(json!(0)));

    // Second any_of constraint has all_of
    assert!(any_constraints[1].all_of.is_some());
    let all_constraints_2 = any_constraints[1]
        .all_of
        .as_ref()
        .expect("Test operation failed");
    assert_eq!(all_constraints_2.len(), 2);
    assert_eq!(all_constraints_2[0].range, Some("string".to_string()));
    assert_eq!(all_constraints_2[1].pattern, Some("^[A-Z]".to_string()));
}

#[tokio::test]
async fn test_validation_with_boolean_constraints() {
    // This test verifies that validation runs with boolean constraints
    // Even though we have placeholder implementations, it should not crash
    let schema = SchemaDefinition {
        id: "https://example.org/test".to_string(),
        name: "test_schema".to_string(),
        slots: [(
            "test_slot".to_string(),
            SlotDefinition {
                name: "test_slot".to_string(),
                any_of: Some(vec![
                    AnonymousSlotExpression {
                        range: Some("string".to_string()),
                        ..Default::default()
                    },
                    AnonymousSlotExpression {
                        range: Some("integer".to_string()),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
        )]
        .into_iter()
        .collect(),
        ..Default::default()
    };

    // Add a class to the schema so validation knows what to validate
    let mut schema_with_class = schema;
    schema_with_class.classes.insert(
        "TestClass".to_string(),
        linkml_core::types::ClassDefinition {
            name: "TestClass".to_string(),
            slots: vec!["test_slot".to_string()],
            ..Default::default()
        },
    );

    let data = json!({
        "test_slot": "hello"
    });

    let options = ValidationOptions::default();
    let report = linkml_service::validator::validate_as_class(
        &schema_with_class,
        &data,
        "TestClass",
        Some(options),
    )
    .await
    .expect("Test operation failed");

    // Should have warnings from placeholder implementation
    assert!(!report.issues.is_empty());
    let constraint_issues: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.code == Some("BOOLEAN_CONSTRAINT_PLACEHOLDER".to_string()))
        .collect();
    assert!(!constraint_issues.is_empty());
}

#[tokio::test]
async fn test_mixed_constraints() {
    // Test that regular constraints work alongside boolean constraints
    let yaml = r#"
id: https://example.org/test
name: test_schema
classes:
  TestClass:
    name: TestClass
    slots:
      - test_slot
slots:
  test_slot:
    name: test_slot
    required: true
    range: string
    pattern: "^[A-Z]"
    any_of:
      - minimum_value: 5
      - maximum_value: 10
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");

    // Verify all constraints are present
    assert_eq!(slot.required, Some(true));
    assert_eq!(slot.range, Some("string".to_string()));
    assert_eq!(slot.pattern, Some("^[A-Z]".to_string()));
    assert!(slot.any_of.is_some());
    assert_eq!(
        slot.any_of.as_ref().expect("Test operation failed").len(),
        2
    );
}

#[tokio::test]
async fn test_anonymous_slot_expression_all_fields() {
    // Test that all fields in AnonymousSlotExpression can be parsed
    let yaml = r#"
id: https://example.org/test
name: test_schema
slots:
  test_slot:
    name: test_slot
    any_of:
      - range: string
        pattern: "^test"
        minimum_value: 5
        maximum_value: 10
        minimum_cardinality: 1
        maximum_cardinality: 5
        required: true
        recommended: false
        multivalued: true
        permissible_values:
          - "option1"
          - "option2"
        inlined: true
        inlined_as_list: false
"#;

    let parser = linkml_service::parser::YamlParser::new();
    let schema = linkml_service::parser::SchemaParser::parse_str(&parser, yaml)
        .expect("Test operation failed");

    let slot = schema
        .slots
        .get("test_slot")
        .expect("Test operation failed");
    let constraints = slot.any_of.as_ref().expect("Test operation failed");
    let expr = &constraints[0];

    assert_eq!(expr.range, Some("string".to_string()));
    assert_eq!(expr.pattern, Some("^test".to_string()));
    assert_eq!(expr.minimum_value, Some(json!(5)));
    assert_eq!(expr.maximum_value, Some(json!(10)));
    assert_eq!(expr.minimum_cardinality, Some(1));
    assert_eq!(expr.maximum_cardinality, Some(5));
    assert_eq!(expr.required, Some(true));
    assert_eq!(expr.recommended, Some(false));
    assert_eq!(expr.multivalued, Some(true));
    assert_eq!(expr.permissible_values.len(), 2);
    assert_eq!(expr.inlined, Some(true));
    assert_eq!(expr.inlined_as_list, Some(false));
}
