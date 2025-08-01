//! Validation engine for LinkML schemas
//!
//! This module implements a high-performance validation engine that validates
//! data against LinkML schemas. It supports:
//!
//! - Type validation (string, integer, float, boolean, date, etc.)
//! - Constraint validation (required, pattern, range, cardinality, etc.)
//! - Cross-field validation
//! - Instance-based permissible values
//! - Compiled validators for performance
//! - Parallel validation support

pub mod buffer_pool;
pub mod cache;
pub mod cache_key_optimizer;
pub mod cache_warmer;
pub mod compiled;
pub mod composition;
pub mod context;
pub mod engine;
pub mod error_recovery;
pub mod instance_loader;
pub mod json_path;
pub mod memory_layout;
pub mod memory_safety;
pub mod multi_layer_cache;
pub mod panic_prevention;
pub mod parallel;
pub mod report;
pub mod resource_limiter;
pub mod security;
pub mod stress_test;
pub mod string_interner;
pub mod ttl_manager;
pub mod validators;

pub use cache_warmer::{AccessEntry, WarmingStrategy};
pub use composition::{ResolvedClass, SchemaComposer};
pub use context::ValidationContext;
pub use engine::{ValidationEngine, ValidationOptions};
pub use instance_loader::{InstanceConfig, InstanceData, InstanceLoader};
pub use report::{Severity, ValidationIssue, ValidationReport};

use linkml_core::error::Result;
use linkml_core::types::SchemaDefinition;
use serde_json::Value;

/// Main validation function - validates JSON data against a schema
///
/// # Errors
///
/// Returns an error if engine creation fails or validation encounters an error.
pub async fn validate(
    schema: &SchemaDefinition,
    data: &Value,
    options: Option<ValidationOptions>,
) -> Result<ValidationReport> {
    let engine = ValidationEngine::new(schema)?;
    engine.validate(data, options).await
}

/// Validate with a specific target class
///
/// # Errors
///
/// Returns an error if engine creation fails or the specified class is not found.
pub async fn validate_as_class(
    schema: &SchemaDefinition,
    data: &Value,
    class_name: &str,
    options: Option<ValidationOptions>,
) -> Result<ValidationReport> {
    let engine = ValidationEngine::new(schema)?;
    engine.validate_as_class(data, class_name, options).await
}

/// Validate a collection of instances with unique key constraints
///
/// # Errors
///
/// Returns an error if engine creation fails or validation encounters an error.
pub async fn validate_collection(
    schema: &SchemaDefinition,
    instances: &[Value],
    class_name: &str,
    options: Option<ValidationOptions>,
) -> Result<ValidationReport> {
    let mut engine = ValidationEngine::new(schema)?;
    engine.validate_collection(instances, class_name, options).await
}
