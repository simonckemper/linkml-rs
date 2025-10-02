//! Pattern-based slot validation for `LinkML`
//!
//! This module provides comprehensive pattern validation including:
//! - Regular expression patterns
//! - Structured patterns (email, URL, UUID, etc.)
//! - Named capture groups with extraction
//! - Pattern inheritance and overrides

use super::report::ValidationIssue;
use linkml_core::prelude::*;
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;

/// Common structured patterns with error handling
static EMAIL_PATTERN: std::sync::LazyLock<Result<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2}$")
        .map_err(|e| LinkMLError::service(format!("Invalid email regex: {e}")))
});

static URL_PATTERN: std::sync::LazyLock<Result<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"^https?://[a-zA-Z0-9.-]+(?:\.[a-zA-Z]{2})+(?:/[^?#]*)?(?:\?[^#]*)?(?:#.*)?$")
        .map_err(|e| LinkMLError::service(format!("Invalid URL regex: {e}")))
});

static UUID_PATTERN: std::sync::LazyLock<Result<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")
        .map_err(|e| LinkMLError::service(format!("Invalid UUID regex: {e}")))
});

static ISO_DATE_PATTERN: std::sync::LazyLock<Result<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}$")
        .map_err(|e| LinkMLError::service(format!("Invalid ISO date regex: {e}")))
});

static ISO_DATETIME_PATTERN: std::sync::LazyLock<Result<Regex>> = std::sync::LazyLock::new(|| {
    Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})?$")
        .map_err(|e| LinkMLError::service(format!("Invalid ISO datetime regex: {e}")))
});

/// Pattern validator for slot values
pub struct PatternValidator {
    /// Compiled regex patterns by slot name
    patterns: HashMap<String, Regex>,

    /// Structured patterns by type (storing owned Regex)
    structured_patterns: HashMap<String, Regex>,

    /// Named capture patterns for extraction
    named_patterns: HashMap<String, Regex>,
}

impl PatternValidator {
    /// Create a new pattern validator
    ///
    /// # Errors
    ///
    /// Returns an error if any of the built-in regex patterns fail to compile
    pub fn new() -> Result<Self> {
        let mut structured_patterns = HashMap::new();

        // Handle potential regex compilation errors
        if let Ok(ref email_regex) = *EMAIL_PATTERN {
            structured_patterns.insert("email".to_string(), email_regex.clone());
        }

        if let Ok(ref url_regex) = *URL_PATTERN {
            structured_patterns.insert("url".to_string(), url_regex.clone());
            structured_patterns.insert("uri".to_string(), url_regex.clone());
        }

        if let Ok(ref uuid_regex) = *UUID_PATTERN {
            structured_patterns.insert("uuid".to_string(), uuid_regex.clone());
        }

        if let Ok(ref date_regex) = *ISO_DATE_PATTERN {
            structured_patterns.insert("date".to_string(), date_regex.clone());
        }

        if let Ok(ref datetime_regex) = *ISO_DATETIME_PATTERN {
            structured_patterns.insert("datetime".to_string(), datetime_regex.clone());
            structured_patterns.insert("timestamp".to_string(), datetime_regex.clone());
        }

        Ok(Self {
            patterns: HashMap::new(),
            structured_patterns,
            named_patterns: HashMap::new(),
        })
    }

    /// Create validator from schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn from_schema(schema: &SchemaDefinition) -> Result<Self> {
        let mut validator = Self::new()?;

        // Compile patterns from slots
        for (slot_name, slot_def) in &schema.slots {
            if let Some(pattern) = &slot_def.pattern {
                eprintln!("DEBUG from_schema: Adding pattern for slot '{slot_name}': '{pattern}'");
                validator.add_pattern(slot_name, pattern)?;
            }

            // Check for structured pattern hints
            if let Some(structured) = Self::detect_structured_pattern(slot_def) {
                validator.add_structured_pattern(slot_name, &structured);
            }
        }

