//! Performance profiling utilities for identifying hot paths
//!
//! This module provides tools to profile and optimize performance-critical
//! sections of the LinkML validation engine.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use parking_lot::Mutex;

/// Performance counter for tracking function call metrics
#[derive(Debug, Default)]
pub struct PerfCounter {
    /// Total number of calls
    pub call_count: AtomicU64,
    /// Total time spent in nanoseconds
    pub total_time_ns: AtomicU64,
    /// Maximum time for a single call in nanoseconds
    pub max_time_ns: AtomicU64,
    /// Minimum time for a single call in nanoseconds
    pub min_time_ns: AtomicU64,
}

impl PerfCounter {
    /// Record a timing measurement
    pub fn record(&self, duration: Duration) {
        let nanos = duration.as_nanos() as u64;
        
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_time_ns.fetch_add(nanos, Ordering::Relaxed);
        
        // Update max
        let mut current_max = self.max_time_ns.load(Ordering::Relaxed);
        while nanos > current_max {
            match self.max_time_ns.compare_exchange_weak(
                current_max,
                nanos,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
        
        // Update min
        let mut current_min = self.min_time_ns.load(Ordering::Relaxed);
        if current_min == 0 || nanos < current_min {
            while current_min == 0 || nanos < current_min {
                match self.min_time_ns.compare_exchange_weak(
                    current_min,
                    nanos,
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(x) => current_min = x,
                }
            }
        }
    }
    
    /// Get average time per call in nanoseconds
    pub fn avg_time_ns(&self) -> f64 {
        let count = self.call_count.load(Ordering::Relaxed);
        if count == 0 {
            0.0
        } else {
            self.total_time_ns.load(Ordering::Relaxed) as f64 / count as f64
        }
    }
    
    /// Get a summary of the counter
    pub fn summary(&self) -> String {
        let count = self.call_count.load(Ordering::Relaxed);
        let total_ns = self.total_time_ns.load(Ordering::Relaxed);
        let max_ns = self.max_time_ns.load(Ordering::Relaxed);
        let min_ns = self.min_time_ns.load(Ordering::Relaxed);
        let avg_ns = self.avg_time_ns();
        
        format!(
            "calls: {}, total: {:.3}ms, avg: {:.3}µs, min: {:.3}µs, max: {:.3}µs",
            count,
            total_ns as f64 / 1_000_000.0,
            avg_ns / 1_000.0,
            min_ns as f64 / 1_000.0,
            max_ns as f64 / 1_000.0,
        )
    }
}

/// Global profiler for tracking performance metrics
pub struct Profiler {
    counters: Arc<Mutex<HashMap<String, Arc<PerfCounter>>>>,
    enabled: AtomicU64,
}

impl Profiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            enabled: AtomicU64::new(1),
        }
    }
    
    /// Enable or disable profiling
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(if enabled { 1 } else { 0 }, Ordering::Relaxed);
    }
    
    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }
    
    /// Get or create a counter for the given key
    pub fn get_counter(&self, key: &str) -> Arc<PerfCounter> {
        let mut counters = self.counters.lock();
        counters.entry(key.to_string())
            .or_insert_with(|| Arc::new(PerfCounter::default()))
            .clone()
    }
    
    /// Record a timing for the given key
    pub fn record(&self, key: &str, duration: Duration) {
        if self.is_enabled() {
            self.get_counter(key).record(duration);
        }
    }
    
    /// Time a function and record the result
    pub fn time<F, R>(&self, key: &str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        if self.is_enabled() {
            let start = Instant::now();
            let result = f();
            let duration = start.elapsed();
            self.record(key, duration);
            result
        } else {
            f()
        }
    }
    
    /// Get a report of all counters
    pub fn report(&self) -> String {
        let mut report = String::from("Performance Profile Report\n");
        report.push_str("==========================\n\n");
        
        let counters = self.counters.lock();
        let mut entries: Vec<_> = counters.iter().collect();
        
        // Sort by total time descending
        entries.sort_by(|a, b| {
            let a_time = a.1.total_time_ns.load(Ordering::Relaxed);
            let b_time = b.1.total_time_ns.load(Ordering::Relaxed);
            b_time.cmp(&a_time)
        });
        
        for (key, counter) in entries {
            report.push_str(&format!("{}: {}\n", key, counter.summary()));
        }
        
        report
    }
    
    /// Clear all counters
    pub fn clear(&self) {
        self.counters.lock().clear();
    }
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

/// RAII guard for timing a scope
pub struct TimingGuard<'a> {
    profiler: &'a Profiler,
    key: String,
    start: Instant,
}

impl<'a> TimingGuard<'a> {
    /// Create a new timing guard
    pub fn new(profiler: &'a Profiler, key: impl Into<String>) -> Self {
        Self {
            profiler,
            key: key.into(),
            start: Instant::now(),
        }
    }
}

impl<'a> Drop for TimingGuard<'a> {
    fn drop(&mut self) {
        if self.profiler.is_enabled() {
            let duration = self.start.elapsed();
            self.profiler.record(&self.key, duration);
        }
    }
}

/// Global profiler instance
static GLOBAL_PROFILER: once_cell::sync::Lazy<Profiler> = 
    once_cell::sync::Lazy::new(Profiler::new);

/// Get the global profiler
pub fn global_profiler() -> &'static Profiler {
    &GLOBAL_PROFILER
}

/// Macro for timing a block of code
#[macro_export]
macro_rules! profile_scope {
    ($key:expr) => {
        let _guard = $crate::performance::profiling::TimingGuard::new(
            $crate::performance::profiling::global_profiler(),
            $key
        );
    };
}

/// Macro for timing a function call
#[macro_export]
macro_rules! profile_fn {
    ($key:expr, $expr:expr) => {
        $crate::performance::profiling::global_profiler().time($key, || $expr)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    
    #[test]
    fn test_perf_counter() {
        let counter = PerfCounter::default();
        
        counter.record(Duration::from_millis(10));
        counter.record(Duration::from_millis(20));
        counter.record(Duration::from_millis(5));
        
        assert_eq!(counter.call_count.load(Ordering::Relaxed), 3);
        assert_eq!(counter.min_time_ns.load(Ordering::Relaxed), 5_000_000);
        assert_eq!(counter.max_time_ns.load(Ordering::Relaxed), 20_000_000);
        assert!((counter.avg_time_ns() - 11_666_666.0).abs() < 1000.0);
    }
    
    #[test]
    fn test_profiler() {
        let profiler = Profiler::new();
        
        // Time some operations
        profiler.time("test_op", || {
            thread::sleep(Duration::from_millis(1));
        });
        
        profiler.time("test_op", || {
            thread::sleep(Duration::from_millis(2));
        });
        
        let counter = profiler.get_counter("test_op");
        assert_eq!(counter.call_count.load(Ordering::Relaxed), 2);
        
        // Test disabling
        profiler.set_enabled(false);
        profiler.time("test_op", || {
            thread::sleep(Duration::from_millis(1));
        });
        
        // Count should still be 2
        assert_eq!(counter.call_count.load(Ordering::Relaxed), 2);
    }
    
    #[test]
    fn test_timing_guard() {
        let profiler = Profiler::new();
        
        {
            let _guard = TimingGuard::new(&profiler, "test_scope");
            thread::sleep(Duration::from_millis(1));
        }
        
        let counter = profiler.get_counter("test_scope");
        assert_eq!(counter.call_count.load(Ordering::Relaxed), 1);
    }
}