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

use linkml_core::types::SchemaDefinition;
pub mod buffer_pool;
pub mod cache;
pub mod cache_key_optimizer;
pub mod cache_warmer;
pub mod compiled;
pub mod composition;
pub mod conditional_validator;
pub mod context;
pub mod default_applier;
pub mod engine;
pub mod error_recovery;
pub mod instance_loader;
pub mod interned_report;
pub mod json_path;
pub mod memory_layout;
pub mod memory_safety;
pub mod multi_layer_cache;
pub mod panic_prevention;
pub mod parallel;
pub mod pattern_validator;
pub mod recursion_checker;
pub mod report;
pub mod resource_limiter;
pub mod security;
pub mod stress_test;
pub mod string_interner;
pub mod ttl_manager;
pub mod unique_key_validator;
pub mod validators;

pub use cache_warmer::{AccessEntry, WarmingStrategy};
pub use composition::{ResolvedClass, SchemaComposer};
pub use conditional_validator::{
    Condition, ConditionalRule, ConditionalValidator, ConditionalViolation, Requirement,
};
pub use context::ValidationContext;
pub use default_applier::{DefaultApplier, apply_defaults_to_instance};
pub use engine::{ValidationEngine, ValidationOptions};
pub use instance_loader::{InstanceConfig, InstanceData, InstanceLoader};
pub use pattern_validator::{PatternTransformer, PatternValidator, validate_patterns};
pub use recursion_checker::{RecursionTracker, check_recursion};
pub use report::{Severity, ValidationIssue, ValidationReport};
pub use unique_key_validator::{UniqueKeyIndex, UniqueKeyValidator, UniqueKeyViolation};
pub use validators::Validator;

use serde_json::Value;

/// Main validation function - validates `JSON` data against a schema
///
/// # Errors
///
/// Returns an error if engine creation fails or validation encounters an error.
pub async fn validate(
    schema: &SchemaDefinition,
    data: &Value,
    options: Option<ValidationOptions>,
) -> linkml_core::error::Result<ValidationReport> {
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
) -> linkml_core::error::Result<ValidationReport> {
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
) -> linkml_core::error::Result<ValidationReport> {
    let mut engine = ValidationEngine::new(schema)?;
    engine
        .validate_collection(instances, class_name, options)
        .await
}
