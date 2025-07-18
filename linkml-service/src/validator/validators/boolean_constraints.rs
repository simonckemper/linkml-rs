//! Boolean constraint validators for LinkML
//!
//! This module implements validators for any_of, all_of, exactly_one_of, and none_of constraints.

use linkml_core::types::{AnonymousSlotExpression, SlotDefinition};
use serde_json::{json, Value};

use crate::validator::{
    context::ValidationContext,
    report::{Severity, ValidationIssue},
};

use super::{
    Validator, RangeValidator, PatternValidator, RequiredValidator, 
    TypeValidator
};

/// Validator for any_of constraints - at least one must be satisfied
pub struct AnyOfValidator;

impl AnyOfValidator {
    /// Create a new any_of validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate a single anonymous slot expression
    fn validate_expression(
        &self,
        value: &Value,
        expr: &AnonymousSlotExpression,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Create a temporary slot definition from the anonymous expression
        let temp_slot = SlotDefinition {
            name: format!("{}_any_of_expr", context.path()),
            range: expr.range.clone(),
            pattern: expr.pattern.clone(),
            minimum_value: expr.minimum_value.clone(),
            maximum_value: expr.maximum_value.clone(),
            required: expr.required,
            // Note: minimum_cardinality and maximum_cardinality are not in SlotDefinition
            // They would need to be handled separately if needed
            ..Default::default()
        };
        
        // Apply relevant validators
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            issues.extend(type_validator.validate(value, &temp_slot, context));
        }
        
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            issues.extend(pattern_validator.validate(value, &temp_slot, context));
        }
        
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            issues.extend(range_validator.validate(value, &temp_slot, context));
        }
        
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            issues.extend(required_validator.validate(value, &temp_slot, context));
        }
        
        issues
    }
}

impl Validator for AnyOfValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        if let Some(constraints) = &slot.any_of {
            if constraints.is_empty() {
                return issues;
            }
            
            let mut satisfied = false;
            let mut all_sub_issues = Vec::new();
            
            // Check if at least one constraint is satisfied
            for (i, constraint) in constraints.iter().enumerate() {
                context.push_path(&format!("any_of[{}]", i));
                let sub_issues = self.validate_expression(value, constraint, context);
                
                if sub_issues.is_empty() {
                    satisfied = true;
                    context.pop_path();
                    break; // Short-circuit: at least one is satisfied
                }
                
                all_sub_issues.extend(sub_issues);
                context.pop_path();
            }
            
            if !satisfied {
                issues.push(
                    ValidationIssue::error(
                        format!(
                            "Value does not satisfy any of the {} constraints",
                            constraints.len()
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("ANY_OF_CONSTRAINT_FAILED")
                    .with_context("constraint_count", json!(constraints.len()))
                    .with_context("value", value.clone())
                );
                
                // Add sub-issues as warnings to help debugging
                for mut sub_issue in all_sub_issues {
                    sub_issue.severity = Severity::Warning;
                    sub_issue.message = format!("Sub-constraint failed: {}", sub_issue.message);
                    issues.push(sub_issue);
                }
            }
        }
        
        issues
    }
    
    fn name(&self) -> &str {
        "AnyOfValidator"
    }
}

/// Validator for all_of constraints - all must be satisfied
pub struct AllOfValidator;

impl AllOfValidator {
    /// Create a new all_of validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate a single anonymous slot expression
    fn validate_expression(
        &self,
        value: &Value,
        expr: &AnonymousSlotExpression,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Create a temporary slot definition from the anonymous expression
        let temp_slot = SlotDefinition {
            name: format!("{}_all_of_expr", context.path()),
            range: expr.range.clone(),
            pattern: expr.pattern.clone(),
            minimum_value: expr.minimum_value.clone(),
            maximum_value: expr.maximum_value.clone(),
            required: expr.required,
            // Note: minimum_cardinality and maximum_cardinality are not in SlotDefinition
            // They would need to be handled separately if needed
            ..Default::default()
        };
        
        // Apply relevant validators
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            issues.extend(type_validator.validate(value, &temp_slot, context));
        }
        
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            issues.extend(pattern_validator.validate(value, &temp_slot, context));
        }
        
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            issues.extend(range_validator.validate(value, &temp_slot, context));
        }
        
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            issues.extend(required_validator.validate(value, &temp_slot, context));
        }
        
        issues
    }
}

