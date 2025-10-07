//! `TypeQL` constraint generation module
//!
//! This module provides comprehensive constraint translation from `LinkML` to `TypeQL`,
//! supporting `TypeDB` 3.0 features including @card, @key, @unique, and regex patterns.

use crate::utils::safe_cast::u64_to_usize_saturating;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// Enhanced constraint translator for `TypeQL` generation
pub struct TypeQLConstraintTranslator {
    /// Cache for compiled regex patterns
    regex_cache: HashMap<String, String>,
}

impl Default for TypeQLConstraintTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeQLConstraintTranslator {
    /// Create a new constraint translator
    #[must_use]
    pub fn new() -> Self {
        Self {
            regex_cache: HashMap::new(),
        }
    }

    /// Translate all constraints for a slot
    pub fn translate_slot_constraints(&mut self, slot: &SlotDefinition) -> Vec<String> {
        let mut constraints = Vec::new();

        // 1. Key constraint (@key)
        if slot.identifier == Some(true) {
            constraints.push("@key".to_string());
        }

        // 2. Unique constraint (@unique) - for non-key unique fields
        // Note: LinkML doesn't have explicit unique flag, but we can infer from patterns
        if slot.identifier != Some(true) && Self::is_unique_constraint(slot) {
            constraints.push("@unique".to_string());
        }

        // 3. Cardinality constraint (@card)
        if let Some(card) = self.translate_cardinality(slot) {
            constraints.push(card);
        }

        // 4. Regex pattern constraint
        if let Some(pattern) = &slot.pattern
            && let Some(regex) = self.translate_regex(pattern)
        {
            constraints.push(regex);
        }

        constraints
    }

    /// Determine if a slot should have unique constraint
    fn is_unique_constraint(slot: &SlotDefinition) -> bool {
        // Check if slot name or description indicates uniqueness
        if let Some(desc) = &slot.description {
            let desc_lower = desc.to_lowercase();
            if desc_lower.contains("unique")
                || desc_lower.contains("identifier")
                || desc_lower.contains("primary key")
            {
                return true;
            }
        }

        // Check for certain patterns that suggest uniqueness
        let name_lower = slot.name.to_lowercase();
        name_lower.ends_with("_id")
            || name_lower.ends_with("_code")
            || name_lower.ends_with("_uuid")
            || name_lower == "urn"
            || name_lower == "doi"
            || name_lower == "isbn"
    }

    /// Translate `LinkML` cardinality to `TypeQL` @card constraint
    fn translate_cardinality(&self, slot: &SlotDefinition) -> Option<String> {
        let min = Self::get_min_cardinality(slot);
        let max = Self::get_max_cardinality(slot);

        // Default cardinality is 0..1 for optional single-valued
        // or 1..1 for required single-valued
        // Only generate @card if different from defaults
        match (min, max) {
            (0, Some(1)) if slot.required != Some(true) => None, // Default optional
            (1, Some(1)) if slot.required == Some(true) => None, // Default required
            (min, Some(max)) => Some(format!("@card({min}..{max})")),
            (min, None) => Some(format!("@card({min}..)")),
        }
    }

    /// Get minimum cardinality for a slot
    fn get_min_cardinality(slot: &SlotDefinition) -> usize {
        usize::from(slot.required == Some(true))
    }

    /// Get maximum cardinality for a slot
    fn get_max_cardinality(slot: &SlotDefinition) -> Option<usize> {
        if slot.multivalued == Some(true) {
            // Check for explicit max cardinality
            if let Some(serde_json::Value::Number(max)) = &slot.maximum_value {
                max.as_u64().map(u64_to_usize_saturating)
            } else {
                None // Unbounded
            }
        } else {
            Some(1) // Single-valued
        }
    }

    /// Translate regex pattern to `TypeQL` format
    fn translate_regex(&mut self, pattern: &str) -> Option<String> {
        // Cache translated patterns
        if let Some(cached) = self.regex_cache.get(pattern) {
            return Some(cached.clone());
        }

        // TypeQL uses Java regex syntax, which is mostly compatible with LinkML patterns
        // But we need to escape certain characters
        let typeql_pattern = Self::escape_regex_for_typeql(pattern);

        // Validate the pattern
        if Self::is_valid_regex(&typeql_pattern) {
            let constraint = format!("regex \"{typeql_pattern}\"");
            self.regex_cache
                .insert(pattern.to_string(), constraint.clone());
            Some(constraint)
        } else {
            None
        }
    }

    /// Escape regex pattern for `TypeQL`
    fn escape_regex_for_typeql(pattern: &str) -> String {
        // TypeQL requires double quotes to be escaped in regex
        pattern.replace('"', "\\\"")
    }

    /// Validate regex pattern
    fn is_valid_regex(pattern: &str) -> bool {
        // Try to compile the regex using Rust's regex crate
        regex::Regex::new(pattern).is_ok()
    }

