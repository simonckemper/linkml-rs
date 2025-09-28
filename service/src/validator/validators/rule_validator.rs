//! Rule-based validator for class-level validation
//!
//! This validator integrates the rule engine with the validation framework
//! to enable if-then-else validation logic at the class level.

use linkml_core::types::{ClassDefinition, SchemaDefinition};
use serde_json::Value;
use std::sync::Arc;

use crate::rule_engine::{RuleEngine, RuleExecutionStrategy};
use crate::validator::{context::ValidationContext, report::ValidationIssue};

/// Validator for class-level rules
pub struct RuleValidator {
    rule_engine: Arc<RuleEngine>,
}

impl RuleValidator {
    /// Create a new rule validator
    #[must_use]
    pub fn new(schema: Arc<SchemaDefinition>) -> Self {
        Self {
            rule_engine: Arc::new(RuleEngine::new(schema)),
        }
    }

    /// Create a rule validator with custom execution strategy
    #[must_use]
    pub fn with_strategy(schema: Arc<SchemaDefinition>, strategy: RuleExecutionStrategy) -> Self {
        Self {
            rule_engine: Arc::new(RuleEngine::with_strategy(schema, strategy)),
        }
    }

    /// Validate an instance against class rules
    pub fn validate_instance(
        &self,
        instance: &Value,
        class_name: &str,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        self.rule_engine.validate(instance, class_name, context)
    }

    /// Get the rule engine for advanced usage
    #[must_use]
    pub fn rule_engine(&self) -> &Arc<RuleEngine> {
        &self.rule_engine
    }
}

/// Integration trait for adding rule validation to the validator pipeline
pub trait RuleValidation {
    /// Validate against class rules
    fn validate_rules(
        &self,
        instance: &Value,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue>;
}

impl RuleValidation for RuleValidator {
    fn validate_rules(
        &self,
        instance: &Value,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        // Only validate if the class has rules
        if class_def.rules.is_empty() {
            return Vec::new();
        }

        self.validate_instance(instance, &class_def.name, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use linkml_core::types::{Rule, RuleConditions, SlotCondition};
    use serde_json::json;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();

        // Create a Person class with rules
        let mut person_class = ClassDefinition {
            name: "Person".to_string(),
            ..Default::default()
        };

        // Add slots
        person_class.slots.push("age".to_string());
        person_class.slots.push("guardian_name".to_string());
        person_class.slots.push("guardian_phone".to_string());

        // Add rule: minors require guardian info
        let mut minor_conditions = IndexMap::new();
        minor_conditions.insert(
            "age".to_string(),
            SlotCondition {
                maximum_value: Some(json!(17)),
                ..Default::default()
            },
        );

        let mut guardian_conditions = IndexMap::new();
        guardian_conditions.insert(
            "guardian_name".to_string(),
            SlotCondition {
                required: Some(true),
                ..Default::default()
            },
        );
        guardian_conditions.insert(
            "guardian_phone".to_string(),
            SlotCondition {
                required: Some(true),
                pattern: Some(r"^\+?[\d\s\-()]+$".to_string()),
                ..Default::default()
            },
        );

        let minor_rule = Rule {
            description: Some("Minors must have guardian information".to_string()),
            priority: Some(100),
            preconditions: Some(RuleConditions {
                slot_conditions: Some(minor_conditions),
                ..Default::default()
            }),
            postconditions: Some(RuleConditions {
                slot_conditions: Some(guardian_conditions),
                ..Default::default()
            }),
            ..Default::default()
        };

        person_class.rules.push(minor_rule);

        // Add rule: adults can't have guardian info
        let mut adult_conditions = IndexMap::new();
        adult_conditions.insert(
            "age".to_string(),
            SlotCondition {
                minimum_value: Some(json!(18)),
                ..Default::default()
            },
        );

        let mut no_guardian_conditions = IndexMap::new();
        no_guardian_conditions.insert(
            "guardian_name".to_string(),
            SlotCondition {
                equals_string: Some("".to_string()),
                ..Default::default()
            },
        );

        let adult_rule = Rule {
            description: Some("Adults should not have guardian information".to_string()),
            priority: Some(50),
            preconditions: Some(RuleConditions {
                slot_conditions: Some(adult_conditions),
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

        // Also add slot definitions
        let mut slots = IndexMap::new();
        slots.insert(
            "age".to_string(),
            linkml_core::types::SlotDefinition {
                name: "age".to_string(),
                range: Some("integer".to_string()),
                ..Default::default()
            },
        );
        slots.insert(
            "guardian_name".to_string(),
            linkml_core::types::SlotDefinition {
                name: "guardian_name".to_string(),
                range: Some("string".to_string()),
                ..Default::default()
            },
        );
        slots.insert(
            "guardian_phone".to_string(),
            linkml_core::types::SlotDefinition {
                name: "guardian_phone".to_string(),
                range: Some("string".to_string()),
                ..Default::default()
            },
        );
        schema.slots = slots;

        schema
    }

    #[test]
    fn test_minor_without_guardian() {
        let schema = Arc::new(create_test_schema());
        let validator = RuleValidator::new(schema.clone());
        let mut context = ValidationContext::new(schema);

        let instance = json!({
            "age": 15,
            "name": "Alice"
        });

        let issues = validator.validate_instance(&instance, "Person", &mut context);

        // Should have 2 issues - missing guardian_name and guardian_phone
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|i| i.message.contains("guardian_name")));
        assert!(issues.iter().any(|i| i.message.contains("guardian_phone")));
    }

    #[test]
    fn test_minor_with_guardian() {
        let schema = Arc::new(create_test_schema());
        let validator = RuleValidator::new(schema.clone());
        let mut context = ValidationContext::new(schema);

        let instance = json!({
            "age": 15,
            "name": "Bob",
            "guardian_name": "Parent",
            "guardian_phone": "+1-555-1234"
        });

        let issues = validator.validate_instance(&instance, "Person", &mut context);

        // Should pass validation
        assert!(issues.is_empty());
    }

    #[test]
    fn test_adult_with_guardian() {
        let schema = Arc::new(create_test_schema());
        let validator = RuleValidator::new(schema.clone());
        let mut context = ValidationContext::new(schema);

        let instance = json!({
            "age": 25,
            "name": "Charlie",
            "guardian_name": "Someone",
            "guardian_phone": "+1-555-5678"
        });

        let issues = validator.validate_instance(&instance, "Person", &mut context);

        // Should have 1 issue - adult shouldn't have guardian
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0]
                .message
                .contains("Adults should not have guardian")
        );
    }

    #[test]
    fn test_adult_without_guardian() {
        let schema = Arc::new(create_test_schema());
        let validator = RuleValidator::new(schema.clone());
        let mut context = ValidationContext::new(schema);

        let instance = json!({
            "age": 30,
            "name": "Diana"
        });

        let issues = validator.validate_instance(&instance, "Person", &mut context);

        // Should pass validation
        assert!(issues.is_empty());
    }
}
