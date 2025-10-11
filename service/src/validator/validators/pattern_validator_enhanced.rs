//! Enhanced pattern validation with named capture groups and LRU caching

use super::{ValidationContext, ValidationIssue, Validator};
use linkml_core::types::SlotDefinition;
use lru::LruCache;
use regex::Regex;
use serde_json::{Map, Value};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};

/// Result of a pattern match with capture groups
#[derive(Debug, Clone)]
pub struct PatternMatchResult {
    /// Whether the pattern matched
    pub matched: bool,
    /// Named capture groups if any
    pub captures: Option<Map<String, Value>>,
}

/// Enhanced pattern validator with named capture group support
pub struct EnhancedPatternValidator {
    name: String,
    /// LRU cache of compiled regex patterns
    pattern_cache: Arc<Mutex<LruCache<String, Arc<Regex>>>>,
    /// Maximum cache size
    cache_size: usize,
}

impl Default for EnhancedPatternValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedPatternValidator {
    /// Create a new enhanced pattern validator
    #[must_use]
    pub fn new() -> Self {
        Self::with_cache_size(100)
    }

    /// Create with custom cache size
    ///
    /// # Panics
    ///
    /// Panics if `NonZeroUsize::new(100)` somehow returns `None` (should not happen).
    #[must_use]
    pub fn with_cache_size(size: usize) -> Self {
        let cache_size = NonZeroUsize::new(size)
            .unwrap_or(NonZeroUsize::new(100).expect("100 is a valid non-zero usize"));
        Self {
            name: "enhanced_pattern_validator".to_string(),
            pattern_cache: Arc::new(Mutex::new(LruCache::new(cache_size))),
            cache_size: size,
        }
    }

    /// Get the configured cache size
    #[must_use]
    pub fn cache_size(&self) -> usize {
        self.cache_size
    }

    /// Get or compile a regex pattern with caching
    fn get_regex(&self, pattern: &str) -> Result<Arc<Regex>, regex::Error> {
        let mut cache = self.pattern_cache.lock().map_err(|e| {
            regex::Error::Syntax(format!("pattern cache mutex should not be poisoned: {e}"))
        })?;

        if let Some(regex) = cache.get(pattern) {
            return Ok(Arc::clone(regex));
        }

        let regex = Arc::new(Regex::new(pattern)?);
        cache.put(pattern.to_string(), Arc::clone(&regex));
        Ok(regex)
    }

