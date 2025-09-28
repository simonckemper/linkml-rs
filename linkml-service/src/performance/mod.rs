//! Performance optimization utilities
//!
//! This module contains tools for profiling, optimizing, and monitoring
//! the performance of the LinkML service.

pub mod memory;
pub mod profiling;
pub mod small_vec;
pub mod string_cache;

pub use memory::{MemoryProfiler, MemoryScope, MemorySize, MemoryStats, global_memory_profiler};
pub use profiling::{PerfCounter, Profiler};
pub use small_vec::{
    IssueVec, PathVec, SlotVec, ValidatorVec, issue_vec, path_vec, slot_vec, validator_vec,
};
pub use string_cache::{global_interner, intern, str_eq_fast};

// Re-export macros
pub use crate::{profile_fn, profile_scope};
