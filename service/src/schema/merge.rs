//! Schema merge functionality for `LinkML`
//!
//! This module provides tools to merge multiple schemas into one.

use crate::cli_enhanced::{ConflictResolution, MergeStrategy};
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Options for schema merging
#[derive(Debug, Clone)]
pub struct MergeOptions {
    /// Merge strategy to use
    pub strategy: MergeStrategy,

    /// How to resolve conflicts
    pub conflict_resolution: ConflictResolution,

    /// Base schema for three-way merge
    pub base_schema: Option<SchemaDefinition>,

    /// Preserve annotations during merge
    pub preserve_annotations: bool,

    /// Merge imports
    pub merge_imports: bool,
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self {
            strategy: MergeStrategy::Union,
            conflict_resolution: ConflictResolution::Error,
            base_schema: None,
            preserve_annotations: true,
            merge_imports: true,
        }
    }
}

/// Result of schema merge
#[derive(Debug, Clone)]
pub struct MergeResult {
    /// Merged schema
    pub schema: SchemaDefinition,

    /// Conflicts encountered
    pub conflicts: Vec<MergeConflict>,

    /// Elements merged from each schema
    pub merge_sources: HashMap<String, Vec<String>>,
}

/// Merge conflict information
#[derive(Debug, Clone)]
pub struct MergeConflict {
    /// Type of element (class, slot, type, enum)
    pub element_type: String,

    /// Element name
    pub element_name: String,

    /// Conflicting values from different schemas
    pub conflicting_values: Vec<ConflictValue>,

    /// Resolution applied
    pub resolution: String,
}

/// Conflicting value from a schema
#[derive(Debug, Clone)]
pub struct ConflictValue {
    /// Source schema index
    pub schema_index: usize,

    /// Value
    pub value: serde_json::Value,
}

/// Schema merge engine
pub struct SchemaMerge {
    options: MergeOptions,
}

impl SchemaMerge {
    /// Create new schema merge engine
    #[must_use]
    pub fn new(options: MergeOptions) -> Self {
        Self { options }
    }

    /// Merge multiple schemas
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No schemas are provided
    /// - Conflicts are found and conflict resolution is set to Error
    pub fn merge(&self, schemas: &[SchemaDefinition]) -> Result<SchemaDefinition> {
        if schemas.is_empty() {
            return Err(LinkMLError::config("No schemas to merge"));
        }

        if schemas.len() == 1 {
            return Ok(schemas[0].clone());
        }

        let mut merged = SchemaDefinition::default();
        let mut conflicts = Vec::new();

        // Merge metadata
        self.merge_metadata(schemas, &mut merged)?;

        // Merge based on strategy
        match self.options.strategy {
            MergeStrategy::Union => self.merge_union(schemas, &mut merged, &mut conflicts)?,
            MergeStrategy::Intersection => {
                self.merge_intersection(schemas, &mut merged, &mut conflicts)?;
            }
            MergeStrategy::Override => self.merge_override(schemas, &mut merged, &mut conflicts)?,
            MergeStrategy::Custom => self.merge_custom(schemas, &mut merged, &mut conflicts)?,
        }

        // Handle conflicts
        if !conflicts.is_empty()
            && matches!(self.options.conflict_resolution, ConflictResolution::Error)
        {
            return Err(LinkMLError::schema_validation(format!(
                "{} conflicts found during merge",
                conflicts.len()
            )));
        }

        Ok(merged)
    }

    /// Merge schema metadata
    fn merge_metadata(
        &self,
        schemas: &[SchemaDefinition],
        merged: &mut SchemaDefinition,
    ) -> Result<()> {
        // Use first schema's metadata as base
        if let Some(first) = schemas.first() {
            merged.name.clone_from(&first.name);
            merged.version.clone_from(&first.version);
            merged.description.clone_from(&first.description);
            merged.license.clone_from(&first.license);

            // Merge prefixes
            for schema in schemas {
                for (prefix, prefix_def) in &schema.prefixes {
                    merged.prefixes.insert(prefix.clone(), prefix_def.clone());
                }
            }

            // Merge imports if enabled
            if self.options.merge_imports {
                let mut all_imports = HashSet::new();
                for schema in schemas {
                    all_imports.extend(schema.imports.clone());
                }
                merged.imports = all_imports.into_iter().collect();
            }
        }

        Ok(())
    }

