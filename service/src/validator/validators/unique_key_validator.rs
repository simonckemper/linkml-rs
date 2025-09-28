//! Unique key validators for `LinkML`
//!
//! This module implements validators for unique key constraints including
//! single-field uniqueness, composite keys, and scoped uniqueness.

use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use crate::validator::{context::ValidationContext, report::ValidationIssue};

use super::Validator;

/// Tracks unique values seen for validation
#[derive(Default)]
pub struct UniqueValueTracker {
    /// Maps from class name to unique key name to set of seen value combinations
    seen_values: HashMap<String, HashMap<String, HashSet<String>>>,
}

impl UniqueValueTracker {
    /// Create a new unique value tracker
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a value combination has been seen before
    /// Returns true if this is a duplicate
    pub fn check_and_record(
        &mut self,
        class_name: &str,
        unique_key_name: &str,
        value_key: String,
    ) -> bool {
        let class_values = self.seen_values.entry(class_name.to_string()).or_default();

        let key_values = class_values.entry(unique_key_name.to_string()).or_default();

        // Returns false if inserted (new value), true if already existed (duplicate)
        !key_values.insert(value_key)
    }

    /// Clear all tracked values
    pub fn clear(&mut self) {
        self.seen_values.clear();
    }

    /// Clear values for a specific class
    pub fn clear_class(&mut self, class_name: &str) {
        self.seen_values.remove(class_name);
    }
}

/// Validator for unique key constraints
pub struct UniqueKeyValidator {
    tracker: Mutex<UniqueValueTracker>,
}

