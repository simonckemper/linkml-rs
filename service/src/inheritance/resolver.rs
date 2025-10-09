//! Complete inheritance resolution for `LinkML` schemas
//!
//! This module handles full multiple inheritance including mixins,
//! slot overrides, and diamond inheritance patterns.

use linkml_core::annotations::Annotations;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};

/// Inheritance resolver for `LinkML` schemas
pub struct InheritanceResolver<'a> {
    schema: &'a SchemaDefinition,
    /// Cache of resolved classes
    resolved_cache: HashMap<String, ClassDefinition>,
    /// Track visited classes to detect cycles
    visited: HashSet<String>,
}

impl<'a> InheritanceResolver<'a> {
    /// Create a new inheritance resolver
    #[must_use]
    pub fn new(schema: &'a SchemaDefinition) -> Self {
        Self {
            schema,
            resolved_cache: HashMap::new(),
            visited: HashSet::new(),
        }
    }

    /// Resolve all inheritance for a class (`is_a` + mixins)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn resolve_class(&mut self, class_name: &str) -> Result<ClassDefinition> {
        // Check cache
        if let Some(resolved) = self.resolved_cache.get(class_name) {
            return Ok(resolved.clone());
        }

        // Check for cycles
        if self.visited.contains(class_name) {
            return Err(LinkMLError::service(format!(
                "Circular inheritance detected for class '{class_name}'"
            )));
        }

        // Get base class
        let base_class = self
            .schema
            .classes
            .get(class_name)
            .ok_or_else(|| LinkMLError::service(format!("Class '{class_name}' not found")))?
            .clone();

        self.visited.insert(class_name.to_string());

        // Start with the base class
        let mut resolved = base_class.clone();

        // Collect all ancestors (is_a chain + mixins)
        let ancestors = self.get_all_ancestors(&base_class)?;

        // Apply inheritance in correct order (C3 linearization)
        let mro = self.c3_linearization(class_name, &ancestors)?;

        // Merge each ancestor into the resolved class
        for ancestor_name in mro.iter().rev() {
            if ancestor_name != class_name
                && let Some(ancestor) = self.schema.classes.get(ancestor_name)
            {
                Self::merge_class(&mut resolved, ancestor);
            }
        }

        // Apply own attributes last (they override inherited)
        self.apply_own_attributes(&mut resolved, &base_class);

        self.visited.remove(class_name);
        self.resolved_cache
            .insert(class_name.to_string(), resolved.clone());