    /// Union merge - include all elements from all schemas
    fn merge_union(
        &self,
        schemas: &[SchemaDefinition],
        merged: &mut SchemaDefinition,
        conflicts: &mut Vec<MergeConflict>,
    ) -> Result<()> {
        // Merge classes
        for (i, schema) in schemas.iter().enumerate() {
            for (name, class) in &schema.classes {
                if let Some(existing) = merged.classes.get(name) {
                    // Conflict detected
                    let conflict = self.create_conflict("class", name, i, existing, class);
                    conflicts.push(conflict);

                    // Resolve conflict
                    let resolved =
                        self.resolve_class_conflict(existing, class, i, schemas.len())?;
                    merged.classes.insert(name.clone(), resolved);
                } else {
                    merged.classes.insert(name.clone(), class.clone());
                }
            }
        }

        // Merge slots
        for (i, schema) in schemas.iter().enumerate() {
            for (name, slot) in &schema.slots {
                if let Some(existing) = merged.slots.get(name) {
                    // Conflict detected
                    let conflict = self.create_conflict("slot", name, i, existing, slot);
                    conflicts.push(conflict);

                    // Resolve conflict
                    let resolved = self.resolve_slot_conflict(existing, slot, i, schemas.len())?;
                    merged.slots.insert(name.clone(), resolved);
                } else {
                    merged.slots.insert(name.clone(), slot.clone());
                }
            }
        }

        // Merge types
        for (i, schema) in schemas.iter().enumerate() {
            for (name, type_def) in &schema.types {
                if let Some(existing) = merged.types.get(name) {
                    // Conflict detected
                    let conflict = self.create_conflict("type", name, i, existing, type_def);
                    conflicts.push(conflict);

                    // Resolve conflict
                    let resolved =
                        self.resolve_type_conflict(existing, type_def, i, schemas.len())?;
                    merged.types.insert(name.clone(), resolved);
                } else {
                    merged.types.insert(name.clone(), type_def.clone());
                }
            }
        }

        // Merge enums
        for schema in schemas {
            for (name, enum_def) in &schema.enums {
                if let Some(existing) = merged.enums.get(name) {
                    // Conflict detected - merge enum values
                    let mut merged_enum = existing.clone();
                    // Add permissible values that don't already exist
                    for pv in &enum_def.permissible_values {
                        let pv_text = match pv {
                            linkml_core::types::PermissibleValue::Simple(s) => s,
                            linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                        };

                        let already_exists =
                            merged_enum.permissible_values.iter().any(|existing_pv| {
                                match existing_pv {
                                    linkml_core::types::PermissibleValue::Simple(s) => s == pv_text,
                                    linkml_core::types::PermissibleValue::Complex {
                                        text, ..
                                    } => text == pv_text,
                                }
                            });

                        if !already_exists {
                            merged_enum.permissible_values.push(pv.clone());
                        }
                    }
                    merged.enums.insert(name.clone(), merged_enum);
                } else {
                    merged.enums.insert(name.clone(), enum_def.clone());
                }
            }
        }

        Ok(())
    }

