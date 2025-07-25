//! Advanced relation analysis for TypeQL generation
//!
//! This module provides sophisticated analysis of LinkML relationships
//! to generate optimal TypeQL relations, including multi-way relations,
//! nested relations, and role detection.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use serde_json::Value;

/// Information about a detected relation
#[derive(Debug, Clone)]
pub struct RelationInfo {
    /// Name of the relation
    pub name: String,
    /// Roles in the relation (slot name -> entity type)
    pub roles: Vec<RoleInfo>,
    /// Attributes owned by the relation
    pub attributes: Vec<String>,
    /// Whether this is a nested relation (can play roles)
    pub is_nested: bool,
    /// Whether this is a multi-way relation (3+ roles)
    pub is_multiway: bool,
}

/// Information about a role in a relation
#[derive(Debug, Clone)]
pub struct RoleInfo {
    /// Role name (usually the slot name)
    pub name: String,
    /// Entity type that plays this role
    pub player_type: String,
    /// Whether this role is required
    pub required: bool,
    /// Cardinality constraints
    pub cardinality: Option<(usize, Option<usize>)>,
    /// Whether this role is inherited
    pub is_inherited: bool,
}

/// Analyzes relations in LinkML schemas for TypeQL generation
pub struct RelationAnalyzer {
    /// Cache of analyzed relations
    relation_cache: HashMap<String, RelationInfo>,
    /// Map of which entities can play which roles
    role_player_map: HashMap<String, Vec<String>>,
}

impl RelationAnalyzer {
    /// Create a new relation analyzer
    #[must_use]
    pub fn new() -> Self {
        Self {
            relation_cache: HashMap::new(),
            role_player_map: HashMap::new(),
        }
    }

