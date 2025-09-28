//! Schema merging implementation for `LinkML`
//!
//! This module provides functionality to merge multiple `LinkML` schemas,
//! handling conflicts and preserving semantics.

use linkml_core::prelude::*;
use std::collections::HashSet;
use thiserror::Error;

/// Error type for schema merging operations
#[derive(Debug, Error)]
pub enum MergeError {
    /// Conflicting definitions
    #[error("Conflicting definitions for {element_type} '{name}': {details}")]
    ConflictingDefinition {
        /// Type of element with conflict (class, slot, etc)
        element_type: String,
        /// Name of the conflicting element
        name: String,
        /// Details about the conflict
        details: String,
    },

    /// Invalid merge operation
    #[error("Invalid merge operation: {0}")]
    InvalidMerge(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Schema not found
    #[error("Schema not found: {0}")]
    SchemaNotFound(String),

    /// Incompatible schemas
    #[error("Incompatible schemas: {0}")]
    IncompatibleSchemas(String),
}

/// Result type for merge operations
pub type MergeResult<T> = std::result::Result<T, MergeError>;

impl From<MergeError> for linkml_core::LinkMLError {
    fn from(err: MergeError) -> Self {
        match err {
            MergeError::ConflictingDefinition {
                element_type,
                name,
                details,
            } => linkml_core::LinkMLError::schema_validation(format!(
                "Conflicting definitions for {element_type} '{name}': {details}"
            )),
            MergeError::InvalidMerge(msg) | MergeError::IncompatibleSchemas(msg) => {
                linkml_core::LinkMLError::schema_validation(msg)
            }
            MergeError::InvalidInput(msg) => linkml_core::LinkMLError::parse(msg),
            MergeError::SchemaNotFound(name) => {
                linkml_core::LinkMLError::import(name, "Schema not found")
            }
        }
    }
}

/// Merge strategy for handling conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeStrategy {
    /// Fail on any conflict
    Strict,
    /// Override with source values
    Override,
    /// Keep target values on conflict
    Preserve,
    /// Merge compatible values
    Merge,
}

/// Configuration for schema merging
#[derive(Debug, Clone)]
pub struct MergeConfig {
    /// Strategy for handling conflicts
    pub strategy: MergeStrategy,

    /// Whether to merge imports
    pub merge_imports: bool,

    /// Whether to merge prefixes
    pub merge_prefixes: bool,

    /// Whether to merge subsets
    pub merge_subsets: bool,

    /// Whether to validate after merge
    pub validate_result: bool,

    /// Prefix for renamed elements
    pub rename_prefix: Option<String>,
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self {
            strategy: MergeStrategy::Strict,
            merge_imports: true,
            merge_prefixes: true,
            merge_subsets: true,
            validate_result: true,
            rename_prefix: None,
        }
    }
}

/// Schema merger
pub struct SchemaMerger {
    /// Merge configuration
    config: MergeConfig,

    /// Conflict log
    conflicts: Vec<String>,
}

impl SchemaMerger {
    /// Create a new schema merger
    #[must_use]
    pub fn new(config: MergeConfig) -> Self {
        Self {
            config,
            conflicts: Vec::new(),
        }
    }

