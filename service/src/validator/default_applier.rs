//! Default value application for `LinkML` slots
//!
//! This module handles the ifabsent logic for applying default values
//! to slots when values are missing.

use crate::expression::ExpressionEngine;
use linkml_core::types::{IfAbsentAction, SchemaDefinition};
use regex::Regex;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

// Re-export chrono types for internal operations
use chrono::{Local, Utc};

// Compile regex pattern once at startup
// Using Result type to handle regex compilation errors properly
static VARIABLE_PATTERN: std::sync::LazyLock<Result<Regex, regex::Error>> =
    std::sync::LazyLock::new(|| Regex::new(r"\{([^}]+)\}"));

/// Apply default values to data based on schema definitions
pub struct DefaultApplier<'a> {
    schema: &'a SchemaDefinition,
    expression_engine: ExpressionEngine,
    timestamp_service: Option<Arc<dyn TimestampService<Error = TimestampError>>>,
}

impl<'a> DefaultApplier<'a> {
    /// Create a new default applier
    #[must_use]
    pub fn new(schema: &'a SchemaDefinition) -> Self {
        Self {
            schema,
            expression_engine: ExpressionEngine::new(),
            timestamp_service: None,
        }
    }

    /// Create a new default applier with timestamp service
    #[must_use]
    pub fn new_with_timestamp_service(
        schema: &'a SchemaDefinition,
        timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            schema,
            expression_engine: ExpressionEngine::new(),
            timestamp_service: Some(timestamp_service),
        }
    }

    /// Create from schema (alias for new)
    #[must_use]
    pub fn from_schema(schema: &'a SchemaDefinition) -> Self {
        Self::new(schema)
    }

    /// Apply defaults to a `JSON` value
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn apply_defaults(
        &self,
        data: &mut Value,
        schema: &SchemaDefinition,
    ) -> Result<(), String> {
        // If it's not an object, nothing to do
        let obj = data
            .as_object_mut()
            .ok_or_else(|| "Data must be an object".to_string())?;

        // Try to determine the class from @type field or use first class
        let class_name = obj
            .get("@type")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| schema.classes.keys().next().cloned())
            .ok_or_else(|| "Cannot determine class for defaults".to_string())?;

        self.apply_defaults_to_object(obj, &class_name)
    }

    /// Apply defaults to an object map
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn apply_defaults_to_object(
        &self,
        data: &mut serde_json::Map<String, Value>,
        class_name: &str,
    ) -> Result<(), String> {
        // Get the class definition
        let class = self
            .schema
            .classes
            .get(class_name)
            .ok_or_else(|| format!("Class '{class_name}' not found"))?;

        // Process all slots for this class
        for slot_name in &class.slots {
            // Skip if value already exists
            if data.contains_key(slot_name) {
                continue;
            }

            // Get slot definition
            if let Some(slot) = self.schema.slots.get(slot_name) {
                // Check if slot has ifabsent
                if let Some(ifabsent) = &slot.ifabsent {
                    let default_value =
                        self.compute_default_value(ifabsent, slot_name, class_name, data);

                    if let Some(value) = default_value {
                        data.insert(slot_name.clone(), value);
                    }
                }
            }
        }

        // Also check slot_usage for class-specific defaults
        for (slot_name, slot_override) in &class.slot_usage {
            if data.contains_key(slot_name) {
                continue;
            }

            if let Some(ifabsent) = &slot_override.ifabsent {
                let default_value =
                    self.compute_default_value(ifabsent, slot_name, class_name, data);

                if let Some(value) = default_value {
                    data.insert(slot_name.clone(), value);
                }
            }
        }

        Ok(())
    }

    /// Compute the default value based on `IfAbsentAction`
    fn compute_default_value(
        &self,
        action: &IfAbsentAction,
        slot_name: &str,
        class_name: &str,
        data: &serde_json::Map<String, Value>,
    ) -> Option<Value> {
        match action {
            IfAbsentAction::SlotName => {
                // Use the slot name as the value
                Some(Value::String(slot_name.to_string()))
            }

            IfAbsentAction::ClassName => {
                // Use the class name as the value
                Some(Value::String(class_name.to_string()))
            }

            IfAbsentAction::ClassSlotCurie => {
                // Create a CURIE from class and slot names
                let curie = format!("{class_name}:{slot_name}");
                Some(Value::String(curie))
            }

            IfAbsentAction::Bnode => {
                // Generate a blank node identifier
                let bnode = format!("_:b{}", uuid::Uuid::new_v4().simple());
                Some(Value::String(bnode))
            }

            IfAbsentAction::DefaultValue => {
                // Look up the default value field in the slot definition
                if let Some(slot_def) = self.schema.slots.get(slot_name) {
                    slot_def.default.clone()
                } else {
                    None
                }
            }

            IfAbsentAction::String(s) => {
                // Use the provided string
                Some(Value::String(s.clone()))
            }

            IfAbsentAction::Date => {
                // Use current date
                if let Some(ts_service) = &self.timestamp_service {
                    // Use TimestampService for getting current date
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    let local_time = rt.block_on(async { ts_service.now_local().await }).ok()?;
                    let current_date = local_time.format("%Y-%m-%d").to_string();
                    Some(Value::String(current_date))
                } else {
                    // Fallback to direct chrono call
                    let current_date = Local::now().format("%Y-%m-%d").to_string();
                    Some(Value::String(current_date))
                }
            }

            IfAbsentAction::Datetime => {
                // Use current datetime
                if let Some(ts_service) = &self.timestamp_service {
                    // Use TimestampService for getting current datetime
                    let rt = tokio::runtime::Runtime::new().ok()?;
                    let utc_time = rt.block_on(async { ts_service.now_utc().await }).ok()?;
                    let datetime = rt
                        .block_on(async { ts_service.format_iso8601(&utc_time).await })
                        .ok()?;
                    Some(Value::String(datetime))
                } else {
                    // Fallback to direct chrono call
                    let datetime = Utc::now().to_rfc3339();
                    Some(Value::String(datetime))
                }
            }

            IfAbsentAction::Int(n) => {
                // Use the provided integer
                Some(Value::Number((*n).into()))
            }

            IfAbsentAction::Expression(expr) => {
                // Evaluate the expression
                let data_hashmap: HashMap<String, Value> =
                    data.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
                Some(self.evaluate_expression(expr, &data_hashmap))
            }
        }
    }

    /// Evaluate an expression to produce a default value using the real expression engine
    fn evaluate_expression(&self, expression: &str, data: &HashMap<String, Value>) -> Value {
        // Use the real expression engine for evaluation
        match self.expression_engine.evaluate(expression, data) {
            Ok(value) => value,
            Err(_) => {
                // Fallback: Handle simple variable references like "{id}_derived"
                // for backwards compatibility with expressions that don't use full engine syntax
                if expression.contains('{') && expression.contains('}') {
                    let mut result = expression.to_string();

                    // Find all {variable} patterns
                    // Handle regex compilation error gracefully
                    if let Ok(ref pattern) = *VARIABLE_PATTERN {
                        for cap in pattern.captures_iter(expression) {
                            if let Some(var_name) = cap.get(1)
                                && let Some(value) = data.get(var_name.as_str())
                                && let Some(str_val) = value.as_str()
                            {
                                result =
                                    result.replace(&format!("{{{}}}", var_name.as_str()), str_val);
                            }
                        }
                    }

                    Value::String(result)
                } else {
                    // Return the expression as-is if no variables found
                    Value::String(expression.to_string())
                }
            }
        }
    }
}

