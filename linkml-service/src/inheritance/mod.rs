//! Complete inheritance resolution for LinkML schemas
//!
//! This module provides comprehensive support for class inheritance including:
//! - Multiple inheritance with C3 linearization
//! - Mixin support with proper resolution order
//! - Diamond inheritance pattern handling
//! - Slot override and usage merging

pub mod resolver;

pub use resolver::{InheritanceResolver, get_inheritance_chain, is_subclass_of};
