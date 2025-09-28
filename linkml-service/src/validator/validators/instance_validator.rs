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
    /// Instance loader for loading permissible values from external sources
    loader: Arc<InstanceLoader>,
    /// Configuration for slots
    slot_configs: HashMap<String, InstanceConfig>,
    /// Cache of loaded instance data per slot
    loaded_data_cache: Arc<dashmap::DashMap<String, Arc<Vec<String>>>>,
}

impl InstanceValidator {
    /// Create a new instance validator
    #[must_use]
    pub fn new(loader: Arc<InstanceLoader>) -> Self {
        Self {
            name: "instance_validator".to_string(),
            loader,
            slot_configs: HashMap::new(),
            loaded_data_cache: Arc::new(dashmap::DashMap::new()),
        }
    }

    /// Add configuration for a specific slot
    pub fn add_slot_config(&mut self, slot_name: String, config: InstanceConfig) {
        self.slot_configs.insert(slot_name, config);
    }

    /// Load instance data for a slot from a file (if configured)
    ///
    /// # Errors
    ///
    /// Returns error if the file cannot be read or parsed, if the slot configuration
    /// is invalid, or if there are I/O issues accessing the specified file.
    async fn load_instance_data_for_slot(
        &self,
        slot_name: &str,
        file_path: &str,
    ) -> Result<Arc<Vec<String>>, String> {
        // Check cache first
        if let Some(cached) = self.loaded_data_cache.get(slot_name) {
            return Ok(Arc::clone(&cached));
        }

        // Get configuration for this slot
        let config = self
            .slot_configs
            .get(slot_name)
            .ok_or_else(|| format!("No configuration for slot '{slot_name}'"))?;

        // Load data using the loader
        let instance_data = self
            .loader
            .load_json_file(file_path, config)
            .await
            .map_err(|e| format!("Failed to load instance data: {e}"))?;

        // Extract values for this slot
        let values = instance_data
            .values
            .get(slot_name)
            .cloned()
            .unwrap_or_default();

        let values_arc = Arc::new(values);

        // Cache the loaded data
        self.loaded_data_cache
            .insert(slot_name.to_string(), Arc::clone(&values_arc));

        Ok(values_arc)
    }

    /// Check if a value is in the permissible values
    fn check_value(&self, value: &str, slot_name: &str, context: &ValidationContext) -> bool {
        // Check slot-specific configuration
        if let Some(config) = self.slot_configs.get(slot_name) {
            // Apply any custom validation from config
            if !config.is_valid() {
                return false;
            }

            // First check the cache for loaded data
            if let Some(cached_values) = self.loaded_data_cache.get(slot_name) {
                return cached_values.contains(&value.to_string());
            }
        }

        // Check if we have instance data for this slot in context
        if let Some(instance_data) = context.instance_data.as_ref()
            && let Some(values) = instance_data.get(slot_name)
        {
            return values.contains(&value.to_string());
        }

        // If we have configuration but no data loaded yet, this is a validation failure
        // (data should have been loaded in the validate method)
        if self.slot_configs.contains_key(slot_name) {
            return false; // Value not found in required instance data
        }

        // No instance validation configured means all values are allowed
        true
    }

    /// Load instance data and validate against it
    fn load_and_validate_instance_data(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Try to load instance data using the loader if configured for this slot
        if let Some(_config) = self.slot_configs.get(&slot.name) {
            let runtime = tokio::runtime::Handle::try_current();

            if let Ok(handle) = runtime {
                issues.extend(self.load_with_existing_runtime(value, slot, context, handle));
            } else {
                issues.extend(self.load_with_new_runtime(value, slot, context));
            }
        }

        issues
    }

    /// Load instance data using existing async runtime
    fn load_with_existing_runtime(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
        handle: tokio::runtime::Handle,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let file_path = "instance_data.json"; // Default filename for instance data

        // Use the async loader method synchronously
        let load_result = handle.block_on(async {
            self.load_instance_data_for_slot(&slot.name, file_path)
                .await
        });

        match load_result {
            Ok(loaded_values) => {
                // Validate the value against loaded data
                if let Some(val_str) = value.as_str()
                    && !loaded_values.contains(&val_str.to_string())
                {
                    let preview: Vec<_> = loaded_values.iter().take(5).cloned().collect();
                    let available = if loaded_values.len() > 5 {
                        format!("{:?} (and {} more)", preview, loaded_values.len() - 5)
                    } else {
                        format!("{preview:?}")
                    };

                    issues.push(ValidationIssue::error(
                            format!("Value '{val_str}' not in loaded instance values. Available: {available}"),
                            context.path(),
                            &self.name,
                        ));
                }
                // Store in context for future use
                Self::store_instance_data_in_context(slot, &loaded_values, context);
            }
            Err(e) => {
                issues.push(ValidationIssue::warning(
                    format!("Failed to load instance data from {file_path}: {e}"),
                    context.path(),
                    &self.name,
                ));
            }
        }

        issues
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
                instance_data.get(&slot.name).map_or_else(
                    || "none".to_string(),
                    |values| {
                        let preview: Vec<_> = values.iter().take(5).cloned().collect();
                        if values.len() > 5 {
                            format!("{:?} (and {} more)", preview, values.len() - 5)
                        } else {
                            format!("{preview:?}")
                        }
                    },
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

    /// Load instance data using new runtime
    fn load_with_new_runtime(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Not in async context, need to create runtime
        let rt = tokio::runtime::Runtime::new();
        if let Ok(runtime) = rt {
            let file_path = "instance_data.json"; // Default filename for instance data
            let load_result = runtime.block_on(async {
                self.load_instance_data_for_slot(&slot.name, file_path)
                    .await
            });

            match load_result {
                Ok(loaded_values) => {
                    if let Some(val_str) = value.as_str()
                        && !loaded_values.contains(&val_str.to_string())
                    {
                        issues.push(ValidationIssue::error(
                            format!("Value '{val_str}' not in loaded instance values"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
                Err(e) => {
                    issues.push(ValidationIssue::warning(
                        format!("Failed to load instance data: {e}"),
                        context.path(),
                        &self.name,
                    ));
                }
            }
        }

        issues
    }

    /// Store loaded instance data in validation context
    fn store_instance_data_in_context(
        slot: &SlotDefinition,
        loaded_values: &Arc<Vec<String>>,
        context: &mut ValidationContext,
    ) {
        if let Some(instance_data) = context.instance_data.as_mut() {
            if let Some(data) = Arc::get_mut(instance_data) {
                data.insert(slot.name.clone(), (**loaded_values).clone());
            }
        } else {
            let mut new_data = HashMap::new();
            new_data.insert(slot.name.clone(), (**loaded_values).clone());
            context.instance_data = Some(Arc::new(new_data));
        }
    }

    /// Validate multivalued instance data
    fn validate_multivalued_instance(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

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

        issues
    }

    /// Validate single instance data
    fn validate_single_instance(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
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

        // Check if we have instance data in context or need to load it
        let has_instance_data = context
            .instance_data
            .as_ref()
            .is_some_and(|data| data.contains_key(&slot.name));

        if !has_instance_data {
            issues.extend(self.load_and_validate_instance_data(value, slot, context));
            return issues;
        }

        // Validate against existing instance data
        if slot.multivalued.unwrap_or(false) {
            issues.extend(self.validate_multivalued_instance(value, slot, context));
        } else {
            issues.extend(self.validate_single_instance(value, slot, context));
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

    #[test]
    fn test_instance_validation() {
        let timestamp_service = Arc::new(timestamp_service::factory::create_timestamp_service());
        let loader = Arc::new(InstanceLoader::new(timestamp_service));
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