        Ok(resolved)
    }

    /// Get all ancestors of a class (`is_a` + mixins, recursively)
    fn get_all_ancestors(&self, class: &ClassDefinition) -> Result<Vec<Vec<String>>> {
        let mut ancestors = Vec::new();

        // Add is_a parent
        if let Some(parent) = &class.is_a {
            let parent_ancestors = if let Some(parent_class) = self.schema.classes.get(parent) {
                let mut chain = self.get_all_ancestors(parent_class)?;
                chain.insert(0, vec![parent.clone()]);
                chain
            } else {
                vec![vec![parent.clone()]]
            };
            ancestors.push(parent_ancestors.into_iter().flatten().collect());
        }

        // Add mixins
        for mixin in &class.mixins {
            if let Some(mixin_class) = self.schema.classes.get(mixin) {
                let mixin_ancestors = self.get_all_ancestors(mixin_class)?;
                let mut chain = vec![mixin.clone()];
                chain.extend(mixin_ancestors.into_iter().flatten());
                ancestors.push(chain);
            } else {
                ancestors.push(vec![mixin.clone()]);
            }
        }

        Ok(ancestors)
    }

    /// C3 linearization for method resolution order
    fn c3_linearization(&self, class_name: &str, ancestors: &[Vec<String>]) -> Result<Vec<String>> {
        // Start with the class itself
        let mut result = vec![class_name.to_string()];

        // Create working lists
        let mut lists: Vec<VecDeque<String>> = ancestors
            .iter()
            .map(|a| a.iter().cloned().collect())
            .collect();

        // Add the list of direct parents
        let mut direct_parents = VecDeque::new();
        if let Some(class) = self.schema.classes.get(class_name) {
            if let Some(parent) = &class.is_a {
                direct_parents.push_back(parent.clone());
            }
            for mixin in &class.mixins {
                direct_parents.push_back(mixin.clone());
            }
        }
        if !direct_parents.is_empty() {
            lists.push(direct_parents);
        }

        // C3 merge
        while lists.iter().any(|l| !l.is_empty()) {
            // Find a class that appears at the head of a list
            // and doesn't appear in the tail of any other list
            let mut candidate = None;

            for list in &lists {
                if let Some(head) = list.front() {
                    // Check if this head appears in any tail
                    let mut in_tail = false;
                    for other_list in &lists {
                        let tail: Vec<_> = other_list.iter().skip(1).cloned().collect();
                        if tail.contains(head) {
                            in_tail = true;
                            break;
                        }
                    }

                    if !in_tail {
                        candidate = Some(head.clone());
                        break;
                    }
                }
            }

            if let Some(selected) = candidate {
                // Add to result if not already there
                if !result.contains(&selected) {
                    result.push(selected.clone());
                }

                // Remove from all lists
                for list in &mut lists {
                    list.retain(|x| x != &selected);
                }
            } else {
                // No valid candidate found - inconsistent hierarchy
                return Err(LinkMLError::service(
                    "Inconsistent class hierarchy detected (C3 linearization failed)",
                ));
            }
        }

        Ok(result)
    }

    /// Merge an ancestor class into the resolved class
    fn merge_class(target: &mut ClassDefinition, source: &ClassDefinition) {
        // Merge slots (don't duplicate)
        for slot in &source.slots {
            if !target.slots.contains(slot) {
                target.slots.push(slot.clone());
            }
        }

        // Merge slot_usage (ancestor definitions are defaults)
        for (slot_name, slot_def) in &source.slot_usage {
            target
                .slot_usage
                .entry(slot_name.clone())
                .or_insert_with(|| slot_def.clone());
        }

        // Merge attributes
        for (attr_name, attr_def) in &source.attributes {
            target
                .attributes
                .entry(attr_name.clone())
                .or_insert_with(|| attr_def.clone());
        }

        // Merge rules
        for rule in &source.rules {
            if !target.rules.contains(rule) {
                target.rules.push(rule.clone());
            }
        }

        // Merge unique keys
        for (key_name, key_def) in &source.unique_keys {
            target
                .unique_keys
                .entry(key_name.clone())
                .or_insert_with(|| key_def.clone());
        }

        // Don't inherit certain properties
        // - name, description, abstract, mixin stay as defined in target
        // - is_a and mixins are not inherited
    }

    /// Apply the class's own attributes (override inherited)
    fn apply_own_attributes(&self, target: &mut ClassDefinition, source: &ClassDefinition) {
        // Own slot_usage overrides inherited, with slot inheritance resolution
        for (slot_name, slot_def) in &source.slot_usage {
            let mut resolved_slot = slot_def.clone();

            // If this slot_usage has is_a, resolve it
            if let Some(parent_slot_name) = &slot_def.is_a
                && let Some(parent_slot) = self.schema.slots.get(parent_slot_name)
            {
                // Start with parent slot's annotations
                if resolved_slot.annotations.is_none() {
                    resolved_slot
                        .annotations
                        .clone_from(&parent_slot.annotations);
                } else if let Some(parent_annotations) = &parent_slot.annotations {
                    // Merge parent annotations (as defaults)
                    if let Some(ref mut slot_annotations) = resolved_slot.annotations {
                        for (key, value) in parent_annotations {
                            slot_annotations.entry(key.clone()).or_insert(value.clone());
                        }
                    }
                }

                // Apply other parent properties as defaults
                if resolved_slot.description.is_none() {
                    resolved_slot
                        .description
                        .clone_from(&parent_slot.description);
                }
                if resolved_slot.range.is_none() {
                    resolved_slot.range.clone_from(&parent_slot.range);
                }
                if resolved_slot.required.is_none() {
                    resolved_slot.required = parent_slot.required;
                }
                if resolved_slot.multivalued.is_none() {
                    resolved_slot.multivalued = parent_slot.multivalued;
                }
                if resolved_slot.pattern.is_none() {
                    resolved_slot.pattern.clone_from(&parent_slot.pattern);
                }
            }

            target.slot_usage.insert(slot_name.clone(), resolved_slot);
        }

        // Own attributes override inherited
        for (attr_name, attr_def) in &source.attributes {
            target
                .attributes
                .insert(attr_name.clone(), attr_def.clone());
        }
    }

    /// Resolve all slots for a class (including inherited and overridden)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn resolve_class_slots(
        &mut self,
        class_name: &str,
    ) -> Result<HashMap<String, SlotDefinition>> {
        let resolved_class = self.resolve_class(class_name)?;
        let mut resolved_slots = HashMap::new();

        // Process each slot
        for slot_name in &resolved_class.slots {
            // Start with the base slot definition
            let mut slot = self
                .schema
                .slots
                .get(slot_name)
                .ok_or_else(|| LinkMLError::service(format!("Slot '{slot_name}' not found")))?
                .clone();

            // Apply slot_usage overrides
            if let Some(override_def) = resolved_class.slot_usage.get(slot_name) {
                self.apply_slot_override(&mut slot, override_def);
            }

            // Check attributes for inline definitions
            if let Some(attr_def) = resolved_class.attributes.get(slot_name) {
                self.apply_slot_override(&mut slot, attr_def);
            }

            resolved_slots.insert(slot_name.clone(), slot);
        }

        Ok(resolved_slots)
    }

    /// Apply slot override/usage to a slot definition
    fn apply_slot_override(&self, target: &mut SlotDefinition, override_def: &SlotDefinition) {
        // Handle slot inheritance through is_a
        if let Some(parent_slot_name) = &override_def.is_a
            && let Some(parent_slot) = self.schema.slots.get(parent_slot_name)
        {
            // First apply parent slot properties as defaults
            self.apply_slot_override(target, parent_slot);
        }

        // Merge annotations (parent annotations as defaults, override annotations win)
        if let Some(override_annotations) = &override_def.annotations {
            if target.annotations.is_none() {
                target.annotations = Some(Annotations::new());
            }
            if let Some(target_annotations) = &mut target.annotations {
                // Add override annotations (they win over inherited)
                for (key, value) in override_annotations {
                    target_annotations.insert(key.clone(), value.clone());
                }
            }
        }

        // Override non-None values
        if override_def.description.is_some() {
            target.description.clone_from(&override_def.description);
        }
        if override_def.required.is_some() {
            target.required = override_def.required;
        }
        if override_def.multivalued.is_some() {
            target.multivalued = override_def.multivalued;
        }
        if override_def.range.is_some() {
            target.range.clone_from(&override_def.range);
        }
        if override_def.pattern.is_some() {
            target.pattern.clone_from(&override_def.pattern);
        }
        if override_def.minimum_value.is_some() {
            target.minimum_value.clone_from(&override_def.minimum_value);
        }
        if override_def.maximum_value.is_some() {
            target.maximum_value.clone_from(&override_def.maximum_value);
        }
        if override_def.ifabsent.is_some() {
            target.ifabsent.clone_from(&override_def.ifabsent);
        }
    }
}

