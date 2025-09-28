//! Schema composition and inheritance resolution
//!
//! Handles class inheritance, mixins, and slot usage overrides

use indexmap::IndexMap;
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_core::{LinkMLError, Result};
use std::collections::{HashMap, HashSet};

/// Resolves schema composition including inheritance and mixins
pub struct SchemaComposer {
    /// The `LinkML` schema definition containing classes, slots, and inheritance relationships.
    schema: SchemaDefinition,
    /// Cache of resolved classes
    resolved_cache: HashMap<String, ResolvedClass>,
}

/// A fully resolved class with all inherited properties
#[derive(Debug, Clone)]
pub struct ResolvedClass {
    /// The base class definition with all inheritance resolved
    pub base: ClassDefinition,
    /// All effective slots (including inherited)
    pub effective_slots: IndexMap<String, SlotDefinition>,
    /// All ancestor classes in order
    pub ancestors: Vec<String>,
    /// All mixin classes applied
    pub mixins: Vec<String>,
    /// Whether this class is abstract
    pub is_abstract: bool,
    /// Whether this class is a tree root
    pub is_tree_root: bool,
}

impl SchemaComposer {
    /// Create a new schema composer
    #[must_use]
    pub fn new(schema: SchemaDefinition) -> Self {
        Self {
            schema,
            resolved_cache: HashMap::new(),
        }
    }

    /// Resolve a class with all its inherited properties
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn resolve_class(&mut self, class_name: &str) -> Result<ResolvedClass> {
        // Check cache first
        if let Some(resolved) = self.resolved_cache.get(class_name) {
            return Ok(resolved.clone());
        }

        // Get the class definition
        let class_def = self
            .schema
            .classes
            .get(class_name)
            .ok_or_else(|| {
                LinkMLError::schema_validation(format!("Class '{class_name}' not found in schema"))
            })?
            .clone();

        // Track visited classes to detect cycles
        let mut visited = HashSet::new();
        visited.insert(class_name.to_string());

        // Resolve the class
        let resolved = self.resolve_class_internal(&class_def, class_name, &mut visited)?;

        // Cache the result
        self.resolved_cache
            .insert(class_name.to_string(), resolved.clone());
        Ok(resolved)
    }

    /// Internal recursive resolution
    fn resolve_class_internal(
        &self,
        class_def: &ClassDefinition,
        class_name: &str,
        visited: &mut HashSet<String>,
    ) -> Result<ResolvedClass> {
        let mut effective_slots = IndexMap::new();
        let mut ancestors = Vec::new();
        let mut all_mixins = Vec::new();

        // First, process parent class if any
        if let Some(parent_name) = &class_def.is_a {
            if visited.contains(parent_name) {
                return Err(LinkMLError::schema_validation(format!(
                    "Circular inheritance detected: {class_name} inherits from {parent_name}"
                )));
            }

            visited.insert(parent_name.clone());

            let parent_class = self.schema.classes.get(parent_name).ok_or_else(|| {
                LinkMLError::schema_validation(format!("Parent class '{parent_name}' not found"))
            })?;

            // Recursively resolve parent
            let parent_resolved =
                self.resolve_class_internal(parent_class, parent_name, visited)?;

            // Inherit from parent
            effective_slots.extend(parent_resolved.effective_slots);
            ancestors.extend(parent_resolved.ancestors);
            ancestors.push(parent_name.clone());
            all_mixins.extend(parent_resolved.mixins);

            visited.remove(parent_name);
        }

        // Process mixins
        for mixin_name in &class_def.mixins {
            if visited.contains(mixin_name) {
                return Err(LinkMLError::schema_validation(format!(
                    "Circular mixin detected: {class_name} includes {mixin_name}"
                )));
            }

            visited.insert(mixin_name.clone());

            let mixin_class = self.schema.classes.get(mixin_name).ok_or_else(|| {
                LinkMLError::schema_validation(format!("Mixin class '{mixin_name}' not found"))
            })?;

            // Recursively resolve mixin
            let mixin_resolved = self.resolve_class_internal(mixin_class, mixin_name, visited)?;

            // Apply mixin slots (mixins override earlier definitions)
            effective_slots.extend(mixin_resolved.effective_slots);
            all_mixins.push(mixin_name.clone());
            all_mixins.extend(mixin_resolved.mixins);

            visited.remove(mixin_name);
        }

        // Add this class's direct slots
        for slot_name in &class_def.slots {
            if let Some(slot_def) = self.schema.slots.get(slot_name) {
                effective_slots.insert(slot_name.clone(), slot_def.clone());
            }
        }

        // Apply slot usage overrides
        for (slot_name, usage) in &class_def.slot_usage {
            if let Some(base_slot) = effective_slots.get_mut(slot_name) {
                // Merge usage with base slot
                Self::apply_slot_usage(base_slot, usage);
            } else {
                // Slot usage for a slot not in the class
                return Err(LinkMLError::schema_validation(format!(
                    "Slot usage for '{slot_name}' but slot not found in class '{class_name}'"
                )));
            }
        }

        // Add inline attributes
        for (attr_name, attr_def) in &class_def.attributes {
            effective_slots.insert(attr_name.clone(), attr_def.clone());
        }

        Ok(ResolvedClass {
            base: class_def.clone(),
            effective_slots,
            ancestors,
            mixins: all_mixins,
            is_abstract: class_def.abstract_.unwrap_or(false),
            is_tree_root: class_def.tree_root.unwrap_or(false),
        })
    }

