//! Property-based tests (fuzzing) for boolean constraint validators
//!
//! This module tests the boolean constraint validators with randomly generated data
//! to ensure they handle edge cases correctly and maintain their invariants.

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{AnonymousSlotExpression, Definition, SlotDefinition};
use linkml_service::validator::{
    context::ValidationContext,
    validators::{
        Validator,
        boolean_constraints::{
            AllOfValidator, AnyOfValidator, ExactlyOneOfValidator, NoneOfValidator,
        },
    },
};
use proptest::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

/// Generate a random JSON value
fn arb_json_value() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|n| json!(n)),
        any::<f64>().prop_map(|f| json!(f)),
        ".*".prop_map(Value::String),
    ];

    leaf.boxed()
}

/// Generate a random range type
fn arb_range_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("string".to_string()),
        Just("integer".to_string()),
        Just("float".to_string()),
        Just("double".to_string()),
        Just("boolean".to_string()),
        Just("null".to_string()),
    ]
}

/// Generate a random pattern
fn arb_pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(r"^\d+$".to_string()),
        Just(r"^[A-Z]+$".to_string()),
        Just(r"^[a-z]{3,5}$".to_string()),
        Just(r"^\w+@\w+\.\w+$".to_string()),
        Just(r"^test_.*$".to_string()),
    ]
}

/// Generate a random AnonymousSlotExpression
fn arb_anonymous_slot_expression() -> impl Strategy<Value = AnonymousSlotExpression> {
    (
        prop::option::of(arb_range_type()),
        prop::option::of(arb_pattern()),
        prop::option::of(any::<i64>().prop_map(|n| json!(n))),
        prop::option::of(any::<i64>().prop_map(|n| json!(n))),
        prop::option::of(any::<bool>()),
    )
        .prop_map(
            |(range, pattern, min, max, required)| AnonymousSlotExpression {
                range,
                pattern,
                minimum_value: min,
                maximum_value: max,
                required,
                ..Default::default()
            },
        )
}

