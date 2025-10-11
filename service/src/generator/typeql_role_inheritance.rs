//! Role inheritance resolution for `TypeQL` generation
//!
//! This module handles role inheritance, specialization, and abstract roles
//! in `TypeQL` relations, supporting `TypeDB`'s advanced role hierarchy features.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Information about role inheritance
#[derive(Debug, Clone)]
pub struct RoleInheritance {
    /// Base role name
    pub base_role: String,
    /// Specialized role name
    pub specialized_role: String,
    /// Relation where specialization occurs
    pub relation: String,
    /// Constraints on the specialization
    pub constraints: Vec<String>,
}

/// Tracks role hierarchy and specialization
#[derive(Debug, Clone)]
pub struct RoleHierarchy {
    /// Parent role -> child roles mapping
    pub children: HashMap<String, Vec<String>>,
    /// Child role -> parent role mapping
    pub parent: HashMap<String, String>,
    /// Abstract roles
    pub abstract_roles: HashSet<String>,
    /// Role specializations (relation:role -> `base_role`)
    pub specializations: HashMap<String, String>,
}

/// Resolves role inheritance in `TypeQL` schemas
pub struct RoleInheritanceResolver {
    /// Role hierarchies by relation
    pub hierarchies: HashMap<String, RoleHierarchy>,
    /// Global role registry
    global_roles: HashMap<String, RoleDefinition>,
}

/// Definition of a role
#[derive(Debug, Clone)]

struct RoleDefinition {
    /// Role name (kept for future use)
    _name: String,
    /// Relation it belongs to (kept for future use)
    _relation: String,
    /// Whether it's abstract (kept for future use)
    _is_abstract: bool,
    /// Base role if specialized (kept for future use)
    _base_role: Option<String>,
    /// Allowed player types
    allowed_players: Vec<String>,
}

impl Default for RoleInheritanceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl RoleInheritanceResolver {
    /// Create a new role inheritance resolver
    #[must_use]
    pub fn new() -> Self {
        Self {
            hierarchies: HashMap::new(),
            global_roles: HashMap::new(),
        }
    }

    /// Analyze role inheritance for a relation
    pub fn analyze_relation_inheritance(
        &mut self,
        relation_name: &str,
        relation_class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Option<RoleHierarchy> {
        let mut hierarchy = RoleHierarchy {
            children: HashMap::new(),
            parent: HashMap::new(),
            abstract_roles: HashSet::new(),
            specializations: HashMap::new(),
        };

        // Check if relation inherits from another
        if let Some(parent_name) = &relation_class.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            // Analyze parent relation first
            if let Some(parent_hierarchy) =
                self.analyze_relation_inheritance(parent_name, parent_class, schema)
            {
                // Inherit parent's role structure
                hierarchy = self.merge_hierarchies(hierarchy, parent_hierarchy);
            }

            // Check for role specializations
            self.detect_role_specializations(
                relation_name,
                relation_class,
                parent_name,
                parent_class,
                schema,
                &mut hierarchy,
            );
        }

        // Check for abstract relation
        if relation_class.abstract_.unwrap_or(false) {
            // Mark all roles as potentially abstract
            for slot_name in &relation_class.slots {
                if self.is_role_slot(slot_name, schema) {
                    hierarchy.abstract_roles.insert(slot_name.clone());
                }
            }
        }

        // Cache the hierarchy
        self.hierarchies
            .insert(relation_name.to_string(), hierarchy.clone());

        Some(hierarchy)
    }

    /// Detect role specializations between parent and child relations
    fn detect_role_specializations(
        &mut self,
        child_relation: &str,
        child_class: &ClassDefinition,
        _parent_relation: &str,
        parent_class: &ClassDefinition,
        schema: &SchemaDefinition,
        hierarchy: &mut RoleHierarchy,
    ) {
        // Get parent roles
        let parent_roles = self.get_relation_roles(parent_class, schema);
        let child_roles = self.get_relation_roles(child_class, schema);

        // Check for role specializations
        for (child_role, child_type) in &child_roles {
            // Check if this might be a specialization
            if let Some((parent_role, _parent_type)) =
                self.find_matching_parent_role(child_role, child_type, &parent_roles, schema)
            {
                // This is a specialization
                let spec_key = format!("{child_relation}:{child_role}");
                hierarchy
                    .specializations
                    .insert(spec_key, parent_role.clone());

                // Update parent-child relationships
                hierarchy
                    .parent
                    .insert(child_role.clone(), parent_role.clone());
                hierarchy
                    .children
                    .entry(parent_role.clone())
                    .or_default()
                    .push(child_role.clone());
            }
        }
    }

