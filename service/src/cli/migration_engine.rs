//! Real migration engine implementation for `LinkML` CLI

use linkml_core::types::{PermissibleValue, SchemaDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// Migration analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationAnalysis {
    /// Source schema version
    pub from_version: String,
    /// Target schema version
    pub to_version: String,
    /// Breaking changes detected
    pub breaking_changes: Vec<BreakingChange>,
    /// Non-breaking changes detected
    pub non_breaking_changes: Vec<NonBreakingChange>,
    /// Data migrations required
    pub data_migrations: Vec<DataMigration>,
    /// Risk assessment
    pub risk_level: RiskLevel,
    /// Estimated migration duration
    pub estimated_duration: String,
}

/// Breaking change types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BreakingChange {
    /// A class was removed from the schema
    ClassRemoved {
        /// Name of the removed class
        name: String,
    },
    /// A slot was removed from a class
    SlotRemoved {
        /// Name of the class containing the slot
        class_name: String,
        /// Name of the removed slot
        slot_name: String,
    },
    /// A type was changed in a way that requires data migration
    TypeChanged {
        /// Name of the entity
        entity: String,
        /// Original type
        from_type: String,
        /// New type
        to_type: String,
    },
    /// A required field was added to a class
    RequiredFieldAdded {
        /// Name of the class
        class_name: String,
        /// Name of the new required field
        field_name: String,
    },
    /// Cardinality of a slot was reduced
    CardinalityReduced {
        /// Name of the class
        class_name: String,
        /// Name of the slot
        slot_name: String,
        /// Original cardinality
        from: String,
        /// New cardinality
        to: String,
    },
    /// An enum value was removed
    EnumValueRemoved {
        /// Name of the enum
        enum_name: String,
        /// Removed value
        value: String,
    },
    /// Inheritance hierarchy changed
    InheritanceChanged {
        /// Name of the class
        class_name: String,
        /// Original parent class
        old_parent: String,
        /// New parent class
        new_parent: String,
    },
}

/// Non-breaking change types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NonBreakingChange {
    /// A new class was added to the schema
    ClassAdded {
        /// Name of the new class
        name: String,
    },
    /// A new slot was added to a class
    SlotAdded {
        /// Name of the class
        class_name: String,
        /// Name of the new slot
        slot_name: String,
    },
    /// An optional field was added to a class
    OptionalFieldAdded {
        /// Name of the class
        class_name: String,
        /// Name of the new optional field
        field_name: String,
    },
    /// Description of an entity changed
    DescriptionChanged {
        /// Name of the entity
        entity: String,
    },
    /// An alias was added to an entity
    AliasAdded {
        /// Name of the entity
        entity: String,
        /// New alias
        alias: String,
    },
    /// A new enum value was added
    EnumValueAdded {
        /// Name of the enum
        enum_name: String,
        /// New value
        value: String,
    },
    /// Default value of a slot changed
    DefaultValueChanged {
        /// Name of the class
        class_name: String,
        /// Name of the slot
        slot_name: String,
    },
}

/// Data migration requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigration {
    /// Migration type
    pub migration_type: MigrationType,
    /// Affected entity
    pub entity: String,
    /// `SQL` or transformation script
    pub script: String,
    /// Validation query
    pub validation: String,
}

/// Migration types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationType {
    /// Transform data from one format to another
    Transform,
    /// Rename an entity
    Rename,
    /// Split one entity into multiple
    Split,
    /// Merge multiple entities into one
    Merge,
    /// Delete an entity
    Delete,
    /// Backfill missing data
    Backfill,
}

/// Risk level assessment
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - minimal impact expected
    Low,
    /// Medium risk - some impact expected
    Medium,
    /// High risk - significant impact expected
    High,
    /// Critical risk - major impact expected
    Critical,
}

/// Migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Plan ID
    pub id: String,
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Migration steps
    pub steps: Vec<MigrationStep>,
    /// Rollback steps
    pub rollback_steps: Vec<MigrationStep>,
    /// Pre-conditions
    pub preconditions: Vec<String>,
    /// Post-conditions
    pub postconditions: Vec<String>,
    /// Estimated duration
    pub estimated_duration: String,
    /// Risk level
    pub risk_level: RiskLevel,
    /// Created timestamp
    pub created_at: String,
}

