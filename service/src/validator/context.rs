//! Validation context for tracking state during validation

use super::buffer_pool::ValidationBufferPools;
use super::compiled::CompiledValidator;
use super::json_path::{JsonNavigator, JsonPath};
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Validation context that tracks state during validation
pub struct ValidationContext {
    /// The LinkML schema being used for validation
    pub schema: Arc<SchemaDefinition>,
    /// Current `JSON` path being validated
    pub current_path: Vec<String>,
    /// Optimized `JSON` path object
    path_object: JsonPath,
    /// `JSON` path navigator for efficient traversal
    navigator: JsonNavigator,
    /// Stack of classes being validated (for inheritance)
    pub class_stack: Vec<String>,
    /// Cached compiled validators
    pub validator_cache: Arc<RwLock<HashMap<String, CompiledValidator>>>,
    /// Instance data for permissible values
    pub instance_data: Option<Arc<HashMap<String, Vec<String>>>>,
    /// Additional context data
    pub data: HashMap<String, serde_json::Value>,
    /// Buffer pools for efficient memory reuse
    pub buffer_pools: Arc<ValidationBufferPools>,
    /// Parent value in the validation tree
    parent_value: Option<serde_json::Value>,
    /// Root value being validated
    root_value: Option<serde_json::Value>,
    /// All instances for cross-reference validation
    pub all_instances: Option<Vec<serde_json::Value>>,
    /// Current instance ID for circular reference detection
    pub current_instance_id: Option<String>,
}

impl ValidationContext {
    /// Create a new validation context
    #[must_use]
    pub fn new(schema: Arc<SchemaDefinition>) -> Self {
        Self {
            schema,
            current_path: Vec::new(),
            path_object: JsonPath::root(),
            navigator: JsonNavigator::new(),
            class_stack: Vec::new(),
            validator_cache: Arc::new(RwLock::new(HashMap::new())),
            instance_data: None,
            data: HashMap::new(),
            buffer_pools: Arc::new(ValidationBufferPools::new()),
            parent_value: None,
            root_value: None,
            all_instances: None,
            current_instance_id: None,
        }
    }

    /// Create a new validation context with shared buffer pools
    #[must_use]
    pub fn with_buffer_pools(
        schema: Arc<SchemaDefinition>,
        buffer_pools: Arc<ValidationBufferPools>,
    ) -> Self {
        Self {
            schema,
            current_path: Vec::new(),
            path_object: JsonPath::root(),
            navigator: JsonNavigator::new(),
            class_stack: Vec::new(),
            validator_cache: Arc::new(RwLock::new(HashMap::new())),
            instance_data: None,
            data: HashMap::new(),
            buffer_pools,
            parent_value: None,
            root_value: None,
            all_instances: None,
            current_instance_id: None,
        }
    }

    /// Get the current `JSON` path as a string
    #[must_use]
    pub fn path(&self) -> String {
        self.path_object.to_string()
    }

    /// Get the optimized path object
    #[must_use]
    pub fn path_object(&self) -> &JsonPath {
        &self.path_object
    }

    /// Push a new path segment
    pub fn push_path(&mut self, segment: impl Into<String>) {
        let segment_str = segment.into();
        self.current_path.push(segment_str.clone());
        self.path_object.property(&segment_str);
    }

    /// Push an array index
    pub fn push_index(&mut self, index: usize) {
        let segment = format!("[{index}]");
        self.current_path.push(segment);
        self.path_object.index(index);
    }

    /// Pop the last path segment
    pub fn pop_path(&mut self) {
        self.current_path.pop();
        // Rebuild path object from current_path
        self.rebuild_path_object();
    }

    /// Rebuild the path object from `current_path`
    fn rebuild_path_object(&mut self) {
        self.path_object = JsonPath::root();
        for segment in &self.current_path {
            if segment.starts_with('[') && segment.ends_with(']') {
                // Array index
                if let Ok(index) = segment[1..segment.len() - 1].parse::<usize>() {
                    self.path_object.index(index);
                }
            } else {
                // Property
                self.path_object.property(segment);
            }
        }
    }

    /// Execute a closure with a path segment pushed
    pub fn with_path<F, R>(&mut self, segment: impl Into<String>, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.push_path(segment);
        let result = f(self);
        self.pop_path();
        result
    }

    /// Push a class to the stack
    pub fn push_class(&mut self, class_name: impl Into<String>) {
        self.class_stack.push(class_name.into());
    }

    /// Pop a class from the stack
    pub fn pop_class(&mut self) {
        self.class_stack.pop();
    }

    /// Get the current recursion depth
    #[must_use]
    pub fn current_depth(&self) -> usize {
        self.class_stack.len()
    }

    /// Get the current class being validated
    #[must_use]
    pub fn current_class(&self) -> Option<&str> {
        self.class_stack.last().map(std::string::String::as_str)
    }

    /// Get a class definition by name
    #[must_use]
    pub fn get_class(&self, name: &str) -> Option<&ClassDefinition> {
        self.schema.classes.get(name)
    }

    /// Get a slot definition by name
    #[must_use]
    pub fn get_slot(&self, name: &str) -> Option<&SlotDefinition> {
        self.schema.slots.get(name)
    }