    /// Generate range constraints for numeric types
    #[must_use]
    pub fn translate_range_constraints(&self, slot: &SlotDefinition) -> Vec<String> {
        let mut constraints = Vec::new();

        // TypeDB 3.0 supports range constraints
        let has_min = slot.minimum_value.is_some();
        let has_max = slot.maximum_value.is_some();

        if has_min || has_max {
            let min_str = if let Some(serde_json::Value::Number(min)) = &slot.minimum_value {
                min.to_string()
            } else {
                String::new()
            };

            let max_str = if let Some(serde_json::Value::Number(max)) = &slot.maximum_value {
                max.to_string()
            } else {
                String::new()
            };

            if has_min && has_max {
                constraints.push(format!("range [{min_str}..{max_str}]"));
            } else if has_min {
                constraints.push(format!("range [{min_str}..)"));
            } else if has_max {
                constraints.push(format!("range (..{max_str}]"));
            }
        }

        constraints
    }

    /// Generate validation rules for complex constraints
    #[must_use]
    pub fn generate_validation_rule(
        &self,
        _class_name: &str,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> Option<String> {
        let mut rule_parts = Vec::new();

        // Check for enum constraints
        if let Some(enum_range) = &slot.range
            && enum_range.ends_with("Enum")
        {
            // Generate enum validation rule
            rule_parts.push(format!("# Validates {slot_name} is a valid {enum_range}"));
        }

        // Check for pattern validation beyond simple regex
        if let Some(pattern) = &slot.pattern
            && (pattern.contains('|') || pattern.contains("(?i)"))
        {
            // Complex pattern that might need a rule
            rule_parts.push(format!(
                "# Complex pattern validation for {slot_name}: {pattern}"
            ));
        }

        // Check for conditional requirements
        if slot.rules.is_some() {
            rule_parts.push(format!("# Conditional validation for {slot_name}"));
        }

        if rule_parts.is_empty() {
            None
        } else {
            Some(rule_parts.join(
                "
",
            ))
        }
    }

    /// Generate composite unique constraints as rules
    pub fn generate_composite_unique_rule(
        &self,
        class_name: &str,
        unique_key: &UniqueKeyDefinition,
        type_name_converter: &dyn Fn(&str) -> String,
    ) -> String {
        let class_typeql = type_name_converter(class_name);
        let rule_name = format!("{}-unique-{}", class_typeql, "key");

        let mut rule = String::new();
        writeln!(rule, "rule {rule_name}:").expect("writeln! to String should never fail");
        rule.push_str(
            "when {
",
        );
        writeln!(rule, "    $x isa {class_typeql};").expect("writeln! to String should never fail");
        writeln!(rule, "    $y isa {class_typeql};").expect("writeln! to String should never fail");
        rule.push_str(
            "    not { $x is $y; };
",
        );

        // Add conditions for each slot in the unique key
        for (i, slot) in unique_key.unique_key_slots.iter().enumerate() {
            let attr = type_name_converter(slot);
            writeln!(rule, "    $x has {attr} $v{i};")
                .expect("writeln! to String should never fail");
            writeln!(rule, "    $y has {attr} $v{i};")
                .expect("writeln! to String should never fail");
        }

        rule.push_str(
            "} then {
",
        );
        // Writing to String should never fail
        let _ = write!(
            rule,
            "    $x has validation-error \"Duplicate values for unique key: {}\";
",
            unique_key.unique_key_slots.join(", ")
        );
        rule.push_str(
            "};
",
        );

        rule
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cardinality_translation() {
        let translator = TypeQLConstraintTranslator::new();

        // Test required single-valued (default)
        let mut slot = SlotDefinition::default();
        slot.required = Some(true);
        slot.multivalued = Some(false);
        assert_eq!(translator.translate_cardinality(&slot), None);

        // Test optional multi-valued
        let mut slot = SlotDefinition::default();
        slot.required = Some(false);
        slot.multivalued = Some(true);
        assert_eq!(
            translator.translate_cardinality(&slot),
            Some("@card(0..)".to_string())
        );

        // Test required multi-valued with max
        let mut slot = SlotDefinition::default();
        slot.required = Some(true);
        slot.multivalued = Some(true);
        slot.maximum_value = Some(serde_json::json!(5));
        assert_eq!(
            translator.translate_cardinality(&slot),
            Some("@card(1..5)".to_string())
        );
    }

    #[test]
    fn test_unique_detection() {
        // Test ID-like names
        let mut slot = SlotDefinition::default();
        slot.name = "user_id".to_string();
        assert!(TypeQLConstraintTranslator::is_unique_constraint(&slot));

        // Test description-based detection
        let mut slot = SlotDefinition::default();
        slot.name = "email".to_string();
        slot.description = Some("Unique email address".to_string());
        assert!(TypeQLConstraintTranslator::is_unique_constraint(&slot));

        // Test non-unique
        let mut slot = SlotDefinition::default();
        slot.name = "name".to_string();
        assert!(!TypeQLConstraintTranslator::is_unique_constraint(&slot));
    }

    #[test]
    fn test_regex_translation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut translator = TypeQLConstraintTranslator::new();

        // Test simple pattern
        let pattern = r"^\d{3}-\d{2}-\d{4}$";
        let result = translator.translate_regex(pattern);
        assert!(result.is_some());
        assert!(
            result
                .expect("should have regex constraint: {}")
                .contains("regex")
        );

        // Test pattern with quotes
        let pattern = r#"^"[A-Z]+"$"#;
        let result = translator.translate_regex(pattern);
        assert!(result.is_some());
        assert!(
            result
                .expect("should have regex constraint with quotes: {}")
                .contains(r#"\""#)
        );
        Ok(())
    }
}
