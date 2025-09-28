//! Conditional validation rules for `LinkML`
//!
//! This module implements if/then conditional requirements,
//! allowing complex validation logic based on field values.

use crate::expression::ExpressionEngine;
use linkml_core::prelude::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;

/// Conditional validator for if/then rules
pub struct ConditionalValidator {
    /// Rules by class name
    rules: HashMap<String, Vec<ConditionalRule>>,

    /// Expression engine for evaluating complex conditions
    expression_engine: Arc<ExpressionEngine>,
}

/// A conditional validation rule
#[derive(Clone, Debug)]
pub struct ConditionalRule {
    /// Name of the rule
    pub name: String,

    /// Condition to evaluate (if)
    pub condition: Condition,

    /// Requirements when condition is true (then)
    pub then_requirements: Vec<Requirement>,

    /// Requirements when condition is false (else)
    pub else_requirements: Option<Vec<Requirement>>,

    /// Error message when rule is violated
    pub message: Option<String>,
}

/// A condition that can be evaluated
#[derive(Clone, Debug)]
pub enum Condition {
    /// Slot has a specific value
    Equals {
        /// Name of the slot to check for equality
        slot: String,
        /// Expected value that the slot must equal for condition to be true
        value: Value,
    },

    /// Slot does not have a specific value
    NotEquals {
        /// Name of the slot to check for inequality
        slot: String,
        /// Value that the slot must not equal for condition to be true
        value: Value,
    },

    /// Slot value is in a set
    In {
        /// Name of the slot to check for membership
        slot: String,
        /// Set of valid values that the slot can contain
        values: Vec<Value>,
    },

    /// Slot value is not in a set
    NotIn {
        /// Name of the slot to check for exclusion
        slot: String,
        /// Set of forbidden values that the slot cannot contain
        values: Vec<Value>,
    },

    /// Slot has any value (not null)
    Present {
        /// Name of the slot that must be present and non-null
        slot: String,
    },

    /// Slot is null or missing
    Absent {
        /// Name of the slot that must be absent or null
        slot: String,
    },

    /// Slot matches a pattern
    Matches {
        /// Name of the slot containing text to match against pattern
        slot: String,
        /// Regular expression pattern that the slot value must match
        pattern: String,
    },

    /// Numeric comparison
    GreaterThan {
        /// Name of the slot containing numeric value to compare
        slot: String,
        /// Threshold value that the slot must exceed
        value: f64,
    },
    /// Less than comparison
    LessThan {
        /// Name of the slot containing numeric value to compare
        slot: String,
        /// Threshold value that the slot must be below
        value: f64,
    },

    /// Logical combinations
    /// All conditions must be true
    And(Vec<Condition>),
    /// At least one condition must be true
    Or(Vec<Condition>),
    /// Logical negation - condition is true if inner condition is false
    Not(Box<Condition>),

    /// Expression-based condition
    Expression(String),
}

/// A requirement that must be satisfied
#[derive(Clone, Debug)]
pub enum Requirement {
    /// Slot must be present
    Required {
        /// Name of the slot that must be present and non-null
        slot: String,
    },

    /// Slot must be absent
    Forbidden {
        /// Name of the slot that must be absent or null
        slot: String,
    },

    /// Slot must have specific value
    MustEqual {
        /// Name of the slot that must equal the specified value
        slot: String,
        /// Required value that the slot must contain
        value: Value,
    },

    /// Slot must match pattern
    MustMatch {
        /// Name of the slot containing text that must match pattern
        slot: String,
        /// Regular expression pattern that the slot value must satisfy
        pattern: String,
    },

    /// Custom validation expression
    Expression(String),
}

