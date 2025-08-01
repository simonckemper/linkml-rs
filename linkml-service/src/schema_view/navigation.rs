//! Schema navigation utilities for traversing LinkML schemas

use linkml_core::{
    error::Result,
    types::{ClassDefinition, SlotDefinition},
};
use std::collections::HashMap;

use super::view::{SchemaView, SchemaViewError};

/// Cache for navigation results to improve performance
pub struct NavigationCache {
    /// Cached induced classes
    induced_classes: HashMap<String, ClassDefinition>,
    
    /// Cached induced slots (key: "class_name.slot_name")
    induced_slots: HashMap<String, SlotDefinition>,
    
    /// Cached inheritance chains
    _inheritance_chains: HashMap<String, Vec<String>>,
}

impl NavigationCache {
    /// Create a new navigation cache
    pub fn new() -> Self {
        Self {
            induced_classes: HashMap::new(),
            induced_slots: HashMap::new(),
            _inheritance_chains: HashMap::new(),
        }
    }
    
    /// Get a cached induced class
    pub fn get_induced_class(&self, name: &str) -> Option<ClassDefinition> {
        self.induced_classes.get(name).cloned()
    }
    
    /// Cache an induced class
    pub fn cache_induced_class(&mut self, name: String, class: ClassDefinition) {
        self.induced_classes.insert(name, class);
    }
    
    /// Get a cached induced slot for a class
    pub fn get_induced_slot(&self, class_name: &str, slot_name: &str) -> Option<SlotDefinition> {
        let key = format!("{}.{}", class_name, slot_name);
        self.induced_slots.get(&key).cloned()
    }
    
    /// Cache an induced slot for a class
    pub fn cache_induced_slot(&mut self, class_name: String, slot_name: String, slot: SlotDefinition) {
        let key = format!("{}.{}", class_name, slot_name);
        self.induced_slots.insert(key, slot);
    }
}

/// Represents an inheritance chain from a class to its root ancestor
#[derive(Debug, Clone)]
pub struct InheritanceChain {
    /// The starting class
    pub start_class: String,
    
    /// Chain of classes from start to root (excluding start)
    pub chain: Vec<String>,
    
    /// All mixins encountered in the chain
    pub mixins: Vec<String>,
}

impl InheritanceChain {
    /// Create a new inheritance chain
    pub fn new(start_class: String) -> Self {
        Self {
            start_class,
            chain: Vec::new(),
            mixins: Vec::new(),
        }
    }
    
    /// Get the direct parent of a class
    pub fn direct_parent(&self) -> Option<&String> {
        self.chain.first()
    }
    
    /// Get the root ancestor
    pub fn root_ancestor(&self) -> Option<&String> {
        self.chain.last()
    }
    
    /// Check if a class is in the inheritance chain
    pub fn contains(&self, class_name: &str) -> bool {
        self.start_class == class_name || self.chain.contains(&class_name.to_string())
    }
    
    /// Get the depth of inheritance
    pub fn depth(&self) -> usize {
        self.chain.len()
    }
}

/// Utilities for resolving slots in the context of classes
pub struct SlotResolution<'a> {
    schema_view: &'a SchemaView,
}

impl<'a> SlotResolution<'a> {
    /// Create a new slot resolution helper
    pub fn new(schema_view: &'a SchemaView) -> Self {
        Self { schema_view }
    }
    
    /// Resolve a slot in the context of a specific class
    pub fn resolve_slot(&self, slot_name: &str, class_name: &str) -> Result<SlotDefinition> {
        // Get the base slot definition
        let base_slot = self.schema_view.get_slot(slot_name)?
            .ok_or_else(|| SchemaViewError::ElementNotFound(format!("Slot '{}'", slot_name)))?;
        
        let mut resolved = base_slot.clone();
        
        // Get the class to check for slot_usage
        if let Some(class_def) = self.schema_view.get_class(class_name)? {
            // Apply slot_usage overrides
            if let Some(usage) = class_def.slot_usage.get(slot_name) {
                self.apply_slot_usage(&mut resolved, usage);
            }
            
            // Apply inherited slot_usage from ancestors
            let ancestors = self.schema_view.class_ancestors(class_name)?;
            for ancestor_name in ancestors {
                if let Some(ancestor) = self.schema_view.get_class(&ancestor_name)? {
                    if let Some(usage) = ancestor.slot_usage.get(slot_name) {
                        self.apply_slot_usage(&mut resolved, usage);
                    }
                }
            }
        }
        
        Ok(resolved)
    }
    