impl Default for UniqueKeyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl UniqueKeyValidator {
    /// Create a new unique key validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            tracker: Mutex::new(UniqueValueTracker::new()),
        }
    }

    /// Extract the value for a slot from an instance
    fn get_slot_value<'a>(instance: &'a Value, slot_name: &str) -> Option<&'a Value> {
        match instance {
            Value::Object(map) => map.get(slot_name),
            _ => None,
        }
    }

    /// Create a composite key string from multiple slot values
    fn create_composite_key(
        instance: &Value,
        slots: &[String],
        consider_nulls_inequal: bool,
    ) -> Option<String> {
        let mut key_parts = Vec::new();

        for slot_name in slots {
            match Self::get_slot_value(instance, slot_name) {
                Some(Value::Null) | None => {
                    if consider_nulls_inequal {
                        // Each null is considered unique
                        key_parts.push(format!("__null_{}__", uuid::Uuid::new_v4()));
                    } else {
                        // Null values make the entire key null (not unique)
                        return None;
                    }
                }
                Some(value) => {
                    // Convert value to a stable string representation
                    let value_str = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => serde_json::to_string(value).unwrap_or_else(|_| "?".to_string()),
                    };
                    key_parts.push(value_str);
                }
            }
        }

        // Join with a separator that's unlikely to appear in values
        Some(key_parts.join("\u{001F}")) // Unit separator character
    }

    /// Validate unique keys for a class instance
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if validation fails or internal state cannot be accessed.
    ///
    /// # Panics
    ///
    /// Panics if the internal tracker mutex is poisoned.
    pub fn validate_class(
        &self,
        instance: &Value,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
        instance_path: &str,
    ) -> Result<Vec<ValidationIssue>, Box<dyn std::error::Error>> {
        let mut issues = Vec::new();
        let mut tracker = self
            .tracker
            .lock()
            .expect("tracker mutex should not be poisoned: {}");

        // Check identifier slot (if present)
        if let Some(identifier_slot) = class_def.slots.iter().find(|slot_name| {
            schema
                .slots
                .get(*slot_name)
                .and_then(|s| s.identifier)
                .unwrap_or(false)
        }) && let Some(value) = Self::get_slot_value(instance, identifier_slot)
            && !matches!(value, Value::Null)
        {
            let key = serde_json::to_string(value).unwrap_or_else(|_| value.to_string());

            if tracker.check_and_record(&class_def.name, "__identifier__", key.clone()) {
                issues.push(
                    ValidationIssue::error(
                        format!("Duplicate identifier value '{key}' for slot '{identifier_slot}'"),
                        instance_path,
                        "UniqueKeyValidator",
                    )
                    .with_code("DUPLICATE_IDENTIFIER")
                    .with_context("slot", serde_json::json!(identifier_slot))
                    .with_context("value", value.clone()),
                );
            }
        }

        // Check unique_keys constraints
        for (key_name, unique_key_def) in &class_def.unique_keys {
            if unique_key_def.unique_key_slots.is_empty() {
                continue;
            }

            let consider_nulls_inequal = unique_key_def.consider_nulls_inequal.unwrap_or(true);

            if let Some(composite_key) = Self::create_composite_key(
                instance,
                &unique_key_def.unique_key_slots,
                consider_nulls_inequal,
            ) && tracker.check_and_record(&class_def.name, key_name, composite_key.clone())
            {
                let slot_values: HashMap<String, Value> = unique_key_def
                    .unique_key_slots
                    .iter()
                    .filter_map(|slot| {
                        Self::get_slot_value(instance, slot).map(|v| (slot.clone(), v.clone()))
                    })
                    .collect();

                issues.push(
                    ValidationIssue::error(
                        format!(
                            "Duplicate values for unique key '{}' on slots: {}",
                            key_name,
                            unique_key_def.unique_key_slots.join(", ")
                        ),
                        instance_path,
                        "UniqueKeyValidator",
                    )
                    .with_code("DUPLICATE_UNIQUE_KEY")
                    .with_context("unique_key_name", serde_json::json!(key_name))
                    .with_context(
                        "unique_key_slots",
                        serde_json::json!(unique_key_def.unique_key_slots),
                    )
                    .with_context("duplicate_values", serde_json::json!(slot_values)),
                );
            }
        }

        Ok(issues)
    }

    /// Reset the validator's state (clear all tracked values)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the reset operation fails.
    ///
    /// # Panics
    ///
    /// Panics if the internal tracker mutex is poisoned.
    pub fn reset(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.tracker
            .lock()
            .expect("tracker mutex should not be poisoned: {}")
            .clear();
        Ok(())
    }

    /// Reset tracking for a specific class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns an error if the reset operation fails.
    ///
    /// # Panics
    ///
    /// Panics if the internal tracker mutex is poisoned.
    pub fn reset_class(&mut self, class_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.tracker
            .lock()
            .expect("tracker mutex should not be poisoned")
            .clear_class(class_name);
        Ok(())
    }

    /// Public method for validating an instance (read-only access)
    /// This wraps the internal mutable method for use in engine
    pub fn validate_instance(
        &self,
        instance: &Value,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        match self.validate_class(instance, class_def, schema, &context.path()) {
            Ok(issues) => issues,
            Err(e) => vec![ValidationIssue::error(
                context.path(),
                format!("Unique key validation error: {e}"),
                "UniqueKeyValidator",
            )],
        }
    }
}

impl Validator for UniqueKeyValidator {
    fn validate(
        &self,
        _value: &Value,
        _slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        // This validator works at the collection level, not individual slot level
        vec![ValidationIssue::warning(
            "UniqueKeyValidator should be used for collection validation, not slot validation",
            context.path(),
            self.name(),
        )]
    }

    fn name(&self) -> &'static str {
        "UniqueKeyValidator"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use linkml_core::types::UniqueKeyDefinition;

    #[test]
    fn test_unique_value_tracker() {
        let mut tracker = UniqueValueTracker::new();

        // First value should not be a duplicate
        assert!(!tracker.check_and_record("Person", "ssn", "123-45-6789".to_string()));

        // Same value should be a duplicate
        assert!(tracker.check_and_record("Person", "ssn", "123-45-6789".to_string()));

        // Different value should not be a duplicate
        assert!(!tracker.check_and_record("Person", "ssn", "987-65-4321".to_string()));

        // Same value in different class should not be a duplicate
        assert!(!tracker.check_and_record("Employee", "ssn", "123-45-6789".to_string()));

        // Same value for different key should not be a duplicate
        assert!(!tracker.check_and_record("Person", "email", "123-45-6789".to_string()));
    }

