//! Schema diff implementation for `LinkML`
//!
//! This module provides functionality to compute differences between `LinkML` schemas,
//! including structural differences, semantic changes, and breaking change detection.

use crate::utils::safe_cast::usize_to_f64;
use linkml_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;
use thiserror::Error;

/// Error type for schema diff operations
#[derive(Debug, Error)]
pub enum DiffError {
    /// Schema analysis failed
    #[error("Failed to analyze schema: {0}")]
    AnalysisError(String),

    /// Invalid comparison
    #[error("Invalid comparison: {0}")]
    InvalidComparison(String),
}

/// Result type for diff operations
pub type DiffResult<T> = std::result::Result<T, DiffError>;

/// Type of change detected
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Element was added
    Added,
    /// Element was removed
    Removed,
    /// Element was modified
    Modified,
    /// Element was renamed
    Renamed,
}

/// Severity of a change
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeSeverity {
    /// Backward compatible change
    Compatible,
    /// Minor breaking change
    Minor,
    /// Major breaking change
    Major,
}

impl std::fmt::Display for ChangeSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeSeverity::Compatible => write!(f, "Compatible"),
            ChangeSeverity::Minor => write!(f, "Minor"),
            ChangeSeverity::Major => write!(f, "Major"),
        }
    }
}

/// A single change in the schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    /// Type of change
    pub change_type: ChangeType,

    /// Element type (class, slot, enum, etc.)
    pub element_type: String,

    /// Element name
    pub element_name: String,

    /// Path to the change
    pub path: Vec<String>,

    /// Description of the change
    pub description: String,

    /// Severity of the change
    pub severity: ChangeSeverity,

    /// Old value (if applicable)
    pub old_value: Option<String>,

    /// New value (if applicable)
    pub new_value: Option<String>,

    /// Additional details
    pub details: HashMap<String, String>,
}

/// Result of comparing two schemas
#[derive(Debug, Clone)]
pub struct SchemaDiff {
    /// List of all changes
    pub changes: Vec<SchemaChange>,

    /// Statistics about the diff
    pub stats: DiffStats,

    /// Breaking changes
    pub breaking_changes: Vec<SchemaChange>,

    /// Compatible changes
    pub compatible_changes: Vec<SchemaChange>,
}

/// Statistics about a schema diff
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    /// Total number of changes
    pub total_changes: usize,

    /// Number of additions
    pub additions: usize,

    /// Number of removals
    pub removals: usize,

    /// Number of modifications
    pub modifications: usize,

    /// Number of renames
    pub renames: usize,

    /// Number of breaking changes
    pub breaking_changes: usize,

    /// Number of compatible changes
    pub compatible_changes: usize,
}

/// Schema differ
pub struct SchemaDiffer {
    /// Configuration for the differ
    config: DiffConfig,

    /// Detected renames
    renames: HashMap<String, String>,
}

/// Configuration for schema diffing
#[derive(Debug, Clone)]
pub struct DiffConfig {
    /// Whether to detect renames
    pub detect_renames: bool,

    /// Similarity threshold for rename detection (0.0 - 1.0)
    pub rename_threshold: f64,

    /// Whether to include compatible changes
    pub include_compatible: bool,

    /// Whether to analyze breaking changes
    pub analyze_breaking: bool,

    /// Whether to include detailed descriptions
    pub detailed_descriptions: bool,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            detect_renames: true,
            rename_threshold: 0.8,
            include_compatible: true,
            analyze_breaking: true,
            detailed_descriptions: true,
        }
    }
}