impl Validator for AllOfValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        if let Some(constraints) = &slot.all_of {
            if constraints.is_empty() {
                return issues;
            }
            
            let mut failed_count = 0;
            
            // Check that all constraints are satisfied
            for (i, constraint) in constraints.iter().enumerate() {
                context.push_path(&format!("all_of[{}]", i));
                let sub_issues = self.validate_expression(value, constraint, context);
                
                if !sub_issues.is_empty() {
                    failed_count += 1;
                    
                    // Add sub-issues directly as they represent actual failures
                    for mut sub_issue in sub_issues {
                        sub_issue.message = format!("all_of[{}]: {}", i, sub_issue.message);
                        issues.push(sub_issue);
                    }
                }
                
                context.pop_path();
            }
            
            if failed_count > 0 {
                // Add a summary error at the beginning
                issues.insert(0,
                    ValidationIssue::error(
                        format!(
                            "Value failed {} of {} constraints in all_of",
                            failed_count, constraints.len()
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("ALL_OF_CONSTRAINT_FAILED")
                    .with_context("total_constraints", json!(constraints.len()))
                    .with_context("failed_constraints", json!(failed_count))
                    .with_context("value", value.clone())
                );
            }
        }
        
        issues
    }
    
    fn name(&self) -> &str {
        "AllOfValidator"
    }
}

/// Validator for exactly_one_of constraints - exactly one must be satisfied
pub struct ExactlyOneOfValidator;

impl ExactlyOneOfValidator {
    /// Create a new exactly_one_of validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate a single anonymous slot expression
    fn validate_expression(
        &self,
        value: &Value,
        expr: &AnonymousSlotExpression,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Create a temporary slot definition from the anonymous expression
        let temp_slot = SlotDefinition {
            name: format!("{}_exactly_one_of_expr", context.path()),
            range: expr.range.clone(),
            pattern: expr.pattern.clone(),
            minimum_value: expr.minimum_value.clone(),
            maximum_value: expr.maximum_value.clone(),
            required: expr.required,
            // Note: minimum_cardinality and maximum_cardinality are not in SlotDefinition
            // They would need to be handled separately if needed
            ..Default::default()
        };
        
        // Apply relevant validators
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            issues.extend(type_validator.validate(value, &temp_slot, context));
        }
        
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            issues.extend(pattern_validator.validate(value, &temp_slot, context));
        }
        
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            issues.extend(range_validator.validate(value, &temp_slot, context));
        }
        
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            issues.extend(required_validator.validate(value, &temp_slot, context));
        }
        
        issues
    }
}

impl Validator for ExactlyOneOfValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        if let Some(constraints) = &slot.exactly_one_of {
            if constraints.is_empty() {
                return issues;
            }
            
            let mut satisfied_count = 0;
            let mut satisfied_indices = Vec::new();
            
            // Count how many constraints are satisfied
            for (i, constraint) in constraints.iter().enumerate() {
                context.push_path(&format!("exactly_one_of[{}]", i));
                let sub_issues = self.validate_expression(value, constraint, context);
                
                if sub_issues.is_empty() {
                    satisfied_count += 1;
                    satisfied_indices.push(i);
                }
                
                context.pop_path();
            }
            
