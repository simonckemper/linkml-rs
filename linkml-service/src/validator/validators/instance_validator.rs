//! Instance-based validation for permissible values

use super::{ValidationContext, ValidationIssue, Validator};
use crate::validator::instance_loader::{InstanceConfig, InstanceLoader};
use linkml_core::types::SlotDefinition;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Validator that checks values against instance data
pub struct InstanceValidator {
    name: String,
    /// Instance loader
    _loader: Arc<InstanceLoader>,
    /// Configuration for slots
    slot_configs: HashMap<String, InstanceConfig>,
}

impl InstanceValidator {
    /// Create a new instance validator
    #[must_use]
    pub fn new(loader: Arc<InstanceLoader>) -> Self {
        Self {
            name: "instance_validator".to_string(),
            _loader: loader,
            slot_configs: HashMap::new(),
        }
    }

    /// Add configuration for a specific slot
    pub fn add_slot_config(&mut self, slot_name: String, config: InstanceConfig) {
        self.slot_configs.insert(slot_name, config);
    }

    /// Check if a value is in the permissible values
    fn check_value(&self, value: &str, slot_name: &str, context: &ValidationContext) -> bool {
        // Check slot-specific configuration
        if let Some(config) = self.slot_configs.get(slot_name) {
            // Apply any custom validation from config
            if !config.is_valid() {
                return false;
            }
        }

        // Check if we have instance data for this slot
        if let Some(instance_data) = context.instance_data.as_ref() {
            if let Some(values) = instance_data.get(slot_name) {
                return values.contains(&value.to_string());
            }
        }

        // No instance data means all values are allowed
        true
    }

    /// Validate a value against instance data
    fn validate_instance_value(
        &self,
        value: &str,
        slot: &SlotDefinition,
        path: &str,
        context: &ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if !self.check_value(value, &slot.name, context) {
            // Get available values for better error message
            let available = if let Some(instance_data) = context.instance_data.as_ref() {
                instance_data
                    .get(&slot.name)
                    .map_or_else(
                        || "none".to_string(),
                        |values| {
                            let preview: Vec<_> = values.iter().take(5).cloned().collect();
                            if values.len() > 5 {
                                format!("{:?} (and {} more)", preview, values.len() - 5)
                            } else {
                                format!("{preview:?}")
                            }
                        }
                    )
            } else {
                "no instance data loaded".to_string()
            };

            issues.push(ValidationIssue::error(
                format!(
                    "Value '{value}' is not in the permissible instance values. Available: {available}"
                ),
                path,
                &self.name,
            ));
        }

        issues
    }
}

impl Validator for InstanceValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Only validate if slot has instance-based permissible values
        // This could be indicated by a special marker in the slot definition
        // For now, check if we have instance data for this slot
        let has_instance_data = context
            .instance_data
            .as_ref()
            .is_some_and(|data| data.contains_key(&slot.name));

        if !has_instance_data {
            return issues; // No instance validation needed
        }

        if slot.multivalued.unwrap_or(false) {
            if let Some(array) = value.as_array() {
                for (i, element) in array.iter().enumerate() {
                    let path = format!("{}[{}]", context.path(), i);
                    if let Some(s) = element.as_str() {
                        issues.extend(self.validate_instance_value(s, slot, &path, context));
                    } else if !element.is_null() {
                        issues.push(ValidationIssue::error(
                            "Instance validation requires string value",
                            &path,
                            &self.name,
                        ));
                    }
                }
            }
        } else {
            let path = context.path();
            if let Some(s) = value.as_str() {
                issues.extend(self.validate_instance_value(s, slot, &path, context));
            } else if !value.is_null() {
                issues.push(ValidationIssue::error(
                    "Instance validation requires string value",
                    &path,
                    &self.name,
                ));
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
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn test_instance_validation() {
        let loader = Arc::new(InstanceLoader::new());
        let validator = InstanceValidator::new(loader);

        // Create test context with instance data
        let schema = Arc::new(linkml_core::types::SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        // Add instance data
        let mut instance_values = HashMap::new();
        instance_values.insert(
            "country".to_string(),
            vec!["US".to_string(), "UK".to_string(), "CA".to_string()],
        );
        context.instance_data = Some(Arc::new(instance_values));

        // Test valid value
        let slot = SlotDefinition {
            name: "country".to_string(),
            ..Default::default()
        };

        let value = json!("US");
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Test invalid value
        let value = json!("XX");
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
        assert!(
            issues[0]
                .message
                .contains("not in the permissible instance values")
        );
    }
}
