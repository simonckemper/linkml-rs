//! Migration impact analysis
//!
//! Analyzes schema changes to determine their impact and categorize them.

use std::collections::HashSet;

use super::MigrationResult;
use super::diff::{
    SchemaDiff, TypeChange, AttributeChange,
    DetailedChange, SlotChange
};

/// Category of change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeCategory {
    /// Safe changes that don't affect existing data
    Safe,
    /// Changes that might need attention but are generally safe
    Warning,
    /// Breaking changes that require data migration
    Breaking}

/// Impact analysis result
#[derive(Debug, Clone)]
pub struct ChangeImpact {
    /// Overall category
    pub category: ChangeCategory,
    /// Specific breaking changes
    pub breaking_changes: Vec<String>,
    /// Warning-level changes
    pub warnings: Vec<String>,
    /// Safe changes
    pub safe_changes: Vec<String>,
    /// Estimated migration complexity (1-10)
    pub complexity_score: u8,
    /// Data migration required
    pub requires_data_migration: bool,
    /// Affected types
    pub affected_types: HashSet<String>,
    /// Recommended migration strategy
    pub migration_strategy: String}

impl Default for ChangeImpact {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangeImpact {
    /// Create a new impact analysis
    #[must_use] pub fn new() -> Self {
        Self {
            category: ChangeCategory::Safe,
            breaking_changes: Vec::new(),
            warnings: Vec::new(),
            safe_changes: Vec::new(),
            complexity_score: 0,
            requires_data_migration: false,
            affected_types: HashSet::new(),
            migration_strategy: String::new()}
    }

    /// Check if there are breaking changes
    #[must_use] pub fn has_breaking_changes(&self) -> bool {
        !self.breaking_changes.is_empty()
    }

    /// Check if there are warnings
    #[must_use] pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    /// Update category based on accumulated changes
    fn update_category(&mut self) {
        if !self.breaking_changes.is_empty() {
            self.category = ChangeCategory::Breaking;
        } else if !self.warnings.is_empty() {
            self.category = ChangeCategory::Warning;
        } else {
            self.category = ChangeCategory::Safe;
        }
    }

    /// Calculate complexity score
    fn calculate_complexity(&mut self) {
        let breaking_weight = 3;
        let warning_weight = 1;

        let score = (self.breaking_changes.len() * breaking_weight +
                    self.warnings.len() * warning_weight) as u8;

        self.complexity_score = score.min(10);
    }
}

/// Migration impact analyzer
pub struct MigrationAnalyzer;

impl MigrationAnalyzer {
    /// Analyze the impact of schema changes
    pub fn analyze_impact(diff: &SchemaDiff) -> MigrationResult<ChangeImpact> {
        let mut impact = ChangeImpact::new();

        // Analyze type changes
        Self::analyze_type_additions(&diff.added_types, &mut impact);
        Self::analyze_type_removals(&diff.removed_types, &mut impact);
        Self::analyze_type_modifications(&diff.modified_types, &mut impact);

        // Analyze attribute changes
        Self::analyze_attribute_additions(&diff.added_attributes, &mut impact);
        Self::analyze_attribute_removals(&diff.removed_attributes, &mut impact);
        Self::analyze_attribute_modifications(&diff.modified_attributes, &mut impact);

        // Generate migration strategy
        impact.migration_strategy = Self::recommend_strategy(&impact);

        // Update final state
        impact.update_category();
        impact.calculate_complexity();

        Ok(impact)
    }

    /// Analyze added types
    fn analyze_type_additions(added: &[TypeChange], impact: &mut ChangeImpact) {
        for type_change in added {
            impact.safe_changes.push(format!("Added new type: {}", type_change.name));
            impact.affected_types.insert(type_change.name.clone());
        }
    }

    /// Analyze removed types
    fn analyze_type_removals(removed: &[TypeChange], impact: &mut ChangeImpact) {
        for type_change in removed {
            impact.breaking_changes.push(format!(
                "Removed type: {}. All instances will be lost!",
                type_change.name
            ));
            impact.affected_types.insert(type_change.name.clone());
            impact.requires_data_migration = true;
        }
    }

