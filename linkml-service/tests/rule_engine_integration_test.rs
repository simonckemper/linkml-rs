//! Integration tests for rule engine with validation framework

use indexmap::IndexMap;
use linkml_core::types::{
    ClassDefinition, EnumDefinition, Rule, RuleConditions, SchemaDefinition, SlotCondition,
    SlotDefinition, SubsetDefinition, TypeDefinition,
};
use linkml_service::validator::engine::{ValidationEngine, ValidationOptions};
use serde_json::json;
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "https://example.org/test-rules".to_string(),
        name: "test_rules_schema".to_string(),
        ..Default::default()
    };

    // Define slots
    let mut slots = IndexMap::new();

    slots.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            description: Some("Person's age in years".to_string()),
            range: Some("integer".to_string()),
            minimum_value: Some(json!(0)),
            maximum_value: Some(json!(150)),
            ..Default::default()
        },
    );

    slots.insert(
        "guardian_name".to_string(),
        SlotDefinition {
            name: "guardian_name".to_string(),
            description: Some("Name of legal guardian".to_string()),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    slots.insert(
        "guardian_phone".to_string(),
        SlotDefinition {
            name: "guardian_phone".to_string(),
            description: Some("Guardian's contact phone".to_string()),
            range: Some("string".to_string()),
            pattern: Some(r"^\+?[\d\s\-()]+$".to_string()),
            ..Default::default()
        },
    );

    slots.insert(
        "status".to_string(),
        SlotDefinition {
            name: "status".to_string(),
            range: Some("string".to_string()),
            permissible_values: vec![
                linkml_core::types::PermissibleValue::Simple("active".to_string()),
                linkml_core::types::PermissibleValue::Simple("inactive".to_string()),
                linkml_core::types::PermissibleValue::Simple("pending".to_string()),
            ],
            ..Default::default()
        },
    );

    slots.insert(
        "reason".to_string(),
        SlotDefinition {
            name: "reason".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots = slots;

    // Create Person class with rules
    let mut person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person with age-based validation rules".to_string()),
        slots: vec![
            "age".to_string(),
            "guardian_name".to_string(),
            "guardian_phone".to_string(),
        ],
        ..Default::default()
    };

    // Rule 1: Minors require guardian information
    let mut minor_preconditions = IndexMap::new();
    minor_preconditions.insert(
        "age".to_string(),
        SlotCondition {
            maximum_value: Some(json!(17)),
            ..Default::default()
        },
    );

    let mut guardian_postconditions = IndexMap::new();
    guardian_postconditions.insert(
        "guardian_name".to_string(),
        SlotCondition {
            required: Some(true),
            ..Default::default()
        },
    );
    guardian_postconditions.insert(
        "guardian_phone".to_string(),
        SlotCondition {
            required: Some(true),
            ..Default::default()
        },
    );

    let minor_rule = Rule {
        description: Some("Minors must have guardian information".to_string()),
        priority: Some(100),
        preconditions: Some(RuleConditions {
            slot_conditions: Some(minor_preconditions),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            slot_conditions: Some(guardian_postconditions),
            ..Default::default()
        }),
        ..Default::default()
    };

    person_class.rules.push(minor_rule);

    // Rule 2: Adults should not have guardian information
    let mut adult_preconditions = IndexMap::new();
    adult_preconditions.insert(
        "age".to_string(),
        SlotCondition {
            minimum_value: Some(json!(18)),
            ..Default::default()
        },
    );

    let adult_rule = Rule {
        description: Some("Adults should not have guardian information".to_string()),
        priority: Some(50),
        preconditions: Some(RuleConditions {
            slot_conditions: Some(adult_preconditions),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            expression_conditions: Some(vec![
                "{guardian_name} == null or {guardian_name} == \"\"".to_string(),
            ]),
            ..Default::default()
        }),
        ..Default::default()
    };

    person_class.rules.push(adult_rule);

    schema.classes.insert("Person".to_string(), person_class);

    // Create Account class with status-based rules
    let mut account_class = ClassDefinition {
        name: "Account".to_string(),
        description: Some("An account with status-based rules".to_string()),
        slots: vec!["status".to_string(), "reason".to_string()],
        ..Default::default()
    };

    // Rule: Inactive accounts must have a reason
    let mut inactive_preconditions = IndexMap::new();
    inactive_preconditions.insert(
        "status".to_string(),
        SlotCondition {
            equals_string: Some("inactive".to_string()),
            ..Default::default()
        },
    );

    let mut reason_postconditions = IndexMap::new();
    reason_postconditions.insert(
        "reason".to_string(),
        SlotCondition {
            required: Some(true),
            ..Default::default()
        },
    );

    let inactive_rule = Rule {
        description: Some("Inactive accounts must have a detailed reason".to_string()),
        priority: Some(100),
        preconditions: Some(RuleConditions {
            slot_conditions: Some(inactive_preconditions),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            slot_conditions: Some(reason_postconditions),
            ..Default::default()
        }),
        ..Default::default()
    };

    account_class.rules.push(inactive_rule);

    schema.classes.insert("Account".to_string(), account_class);

    schema
}

#[tokio::test]
async fn test_minor_without_guardian_fails() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "age": 15
    });

    let report = engine
        .validate_as_class(&instance, "Person", None)
        .await
        .expect("Test operation failed");

    // Debug output
    if !report.issues.is_empty() {
        println!("Validation issues:");
        for issue in &report.issues {
            println!("  - {}: {}", issue.severity, issue.message);
        }
    }

    assert!(!report.valid);
    assert_eq!(report.stats.error_count, 2);

    // Check for specific error messages
    let messages: Vec<_> = report.issues.iter().map(|i| &i.message).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("guardian_name") && m.contains("required"))
    );
    assert!(
        messages
            .iter()
            .any(|m| m.contains("guardian_phone") && m.contains("required"))
    );
}

