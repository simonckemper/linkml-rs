//! String constraint validators for `LinkML`
//!
//! This module implements validators for string-specific constraints like
//! `equals_string_in` and `structured_pattern`.

use linkml_core::{
    Value,
    error::{LinkMLError, Result},
    types::SlotDefinition,
};
use regex::Regex;
use std::collections::HashSet;

use crate::validator::{context::ValidationContext, report::ValidationIssue};

use super::Validator;

/// Validator for `equals_string_in` constraint
///
/// This validator ensures that string values are within a specified set of allowed values.
pub struct EqualsStringInValidator;

impl EqualsStringInValidator {
    /// Create a new `equals_string_in` validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Validator for EqualsStringInValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Only validate if equals_string_in is specified
        let allowed_values = match &slot.equals_string_in {
            Some(values) if !values.is_empty() => values,
            _ => return issues,
        };

        // Convert to HashSet for O(1) lookup
        let allowed_set: HashSet<&str> = allowed_values
            .iter()
            .map(std::string::String::as_str)
            .collect();

        match value {
            Value::String(s) => {
                if !allowed_set.contains(s.as_str()) {
                    let mut issue = ValidationIssue::error(
                        format!(
                            "Value '{}' is not in the allowed set: [{}]",
                            s,
                            allowed_values.join(", ")
                        ),
                        context.path(),
                        "EqualsStringInValidator",
                    );
                    issue.code = Some("EQUALS_STRING_IN_VIOLATION".to_string());
                    issue.context.insert("value".to_string(), value.clone());
                    issue.context.insert(
                        "allowed_values".to_string(),
                        Value::Array(
                            allowed_values
                                .iter()
                                .map(|s| Value::String(s.clone()))
                                .collect(),
                        ),
                    );
                    issues.push(issue);
                }
            }
            Value::Array(arr) if slot.multivalued.unwrap_or(false) => {
                // For multivalued slots, check each value
                for (i, item) in arr.iter().enumerate() {
                    context.push_index(i);
                    if let Value::String(s) = item {
                        if !allowed_set.contains(s.as_str()) {
                            let mut issue = ValidationIssue::error(
                                format!(
                                    "Value '{}' is not in the allowed set: [{}]",
                                    s,
                                    allowed_values.join(", ")
                                ),
                                context.path(),
                                "EqualsStringInValidator",
                            );
                            issue.code = Some("EQUALS_STRING_IN_VIOLATION".to_string());
                            issues.push(issue);
                        }
                    } else {
                        let mut issue = ValidationIssue::error(
                            format!("Expected string value, got {item}"),
                            context.path(),
                            "EqualsStringInValidator",
                        );
                        issue.code = Some("TYPE_MISMATCH".to_string());
                        issues.push(issue);
                    }
                    context.pop_path();
                }
            }
            Value::Null => {
                // Null is allowed unless required
                if slot.required.unwrap_or(false) {
                    let mut issue = ValidationIssue::error(
                        "Required field cannot be null",
                        context.path(),
                        "EqualsStringInValidator",
                    );
                    issue.code = Some("REQUIRED_FIELD_NULL".to_string());
                    issues.push(issue);
                }
            }
            _ => {
                let mut issue = ValidationIssue::error(
                    format!("Expected string value, got {value}"),
                    context.path(),
                    "EqualsStringInValidator",
                );
                issue.code = Some("TYPE_MISMATCH".to_string());
                issues.push(issue);
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "EqualsStringInValidator"
    }
}

/// Validator for `structured_pattern` constraint
///
/// This validator supports advanced pattern matching with different syntaxes
/// and interpolation support.
pub struct StructuredPatternValidator;

impl StructuredPatternValidator {
    /// Create a new structured pattern validator
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Apply pattern interpolation if enabled
    fn interpolate_pattern(pattern: &str, context: &ValidationContext) -> Result<String> {
        let mut result = pattern.to_string();

        // Simple interpolation: replace {variable} with context values
        let var_regex = Regex::new(r"\{(\w+)\}").map_err(|e| {
            LinkMLError::data_validation(format!("Invalid interpolation pattern: {e}"))
        })?;

        for cap in var_regex.captures_iter(pattern) {
            if let Some(var_match) = cap.get(1) {
                let var_name = var_match.as_str();

                // Look up variable in multiple places:
                // 1. Context data (highest priority)
                if let Some(value) = context.get_data(var_name) {
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => continue, // Skip complex types
                    };
                    result = result.replace(&format!("{{{var_name}}}"), &replacement);
                    continue;
                }

                // 2. Current object being validated (if available)
                if let Some(parent) = context.parent()
                    && let Some(obj) = parent.as_object()
                    && let Some(value) = obj.get(var_name)
                {
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => continue,
                    };
                    result = result.replace(&format!("{{{var_name}}}"), &replacement);
                    continue;
                }

                // 3. Root object (if available)
                if let Some(root) = context.root()
                    && let Some(obj) = root.as_object()
                    && let Some(value) = obj.get(var_name)
                {
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => continue,
                    };
                    result = result.replace(&format!("{{{var_name}}}"), &replacement);
                }

