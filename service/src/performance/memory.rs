//! Memory profiling and monitoring utilities
//!
//! This module provides tools to track and optimize memory usage
//! in the `LinkML` validation engine.

use parking_lot::Mutex;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of memory categories to track
const MAX_CATEGORIES: usize = 1000;

thread_local! {
    static THREAD_MEMORY: std::cell::RefCell<u64> = const { std::cell::RefCell::new(0) };
}

/// Global memory statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total bytes allocated
    pub allocated: AtomicU64,
    /// Total bytes deallocated
    pub deallocated: AtomicU64,
    /// Peak memory usage
    pub peak: AtomicU64,
    /// Number of allocations
    pub alloc_count: AtomicU64,
    /// Number of deallocations
    pub dealloc_count: AtomicU64,
}

impl MemoryStats {
    /// Get current memory usage
    pub fn current_usage(&self) -> u64 {
        let allocated = self.allocated.load(Ordering::Relaxed);
        let deallocated = self.deallocated.load(Ordering::Relaxed);
        allocated.saturating_sub(deallocated)
    }

    /// Update peak memory if current is higher
    pub fn update_peak(&self) {
        let current = self.current_usage();
        let mut peak = self.peak.load(Ordering::Relaxed);

        while current > peak {
            match self.peak.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }
    }

    /// Get a summary of memory statistics
    pub fn summary(&self) -> String {
        let current = self.current_usage();
        let peak = self.peak.load(Ordering::Relaxed);
        let allocs = self.alloc_count.load(Ordering::Relaxed);
        let deallocs = self.dealloc_count.load(Ordering::Relaxed);

        format!(
            "Current: {:.2} MB, Peak: {:.2} MB, Allocations: {}, Deallocations: {}",
            current as f64 / 1_048_576.0,
            peak as f64 / 1_048_576.0,
            allocs,
            deallocs
        )
    }
}

/// Memory profiler for tracking allocations by category
pub struct MemoryProfiler {
    stats: Arc<MemoryStats>,
    categories: Arc<Mutex<HashMap<String, MemoryStats>>>,
    enabled: AtomicU64,
}

impl MemoryProfiler {
    /// Create a new memory profiler
    #[must_use]
    pub fn new() -> Self {
        Self {
            stats: Arc::new(MemoryStats::default()),
            categories: Arc::new(Mutex::new(HashMap::new())),
            enabled: AtomicU64::new(0), // Disabled by default due to overhead
        }
    }