    /// Get effective slots for a class (including inherited)
    #[must_use]
    pub fn get_effective_slots<'a>(
        &'a self,
        class_name: &str,
    ) -> Vec<(&'a str, &'a SlotDefinition)> {
        let mut slots = Vec::new();
        let mut visited = std::collections::HashSet::new();
        self.collect_slots_recursive(class_name, &mut slots, &mut visited);
        slots
    }

    /// Recursively collect slots including from parent classes
    fn collect_slots_recursive<'a>(
        &'a self,
        class_name: &str,
        slots: &mut Vec<(&'a str, &'a SlotDefinition)>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        if !visited.insert(class_name.to_string()) {
            return; // Already visited
        }

        if let Some(class) = self.get_class(class_name) {
            // First collect from parent
            if let Some(parent) = &class.is_a {
                self.collect_slots_recursive(parent, slots, visited);
            }

            // Then add this class's slots
            for slot_name in &class.slots {
                if let Some(slot_def) = self.get_slot(slot_name) {
                    // Check if we should override an inherited slot
                    if let Some(pos) = slots.iter().position(|(name, _)| name == slot_name) {
                        slots[pos] = (slot_name, slot_def);
                    } else {
                        slots.push((slot_name, slot_def));
                    }
                }
            }

            // Apply slot usage overrides
            for (slot_name, _usage) in &class.slot_usage {
                if let Some(_pos) = slots.iter().position(|(name, _)| name == slot_name) {
                    // Merge usage with base slot definition
                    // For now, just note that we'd merge here in a real implementation
                    // This would involve creating a new SlotDefinition with overrides applied
                }
            }
        }
    }

    /// Check if a value is in the permissible values from instance data
    #[must_use]
    pub fn check_instance_permissible(&self, key: &str, value: &str) -> bool {
        if let Some(instance_data) = &self.instance_data
            && let Some(values) = instance_data.get(key)
        {
            return values.contains(&value.to_string());
        }
        true // If no instance data, allow all values
    }

    /// Store data in the context
    pub fn set_data(&mut self, key: &str, value: serde_json::Value) {
        self.data.insert(key.to_string(), value);
    }

    /// Get data from the context
    #[must_use]
    pub fn get_data(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Set the parent value for expression evaluation
    pub fn set_parent(&mut self, value: serde_json::Value) {
        self.parent_value = Some(value);
    }

    /// Get the parent value
    #[must_use]
    pub fn parent(&self) -> Option<&serde_json::Value> {
        self.parent_value.as_ref()
    }

    /// Set the root value for expression evaluation
    pub fn set_root(&mut self, value: serde_json::Value) {
        self.root_value = Some(value);
    }

    /// Get the root value
    #[must_use]
    pub fn root(&self) -> Option<&serde_json::Value> {
        self.root_value.as_ref()
    }

    /// Add an informational message (non-error)
    pub fn add_info(&mut self, message: impl Into<String>) {
        // For now, just store in data - could be used for reporting later
        let info_key = format!("info.{}", self.path());
        let message = message.into();

        if let Some(existing) = self.data.get_mut(&info_key) {
            if let Some(array) = existing.as_array_mut() {
                array.push(serde_json::Value::String(message));
            }
        } else {
            self.data.insert(
                info_key,
                serde_json::Value::Array(vec![serde_json::Value::String(message)]),
            );
        }
    }

    /// Get capture groups for a specific slot
    #[must_use]
    pub fn get_captures(
        &self,
        slot_name: &str,
    ) -> Option<&serde_json::Map<String, serde_json::Value>> {
        self.get_data(&format!("captures.{slot_name}"))
            .and_then(|v| v.as_object())
    }

    /// Navigate to values using a `JSON` path
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn navigate_path<'a>(
        &mut self,
        value: &'a serde_json::Value,
        path: &str,
    ) -> Result<Vec<(&'a serde_json::Value, String)>, linkml_core::error::LinkMLError> {
        self.navigator.navigate(value, path)
    }

    /// Get the `JSON` navigator
    pub fn navigator(&mut self) -> &mut JsonNavigator {
        &mut self.navigator
    }

    /// Set all instances for cross-reference validation
    pub fn set_all_instances(&mut self, instances: Vec<serde_json::Value>) {
        self.all_instances = Some(instances);
    }

    /// Set current instance ID for circular reference detection
    pub fn set_current_instance_id(&mut self, id: String) {
        self.current_instance_id = Some(id);
    }

    /// Clear current instance ID
    pub fn clear_current_instance_id(&mut self) {
        self.current_instance_id = None;
    }

    /// Get the current slot name being validated
    #[must_use]
    pub fn current_slot(&self) -> Option<&str> {
        self.current_path.last().map(std::string::String::as_str)
    }

    /// Check if a sibling field exists in the current object
    #[must_use]
    pub fn has_sibling_field(&self, field_name: &str) -> bool {
        // Check if we have parent value and can find the sibling field
        if let Some(parent) = &self.parent_value
            && let Some(obj) = parent.as_object()
        {
            return obj.contains_key(field_name);
        }
        false
    }

    /// Get a sibling field value from the current object
    #[must_use]
    pub fn get_sibling_field(&self, field_name: &str) -> Option<&serde_json::Value> {
        // Get sibling field from parent object
        if let Some(parent) = &self.parent_value
            && let Some(obj) = parent.as_object()
        {
            return obj.get(field_name);
        }
        None
    }

    /// Set the current object being validated (for sibling field access)
    pub fn set_current_object(&mut self, value: serde_json::Value) {
        self.parent_value = Some(value);
    }
}
