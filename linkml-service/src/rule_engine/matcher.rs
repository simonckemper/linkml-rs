//! Rule matching logic for precondition evaluation
//!
//! This module handles determining whether a rule's preconditions are satisfied
//! for a given instance.

use linkml_core::error::{LinkMLError, Result};
use anyhow::anyhow;
use serde_json::Value;
use std::collections::HashMap;

use crate::expression::ExpressionEngine;
use crate::validator::context::ValidationContext;
use crate::validator::validators::{PatternValidator, RangeValidator, Validator as SlotValidator};

use super::types::{
    CompiledCompositeCondition, CompiledCondition, CompiledSlotCondition, RuleExecutionContext,
};

/// Matcher for evaluating rule preconditions
pub struct RuleMatcher {
    expression_engine: ExpressionEngine,
}

impl RuleMatcher {
    /// Create a new rule matcher
    pub fn new(expression_engine: ExpressionEngine) -> Self {
        Self { expression_engine }
    }

    /// Check if a rule's preconditions match
    pub fn matches(
        &self,
        condition: &CompiledCondition,
        context: &RuleExecutionContext,
    ) -> Result<bool> {
        match condition {
            CompiledCondition::SlotConditions(slot_conditions) => {
                self.match_slot_conditions(slot_conditions, context)
            }
            CompiledCondition::ExpressionConditions(expressions) => {
                self.match_expression_conditions(expressions, context)
            }
            CompiledCondition::Composite(composite) => {
                self.match_composite_condition(composite, context)
            }
            CompiledCondition::Combined {
                slot_conditions,
                expression_conditions,
                composite_conditions,
            } => {
                // All components must match (AND logic)
                if let Some(slots) = slot_conditions {
                    if !self.match_slot_conditions(slots, context)? {
                        return Ok(false);
                    }
                }

                if let Some(exprs) = expression_conditions {
                    if !self.match_expression_conditions(exprs, context)? {
                        return Ok(false);
                    }
                }

                if let Some(composite) = composite_conditions {
                    if !self.match_composite_condition(composite, context)? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
        }
    }

    /// Match slot-based conditions
    fn match_slot_conditions(
        &self,
        conditions: &HashMap<String, CompiledSlotCondition>,
        context: &RuleExecutionContext,
    ) -> Result<bool> {
        let instance_obj = match &context.instance {
            Value::Object(map) => map,
            _ => return Ok(false), // Non-object instances can't match slot conditions
        };

        for (slot_name, condition) in conditions {
            let slot_value = instance_obj.get(slot_name).unwrap_or(&Value::Null);

            if !self.match_slot_condition(slot_value, condition, context)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Match a single slot condition
    fn match_slot_condition(
        &self,
        value: &Value,
        condition: &CompiledSlotCondition,
        context: &RuleExecutionContext,
    ) -> Result<bool> {
        let original = &condition.original;

        // Check required
        if let Some(required) = original.required {
            if required && value.is_null() {
                return Ok(false);
            }
        }

        // Skip further checks if value is null
        if value.is_null() {
            return Ok(true);
        }

        // Check range/type
        if let Some(ref range) = original.range {
            if !self.check_range(value, range)? {
                return Ok(false);
            }
        }

        // Check pattern
        if let Some(ref pattern) = original.pattern {
            if let Value::String(_s) = value {
                let validator = PatternValidator::new();
                let slot_def = linkml_core::types::SlotDefinition {
                    name: "temp".to_string(),
                    pattern: Some(pattern.clone()),
                    ..Default::default()
                };
                let mut validation_context = ValidationContext::new(Default::default());
                let issues = validator.validate(value, &slot_def, &mut validation_context);
                if !issues.is_empty() {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }

        // Check equals_string
        if let Some(ref expected) = original.equals_string {
            if let Value::String(actual) = value {
                if actual != expected {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }

        // Check equals_number
        if let Some(expected) = original.equals_number {
            if let Value::Number(num) = value {
                if let Some(actual) = num.as_f64() {
                    if (actual - expected).abs() > f64::EPSILON {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }

        // Check equals_expression
        if let Some(ref expr_ast) = condition.equals_expression_ast {
            let expr_context = context.get_expression_context();
            let computed = self
                .expression_engine
                .evaluate_ast(expr_ast, &expr_context)?;
            if value != &computed {
                return Ok(false);
            }
        }

        // Check min/max values
        if let Some(ref min) = original.minimum_value {
            if !self.compare_values(value, min, |a, b| a >= b)? {
                return Ok(false);
            }
        }

        if let Some(ref max) = original.maximum_value {
            if !self.compare_values(value, max, |a, b| a <= b)? {
                return Ok(false);
            }
        }

        // TODO: Implement any_of, all_of, exactly_one_of, none_of checks

        Ok(true)
    }

    /// Match expression-based conditions
    fn match_expression_conditions(
        &self,
        expressions: &[crate::expression::ast::Expression],
        context: &RuleExecutionContext,
    ) -> Result<bool> {
        let expr_context = context.get_expression_context();

        for expr in expressions {
            let result = self.expression_engine.evaluate_ast(expr, &expr_context)?;

            match result {
                Value::Bool(false) => return Ok(false),
                Value::Bool(true) => {}
                _ => {
                    return Err(LinkMLError::data_validation(
                        "Rule expression must evaluate to boolean",
                    ));
                }
            }
        }

        Ok(true)
    }

    /// Match composite conditions
    fn match_composite_condition(
        &self,
        composite: &CompiledCompositeCondition,
        context: &RuleExecutionContext,
    ) -> Result<bool> {
        match composite {
            CompiledCompositeCondition::AnyOf(conditions) => {
                for condition in conditions {
                    if self.matches(condition, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            CompiledCompositeCondition::AllOf(conditions) => {
                for condition in conditions {
                    if !self.matches(condition, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            CompiledCompositeCondition::ExactlyOneOf(conditions) => {
                let mut count = 0;
                for condition in conditions {
                    if self.matches(condition, context)? {
                        count += 1;
                        if count > 1 {
                            return Ok(false);
                        }
                    }
                }
                Ok(count == 1)
            }
            CompiledCompositeCondition::NoneOf(conditions) => {
                for condition in conditions {
                    if self.matches(condition, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
        }
    }

    /// Check if a value matches a range/type constraint
    fn check_range(&self, value: &Value, range: &str) -> Result<bool> {
        // Use RangeValidator for consistency
        let validator = RangeValidator::new();
        let slot_def = linkml_core::types::SlotDefinition {
            name: "temp".to_string(),
            range: Some(range.to_string()),
            ..Default::default()
        };
        let mut validation_context = ValidationContext::new(Default::default());
        let issues = validator.validate(value, &slot_def, &mut validation_context);
        Ok(issues.is_empty())
    }

    /// Compare values using a comparison function
    fn compare_values<F>(&self, a: &Value, b: &Value, cmp: F) -> Result<bool>
    where
        F: Fn(f64, f64) -> bool,
    {
        match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                if let (Some(v1), Some(v2)) = (n1.as_f64(), n2.as_f64()) {
                    Ok(cmp(v1, v2))
                } else {
                    Ok(false)
                }
            }
            (Value::String(s1), Value::String(s2)) => {
                // String comparison
                Ok(cmp(s1.len() as f64, s2.len() as f64))
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SlotCondition;
    use serde_json::json;

    #[test]
    fn test_slot_condition_matching() {
        let matcher = RuleMatcher::new(ExpressionEngine::new());

        // Create a condition that checks age >= 18
        let condition = CompiledSlotCondition {
            original: SlotCondition {
                minimum_value: Some(json!(18)),
                ..Default::default()
            },
            equals_expression_ast: None,
        };

        let mut validation_ctx = ValidationContext::new(Default::default());
        let context = RuleExecutionContext::new(
            json!({"age": 20}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        assert!(
            matcher
                .match_slot_condition(&json!(20), &condition, &context)
                .map_err(|e| anyhow::anyhow!("should match age >= 18": {}, e))?
        );
        assert!(
            !matcher
                .match_slot_condition(&json!(16), &condition, &context)
                .map_err(|e| anyhow::anyhow!("should not match age < 18": {}, e))?
        );
    }

    #[test]
    fn test_expression_condition_matching() {
        let matcher = RuleMatcher::new(ExpressionEngine::new());

        // Parse expression
        let parser = crate::expression::parser::Parser::new();
        let expr = parser
            .parse("{age} >= 18 and {status} == \"active\"")
            .map_err(|e| anyhow::anyhow!("should parse expression": {}, e))?;

        let mut validation_ctx = ValidationContext::new(Default::default());
        let context = RuleExecutionContext::new(
            json!({"age": 20, "status": "active"}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        assert!(
            matcher
                .match_expression_conditions(&[expr.clone()], &context)
                .map_err(|e| anyhow::anyhow!("should match expression": {}, e))?
        );

        let mut validation_ctx2 = ValidationContext::new(Default::default());
        let context2 = RuleExecutionContext::new(
            json!({"age": 16, "status": "active"}),
            "Person".to_string(),
            &mut validation_ctx2,
        );

        assert!(
            !matcher
                .match_expression_conditions(&[expr], &context2)
                .map_err(|e| anyhow::anyhow!("should not match expression": {}, e))?
        );
    }
}