#[tokio::test]
async fn test_minor_with_guardian_passes() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "age": 15,
        "guardian_name": "John Doe",
        "guardian_phone": "+1-555-1234"
    });

    let report = engine
        .validate_as_class(&instance, "Person", None)
        .await
        .expect("Test operation failed");

    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);
}

#[tokio::test]
async fn test_adult_with_guardian_fails() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "age": 25,
        "guardian_name": "Jane Doe",
        "guardian_phone": "+1-555-5678"
    });

    let report = engine
        .validate_as_class(&instance, "Person", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    assert_eq!(report.stats.error_count, 1);

    // Check for specific error message
    let messages: Vec<_> = report.issues.iter().map(|i| &i.message).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("Adults should not have guardian"))
    );
}

#[tokio::test]
async fn test_adult_without_guardian_passes() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "age": 30
    });

    let report = engine
        .validate_as_class(&instance, "Person", None)
        .await
        .expect("Test operation failed");

    // Debug output
    if !report.issues.is_empty() {
        println!("Validation issues for adult without guardian:");
        for issue in &report.issues {
            println!("  - {}: {}", issue.severity, issue.message);
        }
    }

    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);
}

#[tokio::test]
async fn test_inactive_account_without_reason_fails() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "status": "inactive"
    });

    let report = engine
        .validate_as_class(&instance, "Account", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    assert_eq!(report.stats.error_count, 1);

    let messages: Vec<_> = report.issues.iter().map(|i| &i.message).collect();
    assert!(
        messages
            .iter()
            .any(|m| m.contains("reason") && m.contains("required"))
    );
}

#[tokio::test]
async fn test_inactive_account_with_valid_reason_passes() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "status": "inactive",
        "reason": "Account suspended due to terms of service violation"
    });

    let report = engine
        .validate_as_class(&instance, "Account", None)
        .await
        .expect("Test operation failed");

    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);
}

#[tokio::test]
async fn test_active_account_without_reason_passes() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "status": "active"
    });

    let report = engine
        .validate_as_class(&instance, "Account", None)
        .await
        .expect("Test operation failed");

    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);
}

#[tokio::test]
async fn test_fail_fast_option() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instance = json!({
        "age": 15  // Minor without guardian info - should generate 2 errors
    });

    let options = ValidationOptions {
        fail_fast: true,
        ..Default::default()
    };

    let report = engine
        .validate_as_class(&instance, "Person", Some(options))
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    // With fail_fast, we might only get the first error
    assert!(report.stats.error_count >= 1);
}