impl SchemaDiffer {
    /// Create a new schema differ
    #[must_use]
    pub fn new(config: DiffConfig) -> Self {
        Self {
            config,
            renames: HashMap::new(),
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(DiffConfig::default())
    }

    /// Compute diff between two schemas
    ///
    /// # Errors
    ///
    /// Returns `DiffError::AnalysisError` if schema analysis fails during comparison.
    /// Returns `DiffError::InvalidComparison` if the schemas cannot be meaningfully compared.
    pub fn diff(
        &mut self,
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
    ) -> DiffResult<SchemaDiff> {
        let mut changes = Vec::new();

        // Clear rename detection
        self.renames.clear();

        // Detect renames if enabled
        if self.config.detect_renames {
            self.detect_renames(old_schema, new_schema)?;
        }

        // Compare metadata
        Self::diff_metadata(old_schema, new_schema, &mut changes);

        // Compare classes
        self.diff_classes(old_schema, new_schema, &mut changes)?;

        // Compare slots
        self.diff_slots(old_schema, new_schema, &mut changes)?;

        // Compare types
        Self::diff_types(old_schema, new_schema, &mut changes)?;

        // Compare enums
        self.diff_enums(old_schema, new_schema, &mut changes)?;

        // Compare subsets
        Self::diff_subsets(old_schema, new_schema, &mut changes)?;

        // Build result
        let stats = Self::compute_stats(&changes);
        let breaking_changes = changes
            .iter()
            .filter(|c| c.severity >= ChangeSeverity::Minor)
            .cloned()
            .collect();
        let compatible_changes = changes
            .iter()
            .filter(|c| c.severity == ChangeSeverity::Compatible)
            .cloned()
            .collect();

        Ok(SchemaDiff {
            changes,
            stats,
            breaking_changes,
            compatible_changes,
        })
    }

    /// Detect renames between schemas
    fn detect_renames(
        &mut self,
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
    ) -> DiffResult<()> {
        // Validate schemas are comparable
        if old_schema.name.is_empty() || new_schema.name.is_empty() {
            return Err(DiffError::InvalidComparison(
                "Cannot detect renames: schemas must have names".to_string(),
            ));
        }

        // Validate schemas have some content to compare
        if old_schema.classes.is_empty()
            && old_schema.slots.is_empty()
            && old_schema.types.is_empty()
        {
            return Err(DiffError::AnalysisError(
                "Old schema is empty - no elements to compare".to_string(),
            ));
        }

        if new_schema.classes.is_empty()
            && new_schema.slots.is_empty()
            && new_schema.types.is_empty()
        {
            return Err(DiffError::AnalysisError(
                "New schema is empty - no elements to compare".to_string(),
            ));
        }

        // Detect class renames
        let old_classes: HashSet<_> = old_schema.classes.keys().cloned().collect();
        let new_classes: HashSet<_> = new_schema.classes.keys().cloned().collect();

        let removed_classes: Vec<_> = old_classes.difference(&new_classes).collect();
        let added_classes: Vec<_> = new_classes.difference(&old_classes).collect();

        for removed in &removed_classes {
            for added in &added_classes {
                if let Some(old_class) = old_schema.classes.get(*removed)
                    && let Some(new_class) = new_schema.classes.get(*added)
                {
                    let similarity = Self::calculate_class_similarity(old_class, new_class);

                    // Validate similarity threshold is reasonable
                    if self.config.rename_threshold < 0.0 || self.config.rename_threshold > 1.0 {
                        return Err(DiffError::InvalidComparison(format!(
                            "Invalid rename threshold: {} (must be between 0.0 and 1.0)",
                            self.config.rename_threshold
                        )));
                    }

                    if similarity >= self.config.rename_threshold {
                        // Validate we're not creating duplicate renames
                        if self.renames.contains_key(*removed) {
                            return Err(DiffError::AnalysisError(format!(
                                "Ambiguous rename detected: class '{removed}' matches multiple targets"
                            )));
                        }
                        self.renames
                            .insert((*removed).to_string(), (*added).to_string());
                    }
                }
            }
        }

        // Similarly for slots, types, and enums
        // (Implementation similar to classes)

        Ok(())
    }

    /// Calculate similarity between two classes
    fn calculate_class_similarity(old_class: &ClassDefinition, new_class: &ClassDefinition) -> f64 {
        let mut score = 0.0;
        let mut max_score = 0.0;

        // Compare attributes
        let old_attrs: HashSet<_> = old_class.attributes.keys().cloned().collect();
        let new_attrs: HashSet<_> = new_class.attributes.keys().cloned().collect();
        let attr_intersection = usize_to_f64(old_attrs.intersection(&new_attrs).count());
        let attr_union = usize_to_f64(old_attrs.union(&new_attrs).count());

        if attr_union > 0.0 {
            score += attr_intersection / attr_union;
            max_score += 1.0;
        }

        // Compare description
        if old_class.description == new_class.description && old_class.description.is_some() {
            score += 0.5;
        }
        max_score += 0.5;

        // Compare parent
        if old_class.is_a == new_class.is_a && old_class.is_a.is_some() {
            score += 0.5;
        }
        max_score += 0.5;

        // Compare mixins
        let old_mixins: HashSet<_> = old_class.mixins.iter().cloned().collect();
        let new_mixins: HashSet<_> = new_class.mixins.iter().cloned().collect();
        let mixin_intersection = usize_to_f64(old_mixins.intersection(&new_mixins).count());
        let mixin_union = usize_to_f64(old_mixins.union(&new_mixins).count());

        if mixin_union > 0.0 {
            score += mixin_intersection / mixin_union;
            max_score += 1.0;
        }

        if max_score > 0.0 {
            score / max_score
        } else {
            0.0
        }
    }

    /// Compare schema metadata
    fn diff_metadata(
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) {
        // Compare version
        if old_schema.version != new_schema.version {
            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "schema".to_string(),
                element_name: old_schema.name.clone(),
                path: vec!["version".to_string()],
                description: "Schema version changed".to_string(),
                severity: ChangeSeverity::Compatible,
                old_value: old_schema.version.clone(),
                new_value: new_schema.version.clone(),
                details: HashMap::new(),
            });
        }