/// Get the complete inheritance chain for a class
/// Returns an error if the operation fails
///
/// # Errors
///
pub fn get_inheritance_chain(class_name: &str, schema: &SchemaDefinition) -> Result<Vec<String>> {
    let mut resolver = InheritanceResolver::new(schema);
    let _resolved = resolver.resolve_class(class_name)?;

    // The MRO gives us the complete inheritance chain
    let class = schema
        .classes
        .get(class_name)
        .ok_or_else(|| LinkMLError::service(format!("Class '{class_name}' not found")))?;

    let ancestors = resolver.get_all_ancestors(class)?;
    resolver.c3_linearization(class_name, &ancestors)
}

/// Check if a class is a subclass of another (considering mixins)
/// Returns an error if the operation fails
///
/// # Errors
///
pub fn is_subclass_of(child: &str, parent: &str, schema: &SchemaDefinition) -> Result<bool> {
    if child == parent {
        return Ok(true);
    }

    let chain = get_inheritance_chain(child, schema)?;
    Ok(chain.contains(&parent.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[test]
    fn test_simple_inheritance() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();

        // Create parent class
        let mut animal = ClassDefinition::default();
        animal.name = "Animal".to_string();
        animal.slots = vec!["name".to_string()];
        schema.classes.insert("Animal".to_string(), animal);

        // Create child class
        let mut dog = ClassDefinition::default();
        dog.name = "Dog".to_string();
        dog.is_a = Some("Animal".to_string());
        dog.slots = vec!["breed".to_string()];
        schema.classes.insert("Dog".to_string(), dog);

        // Resolve inheritance
        let mut resolver = InheritanceResolver::new(&schema);
        let result = resolver.resolve_class("Dog")?;

        // Check that slots are inherited
        assert!(result.slots.contains(&"name".to_string()));
        assert!(result.slots.contains(&"breed".to_string()));
        Ok(())
    }

    #[test]
    fn test_multiple_inheritance_with_mixins() -> std::result::Result<(), Box<dyn std::error::Error>>
    {
        let mut schema = SchemaDefinition::default();

        // Create base classes
        let mut named = ClassDefinition::default();
        named.name = "Named".to_string();
        named.slots = vec!["name".to_string()];
        named.mixin = Some(true);
        schema.classes.insert("Named".to_string(), named);

        let mut aged = ClassDefinition::default();
        aged.name = "Aged".to_string();
        aged.slots = vec!["age".to_string()];
        aged.mixin = Some(true);
        schema.classes.insert("Aged".to_string(), aged);

        let mut entity = ClassDefinition::default();
        entity.name = "Entity".to_string();
        entity.slots = vec!["id".to_string()];
        schema.classes.insert("Entity".to_string(), entity);

        // Create class with multiple inheritance
        let mut person = ClassDefinition::default();
        person.name = "Person".to_string();
        person.is_a = Some("Entity".to_string());
        person.mixins = vec!["Named".to_string(), "Aged".to_string()];
        person.slots = vec!["email".to_string()];
        schema.classes.insert("Person".to_string(), person);

        // Resolve inheritance
        let mut resolver = InheritanceResolver::new(&schema);
        let resolved_class = resolver.resolve_class("Person")?;

        // Check that all slots are inherited
        assert!(resolved_class.slots.contains(&"id".to_string())); // From Entity
        assert!(resolved_class.slots.contains(&"name".to_string())); // From Named mixin
        assert!(resolved_class.slots.contains(&"age".to_string())); // From Aged mixin
        assert!(resolved_class.slots.contains(&"email".to_string())); // Own slot
        Ok(())
    }

    #[test]
    fn test_diamond_inheritance() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();

        // Create diamond hierarchy
        //     A
        //    / \
        //   B   C
        //    \ /
        //     D

        let mut a = ClassDefinition::default();
        a.name = "A".to_string();
        a.slots = vec!["a_slot".to_string()];
        schema.classes.insert("A".to_string(), a);

        let mut b = ClassDefinition::default();
        b.name = "B".to_string();
        b.is_a = Some("A".to_string());
        b.slots = vec!["b_slot".to_string()];
        schema.classes.insert("B".to_string(), b);

        let mut c = ClassDefinition::default();
        c.name = "C".to_string();
        c.is_a = Some("A".to_string());
        c.slots = vec!["c_slot".to_string()];
        schema.classes.insert("C".to_string(), c);

        let mut d = ClassDefinition::default();
        d.name = "D".to_string();
        d.is_a = Some("B".to_string());
        d.mixins = vec!["C".to_string()];
        d.slots = vec!["d_slot".to_string()];
        schema.classes.insert("D".to_string(), d);

        // Resolve inheritance
        let mut resolver = InheritanceResolver::new(&schema);
        let resolved_class = resolver.resolve_class("D")?;

        // Check that all slots are inherited (A's slot should appear only once)
        assert!(resolved_class.slots.contains(&"a_slot".to_string()));
        assert!(resolved_class.slots.contains(&"b_slot".to_string()));
        assert!(resolved_class.slots.contains(&"c_slot".to_string()));
        assert!(resolved_class.slots.contains(&"d_slot".to_string()));

        // Check that A's slot appears only once
        let a_count = resolved_class
            .slots
            .iter()
            .filter(|s| *s == "a_slot")
            .count();
        assert_eq!(a_count, 1, "Diamond inheritance should not duplicate slots");
        Ok(())
    }
}