                // If variable not found, leave as-is (could be an error in strict mode)
            }
        }

        Ok(result)
    }

    /// Validate using regex syntax
    fn validate_regex(value: &str, pattern: &str, partial: bool) -> Result<bool> {
        let regex = Regex::new(pattern)
            .map_err(|e| LinkMLError::data_validation(format!("Invalid regex pattern: {e}")))?;

        if partial {
            Ok(regex.is_match(value))
        } else {
            // Full match required
            Ok(regex.find(value).is_some_and(|m| m.as_str() == value))
        }
    }

    /// Validate using glob syntax
    fn validate_glob(value: &str, pattern: &str, partial: bool) -> Result<bool> {
        // Simple glob implementation
        // In production, use a proper glob library
        let regex_pattern = pattern
            .replace('.', r"\.")
            .replace('*', ".*")
            .replace('?', ".");

        // Use anchors for full match, or no anchors for partial match
        let final_pattern = if partial {
            regex_pattern
        } else {
            format!("^{regex_pattern}$")
        };

        let regex = Regex::new(&final_pattern)
            .map_err(|e| LinkMLError::data_validation(format!("Invalid glob pattern: {e}")))?;

        Ok(regex.is_match(value))
    }
}

impl Validator for StructuredPatternValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Only validate if structured_pattern is specified
        let Some(structured_pattern) = &slot.structured_pattern else {
            return issues;
        };

        let Some(pattern) = &structured_pattern.pattern else {
            return issues;
        };

        // Apply interpolation if enabled
        let final_pattern = if structured_pattern.interpolated.unwrap_or(false) {
            match Self::interpolate_pattern(pattern, context) {
                Ok(p) => p,
                Err(e) => {
                    let mut issue = ValidationIssue::error(
                        format!("Pattern interpolation failed: {e}"),
                        context.path(),
                        "StructuredPatternValidator",
                    );
                    issue.code = Some("INTERPOLATION_ERROR".to_string());
                    issues.push(issue);
                    return issues;
                }
            }
        } else {
            pattern.clone()
        };

        let syntax = structured_pattern
            .syntax
            .as_deref()
            .unwrap_or("regular_expression");
        let partial = structured_pattern.partial_match.unwrap_or(false);

        match value {
            Value::String(s) => {
                let matches = match syntax {
                    "regular_expression" | "regex" => {
                        match Self::validate_regex(s, &final_pattern, partial) {
                            Ok(m) => m,
                            Err(e) => {
                                let mut issue = ValidationIssue::error(
                                    format!("Pattern validation error: {e}"),
                                    context.path(),
                                    "StructuredPatternValidator",
                                );
                                issue.code = Some("PATTERN_ERROR".to_string());
                                issues.push(issue);
                                return issues;
                            }
                        }
                    }
                    "glob" => match Self::validate_glob(s, &final_pattern, partial) {
                        Ok(m) => m,
                        Err(e) => {
                            let mut issue = ValidationIssue::error(
                                format!("Pattern validation error: {e}"),
                                context.path(),
                                "StructuredPatternValidator",
                            );
                            issue.code = Some("PATTERN_ERROR".to_string());
                            issues.push(issue);
                            return issues;
                        }
                    },
                    _ => {
                        let mut issue = ValidationIssue::error(
                            format!("Unsupported pattern syntax: {syntax}"),
                            context.path(),
                            "StructuredPatternValidator",
                        );
                        issue.code = Some("UNSUPPORTED_SYNTAX".to_string());
                        issues.push(issue);
                        return issues;
                    }
                };

                if !matches {
                    let mut issue = ValidationIssue::error(
                        format!("Value '{s}' does not match {syntax} pattern '{final_pattern}'"),
                        context.path(),
                        "StructuredPatternValidator",
                    );
                    issue.code = Some("STRUCTURED_PATTERN_VIOLATION".to_string());
                    issue.context.insert("value".to_string(), value.clone());
                    issue
                        .context
                        .insert("pattern".to_string(), Value::String(final_pattern));
                    issue
                        .context
                        .insert("syntax".to_string(), Value::String(syntax.to_string()));
                    issues.push(issue);
                }
            }
            Value::Array(arr) if slot.multivalued.unwrap_or(false) => {
                // For multivalued slots, check each value
                for (i, item) in arr.iter().enumerate() {
                    context.push_index(i);
                    if let Value::String(s) = item {
                        let matches = match syntax {
                            "regular_expression" | "regex" => {
                                match Self::validate_regex(s, &final_pattern, partial) {
                                    Ok(m) => m,
                                    Err(e) => {
                                        let mut issue = ValidationIssue::error(
                                            format!("Pattern validation error: {e}"),
                                            context.path(),
                                            "StructuredPatternValidator",
                                        );
                                        issue.code = Some("PATTERN_ERROR".to_string());
                                        issues.push(issue);
                                        context.pop_path();
                                        continue;
                                    }
                                }
                            }
                            "glob" => match Self::validate_glob(s, &final_pattern, partial) {
                                Ok(m) => m,
                                Err(e) => {
                                    let mut issue = ValidationIssue::error(
                                        format!("Pattern validation error: {e}"),
                                        context.path(),
                                        "StructuredPatternValidator",
                                    );
                                    issue.code = Some("PATTERN_ERROR".to_string());
                                    issues.push(issue);
                                    context.pop_path();
                                    continue;
                                }
                            },
                            _ => {
                                let mut issue = ValidationIssue::error(
                                    format!("Unsupported pattern syntax: {syntax}"),
                                    context.path(),
                                    "StructuredPatternValidator",
                                );
                                issue.code = Some("UNSUPPORTED_SYNTAX".to_string());
                                issues.push(issue);
                                context.pop_path();
                                continue;
                            }
                        };

                        if !matches {
                            let mut issue = ValidationIssue::error(
                                format!(
                                    "Value '{s}' does not match {syntax} pattern '{final_pattern}'"
                                ),
                                context.path(),
                                "StructuredPatternValidator",
                            );
                            issue.code = Some("STRUCTURED_PATTERN_VIOLATION".to_string());
                            issues.push(issue);
                        }
                    }
                    context.pop_path();
                }
            }
            Value::Null
            | Value::Bool(_)
            | Value::Number(_)
            | Value::Object(_)
            | Value::Array(_) => {
                // Null is allowed unless required, and we only validate strings
                // Other types are not validated by this string pattern validator
            }
        }

        issues
    }

    fn name(&self) -> &'static str {
        "StructuredPatternValidator"
    }
}