        // Compare license
        if old_schema.license != new_schema.license {
            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "schema".to_string(),
                element_name: old_schema.name.clone(),
                path: vec!["license".to_string()],
                description: "Schema license changed".to_string(),
                severity: ChangeSeverity::Compatible,
                old_value: old_schema.license.clone(),
                new_value: new_schema.license.clone(),
                details: HashMap::new(),
            });
        }

        // Compare imports
        let old_imports: HashSet<_> = old_schema.imports.iter().cloned().collect();
        let new_imports: HashSet<_> = new_schema.imports.iter().cloned().collect();

        for removed in old_imports.difference(&new_imports) {
            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "import".to_string(),
                element_name: removed.clone(),
                path: vec!["imports".to_string()],
                description: format!("Import '{removed}' was removed"),
                severity: ChangeSeverity::Major,
                old_value: Some(removed.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for added in new_imports.difference(&old_imports) {
            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "import".to_string(),
                element_name: added.clone(),
                path: vec!["imports".to_string()],
                description: format!("Import '{added}' was added"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(added.clone()),
                details: HashMap::new(),
            });
        }
    }

    /// Compare classes
    fn diff_classes(
        &self,
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        // Validate schemas have valid structure for comparison
        if old_schema.name.is_empty() {
            return Err(DiffError::InvalidComparison(
                "Old schema must have a name for comparison".to_string(),
            ));
        }
        if new_schema.name.is_empty() {
            return Err(DiffError::InvalidComparison(
                "New schema must have a name for comparison".to_string(),
            ));
        }

        let old_names: HashSet<_> = old_schema.classes.keys().cloned().collect();
        let new_names: HashSet<_> = new_schema.classes.keys().cloned().collect();

        // Validate we have classes to compare
        if old_names.is_empty() && new_names.is_empty() {
            return Err(DiffError::AnalysisError(
                "No classes found in either schema to compare".to_string(),
            ));
        }

        // Removed classes
        for name in old_names.difference(&new_names) {
            if let Some(new_name) = self.renames.get(name) {
                // This is a rename
                changes.push(SchemaChange {
                    change_type: ChangeType::Renamed,
                    element_type: "class".to_string(),
                    element_name: name.clone(),
                    path: vec!["classes".to_string(), name.clone()],
                    description: format!("Class '{name}' was renamed to '{new_name}'"),
                    severity: ChangeSeverity::Major,
                    old_value: Some(name.clone()),
                    new_value: Some(new_name.clone()),
                    details: HashMap::new(),
                });
            } else {
                // Class was removed
                changes.push(SchemaChange {
                    change_type: ChangeType::Removed,
                    element_type: "class".to_string(),
                    element_name: name.clone(),
                    path: vec!["classes".to_string(), name.clone()],
                    description: format!("Class '{name}' was removed"),
                    severity: ChangeSeverity::Major,
                    old_value: Some(name.clone()),
                    new_value: None,
                    details: HashMap::new(),
                });
            }
        }

        // Added classes
        for name in new_names.difference(&old_names) {
            if !self.renames.values().any(|v| v == name) {
                // Class was added (not a rename target)
                changes.push(SchemaChange {
                    change_type: ChangeType::Added,
                    element_type: "class".to_string(),
                    element_name: name.clone(),
                    path: vec!["classes".to_string(), name.clone()],
                    description: format!("Class '{name}' was added"),
                    severity: ChangeSeverity::Compatible,
                    old_value: None,
                    new_value: Some(name.clone()),
                    details: HashMap::new(),
                });
            }
        }

        // Modified classes
        for name in old_names.intersection(&new_names) {
            if let (Some(old_class), Some(new_class)) =
                (old_schema.classes.get(name), new_schema.classes.get(name))
            {
                Self::diff_class_definition(name, old_class, new_class, changes)?;
            }
        }

        Ok(())
    }

    /// Compare two class definitions
    fn diff_class_definition(
        name: &str,
        old_class: &ClassDefinition,
        new_class: &ClassDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let base_path = vec!["classes".to_string(), name.to_string()];

        // Compare parent
        if old_class.is_a != new_class.is_a {
            let severity = if old_class.is_a.is_some() && new_class.is_a.is_some() {
                ChangeSeverity::Major // Changing parent is breaking
            } else if old_class.is_a.is_none() && new_class.is_a.is_some() {
                ChangeSeverity::Compatible // Adding parent is compatible
            } else {
                ChangeSeverity::Major // Removing parent is breaking
            };

            let mut path = base_path.clone();
            path.push("is_a".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "class".to_string(),
                element_name: name.to_string(),
                path,
                description: format!("Parent class changed for '{name}'"),
                severity,
                old_value: old_class.is_a.clone(),
                new_value: new_class.is_a.clone(),
                details: HashMap::new(),
            });
        }

        // Compare attributes
        let old_attrs: HashSet<_> = old_class.attributes.keys().cloned().collect();
        let new_attrs: HashSet<_> = new_class.attributes.keys().cloned().collect();

        for removed in old_attrs.difference(&new_attrs) {
            let mut path = base_path.clone();
            path.push("attributes".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "attribute".to_string(),
                element_name: removed.clone(),
                path,
                description: format!("Attribute '{removed}' removed from class '{name}'"),
                severity: ChangeSeverity::Major,
                old_value: Some(removed.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for added in new_attrs.difference(&old_attrs) {
            let mut path = base_path.clone();
            path.push("attributes".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "attribute".to_string(),
                element_name: added.clone(),
                path,
                description: format!("Attribute '{added}' added to class '{name}'"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(added.clone()),
                details: HashMap::new(),
            });
        }

        // Compare mixins
        let old_mixins: HashSet<_> = old_class.mixins.iter().cloned().collect();
        let new_mixins: HashSet<_> = new_class.mixins.iter().cloned().collect();

        for removed in old_mixins.difference(&new_mixins) {
            let mut path = base_path.clone();
            path.push("mixins".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "mixin".to_string(),
                element_name: removed.clone(),
                path,
                description: format!("Mixin '{removed}' removed from class '{name}'"),
                severity: ChangeSeverity::Minor,
                old_value: Some(removed.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for added in new_mixins.difference(&old_mixins) {
            let mut path = base_path.clone();
            path.push("mixins".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "mixin".to_string(),
                element_name: added.clone(),
                path,
                description: format!("Mixin '{added}' added to class '{name}'"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(added.clone()),
                details: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Compare slots
    fn diff_slots(
        &self,
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let old_names: HashSet<_> = old_schema.slots.keys().cloned().collect();
        let new_names: HashSet<_> = new_schema.slots.keys().cloned().collect();

        // Similar implementation to diff_classes
        for name in old_names.difference(&new_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "slot".to_string(),
                element_name: name.clone(),
                path: vec!["slots".to_string(), name.clone()],
                description: format!("Slot '{name}' was removed"),
                severity: ChangeSeverity::Major,
                old_value: Some(name.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for name in new_names.difference(&old_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "slot".to_string(),
                element_name: name.clone(),
                path: vec!["slots".to_string(), name.clone()],
                description: format!("Slot '{name}' was added"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(name.clone()),
                details: HashMap::new(),
            });
        }

        // Compare modified slots
        for name in old_names.intersection(&new_names) {
            if let (Some(old_slot), Some(new_slot)) =
                (old_schema.slots.get(name), new_schema.slots.get(name))
            {
                Self::diff_slot_definition(name, old_slot, new_slot, changes)?;
            }
        }

        Ok(())
    }

    /// Compare two slot definitions
    fn diff_slot_definition(
        name: &str,
        old_slot: &SlotDefinition,
        new_slot: &SlotDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let base_path = vec!["slots".to_string(), name.to_string()];

        // Compare range
        if old_slot.range != new_slot.range {
            let mut path = base_path.clone();
            path.push("range".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "slot".to_string(),
                element_name: name.to_string(),
                path,
                description: format!("Range changed for slot '{name}'"),
                severity: ChangeSeverity::Major,
                old_value: old_slot.range.clone(),
                new_value: new_slot.range.clone(),
                details: HashMap::new(),
            });
        }

        // Compare required
        if old_slot.required != new_slot.required {
            let mut path = base_path.clone();
            path.push("required".to_string());

            let severity = if old_slot.required == Some(false) && new_slot.required == Some(true) {
                ChangeSeverity::Major // Making optional field required is breaking
            } else {
                ChangeSeverity::Compatible // Making required field optional is compatible
            };

            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "slot".to_string(),
                element_name: name.to_string(),
                path,
                description: format!("Required constraint changed for slot '{name}'"),
                severity,
                old_value: old_slot.required.map(|v| v.to_string()),
                new_value: new_slot.required.map(|v| v.to_string()),
                details: HashMap::new(),
            });
        }

        // Compare multivalued
        if old_slot.multivalued != new_slot.multivalued {
            let mut path = base_path.clone();
            path.push("multivalued".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Modified,
                element_type: "slot".to_string(),
                element_name: name.to_string(),
                path,
                description: format!("Multivalued constraint changed for slot '{name}'"),
                severity: ChangeSeverity::Major,
                old_value: old_slot.multivalued.map(|v| v.to_string()),
                new_value: new_slot.multivalued.map(|v| v.to_string()),
                details: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Compare types
    fn diff_types(
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let old_names: HashSet<_> = old_schema.types.keys().cloned().collect();
        let new_names: HashSet<_> = new_schema.types.keys().cloned().collect();

        for name in old_names.difference(&new_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "type".to_string(),
                element_name: name.clone(),
                path: vec!["types".to_string(), name.clone()],
                description: format!("Type '{name}' was removed"),
                severity: ChangeSeverity::Major,
                old_value: Some(name.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for name in new_names.difference(&old_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "type".to_string(),
                element_name: name.clone(),
                path: vec!["types".to_string(), name.clone()],
                description: format!("Type '{name}' was added"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(name.clone()),
                details: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Compare enums
    fn diff_enums(
        &self,
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let old_names: HashSet<_> = old_schema.enums.keys().cloned().collect();
        let new_names: HashSet<_> = new_schema.enums.keys().cloned().collect();

        for name in old_names.difference(&new_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "enum".to_string(),
                element_name: name.clone(),
                path: vec!["enums".to_string(), name.clone()],
                description: format!("Enum '{name}' was removed"),
                severity: ChangeSeverity::Major,
                old_value: Some(name.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for name in new_names.difference(&old_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "enum".to_string(),
                element_name: name.clone(),
                path: vec!["enums".to_string(), name.clone()],
                description: format!("Enum '{name}' was added"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(name.clone()),
                details: HashMap::new(),
            });
        }

        // Compare modified enums
        for name in old_names.intersection(&new_names) {
            if let (Some(old_enum), Some(new_enum)) =
                (old_schema.enums.get(name), new_schema.enums.get(name))
            {
                Self::diff_enum_definition(name, old_enum, new_enum, changes)?;
            }
        }

        Ok(())
    }

    /// Compare two enum definitions
    fn diff_enum_definition(
        name: &str,
        old_enum: &EnumDefinition,
        new_enum: &EnumDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let base_path = vec!["enums".to_string(), name.to_string()];

        // Compare permissible values
        let old_values: HashSet<String> = old_enum
            .permissible_values
            .iter()
            .map(|pv| match pv {
                PermissibleValue::Simple(s) => s.clone(),
                PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();
        let new_values: HashSet<String> = new_enum
            .permissible_values
            .iter()
            .map(|pv| match pv {
                PermissibleValue::Simple(s) => s.clone(),
                PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();

        for removed in old_values.difference(&new_values) {
            let mut path = base_path.clone();
            path.push("permissible_values".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "enum_value".to_string(),
                element_name: removed.clone(),
                path,
                description: format!("Enum value '{removed}' removed from '{name}'"),
                severity: ChangeSeverity::Major,
                old_value: Some(removed.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for added in new_values.difference(&old_values) {
            let mut path = base_path.clone();
            path.push("permissible_values".to_string());

            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "enum_value".to_string(),
                element_name: added.clone(),
                path,
                description: format!("Enum value '{added}' added to '{name}'"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(added.clone()),
                details: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Compare subsets
    fn diff_subsets(
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        changes: &mut Vec<SchemaChange>,
    ) -> DiffResult<()> {
        let old_names: HashSet<_> = old_schema.subsets.keys().cloned().collect();
        let new_names: HashSet<_> = new_schema.subsets.keys().cloned().collect();

        for name in old_names.difference(&new_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Removed,
                element_type: "subset".to_string(),
                element_name: name.clone(),
                path: vec!["subsets".to_string(), name.clone()],
                description: format!("Subset '{name}' was removed"),
                severity: ChangeSeverity::Minor,
                old_value: Some(name.clone()),
                new_value: None,
                details: HashMap::new(),
            });
        }

        for name in new_names.difference(&old_names) {
            changes.push(SchemaChange {
                change_type: ChangeType::Added,
                element_type: "subset".to_string(),
                element_name: name.clone(),
                path: vec!["subsets".to_string(), name.clone()],
                description: format!("Subset '{name}' was added"),
                severity: ChangeSeverity::Compatible,
                old_value: None,
                new_value: Some(name.clone()),
                details: HashMap::new(),
            });
        }

        Ok(())
    }

    /// Compute statistics from changes
    fn compute_stats(changes: &[SchemaChange]) -> DiffStats {
        let mut stats = DiffStats {
            total_changes: changes.len(),
            ..DiffStats::default()
        };

        for change in changes {
            match change.change_type {
                ChangeType::Added => stats.additions += 1,
                ChangeType::Removed => stats.removals += 1,
                ChangeType::Modified => stats.modifications += 1,
                ChangeType::Renamed => stats.renames += 1,
            }

            match change.severity {
                ChangeSeverity::Compatible => stats.compatible_changes += 1,
                _ => stats.breaking_changes += 1,
            }
        }

        stats
    }
}

impl Default for SchemaDiffer {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl fmt::Display for SchemaDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Schema Diff Summary:")?;
        writeln!(f, "  Total changes: {}", self.stats.total_changes)?;
        writeln!(f, "  Additions: {}", self.stats.additions)?;
        writeln!(f, "  Removals: {}", self.stats.removals)?;
        writeln!(f, "  Modifications: {}", self.stats.modifications)?;
        writeln!(f, "  Renames: {}", self.stats.renames)?;
        writeln!(f, "  Breaking changes: {}", self.stats.breaking_changes)?;
        writeln!(f, "  Compatible changes: {}", self.stats.compatible_changes)?;

        if !self.breaking_changes.is_empty() {
            writeln!(
                f,
                "
Breaking Changes:"
            )?;
            for change in &self.breaking_changes {
                writeln!(f, "  - {}: {}", change.severity, change.description)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema(name: &str) -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            id: name.to_string(),
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.name = "Person".to_string();
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        // Add a slot
        schema.slots.insert(
            "name".to_string(),
            SlotDefinition {
                name: "name".to_string(),
                range: Some("string".to_string()),
                required: Some(true),
                ..Default::default()
            },
        );

        schema
    }

    #[test]
    fn test_no_changes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema("test");
        let mut differ = SchemaDiffer::with_defaults();

        let diff = differ
            .diff(&schema, &schema)
            .expect("diff should succeed: {}");

        assert_eq!(diff.stats.total_changes, 0);
        assert!(diff.changes.is_empty());
        Ok(())
    }

    #[test]
    fn test_class_addition() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = create_test_schema("test");
        let mut new_schema = old_schema.clone();

        let mut employee_class = ClassDefinition::default();
        employee_class.name = "Employee".to_string();
        employee_class.is_a = Some("Person".to_string());
        employee_class.slots = vec!["employee_id".to_string()];
        new_schema
            .classes
            .insert("Employee".to_string(), employee_class);

        let mut differ = SchemaDiffer::with_defaults();
        let diff = differ
            .diff(&old_schema, &new_schema)
            .expect("diff should succeed: {}");

        assert_eq!(diff.stats.additions, 1);
        assert_eq!(diff.stats.compatible_changes, 1);
        assert_eq!(diff.stats.breaking_changes, 0);
        Ok(())
    }

    #[test]
    fn test_class_removal() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut old_schema = create_test_schema("test");
        let new_schema = create_test_schema("test");

        old_schema.classes.insert(
            "Employee".to_string(),
            ClassDefinition {
                name: "Employee".to_string(),
                ..Default::default()
            },
        );

        let mut differ = SchemaDiffer::with_defaults();
        let diff = differ
            .diff(&old_schema, &new_schema)
            .expect("diff should succeed: {}");

        assert_eq!(diff.stats.removals, 1);
        assert_eq!(diff.stats.breaking_changes, 1);
        Ok(())
    }

    #[test]
    fn test_slot_modification() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = create_test_schema("test");
        let mut new_schema = old_schema.clone();

        if let Some(slot) = new_schema.slots.get_mut("name") {
            slot.required = Some(false); // Make optional
        }

        let mut differ = SchemaDiffer::with_defaults();
        let diff = differ
            .diff(&old_schema, &new_schema)
            .expect("diff should succeed: {}");

        assert_eq!(diff.stats.modifications, 1);
        assert_eq!(diff.stats.compatible_changes, 1); // Making required field optional is compatible
        Ok(())
    }

    #[test]
    fn test_breaking_change() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = create_test_schema("test");
        let mut new_schema = old_schema.clone();

        if let Some(slot) = new_schema.slots.get_mut("name") {
            slot.range = Some("integer".to_string()); // Change type
        }

        let mut differ = SchemaDiffer::with_defaults();
        let diff = differ
            .diff(&old_schema, &new_schema)
            .expect("diff should succeed: {}");

        assert_eq!(diff.stats.modifications, 1);
        assert_eq!(diff.stats.breaking_changes, 1);
        assert!(!diff.breaking_changes.is_empty());
        Ok(())
    }
}
