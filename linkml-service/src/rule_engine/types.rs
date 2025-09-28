//! Internal types for the rule engine
//!
//! This module defines internal representations and compiled forms of rules
//! for efficient evaluation.

use linkml_core::error::LinkMLError;
use linkml_core::types::{CompositeConditions, Rule, RuleConditions, SlotCondition};
use serde_json::Value;
use std::collections::HashMap;

use crate::expression::ast::Expression;
use crate::validator::context::ValidationContext;

/// Compiled form of a rule for efficient evaluation
#[derive(Debug, Clone)]
pub struct CompiledRule {
    /// Original rule definition
    pub original: Rule,
    /// Compiled precondition AST
    pub precondition_ast: Option<CompiledCondition>,
    /// Compiled postcondition AST
    pub postcondition_ast: Option<CompiledCondition>,
    /// Compiled else condition AST
    pub else_condition_ast: Option<CompiledCondition>,
    /// Effective priority (inherited rules may have adjusted priority)
    pub priority: i32,
    /// Source class (for debugging and error messages)
    pub source_class: String,
    /// Whether this rule is deactivated
    pub deactivated: bool,
}

impl CompiledRule {
    /// Compile a rule from its definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compile(rule: Rule, source_class: String) -> linkml_core::error::Result<Self> {
        let priority = rule.priority.unwrap_or(0);
        let deactivated = rule.deactivated.unwrap_or(false);

        let precondition_ast = if let Some(ref conditions) = rule.preconditions {
            Some(CompiledCondition::compile(conditions)?)
        } else {
            None
        };

        let postcondition_ast = if let Some(ref conditions) = rule.postconditions {
            Some(CompiledCondition::compile(conditions)?)
        } else {
            None
        };

        let else_condition_ast = if let Some(ref conditions) = rule.else_conditions {
            Some(CompiledCondition::compile(conditions)?)
        } else {
            None
        };

        Ok(Self {
            original: rule,
            precondition_ast,
            postcondition_ast,
            else_condition_ast,
            priority,
            source_class,
            deactivated,
        })
    }
}

/// Compiled condition for efficient evaluation
#[derive(Debug, Clone)]
pub enum CompiledCondition {
    /// Slot-based conditions
    SlotConditions(HashMap<String, CompiledSlotCondition>),
    /// Expression-based conditions
    ExpressionConditions(Vec<Expression>),
    /// Composite conditions
    Composite(CompiledCompositeCondition),
    /// Combined conditions
    Combined {
        /// Slot-based conditions
        slot_conditions: Option<HashMap<String, CompiledSlotCondition>>,
        /// Expression-based conditions
        expression_conditions: Option<Vec<Expression>>,
        /// Composite conditions
        composite_conditions: Option<Box<CompiledCompositeCondition>>,
    },
}

impl CompiledCondition {
    /// Compile conditions from their definition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compile(conditions: &RuleConditions) -> linkml_core::error::Result<Self> {
        let has_slots = conditions
            .slot_conditions
            .as_ref()
            .is_some_and(|sc| !sc.is_empty());
        let has_exprs = conditions
            .expression_conditions
            .as_ref()
            .is_some_and(|ec| !ec.is_empty());
        let has_composite = conditions.composite_conditions.is_some();