    /// Enable or disable memory profiling
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(u64::from(enabled), Ordering::Relaxed);
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    /// Record an allocation
    pub fn record_alloc(&self, size: u64, category: Option<&str>) {
        if !self.is_enabled() {
            return;
        }

        self.stats.allocated.fetch_add(size, Ordering::Relaxed);
        self.stats.alloc_count.fetch_add(1, Ordering::Relaxed);
        self.stats.update_peak();

        if let Some(cat) = category {
            let mut categories = self.categories.lock();

            // Check category limit before adding new categories
            if categories.len() >= MAX_CATEGORIES && !categories.contains_key(cat) {
                // Log or increment a counter for rejected categories
                return;
            }

            let cat_stats = categories.entry(cat.to_string()).or_default();
            cat_stats.allocated.fetch_add(size, Ordering::Relaxed);
            cat_stats.alloc_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a deallocation
    pub fn record_dealloc(&self, size: u64, category: Option<&str>) {
        if !self.is_enabled() {
            return;
        }

        self.stats.deallocated.fetch_add(size, Ordering::Relaxed);
        self.stats.dealloc_count.fetch_add(1, Ordering::Relaxed);

        if let Some(cat) = category {
            let mut categories = self.categories.lock();
            let cat_stats = categories.entry(cat.to_string()).or_default();
            cat_stats.deallocated.fetch_add(size, Ordering::Relaxed);
            cat_stats.dealloc_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Get overall memory statistics
    pub fn global_stats(&self) -> &MemoryStats {
        &self.stats
    }

    /// Get a report of memory usage by category
    pub fn category_report(&self) -> String {
        let mut report = String::from(
            "Memory Usage by Category
",
        );
        report.push_str(
            "========================

",
        );

        let categories = self.categories.lock();
        let mut entries: Vec<_> = categories
            .iter()
            .map(|(name, stats)| (name, stats.current_usage()))
            .collect();

        // Sort by current usage descending
        entries.sort_by(|a, b| b.1.cmp(&a.1));

        for (name, usage) in entries {
            // Writing to a String should never fail, but handle gracefully
            #[allow(clippy::cast_precision_loss)] // Intentional conversion for display
            let _ = writeln!(report, "{}: {:.2} MB", name, usage as f64 / 1_048_576.0);
        }

        // Writing to a String should never fail, but handle gracefully
        let _ = write!(
            report,
            "
{}
",
            self.stats.summary()
        );
        report
    }

    /// Clear all statistics
    pub fn clear(&self) {
        self.stats.allocated.store(0, Ordering::Relaxed);
        self.stats.deallocated.store(0, Ordering::Relaxed);
        self.stats.peak.store(0, Ordering::Relaxed);
        self.stats.alloc_count.store(0, Ordering::Relaxed);
        self.stats.dealloc_count.store(0, Ordering::Relaxed);
        self.categories.lock().clear();
    }
}

impl Default for MemoryProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Global memory profiler instance
static GLOBAL_MEMORY_PROFILER: std::sync::LazyLock<MemoryProfiler> =
    std::sync::LazyLock::new(MemoryProfiler::new);

/// Get the global memory profiler
#[must_use]
pub fn global_memory_profiler() -> &'static MemoryProfiler {
    &GLOBAL_MEMORY_PROFILER
}

/// Estimate the memory size of common types
pub trait MemorySize {
    /// Estimate the heap memory used by this value
    fn heap_size(&self) -> usize;

    /// Total memory size including stack
    fn total_size(&self) -> usize {
        std::mem::size_of_val(self) + self.heap_size()
    }
}

impl MemorySize for String {
    fn heap_size(&self) -> usize {
        self.capacity()
    }
}

impl<T> MemorySize for Vec<T> {
    fn heap_size(&self) -> usize {
        self.capacity() * std::mem::size_of::<T>()
    }
}

impl<K, V> MemorySize for HashMap<K, V> {
    fn heap_size(&self) -> usize {
        // Approximate - actual size depends on load factor
        self.capacity() * (std::mem::size_of::<K>() + std::mem::size_of::<V>() + 24)
    }
}

impl MemorySize for serde_json::Value {
    fn heap_size(&self) -> usize {
        match self {
            serde_json::Value::String(s) => s.heap_size(),
            serde_json::Value::Array(arr) => {
                arr.heap_size() + arr.iter().map(MemorySize::heap_size).sum::<usize>()
            }
            serde_json::Value::Object(map) => {
                // Estimate overhead based on number of entries
                // HashMap typically allocates power-of-2 capacity
                let estimated_capacity = map.len().next_power_of_two();
                let capacity_size =
                    estimated_capacity * (32 + std::mem::size_of::<serde_json::Value>());
                let content_size: usize =
                    map.iter().map(|(k, v)| k.heap_size() + v.heap_size()).sum();
                capacity_size + content_size
            }
            _ => 0, // Numbers, bools, null have no heap allocation
        }
    }
}

/// RAII guard for tracking memory in a scope
pub struct MemoryScope {
    category: String,
    start_usage: u64,
}

impl MemoryScope {
    /// Create a new memory tracking scope
    pub fn new(category: impl Into<String>) -> Self {
        let profiler = global_memory_profiler();
        let start_usage = profiler.global_stats().current_usage();

        Self {
            category: category.into(),
            start_usage,
        }
    }
}

impl Drop for MemoryScope {
    fn drop(&mut self) {
        let profiler = global_memory_profiler();
        let end_usage = profiler.global_stats().current_usage();

        if end_usage > self.start_usage {
            let allocated = end_usage - self.start_usage;
            profiler.record_alloc(allocated, Some(&self.category));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats::default();

        stats.allocated.store(1000, Ordering::Relaxed);
        stats.deallocated.store(300, Ordering::Relaxed);

        assert_eq!(stats.current_usage(), 700);

        stats.update_peak();
        assert_eq!(stats.peak.load(Ordering::Relaxed), 700);
    }

    #[test]
    fn test_memory_size_estimation() {
        let s = String::from("hello world");
        assert!(s.heap_size() >= 11);

        let v: Vec<i32> = vec![1, 2, 3, 4, 5];
        assert!(v.heap_size() >= 5 * 4);

        let json = serde_json::json!({
            "name": "test",
            "values": [1, 2, 3]
        });
        assert!(json.heap_size() > 0);
    }
}