            if satisfied_count == 0 {
                issues.push(
                    ValidationIssue::error(
                        format!(
                            "Value does not satisfy any of the {} constraints (exactly one required)",
                            constraints.len()
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("EXACTLY_ONE_OF_NONE_SATISFIED")
                    .with_context("constraint_count", json!(constraints.len()))
                    .with_context("value", value.clone())
                );
            } else if satisfied_count > 1 {
                issues.push(
                    ValidationIssue::error(
                        format!(
                            "Value satisfies {} constraints but exactly one is required",
                            satisfied_count
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("EXACTLY_ONE_OF_MULTIPLE_SATISFIED")
                    .with_context("satisfied_count", json!(satisfied_count))
                    .with_context("satisfied_indices", json!(satisfied_indices))
                    .with_context("value", value.clone())
                );
            }
        }
        
        issues
    }
    
    fn name(&self) -> &str {
        "ExactlyOneOfValidator"
    }
}

/// Validator for none_of constraints - none can be satisfied
pub struct NoneOfValidator;

impl NoneOfValidator {
    /// Create a new none_of validator
    pub fn new() -> Self {
        Self
    }
    
    /// Validate a single anonymous slot expression
    fn validate_expression(
        &self,
        value: &Value,
        expr: &AnonymousSlotExpression,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        // Create a temporary slot definition from the anonymous expression
        let temp_slot = SlotDefinition {
            name: format!("{}_none_of_expr", context.path()),
            range: expr.range.clone(),
            pattern: expr.pattern.clone(),
            minimum_value: expr.minimum_value.clone(),
            maximum_value: expr.maximum_value.clone(),
            required: expr.required,
            // Note: minimum_cardinality and maximum_cardinality are not in SlotDefinition
            // They would need to be handled separately if needed
            ..Default::default()
        };
        
        // Apply relevant validators
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            issues.extend(type_validator.validate(value, &temp_slot, context));
        }
        
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            issues.extend(pattern_validator.validate(value, &temp_slot, context));
        }
        
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            issues.extend(range_validator.validate(value, &temp_slot, context));
        }
        
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            issues.extend(required_validator.validate(value, &temp_slot, context));
        }
        
        issues
    }
}