    /// Intersection merge - include only elements present in all schemas
    fn merge_intersection(
        &self,
        schemas: &[SchemaDefinition],
        merged: &mut SchemaDefinition,
        _conflicts: &mut Vec<MergeConflict>,
    ) -> Result<()> {
        if schemas.is_empty() {
            return Ok(());
        }

        // Find common classes
        let mut common_classes: HashSet<String> = schemas[0].classes.keys().cloned().collect();
        for schema in &schemas[1..] {
            let schema_classes: HashSet<String> = schema.classes.keys().cloned().collect();
            common_classes = common_classes
                .intersection(&schema_classes)
                .cloned()
                .collect();
        }

        // Add common classes
        for class_name in common_classes {
            // Use the definition from the first schema
            if let Some(class_def) = schemas[0].classes.get(&class_name) {
                merged.classes.insert(class_name, class_def.clone());
            }
        }

        // Find common slots
        let mut common_slots: HashSet<String> = schemas[0].slots.keys().cloned().collect();
        for schema in &schemas[1..] {
            let schema_slots: HashSet<String> = schema.slots.keys().cloned().collect();
            common_slots = common_slots.intersection(&schema_slots).cloned().collect();
        }

        // Add common slots
        for slot_name in common_slots {
            if let Some(slot_def) = schemas[0].slots.get(&slot_name) {
                merged.slots.insert(slot_name, slot_def.clone());
            }
        }

        // Find common types
        let mut common_types: HashSet<String> = schemas[0].types.keys().cloned().collect();
        for schema in &schemas[1..] {
            let schema_types: HashSet<String> = schema.types.keys().cloned().collect();
            common_types = common_types.intersection(&schema_types).cloned().collect();
        }

        // Add common types
        for type_name in common_types {
            if let Some(type_def) = schemas[0].types.get(&type_name) {
                merged.types.insert(type_name, type_def.clone());
            }
        }

        // Find common enums
        let mut common_enums: HashSet<String> = schemas[0].enums.keys().cloned().collect();
        for schema in &schemas[1..] {
            let schema_enums: HashSet<String> = schema.enums.keys().cloned().collect();
            common_enums = common_enums.intersection(&schema_enums).cloned().collect();
        }

        // Add common enums
        for enum_name in common_enums {
            if let Some(enum_def) = schemas[0].enums.get(&enum_name) {
                merged.enums.insert(enum_name, enum_def.clone());
            }
        }

        Ok(())
    }

    /// Override merge - later schemas override earlier ones
    fn merge_override(
        &self,
        schemas: &[SchemaDefinition],
        merged: &mut SchemaDefinition,
        _conflicts: &mut Vec<MergeConflict>,
    ) -> Result<()> {
        // Process schemas in order, later ones override
        for schema in schemas {
            // Override classes
            for (name, class) in &schema.classes {
                merged.classes.insert(name.clone(), class.clone());
            }

            // Override slots
            for (name, slot) in &schema.slots {
                merged.slots.insert(name.clone(), slot.clone());
            }

            // Override types
            for (name, type_def) in &schema.types {
                merged.types.insert(name.clone(), type_def.clone());
            }

            // Override enums
            for (name, enum_def) in &schema.enums {
                merged.enums.insert(name.clone(), enum_def.clone());
            }
        }

        Ok(())
    }

    /// Custom merge - use custom rules
    fn merge_custom(
        &self,
        schemas: &[SchemaDefinition],
        merged: &mut SchemaDefinition,
        conflicts: &mut Vec<MergeConflict>,
    ) -> Result<()> {
        // For now, implement as union merge
        // In a real implementation, this would read custom merge rules
        self.merge_union(schemas, merged, conflicts)
    }

    /// Create a merge conflict
    fn create_conflict<T: serde::Serialize>(
        &self,
        element_type: &str,
        name: &str,
        schema_index: usize,
        existing: &T,
        new: &T,
    ) -> MergeConflict {
        MergeConflict {
            element_type: element_type.to_string(),
            element_name: name.to_string(),
            conflicting_values: vec![
                ConflictValue {
                    schema_index: 0,
                    value: serde_json::to_value(existing)
                        .expect("should serialize existing element"),
                },
                ConflictValue {
                    schema_index,
                    value: serde_json::to_value(new).expect("should serialize new element"),
                },
            ],
            resolution: format!("{:?}", self.options.conflict_resolution),
        }
    }

    /// Resolve class conflict
    fn resolve_class_conflict(
        &self,
        existing: &ClassDefinition,
        new: &ClassDefinition,
        schema_index: usize,
        total_schemas: usize,
    ) -> Result<ClassDefinition> {
        match self.options.conflict_resolution {
            ConflictResolution::Error => {
                Err(LinkMLError::schema_validation("Class conflict".to_string()))
            }
            ConflictResolution::First => Ok(existing.clone()),
            ConflictResolution::Last => {
                if schema_index == total_schemas - 1 {
                    Ok(new.clone())
                } else {
                    Ok(existing.clone())
                }
            }
            ConflictResolution::Interactive => {
                // In a real implementation, this would prompt the user
                Ok(existing.clone())
            }
        }
    }