/// Individual migration step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationStep {
    /// Step ID
    pub id: String,
    /// Step description
    pub description: String,
    /// Step type
    pub step_type: StepType,
    /// `SQL` or script to execute
    pub script: String,
    /// Validation query
    pub validation: String,
    /// Dependencies on other steps
    pub depends_on: Vec<String>,
    /// Can be parallelized
    pub parallel: bool,
    /// Estimated duration
    pub estimated_duration: String,
}

/// Step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Schema modification step
    Schema,
    /// Data migration step
    Data,
    /// Index creation/modification step
    Index,
    /// Constraint modification step
    Constraint,
    /// Validation step
    Validation,
    /// Backup creation step
    Backup,
}

/// Migration engine
pub struct MigrationEngine {
    /// Source schema
    from_schema: SchemaDefinition,
    /// Target schema
    to_schema: SchemaDefinition,
    /// Timestamp service for creating timestamps
    timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
}

impl MigrationEngine {
    /// Create a new migration engine
    #[must_use]
    pub fn new(from_schema: SchemaDefinition, to_schema: SchemaDefinition) -> Self {
        let timestamp_service = timestamp_service::wiring::wire_timestamp();
        Self {
            from_schema,
            to_schema,
            timestamp_service: timestamp_service.into_inner(),
        }
    }

