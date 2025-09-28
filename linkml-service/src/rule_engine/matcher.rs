//! Rule matching logic for precondition evaluation
//!
//! This module handles determining whether a rule's preconditions are satisfied
//! for a given instance.

use linkml_core::error::LinkMLError;
use linkml_core::types::{SlotCondition, SlotDefinition};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

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
    #[must_use]
    pub fn new(expression_engine: ExpressionEngine) -> Self {
        Self { expression_engine }
    }

    /// Check if a rule's preconditions match
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn matches(
        &self,
        condition: &CompiledCondition,
        context: &RuleExecutionContext,
    ) -> linkml_core::error::Result<bool> {
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
                if let Some(slots) = slot_conditions
                    && !self.match_slot_conditions(slots, context)?
                {
                    return Ok(false);
                }

                if let Some(exprs) = expression_conditions
                    && !self.match_expression_conditions(exprs, context)?
                {
                    return Ok(false);
                }

                if let Some(composite) = composite_conditions
                    && !self.match_composite_condition(composite, context)?
                {
                    return Ok(false);
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
    ) -> linkml_core::error::Result<bool> {
        let Value::Object(instance_obj) = &context.instance else {
            return Ok(false);
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
    ) -> linkml_core::error::Result<bool> {
        let original = &condition.original;

        // Check required
        if let Some(required) = original.required
            && required
            && value.is_null()
        {
            return Ok(false);
        }

        // Skip further checks if value is null
        if value.is_null() {
            return Ok(true);
        }

        // Check range/type
        if let Some(ref range) = original.range
            && !self.check_range(value, range)?
        {
            return Ok(false);
        }

        // Check pattern
        if let Some(ref pattern) = original.pattern {
            if let Value::String(_s) = value {
                let validator = PatternValidator::new();
                let slot_def = SlotDefinition {
                    name: "temp".to_string(),
                    pattern: Some(pattern.clone()),
                    ..SlotDefinition::default()
                };
                let mut validation_context = ValidationContext::new(Arc::default());
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
        if let Some(ref min) = original.minimum_value
            && !self.compare_values(value, min, |a, b| a >= b)?
        {
            return Ok(false);
        }

        if let Some(ref max) = original.maximum_value
            && !self.compare_values(value, max, |a, b| a <= b)?
        {
            return Ok(false);
        }

        // Check any_of constraint (at least one must match)
        if let Some(ref any_of) = original.any_of {
            let mut any_matched = false;
            for condition in any_of {
                // Create a temporary SlotCondition from AnonymousSlotExpression
                let temp_condition = SlotCondition {
                    range: condition.range.clone(),
                    required: condition.required,
                    pattern: condition.pattern.clone(),
                    equals_string: None,
                    equals_number: None,
                    equals_expression: None,
                    minimum_value: condition.minimum_value.clone(),
                    maximum_value: condition.maximum_value.clone(),
                    any_of: None,
                    all_of: None,
                    exactly_one_of: None,
                    none_of: None,
                };

                // Compile and check the condition
                let compiled = CompiledSlotCondition::compile(&temp_condition)?;
                if self.match_slot_condition(value, &compiled, context)? {
                    any_matched = true;
                    break;
                }
            }
            if !any_matched {
                return Ok(false);
            }
        }

        // Check all_of constraint (all must match)
        if let Some(ref all_of) = original.all_of {
            for condition in all_of {
                // Create a temporary SlotCondition from AnonymousSlotExpression
                let temp_condition = SlotCondition {
                    range: condition.range.clone(),
                    required: condition.required,
                    pattern: condition.pattern.clone(),
                    equals_string: None,
                    equals_number: None,
                    equals_expression: None,
                    minimum_value: condition.minimum_value.clone(),
                    maximum_value: condition.maximum_value.clone(),
                    any_of: None,
                    all_of: None,
                    exactly_one_of: None,
                    none_of: None,
                };

                // Compile and check the condition
                let compiled = CompiledSlotCondition::compile(&temp_condition)?;
                if !self.match_slot_condition(value, &compiled, context)? {
                    return Ok(false);
                }
            }
        }

        // Check exactly_one_of constraint (exactly one must match)
        if let Some(ref exactly_one) = original.exactly_one_of {
            let mut match_count = 0;
            for condition in exactly_one {
                // Create a temporary SlotCondition from AnonymousSlotExpression
                let temp_condition = SlotCondition {
                    range: condition.range.clone(),
                    required: condition.required,
                    pattern: condition.pattern.clone(),
                    equals_string: None,
                    equals_number: None,
                    equals_expression: None,
                    minimum_value: condition.minimum_value.clone(),
                    maximum_value: condition.maximum_value.clone(),
                    any_of: None,
                    all_of: None,
                    exactly_one_of: None,
                    none_of: None,
                };

                // Compile and check the condition
                let compiled = CompiledSlotCondition::compile(&temp_condition)?;
                if self.match_slot_condition(value, &compiled, context)? {
                    match_count += 1;
                    if match_count > 1 {
                        return Ok(false); // More than one matched
                    }
                }
            }
            if match_count != 1 {
                return Ok(false); // Either none or more than one matched
            }
        }

        // Check none_of constraint (none must match)
        if let Some(ref none_of) = original.none_of {
            for condition in none_of {
                // Create a temporary SlotCondition from AnonymousSlotExpression
                let temp_condition = SlotCondition {
                    range: condition.range.clone(),
                    required: condition.required,
                    pattern: condition.pattern.clone(),
                    equals_string: None,
                    equals_number: None,
                    equals_expression: None,
                    minimum_value: condition.minimum_value.clone(),
                    maximum_value: condition.maximum_value.clone(),
                    any_of: None,
                    all_of: None,
                    exactly_one_of: None,
                    none_of: None,
                };

                // Compile and check the condition
                let compiled = CompiledSlotCondition::compile(&temp_condition)?;
                if self.match_slot_condition(value, &compiled, context)? {
                    return Ok(false); // One matched when none should
                }
            }
        }

        Ok(true)
    }

    /// Match expression-based conditions
    fn match_expression_conditions(
        &self,
        expressions: &[crate::expression::ast::Expression],
        context: &RuleExecutionContext,
    ) -> linkml_core::error::Result<bool> {
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
    ) -> linkml_core::error::Result<bool> {
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
    fn check_range(&self, value: &Value, range: &str) -> linkml_core::error::Result<bool> {
        // Use RangeValidator for consistency
        let validator = RangeValidator::new();
        let slot_def = SlotDefinition {
            name: "temp".to_string(),
            range: Some(range.to_string()),
            ..SlotDefinition::default()
        };
        let mut validation_context = ValidationContext::new(Arc::default());
        let issues = validator.validate(value, &slot_def, &mut validation_context);
        Ok(issues.is_empty())
    }

    /// Compare values using a comparison function
    fn compare_values<F>(&self, a: &Value, b: &Value, cmp: F) -> linkml_core::error::Result<bool>
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
    fn test_slot_condition_matching() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let matcher = RuleMatcher::new(ExpressionEngine::new());

        // Create a condition that checks age >= 18
        let condition = CompiledSlotCondition {
            original: SlotCondition {
                minimum_value: Some(json!(18)),
                ..SlotCondition::default()
            },
            equals_expression_ast: None,
        };

        let mut validation_ctx = ValidationContext::new(Arc::default());
        let context = RuleExecutionContext::new(
            json!({"age": 20}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        assert!(
            matcher
                .match_slot_condition(&json!(20), &condition, &context)
                .expect("should match age >= 18: {}")
        );
        assert!(
            !matcher
                .match_slot_condition(&json!(16), &condition, &context)
                .expect("should not match age < 18: {}")
        );
        Ok(())
    }

    #[test]
    fn test_expression_condition_matching() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let matcher = RuleMatcher::new(ExpressionEngine::new());

        // Parse expression
        let parser = crate::expression::parser::Parser::new();
        let expr = parser
            .parse("{age} >= 18 and {status} == \"active\"")
            .expect("should parse expression: {}");

        let mut validation_ctx = ValidationContext::new(Arc::default());
        let context = RuleExecutionContext::new(
            json!({"age": 20, "status": "active"}),
            "Person".to_string(),
            &mut validation_ctx,
        );

        assert!(
            matcher
                .match_expression_conditions(&[expr.clone()], &context)
                .expect("should match expression: {}")
        );

        let mut validation_ctx2 = ValidationContext::new(Arc::default());
        let context2 = RuleExecutionContext::new(
            json!({"age": 16, "status": "active"}),
            "Person".to_string(),
            &mut validation_ctx2,
        );

        assert!(
            !matcher
                .match_expression_conditions(&[expr], &context2)
                .expect("should not match expression: {}")
        );
        Ok(())
    }
}