    /// Apply slot usage overrides to a base slot
    fn apply_slot_usage(base_slot: &mut SlotDefinition, usage: &SlotDefinition) {
        // Override properties that are explicitly set in usage
        if usage.required.is_some() {
            base_slot.required = usage.required;
        }
        if usage.multivalued.is_some() {
            base_slot.multivalued = usage.multivalued;
        }
        if usage.range.is_some() {
            base_slot.range.clone_from(&usage.range);
        }
        if usage.pattern.is_some() {
            base_slot.pattern.clone_from(&usage.pattern);
        }
        if usage.minimum_value.is_some() {
            base_slot.minimum_value.clone_from(&usage.minimum_value);
        }
        if usage.maximum_value.is_some() {
            base_slot.maximum_value.clone_from(&usage.maximum_value);
        }
        if usage.description.is_some() {
            base_slot.description.clone_from(&usage.description);
        }
        // Add more overrides as needed
    }

    /// Get all concrete (non-abstract) classes that can be instantiated
    ///
    /// # Errors
    ///
    /// Returns an error if class resolution fails.
    pub fn get_concrete_classes(&mut self) -> Result<Vec<String>> {
        let mut concrete_classes = Vec::new();
        let class_names: Vec<String> = self.schema.classes.keys().cloned().collect();

        for class_name in class_names {
            let resolved = self.resolve_class(&class_name)?;
            if !resolved.is_abstract {
                concrete_classes.push(class_name);
            }
        }

        Ok(concrete_classes)
    }

    /// Get all tree root classes
    ///
    /// # Errors
    ///
    /// Returns an error if class resolution fails.
    pub fn get_tree_roots(&mut self) -> Result<Vec<String>> {
        let mut tree_roots = Vec::new();
        let class_names: Vec<String> = self.schema.classes.keys().cloned().collect();

        for class_name in class_names {
            let resolved = self.resolve_class(&class_name)?;
            if resolved.is_tree_root {
                tree_roots.push(class_name);
            }
        }

        Ok(tree_roots)
    }

    /// Check if one class is a subclass of another
    ///
    /// # Errors
    ///
    /// Returns an error if class resolution fails.
    pub fn is_subclass_of(&mut self, child: &str, parent: &str) -> Result<bool> {
        if child == parent {
            return Ok(true);
        }

        let resolved = self.resolve_class(child)?;
        Ok(resolved.ancestors.contains(&parent.to_string()))
    }

    /// Get all subclasses of a given class
    ///
    /// # Errors
    ///
    /// Returns an error if class resolution fails.
    pub fn get_subclasses(&mut self, parent: &str) -> Result<Vec<String>> {
        let mut subclasses = Vec::new();
        let class_names: Vec<String> = self.schema.classes.keys().cloned().collect();

        for class_name in class_names {
            if self.is_subclass_of(&class_name, parent)? && class_name != parent {
                subclasses.push(class_name);
            }
        }

        Ok(subclasses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        };

        // Base class
        let named_thing = ClassDefinition {
            name: "NamedThing".to_string(),
            abstract_: Some(true),
            slots: vec!["id".to_string(), "name".to_string()],
            ..Default::default()
        };

        // Mixin
        let timestamped = ClassDefinition {
            name: "Timestamped".to_string(),
            mixin: Some(true),
            slots: vec!["created_at".to_string(), "updated_at".to_string()],
            ..Default::default()
        };

        // Concrete class
        let person = ClassDefinition {
            name: "Person".to_string(),
            is_a: Some("NamedThing".to_string()),
            mixins: vec!["Timestamped".to_string()],
            slots: vec!["age".to_string()],
            tree_root: Some(true),
            ..Default::default()
        };

        schema.classes.insert("NamedThing".to_string(), named_thing);
        schema
            .classes
            .insert("Timestamped".to_string(), timestamped);
        schema.classes.insert("Person".to_string(), person);

        // Add slot definitions
        let slots = vec![
            ("id", true),
            ("name", true),
            ("age", false),
            ("created_at", false),
            ("updated_at", false),
        ];

        for (slot_name, required) in slots {
            let slot = SlotDefinition {
                name: slot_name.to_string(),
                required: Some(required),
                ..Default::default()
            };
            schema.slots.insert(slot_name.to_string(), slot);
        }

        schema
    }

    #[test]
    fn test_class_resolution() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let mut composer = SchemaComposer::new(schema);

        let person = composer
            .resolve_class("Person")
            .expect("should resolve Person class: {}");

        // Check inherited slots
        assert_eq!(person.effective_slots.len(), 5);
        assert!(person.effective_slots.contains_key("id"));
        assert!(person.effective_slots.contains_key("name"));
        assert!(person.effective_slots.contains_key("age"));
        assert!(person.effective_slots.contains_key("created_at"));
        assert!(person.effective_slots.contains_key("updated_at"));

        // Check ancestors
        assert_eq!(person.ancestors, vec!["NamedThing"]);

        // Check mixins
        assert_eq!(person.mixins, vec!["Timestamped"]);

        // Check properties
        assert!(!person.is_abstract);
        assert!(person.is_tree_root);
        Ok(())
    }

    #[test]
    fn test_circular_inheritance_detection() {
        let mut schema = SchemaDefinition::default();

        // Create circular inheritance
        let class_a = ClassDefinition {
            name: "A".to_string(),
            is_a: Some("B".to_string()),
            ..Default::default()
        };

        let class_b = ClassDefinition {
            name: "B".to_string(),
            is_a: Some("A".to_string()),
            ..Default::default()
        };

        schema.classes.insert("A".to_string(), class_a);
        schema.classes.insert("B".to_string(), class_b);

        let mut composer = SchemaComposer::new(schema);
        let result = composer.resolve_class("A");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Circular inheritance")
        );
    }
}
