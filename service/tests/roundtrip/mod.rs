//! Round-trip testing framework for LinkML SchemaSheets
//!
//! This module provides comprehensive round-trip testing for bidirectional conversion
//! between LinkML schemas/data and Excel formats.
//!
//! ## Test Organization
//!
//! - `equivalence` - Semantic equivalence checker for schemas and data
//! - `schema_roundtrip` - Schema → Excel → Schema tests
//! - `data_roundtrip` - Data → Excel → Data tests
//!
//! ## Usage
//!
//! Run all round-trip tests:
//! ```bash
//! cargo test --test roundtrip
//! ```
//!
//! Run specific test suites:
//! ```bash
//! cargo test --test schema_roundtrip
//! cargo test --test data_roundtrip
//! ```

pub mod equivalence;
pub mod schema_roundtrip;
pub mod data_roundtrip;

// Re-export key types for convenience
pub use equivalence::{compare_schemas, Difference, EquivalenceResult};
