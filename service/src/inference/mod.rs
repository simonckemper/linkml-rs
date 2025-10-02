//! Schema Inference Module
//!
//! This module provides automated LinkML schema inference from structured data.
//! It analyzes data formats (XML, JSON, CSV) and generates corresponding LinkML
//! schema definitions with inferred types, cardinality, and relationships.
//!
//! # Architecture
//!
//! The inference system consists of several components:
//!
//! - **Schema Builder** (`builder.rs`) - Fluent API for programmatic schema construction
//! - **Type Inference** (future) - Detect data types from sample values
//! - **Format Introspectors** (future) - Format-specific structure analysis
//! - **Inference Engine** (future) - Orchestrate multi-document analysis
//!
//! # Usage Example
//!
//! ```rust
//! use linkml_service::inference::builder::SchemaBuilder;
//!
//! let schema = SchemaBuilder::new("my_schema", "MySchema")
//!     .with_description("Automatically inferred schema")
//!     .with_version("1.0.0")
//!     .add_class("Person")
//!         .add_attribute("name", "string", true, false)
//!         .add_attribute("age", "integer", false, false)
//!         .finish()
//!     .build();
//!
//! // Serialize to YAML
//! let yaml = serde_yaml::to_string(&schema)?;
//! ```

pub mod builder;

pub use builder::{BuilderResult, ClassBuilder, SchemaBuilder, SlotBuilder};