    /// Resolve slot conflict
    fn resolve_slot_conflict(
        &self,
        existing: &SlotDefinition,
        new: &SlotDefinition,
        schema_index: usize,
        total_schemas: usize,
    ) -> Result<SlotDefinition> {
        match self.options.conflict_resolution {
            ConflictResolution::Error => {
                Err(LinkMLError::schema_validation("Slot conflict".to_string()))
            }
            ConflictResolution::First => Ok(existing.clone()),
            ConflictResolution::Last => {
                if schema_index == total_schemas - 1 {
                    Ok(new.clone())
                } else {
                    Ok(existing.clone())
                }
            }
            ConflictResolution::Interactive => {
                // In a real implementation, this would prompt the user
                Ok(existing.clone())
            }
        }
    }

    /// Resolve type conflict
    fn resolve_type_conflict(
        &self,
        existing: &TypeDefinition,
        new: &TypeDefinition,
        schema_index: usize,
        total_schemas: usize,
    ) -> Result<TypeDefinition> {
        match self.options.conflict_resolution {
            ConflictResolution::Error => {
                Err(LinkMLError::schema_validation("Type conflict".to_string()))
            }
            ConflictResolution::First => Ok(existing.clone()),
            ConflictResolution::Last => {
                if schema_index == total_schemas - 1 {
                    Ok(new.clone())
                } else {
                    Ok(existing.clone())
                }
            }
            ConflictResolution::Interactive => {
                // In a real implementation, this would prompt the user
                Ok(existing.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[test]
    fn test_merge_union() -> Result<()> {
        let class1 = ClassDefinition {
            slots: vec!["name".to_string()],
            ..Default::default()
        };

        let mut classes1 = IndexMap::new();
        classes1.insert("Person".to_string(), class1);

        let schema1 = SchemaDefinition {
            name: "Schema1".to_string(),
            classes: classes1,
            ..Default::default()
        };

        let class2 = ClassDefinition {
            slots: vec!["brand".to_string()],
            ..Default::default()
        };

        let mut classes2 = IndexMap::new();
        classes2.insert("Car".to_string(), class2);

        let schema2 = SchemaDefinition {
            name: "Schema2".to_string(),
            classes: classes2,
            ..Default::default()
        };

        let options = MergeOptions {
            strategy: MergeStrategy::Union,
            conflict_resolution: ConflictResolution::First,
            ..Default::default()
        };

        let schema_merger = SchemaMerge::new(options);
        let merged = schema_merger
            .merge(&[schema1, schema2])
            .expect("should merge schemas: {}");

        assert_eq!(merged.classes.len(), 2);
        assert!(merged.classes.contains_key("Person"));
        assert!(merged.classes.contains_key("Car"));
        Ok(())
    }

    #[test]
    fn test_merge_intersection() -> Result<()> {
        let mut schema1 = SchemaDefinition::default();
        let class1 = ClassDefinition::default();
        schema1.classes.insert("Person".to_string(), class1.clone());
        schema1.classes.insert("Car".to_string(), class1.clone());

        let mut schema2 = SchemaDefinition::default();
        schema2.classes.insert("Person".to_string(), class1.clone());
        schema2.classes.insert("Bike".to_string(), class1);

        let options = MergeOptions {
            strategy: MergeStrategy::Intersection,
            ..Default::default()
        };

        let schema_merger = SchemaMerge::new(options);
        let merged = schema_merger
            .merge(&[schema1, schema2])
            .expect("should merge schemas: {}");

        assert_eq!(merged.classes.len(), 1);
        assert!(merged.classes.contains_key("Person"));
        assert!(!merged.classes.contains_key("Car"));
        assert!(!merged.classes.contains_key("Bike"));
        Ok(())
    }
}
