//! Migration script generation
//!
//! Generates `TypeQL` migration scripts from schema differences.

use std::fmt::Write;
use std::sync::Arc;
use rootreal_core_foundation_timestamp_core::{TimestampError, TimestampService};
use crate::utils::timestamp::SyncTimestampUtils;

use crate::generator::typeql_generator_enhanced::EnhancedTypeQLGenerator;
use crate::generator::traits::CodeFormatter;
use super::{
    DetailedChange, MigrationResult, ChangeImpact
};
use super::diff::SchemaDiff;
use linkml_core::prelude::*;

/// A complete migration script
#[derive(Debug, Clone)]
pub struct MigrationScript {
    /// Forward migration (old to new)
    pub forward_script: String,
    /// Rollback migration (new to old)
    pub rollback_script: String,
    /// Data migration queries
    pub data_migrations: Vec<DataMigration>,
    /// Migration metadata
    pub metadata: MigrationMetadata}

/// Data migration operation
#[derive(Debug, Clone)]
pub struct DataMigration {
    /// Description of the migration
    pub description: String,
    /// `TypeQL` query to perform migration
    pub query: String,
    /// Whether this is idempotent
    pub idempotent: bool,
    /// Estimated affected records
    pub estimated_records: Option<usize>}

/// Migration metadata
#[derive(Debug, Clone)]
pub struct MigrationMetadata {
    /// Source version
    pub from_version: String,
    /// Target version
    pub to_version: String,
    /// Generation timestamp
    pub generated_at: String,
    /// Whether this is a breaking change
    pub is_breaking: bool,
    /// Complexity score
    pub complexity: u8}

impl MigrationScript {
    /// Create a new migration script
    #[must_use] pub fn new(from_version: &str, to_version: &str) -> Self {
        Self::with_timestamp(from_version, to_version, "unknown")
    }

    /// Create a new migration script with timestamp
    #[must_use] pub fn with_timestamp(from_version: &str, to_version: &str, timestamp: &str) -> Self {
        Self {
            forward_script: String::new(),
            rollback_script: String::new(),
            data_migrations: Vec::new(),
            metadata: MigrationMetadata {
                from_version: from_version.to_string(),
                to_version: to_version.to_string(),
                generated_at: timestamp.to_string(),
                is_breaking: false,
                complexity: 0}}
    }

    /// Convert fmt::Error to MigrationError
    fn fmt_error_to_migration_error(e: std::fmt::Error) -> super::MigrationError {
        super::MigrationError::GenerationError(e.to_string())
    }

    /// Get the complete forward migration script
    pub fn forward_script(&self) -> MigrationResult<String> {
        let mut script = String::new();

        // Header
        writeln!(&mut script, "# TypeQL Migration Script").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "# From: {} To: {}", self.metadata.from_version, self.metadata.to_version).map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "# Generated: {}", self.metadata.generated_at).map_err(Self::fmt_error_to_migration_error)?;
        if self.metadata.is_breaking {
            writeln!(&mut script, "# WARNING: This migration contains breaking changes!").map_err(Self::fmt_error_to_migration_error)?;
        }
        writeln!(&mut script).map_err(Self::fmt_error_to_migration_error)?;

        // Schema changes
        writeln!(&mut script, "## Schema Changes").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "{}", self.forward_script).map_err(Self::fmt_error_to_migration_error)?;

        // Data migrations
        if !self.data_migrations.is_empty() {
            writeln!(&mut script, "
## Data Migrations").map_err(Self::fmt_error_to_migration_error)?;
            for (i, migration) in self.data_migrations.iter().enumerate() {
                writeln!(&mut script, "
### Step {}: {}", i + 1, migration.description).map_err(Self::fmt_error_to_migration_error)?;
                if !migration.idempotent {
                    writeln!(&mut script, "# WARNING: Not idempotent - run only once!").map_err(Self::fmt_error_to_migration_error)?;
                }
                writeln!(&mut script, "{}", migration.query).map_err(Self::fmt_error_to_migration_error)?;
            }
        }

        Ok(script)
    }

    /// Get the complete rollback script
    pub fn rollback_script(&self) -> MigrationResult<String> {
        let mut script = String::new();

        // Header
        writeln!(&mut script, "# TypeQL Rollback Script").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "# From: {} To: {}", self.metadata.to_version, self.metadata.from_version).map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "# Generated: {}", self.metadata.generated_at).map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script, "# WARNING: Data loss may occur during rollback!").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(&mut script).map_err(Self::fmt_error_to_migration_error)?;

        writeln!(&mut script, "{}", self.rollback_script).map_err(Self::fmt_error_to_migration_error)?;

        Ok(script)
    }
}

