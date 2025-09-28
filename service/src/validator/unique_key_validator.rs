//! Unique key validation for `LinkML` collections
//!
//! This module ensures unique key constraints are enforced across collections,
//! supporting composite keys and case-insensitive matching.

use linkml_core::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

/// Unique key validator for collections
pub struct UniqueKeyValidator {
    /// Unique key definitions by class name
    unique_keys: HashMap<String, Vec<UniqueKeyDefinition>>,

    /// Case sensitivity settings by class
    case_sensitive: HashMap<String, bool>,
}

/// Definition of a unique key constraint
#[derive(Clone, Debug)]
pub struct UniqueKeyDefinition {
    /// Name of the unique key
    pub name: String,

    /// Slots that comprise the unique key
    pub unique_key_slots: Vec<String>,

    /// Whether this is a primary key
    pub is_primary: bool,

    /// Case sensitivity for string comparisons
    pub case_sensitive: bool,

    /// Whether nulls are considered equal
    pub nulls_equal: bool,
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
            unique_keys: HashMap::new(),
            case_sensitive: HashMap::new(),
        }
    }

    /// Create from a `LinkML` schema
    #[must_use]
    pub fn from_schema(schema: &SchemaDefinition) -> Self {
        let mut validator = Self::new();

        // Extract unique keys from classes
        for (class_name, class_def) in &schema.classes {
            let mut keys = Vec::new();

            // Process unique_keys map
            for (key_name, key_def) in &class_def.unique_keys {
                keys.push(UniqueKeyDefinition {
                    name: key_name.clone(),
                    unique_key_slots: key_def.unique_key_slots.clone(),
                    is_primary: key_name == "primary_key" || key_name.contains("_pk"),
                    case_sensitive: !key_def.consider_nulls_inequal.unwrap_or(true),
                    nulls_equal: !key_def.consider_nulls_inequal.unwrap_or(true),
                });
            }

            // Check for identifier slots (implicit unique key)
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && slot_def.identifier.unwrap_or(false)
                {
                    keys.push(UniqueKeyDefinition {
                        name: format!("{class_name}_id"),
                        unique_key_slots: vec![slot_name.clone()],
                        is_primary: true,
                        case_sensitive: true,
                        nulls_equal: false,
                    });
                }
            }

            if !keys.is_empty() {
                validator.unique_keys.insert(class_name.clone(), keys);
            }
        }

        validator
    }

    /// Add a unique key definition
    pub fn add_unique_key(&mut self, class_name: &str, key_def: UniqueKeyDefinition) {
        self.unique_keys
            .entry(class_name.to_string())
            .or_default()
            .push(key_def);
    }

    /// Set case sensitivity for a class
    pub fn set_case_sensitive(&mut self, class_name: &str, case_sensitive: bool) {
        self.case_sensitive
            .insert(class_name.to_string(), case_sensitive);
    }

    /// Validate unique keys in a collection
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn validate_collection(
        &self,
        instances: &[Value],
        class_name: &str,
    ) -> Result<Vec<UniqueKeyViolation>> {
        let mut violations = Vec::new();

        if let Some(key_defs) = self.unique_keys.get(class_name) {
            for key_def in key_defs {
                let key_violations = Self::check_unique_key(instances, key_def);
                violations.extend(key_violations);
            }
        }

        Ok(violations)
    }

    /// Check a single unique key constraint
    fn check_unique_key(
        instances: &[Value],
        key_def: &UniqueKeyDefinition,
    ) -> Vec<UniqueKeyViolation> {
        let mut violations = Vec::new();
        let mut seen_keys: HashMap<Vec<Value>, Vec<usize>> = HashMap::new();

        for (index, instance) in instances.iter().enumerate() {
            if let Value::Object(obj) = instance {
                // Extract key values
                let mut key_values = Vec::new();
                let mut has_null = false;

                for slot_name in &key_def.unique_key_slots {
                    let value = obj.get(slot_name).unwrap_or(&Value::Null);

                    if value == &Value::Null {
                        has_null = true;
                        if !key_def.nulls_equal {
                            // Skip this instance if nulls are not considered equal
                            break;
                        }
                    }

                    // Normalize value for comparison
                    let normalized = Self::normalize_value(value, key_def.case_sensitive);
                    key_values.push(normalized);
                }

                // Skip if we have nulls and they're not considered equal
                if has_null && !key_def.nulls_equal {
                    continue;
                }

                // Check for duplicates
                seen_keys.entry(key_values.clone()).or_default().push(index);
            }
        }

        // Report violations for duplicate keys
        for (key_values, indices) in seen_keys {
            if indices.len() > 1 {
                violations.push(UniqueKeyViolation {
                    key_name: key_def.name.clone(),
                    key_slots: key_def.unique_key_slots.clone(),
                    key_values,
                    duplicate_indices: indices,
                });
            }
        }

        violations
    }

    /// Normalize a value for comparison
    fn normalize_value(value: &Value, case_sensitive: bool) -> Value {
        match value {
            Value::String(s) if !case_sensitive => Value::String(s.to_lowercase()),
            _ => value.clone(),
        }
    }

    /// Validate a single instance for uniqueness (when adding to collection)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn validate_instance(
        &self,
        instance: &Value,
        existing: &[Value],
        class_name: &str,
    ) -> Result<Vec<UniqueKeyViolation>> {
        // Combine with existing and check
        let mut all_instances = existing.to_vec();
        all_instances.push(instance.clone());

        let violations = self.validate_collection(&all_instances, class_name)?;

        // Only return violations involving the new instance
        let new_index = existing.len();
        Ok(violations
            .into_iter()
            .filter(|v| v.duplicate_indices.contains(&new_index))
            .collect())
    }

    /// Get the primary key value(s) for an instance
    #[must_use]
    pub fn get_primary_key(&self, instance: &Value, class_name: &str) -> Option<Vec<Value>> {
        if let Some(key_defs) = self.unique_keys.get(class_name) {
            // Find the primary key
            if let Some(primary) = key_defs.iter().find(|k| k.is_primary)
                && let Value::Object(obj) = instance
            {
                let mut key_values = Vec::new();

                for slot_name in &primary.unique_key_slots {
                    let value = obj.get(slot_name).unwrap_or(&Value::Null);
                    key_values.push(value.clone());
                }

                return Some(key_values);
            }
        }

        None
    }
}

