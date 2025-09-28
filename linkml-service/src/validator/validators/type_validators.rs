//! Type validators for `LinkML` primitive types

use super::{ValidationContext, ValidationIssue, Validator};
use crate::validator::interned_report::{InternedValidationIssue, IssueBuilder};
use crate::validator::string_interner::global_interner;
use chrono::{DateTime, NaiveDate};
use linkml_core::types::SlotDefinition;
use serde_json::Value;
use url::Url;

/// Main type validator that delegates to specific type validators
pub struct TypeValidator {
    name: String,
    issue_builder: IssueBuilder,
}

impl Default for TypeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeValidator {
    /// Create a new type validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "type_validator".to_string(),
            issue_builder: IssueBuilder::new(),
        }
    }

    /// Validate a value against a `LinkML` type
    fn validate_type(&self, value: &Value, type_name: &str, path: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Use interned strings for common cases
        let interner = global_interner();
        let _common = interner.common();

        match type_name {
            "string" | "str" => {
                if !value.is_string() {
                    // Use IssueBuilder for type mismatch
                    let interned_issue =
                        self.issue_builder
                            .type_mismatch("string", value_type_name(value), path);
                    issues.push(interned_issue.to_regular());
                }
            }
            "integer" | "int" => {
                if let Some(_n) = value.as_i64() {
                    // Valid integer
                } else if let Some(n) = value.as_f64() {
                    if n.fract() != 0.0 {
                        // Use interned strings for error message
                        let interned_issue = InternedValidationIssue::error(
                            format!("Expected integer, got float: {n}"),
                            path,
                            &self.name,
                        )
                        .with_code("type_mismatch");
                        issues.push(interned_issue.to_regular());
                    }
                } else {
                    // Use IssueBuilder for type mismatch
                    let interned_issue =
                        self.issue_builder
                            .type_mismatch("integer", value_type_name(value), path);
                    issues.push(interned_issue.to_regular());
                }
            }
            "float" | "double" | "decimal" => {
                if !value.is_number() {
                    issues.push(ValidationIssue::error(
                        format!("Expected number, got {}", value_type_name(value)),
                        path,
                        &self.name,
                    ));
                }
            }
            "boolean" | "bool" => {
                if !value.is_boolean() {
                    issues.push(ValidationIssue::error(
                        format!("Expected boolean, got {}", value_type_name(value)),
                        path,
                        &self.name,
                    ));
                }
            }
            "date" => {
                if let Some(s) = value.as_str() {
                    if NaiveDate::parse_from_str(s, "%Y-%m-%d").is_err() {
                        issues.push(ValidationIssue::error(
                            format!("Invalid date format: '{s}'. Expected YYYY-MM-DD"),
                            path,
                            &self.name,
                        ));
                    }
                } else {
                    issues.push(ValidationIssue::error(
                        "Date must be a string in YYYY-MM-DD format",
                        path,
                        &self.name,
                    ));
                }
            }
            "datetime" => {
                if let Some(s) = value.as_str() {
                    if DateTime::parse_from_rfc3339(s).is_err() {
                        issues.push(ValidationIssue::error(
                            format!("Invalid datetime format: '{s}'. Expected RFC3339"),
                            path,
                            &self.name,
                        ));
                    }
                } else {
                    issues.push(ValidationIssue::error(
                        "Datetime must be a string in RFC3339 format",
                        path,
                        &self.name,
                    ));
                }
            }
            "time" => {
                if let Some(s) = value.as_str() {
                    // Simple time validation HH:MM:SS
                    let parts: Vec<&str> = s.split(':').collect();
                    if parts.len() == 3 {
                        let valid = parts[0].parse::<u8>().is_ok_and(|h| h < 24)
                            && parts[1].parse::<u8>().is_ok_and(|m| m < 60)
                            && parts[2].parse::<u8>().is_ok_and(|s| s < 60);
                        if !valid {
                            issues.push(ValidationIssue::error(
                                format!("Invalid time value: '{s}'"),
                                path,
                                &self.name,
                            ));
                        }
                    } else {
                        issues.push(ValidationIssue::error(
                            format!("Invalid time format: '{s}'. Expected HH:MM:SS"),
                            path,
                            &self.name,
                        ));
                    }
                } else {
                    issues.push(ValidationIssue::error(
                        "Time must be a string in HH:MM:SS format",
                        path,
                        &self.name,
                    ));
                }
            }
            "uri" | "uriorcurie" => {
                if let Some(s) = value.as_str() {
                    // Try to parse as URL
                    if Url::parse(s).is_err() {
                        // For uriorcurie, also accept CURIE format (prefix:local)
                        if type_name == "uriorcurie" && s.contains(':') && !s.starts_with("http") {
                            // Looks like a CURIE, accept it
                        } else {
                            issues.push(ValidationIssue::error(
                                format!("Invalid URI: '{s}'"),
                                path,
                                &self.name,
                            ));
                        }
                    }
                } else {
                    issues.push(ValidationIssue::error(
                        "URI must be a string",
                        path,
                        &self.name,
                    ));
                }
            }
            "ncname" => {
                if let Some(s) = value.as_str() {
                    // NCName: no colons, must start with letter or underscore
                    if s.contains(':') {
                        issues.push(ValidationIssue::error(
                            format!("NCName cannot contain colons: '{s}'"),
                            path,
                            &self.name,
                        ));
                    } else if s.is_empty()
                        || (!s
                            .chars()
                            .next()
                            .expect("non-empty string should have first char")
                            .is_alphabetic()
                            && !s.starts_with('_'))
                    {
                        issues.push(ValidationIssue::error(
                            format!("NCName must start with letter or underscore: '{s}'"),
                            path,
                            &self.name,
                        ));
                    }
                } else {
                    issues.push(ValidationIssue::error(
                        "NCName must be a string",
                        path,
                        &self.name,
                    ));
                }
            }
            "array" => {
                if !value.is_array() {
                    issues.push(ValidationIssue::error(
                        format!("Expected array, got {}", value_type_name(value)),
                        path,
                        &self.name,
                    ));
                }
            }
            "object" => {
                if !value.is_object() {
                    issues.push(ValidationIssue::error(
                        format!("Expected object, got {}", value_type_name(value)),
                        path,
                        &self.name,
                    ));
                }
            }
            _ => {
                // Unknown type or custom type - for now, accept anything
                // In a full implementation, we'd look up custom types in the schema
            }
        }

        issues
    }
}

impl Validator for TypeValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Get the range (type) for this slot
        let type_name = slot.range.as_deref().unwrap_or("string");

        // Handle multivalued slots
        if slot.multivalued.unwrap_or(false) {
            if let Some(array) = value.as_array() {
                // Validate each element
                for (i, element) in array.iter().enumerate() {
                    let element_path = format!("{}[{}]", context.path(), i);
                    let type_issues = self.validate_type(element, type_name, &element_path);
                    issues.extend(type_issues);
                }
            } else {
                issues.push(ValidationIssue::error(
                    format!(
                        "Expected array for multivalued slot, got {}",
                        value_type_name(value)
                    ),
                    context.path(),
                    &self.name,
                ));
            }
        } else {
            // Single valued slot
            let type_issues = self.validate_type(value, type_name, &context.path());
            issues.extend(type_issues);
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Get a human-readable name for a `JSON` value type
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