impl Default for EqualsStringInValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StructuredPatternValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{SchemaDefinition, StructuredPattern};
    use std::sync::Arc;

    #[test]
    fn test_equals_string_in_basic() {
        let validator = EqualsStringInValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        let mut slot = SlotDefinition::new("status");
        slot.equals_string_in = Some(vec![
            "pending".to_string(),
            "active".to_string(),
            "completed".to_string(),
        ]);

        // Valid value
        let value = Value::String("active".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid value
        let value = Value::String("invalid".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("not in the allowed set"));
    }

    #[test]
    fn test_equals_string_in_multivalued() {
        let validator = EqualsStringInValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        let mut slot = SlotDefinition::new("tags");
        slot.multivalued = Some(true);
        slot.equals_string_in = Some(vec![
            "red".to_string(),
            "green".to_string(),
            "blue".to_string(),
        ]);

        // Valid array
        let value = Value::Array(vec![
            Value::String("red".to_string()),
            Value::String("blue".to_string()),
        ]);
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Array with invalid value
        let value = Value::Array(vec![
            Value::String("red".to_string()),
            Value::String("yellow".to_string()),
        ]);
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_structured_pattern_regex() {
        let validator = StructuredPatternValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        let mut slot = SlotDefinition::new("email");
        slot.structured_pattern = Some(StructuredPattern {
            syntax: Some("regular_expression".to_string()),
            pattern: Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2}$".to_string()),
            interpolated: Some(false),
            partial_match: Some(false),
        });

        // Valid email
        let value = Value::String("test@example.com".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid email
        let value = Value::String("invalid-email".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("does not match"));
    }

    #[test]
    fn test_structured_pattern_glob() {
        let validator = StructuredPatternValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        let mut slot = SlotDefinition::new("filename");
        slot.structured_pattern = Some(StructuredPattern {
            syntax: Some("glob".to_string()),
            pattern: Some("*.txt".to_string()),
            interpolated: Some(false),
            partial_match: Some(false),
        });

        // Valid filename
        let value = Value::String("document.txt".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid filename
        let value = Value::String("document.pdf".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_structured_pattern_partial_match() {
        let validator = StructuredPatternValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        let mut slot = SlotDefinition::new("description");
        slot.structured_pattern = Some(StructuredPattern {
            syntax: Some("regular_expression".to_string()),
            pattern: Some(r"error|warning|info".to_string()),
            interpolated: Some(false),
            partial_match: Some(true),
        });

        // Contains keyword
        let value = Value::String("This is an error message".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Doesn't contain keyword
        let value = Value::String("This is a debug message".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_structured_pattern_interpolation() {
        let validator = StructuredPatternValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        // Set up context data for interpolation
        context.set_data("prefix", Value::String("TEST".to_string()));
        context.set_data("suffix", Value::String("\\d+".to_string()));

        let mut slot = SlotDefinition::new("code");
        slot.structured_pattern = Some(StructuredPattern {
            syntax: Some("regular_expression".to_string()),
            pattern: Some("{prefix}-{suffix}".to_string()),
            interpolated: Some(true),
            partial_match: Some(false),
        });

        // Should match TEST-123
        let value = Value::String("TEST-123".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Should not match PROD-123
        let value = Value::String("PROD-123".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);

        // Should not match TEST-ABC
        let value = Value::String("TEST-ABC".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_structured_pattern_interpolation_from_parent() {
        let validator = StructuredPatternValidator::new();
        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        // Set up parent object with data to interpolate
        let parent_obj = serde_json::json!({
            "type": "email",
            "domain": "example\\.com"
        });
        context.set_parent(parent_obj);

        let mut slot = SlotDefinition::new("address");
        slot.structured_pattern = Some(StructuredPattern {
            syntax: Some("regular_expression".to_string()),
            pattern: Some(r"^[\w\.-]+@{domain}$".to_string()),
            interpolated: Some(true),
            partial_match: Some(false),
        });

        // Should match user@example.com
        let value = Value::String("user@example.com".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Should not match user@other.com
        let value = Value::String("user@other.com".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }
}