/// A unique key violation
#[derive(Debug, Clone)]
pub struct UniqueKeyViolation {
    /// Name of the violated key
    pub key_name: String,

    /// Slots that comprise the key
    pub key_slots: Vec<String>,

    /// The duplicate key values that were found
    pub key_values: Vec<Value>,

    /// Indices of instances with duplicate keys
    pub duplicate_indices: Vec<usize>,
}

impl UniqueKeyViolation {
    /// Format the violation as a user-friendly message
    #[must_use]
    pub fn message(&self) -> String {
        let _key_str = self.key_slots.join(", ");
        let value_str = self
            .key_values
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                Value::Null => "null".to_string(),
                _ => serde_json::to_string(v).unwrap_or_else(|_| "?".to_string()),
            })
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "Unique key '{}' violated: duplicate values ({}) found at indices {:?}",
            self.key_name, value_str, self.duplicate_indices
        )
    }
}

/// Build an index for fast lookups by unique key
pub struct UniqueKeyIndex {
    /// Indices by key values
    indices: HashMap<Vec<Value>, usize>,

    /// Key definition
    key_def: UniqueKeyDefinition,
}

impl UniqueKeyIndex {
    /// Create a new index
    #[must_use]
    pub fn new(key_def: UniqueKeyDefinition) -> Self {
        Self {
            indices: HashMap::new(),
            key_def,
        }
    }

    /// Build index from instances
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn build(&mut self, instances: &[Value]) -> Result<()> {
        for (index, instance) in instances.iter().enumerate() {
            if let Value::Object(obj) = instance {
                let mut key_values = Vec::new();

                for slot_name in &self.key_def.unique_key_slots {
                    let value = obj.get(slot_name).unwrap_or(&Value::Null);
                    key_values.push(value.clone());
                }

                if self.indices.contains_key(&key_values) {
                    return Err(LinkMLError::service(format!(
                        "Duplicate key found while building index: {key_values:?}"
                    )));
                }

                self.indices.insert(key_values, index);
            }
        }

        Ok(())
    }

