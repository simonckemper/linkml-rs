//! Boolean constraint validators for `LinkML`
//!
//! This module implements validators for `any_of`, `all_of`, `exactly_one_of`, and `none_of` constraints.
//!
//! ## Performance Features
//!
//! - Parallel evaluation for `all_of` constraints using Rayon
//! - Short-circuit optimization for `any_of` and `none_of`
//! - Efficient expression evaluation caching
//! - Optimized constraint checking order

use linkml_core::types::{AnonymousSlotExpression, SlotDefinition};
use rayon::prelude::*;
use serde_json::{Value, json};
use std::sync::Arc;

use crate::validator::{
    context::ValidationContext,
    report::{Severity, ValidationIssue},
};

use super::{PatternValidator, RangeValidator, RequiredValidator, TypeValidator, Validator};

/// Validator for `any_of` constraints - at least one must be satisfied
pub struct AnyOfValidator;

impl Default for AnyOfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AnyOfValidator {
    /// Create a new `any_of` validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate a single anonymous slot expression
    fn validate_expression(
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
                context.push_path(format!("any_of[{i}]"));
                let sub_issues = Self::validate_expression(value, constraint, context);

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
                    .with_context("value", value.clone()),
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

    fn name(&self) -> &'static str {
        "AnyOfValidator"
    }
}

/// Validator for `all_of` constraints - all must be satisfied
///
/// ## Performance Optimizations
///
/// - Parallel evaluation of constraints when more than 3 constraints exist
/// - Smart ordering: cheap validations (type, required) before expensive ones (pattern)
/// - Thread-local context cloning to avoid contention
pub struct AllOfValidator {
    /// Threshold for parallel evaluation
    parallel_threshold: usize,
}

impl Default for AllOfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl AllOfValidator {
    /// Create a new `all_of` validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            parallel_threshold: 3,
        }
    }

    /// Create a new `all_of` validator with custom parallel threshold
    #[must_use]
    pub fn with_parallel_threshold(threshold: usize) -> Self {
        Self {
            parallel_threshold: threshold,
        }
    }

    /// Validate a single anonymous slot expression
    fn validate_expression(
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

        // Apply validators in order of typical performance (cheapest first)

        // 1. Required check (cheapest)
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            issues.extend(required_validator.validate(value, &temp_slot, context));
            if !issues.is_empty() {
                return issues; // Early exit if required check fails
            }
        }

        // 2. Type check (cheap)
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            issues.extend(type_validator.validate(value, &temp_slot, context));
            if !issues.is_empty() {
                return issues; // Early exit if type check fails
            }
        }

        // 3. Range check (moderate)
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            issues.extend(range_validator.validate(value, &temp_slot, context));
        }

        // 4. Pattern check (expensive - regex compilation/matching)
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            issues.extend(pattern_validator.validate(value, &temp_slot, context));
        }

        issues
    }

    /// Validate a single expression with thread-safe context
    fn validate_expression_parallel(
        value: &Arc<Value>,
        expr: &AnonymousSlotExpression,
        path: &str,
        schema: &Arc<linkml_core::types::SchemaDefinition>,
    ) -> Vec<ValidationIssue> {
        // Create a new context for this thread
        let mut context = ValidationContext::new(Arc::clone(schema));
        context.push_path(path);

        let issues = Self::validate_expression(value, expr, &mut context);

        context.pop_path();
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

            let constraint_count = constraints.len();

            // Use parallel evaluation for many constraints
            if constraint_count > self.parallel_threshold {
                // Prepare data for parallel processing
                let value_arc = Arc::new(value.clone());
                let schema_arc = context.schema.clone();
                let base_path = context.path().to_string();

                // Process constraints in parallel
                let all_issues: Vec<(usize, Vec<ValidationIssue>)> = constraints
                    .par_iter()
                    .enumerate()
                    .map(|(i, constraint)| {
                        let path = format!("{base_path}/all_of[{i}]");
                        let issues = Self::validate_expression_parallel(
                            &value_arc,
                            constraint,
                            &path,
                            &schema_arc,
                        );
                        (i, issues)
                    })
                    .collect();

                // Aggregate results
                let mut failed_count = 0;
                for (i, sub_issues) in all_issues {
                    if !sub_issues.is_empty() {
                        failed_count += 1;

                        // Add sub-issues directly as they represent actual failures
                        for mut sub_issue in sub_issues {
                            sub_issue.message = format!("all_of[{}]: {}", i, sub_issue.message);
                            issues.push(sub_issue);
                        }
                    }
                }

                if failed_count > 0 {
                    // Add a summary error at the beginning
                    issues.insert(
                        0,
                        ValidationIssue::error(
                            format!(
                                "Value failed {failed_count} of {constraint_count} constraints in all_of"
                            ),
                            context.path(),
                            self.name(),
                        )
                        .with_code("ALL_OF_CONSTRAINT_FAILED")
                        .with_context("total_constraints", json!(constraint_count))
                        .with_context("failed_constraints", json!(failed_count))
                        .with_context("value", value.clone()),
                    );
                }
            } else {
                // Sequential evaluation for few constraints
                let mut failed_count = 0;

                // Check that all constraints are satisfied
                for (i, constraint) in constraints.iter().enumerate() {
                    context.push_path(format!("all_of[{i}]"));
                    let sub_issues = Self::validate_expression(value, constraint, context);

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
                    issues.insert(
                        0,
                        ValidationIssue::error(
                            format!(
                                "Value failed {failed_count} of {constraint_count} constraints in all_of"
                            ),
                            context.path(),
                            self.name(),
                        )
                        .with_code("ALL_OF_CONSTRAINT_FAILED")
                        .with_context("total_constraints", json!(constraint_count))
                        .with_context("failed_constraints", json!(failed_count))
                        .with_context("value", value.clone()),
                    );
                }
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "AllOfValidator"
    }
}

