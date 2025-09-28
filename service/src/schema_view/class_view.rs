//! `ClassView` - High-level API for class introspection
//!
//! Provides a dedicated view for individual classes with all inherited
//! properties resolved, following the Kapernikov LinkML-Rust pattern.

use linkml_core::{
    error::Result,
    types::{ClassDefinition, SlotDefinition},
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::view::SchemaView;

/// High-level view of a `LinkML` class with all inherited properties resolved
///
/// This provides a denormalized view similar to Python `LinkML`'s `ClassDefinitionView`,
/// making it easier to work with classes without manually resolving inheritance.
#[derive(Debug, Clone)]
pub struct ClassView {
    name: String,

    definition: ClassDefinition,

    /// All slots applicable to this class (including inherited)
    all_slots: Vec<String>,

    /// Map of slot names to their fully resolved definitions
    resolved_slots: HashMap<String, SlotDefinition>,

    /// Direct parent class (`is_a`)
    parent: Option<String>,

    /// All ancestor classes in the inheritance chain
    ancestors: Vec<String>,

    /// All descendant classes
    descendants: HashSet<String>,

    /// Mixin classes applied to this class
    mixins: Vec<String>,
}

impl ClassView {
    /// Create a new `ClassView` for the specified class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns schema view errors if class induction or slot resolution fails
    pub fn new(class_name: &str, schema_view: Arc<SchemaView>) -> Result<Self> {
        // Get the induced (fully resolved) class
        let definition = schema_view.induced_class(class_name)?;

        // Get all slots
        let all_slots = schema_view.class_slots(class_name)?;

        // Resolve all slots in the context of this class
        let mut resolved_slots = HashMap::new();
        for slot_name in &all_slots {
            if let Ok(resolved) = schema_view.induced_slot(slot_name, class_name) {
                resolved_slots.insert(slot_name.clone(), resolved);
            }
        }

        // Get inheritance information
        let parent = definition.is_a.clone();
        let ancestors = schema_view.class_ancestors(class_name)?;
        let descendants = schema_view
            .class_descendants(class_name)?
            .into_iter()
            .collect();
        let mixins = definition.mixins.clone();

        Ok(Self {
            name: class_name.to_string(),
            definition,
            all_slots,
            resolved_slots,
            parent,
            ancestors,
            descendants,
            mixins,
        })
    }

    /// Get the class name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the class definition
    #[must_use]
    pub fn definition(&self) -> &ClassDefinition {
        &self.definition
    }

    /// Get the class description
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.definition.description.as_deref()
    }

    /// Check if this class is abstract
    #[must_use]
    pub fn is_abstract(&self) -> bool {
        self.definition.abstract_.unwrap_or(false)
    }

    /// Check if this class is a mixin
    #[must_use]
    pub fn is_mixin(&self) -> bool {
        self.definition.mixin.unwrap_or(false)
    }

    /// Get the parent class name (`is_a` relationship)
    #[must_use]
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    /// Get all ancestor class names
    #[must_use]
    pub fn ancestors(&self) -> &[String] {
        &self.ancestors
    }

    /// Check if this class is a descendant of another class
    #[must_use]
    pub fn is_descendant_of(&self, class_name: &str) -> bool {
        self.ancestors.contains(&class_name.to_string())
    }

    /// Get all descendant class names
    pub fn descendants(&self) -> impl Iterator<Item = &String> {
        self.descendants.iter()
    }

    /// Get mixin class names
    #[must_use]
    pub fn mixins(&self) -> &[String] {
        &self.mixins
    }

    /// Get all slot names for this class (including inherited)
    #[must_use]
    pub fn slot_names(&self) -> &[String] {
        &self.all_slots
    }

    /// Get a specific slot definition resolved in the context of this class
    #[must_use]
    pub fn slot(&self, slot_name: &str) -> Option<&SlotDefinition> {
        self.resolved_slots.get(slot_name)
    }

    /// Get all slots as a map
    #[must_use]
    pub fn slots(&self) -> &HashMap<String, SlotDefinition> {
        &self.resolved_slots
    }

    /// Get only the slots defined directly on this class (not inherited)
    #[must_use]
    pub fn own_slots(&self) -> Vec<&str> {
        self.definition
            .slots
            .iter()
            .map(std::string::String::as_str)
            .collect()
    }

    /// Get only the slots inherited from parent classes
    #[must_use]
    pub fn inherited_slots(&self) -> Vec<&str> {
        self.all_slots
            .iter()
            .filter(|slot_name| !self.definition.slots.contains(*slot_name))
            .map(std::string::String::as_str)
            .collect()
    }

    /// Get required slots for this class
    #[must_use]
    pub fn required_slots(&self) -> Vec<&str> {
        self.resolved_slots
            .iter()
            .filter(|(_, slot)| slot.required.unwrap_or(false))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get optional slots for this class
    #[must_use]
    pub fn optional_slots(&self) -> Vec<&str> {
        self.resolved_slots
            .iter()
            .filter(|(_, slot)| !slot.required.unwrap_or(false))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get the identifier slot for this class, if any
    #[must_use]
    pub fn identifier_slot(&self) -> Option<&str> {
        self.resolved_slots
            .iter()
            .find(|(_, slot)| slot.identifier.unwrap_or(false))
            .map(|(name, _)| name.as_str())
    }

    /// Get slots that are multivalued (collections)
    #[must_use]
    pub fn multivalued_slots(&self) -> Vec<&str> {
        self.resolved_slots
            .iter()
            .filter(|(_, slot)| slot.multivalued.unwrap_or(false))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Get slots with a specific range (type)
    #[must_use]
    pub fn slots_with_range(&self, range: &str) -> Vec<&str> {
        self.resolved_slots
            .iter()
            .filter(|(_, slot)| slot.range.as_deref() == Some(range))
            .map(|(name, _)| name.as_str())
            .collect()
    }

    /// Check if this class has a specific slot
    #[must_use]
    pub fn has_slot(&self, slot_name: &str) -> bool {
        self.resolved_slots.contains_key(slot_name)
    }

    /// Get slot usage overrides defined on this class
    pub fn slot_usage(&self) -> impl Iterator<Item = (&String, &SlotDefinition)> {
        self.definition.slot_usage.iter()
    }

    /// Get attributes (inline slots) defined on this class
    pub fn attributes(&self) -> impl Iterator<Item = (&String, &SlotDefinition)> {
        self.definition.attributes.iter()
    }

    /// Get unique key constraints for this class
    pub fn unique_keys(&self) -> impl Iterator<Item = &String> {
        self.definition.unique_keys.keys()
    }

    /// Get rules defined on this class
    #[must_use]
    pub fn rules(&self) -> &[linkml_core::types::Rule] {
        &self.definition.rules
    }

    /// Check if this class is a tree root
    #[must_use]
    pub fn is_tree_root(&self) -> bool {
        self.definition.tree_root.unwrap_or(false)
    }

    /// Get the class URI
    #[must_use]
    pub fn class_uri(&self) -> Option<&str> {
        self.definition.class_uri.as_deref()
    }

    /// Get annotations for this class
    #[must_use]
    pub fn annotations(&self) -> Option<&linkml_core::annotations::Annotations> {
        self.definition.annotations.as_ref()
    }
}

/// Builder for creating `ClassView` instances with caching
#[derive(Debug)]
pub struct ClassViewBuilder {
    schema_view: Arc<SchemaView>,
    cache: HashMap<String, Arc<ClassView>>,
}

impl ClassViewBuilder {
    /// Create a new `ClassViewBuilder`
    #[must_use]
    pub fn new(schema_view: Arc<SchemaView>) -> Self {
        Self {
            schema_view,
            cache: HashMap::new(),
        }
    }

    /// Get or create a `ClassView` for the specified class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns schema view errors if class view creation fails
    pub fn get_or_create(&mut self, class_name: &str) -> Result<Arc<ClassView>> {
        if let Some(view) = self.cache.get(class_name) {
            return Ok(Arc::clone(view));
        }

        let view = Arc::new(ClassView::new(class_name, Arc::clone(&self.schema_view))?);
        self.cache.insert(class_name.to_string(), Arc::clone(&view));
        Ok(view)
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