        Ok(validator)
    }

    /// Add a pattern for a slot
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn add_pattern(&mut self, slot_name: &str, pattern: &str) -> Result<()> {
        // The pattern comes from YAML where \d is already a single backslash
        // No conversion needed - use the pattern as-is
        eprintln!("DEBUG: Adding pattern for slot '{slot_name}': '{pattern}'");
        eprintln!("DEBUG: Pattern bytes: {:?}", pattern.as_bytes());

        let regex = Regex::new(pattern).map_err(|e| {
            LinkMLError::service(format!("Invalid pattern for slot '{slot_name}': {e}"))
        })?;

        eprintln!("DEBUG: Pattern compiled successfully for slot '{slot_name}'");

        // Check if it has named captures
        if regex.capture_names().flatten().count() > 0 {
            self.named_patterns
                .insert(slot_name.to_string(), regex.clone());
        }

        self.patterns.insert(slot_name.to_string(), regex);
        Ok(())
    }

    /// Add a structured pattern
    pub fn add_structured_pattern(&mut self, slot_name: &str, pattern_type: &str) {
        if let Some(pattern) = self.structured_patterns.get(pattern_type) {
            self.patterns
                .insert(slot_name.to_string(), (*pattern).clone());
        }
    }

    /// Detect structured pattern from slot definition
    fn detect_structured_pattern(slot: &SlotDefinition) -> Option<String> {
        // Check slot name hints
        let name_lower = slot.name.to_lowercase();
        if name_lower.contains("email") || name_lower.contains("mail") {
            return Some("email".to_string());
        }
        if name_lower.contains("url") || name_lower.contains("uri") || name_lower.contains("link") {
            return Some("url".to_string());
        }
        if name_lower.contains("uuid") || name_lower.contains("guid") {
            return Some("uuid".to_string());
        }
        if name_lower.contains("datetime") || name_lower.contains("timestamp") {
            return Some("datetime".to_string());
        }
        if name_lower.contains("date") && !name_lower.contains("update") {
            return Some("date".to_string());
        }

        // Check range hints
        if let Some(range) = &slot.range {
            match range.as_str() {
                "email" | "Email" => return Some("email".to_string()),
                "url" | "URL" | "uri" | "URI" => return Some("url".to_string()),
                "uuid" | "UUID" => return Some("uuid".to_string()),
                "date" | "Date" => return Some("date".to_string()),
                "datetime" | "DateTime" | "timestamp" => return Some("datetime".to_string()),
                _ => {}
            }
        }

        None
    }

    /// Validate a slot value against its pattern
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn validate_slot(&self, slot_name: &str, value: &Value) -> Result<()> {
        match value {
            Value::String(s) => {
                if let Some(pattern) = self.patterns.get(slot_name) {
                    eprintln!("DEBUG: Testing '{s}' against pattern for slot '{slot_name}'");
                    eprintln!("DEBUG: Pattern: {:?}", pattern.as_str());
                    if !pattern.is_match(s) {
                        eprintln!("DEBUG: Pattern match failed!");
                        return Err(LinkMLError::service(format!(
                            "Value '{s}' does not match pattern for slot '{slot_name}'"
                        )));
                    }
                    eprintln!("DEBUG: Pattern match succeeded!");
                }
            }
            Value::Number(n) => {
                let string_value = n.to_string();
                if let Some(pattern) = self.patterns.get(slot_name)
                    && !pattern.is_match(&string_value)
                {
                    return Err(LinkMLError::service(format!(
                        "Value '{string_value}' does not match pattern for slot '{slot_name}'"
                    )));
                }
            }
            Value::Array(items) => {
                // Handle multivalued slots
                for (i, item) in items.iter().enumerate() {
                    match item {
                        Value::String(s) => {
                            if let Some(pattern) = self.patterns.get(slot_name)
                                && !pattern.is_match(s)
                            {
                                return Err(LinkMLError::service(format!(
                                    "Array item [{i}] '{s}' does not match pattern for slot '{slot_name}'"
                                )));
                            }
                        }
                        Value::Number(n) => {
                            let string_value = n.to_string();
                            if let Some(pattern) = self.patterns.get(slot_name)
                                && !pattern.is_match(&string_value)
                            {
                                return Err(LinkMLError::service(format!(
                                    "Array item [{i}] '{string_value}' does not match pattern for slot '{slot_name}'"
                                )));
                            }
                        }
                        Value::Null => {} // Skip null values in arrays
                        _ => {
                            return Err(LinkMLError::service(format!(
                                "Pattern validation only applies to string/number values, got {item:?} for slot '{slot_name}' at index {i}"
                            )));
                        }
                    }
                }
            }
            Value::Null => {} // Null values skip pattern validation
            _ => {
                return Err(LinkMLError::service(format!(
                    "Pattern validation only applies to string/number values or arrays thereof, got {value:?} for slot '{slot_name}'"
                )));
            }
        }

        Ok(())
    }

    /// Extract named captures from a value
    #[must_use]
    pub fn extract_captures(
        &self,
        slot_name: &str,
        value: &str,
    ) -> Option<HashMap<String, String>> {
        if let Some(pattern) = self.named_patterns.get(slot_name)
            && let Some(captures) = pattern.captures(value)
        {
            let mut extracted = HashMap::new();

            for name in pattern.capture_names().flatten() {
                if let Some(matched) = captures.name(name) {
                    extracted.insert(name.to_string(), matched.as_str().to_string());
                }
            }

            return Some(extracted);
        }

        None
    }

    /// Validate all slots in an instance
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn validate_instance(
        &self,
        instance: &Value,
        class_name: &str,
        schema: &SchemaDefinition,
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        let class = schema
            .classes
            .get(class_name)
            .ok_or_else(|| LinkMLError::service(format!("Class '{class_name}' not found")))?;

        if let Value::Object(obj) = instance {
            for slot_name in &class.slots {
                if let Some(value) = obj.get(slot_name)
                    && let Err(e) = self.validate_slot(slot_name, value)
                {
                    // Extract array index from error message if present
                    let error_msg = e.to_string();
                    if error_msg.contains("Array item [") {
                        // Parse out the array index from the error message
                        if let Some(start) = error_msg.find("Array item [")
                            && let Some(end) = error_msg[start..].find(']')
                        {
                            let index_str = &error_msg[start + 12..start + end];
                            if let Ok(index) = index_str.parse::<usize>() {
                                issues.push(ValidationIssue::error(
                                    error_msg,
                                    format!("/{slot_name}/[{index}]"),
                                    format!("pattern:{slot_name}[{index}]"),
                                ));
                                continue;
                            }
                        }
                    }
                    // Default error formatting
                    issues.push(ValidationIssue::error(
                        error_msg,
                        format!("/{slot_name}"),
                        format!("pattern:{slot_name}"),
                    ));
                }
            }
        }

        Ok(issues)
    }
}

