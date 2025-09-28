//! # `LinkML` Core
//!
//! Core types and traits for `LinkML` schema validation in Rust.
//!
//! This crate provides the fundamental building blocks for working with `LinkML` schemas,
//! including type definitions, validation traits, and error handling.
//!
//! ## Design Principles
//!
//! - **100% Feature Parity**: Full compatibility with Python `LinkML`
//! - **Native Performance**: Leveraging Rust's zero-cost abstractions
//! - **Type Safety**: Compile-time guarantees where possible
//! - **Composition Over Inheritance**: Following Rust idioms from Chapter 17

#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(missing_docs)] // Documentation is covered by module-level docs

/// Core error types for `LinkML` operations
pub mod error;

/// Core trait definitions for `LinkML` services
pub mod traits;

/// Type definitions for `LinkML` schemas and data
pub mod types;

/// Configuration types for `LinkML` services
pub mod config;

/// Validation types and utilities for `LinkML` schemas
pub mod validation;

/// RootReal-compliant configuration structures
pub mod configuration;

/// Enhanced configuration with full externalization
pub mod configuration_v2;

/// Utility functions and helpers
pub mod utils;

/// Schema settings for controlling validation and generation behavior
pub mod settings;

/// Annotation support for schema elements
pub mod annotations;

/// Metadata support for schema elements
pub mod metadata;

/// String interning pool for memory optimization
pub mod string_pool;

/// Optimized type definitions using string interning
pub mod types_v2;

/// Optimized utility functions that minimize cloning
pub mod utils_v2;

/// Optimized `HashMap` utilities
pub mod hashmap_utils;

/// Arc-based schema handling
pub mod schema_arc;

// Re-export commonly used types
pub use config::LinkMLConfig;
pub use configuration_v2::LinkMLServiceConfig;
pub use error::{LinkMLError, Result};
pub use serde_json::Value;
pub use settings::SchemaSettings;
pub use traits::{LinkMLService, SchemaOperations, SchemaFormat, ValidationOperations};
pub use types::{ClassDefinition, SchemaDefinition, SlotDefinition, StructuredPattern, ValidationReport, ValidationError, ValidationWarning};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::config::LinkMLConfig;
    pub use crate::error::{LinkMLError, Result};
    pub use crate::settings::*;
    pub use crate::traits::*;
    pub use crate::types::*;
}
