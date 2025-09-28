//! Range validation for numeric values

use super::{ValidationContext, ValidationIssue, Validator};
use crate::utils::safe_cast::i64_to_f64_lossy;
use linkml_core::types::SlotDefinition;
use serde_json::Value;

/// Validator for numeric range constraints
pub struct RangeValidator {
    name: String,
}

impl Default for RangeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RangeValidator {
    /// Create a new range validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "range_validator".to_string(),
        }
    }

    /// Validate a numeric value against range constraints
    fn validate_range(
        &self,
        value: f64,
        slot: &SlotDefinition,
        path: &str,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check minimum value
        if let Some(min_val) = &slot.minimum_value {
            let min = if let Some(min) = min_val.as_f64() {
                min
            } else if let Some(min_str) = min_val.as_str() {
                if let Ok(min) = min_str.parse::<f64>() {
                    min
                } else {
                    return issues; // Can't parse minimum value
                }
            } else {
                return issues; // Not a number or string
            };

            if value < min {
                issues.push(ValidationIssue::error(
                    format!("Value {value} is less than minimum {min}"),
                    path,
                    &self.name,
                ));
            }
        }

        // Check maximum value
        if let Some(max_val) = &slot.maximum_value {
            let max = if let Some(max) = max_val.as_f64() {
                max
            } else if let Some(max_str) = max_val.as_str() {
                if let Ok(max) = max_str.parse::<f64>() {
                    max
                } else {
                    return issues; // Can't parse maximum value
                }
            } else {
                return issues; // Not a number or string
            };

            if value > max {
                issues.push(ValidationIssue::error(
                    format!("Value {value} exceeds maximum {max}"),
                    path,
                    &self.name,
                ));
            }
        }

        issues
    }
}

impl Validator for RangeValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Skip if no range constraints
        if slot.minimum_value.is_none() && slot.maximum_value.is_none() {
            return issues;
        }

        // Check if we should validate based on range type or actual value type
        let range_type = slot.range.as_deref().unwrap_or("string");
        let is_numeric_type = matches!(
            range_type,
            "integer" | "int" | "float" | "double" | "decimal" | "number"
        );

        // Also check if the actual value is numeric (for cases where range isn't specified)
        let is_numeric_value = value.is_number();

        if !is_numeric_type && !is_numeric_value {
            // Skip if neither the declared type nor actual value is numeric
            return issues;
        }

        let validate_number = |v: &Value, path: &str| -> Vec<ValidationIssue> {
            if let Some(n) = v.as_f64() {
                self.validate_range(n, slot, path)
            } else if let Some(n) = v.as_i64() {
                // Convert i64 to f64 using safe casting
                let n_f64 = i64_to_f64_lossy(n);
                self.validate_range(n_f64, slot, path)
            } else if !v.is_null() {
                vec![ValidationIssue::error(
                    format!(
                        "Expected numeric value for range validation, got {}",
                        value_type(v)
                    ),
                    path,
                    &self.name,
                )]
            } else {
                vec![]
            }
        };

        if slot.multivalued.unwrap_or(false) {
            if let Some(array) = value.as_array() {
                for (i, element) in array.iter().enumerate() {
                    issues.extend(validate_number(
                        element,
                        &format!("{}[{}]", context.path(), i),
                    ));
                }
            }
        } else {
            issues.extend(validate_number(value, &context.path()));
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Get the type name of a `JSON` value
fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
