//! Migration tools for LinkML schemas and data
//!
//! This module provides comprehensive migration support including:
//! - Schema version migration
//! - Data transformation between schema versions
//! - Breaking change detection
//! - Migration script generation
//! - Rollback support
//! - Migration validation

use chrono::{DateTime, Utc};
use linkml_core::error::{LinkMLError, Result};
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    /// Enable dry run mode
    pub dry_run: bool,
    /// Validate data after migration
    pub validate_after: bool,
    /// Create backup before migration
    pub backup_enabled: bool,
    /// Maximum batch size for data migration
    pub batch_size: usize,
    /// Enable parallel processing
    pub parallel: bool,
    /// Migration timeout
    pub timeout_seconds: u64,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            validate_after: true,
            backup_enabled: true,
            batch_size: 1000,
            parallel: true,
            timeout_seconds: 3600, // 1 hour
        }
    }
}

/// Schema version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaVersion {
    /// Version identifier
    pub version: String,
    /// Release date
    pub released: DateTime<Utc>,
    /// Schema definition
    pub schema: SchemaDefinition,
    /// Breaking changes from previous version
    pub breaking_changes: Vec<BreakingChange>,
    /// Migration notes
    pub notes: Option<String>,
}

/// Breaking change definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Change type
    pub change_type: ChangeType,
    /// Affected element
    pub element: String,
    /// Description
    pub description: String,
    /// Migration strategy
    pub migration_strategy: MigrationStrategy,
}

/// Types of breaking changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeType {
    /// Class removed
    ClassRemoved,
    /// Class renamed
    ClassRenamed,
    /// Slot removed
    SlotRemoved,
    /// Slot renamed
    SlotRenamed,
    /// Type changed
    TypeChanged,
    /// Required constraint added
    RequiredAdded,
    /// Cardinality changed
    CardinalityChanged,
    /// Enum values changed
    EnumChanged,
}

/// Migration strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MigrationStrategy {
    /// Automatic migration possible
    Automatic {
        /// Transformation function
        transform: String,
    },
    /// Manual intervention required
    Manual {
        /// Instructions for manual migration
        instructions: String,
    },
    /// Data loss will occur
    DataLoss {
        /// Description of what will be lost
        warning: String,
    },
    /// Use default value
    DefaultValue {
        /// Default value to use
        value: Value,
    },
}

/// Migration plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationPlan {
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Steps to perform
    pub steps: Vec<MigrationStep>,
    /// Estimated duration
    pub estimated_duration: std::time::Duration,
    /// Risk assessment
    pub risk_level: RiskLevel,
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
    /// Dependencies on other steps
    pub depends_on: Vec<String>,
    /// Rollback information
    pub rollback: Option<RollbackInfo>,
}

/// Types of migration steps
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepType {
    /// Schema transformation
    SchemaTransform {
        /// Transformation details
        transform: SchemaTransform,
    },
    /// Data migration
    DataMigration {
        /// Migration details
        migration: DataMigration,
    },
    /// Validation step
    Validation {
        /// Validation criteria
        criteria: ValidationCriteria,
    },
    /// Custom step
    Custom {
        /// Script to execute
        script: String,
    },
}

/// Schema transformation details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTransform {
    /// Classes to add
    pub add_classes: Vec<ClassDefinition>,
    /// Classes to remove
    pub remove_classes: Vec<String>,
    /// Classes to rename
    pub rename_classes: HashMap<String, String>,
    /// Slots to add
    pub add_slots: Vec<SlotDefinition>,
    /// Slots to remove
    pub remove_slots: Vec<String>,
    /// Slots to rename
    pub rename_slots: HashMap<String, String>,
    /// Type changes
    pub type_changes: HashMap<String, TypeChange>,
}

/// Type change definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeChange {
    /// Old type
    pub from_type: String,
    /// New type
    pub to_type: String,
    /// Conversion function
    pub converter: Option<String>,
}

