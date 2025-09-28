//! Pattern validation using regex

use super::{ValidationContext, ValidationIssue, Validator};
use linkml_core::types::SlotDefinition;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Validator for regex patterns
pub struct PatternValidator {
    name: String,
    /// Cache of compiled regex patterns
    pattern_cache: Arc<Mutex<HashMap<String, Regex>>>,
}

impl Default for PatternValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternValidator {
    /// Create a new pattern validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "pattern_validator".to_string(),
            pattern_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or compile a regex pattern
    fn get_regex(&self, pattern: &str) -> Result<Regex, regex::Error> {
        let mut cache = self
            .pattern_cache
            .lock()
            .map_err(|e| regex::Error::Syntax(format!("pattern cache lock poisoned: {e}")))?;

        if let Some(regex) = cache.get(pattern) {
            return Ok(regex.clone());
        }

        let regex = Regex::new(pattern)?;
        cache.insert(pattern.to_string(), regex.clone());
        Ok(regex)
    }

    /// Validate a string against a pattern
    fn validate_pattern(&self, value: &str, pattern: &str, path: &str) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        match self.get_regex(pattern) {
            Ok(regex) => {
                if !regex.is_match(value) {
                    issues.push(ValidationIssue::error(
                        format!("Value '{value}' does not match pattern '{pattern}'"),
                        path,
                        &self.name,
                    ));
                }
            }
            Err(e) => {
                issues.push(ValidationIssue::error(
                    format!("Invalid regex pattern '{pattern}': {e}"),
                    path,
                    &self.name,
                ));
            }
        }

        issues
    }
}

impl Validator for PatternValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Only validate if there's a pattern
        if let Some(pattern) = &slot.pattern {
            let validate_string = |v: &Value, path: &str| -> Vec<ValidationIssue> {
                if let Some(s) = v.as_str() {
                    self.validate_pattern(s, pattern, path)
                } else if !v.is_null() {
                    vec![ValidationIssue::error(
                        "Pattern validation requires string value",
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
                        issues.extend(validate_string(
                            element,
                            &format!("{}[{}]", context.path(), i),
                        ));
                    }
                }
            } else {
                issues.extend(validate_string(value, &context.path()));
            }
        }

        // Also check if the slot's range has a pattern (for custom types)
        if let Some(range) = &slot.range
            && let Some(type_def) = context.schema.types.get(range)
            && let Some(pattern) = &type_def.pattern
        {
            let validate_string = |v: &Value, path: &str| -> Vec<ValidationIssue> {
                if let Some(s) = v.as_str() {
                    self.validate_pattern(s, pattern, path)
                } else if !v.is_null() {
                    vec![ValidationIssue::error(
                        format!("Type '{range}' with pattern requires string value"),
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
                        issues.extend(validate_string(
                            element,
                            &format!("{}[{}]", context.path(), i),
                        ));
                    }
                }
            } else {
                issues.extend(validate_string(value, &context.path()));
            }
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}