/// Integration with the validation context
/// Returns an error if the operation fails
///
/// # Errors
///
pub fn apply_defaults_to_instance(
    schema: &SchemaDefinition,
    instance: &mut Value,
    class_name: &str,
) -> Result<(), String> {
    // Apply defaults directly to the object
    let applier = DefaultApplier::new(schema);
    if let Value::Object(map) = instance {
        applier.apply_defaults_to_object(map, class_name)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::IfAbsentAction;
    use linkml_core::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_slot_name_default() {
        let mut schema = SchemaDefinition::default();

        // Create a slot with ifabsent
        let slot = SlotDefinition {
            name: "identifier".to_string(),
            ifabsent: Some(IfAbsentAction::SlotName),
            ..Default::default()
        };
        schema.slots.insert("identifier".to_string(), slot);

        // Create a class using the slot
        let class = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["identifier".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Person".to_string(), class);

        // Create instance without the slot value
        let mut data = serde_json::json!({
            "name": "John"
        });

        // Apply defaults
        let applier = DefaultApplier::new(&schema);
        applier
            .apply_defaults(&mut data, &schema)
            .expect("Should apply defaults");

        // For class-specific defaults, use apply_defaults_to_object
        if let Value::Object(ref mut obj) = data {
            applier
                .apply_defaults_to_object(obj, "Person")
                .expect("Should apply class defaults");
        }

        // Check that default was applied
        assert_eq!(
            data.get("identifier"),
            Some(&Value::String("identifier".to_string()))
        );
    }

    #[test]
    fn test_bnode_generation() {
        let mut schema = SchemaDefinition::default();

        // Create a slot with bnode default
        let slot = SlotDefinition {
            name: "id".to_string(),
            ifabsent: Some(IfAbsentAction::Bnode),
            ..Default::default()
        };
        schema.slots.insert("id".to_string(), slot);

        // Create a class
        let class = ClassDefinition {
            name: "Entity".to_string(),
            slots: vec!["id".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Entity".to_string(), class);

        // Create two instances
        let mut data1 = serde_json::json!({});
        let mut data2 = serde_json::json!({});

        // Apply defaults
        let applier = DefaultApplier::new(&schema);
        if let Value::Object(ref mut obj) = data1 {
            applier
                .apply_defaults_to_object(obj, "Entity")
                .expect("Should apply defaults");
        }
        if let Value::Object(ref mut obj) = data2 {
            applier
                .apply_defaults_to_object(obj, "Entity")
                .expect("Should apply defaults");
        }

        // Check that different bnodes were generated
        let id1 = data1
            .get("id")
            .expect("Should have id")
            .as_str()
            .expect("Should be string");
        let id2 = data2
            .get("id")
            .expect("Should have id")
            .as_str()
            .expect("Should be string");

        assert!(id1.starts_with("_:b"));
        assert!(id2.starts_with("_:b"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_expression_default() {
        let mut schema = SchemaDefinition::default();

        // Create a slot with expression default
        let slot = SlotDefinition {
            name: "full_id".to_string(),
            ifabsent: Some(IfAbsentAction::Expression("{prefix}_{number}".to_string())),
            ..Default::default()
        };
        schema.slots.insert("full_id".to_string(), slot);

        // Create a class
        let class = ClassDefinition {
            name: "Item".to_string(),
            slots: vec![
                "full_id".to_string(),
                "prefix".to_string(),
                "number".to_string(),
            ],
            ..Default::default()
        };
        schema.classes.insert("Item".to_string(), class);

        // Create instance with partial data
        let mut data = serde_json::json!({
            "prefix": "ITEM",
            "number": "123"
        });

        // Apply defaults
        let applier = DefaultApplier::new(&schema);
        if let Value::Object(ref mut obj) = data {
            applier
                .apply_defaults_to_object(obj, "Item")
                .expect("Should apply defaults");
        }

        // Check that expression was evaluated
        assert_eq!(
            data.get("full_id"),
            Some(&Value::String("ITEM_123".to_string()))
        );
    }
}
