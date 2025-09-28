//! Tests for TypeQL constraint generation

use linkml_core::Value;
use linkml_core::prelude::*;
use linkml_service::generator::typeql_constraints::TypeQLConstraintTranslator;
use linkml_core::types::{SlotDefinition, UniqueKeyDefinition};

#[test]
fn test_key_constraint_generation() {
    let mut translator = TypeQLConstraintTranslator::new();

    // Test identifier slot generates @key
    let mut slot = SlotDefinition::default();
    slot.identifier = Some(true);

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.contains(&"@key".to_string());
    assert!(!constraints.contains(&"@unique".to_string())); // key implies unique
}

#[test]
fn test_unique_constraint_detection() {
    let mut translator = TypeQLConstraintTranslator::new();

    // Test unique detection by name pattern
    let mut slot = SlotDefinition::default();
    slot.name = "user_id".to_string();
    slot.identifier = Some(false);

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.contains(&"@unique".to_string());

    // Test unique detection by description
    let mut slot = SlotDefinition::default();
    slot.name = "email".to_string();
    slot.description = Some("Unique email address for the user".to_string());

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.contains(&"@unique".to_string());
}

#[test]
fn test_cardinality_constraints() {
    let mut translator = TypeQLConstraintTranslator::new();

    // Test default single-valued optional (no constraint needed)
    let mut slot = SlotDefinition::default();
    slot.required = Some(false);
    slot.multivalued = Some(false);

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(!constraints.iter().any(|c| c.starts_with("@card"));

    // Test multi-valued optional
    let mut slot = SlotDefinition::default();
    slot.required = Some(false);
    slot.multivalued = Some(true);

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.contains(&"@card(0..)".to_string());

    // Test multi-valued required (unbounded since maximum_cardinality not available)
    let mut slot = SlotDefinition::default();
    slot.required = Some(true);
    slot.multivalued = Some(true);

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.contains(&"@card(1..)".to_string());
}

#[test]
fn test_regex_pattern_constraints() {
    let mut translator = TypeQLConstraintTranslator::new();

    // Test simple regex pattern
    let mut slot = SlotDefinition::default();
    slot.pattern = Some(r"^\w+@\w+\.\w+$".to_string());

    let constraints = translator.translate_slot_constraints(&slot);
    assert!(constraints.iter().any(|c| c.starts_with("regex"));

    // Test pattern with quotes that need escaping
    let mut slot = SlotDefinition::default();
    slot.pattern = Some(r#"^"[A-Z]+"$"#.to_string());

    let constraints = translator.translate_slot_constraints(&slot);
    let regex_constraint = constraints
        .iter()
        .find(|c| c.starts_with("regex"))
        .expect("Test operation failed");
    assert!(regex_constraint.contains(r#"\""#));
}

#[test]
fn test_range_constraints() {
    let translator = TypeQLConstraintTranslator::new();

    // Test integer range
    let mut slot = SlotDefinition::default();
    slot.minimum_value = Some(Value::Number(serde_json::Number::from(0));
    slot.maximum_value = Some(Value::Number(serde_json::Number::from(100));

    let constraints = translator.translate_range_constraints(&slot);
    assert!(constraints.contains(&"range [0..100]".to_string());

    // Test float range with only minimum
    let mut slot = SlotDefinition::default();
    slot.minimum_value = Some(Value::Number(
        serde_json::Number::from_f64(0.0).expect("Test operation failed"),
    ));

    let constraints = translator.translate_range_constraints(&slot);
    assert!(constraints.contains(&"range [0..)".to_string());

    // Test range with only maximum
    let mut slot = SlotDefinition::default();
    slot.maximum_value = Some(Value::Number(serde_json::Number::from(999));

    let constraints = translator.translate_range_constraints(&slot);
    assert!(constraints.contains(&"range (..999]".to_string());
}

#[test]
fn test_composite_unique_rule_generation() {
    let translator = TypeQLConstraintTranslator::new();

    let mut unique_key = UniqueKeyDefinition::default();
    // unique_key_name field doesn't exist in current API
    unique_key.unique_key_slots = vec!["code".to_string(), "version".to_string()];

    let converter = |s: &str| s.to_lowercase().replace('_', "-");

    let rule = translator.generate_composite_unique_rule("Product", &unique_key, &converter);

    // Verify rule structure
    assert!(rule.contains("rule product-unique-product-key:"));
    assert!(rule.contains("$x isa product;"));
    assert!(rule.contains("$y isa product;"));
    assert!(rule.contains("not { $x is $y; };"));
    assert!(rule.contains("$x has code $v0;"));
    assert!(rule.contains("$y has code $v0;"));
    assert!(rule.contains("$x has version $v1;"));
    assert!(rule.contains("$y has version $v1;"));
    assert!(rule.contains("validation-error \"Duplicate values for unique key: code, version\""));
}

#[test]
fn test_complex_constraint_combinations() {
    let mut translator = TypeQLConstraintTranslator::new();

    // Test a slot with multiple constraints
    let mut slot = SlotDefinition::default();
    slot.name = "user_code".to_string();
    slot.identifier = Some(true);
    slot.pattern = Some(r"^[A-Z]{3}\d{3}$".to_string());
    slot.required = Some(true);

    let constraints = translator.translate_slot_constraints(&slot);

    // Should have @key
    assert!(constraints.contains(&"@key".to_string());

    // Should have regex
    assert!(constraints.iter().any(|c| c.starts_with("regex"));

    // Should not have cardinality (1..1 is default for required single-valued)
    assert!(!constraints.iter().any(|c| c.starts_with("@card"));
}

#[test]
fn test_validation_rule_generation() {
    let translator = TypeQLConstraintTranslator::new();

    // Test enum validation
    let mut slot = SlotDefinition::default();
    slot.name = "status".to_string();
    slot.range = Some("StatusEnum".to_string());

    let rule = translator.generate_validation_rule("Entity", "status", &slot);
    assert!(rule.is_some());
    assert!(rule.expect("Test operation failed").contains("StatusEnum"));

    // Test complex pattern
    let mut slot = SlotDefinition::default();
    slot.name = "identifier".to_string();
    slot.pattern = Some(r"(?i)^[a-z]+|[0-9]+$".to_string());

    let rule = translator.generate_validation_rule("Entity", "identifier", &slot);
    assert!(rule.is_some());
    assert!(
        rule.expect("Test operation failed")
            .contains("Complex pattern")
    );
}

#[test]
fn test_common_identifier_patterns() {
    let translator = TypeQLConstraintTranslator::new();

    let test_cases = vec![
        ("isbn", true),
        ("doi", true),
        ("urn", true),
        ("person_uuid", true),
        ("order_code", true),
        ("name", false),
        ("description", false),
        ("color", false),
    ];

    for (name, should_be_unique) in test_cases {
        let mut slot = SlotDefinition::default();
        slot.name = name.to_string();

        // is_unique_constraint is a private method, test the actual constraints instead
        let constraints = translator.translate_slot_constraints(&slot);
        let is_unique = constraints.contains(&"@unique".to_string());
        assert_eq!(
            is_unique, should_be_unique,
            "Failed for {}: expected {}, got {}",
            name, should_be_unique, is_unique
        );
    }
}