        match (has_slots, has_exprs, has_composite) {
            (true, false, false) => {
                let slot_conditions = conditions.slot_conditions.as_ref().ok_or_else(|| {
                    LinkMLError::service("Rule error: Slot conditions expected but not found")
                })?;
                let mut compiled = HashMap::new();
                for (slot_name, condition) in slot_conditions {
                    compiled.insert(
                        slot_name.clone(),
                        CompiledSlotCondition::compile(condition)?,
                    );
                }
                Ok(CompiledCondition::SlotConditions(compiled))
            }
            (false, true, false) => {
                let expressions = conditions.expression_conditions.as_ref().ok_or_else(|| {
                    LinkMLError::service("Rule error: Expression conditions expected but not found")
                })?;
                let mut compiled = Vec::new();
                for expr_str in expressions {
                    let parser = crate::expression::parser::Parser::new();
                    compiled.push(
                        parser
                            .parse(expr_str)
                            .map_err(|e| LinkMLError::ParseError {
                                message: format!("Failed to parse expression '{expr_str}': {e}"),
                                location: None,
                            })?,
                    );
                }
                Ok(CompiledCondition::ExpressionConditions(compiled))
            }
            (false, false, true) => {
                let composite = conditions.composite_conditions.as_ref().ok_or_else(|| {
                    LinkMLError::service("Rule error: Composite conditions expected but not found")
                })?;
                Ok(CompiledCondition::Composite(
                    CompiledCompositeCondition::compile(composite)?,
                ))
            }
            _ => {
                // Combined conditions
                let slot_conditions = if has_slots {
                    let slot_conditions = conditions.slot_conditions.as_ref().ok_or_else(|| {
                        LinkMLError::service("Rule error: Slot conditions expected but not found")
                    })?;
                    let mut compiled = HashMap::new();
                    for (slot_name, condition) in slot_conditions {
                        compiled.insert(
                            slot_name.clone(),
                            CompiledSlotCondition::compile(condition)?,
                        );
                    }
                    Some(compiled)
                } else {
                    None
                };

                let expression_conditions = if has_exprs {
                    let expressions =
                        conditions.expression_conditions.as_ref().ok_or_else(|| {
                            LinkMLError::service(
                                "Rule error: Expression conditions expected but not found",
                            )
                        })?;
                    let mut compiled = Vec::new();
                    for expr_str in expressions {
                        let parser = crate::expression::parser::Parser::new();
                        compiled.push(parser.parse(expr_str).map_err(|e| {
                            LinkMLError::ParseError {
                                message: format!("Failed to parse expression '{expr_str}': {e}"),
                                location: None,
                            }
                        })?);
                    }
                    Some(compiled)
                } else {
                    None
                };

                let composite_conditions = if has_composite {
                    let composite = conditions.composite_conditions.as_ref().ok_or_else(|| {
                        LinkMLError::service(
                            "Rule error: Composite conditions expected but not found",
                        )
                    })?;
                    Some(Box::new(CompiledCompositeCondition::compile(composite)?))
                } else {
                    None
                };

                Ok(CompiledCondition::Combined {
                    slot_conditions,
                    expression_conditions,
                    composite_conditions,
                })
            }
        }
    }
}

/// Compiled slot condition
#[derive(Debug, Clone)]
pub struct CompiledSlotCondition {
    /// Original slot condition
    pub original: SlotCondition,
    /// Compiled expression for `equals_expression`
    pub equals_expression_ast: Option<Expression>,
}

impl CompiledSlotCondition {
    /// Compile a slot condition
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compile(condition: &SlotCondition) -> linkml_core::error::Result<Self> {
        let equals_expression_ast = if let Some(ref expr_str) = condition.equals_expression {
            let parser = crate::expression::parser::Parser::new();
            Some(
                parser
                    .parse(expr_str)
                    .map_err(|e| LinkMLError::ParseError {
                        message: format!("Failed to parse expression '{expr_str}': {e}"),
                        location: None,
                    })?,
            )
        } else {
            None
        };

        Ok(Self {
            original: condition.clone(),
            equals_expression_ast,
        })
    }
}

/// Compiled composite condition
#[derive(Debug, Clone)]
pub enum CompiledCompositeCondition {
    /// At least one condition must be true
    AnyOf(Vec<CompiledCondition>),
    /// All conditions must be true
    AllOf(Vec<CompiledCondition>),
    /// Exactly one condition must be true
    ExactlyOneOf(Vec<CompiledCondition>),
    /// No conditions can be true
    NoneOf(Vec<CompiledCondition>),
}

