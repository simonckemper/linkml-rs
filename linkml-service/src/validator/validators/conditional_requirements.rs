//! Conditional requirement validators for `LinkML`
//!
//! This module implements validators for `if_required/then_required` conditional logic.

use serde_json::Value;

use crate::validator::{context::ValidationContext, report::ValidationIssue};

use super::Validator;
use linkml_core::types::{ClassDefinition, ConditionalRequirement, SlotDefinition};

/// Validator for conditional requirements (`if_required/then_required`)
pub struct ConditionalRequirementValidator;

impl Default for ConditionalRequirementValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConditionalRequirementValidator {
    /// Create a new conditional requirement validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if a condition is satisfied
    fn check_condition(
        &self,
        instance: &Value,
        slot_name: &str,
        condition: &ConditionalRequirement,
        context: &mut ValidationContext,
    ) -> Result<bool, ValidationIssue> {
        // Get the slot value
        let slot_value = match instance {
            Value::Object(map) => map.get(slot_name),
            _ => None,
        };

        // If there's no condition specified, it's always satisfied
        let Some(slot_condition) = &condition.condition else {
            return Ok(true);
        };

        // Check various condition types
        if let Some(required) = slot_condition.required {
            let is_present = slot_value.is_some() && !matches!(slot_value, Some(Value::Null));
            if required != is_present {
                return Ok(false);
            }
        }

        if let Some(ref equals_string) = slot_condition.equals_string {
            match slot_value {
                Some(Value::String(s)) => {
                    if s != equals_string {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }

        if let Some(equals_number) = slot_condition.equals_number {
            match slot_value {
                Some(Value::Number(n)) => {
                    if let Some(f) = n.as_f64() {
                        if (f - equals_number).abs() > f64::EPSILON {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
                _ => return Ok(false),
            }
        }

        if let Some(ref pattern) = slot_condition.pattern {
            if let Some(Value::String(s)) = slot_value {
                match regex::Regex::new(pattern) {
                    Ok(re) => {
                        if !re.is_match(s) {
                            return Ok(false);
                        }
                    }
                    Err(e) => {
                        return Err(ValidationIssue::error(
                            format!("Invalid regex pattern '{pattern}': {e}"),
                            context.path(),
                            self.name(),
                        )
                        .with_code("INVALID_CONDITION_PATTERN"));
                    }
                }
            } else {
                return Ok(false);
            }
        }

        // Check range constraints
        if slot_condition.minimum_value.is_some() || slot_condition.maximum_value.is_some() {
            match slot_value {
                Some(Value::Number(n)) => {
                    if let Some(value) = n.as_f64() {
                        if let Some(ref min) = slot_condition.minimum_value
                            && let Some(min_val) = min.as_f64()
                            && value < min_val
                        {
                            return Ok(false);
                        }
                        if let Some(ref max) = slot_condition.maximum_value
                            && let Some(max_val) = max.as_f64()
                            && value > max_val
                        {
                            return Ok(false);
                        }
                    }
                }
                _ => return Ok(false),
            }
        }

        // All conditions passed
        Ok(true)
    }

    /// Validate conditional requirements for a class instance
    pub fn validate_class(
        &self,
        instance: &Value,
        class_def: &ClassDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check if there are any conditional requirements
        let Some(if_required) = &class_def.if_required else {
            return issues;
        };

        // For each conditional requirement
        for (condition_slot, requirement) in if_required {
            context.push_path(format!("if_required[{condition_slot}]"));

            match self.check_condition(instance, condition_slot, requirement, context) {
                Ok(condition_met) => {
                    if condition_met {
                        // Condition is satisfied, check then_required slots
                        if let Some(then_slots) = &requirement.then_required {
                            for required_slot in then_slots {
                                context.push_path(format!("then_required[{required_slot}]"));

                                // Get the value of the required slot
                                let slot_value = match instance {
                                    Value::Object(map) => map.get(required_slot),
                                    _ => None,
                                };

                                // Check if the required slot is missing or null
                                if slot_value.is_none() || matches!(slot_value, Some(Value::Null)) {
                                    let mut issue = ValidationIssue::error(
                                        format!(
                                            "Field '{required_slot}' is required when '{condition_slot}' satisfies condition"
                                        ),
                                        context.path(),
                                        "ConditionalRequirementValidator",
                                    );
                                    issue.code =
                                        Some("CONDITIONAL_REQUIREMENT_NOT_MET".to_string());
                                    issue.context.insert(
                                        "condition_slot".to_string(),
                                        serde_json::json!(condition_slot),
                                    );
                                    issue.context.insert(
                                        "required_slot".to_string(),
                                        serde_json::json!(required_slot),
                                    );
                                    issues.push(issue);
                                }

                                context.pop_path();
                            }
                        }
                    }
                }
                Err(error) => {
                    issues.push(error);
                }
            }

            context.pop_path();
        }

        issues
    }
}

impl Validator for ConditionalRequirementValidator {
    fn validate(
        &self,
        _value: &Value,
        _slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        // This validator works at the class level, not slot level
        // It should be called from the instance validator
        vec![ValidationIssue::warning(
            "ConditionalRequirementValidator should be called at class level, not slot level",
            context.path(),
            self.name(),
        )]
    }

    fn name(&self) -> &'static str {
        "ConditionalRequirementValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use linkml_core::types::{SchemaDefinition, SlotCondition};
    use std::sync::Arc;

    #[test]
    fn test_conditional_requirement_string_equals() {
        let validator = ConditionalRequirementValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a class with conditional requirement
        let mut if_required = IndexMap::new();
        if_required.insert(
            "status".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    equals_string: Some("active".to_string()),
                    ..Default::default()
                }),
                then_required: Some(vec!["email".to_string(), "phone".to_string()]),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            if_required: Some(if_required),
            ..Default::default()
        };

        // Test case 1: Status is "active" but email and phone are missing
        let instance1 = serde_json::json!({
            "name": "John Doe",
            "status": "active"
        });

        let issues = validator.validate_class(&instance1, &class_def, &mut context);
        assert_eq!(issues.len(), 2);
        assert!(issues[0].message.contains("email"));
        assert!(issues[1].message.contains("phone"));

        // Test case 2: Status is "active" and required fields are present
        let instance2 = serde_json::json!({
            "name": "John Doe",
            "status": "active",
            "email": "john@example.com",
            "phone": "555-1234"
        });

        let issues = validator.validate_class(&instance2, &class_def, &mut context);
        assert!(issues.is_empty());

        // Test case 3: Status is "inactive", conditional fields not required
        let instance3 = serde_json::json!({
            "name": "Jane Doe",
            "status": "inactive"
        });

        let issues = validator.validate_class(&instance3, &class_def, &mut context);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_conditional_requirement_number_range() {
        let validator = ConditionalRequirementValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a class with conditional requirement based on age range
        let mut if_required = IndexMap::new();
        if_required.insert(
            "age".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    minimum_value: Some(serde_json::json!(18)),
                    maximum_value: Some(serde_json::json!(65)),
                    ..Default::default()
                }),
                then_required: Some(vec!["employment_status".to_string()]),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            if_required: Some(if_required),
            ..Default::default()
        };

        // Test case 1: Age is 30 (in range), employment_status required
        let instance1 = serde_json::json!({
            "name": "Working Adult",
            "age": 30
        });

        let issues = validator.validate_class(&instance1, &class_def, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("employment_status"));

        // Test case 2: Age is 30 with employment_status
        let instance2 = serde_json::json!({
            "name": "Working Adult",
            "age": 30,
            "employment_status": "employed"
        });

        let issues = validator.validate_class(&instance2, &class_def, &mut context);
        assert!(issues.is_empty());

        // Test case 3: Age is 70 (out of range), no requirement
        let instance3 = serde_json::json!({
            "name": "Retired Person",
            "age": 70
        });

        let issues = validator.validate_class(&instance3, &class_def, &mut context);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_conditional_requirement_pattern() {
        let validator = ConditionalRequirementValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a class with conditional requirement based on pattern
        let mut if_required = IndexMap::new();
        if_required.insert(
            "email".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    pattern: Some(r".*@company\.com$".to_string()),
                    ..Default::default()
                }),
                then_required: Some(vec!["employee_id".to_string()]),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            if_required: Some(if_required),
            ..Default::default()
        };

        // Test case 1: Company email requires employee_id
        let instance1 = serde_json::json!({
            "name": "Employee",
            "email": "john@company.com"
        });

        let issues = validator.validate_class(&instance1, &class_def, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("employee_id"));

        // Test case 2: Non-company email doesn't require employee_id
        let instance2 = serde_json::json!({
            "name": "External User",
            "email": "john@gmail.com"
        });

        let issues = validator.validate_class(&instance2, &class_def, &mut context);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_conditional_requirement_required_field() {
        let validator = ConditionalRequirementValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a class where if phone is provided, then phone_type is required
        let mut if_required = IndexMap::new();
        if_required.insert(
            "phone".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    required: Some(true), // If phone field is present
                    ..Default::default()
                }),
                then_required: Some(vec!["phone_type".to_string()]),
            },
        );

        let class_def = ClassDefinition {
            name: "Contact".to_string(),
            if_required: Some(if_required),
            ..Default::default()
        };

        // Test case 1: Phone is provided but phone_type is missing
        let instance1 = serde_json::json!({
            "name": "Contact Info",
            "phone": "555-1234"
        });

        let issues = validator.validate_class(&instance1, &class_def, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("phone_type"));

        // Test case 2: No phone provided, no phone_type required
        let instance2 = serde_json::json!({
            "name": "Contact Info"
        });

        let issues = validator.validate_class(&instance2, &class_def, &mut context);
        assert!(issues.is_empty());

        // Test case 3: Phone and phone_type both provided
        let instance3 = serde_json::json!({
            "name": "Contact Info",
            "phone": "555-1234",
            "phone_type": "mobile"
        });

        let issues = validator.validate_class(&instance3, &class_def, &mut context);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_multiple_conditional_requirements() {
        let validator = ConditionalRequirementValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a class with multiple conditional requirements
        let mut if_required = IndexMap::new();

        // If country is "US", then state is required
        if_required.insert(
            "country".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    equals_string: Some("US".to_string()),
                    ..Default::default()
                }),
                then_required: Some(vec!["state".to_string()]),
            },
        );

        // If is_student is true, then student_id is required
        if_required.insert(
            "is_student".to_string(),
            ConditionalRequirement {
                condition: Some(SlotCondition {
                    required: Some(true),
                    ..Default::default()
                }),
                then_required: Some(vec!["student_id".to_string()]),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            if_required: Some(if_required),
            ..Default::default()
        };

        // Test case: Both conditions are met, both requirements violated
        let instance = serde_json::json!({
            "name": "US Student",
            "country": "US",
            "is_student": true
        });

        let issues = validator.validate_class(&instance, &class_def, &mut context);
        assert_eq!(issues.len(), 2);

        // Check that we have one error for each missing field
        let error_fields: Vec<String> = issues
            .iter()
            .filter_map(|issue| {
                issue
                    .context
                    .get("required_slot")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect();

        assert!(error_fields.contains(&"state".to_string()));
        assert!(error_fields.contains(&"student_id".to_string()));
    }
}