/// Validator for `exactly_one_of` constraints - exactly one must be satisfied
pub struct ExactlyOneOfValidator;

impl Default for ExactlyOneOfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExactlyOneOfValidator {
    /// Create a new `exactly_one_of` validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Validate a single anonymous slot expression
    fn validate_expression(
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
                context.push_path(format!("exactly_one_of[{i}]"));
                let sub_issues = Self::validate_expression(value, constraint, context);

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
                            "Value satisfies {satisfied_count} constraints but exactly one is required"
                        ),
                        context.path(),
                        self.name(),
                    )
                    .with_code("EXACTLY_ONE_OF_MULTIPLE_SATISFIED")
                    .with_context("satisfied_count", json!(satisfied_count))
                    .with_context("satisfied_indices", json!(satisfied_indices))
                    .with_context("value", value.clone()),
                );
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "ExactlyOneOfValidator"
    }
}

/// Validator for `none_of` constraints - none can be satisfied
///
/// ## Performance Optimizations
///
/// - Early exit on first satisfied constraint (fail-fast)
/// - Optimized constraint ordering for quick rejection
/// - Minimal validation overhead for common cases
pub struct NoneOfValidator;

impl Default for NoneOfValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl NoneOfValidator {
    /// Create a new `none_of` validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Check if expression is satisfied without full validation
    /// Returns true if the expression is satisfied (which means `none_of` should fail)
    fn is_expression_satisfied(value: &Value, expr: &AnonymousSlotExpression) -> bool {
        // Type check if range is specified
        if let Some(range) = &expr.range {
            let type_matches = match (range.as_str(), value) {
                ("integer", Value::Number(n)) => n.is_i64() || n.is_u64(),
                ("string", Value::String(_))
                | ("boolean", Value::Bool(_))
                | ("null", Value::Null)
                | ("array", Value::Array(_))
                | ("object", Value::Object(_))
                | ("float" | "double" | "number", Value::Number(_)) => true,
                _ => false,
            };
            if !type_matches {
                return false;
            }
        }

        // Check pattern constraint
        if let Some(pattern) = &expr.pattern
            && let Value::String(s) = value
        {
            if let Ok(re) = regex::Regex::new(pattern)
                && !re.is_match(s)
            {
                return false;
            }
        } else if expr.pattern.is_some() {
            return false; // Pattern only applies to strings
        }

        // Check minimum value constraint
        if let Some(min_val) = &expr.minimum_value
            && let Value::Number(n) = value
            && let (Some(actual), Some(min)) = (
                n.as_f64(),
                min_val.as_number().and_then(serde_json::Number::as_f64),
            )
            && actual < min
        {
            return false;
        }

        // Check maximum value constraint
        if let Some(max_val) = &expr.maximum_value
            && let Value::Number(n) = value
            && let (Some(actual), Some(max)) = (
                n.as_f64(),
                max_val.as_number().and_then(serde_json::Number::as_f64),
            )
            && actual > max
        {
            return false;
        }

        // Check required constraint
        if let Some(required) = expr.required
            && required
            && value == &Value::Null
        {
            return false;
        }

        // All constraints satisfied
        true
    }

