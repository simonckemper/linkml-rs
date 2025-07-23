//! Performance optimization utilities
//!
//! This module contains tools for profiling, optimizing, and monitoring
//! the performance of the LinkML service.

pub mod profiling;
pub mod string_cache;
pub mod small_vec;
pub mod memory;

pub use profiling::{Profiler, PerfCounter, global_profiler};
pub use string_cache::{intern, str_eq_fast, global_interner};
pub use small_vec::{IssueVec, SlotVec, ValidatorVec, PathVec, issue_vec, slot_vec, validator_vec, path_vec};
pub use memory::{MemoryProfiler, MemoryStats, MemorySize, MemoryScope, global_memory_profiler};

// Re-export macros
pub use crate::{profile_scope, profile_fn};