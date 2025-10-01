//! Prelude module for LinkML service
//!
//! This module re-exports commonly used types and functions for convenient import.

// Re-export core types and traits
pub use linkml_core::prelude::*;

// Re-export service implementations
pub use crate::service::LinkMLServiceImpl;

// Re-export factory functions
pub use crate::factory::{create_linkml_service, create_linkml_service_with_config};

// Re-export parser utilities
pub use crate::parser::{JsonParser, Parser, SchemaParser, YamlParser};

// Re-export validation types
pub use crate::validator::ValidationReport;
