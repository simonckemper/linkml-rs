//! Schema manipulation and analysis tools
//!
//! This module provides utilities for working with LinkML schemas,
//! including diff, merge, patch, and lint functionality.

pub mod diff;
pub mod lint;
pub mod merge;
pub mod patch;

pub use diff::{DiffOptions, DiffResult, SchemaDiff};
pub use lint::{LintOptions, LintResult, LintRule, SchemaLinter, Severity};
pub use merge::{MergeOptions, MergeResult, SchemaMerge};
pub use patch::{PatchOptions, PatchResult, SchemaPatch, SchemaPatcher, create_patch_from_diff};