    /// Validate a single anonymous slot expression
    fn validate_expression(
        value: &Value,
        expr: &AnonymousSlotExpression,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
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

        // Check if this expression has any actual constraints
        let has_constraints = expr.range.is_some()
            || expr.pattern.is_some()
            || expr.minimum_value.is_some()
            || expr.maximum_value.is_some()
            || expr.required.is_some();

        if !has_constraints {
            // No constraints means the expression is always satisfied
            // (any value passes when there are no constraints)
            return Vec::new();
        }

        // Apply validators - for none_of, we check if ALL constraints pass
        // If they all pass (no issues), the expression IS satisfied
        // If any fail (have issues), the expression is NOT satisfied

        // Type check is the primary validator
        if expr.range.is_some() {
            let type_validator = TypeValidator::new();
            let type_issues = type_validator.validate(value, &temp_slot, context);
            if !type_issues.is_empty() {
                // Type doesn't match - constraint NOT satisfied (good for none_of)
                return type_issues;
            }
        }

        // Required check
        if expr.required.is_some() {
            let required_validator = RequiredValidator::new();
            let required_issues = required_validator.validate(value, &temp_slot, context);
            if !required_issues.is_empty() {
                // Required check failed - constraint NOT satisfied (good for none_of)
                return required_issues;
            }
        }

        // Range check
        if expr.minimum_value.is_some() || expr.maximum_value.is_some() {
            let range_validator = RangeValidator::new();
            let range_issues = range_validator.validate(value, &temp_slot, context);
            if !range_issues.is_empty() {
                // Range check failed - constraint NOT satisfied (good for none_of)
                return range_issues;
            }
        }

        // Pattern check
        if expr.pattern.is_some() {
            let pattern_validator = PatternValidator::new();
            let pattern_issues = pattern_validator.validate(value, &temp_slot, context);
            if !pattern_issues.is_empty() {
                // Pattern check failed - constraint NOT satisfied (good for none_of)
                return pattern_issues;
            }
        }

        // All checks passed - the constraint IS satisfied (bad for none_of)
        Vec::new()
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

            // First pass: Quick satisfaction check for simple constraints
            for (i, constraint) in constraints.iter().enumerate() {
                // Quick check for simple type-only constraints
                if Self::is_expression_satisfied(value, constraint) {
                    satisfied_indices.push(i);
                    // Early exit optimization: If we already found a satisfied constraint,
                    // we know none_of will fail, so we can skip remaining checks
                    if satisfied_indices.len() == 1 {
                        issues.push(
                            ValidationIssue::error(
                                format!(
                                    "Value satisfies constraint none_of[{}] (type: {:?})",
                                    i, constraint.range
                                ),
                                context.path(),
                                self.name(),
                            )
                            .with_code("NONE_OF_CONSTRAINT_SATISFIED")
                            .with_context("satisfied_index", json!(i))
                            .with_context("value", value.clone()),
                        );
                        return issues;
                    }
                }
            }

            // Second pass: Full validation for complex constraints
            for (i, constraint) in constraints.iter().enumerate() {
                // Skip if already identified as satisfied in quick check
                if satisfied_indices.contains(&i) {
                    continue;
                }

                context.push_path(format!("none_of[{i}]"));
                let sub_issues = Self::validate_expression(value, constraint, context);

                if sub_issues.is_empty() {
                    satisfied_indices.push(i);

                    // Early exit: Found first satisfied constraint
                    context.pop_path();
                    issues.push(
                        ValidationIssue::error(
                            format!("Value satisfies constraint none_of[{i}]"),
                            context.path(),
                            self.name(),
                        )
                        .with_code("NONE_OF_CONSTRAINT_SATISFIED")
                        .with_context("satisfied_index", json!(i))
                        .with_context("value", value.clone()),
                    );
                    return issues;
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
                    .with_context("value", value.clone()),
                );
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "NoneOfValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::report::Severity;
    use linkml_core::types::{AnonymousSlotExpression, SchemaDefinition};

    #[test]
    fn test_any_of_validator_success() {
        let validator = AnyOfValidator::new();
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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
        assert!(
            issues
                .iter()
                .any(|i| i.code.as_deref() == Some("ALL_OF_CONSTRAINT_FAILED"))
        );
    }

    #[test]
    fn test_exactly_one_of_validator_success() {
        let validator = ExactlyOneOfValidator::new();
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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
        assert_eq!(
            issues[0].code.as_deref(),
            Some("EXACTLY_ONE_OF_MULTIPLE_SATISFIED")
        );
    }

    #[test]
    fn test_none_of_validator_success() {
        let validator = NoneOfValidator::new();
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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
        assert_eq!(
            issues[0].code.as_deref(),
            Some("NONE_OF_CONSTRAINT_SATISFIED")
        );
    }

    #[test]
    fn test_validators_only_trigger_when_constraints_present() {
        let schema = SchemaDefinition::default();
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

        assert!(
            any_of
                .validate(&json!("test"), &slot, &mut context)
                .is_empty()
        );
        assert!(
            all_of
                .validate(&json!("test"), &slot, &mut context)
                .is_empty()
        );
        assert!(
            exactly_one_of
                .validate(&json!("test"), &slot, &mut context)
                .is_empty()
        );
        assert!(
            none_of
                .validate(&json!("test"), &slot, &mut context)
                .is_empty()
        );
    }

    #[test]
    fn test_complex_any_of_with_patterns() {
        let validator = AnyOfValidator::new();
        let schema = SchemaDefinition::default();
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
        let schema = SchemaDefinition::default();
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

    #[test]
    fn test_all_of_parallel_evaluation() {
        // Test with more than 3 constraints to trigger parallel evaluation
        let validator = AllOfValidator::with_parallel_threshold(3);
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        let slot = SlotDefinition {
            name: "complex_validation".to_string(),
            all_of: Some(vec![
                AnonymousSlotExpression {
                    range: Some("string".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    pattern: Some(r"^[A-Z]".to_string()), // Must start with uppercase
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    pattern: Some(r"\d$".to_string()), // Must end with digit
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    minimum_value: Some(json!(5)), // Length >= 5
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    maximum_value: Some(json!(20)), // Length <= 20
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        // Valid: meets all constraints
        let issues = validator.validate(&json!("Hello123"), &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid: doesn't start with uppercase
        let issues = validator.validate(&json!("hello123"), &slot, &mut context);
        assert!(!issues.is_empty());

        // Invalid: doesn't end with digit
        let issues = validator.validate(&json!("Hello"), &slot, &mut context);
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_none_of_early_exit_optimization() {
        let validator = NoneOfValidator::new();
        let schema = SchemaDefinition::default();
        let mut context = ValidationContext::new(Arc::new(schema));

        // Create a slot with many none_of constraints
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
                AnonymousSlotExpression {
                    range: Some("boolean".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    range: Some("array".to_string()),
                    ..Default::default()
                },
                AnonymousSlotExpression {
                    range: Some("object".to_string()),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        // String value should fail fast (first constraint satisfied)
        let issues = validator.validate(&json!("hello"), &slot, &mut context);
        assert!(!issues.is_empty());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("none_of[0]"));

        // Float value should pass (no constraint satisfied)
        // 3.14 is a float/double, not integer, string, boolean, array, or object
        #[allow(clippy::approx_constant)]
        let issues = validator.validate(&json!(3.14), &slot, &mut context);
        assert!(issues.is_empty());
    }
}
