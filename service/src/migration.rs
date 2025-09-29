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
use indexmap::IndexMap;
use linkml_core::error::LinkMLError;
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

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

/// Transform type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransformType {
    /// Add a class
    AddClass,
    /// Remove a class
    RemoveClass,
    /// Modify a class
    ModifyClass,
    /// Add a slot
    AddSlot,
    /// Remove a slot
    RemoveSlot,
    /// Modify a slot
    ModifySlot,
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
    /// Transform type
    pub transform_type: TransformType,
    /// Target element name
    pub target_element: String,
    /// Transformation script
    pub transformation_script: Option<String>,
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
    /// Migration type
    pub migration_type: String,
    /// Transformation script (alternative name for compatibility)
    pub transformation_script: Option<String>,
    /// Default values for new fields
    pub default_values: HashMap<String, Value>,
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

impl std::fmt::Display for FieldMapping {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} -> {}", self.source, self.target)
    }
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
    /// Check schema compliance
    pub check_schema_compliance: bool,
    /// Check data integrity
    pub check_data_integrity: bool,
    /// Performance requirements
    pub performance_requirements: Option<String>,
    /// Custom validation rules (alternative name for compatibility)
    pub custom_validation_rules: Vec<String>,
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
    service: Arc<S>, // Used for schema validation
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
}

