//! Inheritance resolution for `LinkML` schemas
//!
//! This module provides functionality to resolve inheritance relationships
//! in `LinkML` schemas, including mixin composition and attribute inheritance.

use indexmap::IndexMap;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use thiserror::Error;

/// Error type for inheritance resolution
#[derive(Debug, Error)]
pub enum InheritanceError {
    /// Circular inheritance detected
    #[error("Circular inheritance detected: {0}")]
    CircularInheritance(String),

    /// Parent not found
    #[error("Parent not found: {0}")]
    ParentNotFound(String),

    /// Mixin not found
    #[error("Mixin not found: {0}")]
    MixinNotFound(String),

    /// Invalid path
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Invalid inheritance
    #[error("Invalid inheritance: {0}")]
    InvalidInheritance(String),

    /// Conflict in inherited attributes
    #[error("Attribute conflict in inheritance: {attribute} from {sources:?}")]
    AttributeConflict {
        /// Name of the conflicting attribute
        attribute: String,
        /// Sources where the attribute is defined
        sources: Vec<String>,
    },
}

/// Result type for inheritance operations
pub type InheritanceResult<T> = std::result::Result<T, InheritanceError>;

/// Inheritance resolver for `LinkML` schemas
pub struct InheritanceResolver {
    /// Cache of resolved classes
    resolved_cache: HashMap<String, Arc<ClassDefinition>>,

    /// Inheritance depth limit
    max_depth: usize,
}

impl InheritanceResolver {
    /// Create a new inheritance resolver
    #[must_use]
    pub fn new() -> Self {
        Self {
            resolved_cache: HashMap::new(),
            max_depth: 100,
        }
    }

    /// Create with custom depth limit
    #[must_use]
    pub fn with_max_depth(max_depth: usize) -> Self {
        Self {
            resolved_cache: HashMap::new(),
            max_depth,
        }
    }

    /// Resolve all inheritance in a schema
    ///
    /// # Errors
    /// Returns `InheritanceError::CircularInheritance` if circular inheritance is detected, or other inheritance resolution errors
    pub fn resolve_schema(&mut self, schema: &mut SchemaDefinition) -> InheritanceResult<()> {
        // Clear cache for new schema
        self.resolved_cache.clear();

        // First, detect any circular inheritance
        self.detect_circular_inheritance(schema)?;

        // Resolve classes in topological order
        let order = self.topological_sort_classes(schema)?;

        // Process classes in order
        for class_name in order {
            if let Some(class_def) = schema.classes.get(&class_name).cloned() {
                let resolved = self.resolve_class(&class_def, schema)?;
                schema.classes.insert(class_name.clone(), resolved);
            }
        }

        // Resolve slots
        self.resolve_slots(schema)?;

        // Resolve enums
        Self::resolve_enums(schema);

        Ok(())
    }