    /// Analyze a class to determine if it's a relation and extract roles
    pub fn analyze_relation(
        &mut self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Option<RelationInfo> {
        // Check cache first
        if let Some(cached) = self.relation_cache.get(class_name) {
            return Some(cached.clone());
        }

        // Determine if this is a relation
        if !self.is_relation_class(class_name, class, schema) {
            return None;
        }

        // Extract roles and attributes
        let (roles, attributes) = self.extract_roles_and_attributes(class, schema);

        // Check if this is a nested relation
        let is_nested = self.is_nested_relation(class_name, schema);

        // Create relation info
        let is_multiway = roles.len() > 2;
        let relation_info = RelationInfo {
            name: class_name.to_string(),
            roles,
            attributes,
            is_nested,
            is_multiway,
        };

        // Cache the result
        self.relation_cache.insert(class_name.to_string(), relation_info.clone());

        // Update role player map
        self.update_role_player_map(&relation_info);

        Some(relation_info)
    }

    /// Determine if a class represents a relation
    fn is_relation_class(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> bool {
        // Check name patterns
        let name_lower = class_name.to_lowercase();
        if name_lower.contains("relationship") 
            || name_lower.contains("association")
            || name_lower.contains("membership")
            || name_lower.contains("enrollment")
            || name_lower.contains("employment") {
            return true;
        }

        // Count object-valued slots
        let object_slots = self.count_object_valued_slots(class, schema);
        
        // If has 2+ object-valued slots, likely a relation
        if object_slots >= 2 {
            return true;
        }

        // Check description for relation indicators
        if let Some(desc) = &class.description {
            let desc_lower = desc.to_lowercase();
            if desc_lower.contains("relation") 
                || desc_lower.contains("links")
                || desc_lower.contains("connects")
                || desc_lower.contains("between") {
                return true;
            }
        }

        false
    }

    /// Count object-valued slots in a class
    fn count_object_valued_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> usize {
        class.slots.iter()
            .filter(|slot_name| {
                if let Some(slot) = schema.slots.get(*slot_name)
                    .or_else(|| class.slot_usage.get(*slot_name)) {
                    if let Some(range) = &slot.range {
                        // Check if range is a class (not a type)
                        return schema.classes.contains_key(range);
                    }
                }
                false
            })
            .count()
    }

    /// Extract roles and attributes from a relation class
    fn extract_roles_and_attributes(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> (Vec<RoleInfo>, Vec<String>) {
        let mut roles = Vec::new();
        let mut attributes = Vec::new();

        for slot_name in &class.slots {
            if let Some(slot) = schema.slots.get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name)) {
                
                if let Some(range) = &slot.range {
                    if schema.classes.contains_key(range) {
                        // This is a role
                        let role_info = RoleInfo {
                            name: slot_name.clone(),
                            player_type: range.clone(),
                            required: slot.required.unwrap_or(false),
                            cardinality: self.get_slot_cardinality(slot),
                            is_inherited: false, // Will be set later
                        };
                        roles.push(role_info);
                    } else {
                        // This is an attribute
                        attributes.push(slot_name.clone());
                    }
                } else {
                    // No range specified, assume attribute
                    attributes.push(slot_name.clone());
                }
            }
        }

        // Sort roles by name for consistent output
        roles.sort_by(|a, b| a.name.cmp(&b.name));

        (roles, attributes)
    }

    /// Get cardinality for a slot
    fn get_slot_cardinality(&self, slot: &SlotDefinition) -> Option<(usize, Option<usize>)> {
        let min = if slot.required.unwrap_or(false) { 1 } else { 0 };
        let max = if slot.multivalued.unwrap_or(false) {
            if let Some(Value::Number(max_card)) = &slot.maximum_value {
                max_card.as_u64().map(|n| n as usize)
            } else {
                None
            }
        } else {
            Some(1)
        };

        // Only return if different from default (0..1 or 1..1)
        match (min, max) {
            (0, Some(1)) if !slot.required.unwrap_or(false) => None,
            (1, Some(1)) if slot.required.unwrap_or(false) => None,
            _ => Some((min, max)),
        }
    }

    /// Check if this relation can be nested (play roles in other relations)
    fn is_nested_relation(&self, class_name: &str, schema: &SchemaDefinition) -> bool {
        // Check if any other class has a slot with this class as range
        for (_, other_class) in &schema.classes {
            for slot_name in &other_class.slots {
                if let Some(slot) = schema.slots.get(slot_name)
                    .or_else(|| other_class.slot_usage.get(slot_name)) {
                    if let Some(range) = &slot.range {
                        if range == class_name {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Update the role player map with information from a relation
    fn update_role_player_map(&mut self, relation_info: &RelationInfo) {
        for role in &relation_info.roles {
            self.role_player_map
                .entry(role.player_type.clone())
                .or_insert_with(Vec::new)
                .push(format!("{}:{}", relation_info.name, role.name));
        }
    }

    /// Get all roles that an entity type can play
    pub fn get_playable_roles(&self, entity_type: &str) -> Vec<String> {
        self.role_player_map
            .get(entity_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Analyze role inheritance in a relation
    pub fn analyze_role_inheritance(
        &mut self,
        relation_info: &mut RelationInfo,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) {
        // Check if relation inherits from another
        if let Some(parent_name) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent_name) {
                if let Some(parent_info) = self.analyze_relation(parent_name, parent_class, schema) {
                    // Mark inherited roles
                    for role in &mut relation_info.roles {
                        if parent_info.roles.iter().any(|pr| pr.name == role.name) {
                            role.is_inherited = true;
                        }
                    }
                }
            }
        }
    }

    /// Detect polymorphic roles (multiple types playing same role)
    pub fn detect_polymorphic_roles(&self, schema: &SchemaDefinition) -> HashMap<String, Vec<String>> {
        let mut polymorphic_roles: HashMap<String, HashSet<String>> = HashMap::new();

        // Analyze all relations
        for (rel_name, _rel_class) in &schema.classes {
            if let Some(rel_info) = self.relation_cache.get(rel_name) {
                for role in &rel_info.roles {
                    // Check if player type has subtypes
                    let player_subtypes = self.get_all_subtypes(&role.player_type, schema);
                    if player_subtypes.len() > 1 {
                        let role_key = format!("{}:{}", rel_name, role.name);
                        polymorphic_roles.insert(role_key, player_subtypes);
                    }
                }
            }
        }

        // Convert to Vec
        polymorphic_roles.into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect()
    }

    /// Get all subtypes of a class (including itself)
    fn get_all_subtypes(&self, class_name: &str, schema: &SchemaDefinition) -> HashSet<String> {
        let mut subtypes = HashSet::new();
        subtypes.insert(class_name.to_string());

        // Find all classes that inherit from this one
        for (name, class) in &schema.classes {
            if let Some(parent) = &class.is_a {
                if parent == class_name {
                    // Recursively get subtypes
                    subtypes.extend(self.get_all_subtypes(name, schema));
                }
            }
        }

        subtypes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relation_detection() {
        let mut analyzer = RelationAnalyzer::new();
        let schema = create_test_schema();
        
        // Test employment relation detection
        let employment_class = schema.classes.get("Employment").expect("Employment class should exist");
        let relation_info = analyzer.analyze_relation("Employment", employment_class, &schema);
        
        assert!(relation_info.is_some());
        let info = relation_info.expect("relation info should exist");
        assert_eq!(info.roles.len(), 2);
        assert!(!info.is_multiway);
    }

    #[test]
    fn test_multiway_relation() {
        let mut analyzer = RelationAnalyzer::new();
        let schema = create_multiway_schema();
        
        // Test enrollment relation (student, course, instructor)
        let enrollment_class = schema.classes.get("Enrollment").expect("Enrollment class should exist");
        let relation_info = analyzer.analyze_relation("Enrollment", enrollment_class, &schema);
        
        assert!(relation_info.is_some());
        let info = relation_info.expect("relation info should exist");
        assert_eq!(info.roles.len(), 3);
        assert!(info.is_multiway);
    }

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        
        // Add basic classes
        schema.classes.insert("Person".to_string(), ClassDefinition::default());
        schema.classes.insert("Organization".to_string(), ClassDefinition::default());
        
        // Add employment relation
        let mut employment = ClassDefinition::default();
        employment.slots = vec!["employee".to_string(), "employer".to_string()];
        
        // Add slots
        let mut employee_slot = SlotDefinition::default();
        employee_slot.range = Some("Person".to_string());
        schema.slots.insert("employee".to_string(), employee_slot);
        
        let mut employer_slot = SlotDefinition::default();
        employer_slot.range = Some("Organization".to_string());
        schema.slots.insert("employer".to_string(), employer_slot);
        
        schema.classes.insert("Employment".to_string(), employment);
        schema
    }

    fn create_multiway_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        
        // Add entity classes
        schema.classes.insert("Student".to_string(), ClassDefinition::default());
        schema.classes.insert("Course".to_string(), ClassDefinition::default());
        schema.classes.insert("Instructor".to_string(), ClassDefinition::default());
        
        // Add enrollment relation
        let mut enrollment = ClassDefinition::default();
        enrollment.slots = vec![
            "student".to_string(),
            "course".to_string(),
            "instructor".to_string(),
            "grade".to_string(),
        ];
        
        // Add role slots
        let mut student_slot = SlotDefinition::default();
        student_slot.range = Some("Student".to_string());
        schema.slots.insert("student".to_string(), student_slot);
        
        let mut course_slot = SlotDefinition::default();
        course_slot.range = Some("Course".to_string());
        schema.slots.insert("course".to_string(), course_slot);
        
        let mut instructor_slot = SlotDefinition::default();
        instructor_slot.range = Some("Instructor".to_string());
        schema.slots.insert("instructor".to_string(), instructor_slot);
        
        // Add attribute slot
        let mut grade_slot = SlotDefinition::default();
        grade_slot.range = Some("string".to_string());
        schema.slots.insert("grade".to_string(), grade_slot);
        
        schema.classes.insert("Enrollment".to_string(), enrollment);
        schema
    }
}