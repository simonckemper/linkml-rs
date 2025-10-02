//! Schema Inference Module - Phase 4: Integration & Optimization
//!
//! This module provides automated LinkML schema inference from structured data
//! with full RootReal service integration including Format Identification Service,
//! Parse Service, Logger Service, Timestamp Service, and Task Management Service.
//!
//! # Architecture
//!
//! The inference system consists of several components:
//!
//! - **Inference Engine** (`engine.rs`) - Orchestrates format detection and schema generation
//! - **Schema Builder** (`builder.rs`) - Fluent API for programmatic schema construction
//! - **Type Inference** (`type_inference.rs`) - Detect data types from sample values
//! - **Format Introspectors** (`introspectors/`) - Format-specific structure analysis
//! - **Factory Functions** (`factory.rs`) - Service creation with dependency injection
//!
//! # Phase 4 Features
//!
//! - Automatic format detection using PRONOM signatures
//! - Parse Service integration for data extraction
//! - Multi-document statistical analysis
//! - Parallel batch processing
//! - Type refinement with confidence scoring
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use linkml_service::inference::create_inference_engine;
//! use std::path::Path;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create engine with all services
//! let engine = create_inference_engine().await?;
//!
//! // Automatic format detection and schema generation
//! let schema = engine.infer_from_file_auto(Path::new("data.xml")).await?;
//! println!("Generated schema: {}", schema.name);
//! # Ok(())
//! # }
//! ```

pub mod builder;
pub mod engine;
pub mod factory;
pub mod introspectors;
pub mod traits;
pub mod type_inference;
pub mod types;

pub use builder::{BuilderResult, ClassBuilder, SchemaBuilder, SlotBuilder};
pub use engine::InferenceEngine;
pub use factory::{
    create_csv_introspector, create_inference_engine, create_json_introspector,
    create_xml_introspector,
};
pub use introspectors::{CsvIntrospector, JsonIntrospector, XmlIntrospector};
pub use traits::{DataIntrospector, InferenceError, InferenceResult, InferredType, TypeInferencer};
pub use type_inference::{StandardTypeInferencer, create_type_inferencer};
pub use types::{
    AggregatedChildStats, AggregatedElementStats, AggregatedStats, AttributeStats, ChildStats,
    DocumentMetrics, DocumentStats, ElementStats, InferenceConfig, SchemaMetadata, TypeVotes,
};
