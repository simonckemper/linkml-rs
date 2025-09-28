//! Pattern matching module with named capture groups
//!
//! This module provides advanced pattern matching capabilities for LinkML validation,
//! including named capture groups and sophisticated pattern matching operations.
//! All pattern matching is implemented using Rust's regex crate for compatibility
//! and performance.

/// Named capture group pattern matching
pub mod named_captures;

/// Advanced pattern matcher with caching and optimization
pub mod pattern_matcher;