/// Apply pattern validation to an entire dataset
/// Returns an error if the operation fails
///
/// # Errors
///
pub fn validate_patterns(
    data: &Value,
    class_name: &str,
    schema: &SchemaDefinition,
) -> Result<Vec<ValidationIssue>> {
    let validator = PatternValidator::from_schema(schema)?;

    match data {
        Value::Array(items) => {
            let mut all_issues = Vec::new();
            for (i, item) in items.iter().enumerate() {
                let mut issues = validator.validate_instance(item, class_name, schema)?;
                // Add index to path
                for issue in &mut issues {
                    issue.path = format!("[{}]{}", i, issue.path);
                }
                all_issues.extend(issues);
            }
            Ok(all_issues)
        }
        _ => validator.validate_instance(data, class_name, schema),
    }
}

/// Pattern-based data transformation
pub struct PatternTransformer {
    /// Transformation patterns
    transformations: HashMap<String, TransformPattern>,
}

/// A transformation pattern
#[derive(Clone)]
pub struct TransformPattern {
    /// Pattern to match
    pub pattern: Regex,
    /// Replacement template
    pub replacement: String,
}

impl Default for PatternTransformer {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternTransformer {
    /// Create a new pattern transformer
    #[must_use]
    pub fn new() -> Self {
        Self {
            transformations: HashMap::new(),
        }
    }