impl Validator for NoneOfValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        
        if let Some(constraints) = &slot.none_of {
            if constraints.is_empty() {
                return issues;
            }
            
            let mut satisfied_indices = Vec::new();
            
            // Check if any constraints are satisfied (they shouldn't be)
            for (i, constraint) in constraints.iter().enumerate() {
                context.push_path(&format!("none_of[{}]", i));
                let sub_issues = self.validate_expression(value, constraint, context);
                
                if sub_issues.is_empty() {
                    satisfied_indices.push(i);
                }
                
                context.pop_path();
            }
            
            if !satisfied_indices.is_empty() {
                issues.push(
                    ValidationIssue::error(
                        format!(
                            "Value satisfies {} constraint(s) that should not be satisfied",
                            satisfied_indices.len()
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("NONE_OF_CONSTRAINT_SATISFIED")
                    .with_context("satisfied_count", json!(satisfied_indices.len()))
                    .with_context("satisfied_indices", json!(satisfied_indices))
                    .with_context("value", value.clone())
                );
            }
        }
        
        issues
    }
    
    fn name(&self) -> &str {
        "NoneOfValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::report::Severity;
    use linkml_core::types::AnonymousSlotExpression;
    use std::sync::Arc;
    
    #[test]
    fn test_any_of_validator_success() {
        let validator = AnyOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            any_of: Some(vec![
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
        
        // String value should satisfy first constraint
        let issues = validator.validate(&json!("hello"), &slot, &mut context);
        assert!(issues.is_empty());
        
        // Integer value should satisfy second constraint
        let issues = validator.validate(&json!(42), &slot, &mut context);
        assert!(issues.is_empty());
    }
    
    #[test]
    fn test_any_of_validator_failure() {
        let validator = AnyOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            any_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("string".to_string()),
                    pattern: Some(r"^\d+$".to_string()), // Only digits
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    range: Some("integer".to_string()),
                    minimum_value: Some(json!(100)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // This value doesn't satisfy either constraint
        let issues = validator.validate(&json!("hello"), &slot, &mut context);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].severity, Severity::Error);
        assert_eq!(issues[0].code.as_deref(), Some("ANY_OF_CONSTRAINT_FAILED"));
    }
    
    #[test]
    fn test_all_of_validator_success() {
        let validator = AllOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            all_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("integer".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    minimum_value: Some(json!(0)),
                    maximum_value: Some(json!(100)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Value satisfies all constraints
        let issues = validator.validate(&json!(50), &slot, &mut context);
        assert!(issues.is_empty());
    }
    
    #[test]
    fn test_all_of_validator_failure() {
        let validator = AllOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            all_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("integer".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    minimum_value: Some(json!(0)),
                    maximum_value: Some(json!(100)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Value violates range constraint
        let issues = validator.validate(&json!(150), &slot, &mut context);
        assert!(!issues.is_empty());
        // Should have summary error and specific constraint error
        assert!(issues.iter().any(|i| i.code.as_deref() == Some("ALL_OF_CONSTRAINT_FAILED")));
    }
    
    #[test]
    fn test_exactly_one_of_validator_success() {
        let validator = ExactlyOneOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            exactly_one_of: Some(vec![
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
        
        // String value satisfies exactly one constraint
        let issues = validator.validate(&json!("hello"), &slot, &mut context);
        assert!(issues.is_empty());
    }
    
    #[test]
    fn test_exactly_one_of_validator_multiple_satisfied() {
        let validator = ExactlyOneOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            exactly_one_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("integer".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    minimum_value: Some(json!(0)),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Integer value satisfies both constraints
        let issues = validator.validate(&json!(50), &slot, &mut context);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].code.as_deref(), Some("EXACTLY_ONE_OF_MULTIPLE_SATISFIED"));
    }
    
    #[test]
    fn test_none_of_validator_success() {
        let validator = NoneOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            none_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("string".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    range: Some("boolean".to_string()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Integer value doesn't satisfy any constraint
        let issues = validator.validate(&json!(42), &slot, &mut context);
        assert!(issues.is_empty());
    }
    
    #[test]
    fn test_none_of_validator_failure() {
        let validator = NoneOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "test".to_string(),
            none_of: Some(vec![
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
        
        // String value satisfies first constraint
        let issues = validator.validate(&json!("hello"), &slot, &mut context);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].code.as_deref(), Some("NONE_OF_CONSTRAINT_SATISFIED"));
    }
    
    #[test]
    fn test_validators_only_trigger_when_constraints_present() {
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        // Slot without any boolean constraints
        let slot = SlotDefinition {
            name: "test".to_string(),
            ..Default::default()
        };
        
        // Test all validators - none should produce issues
        let any_of = AnyOfValidator::new();
        let all_of = AllOfValidator::new();
        let exactly_one_of = ExactlyOneOfValidator::new();
        let none_of = NoneOfValidator::new();
        
        assert!(any_of.validate(&json!("test"), &slot, &mut context).is_empty());
        assert!(all_of.validate(&json!("test"), &slot, &mut context).is_empty());
        assert!(exactly_one_of.validate(&json!("test"), &slot, &mut context).is_empty());
        assert!(none_of.validate(&json!("test"), &slot, &mut context).is_empty());
    }
    
    #[test]
    fn test_complex_any_of_with_patterns() {
        let validator = AnyOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "identifier".to_string(),
            any_of: Some(vec![
                AnonymousSlotExpression {
                    pattern: Some(r"^[A-Z]{2}\d{6}$".to_string()), // Format: XX123456
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    pattern: Some(r"^\d{3}-\d{2}-\d{4}$".to_string()), // Format: 123-45-6789
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Test first pattern
        let issues = validator.validate(&json!("AB123456"), &slot, &mut context);
        assert!(issues.is_empty());
        
        // Test second pattern
        let issues = validator.validate(&json!("123-45-6789"), &slot, &mut context);
        assert!(issues.is_empty());
        
        // Test invalid
        let issues = validator.validate(&json!("invalid"), &slot, &mut context);
        assert!(!issues.is_empty());
    }
    
    #[test]
    fn test_all_of_with_overlapping_ranges() {
        let validator = AllOfValidator::new();
        let schema = Default::default();
        let mut context = ValidationContext::new(Arc::new(schema));
        
        let slot = SlotDefinition {
            name: "age".to_string(),
            all_of: Some(vec![
                AnonymousSlotExpression {
                    minimum_value: Some(json!(18)), // Must be >= 18
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    maximum_value: Some(json!(65)), // Must be <= 65
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    range: Some("integer".to_string()), // Must be integer
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };
        
        // Valid: integer in range
        let issues = validator.validate(&json!(30), &slot, &mut context);
        assert!(issues.is_empty());
        
        // Invalid: too young
        let issues = validator.validate(&json!(15), &slot, &mut context);
        assert!(!issues.is_empty());
        
        // Invalid: too old
        let issues = validator.validate(&json!(70), &slot, &mut context);
        assert!(!issues.is_empty());
        
        // Invalid: not an integer
        let issues = validator.validate(&json!(30.5), &slot, &mut context);
        assert!(!issues.is_empty());
    }
}