impl CompiledCompositeCondition {
    /// Compile composite conditions
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compile(conditions: &CompositeConditions) -> linkml_core::error::Result<Self> {
        if let Some(ref any_of) = conditions.any_of {
            let mut compiled = Vec::new();
            for condition in any_of {
                compiled.push(CompiledCondition::compile(condition)?);
            }
            Ok(CompiledCompositeCondition::AnyOf(compiled))
        } else if let Some(ref all_of) = conditions.all_of {
            let mut compiled = Vec::new();
            for condition in all_of {
                compiled.push(CompiledCondition::compile(condition)?);
            }
            Ok(CompiledCompositeCondition::AllOf(compiled))
        } else if let Some(ref exactly_one_of) = conditions.exactly_one_of {
            let mut compiled = Vec::new();
            for condition in exactly_one_of {
                compiled.push(CompiledCondition::compile(condition)?);
            }
            Ok(CompiledCompositeCondition::ExactlyOneOf(compiled))
        } else if let Some(ref none_of) = conditions.none_of {
            let mut compiled = Vec::new();
            for condition in none_of {
                compiled.push(CompiledCondition::compile(condition)?);
            }
            Ok(CompiledCompositeCondition::NoneOf(compiled))
        } else {
            Err(LinkMLError::schema_validation(
                "CompositeConditions must have at least one condition type",
            ))
        }
    }
}

/// Context for rule execution
pub struct RuleExecutionContext<'a> {
    /// Instance being validated
    pub instance: Value,
    /// Class name being validated
    pub class_name: String,
    /// Validation context
    pub validation_context: &'a mut ValidationContext,
    /// Rules that have already been matched
    pub matched_rules: Vec<String>,
    /// Current rule being evaluated (for recursion detection)
    pub current_rule: Option<String>,
}

impl<'a> RuleExecutionContext<'a> {
    /// Create a new execution context
    pub fn new(
        instance: Value,
        class_name: String,
        validation_context: &'a mut ValidationContext,
    ) -> Self {
        Self {
            instance,
            class_name,
            validation_context,
            matched_rules: Vec::new(),
            current_rule: None,
        }
    }

    /// Mark a rule as matched
    pub fn mark_matched(&mut self, rule_desc: String) {
        self.matched_rules.push(rule_desc);
    }

    /// Get expression evaluation context
    #[must_use]
    pub fn get_expression_context(&self) -> HashMap<String, Value> {
        let mut context = HashMap::new();

        // First, add all slots for the class with null defaults
        let effective_slots = self
            .validation_context
            .get_effective_slots(&self.class_name);
        for (slot_name, _slot_def) in effective_slots {
            context.insert(slot_name.to_string(), Value::Null);
        }

        // Then override with actual instance data
        if let Value::Object(map) = &self.instance {
            for (key, value) in map {
                context.insert(key.clone(), value.clone());
            }
        }

        // Add special variables
        context.insert("_instance".to_string(), self.instance.clone());
        context.insert("_class".to_string(), Value::String(self.class_name.clone()));

        // Add parent/root from validation context
        if let Some(parent) = self.validation_context.parent() {
            context.insert("parent".to_string(), parent.clone());
        }

        if let Some(root) = self.validation_context.root() {
            context.insert("root".to_string(), root.clone());
        }

        context
    }
}

/// Rule execution strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleExecutionStrategy {
    /// Execute rules sequentially by priority
    Sequential,
    /// Execute independent rules in parallel
    Parallel,
    /// Stop on first failure
    FailFast,
    /// Collect all failures
    CollectAll,
}

impl Default for RuleExecutionStrategy {
    fn default() -> Self {
        Self::Sequential
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiled_rule_creation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let rule = Rule {
            description: Some("Test rule".to_string()),
            priority: Some(10),
            ..Default::default()
        };

        let compiled = CompiledRule::compile(rule, "TestClass".to_string())?;
        assert_eq!(compiled.priority, 10);
        assert_eq!(compiled.source_class, "TestClass");
        assert!(!compiled.deactivated);
        Ok(())
    }

    #[test]
    fn test_execution_context() {
        let instance = serde_json::json!({
            "name": "test",
            "value": 42
        });

        let mut validation_ctx = ValidationContext::new(Default::default());
        let ctx = RuleExecutionContext::new(
            instance.clone(),
            "TestClass".to_string(),
            &mut validation_ctx,
        );

        let expr_ctx = ctx.get_expression_context();
        assert_eq!(expr_ctx.get("name"), Some(&serde_json::json!("test")));
        assert_eq!(expr_ctx.get("value"), Some(&serde_json::json!(42)));
        assert_eq!(
            expr_ctx.get("_class"),
            Some(&serde_json::json!("TestClass"))
        );
    }
}
