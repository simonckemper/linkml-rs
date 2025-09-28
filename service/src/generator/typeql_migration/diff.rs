//! Schema difference detection
//!
//! Compares two schemas and identifies all changes between them.

use linkml_core::prelude::*;
use std::collections::HashSet;

use super::MigrationResult;

/// Represents a change to a type (entity or relation)
#[derive(Debug, Clone, PartialEq)]
pub struct TypeChange {
    /// Name of the type
    pub name: String,
    /// Type before change (None if added)
    pub old_type: Option<ClassDefinition>,
    /// Type after change (None if removed)
    pub new_type: Option<ClassDefinition>,
    /// Specific changes
    pub changes: Vec<DetailedChange>}

/// Represents a change to an attribute
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeChange {
    /// Name of the attribute
    pub name: String,
    /// Attribute before change
    pub old_attr: Option<SlotDefinition>,
    /// Attribute after change
    pub new_attr: Option<SlotDefinition>,
    /// Owner type
    pub owner: String,
    /// Specific changes
    pub changes: Vec<DetailedChange>}

/// Represents a change to a relation
#[derive(Debug, Clone, PartialEq)]
pub struct RelationChange {
    /// Name of the relation
    pub name: String,
    /// Roles before change
    pub old_roles: Vec<String>,
    /// Roles after change
    pub new_roles: Vec<String>,
    /// Role player changes
    pub role_changes: Vec<RolePlayerChange>}

/// Represents a change to a rule
#[derive(Debug, Clone, PartialEq)]
pub struct RuleChange {
    /// Name of the rule
    pub name: String,
    /// Rule before change
    pub old_rule: Option<String>,
    /// Rule after change
    pub new_rule: Option<String>}

/// Detailed change information
#[derive(Debug, Clone, PartialEq)]
pub enum DetailedChange {
    /// Added inheritance
    AddedInheritance(String),
    /// Removed inheritance
    RemovedInheritance(String),
    /// Changed abstract status
    AbstractChanged(bool, bool),
    /// Added slot
    AddedSlot(String),
    /// Removed slot
    RemovedSlot(String),
    /// Changed slot properties
    SlotChanged(String, SlotChange),
    /// Added mixin
    AddedMixin(String),
    /// Removed mixin
    RemovedMixin(String)}

/// Specific slot changes
#[derive(Debug, Clone, PartialEq)]
pub struct SlotChange {
    /// Changed from optional to required or vice versa
    pub required_changed: Option<(bool, bool)>,
    /// Changed cardinality
    pub cardinality_changed: Option<(bool, bool)>,
    /// Changed range/type
    pub range_changed: Option<(String, String)>,
    /// Changed pattern
    pub pattern_changed: Option<(Option<String>, Option<String>)>}

/// Role player change
#[derive(Debug, Clone, PartialEq)]
pub struct RolePlayerChange {
    /// Role name
    pub role: String,
    /// Old player types
    pub old_players: Vec<String>,
    /// New player types
    pub new_players: Vec<String>}

/// Complete schema diff
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    /// Added types
    pub added_types: Vec<TypeChange>,
    /// Removed types
    pub removed_types: Vec<TypeChange>,
    /// Modified types
    pub modified_types: Vec<TypeChange>,
    /// Added attributes
    pub added_attributes: Vec<AttributeChange>,
    /// Removed attributes
    pub removed_attributes: Vec<AttributeChange>,
    /// Modified attributes
    pub modified_attributes: Vec<AttributeChange>,
    /// Relation changes
    pub relation_changes: Vec<RelationChange>,
    /// Rule changes
    pub rule_changes: Vec<RuleChange>}

impl Default for SchemaDiff {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaDiff {
    /// Create an empty diff
    #[must_use] pub fn new() -> Self {
        Self {
            added_types: Vec::new(),
            removed_types: Vec::new(),
            modified_types: Vec::new(),
            added_attributes: Vec::new(),
            removed_attributes: Vec::new(),
            modified_attributes: Vec::new(),
            relation_changes: Vec::new(),
            rule_changes: Vec::new()}
    }

    /// Check if there are any changes
    #[must_use] pub fn is_empty(&self) -> bool {
        self.added_types.is_empty() &&
        self.removed_types.is_empty() &&
        self.modified_types.is_empty() &&
        self.added_attributes.is_empty() &&
        self.removed_attributes.is_empty() &&
        self.modified_attributes.is_empty() &&
        self.relation_changes.is_empty() &&
        self.rule_changes.is_empty()
    }

    /// Count total changes
    #[must_use] pub fn change_count(&self) -> usize {
        self.added_types.len() +
        self.removed_types.len() +
        self.modified_types.len() +
        self.added_attributes.len() +
        self.removed_attributes.len() +
        self.modified_attributes.len() +
        self.relation_changes.len() +
        self.rule_changes.len()
    }
}

/// Schema comparison utility
pub struct SchemaDiffer;