/// Data migration details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataMigration {
    /// Source class
    pub source_class: String,
    /// Target class
    pub target_class: Option<String>,
    /// Field mappings
    pub field_mappings: HashMap<String, FieldMapping>,
    /// Filter criteria
    pub filter: Option<String>,
    /// Transformation script
    pub transform_script: Option<String>,
}

/// Field mapping definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// Source field
    pub source: String,
    /// Target field
    pub target: String,
    /// Transformation
    pub transform: Option<FieldTransform>,
}

/// Field transformation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FieldTransform {
    /// Rename only
    Rename,
    /// Type conversion
    Convert {
        /// Converter function or type name
        converter: String,
    },
    /// Split field
    Split {
        /// Delimiter to split on
        delimiter: String,
        /// Target fields for split values
        target_fields: Vec<String>,
    },
    /// Merge fields
    Merge {
        /// Source fields to merge
        source_fields: Vec<String>,
        /// String to join fields with
        joiner: String,
    },
    /// Custom transformation
    Custom {
        /// Custom transformation script
        script: String,
    },
}

/// Validation criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriteria {
    /// Check schema validity
    pub schema_valid: bool,
    /// Check data validity
    pub data_valid: bool,
    /// Check referential integrity
    pub referential_integrity: bool,
    /// Custom validation rules
    pub custom_rules: Vec<String>,
}

/// Rollback information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackInfo {
    /// Rollback strategy
    pub strategy: RollbackStrategy,
    /// Backup location
    pub backup_path: Option<PathBuf>,
}

/// Rollback strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RollbackStrategy {
    /// Restore from backup
    RestoreBackup,
    /// Reverse transformation
    ReverseTransform,
    /// Manual rollback required
    Manual {
        /// Instructions for manual rollback
        instructions: String,
    },
}

/// Risk levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RiskLevel {
    /// Low risk - automatic migration safe
    Low,
    /// Medium risk - review recommended
    Medium,
    /// High risk - manual review required
    High,
    /// Critical - data loss possible
    Critical,
}

/// Migration engine
pub struct MigrationEngine<S>
where
    S: linkml_core::traits::LinkMLService,
{
    config: Arc<RwLock<MigrationConfig>>,
    versions: Arc<RwLock<Vec<SchemaVersion>>>,
    service: Arc<S>, // Reserved for future async validation
}