    /// Add a transformation pattern
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn add_transformation(
        &mut self,
        slot_name: &str,
        pattern: &str,
        replacement: &str,
    ) -> Result<()> {
        let regex = Regex::new(pattern)
            .map_err(|e| LinkMLError::service(format!("Invalid transform pattern: {e}")))?;

        self.transformations.insert(
            slot_name.to_string(),
            TransformPattern {
                pattern: regex,
                replacement: replacement.to_string(),
            },
        );

        Ok(())
    }

    /// Transform a value using patterns
    #[must_use]
    pub fn transform(&self, slot_name: &str, value: &str) -> String {
        if let Some(transform) = self.transformations.get(slot_name) {
            transform
                .pattern
                .replace_all(value, &transform.replacement)
                .to_string()
        } else {
            value.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_email_pattern_validation() {
        let mut validator = match PatternValidator::new() {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to create pattern validator: {e}");
                return;
            }
        };
        validator.add_structured_pattern("email", "email");

        // Valid emails
        assert!(
            validator
                .validate_slot("email", &json!("user@example.com"))
                .is_ok()
        );
        assert!(
            validator
                .validate_slot("email", &json!("test.user+tag@domain.co.uk"))
                .is_ok()
        );

        // Invalid emails
        assert!(
            validator
                .validate_slot("email", &json!("not-an-email"))
                .is_err()
        );
        assert!(
            validator
                .validate_slot("email", &json!("@example.com"))
                .is_err()
        );
        assert!(validator.validate_slot("email", &json!("user@")).is_err());
    }

    #[test]
    fn test_custom_pattern() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut validator = match PatternValidator::new() {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to create pattern validator: {e}");
                return Ok(());
            }
        };

        // Add a custom pattern for phone numbers
        validator
            .add_pattern(
                "phone",
                r"^\+?[1-9]\d{1,14}$", // E.164 format
            )
            .expect("Should add pattern");

        // Valid phone numbers
        assert!(
            validator
                .validate_slot("phone", &json!("+14155552671"))
                .is_ok()
        );
        assert!(
            validator
                .validate_slot("phone", &json!("14155552671"))
                .is_ok()
        );

        // Invalid phone numbers
        assert!(
            validator
                .validate_slot("phone", &json!("555-1234"))
                .is_err()
        );
        assert!(
            validator
                .validate_slot("phone", &json!("+0123456789"))
                .is_err()
        );
        Ok(())
    }

    #[test]
    fn test_named_captures() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut validator = match PatternValidator::new() {
            Ok(v) => v,
            Err(e) => {
                assert!(false, "Failed to create pattern validator: {e}");
                return Ok(());
            }
        };

        // Pattern with named captures
        validator
            .add_pattern(
                "version",
                r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)$",
            )
            .expect("Should add pattern");

        let captures = validator
            .extract_captures("version", "1.2.3")
            .expect("Should extract captures");
        assert_eq!(captures.get("major").expect("Should have major"), "1");
        assert_eq!(captures.get("minor").expect("Should have minor"), "2");
        assert_eq!(captures.get("patch").expect("Should have patch"), "3");
        Ok(())
    }

    #[test]
    fn test_pattern_transformation() {
        let mut transformer = PatternTransformer::new();

        // Add transformation to normalize phone numbers
        transformer
            .add_transformation(
                "phone",
                r"[\s\-\(\)]", // Remove spaces, dashes, parentheses
                "",
            )
            .expect("Should add pattern");

        assert_eq!(
            transformer.transform("phone", "(415) 555-2671"),
            "4155552671"
        );
        assert_eq!(transformer.transform("phone", "415-555-2671"), "4155552671");
    }
}