    /// Find matching parent role for potential specialization
    fn find_matching_parent_role(
        &self,
        child_role: &str,
        child_type: &str,
        parent_roles: &HashMap<String, String>,
        schema: &SchemaDefinition,
    ) -> Option<(String, String)> {
        // Direct name match
        if let Some(parent_type) = parent_roles.get(child_role) {
            // Check if child type is subtype of parent type
            if self.is_subtype_of(child_type, parent_type, schema) {
                return Some((child_role.to_string(), parent_type.clone()));
            }
        }

        // Check for naming patterns (e.g., "participant" -> "student")
        for (parent_role, parent_type) in parent_roles {
            if (child_role.contains(parent_role)
                || self.is_semantic_specialization(child_role, parent_role))
                && self.is_subtype_of(child_type, parent_type, schema)
            {
                return Some((parent_role.clone(), parent_type.clone()));
            }
        }

        None
    }

    /// Check if one type is a subtype of another
    fn is_subtype_of(&self, child: &str, parent: &str, schema: &SchemaDefinition) -> bool {
        if child == parent {
            return true;
        }

        if let Some(child_class) = schema.classes.get(child)
            && let Some(parent_name) = &child_class.is_a
        {
            if parent_name == parent {
                return true;
            }
            // Recursive check
            return self.is_subtype_of(parent_name, parent, schema);
        }

        false
    }

    /// Check if role names indicate semantic specialization
    fn is_semantic_specialization(&self, specialized: &str, base: &str) -> bool {
        // Common specialization patterns
        let patterns = [
            ("participant", ["student", "teacher", "member"]),
            ("party", ["buyer", "seller", "agent"]),
            ("actor", ["performer", "director", "producer"]),
            ("entity", ["person", "organization", "system"]),
        ];

        for (base_pattern, specializations) in &patterns {
            if base.contains(base_pattern) {
                for spec in specializations {
                    if specialized.contains(spec) {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Get all roles in a relation
    fn get_relation_roles(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> HashMap<String, String> {
        let mut roles = HashMap::new();

        for slot_name in &class.slots {
            if let Some(slot) = schema
                .slots
                .get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name))
                && let Some(range) = &slot.range
                && schema.classes.contains_key(range)
            {
                roles.insert(slot_name.clone(), range.clone());
            }
        }

        roles
    }

    /// Check if a slot represents a role
    fn is_role_slot(&self, slot_name: &str, schema: &SchemaDefinition) -> bool {
        if let Some(slot) = schema.slots.get(slot_name)
            && let Some(range) = &slot.range
        {
            return schema.classes.contains_key(range);
        }
        false
    }

    /// Merge two role hierarchies
    fn merge_hierarchies(&self, mut child: RoleHierarchy, parent: RoleHierarchy) -> RoleHierarchy {
        // Merge children
        for (role, children) in parent.children {
            child.children.entry(role).or_default().extend(children);
        }

        // Merge parents
        child.parent.extend(parent.parent);

        // Merge abstract roles
        child.abstract_roles.extend(parent.abstract_roles);

        // Merge specializations
        child.specializations.extend(parent.specializations);

        child
    }

    /// Generate `TypeQL` for role inheritance
    #[must_use]
    pub fn generate_role_inheritance_typeql(
        &self,
        _relation_name: &str,
        role_name: &str,
        base_role: &str,
    ) -> String {
        format!("    relates {role_name} as {base_role}")
    }

    /// Get all abstract roles in the schema
    #[must_use]
    pub fn get_abstract_roles(&self) -> Vec<String> {
        let mut abstract_roles = Vec::new();

        for hierarchy in self.hierarchies.values() {
            abstract_roles.extend(hierarchy.abstract_roles.iter().cloned());
        }

        abstract_roles.sort();
        abstract_roles.dedup();
        abstract_roles
    }

    /// Check if a role can be played by multiple types (polymorphic)
    #[must_use]
    pub fn is_polymorphic_role(&self, relation: &str, role: &str) -> bool {
        let role_key = format!("{relation}:{role}");

        if let Some(role_def) = self.global_roles.get(&role_key) {
            // Polymorphic if multiple types can play it or if base type has subtypes
            role_def.allowed_players.len() > 1
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_specialization_detection() {
        let resolver = RoleInheritanceResolver::new();

        // Test semantic specialization
        assert!(resolver.is_semantic_specialization("student", "participant"));
        assert!(resolver.is_semantic_specialization("buyer", "party"));
        assert!(!resolver.is_semantic_specialization("name", "attribute"));
    }

    #[test]
    fn test_subtype_checking() {
        let mut schema = SchemaDefinition::default();

        // Create type hierarchy
        let person = ClassDefinition::default();
        schema.classes.insert("Person".to_string(), person);

        let student = ClassDefinition {
            is_a: Some("Person".to_string()),
            ..Default::default()
        };
        schema.classes.insert("Student".to_string(), student);

        let resolver = RoleInheritanceResolver::new();

        assert!(resolver.is_subtype_of("Student", "Person", &schema));
        assert!(resolver.is_subtype_of("Person", "Person", &schema));
        assert!(!resolver.is_subtype_of("Person", "Student", &schema));
    }
}