/// Migration script generator
pub struct MigrationGenerator {
    /// `TypeQL` generator for identifier conversion
    generator: EnhancedTypeQLGenerator}

impl MigrationGenerator {
    /// Create a new migration generator
    #[must_use] pub fn new() -> Self {
        Self {
            generator: EnhancedTypeQLGenerator::new()}
    }

    /// Generate migration scripts from a schema diff
    pub fn generate(
        &self,
        diff: &SchemaDiff,
        impact: &ChangeImpact,
        from_version: &str,
        to_version: &str
    ) -> MigrationResult<MigrationScript> {
        let mut migration = MigrationScript::new(from_version, to_version);
        migration.metadata.is_breaking = impact.has_breaking_changes();
        migration.metadata.complexity = impact.complexity_score;

        // Generate forward migration
        self.generate_forward_migration(diff, &mut migration)?;

        // Generate rollback migration
        self.generate_rollback_migration(diff, &mut migration)?;

        // Generate data migrations if needed
        if impact.requires_data_migration {
            self.generate_data_migrations(diff, &mut migration)?;
        }

        Ok(migration)
    }

    /// Generate forward migration script
    fn generate_forward_migration(&self, diff: &SchemaDiff, migration: &mut MigrationScript) -> MigrationResult<()> {
        let script = &mut migration.forward_script;

        // First, handle removals (undefine)
        if !diff.removed_types.is_empty() || !diff.removed_attributes.is_empty() {
            writeln!(script, "# Remove deprecated elements").map_err(Self::fmt_error_to_migration_error)?;
            writeln!(script, "undefine").map_err(Self::fmt_error_to_migration_error)?;

            // Remove types
            for type_change in &diff.removed_types {
                let type_name = self.generator.convert_identifier(&type_change.name);
                writeln!(script, "  {type_name} sub thing;").map_err(Self::fmt_error_to_migration_error)?;
            }

            // Remove attributes
            for attr in &diff.removed_attributes {
                let attr_name = self.generator.convert_identifier(&attr.name);
                writeln!(script, "  {attr_name} sub attribute;").map_err(Self::fmt_error_to_migration_error)?;
            }

            writeln!(script).map_err(Self::fmt_error_to_migration_error)?;
        }

        // Handle modifications that require undefine/define
        self.generate_modifications(diff, script)?;

        // Then, handle additions (define)
        if !diff.added_types.is_empty() || !diff.added_attributes.is_empty() {
            writeln!(script, "# Add new elements").map_err(Self::fmt_error_to_migration_error)?;
            writeln!(script, "define").map_err(Self::fmt_error_to_migration_error)?;

            // Add attributes first
            for attr in &diff.added_attributes {
                if let Some(slot_def) = &attr.new_attr {
                    self.generate_attribute_definition(script, &attr.name, slot_def)?;
                }
            }

            // Add types
            for type_change in &diff.added_types {
                if let Some(class_def) = &type_change.new_type {
                    self.generate_type_definition(script, &type_change.name, class_def)?;
                }
            }

            writeln!(script).map_err(Self::fmt_error_to_migration_error)?;
        }

        Ok(())
    }

    /// Generate modifications that require undefine/define
    fn generate_modifications(&self, diff: &SchemaDiff, script: &mut String) -> MigrationResult<()> {
        let mut has_modifications = false;

        // Check for type modifications that require redefinition
        for type_change in &diff.modified_types {
            for change in &type_change.changes {
                match change {
                    DetailedChange::RemovedInheritance(_) |
                    DetailedChange::AbstractChanged(_, _) => {
                        has_modifications = true;
                        break;
                    }
                    _ => {}
                }
            }
        }

        // Check for attribute modifications
        if !diff.modified_attributes.is_empty() {
            has_modifications = true;
        }

        if has_modifications {
            writeln!(script, "# Modify existing elements").map_err(Self::fmt_error_to_migration_error)?;

            // First undefine
            writeln!(script, "undefine").map_err(Self::fmt_error_to_migration_error)?;
            for type_change in &diff.modified_types {
                if Self::needs_redefinition(&type_change.changes) {
                    let type_name = self.generator.convert_identifier(&type_change.name);
                    writeln!(script, "  {type_name} sub thing;").map_err(Self::fmt_error_to_migration_error)?;
                }
            }

            for attr in &diff.modified_attributes {
                let attr_name = self.generator.convert_identifier(&attr.name);
                writeln!(script, "  {attr_name} sub attribute;").map_err(Self::fmt_error_to_migration_error)?;
            }

            // Then redefine
            writeln!(script, "
define").map_err(Self::fmt_error_to_migration_error)?;
            for type_change in &diff.modified_types {
                if Self::needs_redefinition(&type_change.changes) {
                    if let Some(class_def) = &type_change.new_type {
                        self.generate_type_definition(script, &type_change.name, class_def)?;
                    }
                }
            }

            for attr in &diff.modified_attributes {
                if let Some(slot_def) = &attr.new_attr {
                    self.generate_attribute_definition(script, &attr.name, slot_def)?;
                }
            }

            writeln!(script).map_err(Self::fmt_error_to_migration_error)?;
        }

        Ok(())
    }

    /// Check if changes require redefinition
    fn needs_redefinition(changes: &[DetailedChange]) -> bool {
        changes.iter().any(|change| matches!(
            change,
            DetailedChange::RemovedInheritance(_) |
            DetailedChange::AbstractChanged(_, _) |
            DetailedChange::RemovedSlot(_)
        ))
    }

    /// Generate attribute definition
    fn generate_attribute_definition(&self, script: &mut String, name: &str, slot: &SlotDefinition) -> MigrationResult<()> {
        let attr_name = self.generator.convert_identifier(name);
        let value_type = self.map_slot_to_typeql_type(slot);

        write!(script, "  {attr_name} sub attribute, value {value_type};").map_err(Self::fmt_error_to_migration_error)?;

        // Add constraints
        if let Some(pattern) = &slot.pattern {
            write!(script, ", regex \"{}\"", self.escape_regex(pattern)).map_err(Self::fmt_error_to_migration_error)?;
        }

        writeln!(script, ";").map_err(Self::fmt_error_to_migration_error)?;
        Ok(())
    }

    /// Generate type definition
    fn generate_type_definition(&self, script: &mut String, name: &str, class: &ClassDefinition) -> MigrationResult<()> {
        let type_name = self.generator.convert_identifier(name);
        let is_abstract = class.abstract_.unwrap_or(false);

        // Determine parent
        let parent = if let Some(parent_name) = &class.is_a {
            self.generator.convert_identifier(parent_name)
        } else {
            "entity".to_string()
        };

        write!(script, "  {type_name}").map_err(Self::fmt_error_to_migration_error)?;
        if is_abstract {
            write!(script, " sub {parent}, abstract").map_err(Self::fmt_error_to_migration_error)?;
        } else {
            write!(script, " sub {parent}").map_err(Self::fmt_error_to_migration_error)?;
        }

        // Add owned attributes
        for slot_name in &class.slots {
            let attr_name = self.generator.convert_identifier(slot_name);
            write!(script, ",
    owns {attr_name}").map_err(Self::fmt_error_to_migration_error)?;

            // Add constraints from slot_usage
            if let Some(slot_def) = class.slot_usage.get(slot_name) {
                if slot_def.identifier == Some(true) {
                    write!(script, " @key").map_err(Self::fmt_error_to_migration_error)?;
                } else if slot_def.required == Some(true) {
                    write!(script, " @card(1..1)").map_err(Self::fmt_error_to_migration_error)?;
                }
            }
        }

        writeln!(script, ";").map_err(Self::fmt_error_to_migration_error)?;
        Ok(())
    }

    /// Generate rollback migration
    fn generate_rollback_migration(&self, diff: &SchemaDiff, migration: &mut MigrationScript) -> MigrationResult<()> {
        let script = &mut migration.rollback_script;

        writeln!(script, "# WARNING: This rollback may cause data loss!").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(script, "# Backup your data before running this script.").map_err(Self::fmt_error_to_migration_error)?;
        writeln!(script).map_err(Self::fmt_error_to_migration_error)?;

        // Rollback is essentially the reverse of forward migration
        // First, undefine what was added
        if !diff.added_types.is_empty() || !diff.added_attributes.is_empty() {
            writeln!(script, "# Remove elements that were added").map_err(Self::fmt_error_to_migration_error)?;
            writeln!(script, "undefine").map_err(Self::fmt_error_to_migration_error)?;

            for type_change in &diff.added_types {
                let type_name = self.generator.convert_identifier(&type_change.name);
                writeln!(script, "  {type_name} sub thing;").map_err(Self::fmt_error_to_migration_error)?;
            }

            for attr in &diff.added_attributes {
                let attr_name = self.generator.convert_identifier(&attr.name);
                writeln!(script, "  {attr_name} sub attribute;").map_err(Self::fmt_error_to_migration_error)?;
            }

            writeln!(script).map_err(Self::fmt_error_to_migration_error)?;
        }

        // Then, redefine what was removed
        if !diff.removed_types.is_empty() || !diff.removed_attributes.is_empty() {
            writeln!(script, "# Restore removed elements").map_err(Self::fmt_error_to_migration_error)?;
            writeln!(script, "define").map_err(Self::fmt_error_to_migration_error)?;

            for attr in &diff.removed_attributes {
                if let Some(slot_def) = &attr.old_attr {
                    self.generate_attribute_definition(script, &attr.name, slot_def)?;
                }
            }

            for type_change in &diff.removed_types {
                if let Some(class_def) = &type_change.old_type {
                    self.generate_type_definition(script, &type_change.name, class_def)?;
                }
            }

            writeln!(script).map_err(Self::fmt_error_to_migration_error)?;
        }

        Ok(())
    }

    /// Generate data migration queries
    fn generate_data_migrations(&self, diff: &SchemaDiff, migration: &mut MigrationScript) -> MigrationResult<()> {
        // Handle attribute renames
        for attr in &diff.modified_attributes {
            for change in &attr.changes {
                if let DetailedChange::SlotChanged(_, slot_change) = change {
                    if let Some((old_range, new_range)) = &slot_change.range_changed {
                        // Generate type conversion migration
                        let data_migration = DataMigration {
                            description: format!("Convert {} from {} to {}", attr.name, old_range, new_range),
                            query: self.generate_type_conversion_query(&attr.name, old_range, new_range),
                            idempotent: false,
                            estimated_records: None};
                        migration.data_migrations.push(data_migration);
                    }
                }
            }
        }

        // Handle required field additions
        for attr in &diff.added_attributes {
            if let Some(slot) = &attr.new_attr {
                if slot.required == Some(true) {
                    // Generate default value insertion
                    let default_value = self.get_default_value_for_type(slot);
                    let data_migration = DataMigration {
                        description: format!("Add default value for required field {}", attr.name),
                        query: self.generate_default_value_query(&attr.name, &default_value),
                        idempotent: true,
                        estimated_records: None};
                    migration.data_migrations.push(data_migration);
                }
            }
        }

        Ok(())
    }

    /// Generate type conversion query
    fn generate_type_conversion_query(&self, attr_name: &str, old_type: &str, new_type: &str) -> String {
        let attr = self.generator.convert_identifier(attr_name);

        // Example: Convert string to integer
        format!(
            r"match
  $x isa thing, has {attr} $old;
  $old isa {attr};
get $x, $old;
# Then process results and insert new values
# This requires application-level processing"
        )
    }

    /// Generate default value insertion query
    fn generate_default_value_query(&self, attr_name: &str, default_value: &str) -> String {
        let attr = self.generator.convert_identifier(attr_name);

        format!(
            r"match
  $x isa thing;
  not {{ $x has {attr}; }};
insert
  $x has {attr} {default_value};"
        )
    }

    /// Get default value for a type
    fn get_default_value_for_type(&self, slot: &SlotDefinition) -> String {
        match slot.range.as_deref() {
            Some("string") => "\"\"".to_string(),
            Some("integer" | "long") => "0".to_string(),
            Some("float" | "double") => "0.0".to_string(),
            Some("boolean") => "false".to_string(),
            Some("date" | "datetime") => "2024-01-01T00:00:00".to_string(),
            _ => "\"\"".to_string()}
    }

    /// Map slot type to `TypeQL` value type
    fn map_slot_to_typeql_type(&self, slot: &SlotDefinition) -> &'static str {
        match slot.range.as_deref() {
            Some("string" | "str") => "string",
            Some("integer" | "int") => "long",
            Some("float" | "double") => "double",
            Some("boolean" | "bool") => "boolean",
            Some("date" | "datetime") => "datetime",
            _ => "string"}
    }

    /// Escape regex for `TypeQL`
    fn escape_regex(&self, pattern: &str) -> String {
        pattern.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

impl Default for MigrationGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::typeql_migration::{
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};
        diff::SchemaDiffer,
        analyzer::MigrationAnalyzer};


    #[test]
    fn test_simple_migration_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let old_schema = SchemaDefinition::default();
        let mut new_schema = SchemaDefinition::default();

        // Add a new class
        let mut person = ClassDefinition::default();
        person.slots.push("name".to_string());
        new_schema.classes.insert("Person".to_string(), person);

        // Add a new attribute
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        new_schema.slots.insert("name".to_string(), name_slot);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should compare schemas: {}");
        let impact = MigrationAnalyzer::analyze_impact(&diff).expect("should analyze impact: {}");

        let generator = MigrationGenerator::new();
        let migration = generator.generate(&diff, &impact, "1.0.0", "1.1.0").expect("should generate migration: {}");

        let forward = migration.forward_script().expect("should generate forward script: {}");
        assert!(forward.contains("define"));
        assert!(forward.contains("name sub attribute"));
        assert!(forward.contains("person sub entity"));
        assert!(forward.contains("owns name"));
        Ok(())
    }

    #[test]
    fn test_breaking_migration_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut old_schema = SchemaDefinition::default();
        let new_schema = SchemaDefinition::default();

        // Add a class to old that's removed in new
        let removed_class = ClassDefinition::default();
        old_schema.classes.insert("OldClass".to_string(), removed_class);

        let diff = SchemaDiffer::compare(&old_schema, &new_schema).expect("should compare schemas: {}");
        let impact = MigrationAnalyzer::analyze_impact(&diff).expect("should analyze impact: {}");

        let generator = MigrationGenerator::new();
        let migration = generator.generate(&diff, &impact, "1.0.0", "2.0.0").expect("should generate migration: {}");

        assert!(migration.metadata.is_breaking);

        let forward = migration.forward_script().expect("should generate forward script: {}");
        assert!(forward.contains("undefine"));
        assert!(forward.contains("old-class sub thing"));

        let rollback = migration.rollback_script().expect("should generate rollback script: {}");
        assert!(rollback.contains("define"));
        assert!(rollback.contains("old-class sub entity"));
        Ok(())
    }
}