impl Default for ConditionalValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalValidator {
    /// Create a new conditional validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
            expression_engine: Arc::new(ExpressionEngine::new()),
        }
    }

    /// Create from `LinkML` schema
    #[must_use]
    pub fn from_schema(schema: &SchemaDefinition) -> Self {
        let mut validator = Self::new();

        // Extract rules from classes
        for (class_name, class_def) in &schema.classes {
            let mut class_rules = Vec::new();

            // Process conditional requirements from ClassDefinition.if_required
            if let Some(if_required_map) = &class_def.if_required {
                for (trigger_slot, conditional_req) in if_required_map {
                    if let Some(condition) = &conditional_req.condition
                        && let Some(then_required) = &conditional_req.then_required
                    {
                        // Convert SlotCondition to our Condition enum
                        let our_condition = if condition.required == Some(true) {
                            Condition::Present {
                                slot: trigger_slot.clone(),
                            }
                        } else if let Some(equals_string) = &condition.equals_string {
                            Condition::Equals {
                                slot: trigger_slot.clone(),
                                value: json!(equals_string),
                            }
                        } else if let Some(equals_number) = &condition.equals_number {
                            Condition::Equals {
                                slot: trigger_slot.clone(),
                                value: json!(equals_number),
                            }
                        } else {
                            Condition::Present {
                                slot: trigger_slot.clone(),
                            }
                        };

                        let rule = ConditionalRule {
                            name: format!("{trigger_slot}_conditional_requirement"),
                            condition: our_condition,
                            then_requirements: then_required
                                .iter()
                                .map(|s| Requirement::Required { slot: s.clone() })
                                .collect(),
                            else_requirements: None,
                            message: Some(format!(
                                "When '{}' meets condition, the following fields are required: {}",
                                trigger_slot,
                                then_required.join(", ")
                            )),
                        };
                        class_rules.push(rule);
                    }
                }
            }

            // Process explicit rules from ClassDefinition.rules
            for rule in &class_def.rules {
                if let Some(parsed) = Self::parse_rule(rule) {
                    class_rules.push(parsed);
                }
            }

            if !class_rules.is_empty() {
                validator.rules.insert(class_name.clone(), class_rules);
            }
        }

        validator
    }

    /// Parse a rule from ClassDefinition.rules into a `ConditionalRule`
    fn parse_rule(rule: &linkml_core::types::Rule) -> Option<ConditionalRule> {
        // Convert Rule to ConditionalRule
        if let Some(preconditions) = &rule.preconditions {
            if let Some(postconditions) = &rule.postconditions {
                // Extract condition from preconditions
                let condition = if let Some(slot_conditions) = &preconditions.slot_conditions {
                    // Use first slot condition as the trigger
                    if let Some((slot_name, slot_condition)) = slot_conditions.iter().next() {
                        if slot_condition.required == Some(true) {
                            Condition::Present {
                                slot: slot_name.clone(),
                            }
                        } else if let Some(equals_string) = &slot_condition.equals_string {
                            Condition::Equals {
                                slot: slot_name.clone(),
                                value: json!(equals_string),
                            }
                        } else if let Some(equals_number) = &slot_condition.equals_number {
                            Condition::Equals {
                                slot: slot_name.clone(),
                                value: json!(equals_number),
                            }
                        } else {
                            Condition::Present {
                                slot: slot_name.clone(),
                            }
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                };

                // Extract requirements from postconditions
                let mut then_requirements = Vec::new();
                if let Some(slot_conditions) = &postconditions.slot_conditions {
                    for (slot_name, slot_condition) in slot_conditions {
                        if slot_condition.required == Some(true) {
                            then_requirements.push(Requirement::Required {
                                slot: slot_name.clone(),
                            });
                        }
                    }
                }

                // Handle else conditions if present
                let else_requirements = rule.else_conditions.as_ref().and_then(|else_conds| {
                    if let Some(slot_conditions) = &else_conds.slot_conditions {
                        let mut else_reqs = Vec::new();
                        for (slot_name, slot_condition) in slot_conditions {
                            if slot_condition.required == Some(true) {
                                else_reqs.push(Requirement::Required {
                                    slot: slot_name.clone(),
                                });
                            }
                        }
                        if else_reqs.is_empty() {
                            None
                        } else {
                            Some(else_reqs)
                        }
                    } else {
                        None
                    }
                });

                Some(ConditionalRule {
                    name: rule
                        .title
                        .clone()
                        .unwrap_or_else(|| "unnamed_rule".to_string()),
                    condition,
                    then_requirements,
                    else_requirements,
                    message: rule.description.clone(),
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Add a conditional rule
    pub fn add_rule(&mut self, class_name: &str, rule: ConditionalRule) {
        self.rules
            .entry(class_name.to_string())
            .or_default()
            .push(rule);
    }

    /// Validate an instance against conditional rules
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn validate(
        &self,
        instance: &Value,
        class_name: &str,
    ) -> Result<Vec<ConditionalViolation>> {
        let mut violations = Vec::new();

        if let Some(rules) = self.rules.get(class_name) {
            for rule in rules {
                if let Some(violation) = self.check_rule(instance, rule)? {
                    violations.push(violation);
                }
            }
        }

        Ok(violations)
    }

    /// Check a single rule
    fn check_rule(
        &self,
        instance: &Value,
        rule: &ConditionalRule,
    ) -> Result<Option<ConditionalViolation>> {
        // Evaluate condition
        let condition_met = self.evaluate_condition(instance, &rule.condition)?;

        // Check requirements based on condition result
        let requirements = if condition_met {
            &rule.then_requirements
        } else if let Some(else_reqs) = &rule.else_requirements {
            else_reqs
        } else {
            // No else requirements, rule passes
            return Ok(None);
        };

        // Check each requirement
        let mut failed_requirements = Vec::new();
        for requirement in requirements {
            if !self.check_requirement(instance, requirement)? {
                failed_requirements.push(requirement.clone());
            }
        }

        if failed_requirements.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ConditionalViolation {
                rule_name: rule.name.clone(),
                condition: rule.condition.clone(),
                condition_met,
                failed_requirements,
                message: rule.message.clone(),
            }))
        }
    }

    /// Evaluate a condition
    fn evaluate_condition(&self, instance: &Value, condition: &Condition) -> Result<bool> {
        let Value::Object(obj) = instance else {
            return Ok(false);
        };

        match condition {
            Condition::Equals { slot, value } => Ok(obj.get(slot) == Some(value)),
            Condition::NotEquals { slot, value } => Ok(obj.get(slot) != Some(value)),
            Condition::In { slot, values } => {
                if let Some(v) = obj.get(slot) {
                    Ok(values.contains(v))
                } else {
                    Ok(false)
                }
            }
            Condition::NotIn { slot, values } => {
                if let Some(v) = obj.get(slot) {
                    Ok(!values.contains(v))
                } else {
                    Ok(true)
                }
            }
            Condition::Present { slot } => {
                Ok(obj.get(slot).is_some() && obj.get(slot) != Some(&Value::Null))
            }
            Condition::Absent { slot } => {
                Ok(obj.get(slot).is_none() || obj.get(slot) == Some(&Value::Null))
            }
            Condition::Matches { slot, pattern } => {
                if let Some(Value::String(s)) = obj.get(slot) {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| LinkMLError::service(format!("Invalid pattern: {e}")))?;
                    Ok(re.is_match(s))
                } else {
                    Ok(false)
                }
            }
            Condition::GreaterThan { slot, value } => {
                if let Some(Value::Number(n)) = obj.get(slot) {
                    Ok(n.as_f64().unwrap_or(0.0) > *value)
                } else {
                    Ok(false)
                }
            }
            Condition::LessThan { slot, value } => {
                if let Some(Value::Number(n)) = obj.get(slot) {
                    Ok(n.as_f64().unwrap_or(0.0) < *value)
                } else {
                    Ok(false)
                }
            }
            Condition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_condition(instance, cond)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            Condition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_condition(instance, cond)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            Condition::Not(cond) => Ok(!self.evaluate_condition(instance, cond)?),
            Condition::Expression(expr) => {
                // Use the expression engine to evaluate complex conditions
                // Convert instance to context map
                let context = Self::value_to_context(instance);
                self.expression_engine
                    .evaluate(expr, &context)
                    .map(|result| matches!(result, Value::Bool(true)))
            }
        }
    }

    /// Check if a requirement is satisfied
    fn check_requirement(&self, instance: &Value, requirement: &Requirement) -> Result<bool> {
        let Value::Object(obj) = instance else {
            return Ok(false);
        };

        match requirement {
            Requirement::Required { slot } => {
                Ok(obj.get(slot).is_some() && obj.get(slot) != Some(&Value::Null))
            }
            Requirement::Forbidden { slot } => {
                Ok(obj.get(slot).is_none() || obj.get(slot) == Some(&Value::Null))
            }
            Requirement::MustEqual { slot, value } => Ok(obj.get(slot) == Some(value)),
            Requirement::MustMatch { slot, pattern } => {
                if let Some(Value::String(s)) = obj.get(slot) {
                    let re = regex::Regex::new(pattern)
                        .map_err(|e| LinkMLError::service(format!("Invalid pattern: {e}")))?;
                    Ok(re.is_match(s))
                } else {
                    Ok(false)
                }
            }
            Requirement::Expression(expr) => {
                // Use the expression engine to evaluate complex requirements
                // Convert instance to context map
                let context = Self::value_to_context(instance);
                self.expression_engine
                    .evaluate(expr, &context)
                    .map(|result| matches!(result, Value::Bool(true)))
            }
        }
    }

    /// Convert a `JSON` Value to a context map for expression evaluation
    fn value_to_context(value: &Value) -> HashMap<String, Value> {
        if let Value::Object(map) = value {
            // Convert the serde_json::Map to a HashMap
            let mut context = HashMap::new();
            for (key, val) in map {
                context.insert(key.clone(), val.clone());
            }
            context
        } else {
            // For non-object values, create a context with a single "value" key
            let mut context = HashMap::new();
            context.insert("value".to_string(), value.clone());
            context
        }
    }
}

/// A conditional rule violation
#[derive(Debug, Clone)]
pub struct ConditionalViolation {
    /// Name of the violated rule
    pub rule_name: String,

    /// The condition that was evaluated when this violation occurred
    pub condition: Condition,

    /// Whether the condition was met
    pub condition_met: bool,

    /// Requirements that failed
    pub failed_requirements: Vec<Requirement>,

    /// Custom error message
    pub message: Option<String>,
}

impl ConditionalViolation {
    /// Format as user-friendly message
    #[must_use]
    pub fn format_message(&self) -> String {
        if let Some(msg) = &self.message {
            msg.clone()
        } else {
            format!(
                "Conditional rule '{}' violated: condition was {} but requirements were not met",
                self.rule_name,
                if self.condition_met { "true" } else { "false" }
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_conditional() -> anyhow::Result<()> {
        let mut validator = ConditionalValidator::new();

        // Rule: if country is "US", then state is required
        validator.add_rule(
            "Address",
            ConditionalRule {
                name: "us_requires_state".to_string(),
                condition: Condition::Equals {
                    slot: "country".to_string(),
                    value: json!("US"),
                },
                then_requirements: vec![Requirement::Required {
                    slot: "state".to_string(),
                }],
                else_requirements: None,
                message: Some("US addresses require a state".to_string()),
            },
        );

        // Valid US address
        let valid_us = json!({
            "country": "US",
            "state": "CA",
            "city": "San Francisco"
        });
        assert!(validator.validate(&valid_us, "Address")?.is_empty());

        // Invalid US address (missing state)
        let invalid_us = json!({
            "country": "US",
            "city": "San Francisco"
        });
        let violations = validator.validate(&invalid_us, "Address")?;
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_name, "us_requires_state");

        // Valid non-US address (state not required)
        let valid_other = json!({
            "country": "UK",
            "city": "London"
        });
        assert!(validator.validate(&valid_other, "Address")?.is_empty());
        Ok(())
    }

    #[test]
    fn test_complex_conditions() -> anyhow::Result<()> {
        let mut validator = ConditionalValidator::new();

        // Rule: if age >= 18 AND country = "US", then ssn is required
        validator.add_rule(
            "Person",
            ConditionalRule {
                name: "adult_us_requires_ssn".to_string(),
                condition: Condition::And(vec![
                    Condition::GreaterThan {
                        slot: "age".to_string(),
                        value: 17.99,
                    },
                    Condition::Equals {
                        slot: "country".to_string(),
                        value: json!("US"),
                    },
                ]),
                then_requirements: vec![Requirement::Required {
                    slot: "ssn".to_string(),
                }],
                else_requirements: None,
                message: Some("US adults require SSN".to_string()),
            },
        );

        // US adult without SSN - should fail
        let invalid = json!({
            "age": 25,
            "country": "US",
            "name": "John"
        });
        assert!(!validator.validate(&invalid, "Person")?.is_empty());

        // US minor without SSN - should pass
        let valid_minor = json!({
            "age": 16,
            "country": "US",
            "name": "Jane"
        });
        assert!(validator.validate(&valid_minor, "Person")?.is_empty());

        // Non-US adult without SSN - should pass
        let valid_foreign = json!({
            "age": 30,
            "country": "Canada",
            "name": "Bob"
        });
        assert!(validator.validate(&valid_foreign, "Person")?.is_empty());
        Ok(())
    }
}