    /// Find all classes that use a specific slot
    pub fn find_slot_users(&self, slot_name: &str) -> Result<Vec<String>> {
        let mut users = Vec::new();
        
        for (class_name, _) in self.schema_view.all_classes()? {
            let slots = self.schema_view.class_slots(&class_name)?;
            if slots.contains(&slot_name.to_string()) {
                users.push(class_name);
            }
        }
        
        Ok(users)
    }
    
    /// Get the effective range of a slot in a class context
    pub fn get_effective_range(&self, slot_name: &str, class_name: &str) -> Result<Option<String>> {
        let slot = self.resolve_slot(slot_name, class_name)?;
        Ok(slot.range)
    }
    
    /// Check if a slot is required in a class context
    pub fn is_required(&self, slot_name: &str, class_name: &str) -> Result<bool> {
        let slot = self.resolve_slot(slot_name, class_name)?;
        Ok(slot.required.unwrap_or(false))
    }
    
    /// Check if a slot is multivalued in a class context
    pub fn is_multivalued(&self, slot_name: &str, class_name: &str) -> Result<bool> {
        let slot = self.resolve_slot(slot_name, class_name)?;
        Ok(slot.multivalued.unwrap_or(false))
    }
    
    fn apply_slot_usage(&self, slot: &mut SlotDefinition, usage: &SlotDefinition) {
        // Override properties from slot_usage
        if usage.required.is_some() {
            slot.required = usage.required;
        }
        if usage.multivalued.is_some() {
            slot.multivalued = usage.multivalued;
        }
        if usage.range.is_some() {
            slot.range = usage.range.clone();
        }
        if usage.pattern.is_some() {
            slot.pattern = usage.pattern.clone();
        }
        if usage.minimum_value.is_some() {
            slot.minimum_value = usage.minimum_value.clone();
        }
        if usage.maximum_value.is_some() {
            slot.maximum_value = usage.maximum_value.clone();
        }
        if usage.description.is_some() {
            slot.description = usage.description.clone();
        }
        // Add more overrides as needed
    }
}

/// Navigate and analyze class hierarchies
pub struct ClassNavigator<'a> {
    schema_view: &'a SchemaView,
}

impl<'a> ClassNavigator<'a> {
    /// Create a new class navigator
    pub fn new(schema_view: &'a SchemaView) -> Self {
        Self { schema_view }
    }
    
    /// Get the full inheritance chain for a class
    pub fn get_inheritance_chain(&self, class_name: &str) -> Result<InheritanceChain> {
        let mut chain = InheritanceChain::new(class_name.to_string());
        
        // Get ancestors
        chain.chain = self.schema_view.class_ancestors(class_name)?;
        
        // Collect mixins
        if let Some(class_def) = self.schema_view.get_class(class_name)? {
            chain.mixins = class_def.mixins.clone();
            
            // Also collect mixins from ancestors
            for ancestor_name in &chain.chain {
                if let Some(ancestor) = self.schema_view.get_class(ancestor_name)? {
                    chain.mixins.extend(ancestor.mixins.clone());
                }
            }
        }
        
        Ok(chain)
    }
    
    /// Find common ancestors of two classes
    pub fn find_common_ancestors(&self, class1: &str, class2: &str) -> Result<Vec<String>> {
        let ancestors1 = self.schema_view.class_ancestors(class1)?;
        let ancestors2 = self.schema_view.class_ancestors(class2)?;
        
        let mut common = Vec::new();
        for ancestor in &ancestors1 {
            if ancestors2.contains(ancestor) {
                common.push(ancestor.clone());
            }
        }
        
        Ok(common)
    }
    
    /// Check if one class is an ancestor of another
    pub fn is_ancestor(&self, potential_ancestor: &str, class: &str) -> Result<bool> {
        let ancestors = self.schema_view.class_ancestors(class)?;
        Ok(ancestors.contains(&potential_ancestor.to_string()))
    }
    
    /// Get all leaf classes (classes with no subclasses)
    pub fn get_leaf_classes(&self) -> Result<Vec<String>> {
        let all_classes = self.schema_view.all_classes()?;
        let mut leaf_classes = Vec::new();
        
        for class_name in all_classes.keys() {
            let descendants = self.schema_view.class_descendants(class_name)?;
            if descendants.is_empty() {
                leaf_classes.push(class_name.clone());
            }
        }
        
        Ok(leaf_classes)
    }
    
    /// Get all root classes (classes with no superclass)
    pub fn get_root_classes(&self) -> Result<Vec<String>> {
        let all_classes = self.schema_view.all_classes()?;
        let mut root_classes = Vec::new();
        
        for (class_name, class_def) in all_classes {
            if class_def.is_a.is_none() {
                root_classes.push(class_name);
            }
        }
        
        Ok(root_classes)
    }
}