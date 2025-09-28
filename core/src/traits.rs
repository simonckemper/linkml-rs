//! Core trait definitions for LinkML services

use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;

use crate::error::Result;
use crate::types::{NamedCaptures, SchemaDefinition, ValidationReport};

/// Main trait for `LinkML` service operations
///
/// This trait is dyn-compatible and can be used as `Arc<dyn LinkMLService>`.
/// For generic operations, see `LinkMLServiceExt`.
#[async_trait]
pub trait LinkMLService: Send + Sync {
    /// Load a schema from a file path
    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition>;

    /// Load a schema from a string
    async fn load_schema_str(
        &self,
        content: &str,
        format: SchemaFormat,
    ) -> Result<SchemaDefinition>;

    /// Validate data against a schema
    async fn validate(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
        target_class: &str,
    ) -> Result<ValidationReport>;
}

/// Extension trait for generic `LinkML` operations
///
/// This trait contains methods with generic parameters that make it non-dyn-compatible.
/// Use this trait when you need typed validation operations.
#[async_trait]
pub trait LinkMLServiceExt: LinkMLService {
    /// Validate and return typed value
    async fn validate_typed<T>(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
        target_class: &str,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned;
}

/// Schema format enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaFormat {
    /// `YAML` format
    Yaml,
    /// `JSON` format
    Json,
}

/// Operations specific to schema manipulation
#[async_trait]
pub trait SchemaOperations: Send + Sync {
    /// Merge multiple schemas into one
    async fn merge_schemas(&self, schemas: Vec<SchemaDefinition>) -> Result<SchemaDefinition>;

    /// Resolve all imports in a schema
    async fn resolve_imports(&self, schema: &mut SchemaDefinition) -> Result<()>;

    /// Validate a schema against the meta-schema
    async fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()>;

    /// Get the effective slots for a class (including inherited)
    async fn get_class_slots(
        &self,
        schema: &SchemaDefinition,
        class_name: &str,
    ) -> Result<Vec<String>>;

    /// Check if a class is a subclass of another
    async fn is_subclass_of(
        &self,
        schema: &SchemaDefinition,
        child: &str,
        parent: &str,
    ) -> Result<bool>;
}

/// Operations for data validation
#[async_trait]
pub trait ValidationOperations: Send + Sync {
    /// Validate a single value against a slot definition
    async fn validate_slot(
        &self,
        value: &Value,
        slot_name: &str,
        schema: &SchemaDefinition,
    ) -> Result<()>;

    /// Validate a pattern with named capture groups
    async fn validate_pattern(&self, value: &str, pattern: &str) -> Result<NamedCaptures>;

    /// Check permissible values
    async fn check_permissible(
        &self,
        value: &str,
        slot_name: &str,
        schema: &SchemaDefinition,
    ) -> Result<bool>;

    /// Coerce a value to the expected type
    async fn coerce_value(&self, value: &Value, target_type: &str) -> Result<Value>;
}

/// Operations for code generation
#[async_trait]
pub trait GenerationOperations: Send + Sync {
    /// Generate `TypeQL` schema
    async fn generate_typeql(&self, schema: &SchemaDefinition) -> Result<String>;

    /// Generate Rust code
    async fn generate_rust(&self, schema: &SchemaDefinition) -> Result<String>;

    /// Generate GraphQL schema
    async fn generate_graphql(&self, schema: &SchemaDefinition) -> Result<String>;

    /// Generate documentation
    async fn generate_docs(&self, schema: &SchemaDefinition, format: DocFormat) -> Result<String>;
}

/// Documentation format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocFormat {
    /// Markdown format
    Markdown,
    /// HTML format
    Html,
    /// `ReStructuredText` format
    Rst,
}

/// Pattern matching operations
#[async_trait]
pub trait PatternOperations: Send + Sync {
    /// Extract named capture groups from a pattern match
    async fn extract_captures(&self, value: &str, pattern: &str) -> Result<NamedCaptures>;

    /// Validate against multiple patterns
    async fn validate_patterns(
        &self,
        value: &str,
        patterns: &[String],
    ) -> Result<Vec<NamedCaptures>>;

    /// Check if value matches any pattern
    async fn matches_any(&self, value: &str, patterns: &[String]) -> Result<bool>;
}

/// Instance-based validation operations
#[async_trait]
pub trait InstanceOperations: Send + Sync {
    /// Load permissible values from instance file
    async fn load_permissibles(&self, path: &Path, slot_name: &str) -> Result<Vec<String>>;

    /// Validate against instance-based permissibles
    async fn validate_instance(
        &self,
        value: &str,
        instance_path: &Path,
        slot_name: &str,
    ) -> Result<bool>;

    /// Stream validation for large instance sets
    async fn stream_validate(
        &self,
        values: Vec<String>,
        instance_path: &Path,
        slot_name: &str,
    ) -> Result<Vec<bool>>;
}

/// Schema evolution and migration operations
#[async_trait]
pub trait EvolutionOperations: Send + Sync {
    /// Generate migration between schema versions
    async fn generate_migration(
        &self,
        from: &SchemaDefinition,
        to: &SchemaDefinition,
    ) -> Result<SchemaMigration>;

    /// Apply migration to data
    async fn apply_migration(&self, data: &Value, migration: &SchemaMigration) -> Result<Value>;

    /// Validate data against historical schema version
    async fn validate_at_version(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
        version: &str,
    ) -> Result<ValidationReport>;
}

/// Schema migration information
#[derive(Debug, Clone)]
pub struct SchemaMigration {
    /// From version
    pub from_version: String,
    /// To version
    pub to_version: String,
    /// Migration steps
    pub steps: Vec<MigrationStep>,
}

/// Individual migration step
#[derive(Debug, Clone)]
pub enum MigrationStep {
    /// Add a new field
    AddField {
        class: String,
        field: String,
        default: Option<Value>,
    },
    /// Remove a field
    RemoveField { class: String, field: String },
    /// Rename a field
    RenameField {
        class: String,
        from: String,
        to: String,
    },
    /// Change field type
    ChangeType {
        class: String,
        field: String,
        from_type: String,
        to_type: String,
    },
    /// Add a new class
    AddClass { name: String },
    /// Remove a class
    RemoveClass { name: String },
    /// Custom transformation
    Transform {
        description: String,
        transform: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_format() {
        assert_eq!(SchemaFormat::Yaml, SchemaFormat::Yaml);
        assert_ne!(SchemaFormat::Yaml, SchemaFormat::Json);
    }

    #[test]
    fn test_doc_format() {
        assert_eq!(DocFormat::Markdown, DocFormat::Markdown);
        assert_ne!(DocFormat::Markdown, DocFormat::Html);
    }
}