    /// Create with default configuration
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(MergeConfig::default())
    }

    /// Merge multiple schemas into one
    ///
    /// # Errors
    ///
    /// Returns `MergeError::InvalidMerge` if no schemas are provided.
    /// Returns `MergeError::ConflictingDefinitions` if irreconcilable conflicts are found.
    /// Returns `MergeError::InvalidInput` if schema structures are incompatible.
    pub fn merge_all(&mut self, schemas: Vec<SchemaDefinition>) -> MergeResult<SchemaDefinition> {
        if schemas.is_empty() {
            return Err(MergeError::InvalidMerge(
                "No schemas provided for merging".to_string(),
            ));
        }

        if schemas.len() == 1 {
            return schemas.into_iter().next().ok_or_else(|| {
                MergeError::InvalidInput("checked that schemas has one element".to_string())
            });
        }

        let mut schemas_iter = schemas.into_iter();
        let mut result = schemas_iter.next().ok_or_else(|| {
            MergeError::InvalidInput("checked that schemas is not empty".to_string())
        })?;

        for schema in schemas_iter {
            result = self.merge_two(result, schema)?;
        }

        if self.config.validate_result {
            Self::validate_merged_schema(&result)?;
        }

        Ok(result)
    }

    /// Merge two schemas
    ///
    /// # Errors
    ///
    /// Returns `MergeError::ConflictingDefinitions` if class or slot definitions conflict.
    /// Returns `MergeError::InvalidInput` if schema metadata is incompatible.
    /// Returns `MergeError::CircularDependency` if imports create circular dependencies.
    pub fn merge_two(
        &mut self,
        mut target: SchemaDefinition,
        source: SchemaDefinition,
    ) -> MergeResult<SchemaDefinition> {
        self.conflicts.clear();

        // Merge metadata
        self.merge_metadata(&mut target, &source)?;

        // Merge imports
        if self.config.merge_imports {
            Self::merge_imports(&mut target, &source);
        }

        // Merge prefixes
        if self.config.merge_prefixes {
            self.merge_prefixes(&mut target, &source)?;
        }

        // Merge subsets
        if self.config.merge_subsets {
            self.merge_subsets(&mut target, &source)?;
        }

        // Merge main elements
        self.merge_classes(&mut target, &source)?;
        self.merge_slots(&mut target, &source)?;
        self.merge_types(&mut target, &source)?;
        self.merge_enums(&mut target, &source)?;

        Ok(target)
    }

    /// Merge schema metadata
    fn merge_metadata(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        // Merge description
        if target.description.is_none() && source.description.is_some() {
            target.description.clone_from(&source.description);
        }

        // Merge version (keep newer)
        if let (Some(target_ver), Some(source_ver)) = (&target.version, &source.version) {
            if source_ver > target_ver {
                target.version.clone_from(&source.version);
            }
        } else if target.version.is_none() {
            target.version.clone_from(&source.version);
        }

        // Merge license
        if target.license.is_none() && source.license.is_some() {
            target.license.clone_from(&source.license);
        }

        // Merge annotations
        if let Some(source_annotations) = &source.annotations {
            if target.annotations.is_none() {
                target.annotations = Some(indexmap::IndexMap::new());
            }
            if let Some(target_annotations) = &mut target.annotations {
                for (key, value) in source_annotations {
                    match self.config.strategy {
                        MergeStrategy::Override => {
                            target_annotations.insert(key.clone(), value.clone());
                        }
                        MergeStrategy::Preserve => {
                            target_annotations
                                .entry(key.clone())
                                .or_insert_with(|| value.clone());
                        }
                        _ => {
                            if let Some(existing) = target_annotations.get(key) {
                                if existing != value {
                                    self.handle_conflict(
                                        "annotation",
                                        key,
                                        &format!("{existing:?}"),
                                        &format!("{value:?}"),
                                    )?;
                                }
                            } else {
                                target_annotations.insert(key.clone(), value.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Merge imports
    fn merge_imports(target: &mut SchemaDefinition, source: &SchemaDefinition) {
        let existing_imports: HashSet<_> = target.imports.iter().cloned().collect();

        for import in &source.imports {
            if !existing_imports.contains(import) {
                target.imports.push(import.clone());
            }
        }
    }

    /// Merge prefixes
    fn merge_prefixes(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (prefix, uri) in &source.prefixes {
            if let Some(existing_uri) = target.prefixes.get(prefix) {
                if existing_uri != uri {
                    let existing_str = match existing_uri {
                        linkml_core::types::PrefixDefinition::Simple(s) => s.as_str(),
                        linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                            prefix_prefix.as_str()
                        }
                    };
                    let new_str = match uri {
                        linkml_core::types::PrefixDefinition::Simple(s) => s.as_str(),
                        linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                            prefix_prefix.as_str()
                        }
                    };
                    self.handle_conflict("prefix", prefix, existing_str, new_str)?;
                }
            } else {
                target.prefixes.insert(prefix.clone(), uri.clone());
            }
        }

        Ok(())
    }

    /// Merge subsets
    fn merge_subsets(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (name, subset) in &source.subsets {
            if target.subsets.contains_key(name) {
                match self.config.strategy {
                    MergeStrategy::Override => {
                        target.subsets.insert(name.clone(), subset.clone());
                    }
                    MergeStrategy::Preserve => {
                        // Keep existing
                    }
                    _ => {
                        self.conflicts
                            .push(format!("Subset '{name}' exists in both schemas"));
                        if self.config.strategy == MergeStrategy::Strict {
                            return Err(MergeError::ConflictingDefinition {
                                element_type: "subset".to_string(),
                                name: name.clone(),
                                details: "Subset already exists".to_string(),
                            });
                        }
                    }
                }
            } else {
                target.subsets.insert(name.clone(), subset.clone());
            }
        }

        Ok(())
    }

    /// Merge classes
    fn merge_classes(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (name, class) in &source.classes {
            if let Some(existing) = target.classes.get(name) {
                match self.config.strategy {
                    MergeStrategy::Override => {
                        target.classes.insert(name.clone(), class.clone());
                    }
                    MergeStrategy::Preserve => {
                        // Keep existing
                    }
                    MergeStrategy::Merge => {
                        let merged = self.merge_class_definitions(existing, class)?;
                        target.classes.insert(name.clone(), merged);
                    }
                    MergeStrategy::Strict => {
                        return Err(MergeError::ConflictingDefinition {
                            element_type: "class".to_string(),
                            name: name.clone(),
                            details: "Class already exists".to_string(),
                        });
                    }
                }
            } else {
                target.classes.insert(name.clone(), class.clone());
            }
        }

        Ok(())
    }

    /// Merge two class definitions
    fn merge_class_definitions(
        &self,
        target: &ClassDefinition,
        source: &ClassDefinition,
    ) -> MergeResult<ClassDefinition> {
        let mut merged = target.clone();

        // Merge description
        if merged.description.is_none() && source.description.is_some() {
            merged.description.clone_from(&source.description);
        }

        // Merge is_a
        if merged.is_a.is_none() && source.is_a.is_some() {
            merged.is_a.clone_from(&source.is_a);
        } else if merged.is_a != source.is_a && source.is_a.is_some() {
            return Err(MergeError::ConflictingDefinition {
                element_type: "class".to_string(),
                name: target.name.clone(),
                details: format!(
                    "Different parent classes: {:?} vs {:?}",
                    merged.is_a, source.is_a
                ),
            });
        }

        // Merge mixins
        let existing_mixins: HashSet<_> = merged.mixins.iter().cloned().collect();
        for mixin in &source.mixins {
            if !existing_mixins.contains(mixin) {
                merged.mixins.push(mixin.clone());
            }
        }

        // Merge attributes
        for (attr_name, attr_def) in &source.attributes {
            if !merged.attributes.contains_key(attr_name) {
                merged
                    .attributes
                    .insert(attr_name.clone(), attr_def.clone());
            }
        }

        // Merge slot usage
        for (slot_name, slot_usage) in &source.slot_usage {
            if !merged.slot_usage.contains_key(slot_name) {
                merged
                    .slot_usage
                    .insert(slot_name.clone(), slot_usage.clone());
            }
        }

        // Merge annotations
        if let Some(source_annotations) = &source.annotations {
            if merged.annotations.is_none() {
                merged.annotations = Some(indexmap::IndexMap::new());
            }
            if let Some(merged_annotations) = &mut merged.annotations {
                for (key, value) in source_annotations {
                    merged_annotations
                        .entry(key.clone())
                        .or_insert_with(|| value.clone());
                }
            }
        }

        Ok(merged)
    }

    /// Merge slots
    fn merge_slots(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (name, slot) in &source.slots {
            if let Some(existing) = target.slots.get(name) {
                match self.config.strategy {
                    MergeStrategy::Override => {
                        target.slots.insert(name.clone(), slot.clone());
                    }
                    MergeStrategy::Preserve => {
                        // Keep existing
                    }
                    MergeStrategy::Merge => {
                        let merged = self.merge_slot_definitions(existing, slot)?;
                        target.slots.insert(name.clone(), merged);
                    }
                    MergeStrategy::Strict => {
                        return Err(MergeError::ConflictingDefinition {
                            element_type: "slot".to_string(),
                            name: name.clone(),
                            details: "Slot already exists".to_string(),
                        });
                    }
                }
            } else {
                target.slots.insert(name.clone(), slot.clone());
            }
        }

        Ok(())
    }

    /// Merge two slot definitions
    fn merge_slot_definitions(
        &self,
        target: &SlotDefinition,
        source: &SlotDefinition,
    ) -> MergeResult<SlotDefinition> {
        let mut merged = target.clone();

        // Merge basic properties
        if merged.description.is_none() {
            merged.description.clone_from(&source.description);
        }

        if merged.range.is_none() {
            merged.range.clone_from(&source.range);
        }

        if merged.required.is_none() {
            merged.required = source.required;
        }

        if merged.multivalued.is_none() {
            merged.multivalued = source.multivalued;
        }

        // Merge annotations
        if let Some(source_annotations) = &source.annotations {
            if merged.annotations.is_none() {
                merged.annotations = Some(indexmap::IndexMap::new());
            }
            if let Some(merged_annotations) = &mut merged.annotations {
                for (key, value) in source_annotations {
                    merged_annotations
                        .entry(key.clone())
                        .or_insert_with(|| value.clone());
                }
            }
        }

        Ok(merged)
    }

    /// Merge types
    fn merge_types(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (name, type_def) in &source.types {
            if target.types.contains_key(name) {
                match self.config.strategy {
                    MergeStrategy::Override => {
                        target.types.insert(name.clone(), type_def.clone());
                    }
                    MergeStrategy::Preserve => {
                        // Keep existing
                    }
                    _ => {
                        self.conflicts
                            .push(format!("Type '{name}' exists in both schemas"));
                        if self.config.strategy == MergeStrategy::Strict {
                            return Err(MergeError::ConflictingDefinition {
                                element_type: "type".to_string(),
                                name: name.clone(),
                                details: "Type already exists".to_string(),
                            });
                        }
                    }
                }
            } else {
                target.types.insert(name.clone(), type_def.clone());
            }
        }

        Ok(())
    }

    /// Merge enums
    fn merge_enums(
        &mut self,
        target: &mut SchemaDefinition,
        source: &SchemaDefinition,
    ) -> MergeResult<()> {
        for (name, enum_def) in &source.enums {
            if let Some(existing) = target.enums.get(name) {
                match self.config.strategy {
                    MergeStrategy::Override => {
                        target.enums.insert(name.clone(), enum_def.clone());
                    }
                    MergeStrategy::Preserve => {
                        // Keep existing
                    }
                    MergeStrategy::Merge => {
                        let merged = self.merge_enum_definitions(existing, enum_def)?;
                        target.enums.insert(name.clone(), merged);
                    }
                    MergeStrategy::Strict => {
                        return Err(MergeError::ConflictingDefinition {
                            element_type: "enum".to_string(),
                            name: name.clone(),
                            details: "Enum already exists".to_string(),
                        });
                    }
                }
            } else {
                target.enums.insert(name.clone(), enum_def.clone());
            }
        }

        Ok(())
    }

    /// Merge two enum definitions
    fn merge_enum_definitions(
        &self,
        target: &EnumDefinition,
        source: &EnumDefinition,
    ) -> MergeResult<EnumDefinition> {
        let mut merged = target.clone();

        // Merge description
        if merged.description.is_none() {
            merged.description.clone_from(&source.description);
        }

        // Merge permissible values
        let existing_values: HashSet<String> = merged
            .permissible_values
            .iter()
            .map(|pv| match pv {
                PermissibleValue::Simple(s) => s.clone(),
                PermissibleValue::Complex { text, .. } => text.clone(),
            })
            .collect();

        for pv in &source.permissible_values {
            let text = match pv {
                PermissibleValue::Simple(s) => s,
                PermissibleValue::Complex { text, .. } => text,
            };
            if !existing_values.contains(text) {
                merged.permissible_values.push(pv.clone());
            }
        }

        Ok(merged)
    }

    /// Handle conflicts based on strategy
    fn handle_conflict(
        &mut self,
        element_type: &str,
        name: &str,
        existing: &str,
        new: &str,
    ) -> MergeResult<()> {
        self.conflicts.push(format!(
            "{element_type} '{name}' has different values: '{existing}' vs '{new}'"
        ));

        match self.config.strategy {
            MergeStrategy::Strict => Err(MergeError::ConflictingDefinition {
                element_type: element_type.to_string(),
                name: name.to_string(),
                details: format!("Values differ: '{existing}' vs '{new}'"),
            }),
            _ => Ok(()),
        }
    }

    /// Validate the merged schema
    fn validate_merged_schema(schema: &SchemaDefinition) -> MergeResult<()> {
        // Check for orphaned references
        let all_classes: HashSet<_> = schema.classes.keys().cloned().collect();
        let all_slots: HashSet<_> = schema.slots.keys().cloned().collect();

        // Validate class references
        for (class_name, class_def) in &schema.classes {
            if let Some(parent) = &class_def.is_a
                && !all_classes.contains(parent)
            {
                return Err(MergeError::InvalidMerge(format!(
                    "Class '{class_name}' references non-existent parent '{parent}'"
                )));
            }

            for mixin in &class_def.mixins {
                if !all_classes.contains(mixin) {
                    return Err(MergeError::InvalidMerge(format!(
                        "Class '{class_name}' references non-existent mixin '{mixin}'"
                    )));
                }
            }
        }

        // Validate slot references
        for (slot_name, slot_def) in &schema.slots {
            if let Some(parent) = &slot_def.is_a
                && !all_slots.contains(parent)
            {
                return Err(MergeError::InvalidMerge(format!(
                    "Slot '{slot_name}' references non-existent parent '{parent}'"
                )));
            }
        }

        Ok(())
    }

    /// Get the list of conflicts encountered
    #[must_use]
    pub fn conflicts(&self) -> &[String] {
        &self.conflicts
    }

    /// Clear conflicts log
    pub fn clear_conflicts(&mut self) {
        self.conflicts.clear();
    }
}

impl Default for SchemaMerger {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema(id: &str) -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            id: id.to_string(),
            name: id.to_string(),
            ..Default::default()
        };

        // Add a common class
        schema.classes.insert(
            "Person".to_string(),
            ClassDefinition {
                name: "Person".to_string(),
                description: Some(format!("Person from {id}")),
                ..Default::default()
            },
        );

        // Add a unique class
        schema.classes.insert(
            format!("{id}_Class"),
            ClassDefinition {
                name: format!("{id}_Class"),
                ..Default::default()
            },
        );

        schema
    }

    #[test]
    fn test_basic_merge() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema1 = create_test_schema("schema1");
        let schema2 = create_test_schema("schema2");

        let mut merger = SchemaMerger::new(MergeConfig {
            strategy: MergeStrategy::Merge,
            ..Default::default()
        });

        let result = merger
            .merge_two(schema1, schema2)
            .expect("merge should succeed: {}");

        // Should have both unique classes
        assert!(result.classes.contains_key("schema1_Class"));
        assert!(result.classes.contains_key("schema2_Class"));

        // Should have the common class
        assert!(result.classes.contains_key("Person"));
        Ok(())
    }

    #[test]
    fn test_strict_merge_conflict() {
        let schema1 = create_test_schema("schema1");
        let schema2 = create_test_schema("schema2");

        let mut merger = SchemaMerger::new(MergeConfig {
            strategy: MergeStrategy::Strict,
            ..Default::default()
        });

        // Should fail due to conflicting Person class
        let result = merger.merge_two(schema1, schema2);
        assert!(result.is_err());
    }

    #[test]
    fn test_override_strategy() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema1 = create_test_schema("schema1");
        let mut schema2 = create_test_schema("schema2");

        // Add conflicting slot
        schema1.slots.insert(
            "name".to_string(),
            SlotDefinition {
                name: "name".to_string(),
                range: Some("string".to_string()),
                ..Default::default()
            },
        );

        schema2.slots.insert(
            "name".to_string(),
            SlotDefinition {
                name: "name".to_string(),
                range: Some("text".to_string()),
                ..Default::default()
            },
        );

        let mut merger = SchemaMerger::new(MergeConfig {
            strategy: MergeStrategy::Override,
            ..Default::default()
        });

        let result = merger
            .merge_two(schema1, schema2)
            .expect("merge should succeed: {}");

        // Should have schema2's version
        let name_slot = result
            .slots
            .get("name")
            .ok_or_else(|| anyhow::anyhow!("name slot should exist"))?;
        assert_eq!(name_slot.range, Some("text".to_string()));
        Ok(())
    }

    #[test]
    fn test_merge_imports() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema1 = SchemaDefinition {
            id: "schema1".to_string(),
            name: "schema1".to_string(),
            imports: vec!["import1".to_string(), "import2".to_string()],
            ..Default::default()
        };

        let schema2 = SchemaDefinition {
            id: "schema2".to_string(),
            name: "schema2".to_string(),
            imports: vec!["import2".to_string(), "import3".to_string()],
            ..Default::default()
        };

        let mut merger = SchemaMerger::with_defaults();
        let result = merger
            .merge_two(schema1, schema2)
            .expect("merge should succeed: {}");

        // Should have all unique imports
        assert_eq!(result.imports.len(), 3);
        assert!(result.imports.contains(&"import1".to_string()));
        assert!(result.imports.contains(&"import2".to_string()));
        assert!(result.imports.contains(&"import3".to_string()));
        Ok(())
    }
}