    /// Analyze modified types
    fn analyze_type_modifications(modified: &[TypeChange], impact: &mut ChangeImpact) {
        for type_change in modified {
            impact.affected_types.insert(type_change.name.clone());

            for change in &type_change.changes {
                match change {
                    DetailedChange::AddedInheritance(parent) => {
                        impact.warnings.push(format!(
                            "Type {} now inherits from {}. Existing instances may need validation.",
                            type_change.name, parent
                        ));
                    }
                    DetailedChange::RemovedInheritance(parent) => {
                        impact.breaking_changes.push(format!(
                            "Type {} no longer inherits from {}. Inherited attributes will be lost!",
                            type_change.name, parent
                        ));
                        impact.requires_data_migration = true;
                    }
                    DetailedChange::AbstractChanged(old, new) => {
                        if !old && *new {
                            impact.warnings.push(format!(
                                "Type {} changed to abstract. No new instances can be created.",
                                type_change.name
                            ));
                        } else {
                            impact.safe_changes.push(format!(
                                "Type {} is no longer abstract.",
                                type_change.name
                            ));
                        }
                    }
                    DetailedChange::AddedSlot(slot) => {
                        impact.safe_changes.push(format!(
                            "Added slot {} to type {}",
                            slot, type_change.name
                        ));
                    }
                    DetailedChange::RemovedSlot(slot) => {
                        impact.breaking_changes.push(format!(
                            "Removed slot {} from type {}. Data will be lost!",
                            slot, type_change.name
                        ));
                        impact.requires_data_migration = true;
                    }
                    DetailedChange::SlotChanged(slot, slot_change) => {
                        Self::analyze_slot_change(&type_change.name, slot, slot_change, impact);
                    }
                    DetailedChange::AddedMixin(mixin) => {
                        impact.safe_changes.push(format!(
                            "Type {} now includes mixin {}",
                            type_change.name, mixin
                        ));
                    }
                    DetailedChange::RemovedMixin(mixin) => {
                        impact.warnings.push(format!(
                            "Type {} no longer includes mixin {}",
                            type_change.name, mixin
                        ));
                    }
                }
            }
        }
    }

    /// Analyze changes to a slot
    fn analyze_slot_change(type_name: &str, slot_name: &str, change: &SlotChange, impact: &mut ChangeImpact) {
        if let Some((old_req, new_req)) = change.required_changed {
            if !old_req && new_req {
                impact.breaking_changes.push(format!(
                    "Slot {slot_name} in type {type_name} is now required. Existing instances without this field will be invalid!"
                ));
                impact.requires_data_migration = true;
            } else {
                impact.safe_changes.push(format!(
                    "Slot {slot_name} in type {type_name} is now optional"
                ));
            }
        }

        if let Some((old_multi, new_multi)) = change.cardinality_changed {
            if old_multi && !new_multi {
                impact.breaking_changes.push(format!(
                    "Slot {slot_name} in type {type_name} changed from multi-valued to single-valued. Extra values will be lost!"
                ));
                impact.requires_data_migration = true;
            } else {
                impact.safe_changes.push(format!(
                    "Slot {slot_name} in type {type_name} is now multi-valued"
                ));
            }
        }

        if let Some((old_range, new_range)) = &change.range_changed {
            impact.warnings.push(format!(
                "Slot {slot_name} in type {type_name} changed type from {old_range} to {new_range}. Type conversion may be needed."
            ));
            impact.requires_data_migration = true;
        }

        if let Some((old_pattern, new_pattern)) = &change.pattern_changed {
            if new_pattern.is_some() && old_pattern.is_none() {
                impact.warnings.push(format!(
                    "Slot {slot_name} in type {type_name} now has a pattern constraint. Existing values may not match."
                ));
            } else if new_pattern.is_none() && old_pattern.is_some() {
                impact.safe_changes.push(format!(
                    "Removed pattern constraint from slot {slot_name} in type {type_name}"
                ));
            } else {
                impact.warnings.push(format!(
                    "Pattern constraint changed for slot {slot_name} in type {type_name}"
                ));
            }
        }
    }