    /// Create a new migration engine with custom timestamp service
    #[must_use]
    pub fn with_timestamp_service(
        from_schema: SchemaDefinition,
        to_schema: SchemaDefinition,
        timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            from_schema,
            to_schema,
            timestamp_service,
        }
    }

    /// Analyze schema changes
    ///
    /// # Errors
    ///
    /// Returns an error if schema analysis fails
    pub fn analyze(&self) -> crate::Result<MigrationAnalysis> {
        let mut breaking_changes = Vec::new();
        let mut non_breaking_changes = Vec::new();
        let mut data_migrations = Vec::new();

        // Analyze class changes
        self.analyze_class_changes(&mut breaking_changes, &mut non_breaking_changes)?;

        // Analyze slot changes
        self.analyze_slot_changes(&mut breaking_changes, &mut non_breaking_changes)?;

        // Analyze type changes
        self.analyze_type_changes(&mut breaking_changes, &mut non_breaking_changes)?;

        // Analyze enum changes
        self.analyze_enum_changes(&mut breaking_changes, &mut non_breaking_changes)?;

        // Generate data migrations
        self.generate_data_migrations(&breaking_changes, &mut data_migrations)?;

        // Assess risk level
        let risk_level = self.assess_risk_level(&breaking_changes);

        // Estimate duration
        let estimated_duration =
            self.estimate_duration(&breaking_changes, &non_breaking_changes, &data_migrations);

        Ok(MigrationAnalysis {
            from_version: self
                .from_schema
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            to_version: self
                .to_schema
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            breaking_changes,
            non_breaking_changes,
            data_migrations,
            risk_level,
            estimated_duration,
        })
    }

    /// Analyze class changes
    fn analyze_class_changes(
        &self,
        breaking_changes: &mut Vec<BreakingChange>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) -> crate::Result<()> {
        // Validate schemas have class definitions
        if self.from_schema.classes.is_empty() && self.to_schema.classes.is_empty() {
            return Err(linkml_core::error::LinkMLError::data_validation(
                "Cannot analyze class changes: both schemas have no classes defined".to_string(),
            ));
        }

        let from_classes: HashSet<_> = self.from_schema.classes.keys().cloned().collect();
        let to_classes: HashSet<_> = self.to_schema.classes.keys().cloned().collect();

        // Find removed classes (breaking)
        for removed in from_classes.difference(&to_classes) {
            // Validate that removed class exists in from_schema
            if let Some(removed_class) = self.from_schema.classes.get(removed) {
                // Check if class has dependencies that would make removal dangerous
                if !removed_class.slots.is_empty() {
                    return Err(linkml_core::error::LinkMLError::data_validation(format!(
                        "Cannot remove class '{}': class has {} slots that would be orphaned",
                        removed,
                        removed_class.slots.len()
                    )));
                }
            }
            breaking_changes.push(BreakingChange::ClassRemoved {
                name: removed.clone(),
            });
        }

        // Find added classes (non-breaking)
        for added in to_classes.difference(&from_classes) {
            // Validate that added class is properly defined
            if let Some(added_class) = self.to_schema.classes.get(added) {
                // Check for circular inheritance
                if let Some(parent) = &added_class.is_a
                    && parent == added
                {
                    return Err(linkml_core::error::LinkMLError::data_validation(format!(
                        "Invalid class '{added}': cannot inherit from itself"
                    )));
                }
            }
            non_breaking_changes.push(NonBreakingChange::ClassAdded {
                name: added.clone(),
            });
        }

        // Check for inheritance changes
        for class_name in from_classes.intersection(&to_classes) {
            let from_class = &self.from_schema.classes[class_name];
            let to_class = &self.to_schema.classes[class_name];

            if from_class.is_a != to_class.is_a
                && let (Some(old_parent), Some(new_parent)) = (&from_class.is_a, &to_class.is_a)
                && old_parent != new_parent
            {
                breaking_changes.push(BreakingChange::InheritanceChanged {
                    class_name: class_name.clone(),
                    old_parent: old_parent.clone(),
                    new_parent: new_parent.clone(),
                });
            }
        }

        Ok(())
    }

    /// Analyze slot changes
    fn analyze_slot_changes(
        &self,
        breaking_changes: &mut Vec<BreakingChange>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) -> crate::Result<()> {
        // Check slots in each class
        for (class_name, from_class) in &self.from_schema.classes {
            if let Some(to_class) = self.to_schema.classes.get(class_name) {
                let from_slots: HashSet<_> = from_class.slots.iter().cloned().collect();
                let to_slots: HashSet<_> = to_class.slots.iter().cloned().collect();

                // Find removed slots (breaking)
                for removed in from_slots.difference(&to_slots) {
                    // Validate that removed slot exists in schema definition
                    if let Some(removed_slot) = self.from_schema.slots.get(removed) {
                        // Check if removing a required slot
                        if removed_slot.required.unwrap_or(false) {
                            return Err(linkml_core::error::LinkMLError::data_validation(format!(
                                "Cannot remove required slot '{removed}' from class '{class_name}': would break existing data"
                            )));
                        }
                        // Check if slot has constraints that indicate it's critical
                        if removed_slot.identifier.unwrap_or(false) {
                            return Err(linkml_core::error::LinkMLError::data_validation(format!(
                                "Cannot remove identifier slot '{removed}' from class '{class_name}': would break data integrity"
                            )));
                        }
                    }
                    breaking_changes.push(BreakingChange::SlotRemoved {
                        class_name: class_name.clone(),
                        slot_name: removed.clone(),
                    });
                }

                // Find added slots
                for added in to_slots.difference(&from_slots) {
                    // Validate that added slot is properly defined
                    if let Some(slot_def) = self.to_schema.slots.get(added) {
                        // Check for invalid slot configurations
                        if slot_def.required.unwrap_or(false) && slot_def.range.is_none() {
                            return Err(linkml_core::error::LinkMLError::data_validation(format!(
                                "Invalid required slot '{added}' in class '{class_name}': required slots must have a range"
                            )));
                        }

                        if slot_def.required.unwrap_or(false) {
                            breaking_changes.push(BreakingChange::RequiredFieldAdded {
                                class_name: class_name.clone(),
                                field_name: added.clone(),
                            });
                        } else {
                            non_breaking_changes.push(NonBreakingChange::OptionalFieldAdded {
                                class_name: class_name.clone(),
                                field_name: added.clone(),
                            });
                        }
                    } else {
                        return Err(linkml_core::error::LinkMLError::data_validation(format!(
                            "Added slot '{added}' in class '{class_name}' is not defined in schema slots"
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Analyze type changes
    fn analyze_type_changes(
        &self,
        breaking_changes: &mut Vec<BreakingChange>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) -> crate::Result<()> {
        // Check type changes in slots
        for (slot_name, from_slot) in &self.from_schema.slots {
            if let Some(to_slot) = self.to_schema.slots.get(slot_name) {
                if from_slot.range != to_slot.range
                    && let (Some(from_range), Some(to_range)) = (&from_slot.range, &to_slot.range)
                    && from_range != to_range
                {
                    // Check if type change is compatible (non-breaking) or incompatible (breaking)
                    if self.are_types_compatible(from_range, to_range)? {
                        // Compatible type changes are non-breaking (e.g., integer -> float)
                        non_breaking_changes.push(NonBreakingChange::DescriptionChanged {
                            entity: format!("{slot_name} (type: {from_range} -> {to_range})"),
                        });
                    } else {
                        // Incompatible type changes are breaking and require data migration
                        breaking_changes.push(BreakingChange::TypeChanged {
                            entity: slot_name.clone(),
                            from_type: from_range.clone(),
                            to_type: to_range.clone(),
                        });
                    }
                }

                // Check cardinality changes
                let from_multivalued = from_slot.multivalued.unwrap_or(false);
                let to_multivalued = to_slot.multivalued.unwrap_or(false);

                if from_multivalued && !to_multivalued {
                    // Reducing cardinality from multiple to single is breaking
                    if from_slot.required.unwrap_or(false) {
                        return Err(linkml_core::error::LinkMLError::data_validation(format!(
                            "Cannot reduce cardinality of required slot '{slot_name}' from multiple to single: data loss risk"
                        )));
                    }
                    breaking_changes.push(BreakingChange::CardinalityReduced {
                        class_name: "global".to_string(),
                        slot_name: slot_name.clone(),
                        from: "multiple".to_string(),
                        to: "single".to_string(),
                    });
                } else if !from_multivalued && to_multivalued {
                    // Increasing cardinality from single to multiple is non-breaking
                    non_breaking_changes.push(NonBreakingChange::DescriptionChanged {
                        entity: format!("{slot_name} (cardinality: single -> multiple)"),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check if two types are compatible for migration
    fn are_types_compatible(&self, from_type: &str, to_type: &str) -> crate::Result<bool> {
        // Define type compatibility matrix
        let compatible_conversions = [
            // Numeric widening is generally safe
            ("integer", "float"),
            ("integer", "double"),
            ("float", "double"),
            // String conversions are usually safe
            ("integer", "string"),
            ("float", "string"),
            ("boolean", "string"),
            // URI/IRI conversions
            ("uri", "string"),
            ("iri", "string"),
        ];

        // Same type is always compatible
        if from_type == to_type {
            return Ok(true);
        }

        // Check if conversion is in our safe list
        for (from, to) in &compatible_conversions {
            if from_type == *from && to_type == *to {
                return Ok(true);
            }
        }

        // Check if both types exist in schemas
        let from_exists = self.from_schema.types.contains_key(from_type)
            || self.from_schema.enums.contains_key(from_type)
            || self.is_builtin_type(from_type);
        let to_exists = self.to_schema.types.contains_key(to_type)
            || self.to_schema.enums.contains_key(to_type)
            || self.is_builtin_type(to_type);

        if !from_exists {
            return Err(linkml_core::error::LinkMLError::data_validation(format!(
                "Source type '{from_type}' not found in schema"
            )));
        }
        if !to_exists {
            return Err(linkml_core::error::LinkMLError::data_validation(format!(
                "Target type '{to_type}' not found in schema"
            )));
        }

        // Default to incompatible for safety
        Ok(false)
    }

    /// Check if a type is a built-in `LinkML` type
    fn is_builtin_type(&self, type_name: &str) -> bool {
        matches!(
            type_name,
            "string"
                | "integer"
                | "float"
                | "double"
                | "boolean"
                | "date"
                | "datetime"
                | "time"
                | "uri"
                | "iri"
                | "decimal"
        )
    }

    /// Analyze enum changes
    fn analyze_enum_changes(
        &self,
        breaking_changes: &mut Vec<BreakingChange>,
        non_breaking_changes: &mut Vec<NonBreakingChange>,
    ) -> crate::Result<()> {
        for (enum_name, from_enum) in &self.from_schema.enums {
            if let Some(to_enum) = self.to_schema.enums.get(enum_name) {
                let from_values: HashSet<_> = from_enum
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();
                let to_values: HashSet<_> = to_enum
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();

                // Removed enum values are breaking
                for removed in from_values.difference(&to_values) {
                    breaking_changes.push(BreakingChange::EnumValueRemoved {
                        enum_name: enum_name.clone(),
                        value: removed.clone(),
                    });
                }

                // Added enum values are non-breaking
                for added in to_values.difference(&from_values) {
                    non_breaking_changes.push(NonBreakingChange::EnumValueAdded {
                        enum_name: enum_name.clone(),
                        value: added.clone(),
                    });
                }
            }
        }

        Ok(())
    }

    /// Generate data migrations
    fn generate_data_migrations(
        &self,
        breaking_changes: &[BreakingChange],
        data_migrations: &mut Vec<DataMigration>,
    ) -> crate::Result<()> {
        for change in breaking_changes {
            match change {
                BreakingChange::ClassRemoved { name } => {
                    data_migrations.push(DataMigration {
                        migration_type: MigrationType::Delete,
                        entity: name.clone(),
                        script: format!(
                            "-- Archive data from table {name}
INSERT INTO archive.{name} SELECT * FROM {name};"
                        ),
                        validation: format!("SELECT COUNT(*) FROM archive.{name};"),
                    });
                }
                BreakingChange::TypeChanged {
                    entity,
                    from_type,
                    to_type,
                } => {
                    data_migrations.push(DataMigration {
                        migration_type: MigrationType::Transform,
                        entity: entity.clone(),
                        script: format!(
                            "-- Convert {entity} from {from_type} to {to_type}
ALTER TABLE data MODIFY COLUMN {entity} {to_type};"
                        ),
                        validation: format!(
                            "SELECT COUNT(*) FROM data WHERE {entity} IS NOT NULL;"
                        ),
                    });
                }
                BreakingChange::SlotRemoved {
                    class_name,
                    slot_name,
                } => {
                    data_migrations.push(DataMigration {
                        migration_type: MigrationType::Delete,
                        entity: format!("{class_name}.{slot_name}"),
                        script: format!("-- Remove column {class_name}.{slot_name}
ALTER TABLE {class_name} DROP COLUMN {slot_name};"),
                        validation: format!("SELECT COUNT(*) FROM information_schema.columns WHERE table_name = '{class_name}' AND column_name = '{slot_name}';")});
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Assess risk level
    fn assess_risk_level(&self, breaking_changes: &[BreakingChange]) -> RiskLevel {
        if breaking_changes.is_empty() {
            RiskLevel::Low
        } else if breaking_changes.len() <= 2 {
            RiskLevel::Medium
        } else if breaking_changes.len() <= 5 {
            RiskLevel::High
        } else {
            RiskLevel::Critical
        }
    }

    /// Estimate migration duration
    fn estimate_duration(
        &self,
        breaking_changes: &[BreakingChange],
        non_breaking_changes: &[NonBreakingChange],
        data_migrations: &[DataMigration],
    ) -> String {
        let total_changes =
            breaking_changes.len() + non_breaking_changes.len() + data_migrations.len();

        if total_changes == 0 {
            "< 1 minute".to_string()
        } else if total_changes <= 5 {
            "5-10 minutes".to_string()
        } else if total_changes <= 20 {
            "30-60 minutes".to_string()
        } else {
            "> 2 hours".to_string()
        }
    }

    /// Create migration plan
    ///
    /// # Errors
    ///
    /// Returns an error if migration plan creation fails
    pub async fn create_plan(&self, analysis: &MigrationAnalysis) -> crate::Result<MigrationPlan> {
        let mut steps = Vec::new();
        let mut rollback_steps = Vec::new();
        let mut step_counter = 0;

        // Create backup step
        step_counter += 1;
        steps.push(MigrationStep {
            id: format!("step_{step_counter:03}"),
            description: "Create full backup".to_string(),
            step_type: StepType::Backup,
            script: "CREATE BACKUP OF DATABASE;".to_string(),
            validation: "SELECT backup_status FROM system.backups WHERE id = LAST_INSERT_ID();"
                .to_string(),
            depends_on: vec![],
            parallel: false,
            estimated_duration: "5 minutes".to_string(),
        });

        // Create schema migration steps
        for change in &analysis.breaking_changes {
            step_counter += 1;
            let step = self.create_migration_step(step_counter, change)?;
            rollback_steps.push(self.create_rollback_step(step_counter, change)?);
            steps.push(step);
        }

        // Create data migration steps
        for migration in &analysis.data_migrations {
            step_counter += 1;
            steps.push(MigrationStep {
                id: format!("step_{step_counter:03}"),
                description: format!("Migrate data for {}", migration.entity),
                step_type: StepType::Data,
                script: migration.script.clone(),
                validation: migration.validation.clone(),
                depends_on: vec![],
                parallel: false,
                estimated_duration: "Variable".to_string(),
            });
        }

        // Create validation step
        step_counter += 1;
        steps.push(MigrationStep {
            id: format!("step_{step_counter:03}"),
            description: "Validate migration".to_string(),
            step_type: StepType::Validation,
            script: "CALL validate_migration();".to_string(),
            validation:
                "SELECT validation_status FROM system.validations WHERE id = LAST_INSERT_ID();"
                    .to_string(),
            depends_on: steps.iter().map(|s| s.id.clone()).collect(),
            parallel: false,
            estimated_duration: "2 minutes".to_string(),
        });

        Ok(MigrationPlan {
            id: uuid::Uuid::new_v4().to_string(),
            from_version: analysis.from_version.clone(),
            to_version: analysis.to_version.clone(),
            steps,
            rollback_steps,
            preconditions: vec![
                "Database is accessible".to_string(),
                "Sufficient disk space for backup".to_string(),
                "No active transactions".to_string(),
            ],
            postconditions: vec![
                "All data migrated successfully".to_string(),
                "Schema matches target version".to_string(),
                "No data loss occurred".to_string(),
            ],
            estimated_duration: analysis.estimated_duration.clone(),
            risk_level: analysis.risk_level,
            created_at: self
                .timestamp_service
                .now_utc()
                .await
                .map_err(|e| linkml_core::LinkMLError::service(format!("Timestamp error: {e}")))?
                .to_rfc3339(),
        })
    }

    /// Create migration step for a breaking change
    fn create_migration_step(
        &self,
        id: usize,
        change: &BreakingChange,
    ) -> crate::Result<MigrationStep> {
        let (description, script, validation) = match change {
            BreakingChange::ClassRemoved { name } => (
                format!("Remove class {name}"),
                format!("DROP TABLE IF EXISTS {name};"),
                format!(
                    "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '{name}';"
                ),
            ),
            BreakingChange::SlotRemoved {
                class_name,
                slot_name,
            } => (
                format!("Remove slot {slot_name} from {class_name}"),
                format!("ALTER TABLE {class_name} DROP COLUMN {slot_name};"),
                format!(
                    "SELECT COUNT(*) FROM information_schema.columns WHERE table_name = '{class_name}' AND column_name = '{slot_name}';"
                ),
            ),
            BreakingChange::TypeChanged {
                entity,
                from_type,
                to_type,
            } => (
                format!("Change type of {entity} from {from_type} to {to_type}"),
                format!("ALTER TABLE data MODIFY COLUMN {entity} {to_type};"),
                format!(
                    "SELECT data_type FROM information_schema.columns WHERE column_name = '{entity}';"
                ),
            ),
            _ => (
                "Generic migration step".to_string(),
                "-- Custom migration required".to_string(),
                "SELECT 1;".to_string(),
            ),
        };

        Ok(MigrationStep {
            id: format!("step_{id:03}"),
            description,
            step_type: StepType::Schema,
            script,
            validation,
            depends_on: vec![],
            parallel: false,
            estimated_duration: "1 minute".to_string(),
        })
    }

    /// Create rollback step for a breaking change
    fn create_rollback_step(
        &self,
        id: usize,
        change: &BreakingChange,
    ) -> crate::Result<MigrationStep> {
        let (description, script, validation) = match change {
            BreakingChange::ClassRemoved { name } => (
                format!("Restore class {name}"),
                format!("CREATE TABLE {name} AS SELECT * FROM archive.{name};"),
                format!("SELECT COUNT(*) FROM {name};"),
            ),
            _ => (
                "Rollback migration".to_string(),
                "-- Restore from backup".to_string(),
                "SELECT 1;".to_string(),
            ),
        };

        Ok(MigrationStep {
            id: format!("rollback_{id:03}"),
            description,
            step_type: StepType::Schema,
            script,
            validation,
            depends_on: vec![],
            parallel: false,
            estimated_duration: "1 minute".to_string(),
        })
    }
}