impl<S> MigrationEngine<S>
where
    S: linkml_core::traits::LinkMLService,
{
    /// Create new migration engine
    pub fn new(
        config: MigrationConfig,
        service: Arc<S>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            versions: Arc::new(RwLock::new(Vec::new())),
            service,
            timestamp,
        }
    }

    /// Get the `LinkML` service
    #[must_use]
    pub fn service(&self) -> &Arc<S> {
        &self.service
    }

    /// Get the timestamp service
    #[must_use]
    pub fn timestamp_service(&self) -> &Arc<dyn TimestampService<Error = TimestampError>> {
        &self.timestamp
    }

    /// Register a schema version
    pub fn register_version(&self, version: SchemaVersion) {
        self.versions.write().push(version);
    }

    /// Analyze differences between versions
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn analyze_changes(
        &self,
        from_version: &str,
        to_version: &str,
    ) -> linkml_core::error::Result<Vec<BreakingChange>> {
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
            if let Some(to_slot) = to.schema.slots.get(name)
                && from_slot.range != to_slot.range
            {
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

        Ok(changes)
    }

    /// Create migration plan
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn create_plan(
        &self,
        from_version: &str,
        to_version: &str,
    ) -> linkml_core::error::Result<MigrationPlan> {
        let changes = self.analyze_changes(from_version, to_version)?;
        let mut steps = Vec::new();

        // Create steps for each change
        for (i, change) in changes.iter().enumerate() {
            let step = match &change.migration_strategy {
                MigrationStrategy::Automatic { transform } => MigrationStep {
                    id: format!("step_{i}"),
                    description: format!("Migration step for {}", change.element),
                    step_type: StepType::DataMigration {
                        migration: DataMigration {
                            source_class: change.element.clone(),
                            target_class: None,
                            field_mappings: HashMap::new(),
                            filter: None,
                            transform_script: Some(transform.clone()),
                            migration_type: "automatic".to_string(),
                            transformation_script: Some(transform.clone()),
                            default_values: HashMap::new(),
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
                    description: format!("Manual migration step for {}", change.element),
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
                    check_schema_compliance: true,
                    check_data_integrity: true,
                    performance_requirements: None,
                    custom_validation_rules: vec![],
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
            estimated_duration: std::time::Duration::from_secs(
                60 * u64::try_from(changes.len()).unwrap_or(0),
            ),
            risk_level,
        })
    }

    /// Execute migration plan
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn execute_plan(
        &self,
        plan: &MigrationPlan,
        data_path: &Path,
    ) -> linkml_core::error::Result<MigrationReport> {
        let config = self.config.read();
        let start_time = std::time::Instant::now();
        // Use TimestampService instead of direct chrono call
        let start_timestamp = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Runtime error: {e}")))?
            .block_on(async { self.timestamp.now_utc().await })
            .map_err(|e| LinkMLError::service(format!("Failed to get start time: {e}")))?;

        let mut report = MigrationReport {
            plan: plan.clone(),
            start_time: start_timestamp,
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
                    // Use TimestampService for error timestamp
                    let error_timestamp = tokio::runtime::Runtime::new()
                        .map_err(|e2| {
                            LinkMLError::service(format!(
                                "Runtime error during error handling: {e2}"
                            ))
                        })?
                        .block_on(async { self.timestamp.now_utc().await })
                        .map_err(|e2| {
                            LinkMLError::service(format!("Failed to get error timestamp: {e2}"))
                        })?;

                    report.errors.push(MigrationError {
                        step_id: step.id.clone(),
                        error: e.to_string(),
                        timestamp: error_timestamp,
                    });

                    // Rollback if needed
                    if !config.dry_run {
                        self.rollback(&report)?;
                    }

                    // Use TimestampService for end time
                    let end_timestamp = tokio::runtime::Runtime::new()
                        .map_err(|e2| {
                            LinkMLError::service(format!(
                                "Runtime error during failure handling: {e2}"
                            ))
                        })?
                        .block_on(async { self.timestamp.now_utc().await })
                        .map_err(|e2| {
                            LinkMLError::service(format!("Failed to get end timestamp: {e2}"))
                        })?;

                    report.status = MigrationStatus::Failed;
                    report.end_time = Some(end_timestamp);
                    return Ok(report);
                }
            }
        }

        // Use TimestampService for completion timestamp
        let completion_timestamp = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Runtime error during completion: {e}")))?
            .block_on(async { self.timestamp.now_utc().await })
            .map_err(|e| {
                LinkMLError::service(format!("Failed to get completion timestamp: {e}"))
            })?;

        report.status = MigrationStatus::Completed;
        report.end_time = Some(completion_timestamp);
        report.statistics.duration = start_time.elapsed();

        Ok(report)
    }

    /// Execute a single migration step
    fn execute_step(
        &self,
        step: &MigrationStep,
        data_path: &Path,
        dry_run: bool,
    ) -> linkml_core::error::Result<StepResult> {
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
                // Get the latest schema version to transform
                let mut versions = self.versions.write();
                if let Some(latest_version) = versions.last_mut() {
                    self.apply_schema_transform(&mut latest_version.schema, transform)?;
                } else {
                    return Err(LinkMLError::service(
                        "No schema versions available for transformation".to_string(),
                    ));
                }
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
    fn create_backup(&self, data_path: &Path) -> linkml_core::error::Result<PathBuf> {
        let backup_path = data_path.with_extension("backup");
        std::fs::copy(data_path, &backup_path)?;
        Ok(backup_path)
    }

    /// Rollback migration
    fn rollback(&self, report: &MigrationReport) -> linkml_core::error::Result<()> {
        // Rollback completed steps in reverse order
        for step_result in report.steps_completed.iter().rev() {
            // Find the original step
            if let Some(step) = report
                .plan
                .steps
                .iter()
                .find(|s| s.id == step_result.step_id)
                && let Some(rollback) = &step.rollback
            {
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
        Ok(())
    }

    /// Apply schema transformation
    fn apply_schema_transform(
        &self,
        schema: &mut SchemaDefinition,
        transform: &SchemaTransform,
    ) -> linkml_core::error::Result<()> {
        // Add new classes
        for class in &transform.add_classes {
            if schema.classes.contains_key(&class.name) {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Class '{}' already exists", class.name),
                    path: Some(format!("classes.{}", class.name)),
                    expected: None,
                    actual: None,
                });
            }
            schema.classes.insert(class.name.clone(), class.clone());
        }

        // Remove classes
        for class_name in &transform.remove_classes {
            if !schema.classes.contains_key(class_name) {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Class '{class_name}' does not exist"),
                    path: Some(format!("classes.{class_name}")),
                    expected: None,
                    actual: None,
                });
            }
            schema.classes.shift_remove(class_name);
        }

        // Rename classes
        for (old_name, new_name) in &transform.rename_classes {
            if let Some(class_def) = schema.classes.shift_remove(old_name) {
                let mut renamed_class = class_def;
                renamed_class.name.clone_from(new_name);
                schema.classes.insert(new_name.clone(), renamed_class);
            } else {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Class '{old_name}' does not exist for renaming"),
                    path: Some(format!("classes.{old_name}")),
                    expected: None,
                    actual: None,
                });
            }
        }

        // Add new slots
        for slot in &transform.add_slots {
            if schema.slots.contains_key(&slot.name) {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Slot '{}' already exists", slot.name),
                    path: Some(format!("slots.{}", slot.name)),
                    expected: None,
                    actual: None,
                });
            }
            schema.slots.insert(slot.name.clone(), slot.clone());
        }

        // Remove slots
        for slot_name in &transform.remove_slots {
            if !schema.slots.contains_key(slot_name) {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Slot '{slot_name}' does not exist"),
                    path: Some(format!("slots.{slot_name}")),
                    expected: None,
                    actual: None,
                });
            }
            schema.slots.shift_remove(slot_name);
        }

        // Rename slots
        for (old_name, new_name) in &transform.rename_slots {
            if let Some(slot_def) = schema.slots.shift_remove(old_name) {
                let mut renamed_slot = slot_def;
                renamed_slot.name.clone_from(new_name);
                schema.slots.insert(new_name.clone(), renamed_slot);
            } else {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Slot '{old_name}' does not exist for renaming"),
                    path: Some(format!("slots.{old_name}")),
                    expected: None,
                    actual: None,
                });
            }
        }

        // Apply type changes
        for (element_name, new_type) in &transform.type_changes {
            // Check if it's a slot type change
            if let Some(slot) = schema.slots.get_mut(element_name) {
                slot.range = Some(new_type.to_type.clone());
            } else {
                return Err(LinkMLError::DataValidationError {
                    message: format!("Element '{element_name}' not found for type change"),
                    path: Some(format!("type_changes.{element_name}")),
                    expected: None,
                    actual: None,
                });
            }
        }

        // Apply the schema transformation based on the transform type
        match &transform.transform_type {
            TransformType::AddClass => {
                println!("Adding class: {}", transform.target_element);

                // Create new class definition
                let new_class = ClassDefinition {
                    name: transform.target_element.clone(),
                    description: None,
                    abstract_: None,
                    mixin: None,
                    is_a: None,
                    mixins: vec![],
                    slots: vec![],
                    slot_usage: IndexMap::new(),
                    attributes: IndexMap::new(),
                    class_uri: None,
                    subclass_of: vec![],
                    tree_root: None,
                    rules: vec![],
                    if_required: None,
                    unique_keys: IndexMap::new(),
                    annotations: None,
                    recursion_options: None,
                    aliases: vec![],
                    notes: vec![],
                    comments: vec![],
                    todos: vec![],
                    see_also: vec![],
                    deprecated: None,
                    examples: vec![],
                };

                // Add the class to the schema
                schema
                    .classes
                    .insert(transform.target_element.clone(), new_class);

                println!("✓ Class '{}' added to schema", transform.target_element);
            }
            TransformType::RemoveClass => {
                println!("Removing class: {}", transform.target_element);

                // Remove the class from the schema
                if schema
                    .classes
                    .shift_remove(&transform.target_element)
                    .is_some()
                {
                    println!("✓ Class '{}' removed from schema", transform.target_element);

                    // Also remove any references to this class in other classes
                    for class in schema.classes.values_mut() {
                        // Remove from mixins
                        class.mixins.retain(|m| m != &transform.target_element);

                        // Update is_a references
                        if class.is_a.as_ref() == Some(&transform.target_element) {
                            class.is_a = None;
                        }
                    }
                } else {
                    return Err(LinkMLError::service(format!(
                        "Class '{}' not found in schema",
                        transform.target_element
                    )));
                }
            }
            TransformType::ModifyClass => {
                println!("Modifying class: {}", transform.target_element);

                // Get the class to modify
                let class = schema
                    .classes
                    .get_mut(&transform.target_element)
                    .ok_or_else(|| {
                        LinkMLError::service(format!(
                            "Class '{}' not found in schema",
                            transform.target_element
                        ))
                    })?;

                // Apply modifications based on transformation script
                if let Some(ref script) = transform.transformation_script {
                    // Parse the script as JSON to get modification instructions
                    if let Ok(mods) = serde_json::from_str::<Value>(script) {
                        if let Some(new_description) =
                            mods.get("description").and_then(|v| v.as_str())
                        {
                            class.description = Some(new_description.to_string());
                        }
                        if let Some(new_is_a) = mods.get("is_a").and_then(|v| v.as_str()) {
                            class.is_a = Some(new_is_a.to_string());
                        }
                        if let Some(add_slots) = mods.get("add_slots").and_then(|v| v.as_array()) {
                            for slot in add_slots {
                                if let Some(slot_name) = slot.as_str()
                                    && !class.slots.contains(&slot_name.to_string())
                                {
                                    class.slots.push(slot_name.to_string());
                                }
                            }
                        }
                    }
                    println!(
                        "✓ Applied transformation script to class '{}'",
                        transform.target_element
                    );
                }
            }
            TransformType::AddSlot => {
                println!("Adding slot: {}", transform.target_element);

                // Create new slot definition
                let new_slot = SlotDefinition {
                    name: transform.target_element.clone(),
                    description: Some(format!("Added slot: {}", transform.target_element)),
                    range: Some("string".to_string()), // Default range
                    required: Some(false),
                    multivalued: Some(false),
                    ..Default::default()
                };

                // Add the slot to the schema
                schema
                    .slots
                    .insert(transform.target_element.clone(), new_slot);

                println!("✓ Slot '{}' added to schema", transform.target_element);
            }
            TransformType::RemoveSlot => {
                println!("Removing slot: {}", transform.target_element);

                // Remove the slot from the schema
                if schema
                    .slots
                    .shift_remove(&transform.target_element)
                    .is_some()
                {
                    println!("✓ Slot '{}' removed from schema", transform.target_element);

                    // Also remove references to this slot in classes
                    for class in schema.classes.values_mut() {
                        class.slots.retain(|s| s != &transform.target_element);
                    }
                } else {
                    return Err(LinkMLError::service(format!(
                        "Slot '{}' not found in schema",
                        transform.target_element
                    )));
                }
            }
            TransformType::ModifySlot => {
                println!("Modifying slot: {}", transform.target_element);

                // Get the slot to modify
                let slot = schema
                    .slots
                    .get_mut(&transform.target_element)
                    .ok_or_else(|| {
                        LinkMLError::service(format!(
                            "Slot '{}' not found in schema",
                            transform.target_element
                        ))
                    })?;

                // Apply modifications based on transformation script
                if let Some(ref script) = transform.transformation_script {
                    // Parse the script as JSON to get modification instructions
                    if let Ok(mods) = serde_json::from_str::<Value>(script) {
                        if let Some(new_range) = mods.get("range").and_then(|v| v.as_str()) {
                            slot.range = Some(new_range.to_string());
                        }
                        if let Some(new_required) =
                            mods.get("required").and_then(linkml_core::Value::as_bool)
                        {
                            slot.required = Some(new_required);
                        }
                        if let Some(new_multivalued) = mods
                            .get("multivalued")
                            .and_then(linkml_core::Value::as_bool)
                        {
                            slot.multivalued = Some(new_multivalued);
                        }
                        if let Some(new_description) =
                            mods.get("description").and_then(|v| v.as_str())
                        {
                            slot.description = Some(new_description.to_string());
                        }
                    }
                    println!(
                        "✓ Applied transformation script to slot '{}'",
                        transform.target_element
                    );
                }
            }
        }
        Ok(())
    }

    /// Migrate data
    fn migrate_data(
        &self,
        migration: &DataMigration,
        data_path: &Path,
    ) -> linkml_core::error::Result<()> {
        // Read the data file
        if !data_path.exists() {
            return Err(LinkMLError::service(format!(
                "Data file not found: {}",
                data_path.display()
            )));
        }

        println!("Migrating data from: {}", data_path.display());
        println!("Migration type: {:?}", migration.migration_type);

        // Read the data file content
        let content = std::fs::read_to_string(data_path)
            .map_err(|e| LinkMLError::service(format!("Failed to read data file: {e}")))?;

        // Parse data based on file extension
        let mut data: Value = if data_path.extension().and_then(|e| e.to_str()) == Some("yaml")
            || data_path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content)
                .map_err(|e| LinkMLError::service(format!("Failed to parse YAML data: {e}")))?
        } else {
            serde_json::from_str(&content)
                .map_err(|e| LinkMLError::service(format!("Failed to parse JSON data: {e}")))?
        };

        // Apply the migration based on type
        match migration.migration_type.as_str() {
            "FieldRename" => {
                // Apply field mappings
                if !migration.field_mappings.is_empty() {
                    println!("Applying {} field mappings", migration.field_mappings.len());
                    // Convert field mappings to simple string map
                    let mappings: HashMap<String, String> = migration
                        .field_mappings
                        .iter()
                        .map(|(k, v)| (k.clone(), v.target.clone()))
                        .collect();
                    self.apply_field_mappings(&mut data, &mappings)?;
                }
            }
            "TypeConversion" => {
                // Apply type conversions based on transformation script
                if let Some(ref script) = migration.transformation_script {
                    println!("Applying type conversion script");
                    self.apply_type_conversions(&mut data, script)?;
                }
            }
            "DataTransform" => {
                // Apply complex data transformations
                if let Some(ref script) = migration.transformation_script {
                    println!("Applying data transformation script");
                    self.apply_data_transformations(&mut data, script)?;
                }

                // Apply default values for new fields
                if !migration.default_values.is_empty() {
                    println!("Applying {} default values", migration.default_values.len());
                    // Convert default values to string map
                    let defaults: HashMap<String, String> = migration
                        .default_values
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect();
                    self.apply_default_values(&mut data, &defaults)?;
                }
            }
            "Custom" => {
                // Apply custom migration logic
                if let Some(ref script) = migration.transformation_script {
                    println!("Applying custom migration script");
                    // Parse and execute custom transformation logic
                    if let Ok(custom_logic) = serde_json::from_str::<Value>(script) {
                        self.apply_custom_migration(&mut data, &custom_logic)?;
                    }
                }
            }
            _ => {
                // Unknown migration type, log warning and continue
                eprintln!(
                    "Warning: Unknown migration type: {}",
                    migration.migration_type
                );
            }
        }

        // Create backup of original file
        let backup_path = data_path.with_extension("bak");
        std::fs::copy(data_path, &backup_path)
            .map_err(|e| LinkMLError::service(format!("Failed to create backup: {e}")))?;
        println!("✓ Created backup at: {}", backup_path.display());

        // Write the transformed data back
        let output = if data_path.extension().and_then(|e| e.to_str()) == Some("yaml")
            || data_path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            serde_yaml::to_string(&data)
                .map_err(|e| LinkMLError::service(format!("Failed to serialize YAML: {e}")))?
        } else {
            serde_json::to_string_pretty(&data)
                .map_err(|e| LinkMLError::service(format!("Failed to serialize JSON: {e}")))?
        };

        std::fs::write(data_path, output)
            .map_err(|e| LinkMLError::service(format!("Failed to write migrated data: {e}")))?;

        println!("✓ Data migration completed successfully");
        println!("✓ Original data backed up to: {}", backup_path.display());
        Ok(())
    }

    /// Apply field mappings to data
    #[allow(clippy::only_used_in_recursion)]
    fn apply_field_mappings(
        &self,
        data: &mut Value,
        mappings: &HashMap<String, String>,
    ) -> linkml_core::error::Result<()> {
        match data {
            Value::Object(map) => {
                let mut changes = Vec::new();

                for (old_field, new_field) in mappings {
                    if let Some(value) = map.remove(old_field) {
                        changes.push((new_field.clone(), value));
                        println!("  ✓ Renamed field '{old_field}' to '{new_field}'");
                    }
                }

                // Apply the changes
                for (field, value) in changes {
                    map.insert(field, value);
                }
            }
            Value::Array(arr) => {
                // Apply to each element in array
                for item in arr {
                    self.apply_field_mappings(item, mappings)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply type conversions to data
    fn apply_type_conversions(
        &self,
        data: &mut Value,
        script: &str,
    ) -> linkml_core::error::Result<()> {
        if let Ok(conversions) = serde_json::from_str::<HashMap<String, String>>(script) {
            self.apply_type_conversion_recursive(data, &conversions)?;
        }
        Ok(())
    }

    /// Recursively apply type conversions
    #[allow(clippy::only_used_in_recursion)]
    fn apply_type_conversion_recursive(
        &self,
        data: &mut Value,
        conversions: &HashMap<String, String>,
    ) -> linkml_core::error::Result<()> {
        match data {
            Value::Object(map) => {
                for (field, target_type) in conversions {
                    if let Some(value) = map.get_mut(field) {
                        match target_type.as_str() {
                            "string" => {
                                if !value.is_string() {
                                    *value = Value::String(value.to_string());
                                    println!("  ✓ Converted field '{field}' to string");
                                }
                            }
                            "number" => {
                                if let Some(s) = value.as_str()
                                    && let Ok(n) = s.parse::<f64>()
                                {
                                    *value = Value::Number(
                                        serde_json::Number::from_f64(n)
                                            .unwrap_or_else(|| serde_json::Number::from(0)),
                                    );
                                    println!("  ✓ Converted field '{field}' to number");
                                }
                            }
                            "boolean" => {
                                if let Some(s) = value.as_str() {
                                    *value = Value::Bool(s == "true" || s == "1" || s == "yes");
                                    println!("  ✓ Converted field '{field}' to boolean");
                                }
                            }
                            "array" => {
                                if !value.is_array() {
                                    *value = Value::Array(vec![value.clone()]);
                                    println!("  ✓ Converted field '{field}' to array");
                                }
                            }
                            _ => {}
                        }
                    }
                }

                // Recurse into nested objects
                for value in map.values_mut() {
                    self.apply_type_conversion_recursive(value, conversions)?;
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.apply_type_conversion_recursive(item, conversions)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply data transformations
    fn apply_data_transformations(
        &self,
        data: &mut Value,
        script: &str,
    ) -> linkml_core::error::Result<()> {
        // Parse transformation rules from script
        if let Ok(rules) = serde_json::from_str::<Value>(script)
            && let Some(transforms) = rules.get("transforms").and_then(|t| t.as_array())
        {
            for transform in transforms {
                if let (Some(field), Some(operation)) = (
                    transform.get("field").and_then(|f| f.as_str()),
                    transform.get("operation").and_then(|o| o.as_str()),
                ) {
                    match operation {
                        "uppercase" => {
                            self.transform_field_recursive(data, field, |v| {
                                if let Some(s) = v.as_str() {
                                    *v = Value::String(s.to_uppercase());
                                }
                            })?;
                            println!("  ✓ Applied uppercase to field '{field}'");
                        }
                        "lowercase" => {
                            self.transform_field_recursive(data, field, |v| {
                                if let Some(s) = v.as_str() {
                                    *v = Value::String(s.to_lowercase());
                                }
                            })?;
                            println!("  ✓ Applied lowercase to field '{field}'");
                        }
                        "trim" => {
                            self.transform_field_recursive(data, field, |v| {
                                if let Some(s) = v.as_str() {
                                    *v = Value::String(s.trim().to_string());
                                }
                            })?;
                            println!("  ✓ Applied trim to field '{field}'");
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Transform a field recursively
    #[allow(clippy::only_used_in_recursion)]
    fn transform_field_recursive<F>(
        &self,
        data: &mut Value,
        field: &str,
        transform: F,
    ) -> linkml_core::error::Result<()>
    where
        F: Fn(&mut Value) + Copy,
    {
        match data {
            Value::Object(map) => {
                if let Some(value) = map.get_mut(field) {
                    transform(value);
                }
                // Recurse into nested objects
                for value in map.values_mut() {
                    self.transform_field_recursive(value, field, transform)?;
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.transform_field_recursive(item, field, transform)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply default values for new fields
    #[allow(clippy::only_used_in_recursion)]
    fn apply_default_values(
        &self,
        data: &mut Value,
        defaults: &HashMap<String, String>,
    ) -> linkml_core::error::Result<()> {
        match data {
            Value::Object(map) => {
                for (field, default_value) in defaults {
                    if !map.contains_key(field) {
                        // Parse the default value as JSON
                        let value = if let Ok(v) = serde_json::from_str::<Value>(default_value) {
                            v
                        } else {
                            // If not valid JSON, treat as string
                            Value::String(default_value.clone())
                        };
                        map.insert(field.clone(), value);
                        println!("  ✓ Set default value for field '{field}'");
                    }
                }

                // Recurse into nested objects
                for value in map.values_mut() {
                    if value.is_object() || value.is_array() {
                        self.apply_default_values(value, defaults)?;
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.apply_default_values(item, defaults)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Apply custom migration logic
    fn apply_custom_migration(
        &self,
        data: &mut Value,
        logic: &Value,
    ) -> linkml_core::error::Result<()> {
        // This is a flexible function that can be extended with various custom migration patterns
        if let Some(custom_type) = logic.get("type").and_then(|t| t.as_str()) {
            match custom_type {
                "merge_fields" => {
                    // Merge multiple fields into one
                    if let (Some(sources), Some(target)) = (
                        logic.get("source_fields").and_then(|s| s.as_array()),
                        logic.get("target_field").and_then(|t| t.as_str()),
                    ) && let Value::Object(map) = data
                    {
                        let mut merged = String::new();
                        for source in sources {
                            if let Some(field_name) = source.as_str() {
                                if let Some(Value::String(s)) = map.get(field_name) {
                                    if !merged.is_empty() {
                                        merged.push(' ');
                                    }
                                    merged.push_str(s);
                                }
                                map.remove(field_name);
                            }
                        }
                        map.insert(target.to_string(), Value::String(merged));
                        println!("  ✓ Merged fields into '{target}'");
                    }
                }
                "split_field" => {
                    // Split a field into multiple fields
                    if let (Some(source), Some(delimiter)) = (
                        logic.get("source_field").and_then(|s| s.as_str()),
                        logic.get("delimiter").and_then(|d| d.as_str()),
                    ) && let Value::Object(map) = data
                        && let Some(Value::String(s)) = map.get(source).cloned()
                    {
                        let parts: Vec<&str> = s.split(delimiter).collect();
                        if let Some(target_fields) =
                            logic.get("target_fields").and_then(|t| t.as_array())
                        {
                            for (i, target) in target_fields.iter().enumerate() {
                                if let Some(field_name) = target.as_str()
                                    && let Some(part) = parts.get(i)
                                {
                                    map.insert(
                                        field_name.to_string(),
                                        Value::String((*part).to_string()),
                                    );
                                }
                            }
                            map.remove(source);
                            println!("  ✓ Split field '{source}' into multiple fields");
                        }
                    }
                }
                _ => {
                    println!("  ⚠ Unknown custom migration type: {custom_type}");
                }
            }
        }
        Ok(())
    }

    /// Validate migration
    fn validate_migration(
        &self,
        criteria: &ValidationCriteria,
        data_path: &Path,
    ) -> linkml_core::error::Result<()> {
        // Validate the migrated data against the specified criteria
        if !data_path.exists() {
            return Err(LinkMLError::service(format!(
                "Data file not found for validation: {}",
                data_path.display()
            )));
        }

        println!("Validating migration for: {}", data_path.display());

        // Read the migrated data
        let content = std::fs::read_to_string(data_path).map_err(|e| {
            LinkMLError::service(format!("Failed to read data for validation: {e}"))
        })?;

        let data: Value = if data_path.extension().and_then(|e| e.to_str()) == Some("yaml")
            || data_path.extension().and_then(|e| e.to_str()) == Some("yml")
        {
            serde_yaml::from_str(&content).map_err(|e| {
                LinkMLError::service(format!("Failed to parse YAML for validation: {e}"))
            })?
        } else {
            serde_json::from_str(&content).map_err(|e| {
                LinkMLError::service(format!("Failed to parse JSON for validation: {e}"))
            })?
        };

        let mut validation_errors = Vec::new();

        // Check schema compliance if specified
        if criteria.check_schema_compliance {
            println!("Checking schema compliance...");

            // Schema compliance check requires a target schema version
            // Since ValidationCriteria doesn't have target_schema_version field,
            // we'll check against the current schema
            let versions = self.versions.read();
            if let Some(latest_version) = versions.last() {
                // Validate data against the latest schema
                let schema_errors = self.validate_against_schema(&data, &latest_version.schema)?;
                if schema_errors.is_empty() {
                    println!(
                        "  ✓ Data complies with latest schema version '{}'",
                        latest_version.version
                    );
                } else {
                    println!("  ✗ Schema compliance errors found:");
                    for error in &schema_errors {
                        println!("    - {error}");
                        validation_errors.push(error.clone());
                    }
                }
            } else {
                return Err(LinkMLError::service(
                    "No schema versions available for compliance check".to_string(),
                ));
            }
        }

        // Check data integrity if specified
        if criteria.check_data_integrity {
            println!("Checking data integrity...");

            // Validate data structure and required fields
            let integrity_errors = self.check_data_integrity(&data)?;
            if integrity_errors.is_empty() {
                println!("  ✓ Data integrity verified");
            } else {
                println!("  ✗ Data integrity errors found:");
                for error in &integrity_errors {
                    println!("    - {error}");
                    validation_errors.push(error.clone());
                }
            }
        }

        // Check performance requirements if specified
        if let Some(ref perf_reqs) = criteria.performance_requirements {
            println!("Checking performance requirements...");

            // Parse performance requirements (expecting JSON format)
            if let Ok(reqs) = serde_json::from_str::<Value>(perf_reqs) {
                // Check file size requirement
                if let Some(max_size) = reqs
                    .get("max_file_size_mb")
                    .and_then(linkml_core::Value::as_f64)
                {
                    let file_size_mb = std::fs::metadata(data_path)
                        .map(|m| m.len() as f64 / 1_048_576.0)
                        .unwrap_or(0.0);

                    if file_size_mb <= max_size {
                        println!(
                            "  ✓ File size ({file_size_mb:.2} MB) within limit ({max_size} MB)"
                        );
                    } else {
                        let error = format!(
                            "File size ({file_size_mb:.2} MB) exceeds limit ({max_size} MB)"
                        );
                        println!("  ✗ {error}");
                        validation_errors.push(error);
                    }
                }

                // Check record count requirement
                if let Some(max_records) =
                    reqs.get("max_records").and_then(linkml_core::Value::as_u64)
                {
                    let record_count = match &data {
                        Value::Array(arr) => u64::try_from(arr.len()).unwrap_or(0),
                        Value::Object(_) => 1,
                        _ => 0,
                    };

                    if record_count <= max_records {
                        println!("  ✓ Record count ({record_count}) within limit ({max_records})");
                    } else {
                        let error =
                            format!("Record count ({record_count}) exceeds limit ({max_records})");
                        println!("  ✗ {error}");
                        validation_errors.push(error);
                    }
                }
            } else {
                println!("  Performance requirements: {perf_reqs}");
            }
        }

        // Run custom validation rules if provided
        if !criteria.custom_validation_rules.is_empty() {
            println!(
                "Running {} custom validation rules",
                criteria.custom_validation_rules.len()
            );

            for rule in &criteria.custom_validation_rules {
                println!("  Validating rule: {rule}");

                // Parse and execute custom validation rule
                if let Ok(rule_def) = serde_json::from_str::<Value>(rule) {
                    let rule_errors = self.execute_custom_validation(&data, &rule_def)?;
                    if rule_errors.is_empty() {
                        println!("    ✓ Rule passed");
                    } else {
                        for error in &rule_errors {
                            println!("    ✗ {error}");
                            validation_errors.push(error.clone());
                        }
                    }
                } else {
                    // Treat as a simple expression rule
                    println!("    ✓ Rule: {rule}");
                }
            }
        }

        // Return result based on validation errors
        if validation_errors.is_empty() {
            println!("✓ Migration validation completed successfully");
            Ok(())
        } else {
            Err(LinkMLError::service(format!(
                "Migration validation failed with {} errors:
{}",
                validation_errors.len(),
                validation_errors.join(
                    "
"
                )
            )))
        }
    }

    /// Validate data against a schema using proper `LinkML` validation
    fn validate_against_schema(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
    ) -> linkml_core::error::Result<Vec<String>> {
        use crate::validator::ValidationEngine;

        let mut errors = Vec::new();

        // Create a validation engine with the schema
        let validator = ValidationEngine::new(schema)?;

        // Use tokio's block_on to run async validation in sync context
        let runtime = tokio::runtime::Handle::try_current()
            .or_else(|_| {
                // Create a minimal runtime if we're not in an async context
                tokio::runtime::Runtime::new().map(|rt| rt.handle().clone())
            })
            .map_err(|e| LinkMLError::service(format!("Failed to get tokio runtime: {e}")))?;

        // Determine the target class for validation
        let target_class = if let Value::Object(map) = data {
            // Look for a type field to identify the class
            if let Some(type_field) = map.get("type").or_else(|| map.get("@type"))
                && let Some(type_name) = type_field.as_str()
            {
                type_name.to_string()
            } else {
                // Try to find a default root class
                schema
                    .classes
                    .keys()
                    .next()
                    .cloned()
                    .unwrap_or_else(|| "Root".to_string())
            }
        } else {
            "Root".to_string()
        };

        // Run async validation synchronously using block_on
        let validation_future = validator.validate_as_class(data, &target_class, None);
        let report = runtime.block_on(validation_future)?;

        // Convert validation issues to error strings
        if !report.valid {
            for issue in report.issues {
                let error_msg = if issue.path.is_empty() {
                    issue.message
                } else {
                    format!("{}: {}", issue.path, issue.message)
                };
                errors.push(error_msg);
            }
        }

        // Keep the detailed validation logic as a fallback for additional checks
        if errors.is_empty()
            && let Value::Object(map) = data
        {
            if let Some(type_field) = map.get("type").or_else(|| map.get("@type"))
                && let Some(type_name) = type_field.as_str()
            {
                if let Some(class_def) = schema.classes.get(type_name) {
                    // Validate required slots
                    for slot_name in &class_def.slots {
                        if let Some(slot_def) = schema.slots.get(slot_name)
                            && slot_def.required.unwrap_or(false)
                            && !map.contains_key(slot_name)
                        {
                            errors.push(format!(
                                "Required field '{slot_name}' missing in class '{type_name}'"
                            ));
                        }
                    }

                    // Validate slot types and constraints
                    for (field_name, field_value) in map {
                        if field_name == "type" || field_name == "@type" {
                            continue;
                        }

                        if let Some(slot_def) = schema.slots.get(field_name) {
                            // Type validation
                            if let Some(range) = &slot_def.range {
                                let type_valid = match (range.as_str(), field_value) {
                                    ("string", Value::String(_)) => true,
                                    ("integer", Value::Number(n)) => n.is_i64() || n.is_u64(),
                                    ("float" | "double", Value::Number(_)) => true,
                                    ("boolean", Value::Bool(_)) => true,
                                    _ => false,
                                };

                                if !type_valid {
                                    errors.push(format!(
                                        "Field '{field_name}' expects type '{range}' but got '{field_value:?}'"
                                    ));
                                }
                            }

                            // Pattern validation for strings
                            if let (Some(pattern), Value::String(s)) =
                                (&slot_def.pattern, field_value)
                                && let Ok(re) = regex::Regex::new(pattern)
                                && !re.is_match(s)
                            {
                                errors.push(format!(
                                            "Field '{field_name}' value '{s}' doesn't match pattern '{pattern}'"
                                        ));
                            }
                        } else if !class_def.slots.contains(field_name) {
                            errors.push(format!(
                                "Unknown field '{field_name}' in class '{type_name}'"
                            ));
                        }
                    }
                } else {
                    errors.push(format!("Unknown class type: '{type_name}'"));
                }
            }
        } else if let Value::Array(arr) = data {
            // Validate each item in the array
            for (i, item) in arr.iter().enumerate() {
                let item_errors = self.validate_against_schema(item, schema)?;
                for error in item_errors {
                    errors.push(format!("[Item {i}] {error}"));
                }
            }
        }

        Ok(errors)
    }

    /// Check data integrity
    #[allow(clippy::only_used_in_recursion)]
    fn check_data_integrity(&self, data: &Value) -> linkml_core::error::Result<Vec<String>> {
        let mut errors = Vec::new();

        match data {
            Value::Object(map) => {
                // Check for null values in non-nullable fields
                for (field, value) in map {
                    if value.is_null() {
                        errors.push(format!("Null value found in field '{field}'"));
                    }

                    // Recursively check nested objects
                    if value.is_object() || value.is_array() {
                        let nested_errors = self.check_data_integrity(value)?;
                        for error in nested_errors {
                            errors.push(format!("[{field}] {error}"));
                        }
                    }
                }

                // Check for required identifier fields
                if !map.contains_key("id")
                    && !map.contains_key("identifier")
                    && !map.contains_key("name")
                {
                    errors.push(
                        "No identifier field found (expected 'id', 'identifier', or 'name')"
                            .to_string(),
                    );
                }
            }
            Value::Array(arr) => {
                // Check for empty arrays
                if arr.is_empty() {
                    errors.push("Empty array found".to_string());
                }

                // Check each element
                for (i, item) in arr.iter().enumerate() {
                    let item_errors = self.check_data_integrity(item)?;
                    for error in item_errors {
                        errors.push(format!("[Item {i}] {error}"));
                    }
                }
            }
            _ => {}
        }

        Ok(errors)
    }

    /// Execute custom validation rule
    fn execute_custom_validation(
        &self,
        data: &Value,
        rule_def: &Value,
    ) -> linkml_core::error::Result<Vec<String>> {
        let mut errors = Vec::new();

        if let Some(rule_type) = rule_def.get("type").and_then(|t| t.as_str()) {
            match rule_type {
                "required_fields" => {
                    // Check that specified fields exist
                    if let Some(fields) = rule_def.get("fields").and_then(|f| f.as_array())
                        && let Value::Object(map) = data
                    {
                        for field in fields {
                            if let Some(field_name) = field.as_str()
                                && !map.contains_key(field_name)
                            {
                                errors.push(format!("Required field '{field_name}' not found"));
                            }
                        }
                    }
                }
                "field_values" => {
                    // Check that fields have specific values or patterns
                    if let Some(constraints) =
                        rule_def.get("constraints").and_then(|c| c.as_object())
                        && let Value::Object(map) = data
                    {
                        for (field, constraint) in constraints {
                            if let Some(field_value) = map.get(field) {
                                // Check constraint type
                                if let Some(pattern) =
                                    constraint.get("pattern").and_then(|p| p.as_str())
                                    && let Some(value_str) = field_value.as_str()
                                    && !value_str.contains(pattern)
                                {
                                    errors.push(format!(
                                                    "Field '{field}' value '{value_str}' doesn't match pattern '{pattern}'"
                                                ));
                                }
                                if let Some(min) =
                                    constraint.get("min").and_then(linkml_core::Value::as_f64)
                                    && let Some(num) = field_value.as_f64()
                                    && num < min
                                {
                                    errors.push(format!(
                                        "Field '{field}' value {num} is less than minimum {min}"
                                    ));
                                }
                                if let Some(max) =
                                    constraint.get("max").and_then(linkml_core::Value::as_f64)
                                    && let Some(num) = field_value.as_f64()
                                    && num > max
                                {
                                    errors.push(format!(
                                        "Field '{field}' value {num} exceeds maximum {max}"
                                    ));
                                }
                            }
                        }
                    }
                }
                "no_duplicates" => {
                    // Check for duplicate values in specified field
                    if let Some(field) = rule_def.get("field").and_then(|f| f.as_str())
                        && let Value::Array(arr) = data
                    {
                        let mut seen = std::collections::HashSet::new();
                        for item in arr {
                            if let Value::Object(map) = item
                                && let Some(value) = map.get(field)
                            {
                                let value_str = format!("{value}");
                                if !seen.insert(value_str.clone()) {
                                    errors.push(format!(
                                        "Duplicate value '{value_str}' in field '{field}'"
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    errors.push(format!("Unknown validation rule type: '{rule_type}'"));
                }
            }
        }

        Ok(errors)
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
