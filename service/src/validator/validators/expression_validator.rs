//! Expression validator for computed fields and validation rules
//!
//! This module validates `equals_expression` and rules constraints using the expression engine.

use linkml_core::types::SlotDefinition;
use serde_json::Value;
use std::collections::HashMap;

use crate::expression::ExpressionEngine;
use crate::validator::{context::ValidationContext, report::ValidationIssue};

use super::Validator;

/// Validator for expression-based constraints
pub struct ExpressionValidator {
    engine: ExpressionEngine,
}

impl Default for ExpressionValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionValidator {
    /// Create a new expression validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            engine: ExpressionEngine::new(),
        }
    }
}

impl Validator for ExpressionValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Build expression context from current data
        let expr_context = build_expression_context(value, context);

        // Validate equals_expression if present
        if let Some(equals_expr) = &slot.equals_expression {
            match self.engine.evaluate(equals_expr, &expr_context) {
                Ok(computed_value) => {
                    if value != &computed_value {
                        issues.push(
                            ValidationIssue::error(
                                format!(
                                    "Value does not match computed expression. Expected: {computed_value:?}, Got: {value:?}"
                                ),
                                context.path(),
                                self.name(),
                            )
                            .with_code("EQUALS_EXPRESSION_MISMATCH")
                            .with_context("expression", equals_expr.as_str().into())
                            .with_context("computed_value", computed_value)
                            .with_context("actual_value", value.clone())
                        );
                    }
                }
                Err(e) => {
                    issues.push(
                        ValidationIssue::error(
                            format!("Failed to evaluate equals_expression: {e}"),
                            context.path(),
                            self.name(),
                        )
                        .with_code("EXPRESSION_EVALUATION_ERROR")
                        .with_context("expression", equals_expr.as_str().into())
                        .with_context("error", e.to_string().into()),
                    );
                }
            }
        }

        // Validate rules if present
        if let Some(rules) = &slot.rules {
            for (i, rule) in rules.iter().enumerate() {
                match self.engine.evaluate(rule, &expr_context) {
                    Ok(result) => {
                        // Rule must evaluate to true
                        if let Value::Bool(false) = result {
                            issues.push(
                                ValidationIssue::error(
                                    format!("Rule {} failed: {}", i + 1, rule),
                                    context.path(),
                                    self.name(),
                                )
                                .with_code("RULE_VIOLATION")
                                .with_context("rule_index", (i + 1).into())
                                .with_context("rule", rule.as_str().into()),
                            );
                        } else if !result.is_boolean() {
                            issues.push(
                                ValidationIssue::error(
                                    format!("Rule {} did not evaluate to boolean: {}", i + 1, rule),
                                    context.path(),
                                    self.name(),
                                )
                                .with_code("RULE_TYPE_ERROR")
                                .with_context("rule_index", (i + 1).into())
                                .with_context("rule", rule.as_str().into())
                                .with_context("result_type", result.type_name().into()),
                            );
                        }
                    }
                    Err(e) => {
                        issues.push(
                            ValidationIssue::error(
                                format!("Failed to evaluate rule {}: {}", i + 1, e),
                                context.path(),
                                self.name(),
                            )
                            .with_code("RULE_EVALUATION_ERROR")
                            .with_context("rule_index", (i + 1).into())
                            .with_context("rule", rule.as_str().into())
                            .with_context("error", e.to_string().into()),
                        );
                    }
                }
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "ExpressionValidator"
    }
}

/// Build expression context from current validation state
fn build_expression_context(value: &Value, context: &ValidationContext) -> HashMap<String, Value> {
    let mut expr_context = HashMap::new();

    // Add current value
    expr_context.insert("value".to_string(), value.clone());

    // Add parent object if available
    if let Some(parent) = context.parent() {
        expr_context.insert("parent".to_string(), parent.clone());
    }

    // Add root object
    if let Some(root) = context.root() {
        expr_context.insert("root".to_string(), root.clone());
    }

    // Add path information
    expr_context.insert("path".to_string(), context.path().into());

    // Add any other context variables that might be useful
    // This could be extended to include schema metadata, etc.

    expr_context
}

/// Helper trait to get type name for error messages
trait TypeName {
    fn type_name(&self) -> &str;
}

impl TypeName for Value {
    fn type_name(&self) -> &str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::report::Severity;
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_equals_expression_validation() {
        let validator = ExpressionValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Set up parent context
        context.push_path("person");
        context.set_parent(json!({
            "first_name": "John",
            "last_name": "Doe"
        }));
        context.push_path("full_name");

        // Slot with equals_expression
        let slot = SlotDefinition {
            name: "full_name".to_string(),
            equals_expression: Some("{parent.first_name} + \" \" + {parent.last_name}".to_string()),
            ..Default::default()
        };

        // Test correct value
        let issues = validator.validate(&json!("John Doe"), &slot, &mut context);
        assert!(issues.is_empty());

        // Test incorrect value
        let issues = validator.validate(&json!("Jane Doe"), &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(
            issues[0].code,
            Some("EQUALS_EXPRESSION_MISMATCH".to_string())
        );
    }

    #[test]
    fn test_rules_validation() {
        let validator = ExpressionValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Slot with validation rules
        let slot = SlotDefinition {
            name: "age".to_string(),
            rules: Some(vec![
                "{value} >= 0".to_string(),
                "{value} <= 150".to_string(),
            ]),
            ..Default::default()
        };

        // Test valid value
        let issues = validator.validate(&json!(25), &slot, &mut context);
        assert!(issues.is_empty());

        // Test invalid value (negative)
        let issues = validator.validate(&json!(-5), &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, Some("RULE_VIOLATION".to_string()));

        // Test invalid value (too large)
        let issues = validator.validate(&json!(200), &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, Some("RULE_VIOLATION".to_string()));
    }

    #[test]
    fn test_complex_rule_expressions() {
        let validator = ExpressionValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Set up parent context with password policy
        context.set_parent(json!({
            "password_policy": {
                "min_length": 8,
                "require_special": true
            }
        }));

        let slot = SlotDefinition {
            name: "password".to_string(),
            rules: Some(vec![
                "len({value}) >= {parent.password_policy.min_length}".to_string(),
                "not {parent.password_policy.require_special} or contains({value}, \"@\") or contains({value}, \"!\") or contains({value}, \"#\")".to_string(),
            ]),
            ..Default::default()
        };

        // Test valid password
        let issues = validator.validate(&json!("secure@123"), &slot, &mut context);
        assert!(issues.is_empty());

        // Test too short password
        let issues = validator.validate(&json!("pass@"), &slot, &mut context);
        assert_eq!(issues.len(), 1);

        // Test missing special character
        let issues = validator.validate(&json!("securepwd123"), &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }
}