    #[test]
    fn test_identifier_uniqueness() -> anyhow::Result<()> {
        let validator = UniqueKeyValidator::new();
        let mut schema = SchemaDefinition::default();
        schema.slots.insert(
            "id".to_string(),
            SlotDefinition {
                name: "id".to_string(),
                identifier: Some(true),
                ..Default::default()
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["id".to_string()],
            ..Default::default()
        };

        // First instance
        let instance1 = serde_json::json!({
            "id": "person-1"
        });

        let issues1 = validator
            .validate_class(&instance1, &class_def, &schema, "$.persons[0]")
            .expect("validation failed: {}");
        assert!(issues1.is_empty());

        // Duplicate ID
        let instance2 = serde_json::json!({
            "id": "person-1"
        });

        let issues2 = validator
            .validate_class(&instance2, &class_def, &schema, "$.persons[1]")
            .expect("validation failed: {}");
        assert_eq!(issues2.len(), 1);
        assert!(issues2[0].message.contains("Duplicate identifier"));
        assert_eq!(issues2[0].code.as_deref(), Some("DUPLICATE_IDENTIFIER"));
        Ok(())
    }

    #[test]
    fn test_composite_unique_key() -> anyhow::Result<()> {
        let validator = UniqueKeyValidator::new();
        let schema = SchemaDefinition::default();

        let mut unique_keys = IndexMap::new();
        unique_keys.insert(
            "name_email".to_string(),
            UniqueKeyDefinition {
                description: Some("Name and email must be unique together".to_string()),
                unique_key_slots: vec![
                    "first_name".to_string(),
                    "last_name".to_string(),
                    "email".to_string(),
                ],
                consider_nulls_inequal: Some(true),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec![
                "first_name".to_string(),
                "last_name".to_string(),
                "email".to_string(),
            ],
            unique_keys,
            ..Default::default()
        };

        // First instance
        let instance1 = serde_json::json!({
            "first_name": "John",
            "last_name": "Doe",
            "email": "john.doe@example.com"
        });

        let issues1 = validator
            .validate_class(&instance1, &class_def, &schema, "$.persons[0]")
            .expect("validation failed: {}");
        assert!(issues1.is_empty());

        // Different person with same first name
        let instance2 = serde_json::json!({
            "first_name": "John",
            "last_name": "Smith",
            "email": "john.smith@example.com"
        });

        let issues2 = validator
            .validate_class(&instance2, &class_def, &schema, "$.persons[1]")
            .expect("validation failed: {}");
        assert!(issues2.is_empty());

        // Duplicate person
        let instance3 = serde_json::json!({
            "first_name": "John",
            "last_name": "Doe",
            "email": "john.doe@example.com"
        });

        let issues3 = validator
            .validate_class(&instance3, &class_def, &schema, "$.persons[2]")
            .expect("validation failed: {}");
        assert_eq!(issues3.len(), 1);
        assert!(
            issues3[0]
                .message
                .contains("Duplicate values for unique key 'name_email'")
        );
        assert_eq!(issues3[0].code.as_deref(), Some("DUPLICATE_UNIQUE_KEY"));
        Ok(())
    }

    #[test]
    fn test_null_handling_in_unique_keys() -> anyhow::Result<()> {
        let validator = UniqueKeyValidator::new();
        let schema = SchemaDefinition::default();

        // Test with consider_nulls_inequal = true (default)
        let mut unique_keys = IndexMap::new();
        unique_keys.insert(
            "email".to_string(),
            UniqueKeyDefinition {
                description: None,
                unique_key_slots: vec!["email".to_string()],
                consider_nulls_inequal: Some(true),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["email".to_string()],
            unique_keys,
            ..Default::default()
        };

        // Two instances with null emails should not conflict
        let instance1 = serde_json::json!({
            "email": null
        });

        let instance2 = serde_json::json!({
            "email": null
        });

        let issues1 = validator
            .validate_class(&instance1, &class_def, &schema, "$.persons[0]")
            .expect("validation failed: {}");
        assert!(issues1.is_empty());

        let issues2 = validator
            .validate_class(&instance2, &class_def, &schema, "$.persons[1]")
            .expect("validation failed: {}");
        assert!(
            issues2.is_empty(),
            "Null values should be considered unique when consider_nulls_inequal is true"
        );
        Ok(())
    }

    #[test]
    fn test_null_handling_inequal_false() -> anyhow::Result<()> {
        let validator = UniqueKeyValidator::new();
        let schema = SchemaDefinition::default();

        // Test with consider_nulls_inequal = false
        let mut unique_keys = IndexMap::new();
        unique_keys.insert(
            "email".to_string(),
            UniqueKeyDefinition {
                description: None,
                unique_key_slots: vec!["email".to_string()],
                consider_nulls_inequal: Some(false),
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["email".to_string()],
            unique_keys,
            ..Default::default()
        };

        // Instances with null values should not be checked for uniqueness
        let instance1 = serde_json::json!({
            "email": null
        });

        let instance2 = serde_json::json!({
            "email": null
        });

        let instance3 = serde_json::json!({
            "email": "test@example.com"
        });

        let instance4 = serde_json::json!({
            "email": "test@example.com"
        });

        validator
            .validate_class(&instance1, &class_def, &schema, "$.persons[0]")
            .expect("validation failed: {}");
        validator
            .validate_class(&instance2, &class_def, &schema, "$.persons[1]")
            .expect("validation failed: {}");
        validator
            .validate_class(&instance3, &class_def, &schema, "$.persons[2]")
            .expect("validation failed: {}");

        let issues = validator
            .validate_class(&instance4, &class_def, &schema, "$.persons[3]")
            .expect("validation failed: {}");
        assert_eq!(
            issues.len(),
            1,
            "Non-null duplicate values should still be caught"
        );
        Ok(())
    }

    #[test]
    fn test_multiple_unique_keys() -> anyhow::Result<()> {
        let validator = UniqueKeyValidator::new();
        let schema = SchemaDefinition::default();

        let mut unique_keys = IndexMap::new();
        unique_keys.insert(
            "ssn".to_string(),
            UniqueKeyDefinition {
                unique_key_slots: vec!["ssn".to_string()],
                ..Default::default()
            },
        );
        unique_keys.insert(
            "email".to_string(),
            UniqueKeyDefinition {
                unique_key_slots: vec!["email".to_string()],
                ..Default::default()
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["ssn".to_string(), "email".to_string()],
            unique_keys,
            ..Default::default()
        };

        // First instance
        let instance1 = serde_json::json!({
            "ssn": "123-45-6789",
            "email": "john@example.com"
        });

        validator
            .validate_class(&instance1, &class_def, &schema, "$.persons[0]")
            .expect("validation failed: {}");

        // Different SSN but same email - should fail
        let instance2 = serde_json::json!({
            "ssn": "987-65-4321",
            "email": "john@example.com"
        });

        let issues2 = validator
            .validate_class(&instance2, &class_def, &schema, "$.persons[1]")
            .expect("validation failed: {}");
        assert_eq!(issues2.len(), 1);
        assert!(issues2[0].message.contains("email"));

        // Same SSN but different email - should fail
        let instance3 = serde_json::json!({
            "ssn": "123-45-6789",
            "email": "jane@example.com"
        });

        let issues3 = validator
            .validate_class(&instance3, &class_def, &schema, "$.persons[2]")
            .expect("validation failed: {}");
        assert_eq!(issues3.len(), 1);
        assert!(issues3[0].message.contains("ssn"));
        Ok(())
    }

    #[test]
    fn test_reset_functionality() -> anyhow::Result<()> {
        let mut validator = UniqueKeyValidator::new();
        let schema = SchemaDefinition::default();

        let mut unique_keys = IndexMap::new();
        unique_keys.insert(
            "id".to_string(),
            UniqueKeyDefinition {
                unique_key_slots: vec!["id".to_string()],
                ..Default::default()
            },
        );

        let class_def = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["id".to_string()],
            unique_keys,
            ..Default::default()
        };

        let instance = serde_json::json!({
            "id": "person-1"
        });

        // First time should pass
        let issues1 = validator
            .validate_class(&instance, &class_def, &schema, "$")
            .expect("validation failed: {}");
        assert!(issues1.is_empty());

        // Second time should fail
        let issues2 = validator
            .validate_class(&instance, &class_def, &schema, "$")
            .expect("validation failed: {}");
        assert!(!issues2.is_empty());

        // After reset, should pass again
        let _ = validator.reset();
        let issues3 = validator
            .validate_class(&instance, &class_def, &schema, "$")
            .expect("validation failed: {}");
        assert!(issues3.is_empty());
        Ok(())
    }
}