impl SchemaDiffer {
    /// Compare two schemas and generate a diff
    pub fn compare(old_schema: &SchemaDefinition, new_schema: &SchemaDefinition) -> MigrationResult<SchemaDiff> {
        let mut diff = SchemaDiff::new();

        // Compare classes (types)
        Self::compare_classes(old_schema, new_schema, &mut diff)?;

        // Compare global attributes
        Self::compare_attributes(old_schema, new_schema, &mut diff)?;

        // Compare rules (if we had them in the schema)
        // For now, rules are generated from constraints, so we don't compare them directly

        Ok(diff)
    }

    /// Compare classes between schemas
    fn compare_classes(old_schema: &SchemaDefinition, new_schema: &SchemaDefinition, diff: &mut SchemaDiff) -> MigrationResult<()> {
        let old_classes: HashSet<_> = old_schema.classes.keys().collect();
        let new_classes: HashSet<_> = new_schema.classes.keys().collect();

        // Find added classes
        for name in new_classes.difference(&old_classes) {
            if let Some(class) = new_schema.classes.get(*name) {
                diff.added_types.push(TypeChange {
                    name: (*name).clone(),
                    old_type: None,
                    new_type: Some(class.clone()),
                    changes: vec![]});
            }
        }

        // Find removed classes
        for name in old_classes.difference(&new_classes) {
            if let Some(class) = old_schema.classes.get(*name) {
                diff.removed_types.push(TypeChange {
                    name: (*name).clone(),
                    old_type: Some(class.clone()),
                    new_type: None,
                    changes: vec![]});
            }
        }

        // Find modified classes
        for name in old_classes.intersection(&new_classes) {
            if let (Some(old_class), Some(new_class)) = (old_schema.classes.get(*name), new_schema.classes.get(*name)) {
                let changes = Self::compare_class_details(old_class, new_class);
                if !changes.is_empty() {
                    diff.modified_types.push(TypeChange {
                        name: (*name).clone(),
                        old_type: Some(old_class.clone()),
                        new_type: Some(new_class.clone()),
                        changes});
                }

                // Compare class-specific slots
                Self::compare_class_slots(name, old_class, new_class, diff)?;
            }
        }

        Ok(())
    }