    /// Look up an instance by key values
    #[must_use]
    pub fn lookup(&self, key_values: &[Value]) -> Option<usize> {
        self.indices.get(key_values).copied()
    }

    /// Check if key values exist
    #[must_use]
    pub fn contains(&self, key_values: &[Value]) -> bool {
        self.indices.contains_key(key_values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_unique_key() -> anyhow::Result<()> {
        let mut validator = UniqueKeyValidator::new();

        validator.add_unique_key(
            "Person",
            UniqueKeyDefinition {
                name: "person_id".to_string(),
                unique_key_slots: vec!["id".to_string()],
                is_primary: true,
                case_sensitive: true,
                nulls_equal: false,
            },
        );

        let instances = vec![
            json!({"id": "P001", "name": "Alice"}),
            json!({"id": "P002", "name": "Bob"}),
            json!({"id": "P001", "name": "Charlie"}), // Duplicate ID
        ];

        let violations = validator.validate_collection(&instances, "Person")?;

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].key_name, "person_id");
        assert_eq!(violations[0].duplicate_indices, vec![0, 2]);
        Ok(())
    }

    #[test]
    fn test_composite_unique_key() -> anyhow::Result<()> {
        let mut validator = UniqueKeyValidator::new();

        validator.add_unique_key(
            "Assignment",
            UniqueKeyDefinition {
                name: "assignment_key".to_string(),
                unique_key_slots: vec!["student_id".to_string(), "course_id".to_string()],
                is_primary: true,
                case_sensitive: true,
                nulls_equal: false,
            },
        );

        let instances = vec![
            json!({"student_id": "S001", "course_id": "C001", "grade": "A"}),
            json!({"student_id": "S001", "course_id": "C002", "grade": "B"}),
            json!({"student_id": "S002", "course_id": "C001", "grade": "A"}),
            json!({"student_id": "S001", "course_id": "C001", "grade": "B"}), // Duplicate
        ];

        let violations = validator.validate_collection(&instances, "Assignment")?;

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].duplicate_indices, vec![0, 3]);
        Ok(())
    }

    #[test]
    fn test_case_insensitive_key() -> anyhow::Result<()> {
        let mut validator = UniqueKeyValidator::new();

        validator.add_unique_key(
            "User",
            UniqueKeyDefinition {
                name: "username".to_string(),
                unique_key_slots: vec!["username".to_string()],
                is_primary: false,
                case_sensitive: false, // Case-insensitive
                nulls_equal: false,
            },
        );

        let instances = vec![
            json!({"username": "alice", "email": "alice@example.com"}),
            json!({"username": "Bob", "email": "bob@example.com"}),
            json!({"username": "ALICE", "email": "alice2@example.com"}), // Should violate
        ];

        let violations = validator.validate_collection(&instances, "User")?;

        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].duplicate_indices, vec![0, 2]);
        Ok(())
    }

    #[test]
    fn test_unique_key_index() -> anyhow::Result<()> {
        let key_def = UniqueKeyDefinition {
            name: "id_key".to_string(),
            unique_key_slots: vec!["id".to_string()],
            is_primary: true,
            case_sensitive: true,
            nulls_equal: false,
        };

        let mut index = UniqueKeyIndex::new(key_def);

        let instances = vec![
            json!({"id": "001", "name": "Alice"}),
            json!({"id": "002", "name": "Bob"}),
            json!({"id": "003", "name": "Charlie"}),
        ];

        index.build(&instances)?;

        // Test lookups
        assert_eq!(index.lookup(&[json!("002")]), Some(1));
        assert_eq!(index.lookup(&[json!("004")]), None);
        assert!(index.contains(&[json!("001")]));
        assert!(!index.contains(&[json!("999")]));
        Ok(())
    }
}
