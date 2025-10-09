//! `SlotView` - High-level API for slot introspection
//!
//! Provides a dedicated view for individual slots with all inherited
//! properties resolved, following the Kapernikov LinkML-Rust pattern.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SlotDefinition,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::view::SchemaView;

/// High-level view of a `LinkML` slot with all inherited properties resolved
///
/// This provides a denormalized view similar to Python `LinkML`'s `SlotDefinitionView`,
/// making it easier to work with slots without manually resolving inheritance.
#[derive(Debug, Clone)]
pub struct SlotView {
    name: String,

    definition: SlotDefinition,

    /// Classes that use this slot
    used_by_classes: HashSet<String>,

    /// Parent slot (`is_a` relationship)
    parent: Option<String>,

    /// All ancestor slots in the inheritance chain
    ancestors: Vec<String>,

    /// Mixin slots applied to this slot
    mixins: Vec<String>,

    /// Slot usage overrides by class
    class_overrides: HashMap<String, SlotDefinition>,
}

impl SlotView {
    /// Create a new `SlotView` for the specified slot
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn new(slot_name: &str, schema_view: Arc<SchemaView>) -> Result<Self> {
        // Get the base slot definition
        let definition = schema_view
            .get_slot(slot_name)?
            .ok_or_else(|| LinkMLError::service(format!("Slot '{slot_name}' not found")))?;

        // Find classes that use this slot
        let mut used_by_classes = HashSet::new();
        for (class_name, _) in schema_view.all_classes()? {
            let class_slots = schema_view.class_slots(&class_name)?;
            if class_slots.contains(&slot_name.to_string()) {
                used_by_classes.insert(class_name);
            }
        }

        // Get inheritance information
        let parent = definition.is_a.clone();
        let ancestors = Self::get_slot_ancestors(&definition, &schema_view)?;
        let mixins = definition.mixins.clone();

        // Collect class-specific overrides
        let mut class_overrides = HashMap::new();
        for (class_name, class_def) in schema_view.all_classes()? {
            // Check slot_usage for overrides
            if let Some(override_def) = class_def.slot_usage.get(slot_name) {
                class_overrides.insert(class_name.clone(), override_def.clone());
            }
            // Check attributes for inline definitions
            if let Some(attr_def) = class_def.attributes.get(slot_name) {
                class_overrides.insert(class_name.clone(), attr_def.clone());
            }
        }

        Ok(Self {
            name: slot_name.to_string(),
            definition,
            used_by_classes,
            parent,
            ancestors,
            mixins,
            class_overrides,
        })
    }

    /// Get slot ancestors recursively
    fn get_slot_ancestors(slot: &SlotDefinition, schema_view: &SchemaView) -> Result<Vec<String>> {
        let mut ancestors = Vec::new();
        let mut current = slot.is_a.clone();
        let mut visited = HashSet::new();

        while let Some(ref parent_name) = current.clone() {
            if visited.contains(parent_name) {
                return Err(LinkMLError::service(format!(
                    "Circular inheritance detected in slot '{parent_name}'"
                )));
            }
            visited.insert(parent_name.clone());
            ancestors.push(parent_name.clone());

            if let Some(parent_slot) = schema_view.get_slot(parent_name)? {
                current.clone_from(&parent_slot.is_a);
            } else {
                current = None;
            }
        }

        Ok(ancestors)
    }

    /// Get the slot name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the slot definition
    #[must_use]
    pub fn definition(&self) -> &SlotDefinition {
        &self.definition
    }