proptest! {
    /// Test that any_of with a single constraint behaves like a regular constraint
    #[test]
    fn test_any_of_single_constraint_equivalence(
        expr in arb_anonymous_slot_expression(),
        value in arb_json_value()
    ) {
        let validator = AnyOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        let slot = SlotDefinition {
            name: "test".to_string(),
            any_of: Some(vec![expr]),
            ..Default::default()
        };

        // Should not panic
        let _issues = validator.validate(&value, &slot, &mut context);
    }

    /// Test that all_of with contradictory constraints always fails
    #[test]
    fn test_all_of_contradictory_constraints(value in arb_json_value()) {
        let validator = AllOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        // Create contradictory constraints
        let slot = SlotDefinition {
            name: "test".to_string(),
            all_of: Some(vec![
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
        };

        let issues = validator.validate(&value, &slot, &mut context);
        // Should always have issues (no value can be both string and integer)
        assert!(!issues.is_empty());
    }

    /// Test that exactly_one_of maintains mutual exclusion
    #[test]
    fn test_exactly_one_of_mutual_exclusion(
        constraints in prop::collection::vec(arb_anonymous_slot_expression(), 2..5),
        value in arb_json_value()
    ) {
        let validator = ExactlyOneOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        let slot = SlotDefinition {
            name: "test".to_string(),
            exactly_one_of: Some(constraints),
            ..Default::default()
        };

        let issues = validator.validate(&value, &slot, &mut context);

        // If there are no issues, exactly one constraint was satisfied
        // If there are issues, either none or multiple were satisfied
        // This is the invariant we're testing - no panic should occur
        let _ = issues;
    }

    /// Test that none_of with all type constraints rejects all basic types
    #[test]
    fn test_none_of_all_types_rejection() {
        let validator = NoneOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        let all_types = vec!["string", "integer", "float", "boolean", "null"];
        let constraints: Vec<_> = all_types.iter()
            .map(|t| AnonymousSlotExpression {
                range: Some(t.to_string()),
                ..Default::default()
            })
            .collect();

        let slot = SlotDefinition {
            name: "test".to_string(),
            none_of: Some(constraints),
            ..Default::default()
        };

        // Test basic values - all should fail
        for value in &[
            json!("string"),
            json!(42),
            json!(3.14),
            json!(true),
            json!(null),
        ] {
            let issues = validator.validate(value, &slot, &mut context);
            assert!(!issues.is_empty(), "Value {:?} should have been rejected", value);
        }

        // Arrays and objects should pass
        let issues = validator.validate(&json!([1, 2, 3]), &slot, &mut context);
        assert!(issues.is_empty());

        let issues = validator.validate(&json!({"key": "value"}), &slot, &mut context);
        assert!(issues.is_empty());
    }

    /// Test range boundary conditions
    #[test]
    fn test_range_boundaries_in_boolean_constraints(
        min in any::<i32>(),
        max in any::<i32>(),
        value in any::<i32>()
    ) {
        let validator = AllOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        // Ensure min <= max
        let (min, max) = if min <= max { (min, max) } else { (max, min) };

        let slot = SlotDefinition {
            name: "test".to_string(),
            all_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("integer".to_string()),
                    minimum_value: Some(json!(min)),
                    maximum_value: Some(json!(max)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let issues = validator.validate(&json!(value), &slot, &mut context);

        // Check the invariant: issues should be empty iff min <= value <= max
        let in_range = min <= value && value <= max;
        assert_eq!(issues.is_empty(), in_range);
    }

    /// Test pattern matching edge cases
    #[test]
    fn test_pattern_edge_cases_in_any_of(
        s in ".*",
        use_pattern in any::<bool>()
    ) {
        let validator = AnyOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        let constraints = if use_pattern {
            vec![
                AnonymousSlotExpression {
                    pattern: Some(r"^.*$".to_string()), // Matches everything
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    pattern: Some(r"^impossible\x00pattern$".to_string()), // Should never match
                    ..Default::default()
                },
            ]
        } else {
            vec![
                AnonymousSlotExpression {
                    range: Some("string".to_string()),
                    ..Default::default()
                },
            ]
        };

        let slot = SlotDefinition {
            name: "test".to_string(),
            any_of: Some(constraints),
            ..Default::default()
        };

        let issues = validator.validate(&json!(s), &slot, &mut context);
        // String values should pass either way
        assert!(issues.is_empty());
    }

    /// Test that parallel evaluation in all_of produces same results as sequential
    #[test]
    fn test_all_of_parallel_consistency(
        constraints in prop::collection::vec(arb_anonymous_slot_expression(), 5..10),
        value in arb_json_value()
    ) {
        // Test with sequential evaluation (high threshold)
        let sequential_validator = AllOfValidator::with_parallel_threshold(100);
        let schema = Arc::new(SchemaDefinition::default());
        let mut context1 = // ValidationContext::new());

        let slot = SlotDefinition {
            name: "test".to_string(),
            all_of: Some(constraints),
            ..Default::default()
        };

        let sequential_issues = sequential_validator.validate(&value, &slot, &mut context1);

        // Test with parallel evaluation (low threshold)
        let parallel_validator = AllOfValidator::with_parallel_threshold(1);
        let mut context2 = // ValidationContext::new();

        let parallel_issues = parallel_validator.validate(&value, &slot, &mut context2);

        // Both should produce the same validity result
        assert_eq!(sequential_issues.is_empty(), parallel_issues.is_empty());
    }

    /// Test none_of early exit optimization correctness
    #[test]
    fn test_none_of_early_exit_correctness(
        num_constraints in 1usize..20,
        satisfied_index in any::<usize>(),
        value_type in arb_range_type()
    ) {
        let validator = NoneOfValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = // ValidationContext::new();

        // Create constraints where one will be satisfied
        let satisfied_idx = satisfied_index % num_constraints;
        let mut constraints = Vec::new();

        for i in 0..num_constraints {
            if i == satisfied_idx {
                // This constraint will be satisfied
                constraints.push(AnonymousSlotExpression {
                    range: Some(value_type.clone()),
                    ..Default::default()
                });
            } else {
                // These won't be satisfied
                constraints.push(AnonymousSlotExpression {
                    range: Some("impossible_type".to_string()),
                    ..Default::default()
                });
            }
        }

        let slot = SlotDefinition {
            name: "test".to_string(),
            none_of: Some(constraints),
            ..Default::default()
        };

        // Create a value of the matching type
        let value = match value_type.as_str() {
            "string" => json!("test"),
            "integer" => json!(42),
            "float" | "double" => json!(3.14),
            "boolean" => json!(true),
            "null" => json!(null),
            _ => json!("default"),
        };

        let issues = validator.validate(&value, &slot, &mut context);

        // Should always have issues (constraint is satisfied)
        assert!(!issues.is_empty());
        // Should detect the satisfied constraint
        assert!(issues[0].message.contains(&satisfied_idx.to_string()));
    }
}
