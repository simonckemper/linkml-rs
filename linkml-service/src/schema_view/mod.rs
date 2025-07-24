//! SchemaView - High-level API for LinkML schema introspection and navigation
//!
//! This module provides a "denormalized view" of LinkML schemas, making it easier to
//! programmatically introspect, navigate, and analyze schemas by resolving inheritance,
//! imports, and slot usage patterns.

pub mod analysis;
pub mod navigation;
pub mod view;

pub use view::{SchemaView, SchemaViewError, ElementType};

// Re-export commonly used types
pub use analysis::{SchemaStatistics, UsageInfo};
pub use navigation::{InheritanceChain, SlotResolution};