    /// Validate a string against a pattern and extract named captures
    fn validate_pattern_with_captures(
        &self,
        value: &str,
        pattern: &str,
        path: &str,
    ) -> (Vec<ValidationIssue>, Option<Map<String, Value>>) {
        let mut issues = Vec::new();
        let mut captures = None;

        match self.get_regex(pattern) {
            Ok(regex) => {
                if let Some(caps) = regex.captures(value) {
                    // Pattern matched, extract named captures
                    let mut capture_map = Map::new();

                    // Get all named capture groups
                    for name in regex.capture_names().flatten() {
                        if let Some(matched) = caps.name(name) {
                            capture_map.insert(
                                name.to_string(),
                                Value::String(matched.as_str().to_string()),
                            );
                        }
                    }

                    if !capture_map.is_empty() {
                        captures = Some(capture_map);
                    }
                } else {
                    // Pattern didn't match
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

        (issues, captures)
    }

    /// Store capture groups in validation context for cross-field validation
    fn store_captures(
        context: &mut ValidationContext,
        slot_name: &str,
        captures: Map<String, Value>,
    ) {
        // Store captures in context for potential cross-field validation
        context.set_data(&format!("captures.{slot_name}"), Value::Object(captures));
    }

    /// Validate pattern with group constraints
    fn validate_pattern_groups(
        &self,
        value: &str,
        pattern: &str,
        slot: &SlotDefinition,
        path: &str,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let (issues, captures) = self.validate_pattern_with_captures(value, pattern, path);

        // If we have captures and the slot name, store them
        if let Some(capture_map) = captures
            && !capture_map.is_empty()
        {
            Self::store_captures(context, &slot.name, capture_map.clone());

            // Check if there are any group constraints (future enhancement)
            // For now, just add info about successful capture
            context.add_info(format!(
                "Pattern matched with {} capture groups",
                capture_map.len()
            ));
        }

        issues
    }
}

impl Validator for EnhancedPatternValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Debug: Log the slot pattern
        eprintln!(
            "DEBUG EnhancedPatternValidator: Validating slot '{}' with pattern: {:?}",
            slot.name, slot.pattern
        );

        // Only validate if there's a pattern
        if let Some(pattern) = &slot.pattern {
            if slot.multivalued.unwrap_or(false) {
                if let Some(array) = value.as_array() {
                    for (i, element) in array.iter().enumerate() {
                        let path = format!("{}[{}]", context.path(), i);
                        if let Some(s) = element.as_str() {
                            issues.extend(
                                self.validate_pattern_groups(s, pattern, slot, &path, context),
                            );
                        } else if !element.is_null() {
                            issues.push(ValidationIssue::error(
                                "Pattern validation requires string value",
                                &path,
                                &self.name,
                            ));
                        }
                    }
                }
            } else {
                let path = context.path();
                if let Some(s) = value.as_str() {
                    issues.extend(self.validate_pattern_groups(s, pattern, slot, &path, context));
                } else if !value.is_null() {
                    issues.push(ValidationIssue::error(
                        "Pattern validation requires string value",
                        &path,
                        &self.name,
                    ));
                }
            }
        }

        // Also check if the slot's range has a pattern (for custom types)
        if let Some(range) = &slot.range {
            let type_pattern = context
                .schema
                .types
                .get(range)
                .and_then(|type_def| type_def.pattern.clone());

            if let Some(pattern) = type_pattern {
                if slot.multivalued.unwrap_or(false) {
                    if let Some(array) = value.as_array() {
                        for (i, element) in array.iter().enumerate() {
                            let path = format!("{}[{}]", context.path(), i);
                            if let Some(s) = element.as_str() {
                                issues.extend(
                                    self.validate_pattern_groups(s, &pattern, slot, &path, context),
                                );
                            } else if !element.is_null() {
                                issues.push(ValidationIssue::error(
                                    format!("Type '{range}' with pattern requires string value"),
                                    &path,
                                    &self.name,
                                ));
                            }
                        }
                    }
                } else {
                    let path = context.path();
                    if let Some(s) = value.as_str() {
                        issues.extend(
                            self.validate_pattern_groups(s, &pattern, slot, &path, context),
                        );
                    } else if !value.is_null() {
                        issues.push(ValidationIssue::error(
                            format!("Type '{range}' with pattern requires string value"),
                            &path,
                            &self.name,
                        ));
                    }
                }
            }
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validator::{context::ValidationContext, report::Severity};
    use linkml_core::types::{SchemaDefinition, SlotDefinition};
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_named_capture_groups() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let validator = EnhancedPatternValidator::new();
        let pattern = r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})$";

        let (issues, captures) =
            validator.validate_pattern_with_captures("2025-01-31", pattern, "test_date");

        assert!(issues.is_empty());
        assert!(captures.is_some());

        let caps = captures.expect("should have captured groups for date pattern");
        assert_eq!(
            caps.get("year").expect("should have year capture"),
            &json!("2025")
        );
        assert_eq!(
            caps.get("month").expect("should have month capture"),
            &json!("01")
        );
        assert_eq!(
            caps.get("day").expect("should have day capture"),
            &json!("31")
        );

        Ok(())
    }

    #[test]
    fn test_pattern_caching() -> Result<(), Box<dyn std::error::Error>> {
        let validator = EnhancedPatternValidator::with_cache_size(2);
        let pattern1 = r"^\d+$";
        let pattern2 = r"^[a-z]+$";
        let pattern3 = r"^[A-Z]+$";

        // First access - compiles and caches
        let _ = validator.get_regex(pattern1)?;
        let _ = validator.get_regex(pattern2)?;

        // Access again - should be cached
        let _ = validator.get_regex(pattern1)?;
        let _ = validator.get_regex(pattern2)?;

        // Third pattern - should evict the least recently used (pattern1)
        let _ = validator.get_regex(pattern3)?;

        // Accessing pattern2 should still be cached
        let _ = validator.get_regex(pattern2)?;

        Ok(())
    }

    #[test]
    fn test_pattern_violation_reports_issue() {
        let validator = EnhancedPatternValidator::new();
        let slot = SlotDefinition {
            name: "code".to_string(),
            pattern: Some(r"^[A-Z]{3}$".to_string()),
            ..Default::default()
        };

        let schema = SchemaDefinition {
            id: "test".to_string(),
            name: "Test".to_string(),
            ..Default::default()
        };

        let mut context = ValidationContext::new(Arc::new(schema));
        let value = json!("abc");

        let issues = validator.validate(&value, &slot, &mut context);
        assert!(!issues.is_empty());
        assert!(issues.iter().any(|issue| issue.severity == Severity::Error));
    }
}
