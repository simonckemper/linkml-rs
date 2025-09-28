//! SchemaView - High-level API for LinkML schema introspection and navigation
//!
//! This module provides a "denormalized view" of LinkML schemas, making it easier to
//! programmatically introspect, navigate, and analyze schemas by resolving inheritance,
//! imports, and slot usage patterns.

pub mod analysis;
pub mod class_view;
pub mod navigation;
pub mod slot_view;
pub mod view;

pub use class_view::{ClassView, ClassViewBuilder};
pub use slot_view::{SlotView, SlotViewBuilder};
pub use view::{ElementType, SchemaView, SchemaViewError};

// Re-export commonly used types
pub use analysis::{SchemaStatistics, UsageInfo};
pub use navigation::{InheritanceChain, SlotResolution};