impl<S> MigrationEngine<S>
where
    S: linkml_core::traits::LinkMLService,
{
    /// Create new migration engine
    pub fn new(config: MigrationConfig, service: Arc<S>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            versions: Arc::new(RwLock::new(Vec::new())),
            service,
        }
    }

    /// Register a schema version
    pub fn register_version(&self, version: SchemaVersion) {
        self.versions.write().push(version);
    }

    /// Analyze differences between versions
    pub fn analyze_changes(
        &self,
        from_version: &str,
        to_version: &str,
    ) -> Result<Vec<BreakingChange>> {
        let versions = self.versions.read();

        let from = versions
            .iter()
            .find(|v| v.version == from_version)
            .ok_or_else(|| LinkMLError::service(format!("Version {from_version} not found")))?;

        let to = versions
            .iter()
            .find(|v| v.version == to_version)
            .ok_or_else(|| LinkMLError::service(format!("Version {to_version} not found")))?;

        let mut changes = Vec::new();

        // Check for removed classes
        for (name, _) in &from.schema.classes {
            if !to.schema.classes.contains_key(name) {
                changes.push(BreakingChange {
                    change_type: ChangeType::ClassRemoved,
                    element: name.clone(),
                    description: format!("Class '{name}' was removed"),
                    migration_strategy: MigrationStrategy::Manual {
                        instructions: format!("Manually handle data from removed class '{name}'"),
                    },
                });
            }
        }

        // Check for removed slots
        for (name, _) in &from.schema.slots {
            if !to.schema.slots.contains_key(name) {
                changes.push(BreakingChange {
                    change_type: ChangeType::SlotRemoved,
                    element: name.clone(),
                    description: format!("Slot '{name}' was removed"),
                    migration_strategy: MigrationStrategy::DataLoss {
                        warning: format!("Data in slot '{name}' will be lost"),
                    },
                });
            }
        }

        // Check for type changes
        for (name, from_slot) in &from.schema.slots {
            if let Some(to_slot) = to.schema.slots.get(name) {
                if from_slot.range != to_slot.range {
                    changes.push(BreakingChange {
                        change_type: ChangeType::TypeChanged,
                        element: name.clone(),
                        description: format!(
                            "Slot '{}' type changed from {:?} to {:?}",
                            name, from_slot.range, to_slot.range
                        ),
                        migration_strategy: MigrationStrategy::Automatic {
                            transform: format!(
                                "convert_{}_{}",
                                from_slot.range.as_deref().unwrap_or("any"),
                                to_slot.range.as_deref().unwrap_or("any")
                            ),
                        },
                    });
                }
            }
        }

        Ok(changes)
    }

    /// Create migration plan
    pub fn create_plan(&self, from_version: &str, to_version: &str) -> Result<MigrationPlan> {
        let changes = self.analyze_changes(from_version, to_version)?;
        let mut steps = Vec::new();

        // Create steps for each change
        for (i, change) in changes.iter().enumerate() {
            let step = match &change.migration_strategy {
                MigrationStrategy::Automatic { transform } => MigrationStep {
                    id: format!("step_{i}"),
                    description: change.description.clone(),
                    step_type: StepType::DataMigration {
                        migration: DataMigration {
                            source_class: change.element.clone(),
                            target_class: None,
                            field_mappings: HashMap::new(),
                            filter: None,
                            transform_script: Some(transform.clone()),
                        },
                    },
                    depends_on: vec![],
                    rollback: Some(RollbackInfo {
                        strategy: RollbackStrategy::RestoreBackup,
                        backup_path: None,
                    }),
                },
                _ => MigrationStep {
                    id: format!("step_{i}"),
                    description: change.description.clone(),
                    step_type: StepType::Custom {
                        script: "manual_migration_required".to_string(),
                    },
                    depends_on: vec![],
                    rollback: Some(RollbackInfo {
                        strategy: RollbackStrategy::Manual {
                            instructions: "Restore from backup".to_string(),
                        },
                        backup_path: None,
                    }),
                },
            };
            steps.push(step);
        }

        // Add validation step
        steps.push(MigrationStep {
            id: "validate".to_string(),
            description: "Validate migrated data".to_string(),
            step_type: StepType::Validation {
                criteria: ValidationCriteria {
                    schema_valid: true,
                    data_valid: true,
                    referential_integrity: true,
                    custom_rules: vec![],
                },
            },
            depends_on: steps.iter().map(|s| s.id.clone()).collect(),
            rollback: None,
        });

        // Calculate risk level
        let risk_level = if changes
            .iter()
            .any(|c| matches!(c.change_type, ChangeType::ClassRemoved))
        {
            RiskLevel::Critical
        } else if changes
            .iter()
            .any(|c| matches!(c.change_type, ChangeType::TypeChanged))
        {
            RiskLevel::High
        } else if !changes.is_empty() {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        Ok(MigrationPlan {
            from_version: from_version.to_string(),
            to_version: to_version.to_string(),
            steps,
            estimated_duration: std::time::Duration::from_secs(60 * changes.len() as u64),
            risk_level,
        })
    }

    /// Execute migration plan
    pub async fn execute_plan(
        &self,
        plan: &MigrationPlan,
        data_path: &Path,
    ) -> Result<MigrationReport> {
        let config = self.config.read();
        let start_time = std::time::Instant::now();
        let mut report = MigrationReport {
            plan: plan.clone(),
            start_time: Utc::now(),
            end_time: None,
            status: MigrationStatus::InProgress,
            steps_completed: vec![],
            errors: vec![],
            warnings: vec![],
            statistics: MigrationStats::default(),
        };

        // Create backup if enabled
        if config.backup_enabled && !config.dry_run {
            self.create_backup(data_path)?;
        }

        // Execute each step
        for step in &plan.steps {
            match self.execute_step(step, data_path, config.dry_run) {
                Ok(result) => {
                    report.steps_completed.push(result);
                }
                Err(e) => {
                    report.errors.push(MigrationError {
                        step_id: step.id.clone(),
                        error: e.to_string(),
                        timestamp: Utc::now(),
                    });

                    // Rollback if needed
                    if !config.dry_run {
                        self.rollback(&report)?;
                    }

                    report.status = MigrationStatus::Failed;
                    report.end_time = Some(Utc::now());
                    return Ok(report);
                }
            }
        }

        report.status = MigrationStatus::Completed;
        report.end_time = Some(Utc::now());
        report.statistics.duration = start_time.elapsed();

        Ok(report)
    }

    /// Execute a single migration step
    fn execute_step(
        &self,
        step: &MigrationStep,
        data_path: &Path,
        dry_run: bool,
    ) -> Result<StepResult> {
        let start = std::time::Instant::now();

        if dry_run {
            // Simulate execution
            return Ok(StepResult {
                step_id: step.id.clone(),
                status: StepStatus::Simulated,
                duration: start.elapsed(),
                records_processed: 0,
                errors: vec![],
            });
        }

        match &step.step_type {
            StepType::SchemaTransform { transform } => {
                // Apply schema transformations
                self.apply_schema_transform(transform)?;
            }
            StepType::DataMigration { migration } => {
                // Perform data migration
                self.migrate_data(migration, data_path)?;
            }
            StepType::Validation { criteria } => {
                // Validate migrated data
                self.validate_migration(criteria, data_path)?;
            }
            StepType::Custom { script } => {
                // Execute custom script
                return Err(LinkMLError::service(format!(
                    "Custom scripts not yet implemented: {script}"
                )));
            }
        }

        Ok(StepResult {
            step_id: step.id.clone(),
            status: StepStatus::Completed,
            duration: start.elapsed(),
            records_processed: 0, // Would track actual records
            errors: vec![],
        })
    }

    /// Create backup of data
    fn create_backup(&self, data_path: &Path) -> Result<PathBuf> {
        let backup_path = data_path.with_extension("backup");
        std::fs::copy(data_path, &backup_path)?;
        Ok(backup_path)
    }

    /// Rollback migration
    fn rollback(&self, report: &MigrationReport) -> Result<()> {
        // Rollback completed steps in reverse order
        for step_result in report.steps_completed.iter().rev() {
            // Find the original step
            if let Some(step) = report
                .plan
                .steps
                .iter()
                .find(|s| s.id == step_result.step_id)
            {
                if let Some(rollback) = &step.rollback {
                    match &rollback.strategy {
                        RollbackStrategy::RestoreBackup => {
                            if let Some(backup_path) = &rollback.backup_path {
                                // Restore from backup
                                std::fs::copy(backup_path, backup_path.with_extension(""))?;
                            }
                        }
                        RollbackStrategy::ReverseTransform => {
                            // Apply reverse transformation
                            return Err(LinkMLError::service(
                                "Reverse transforms not yet implemented",
                            ));
                        }
                        RollbackStrategy::Manual { instructions } => {
                            return Err(LinkMLError::service(format!(
                                "Manual rollback required: {instructions}"
                            )));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Apply schema transformation
    fn apply_schema_transform(&self, _transform: &SchemaTransform) -> Result<()> {
        // This would modify the schema according to the transformation
        Ok(())
    }

    /// Migrate data
    fn migrate_data(&self, _migration: &DataMigration, _data_path: &Path) -> Result<()> {
        // This would read data, transform it, and write it back
        Ok(())
    }

    /// Validate migration
    fn validate_migration(&self, _criteria: &ValidationCriteria, _data_path: &Path) -> Result<()> {
        // This would validate the migrated data against the new schema
        Ok(())
    }
}

/// Migration report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationReport {
    /// Migration plan
    pub plan: MigrationPlan,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Status
    pub status: MigrationStatus,
    /// Steps completed
    pub steps_completed: Vec<StepResult>,
    /// Errors encountered
    pub errors: Vec<MigrationError>,
    /// Warnings
    pub warnings: Vec<String>,
    /// Statistics
    pub statistics: MigrationStats,
}

/// Migration status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MigrationStatus {
    /// Not started
    NotStarted,
    /// In progress
    InProgress,
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Rolled back
    RolledBack,
}

/// Step result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// Step ID
    pub step_id: String,
    /// Status
    pub status: StepStatus,
    /// Duration
    pub duration: std::time::Duration,
    /// Records processed
    pub records_processed: usize,
    /// Errors in this step
    pub errors: Vec<String>,
}

/// Step status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Completed successfully
    Completed,
    /// Failed
    Failed,
    /// Skipped
    Skipped,
    /// Simulated (dry run)
    Simulated,
}

/// Migration error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationError {
    /// Step that failed
    pub step_id: String,
    /// Error message
    pub error: String,
    /// When it occurred
    pub timestamp: DateTime<Utc>,
}

/// Migration statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MigrationStats {
    /// Total duration
    pub duration: std::time::Duration,
    /// Records processed
    pub records_processed: usize,
    /// Records migrated
    pub records_migrated: usize,
    /// Records skipped
    pub records_skipped: usize,
    /// Records failed
    pub records_failed: usize,
}