    /// Resolve a single class
    ///
    /// # Errors
    ///
    /// Returns `InheritanceError::CircularInheritance` if circular inheritance is detected.
    /// Returns `InheritanceError::ParentNotFound` if a parent class is not found in the schema.
    /// Returns `InheritanceError::ConflictingSlots` if slot conflicts cannot be resolved.
    pub fn resolve_class(
        &mut self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> InheritanceResult<ClassDefinition> {
        // Check cache first
        if let Some(cached) = self.resolved_cache.get(&class.name) {
            return Ok((**cached).clone());
        }

        let mut resolved = class.clone();
        let mut visited = HashSet::new();

        // Resolve parent inheritance
        if let Some(parent_name) = &class.is_a {
            self.inherit_from_parent(&mut resolved, parent_name, schema, &mut visited)?;
        }

        // Resolve mixins
        for mixin_name in &class.mixins {
            self.apply_mixin(&mut resolved, mixin_name, schema, &mut visited)?;
        }

        // Cache the result
        self.resolved_cache
            .insert(class.name.clone(), Arc::new(resolved.clone()));

        Ok(resolved)
    }

    /// Inherit attributes from parent
    fn inherit_from_parent(
        &mut self,
        class: &mut ClassDefinition,
        parent_name: &str,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
    ) -> InheritanceResult<()> {
        if !visited.insert(parent_name.to_string()) {
            return Err(InheritanceError::CircularInheritance(format!(
                "Circular inheritance involving {parent_name}"
            )));
        }

        // Check depth limit
        if visited.len() > self.max_depth {
            return Err(InheritanceError::InvalidInheritance(format!(
                "Maximum inheritance depth {} exceeded",
                self.max_depth
            )));
        }

        let parent = schema
            .classes
            .get(parent_name)
            .ok_or_else(|| InheritanceError::ParentNotFound(parent_name.to_string()))?;

        // Recursively resolve parent first
        let resolved_parent = self.resolve_class(parent, schema)?;

        // Merge attributes
        self.merge_class_attributes(class, &resolved_parent)?;

        Ok(())
    }

    /// Apply mixin to class
    fn apply_mixin(
        &mut self,
        class: &mut ClassDefinition,
        mixin_name: &str,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
    ) -> InheritanceResult<()> {
        if !visited.insert(format!("mixin:{mixin_name}")) {
            return Err(InheritanceError::CircularInheritance(format!(
                "Circular mixin reference involving {mixin_name}"
            )));
        }

        // Check depth limit for mixins
        if visited.len() > self.max_depth {
            return Err(InheritanceError::InvalidInheritance(format!(
                "Maximum mixin depth {} exceeded",
                self.max_depth
            )));
        }

        let mixin = schema
            .classes
            .get(mixin_name)
            .ok_or_else(|| InheritanceError::MixinNotFound(mixin_name.to_string()))?;

        if !mixin.mixin.unwrap_or(false) {
            return Err(InheritanceError::InvalidInheritance(format!(
                "{mixin_name} is not marked as a mixin"
            )));
        }

        // Recursively resolve mixin first
        let resolved_mixin = self.resolve_class(mixin, schema)?;

        // Merge attributes
        self.merge_class_attributes(class, &resolved_mixin)?;

        Ok(())
    }

    /// Merge class attributes
    fn merge_class_attributes(
        &self,
        target: &mut ClassDefinition,
        source: &ClassDefinition,
    ) -> InheritanceResult<()> {
        // Merge slots
        for (slot_name, slot_usage) in &source.slot_usage {
            if !target.slot_usage.contains_key(slot_name) {
                target
                    .slot_usage
                    .insert(slot_name.clone(), slot_usage.clone());
            }
        }

        // Merge attributes
        for (attr_name, attr_def) in &source.attributes {
            if !target.attributes.contains_key(attr_name) {
                target
                    .attributes
                    .insert(attr_name.clone(), attr_def.clone());
            }
        }

        // Merge description if not set
        if target.description.is_none() && source.description.is_some() {
            target.description.clone_from(&source.description);
        }

        // Merge annotations
        self.merge_annotations(&mut target.annotations, source.annotations.as_ref());

        // Merge other properties
        if target.abstract_.is_none() {
            target.abstract_ = source.abstract_;
        }

        if target.tree_root.is_none() {
            target.tree_root = source.tree_root;
        }

        Ok(())
    }

    /// Merge annotations
    fn merge_annotations(
        &self,
        target: &mut Option<linkml_core::annotations::Annotations>,
        source: Option<&linkml_core::annotations::Annotations>,
    ) {
        if let Some(source_annotations) = source {
            if target.is_none() {
                *target = Some(IndexMap::new());
            }
            if let Some(target_annotations) = target {
                for (key, value) in source_annotations {
                    target_annotations
                        .entry(key.clone())
                        .or_insert_with(|| value.clone());
                }
            }
        }
    }

    /// Detect circular inheritance
    fn detect_circular_inheritance(&self, schema: &SchemaDefinition) -> InheritanceResult<()> {
        for (class_name, class_def) in &schema.classes {
            let mut visited = HashSet::new();
            let mut path = Vec::new();

            self.check_circular_class(class_name, class_def, schema, &mut visited, &mut path)?;
        }

        Ok(())
    }

    /// Check for circular inheritance in a class
    fn check_circular_class(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> InheritanceResult<()> {
        if path.contains(&class_name.to_string()) {
            let pos = path.iter().position(|x| x == class_name).ok_or_else(|| {
                InheritanceError::InvalidPath("Class not found in path".to_string())
            })?;
            let cycle = path[pos..].to_vec().join(" -> ");
            return Err(InheritanceError::CircularInheritance(format!(
                "{cycle} -> {class_name}"
            )));
        }

        // Check depth limit
        if path.len() >= self.max_depth {
            return Err(InheritanceError::InvalidInheritance(format!(
                "Maximum inheritance depth {} exceeded at class {}",
                self.max_depth, class_name
            )));
        }

        if visited.contains(class_name) {
            return Ok(());
        }

        path.push(class_name.to_string());

        // Check parent
        if let Some(parent_name) = &class.is_a
            && let Some(parent) = schema.classes.get(parent_name)
        {
            self.check_circular_class(parent_name, parent, schema, visited, path)?;
        }

        // Check mixins
        for mixin_name in &class.mixins {
            if let Some(mixin) = schema.classes.get(mixin_name) {
                self.check_circular_class(mixin_name, mixin, schema, visited, path)?;
            }
        }

        path.pop();
        visited.insert(class_name.to_string());

        Ok(())
    }

    /// Topological sort of classes
    fn topological_sort_classes(
        &self,
        schema: &SchemaDefinition,
    ) -> InheritanceResult<Vec<String>> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        // Build dependency graph
        for (class_name, class_def) in &schema.classes {
            in_degree.entry(class_name.clone()).or_insert(0);
            graph.entry(class_name.clone()).or_default();

            // Add parent dependency
            if let Some(parent) = &class_def.is_a {
                graph
                    .entry(parent.clone())
                    .or_default()
                    .push(class_name.clone());
                *in_degree.entry(class_name.clone()).or_insert(0) += 1;
            }

            // Add mixin dependencies
            for mixin in &class_def.mixins {
                graph
                    .entry(mixin.clone())
                    .or_default()
                    .push(class_name.clone());
                *in_degree.entry(class_name.clone()).or_insert(0) += 1;
            }
        }

        // Kahn's algorithm
        let mut queue = VecDeque::new();
        let mut result = Vec::new();

        // Find nodes with no dependencies
        for (class_name, &degree) in &in_degree {
            if degree == 0 {
                queue.push_back(class_name.clone());
            }
        }

        while let Some(current) = queue.pop_front() {
            result.push(current.clone());

            if let Some(dependents) = graph.get(&current) {
                for dependent in dependents {
                    if let Some(degree) = in_degree.get_mut(dependent) {
                        *degree -= 1;
                        if *degree == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if result.len() != schema.classes.len() {
            return Err(InheritanceError::CircularInheritance(
                "Circular dependency detected in class hierarchy".to_string(),
            ));
        }

        Ok(result)
    }

    /// Resolve slot inheritance
    fn resolve_slots(&self, schema: &mut SchemaDefinition) -> InheritanceResult<()> {
        let slots_to_resolve: Vec<String> = schema.slots.keys().cloned().collect();

        for slot_name in slots_to_resolve {
            if let Some(mut slot) = schema.slots.get(&slot_name).cloned() {
                // Resolve slot inheritance
                if let Some(parent_name) = &slot.is_a
                    && let Some(parent_slot) = schema.slots.get(parent_name)
                {
                    self.merge_slot_attributes(&mut slot, parent_slot);
                }

                // Apply mixins
                for mixin_name in &slot.mixins.clone() {
                    if let Some(mixin_slot) = schema.slots.get(mixin_name) {
                        self.merge_slot_attributes(&mut slot, mixin_slot);
                    }
                }

                schema.slots.insert(slot_name, slot);
            }
        }

        Ok(())
    }

    /// Merge slot attributes
    fn merge_slot_attributes(&self, target: &mut SlotDefinition, source: &SlotDefinition) {
        // Merge basic properties
        if target.description.is_none() {
            target.description.clone_from(&source.description);
        }

        if target.range.is_none() {
            target.range.clone_from(&source.range);
        }

        if target.required.is_none() {
            target.required = source.required;
        }

        if target.multivalued.is_none() {
            target.multivalued = source.multivalued;
        }

        if target.pattern.is_none() {
            target.pattern.clone_from(&source.pattern);
        }

        // Merge constraints
        if target.minimum_value.is_none() {
            target.minimum_value.clone_from(&source.minimum_value);
        }

        if target.maximum_value.is_none() {
            target.maximum_value.clone_from(&source.maximum_value);
        }

        // Merge annotations
        self.merge_annotations(&mut target.annotations, source.annotations.as_ref());
    }

    /// Resolve enum inheritance
    fn resolve_enums(schema: &mut SchemaDefinition) {
        // EnumDefinition doesn't support inheritance in LinkML schema specification
        // (no is_a or mixins fields in the EnumDefinition structure)
        // This method is provided for completeness in the resolution pipeline
        // but enums are atomic definitions that don't inherit from each other

        // Validate enum references in the schema are well-formed
        for (enum_name, enum_def) in &schema.enums {
            if enum_def.permissible_values.is_empty() {
                // Log warning but don't fail - empty enums are technically valid
                tracing::warn!("Enum '{enum_name}' has no permissible values defined");
            }
        }
    }

    /// Clear the resolution cache
    pub fn clear_cache(&mut self) {
        self.resolved_cache.clear();
    }
}

impl Default for InheritanceResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        };

        // Base class
        let mut base_class = ClassDefinition {
            name: "Base".to_string(),
            description: Some("Base class".to_string()),
            ..Default::default()
        };
        base_class.attributes.insert(
            "id".to_string(),
            SlotDefinition {
                name: "id".to_string(),
                required: Some(true),
                ..Default::default()
            },
        );
        base_class.slot_usage.insert(
            "id".to_string(),
            SlotDefinition {
                name: "id".to_string(),
                required: Some(true),
                ..Default::default()
            },
        );

        // Mixin class
        let mut mixin_class = ClassDefinition {
            name: "NamedThing".to_string(),
            mixin: Some(true),
            ..Default::default()
        };
        mixin_class.attributes.insert(
            "name".to_string(),
            SlotDefinition {
                name: "name".to_string(),
                ..Default::default()
            },
        );

        // Derived class
        let mut derived_class = ClassDefinition {
            name: "Person".to_string(),
            is_a: Some("Base".to_string()),
            mixins: vec!["NamedThing".to_string()],
            ..Default::default()
        };
        derived_class.attributes.insert(
            "age".to_string(),
            SlotDefinition {
                name: "age".to_string(),
                ..Default::default()
            },
        );

        schema.classes.insert("Base".to_string(), base_class);
        schema.classes.insert("NamedThing".to_string(), mixin_class);
        schema.classes.insert("Person".to_string(), derived_class);

        schema
    }

    #[test]
    fn test_basic_inheritance() {
        let mut schema = create_test_schema();
        let mut resolver = InheritanceResolver::new();

        resolver
            .resolve_schema(&mut schema)
            .expect("resolution should succeed");

        let person = schema
            .classes
            .get("Person")
            .expect("Person class should exist");

        // Should have inherited attributes
        assert!(person.attributes.contains_key("id"));
        assert!(person.attributes.contains_key("name"));
        assert!(person.attributes.contains_key("age"));

        // Should have inherited slot usage
        assert!(person.slot_usage.contains_key("id"));
    }

    #[test]
    fn test_circular_inheritance_detection() {
        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        };

        // Create circular inheritance
        schema.classes.insert(
            "A".to_string(),
            ClassDefinition {
                name: "A".to_string(),
                is_a: Some("B".to_string()),
                ..Default::default()
            },
        );

        schema.classes.insert(
            "B".to_string(),
            ClassDefinition {
                name: "B".to_string(),
                is_a: Some("C".to_string()),
                ..Default::default()
            },
        );

        schema.classes.insert(
            "C".to_string(),
            ClassDefinition {
                name: "C".to_string(),
                is_a: Some("A".to_string()),
                ..Default::default()
            },
        );

        let mut resolver = InheritanceResolver::new();
        let result = resolver.resolve_schema(&mut schema);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, InheritanceError::CircularInheritance(_)));
        }
    }

    #[test]
    fn test_mixin_validation() {
        let mut schema = create_test_schema();

        // Add invalid mixin reference
        schema.classes.insert(
            "Invalid".to_string(),
            ClassDefinition {
                name: "Invalid".to_string(),
                mixins: vec!["Base".to_string()], // Base is not a mixin
                ..Default::default()
            },
        );

        let mut resolver = InheritanceResolver::new();
        let result = resolver.resolve_schema(&mut schema);

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e, InheritanceError::InvalidInheritance(_)));
        }
    }
}
