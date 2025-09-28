//! Rule evaluator for postcondition validation
//!
//! This module handles evaluating postconditions and else conditions
//! after a rule's preconditions have been matched.

use serde_json::Value;
use std::collections::HashMap;

use crate::expression::ExpressionEngine;
use crate::validator::report::{Severity, ValidationIssue};

use super::matcher::RuleMatcher;
use super::types::{
    CompiledCompositeCondition, CompiledCondition, CompiledSlotCondition, RuleExecutionContext,
};

/// Evaluator for rule postconditions
pub struct RuleEvaluator {
    expression_engine: ExpressionEngine,
    matcher: RuleMatcher,
}

impl RuleEvaluator {
    /// Create a new rule evaluator
    #[must_use]
    pub fn new(expression_engine: ExpressionEngine) -> Self {
        let matcher = RuleMatcher::new(expression_engine.clone());
        Self {
            expression_engine,
            matcher,
        }
    }

    /// Evaluate postconditions and generate validation issues
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate_postconditions(
        &self,
        condition: &CompiledCondition,
        context: &RuleExecutionContext,
        rule_description: Option<&str>,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // Check if postconditions are satisfied
        let satisfied = self.matcher.matches(condition, context)?;

        if !satisfied {
            // Generate detailed error messages based on condition type
            match condition {
                CompiledCondition::SlotConditions(slot_conditions) => {
                    issues.extend(self.evaluate_slot_conditions(
                        slot_conditions,
                        context,
                        rule_description,
                    )?);
                }
                CompiledCondition::ExpressionConditions(expressions) => {
                    issues.extend(self.evaluate_expression_conditions(
                        expressions,
                        context,
                        rule_description,
                    )?);
                }
                CompiledCondition::Composite(composite) => {
                    issues.extend(self.evaluate_composite_condition(
                        composite,
                        context,
                        rule_description,
                    )?);
                }
                CompiledCondition::Combined {
                    slot_conditions,
                    expression_conditions,
                    composite_conditions,
                } => {
                    if let Some(slots) = slot_conditions {
                        issues.extend(self.evaluate_slot_conditions(
                            slots,
                            context,
                            rule_description,
                        )?);
                    }

                    if let Some(exprs) = expression_conditions {
                        issues.extend(self.evaluate_expression_conditions(
                            exprs,
                            context,
                            rule_description,
                        )?);
                    }

                    if let Some(composite) = composite_conditions {
                        issues.extend(self.evaluate_composite_condition(
                            composite,
                            context,
                            rule_description,
                        )?);
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Evaluate slot conditions and generate specific errors
    fn evaluate_slot_conditions(
        &self,
        conditions: &HashMap<String, CompiledSlotCondition>,
        context: &RuleExecutionContext,
        rule_description: Option<&str>,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        let Value::Object(instance_obj) = &context.instance else {
            issues.push(
                ValidationIssue::error(
                    "Instance must be an object for slot condition evaluation",
                    context.validation_context.path(),
                    "RuleEvaluator",
                )
                .with_code("INVALID_INSTANCE_TYPE"),
            );
            return Ok(issues);
        };

        for (slot_name, condition) in conditions {
            let slot_value = instance_obj.get(slot_name).unwrap_or(&Value::Null);
            let slot_path = format!("{}.{}", context.validation_context.path(), slot_name);

            issues.extend(self.evaluate_slot_condition(
                slot_value,
                condition,
                &slot_path,
                context,
                rule_description,
            )?);
        }

        Ok(issues)
    }

    /// Evaluate a single slot condition
    fn evaluate_slot_condition(
        &self,
        value: &Value,
        condition: &CompiledSlotCondition,
        path: &str,
        context: &RuleExecutionContext,
        rule_description: Option<&str>,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let original = &condition.original;

        // Check required
        if let Some(true) = original.required
            && value.is_null()
        {
            // Extract field name from path (e.g., "person.guardian_name" -> "guardian_name")
            let field_name = path.split('.').next_back().unwrap_or(path);
            let msg = if let Some(desc) = rule_description {
                format!("Field '{field_name}' is required by rule: {desc}")
            } else {
                format!("Field '{field_name}' is required by rule")
            };

            issues.push(
                ValidationIssue::error(&msg, path, "RuleEvaluator")
                    .with_code("RULE_REQUIRED_FIELD"),
            );
            return Ok(issues);
        }

        // Skip further checks if value is null
        if value.is_null() {
            return Ok(issues);
        }

        // Check equals_string
        if let Some(ref expected) = original.equals_string
            && let Value::String(actual) = value
            && actual != expected
        {
            let msg = format!(
                "Value must equal '{}', got '{}'{}",
                expected,
                actual,
                rule_description
                    .map(|d| format!(" (rule: {d})"))
                    .unwrap_or_default()
            );

            issues.push(
                ValidationIssue::error(&msg, path, "RuleEvaluator")
                    .with_code("RULE_EQUALS_STRING")
                    .with_context("expected", expected.clone().into())
                    .with_context("actual", actual.clone().into()),
            );
        }

        // Check equals_expression
        if let Some(ref expr_ast) = condition.equals_expression_ast {
            let expr_context = context.get_expression_context();
            match self.expression_engine.evaluate_ast(expr_ast, &expr_context) {
                Ok(computed) => {
                    if value != &computed {
                        let msg = format!(
                            "Value must equal computed expression result{}",
                            rule_description
                                .map(|d| format!(" (rule: {d})"))
                                .unwrap_or_default()
                        );

                        issues.push(
                            ValidationIssue::error(&msg, path, "RuleEvaluator")
                                .with_code("RULE_EQUALS_EXPRESSION")
                                .with_context("computed", computed)
                                .with_context("actual", value.clone()),
                        );
                    }
                }
                Err(e) => {
                    issues.push(
                        ValidationIssue::error(
                            format!("Failed to evaluate expression: {e}"),
                            path,
                            "RuleEvaluator",
                        )
                        .with_code("RULE_EXPRESSION_ERROR"),
                    );
                }
            }
        }

        // Additional constraint checks would go here...

        Ok(issues)
    }

    /// Evaluate expression conditions
    fn evaluate_expression_conditions(
        &self,
        expressions: &[crate::expression::ast::Expression],
        context: &RuleExecutionContext,
        rule_description: Option<&str>,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        let expr_context = context.get_expression_context();

        for (i, expr) in expressions.iter().enumerate() {
            match self.expression_engine.evaluate_ast(expr, &expr_context) {
                Ok(Value::Bool(false)) => {
                    let msg = format!(
                        "Expression {} failed{}",
                        i + 1,
                        rule_description
                            .map(|d| format!(" (rule: {d})"))
                            .unwrap_or_default()
                    );

                    issues.push(
                        ValidationIssue::error(
                            &msg,
                            context.validation_context.path(),
                            "RuleEvaluator",
                        )
                        .with_code("RULE_EXPRESSION_FAILED")
                        .with_context("expression_index", (i + 1).into()),
                    );
                }
                Ok(Value::Bool(true)) => {}
                Ok(_) => {
                    issues.push(
                        ValidationIssue::error(
                            format!("Expression {} must evaluate to boolean", i + 1),
                            context.validation_context.path(),
                            "RuleEvaluator",
                        )
                        .with_code("RULE_EXPRESSION_TYPE_ERROR"),
                    );
                }
                Err(e) => {
                    issues.push(
                        ValidationIssue::error(
                            format!("Failed to evaluate expression {}: {}", i + 1, e),
                            context.validation_context.path(),
                            "RuleEvaluator",
                        )
                        .with_code("RULE_EXPRESSION_ERROR"),
                    );
                }
            }
        }

        Ok(issues)
    }

    /// Evaluate composite conditions
    fn evaluate_composite_condition(
        &self,
        composite: &CompiledCompositeCondition,
        context: &RuleExecutionContext,
        rule_description: Option<&str>,
    ) -> linkml_core::error::Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        match composite {
            CompiledCompositeCondition::AnyOf(conditions) => {
                // For any_of, all must fail to generate an error
                let mut all_failed = true;
                let mut sub_issues = Vec::new();

                for condition in conditions {
                    let condition_issues =
                        self.evaluate_postconditions(condition, context, rule_description)?;
                    if condition_issues.is_empty() {
                        all_failed = false;
                        break;
                    }
                    sub_issues.extend(condition_issues);
                }

                if all_failed {
                    let msg = format!(
                        "At least one condition must be satisfied{}",
                        rule_description
                            .map(|d| format!(" (rule: {d})"))
                            .unwrap_or_default()
                    );

                    issues.push(
                        ValidationIssue::error(
                            &msg,
                            context.validation_context.path(),
                            "RuleEvaluator",
                        )
                        .with_code("RULE_ANY_OF_FAILED"),
                    );

                    // Add sub-issues as warnings for debugging
                    for mut issue in sub_issues {
                        issue.severity = Severity::Warning;
                        issues.push(issue);
                    }
                }
            }
            CompiledCompositeCondition::AllOf(conditions) => {
                // For all_of, collect all failures
                for condition in conditions {
                    issues.extend(self.evaluate_postconditions(
                        condition,
                        context,
                        rule_description,
                    )?);
                }
            }
            CompiledCompositeCondition::ExactlyOneOf(conditions) => {
                // For exactly_one_of, count satisfied conditions
                let mut satisfied_count = 0;
                let mut satisfied_indices = Vec::new();

                for (i, condition) in conditions.iter().enumerate() {
                    let condition_issues =
                        self.evaluate_postconditions(condition, context, rule_description)?;
                    if condition_issues.is_empty() {
                        satisfied_count += 1;
                        satisfied_indices.push(i + 1);
                    }
                }

                if satisfied_count != 1 {
                    let msg = format!(
                        "Exactly one condition must be satisfied, but {} were{}",
                        satisfied_count,
                        rule_description
                            .map(|d| format!(" (rule: {d})"))
                            .unwrap_or_default()
                    );

                    issues.push(
                        ValidationIssue::error(
                            &msg,
                            context.validation_context.path(),
                            "RuleEvaluator",
                        )
                        .with_code("RULE_EXACTLY_ONE_OF_FAILED")
                        .with_context("satisfied_count", satisfied_count.into())
                        .with_context("satisfied_indices", serde_json::json!(satisfied_indices)),
                    );
                }
            }
            CompiledCompositeCondition::NoneOf(conditions) => {
                // For none_of, any satisfied condition is an error
                for (i, condition) in conditions.iter().enumerate() {
                    let condition_issues =
                        self.evaluate_postconditions(condition, context, rule_description)?;
                    if condition_issues.is_empty() {
                        let msg = format!(
                            "Condition {} must not be satisfied{}",
                            i + 1,
                            rule_description
                                .map(|d| format!(" (rule: {d})"))
                                .unwrap_or_default()
                        );

                        issues.push(
                            ValidationIssue::error(
                                &msg,
                                context.validation_context.path(),
                                "RuleEvaluator",
                            )
                            .with_code("RULE_NONE_OF_FAILED")
                            .with_context("violated_condition", (i + 1).into()),
                        );
                    }
                }
            }
        }

        Ok(issues)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::context::ValidationContext;
    use linkml_core::types::SlotCondition;
    use serde_json::json;

    #[test]
    fn test_required_field_evaluation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let evaluator = RuleEvaluator::new(ExpressionEngine::new());

        let condition = CompiledSlotCondition {
            original: SlotCondition {
                required: Some(true),
                ..Default::default()
            },
            equals_expression_ast: None,
        };

        let mut validation_ctx = ValidationContext::new(Default::default());
        let context = RuleExecutionContext::new(
            json!({"name": "test"}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        // Missing required field should generate error
        let issues = evaluator.evaluate_slot_condition(
            &Value::Null,
            &condition,
            "person.id",
            &context,
            Some("ID required for persons"),
        )?;

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, Some("RULE_REQUIRED_FIELD".to_string()));
        assert!(issues[0].message.contains("ID required for persons"));
        Ok(())
    }

    #[test]
    fn test_expression_evaluation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let evaluator = RuleEvaluator::new(ExpressionEngine::new());

        // Parse a failing expression
        let parser = crate::expression::parser::Parser::new();
        let expr = parser.parse("{age} >= 21")?;

        let mut validation_ctx = ValidationContext::new(Default::default());
        let context = RuleExecutionContext::new(
            json!({"age": 18}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        let issues = evaluator.evaluate_expression_conditions(
            &[expr],
            &context,
            Some("Must be 21 or older"),
        )?;

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].code, Some("RULE_EXPRESSION_FAILED".to_string()));
        Ok(())
    }
}