    /// Compare details of two class definitions
    fn compare_class_details(old_class: &ClassDefinition, new_class: &ClassDefinition) -> Vec<DetailedChange> {
        let mut changes = Vec::new();

        // Check inheritance changes
        if old_class.is_a != new_class.is_a {
            if let Some(old_parent) = &old_class.is_a {
                if new_class.is_a.is_none() || new_class.is_a.as_ref() != Some(old_parent) {
                    changes.push(DetailedChange::RemovedInheritance(old_parent.clone());
                }
            }
            if let Some(new_parent) = &new_class.is_a {
                if old_class.is_a.is_none() || old_class.is_a.as_ref() != Some(new_parent) {
                    changes.push(DetailedChange::AddedInheritance(new_parent.clone());
                }
            }
        }

        // Check abstract status
        if old_class.abstract_ != new_class.abstract_ {
            changes.push(DetailedChange::AbstractChanged(
                old_class.abstract_.unwrap_or(false),
                new_class.abstract_.unwrap_or(false)
            ));
        }

        // Check mixin changes
        let old_mixins: HashSet<_> = old_class.mixins.iter().collect();
        let new_mixins: HashSet<_> = new_class.mixins.iter().collect();

        for mixin in new_mixins.difference(&old_mixins) {
            changes.push(DetailedChange::AddedMixin((*mixin).clone());
        }

        for mixin in old_mixins.difference(&new_mixins) {
            changes.push(DetailedChange::RemovedMixin((*mixin).clone());
        }

        // Check slot changes (just added/removed, details handled separately)
        let old_slots: HashSet<_> = old_class.slots.iter().collect();
        let new_slots: HashSet<_> = new_class.slots.iter().collect();

        for slot in new_slots.difference(&old_slots) {
            changes.push(DetailedChange::AddedSlot((*slot).clone());
        }

        for slot in old_slots.difference(&new_slots) {
            changes.push(DetailedChange::RemovedSlot((*slot).clone());
        }

        changes
    }

    /// Compare class-specific slots
    fn compare_class_slots(
        class_name: &str,
        old_class: &ClassDefinition,
        new_class: &ClassDefinition,
        diff: &mut SchemaDiff
    ) -> MigrationResult<()> {
        // Compare slot_usage for detailed slot changes
        for (slot_name, old_slot) in &old_class.slot_usage {
            if let Some(new_slot) = new_class.slot_usage.get(slot_name) {
                let slot_changes = Self::compare_slot_details(old_slot, new_slot);
                if let Some(change) = slot_changes {
                    diff.modified_attributes.push(AttributeChange {
                        name: slot_name.clone(),
                        old_attr: Some(old_slot.clone()),
                        new_attr: Some(new_slot.clone()),
                        owner: class_name.to_string(),
                        changes: vec![DetailedChange::SlotChanged(slot_name.clone(), change)]});
                }
            }
        }

        Ok(())
    }

    /// Compare slot details
    fn compare_slot_details(old_slot: &SlotDefinition, new_slot: &SlotDefinition) -> Option<SlotChange> {
        let mut change = SlotChange {
            required_changed: None,
            cardinality_changed: None,
            range_changed: None,
            pattern_changed: None};

        let mut has_changes = false;

        // Check required status
        if old_slot.required != new_slot.required {
            change.required_changed = Some((
                old_slot.required.unwrap_or(false),
                new_slot.required.unwrap_or(false)
            ));
            has_changes = true;
        }

        // Check cardinality
        if old_slot.multivalued != new_slot.multivalued {
            change.cardinality_changed = Some((
                old_slot.multivalued.unwrap_or(false),
                new_slot.multivalued.unwrap_or(false)
            ));
            has_changes = true;
        }

        // Check range/type
        if old_slot.range != new_slot.range {
            if let (Some(old_range), Some(new_range)) = (&old_slot.range, &new_slot.range) {
                change.range_changed = Some((old_range.clone(), new_range.clone());
                has_changes = true;
            }
        }

        // Check pattern
        if old_slot.pattern != new_slot.pattern {
            change.pattern_changed = Some((old_slot.pattern.clone(), new_slot.pattern.clone());
            has_changes = true;
        }

        if has_changes {
            Some(change)
        } else {
            None
        }
    }

    /// Compare global attributes
    fn compare_attributes(old_schema: &SchemaDefinition, new_schema: &SchemaDefinition, diff: &mut SchemaDiff) -> MigrationResult<()> {
        let old_slots: HashSet<_> = old_schema.slots.keys().collect();
        let new_slots: HashSet<_> = new_schema.slots.keys().collect();

        // Find added slots
        for name in new_slots.difference(&old_slots) {
            if let Some(slot) = new_schema.slots.get(*name) {
                diff.added_attributes.push(AttributeChange {
                    name: (*name).clone(),
                    old_attr: None,
                    new_attr: Some(slot.clone()),
                    owner: "global".to_string(),
                    changes: vec![]});
            }
        }

        // Find removed slots
        for name in old_slots.difference(&new_slots) {
            if let Some(slot) = old_schema.slots.get(*name) {
                diff.removed_attributes.push(AttributeChange {
                    name: (*name).clone(),
                    old_attr: Some(slot.clone()),
                    new_attr: None,
                    owner: "global".to_string(),
                    changes: vec![]});
            }
        }

        // Find modified slots
        for name in old_slots.intersection(&new_slots) {
            if let (Some(old_slot), Some(new_slot)) = (old_schema.slots.get(*name), new_schema.slots.get(*name)) {
                if let Some(change) = Self::compare_slot_details(old_slot, new_slot) {
                    diff.modified_attributes.push(AttributeChange {
                        name: (*name).clone(),
                        old_attr: Some(old_slot.clone()),
                        new_attr: Some(new_slot.clone()),
                        owner: "global".to_string(),
                        changes: vec![DetailedChange::SlotChanged((*name).clone(), change)]});
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};

    #[test]
    fn test_empty_diff() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = SchemaDefinition::default();
        let diff = SchemaDiffer::compare(&schema, &schema).expect("should compare schemas: {}");
        assert!(diff.is_empty());
        assert_eq!(diff.change_count(), 0);
        Ok(())
    }

    #[test]
    fn test_added_class() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = SchemaDefinition::default();
        let mut new_schema = SchemaDefinition::default();

        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        new_schema.classes.insert("Person".to_string(), person_class);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should compare schemas: {}");
        assert_eq!(diff.added_types.len(), 1);
        assert_eq!(diff.added_types[0].name, "Person");
        Ok(())
    }

    #[test]
    fn test_removed_class() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut old_schema = SchemaDefinition::default();
        let new_schema = SchemaDefinition::default();

        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        old_schema.classes.insert("Person".to_string(), person_class);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should compare schemas: {}");
        assert_eq!(diff.removed_types.len(), 1);
        assert_eq!(diff.removed_types[0].name, "Person");
        Ok(())
    }

    #[test]
    fn test_modified_class() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut old_schema = SchemaDefinition::default();
        let mut new_schema = SchemaDefinition::default();

        let mut old_class = ClassDefinition::default();
        old_class.abstract_ = Some(false);
        old_schema.classes.insert("Person".to_string(), old_class);

        let mut new_class = ClassDefinition::default();
        new_class.abstract_ = Some(true);
        new_schema.classes.insert("Person".to_string(), new_class);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should compare schemas: {}");
        assert_eq!(diff.modified_types.len(), 1);
        assert_eq!(diff.modified_types[0].name, "Person");
        assert!(matches!(
            &diff.modified_types[0].changes[0],
            DetailedChange::AbstractChanged(false, true)
        ));
        Ok(())
    }
}