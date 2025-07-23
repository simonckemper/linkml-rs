//! Schema manipulation and analysis tools
//!
//! This module provides utilities for working with LinkML schemas,
//! including diff, merge, and lint functionality.

pub mod diff;
pub mod merge;
pub mod lint;

pub use diff::{SchemaDiff, DiffOptions, DiffResult};
pub use merge::{SchemaMerge, MergeOptions, MergeResult};
pub use lint::{SchemaLinter, LintOptions, LintResult, LintRule, Severity};