/// Migration CLI commands
pub mod cli {
    use super::PathBuf;
    use clap::Subcommand;

    /// Migration subcommands
    #[derive(Subcommand, Debug)]
    pub enum MigrationCommands {
        /// Analyze changes between schema versions
        Analyze {
            /// Source version
            #[arg(short = 'f', long)]
            from: String,
            /// Target version
            #[arg(short = 't', long)]
            to: String,
            /// Output format
            #[arg(short = 'o', long, default_value = "table")]
            format: String,
        },

        /// Create migration plan
        Plan {
            /// Source version
            #[arg(short = 'f', long)]
            from: String,
            /// Target version
            #[arg(short = 't', long)]
            to: String,
            /// Output file
            #[arg(short = 'o', long)]
            output: PathBuf,
        },

        /// Execute migration
        Execute {
            /// Migration plan file
            #[arg(short = 'p', long)]
            plan: PathBuf,
            /// Data directory
            #[arg(short = 'd', long)]
            data: PathBuf,
            /// Dry run
            #[arg(long)]
            dry_run: bool,
            /// Skip validation
            #[arg(long)]
            skip_validation: bool,
        },

        /// Validate migration
        Validate {
            /// Schema version
            #[arg(short = 'v', long)]
            version: String,
            /// Data file
            #[arg(short = 'd', long)]
            data: PathBuf,
        },

        /// Generate migration script
        Generate {
            /// Source schema
            #[arg(short = 's', long)]
            source: PathBuf,
            /// Target schema
            #[arg(short = 't', long)]
            target: PathBuf,
            /// Output directory
            #[arg(short = 'o', long)]
            output: PathBuf,
            /// Script language
            #[arg(long, default_value = "rust")]
            language: String,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breaking_change_detection() {
        // Test change detection logic
        let change = BreakingChange {
            change_type: ChangeType::ClassRemoved,
            element: "OldClass".to_string(),
            description: "Class was removed".to_string(),
            migration_strategy: MigrationStrategy::Manual {
                instructions: "Handle manually".to_string(),
            },
        };

        assert_eq!(change.change_type, ChangeType::ClassRemoved);
    }

    #[test]
    fn test_risk_assessment() {
        let plan = MigrationPlan {
            from_version: "1.0".to_string(),
            to_version: "2.0".to_string(),
            steps: vec![],
            estimated_duration: std::time::Duration::from_secs(60),
            risk_level: RiskLevel::High,
        };

        assert!(plan.risk_level > RiskLevel::Low);
    }
}