    /// Analyze added attributes
    fn analyze_attribute_additions(added: &[AttributeChange], impact: &mut ChangeImpact) {
        for attr in added {
            if attr.new_attr.as_ref().is_some_and(|a| a.required.unwrap_or(false)) {
                impact.warnings.push(format!(
                    "Added required attribute: {}. Existing instances will need this field.",
                    attr.name
                ));
            } else {
                impact.safe_changes.push(format!("Added optional attribute: {}", attr.name));
            }
        }
    }

    /// Analyze removed attributes
    fn analyze_attribute_removals(removed: &[AttributeChange], impact: &mut ChangeImpact) {
        for attr in removed {
            impact.breaking_changes.push(format!(
                "Removed attribute: {}. All data for this attribute will be lost!",
                attr.name
            ));
            impact.requires_data_migration = true;
        }
    }

    /// Analyze modified attributes
    fn analyze_attribute_modifications(modified: &[AttributeChange], impact: &mut ChangeImpact) {
        for attr in modified {
            for change in &attr.changes {
                if let DetailedChange::SlotChanged(slot_name, slot_change) = change {
                    Self::analyze_slot_change(&attr.owner, slot_name, slot_change, impact);
                }
            }
        }
    }

    /// Recommend migration strategy based on impact
    fn recommend_strategy(impact: &ChangeImpact) -> String {
        if impact.breaking_changes.is_empty() {
            if impact.warnings.is_empty() {
                "Simple schema update - no data migration required".to_string()
            } else {
                "Schema update with warnings - validate existing data after migration".to_string()
            }
        } else {
            let mut strategy = String::from("Complex migration required:
");

            if impact.requires_data_migration {
                strategy.push_str("1. Backup all data before migration
");
                strategy.push_str("2. Create data transformation queries
");
                strategy.push_str("3. Validate data integrity after migration
");
            }

            if impact.complexity_score > 5 {
                strategy.push_str("4. Consider phased migration due to high complexity
");
                strategy.push_str("5. Test migration on non-production environment first
");
            }

            strategy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::typeql_migration::diff::SchemaDiffer;
    use linkml_core::prelude::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};

    #[test]
    fn test_safe_changes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = SchemaDefinition::default();
        let mut new_schema = SchemaDefinition::default();

        // Add a new optional field
        let mut slot = SlotDefinition::default();
        slot.required = Some(false);
        new_schema.slots.insert("new_field".to_string(), slot);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should generate schema diff for safe changes: {}");
        let impact = MigrationAnalyzer::analyze_impact(&diff).expect("should analyze impact for safe changes: {}");

        assert_eq!(impact.category, ChangeCategory::Safe);
        assert!(!impact.has_breaking_changes());
        assert!(!impact.requires_data_migration);
        Ok(())
    }

    #[test]
    fn test_breaking_changes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut old_schema = SchemaDefinition::default();
        let new_schema = SchemaDefinition::default();

        // Add a class to old schema that's removed in new
        let class = ClassDefinition::default();
        old_schema.classes.insert("RemovedClass".to_string(), class);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should generate schema diff for breaking changes: {}");
        let impact = MigrationAnalyzer::analyze_impact(&diff).expect("should analyze impact for breaking changes: {}");

        assert_eq!(impact.category, ChangeCategory::Breaking);
        assert!(impact.has_breaking_changes());
        assert!(impact.requires_data_migration);
        assert_eq!(impact.breaking_changes.len(), 1);
        Ok(())
    }

    #[test]
    fn test_warning_changes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = SchemaDefinition::default();
        let mut new_schema = SchemaDefinition::default();

        // Add a required field
        let mut slot = SlotDefinition::default();
        slot.required = Some(true);
        new_schema.slots.insert("required_field".to_string(), slot);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should generate schema diff for warning changes: {}");
        let impact = MigrationAnalyzer::analyze_impact(&diff).expect("should analyze impact for warning changes: {}");

        assert_eq!(impact.category, ChangeCategory::Warning);
        assert!(impact.has_warnings());
        assert!(!impact.has_breaking_changes());
        Ok(())
    }
}