    /// Get the slot description
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.definition.description.as_deref()
    }

    /// Get the slot range (type)
    #[must_use]
    pub fn range(&self) -> Option<&str> {
        self.definition.range.as_deref()
    }

    /// Check if this slot is required
    #[must_use]
    pub fn is_required(&self) -> bool {
        self.definition.required.unwrap_or(false)
    }

    /// Check if this slot is multivalued
    #[must_use]
    pub fn is_multivalued(&self) -> bool {
        self.definition.multivalued.unwrap_or(false)
    }

    /// Check if this slot is an identifier
    #[must_use]
    pub fn is_identifier(&self) -> bool {
        self.definition.identifier.unwrap_or(false)
    }

    /// Get the parent slot name (`is_a` relationship)
    #[must_use]
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    /// Get all ancestor slot names
    #[must_use]
    pub fn ancestors(&self) -> &[String] {
        &self.ancestors
    }

    /// Get mixin slot names
    #[must_use]
    pub fn mixins(&self) -> &[String] {
        &self.mixins
    }

    /// Get classes that use this slot
    pub fn used_by(&self) -> impl Iterator<Item = &String> {
        self.used_by_classes.iter()
    }

    /// Check if this slot is used by a specific class
    #[must_use]
    pub fn is_used_by(&self, class_name: &str) -> bool {
        self.used_by_classes.contains(class_name)
    }

    /// Get the slot definition as resolved for a specific class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn in_class(&self, class_name: &str) -> Result<SlotDefinition> {
        // If there's a class-specific override, use it
        if let Some(override_def) = self.class_overrides.get(class_name) {
            // Merge the override with the base definition
            let mut resolved = self.definition.clone();

            // Apply overrides (only non-None values)
            if override_def.description.is_some() {
                resolved.description.clone_from(&override_def.description);
            }
            if override_def.required.is_some() {
                resolved.required = override_def.required;
            }
            if override_def.multivalued.is_some() {
                resolved.multivalued = override_def.multivalued;
            }
            if override_def.range.is_some() {
                resolved.range.clone_from(&override_def.range);
            }
            if override_def.pattern.is_some() {
                resolved.pattern.clone_from(&override_def.pattern);
            }
            if override_def.minimum_value.is_some() {
                resolved
                    .minimum_value
                    .clone_from(&override_def.minimum_value);
            }
            if override_def.maximum_value.is_some() {
                resolved
                    .maximum_value
                    .clone_from(&override_def.maximum_value);
            }

            return Ok(resolved);
        }

        // Otherwise return the base definition
        Ok(self.definition.clone())
    }

    /// Get pattern constraint if defined
    #[must_use]
    pub fn pattern(&self) -> Option<&str> {
        self.definition.pattern.as_deref()
    }

    /// Get minimum value constraint
    #[must_use]
    pub fn minimum_value(&self) -> Option<&serde_json::Value> {
        self.definition.minimum_value.as_ref()
    }

    /// Get maximum value constraint
    #[must_use]
    pub fn maximum_value(&self) -> Option<&serde_json::Value> {
        self.definition.maximum_value.as_ref()
    }

    /// Get permissible values for enum slots
    #[must_use]
    pub fn permissible_values(&self) -> &[linkml_core::types::PermissibleValue] {
        &self.definition.permissible_values
    }

    /// Check if this slot has permissible values (is an enum)
    #[must_use]
    pub fn is_enum(&self) -> bool {
        !self.definition.permissible_values.is_empty()
    }

    /// Get the slot URI
    #[must_use]
    pub fn slot_uri(&self) -> Option<&str> {
        self.definition.slot_uri.as_deref()
    }

    /// Get aliases for this slot
    #[must_use]
    pub fn aliases(&self) -> &[String] {
        &self.definition.aliases
    }

    /// Check if a string is an alias for this slot
    #[must_use]
    pub fn is_alias(&self, name: &str) -> bool {
        self.aliases().contains(&name.to_string())
    }

    /// Get annotations for this slot
    #[must_use]
    pub fn annotations(&self) -> Option<&linkml_core::annotations::Annotations> {
        self.definition.annotations.as_ref()
    }

    /// Get all class-specific overrides for this slot
    #[must_use]
    pub fn class_overrides(&self) -> &HashMap<String, SlotDefinition> {
        &self.class_overrides
    }

    /// Check if this slot has any class-specific overrides
    #[must_use]
    pub fn has_overrides(&self) -> bool {
        !self.class_overrides.is_empty()
    }

    /// Get classes where this slot is required (considering overrides)
    #[must_use]
    pub fn required_in_classes(&self) -> Vec<String> {
        let mut classes = Vec::new();

        for class_name in &self.used_by_classes {
            if let Ok(resolved) = self.in_class(class_name)
                && resolved.required.unwrap_or(false)
            {
                classes.push(class_name.clone());
            }
        }

        classes
    }

    /// Get classes where this slot is optional (considering overrides)
    #[must_use]
    pub fn optional_in_classes(&self) -> Vec<String> {
        let mut classes = Vec::new();

        for class_name in &self.used_by_classes {
            if let Ok(resolved) = self.in_class(class_name)
                && !resolved.required.unwrap_or(false)
            {
                classes.push(class_name.clone());
            }
        }

        classes
    }
}

/// Builder for creating `SlotView` instances with caching
#[derive(Debug)]
pub struct SlotViewBuilder {
    schema_view: Arc<SchemaView>,
    cache: HashMap<String, Arc<SlotView>>,
}

impl SlotViewBuilder {
    /// Create a new `SlotViewBuilder`
    #[must_use]
    pub fn new(schema_view: Arc<SchemaView>) -> Self {
        Self {
            schema_view,
            cache: HashMap::new(),
        }
    }

    /// Get or create a `SlotView` for the specified slot
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_or_create(&mut self, slot_name: &str) -> Result<Arc<SlotView>> {
        if let Some(view) = self.cache.get(slot_name) {
            return Ok(Arc::clone(view));
        }

        let view = Arc::new(SlotView::new(slot_name, Arc::clone(&self.schema_view))?);
        self.cache.insert(slot_name.to_string(), Arc::clone(&view));
        Ok(view)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
