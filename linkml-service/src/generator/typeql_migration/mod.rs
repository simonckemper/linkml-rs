//! `TypeQL` schema migration support
//!
//! This module provides functionality for detecting schema changes,
//! generating migration scripts, and managing schema versions.

mod version;
mod diff;
mod analyzer;
mod generator;

pub use version::{SchemaVersion, VersionedSchema};
pub use diff::{
    SchemaDiff, SchemaDiffer, TypeChange, AttributeChange, RelationChange, RuleChange,
    DetailedChange, SlotChange, RolePlayerChange
};
pub use analyzer::{MigrationAnalyzer, ChangeImpact, ChangeCategory};
pub use generator::{MigrationGenerator, MigrationScript, DataMigration, MigrationMetadata};

use thiserror::Error;

/// Errors that can occur during migration operations
#[derive(Debug, Error)]
pub enum MigrationError {
    /// Schema version parsing error
    #[error("Invalid version format: {0}")]
    InvalidVersion(String),

    /// Schema comparison error
    #[error("Cannot compare schemas: {0}")]
    ComparisonError(String),

    /// Migration generation error
    #[error("Migration generation failed: {0}")]
    GenerationError(String),

    /// Breaking change detected
    #[error("Breaking change detected: {0}")]
    BreakingChange(String),

    /// Validation error
    #[error("Migration validation failed: {0}")]
    ValidationError(String)}

/// Result type for migration operations
pub type MigrationResult<T> = Result<T, MigrationError>;