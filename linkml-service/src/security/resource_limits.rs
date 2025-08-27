//! Resource limiting utilities
//!
//! This module provides mechanisms to limit resource usage during
//! validation and expression evaluation to prevent DoS attacks.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use thiserror::Error;

/// Errors related to resource limits
#[derive(Debug, Error)]
pub enum ResourceError {
    /// Validation timeout exceeded
    #[error("Validation timeout exceeded: {elapsed:.2}s (max: {max:.2}s)")]
    Timeout {
        /// Elapsed time in seconds
        elapsed: f64,
        /// Maximum allowed time in seconds
        max: f64,
    },

    /// Memory limit exceeded
    #[error("Memory limit exceeded: {used} bytes (max: {max} bytes)")]
    MemoryExceeded {
        /// Memory used in bytes
        used: usize,
        /// Maximum allowed memory in bytes
        max: usize,
    },

    /// Too many parallel operations
    #[error("Too many parallel operations: {current} (max: {max})")]
    TooManyParallelOps {
        /// Current number of operations
        current: usize,
        /// Maximum allowed operations
        max: usize,
    },

    /// Cache memory exceeded
    #[error("Cache memory exceeded: {used} bytes (max: {max} bytes)")]
    CacheMemoryExceeded {
        /// Cache memory used in bytes
        used: usize,
        /// Maximum allowed cache memory in bytes
        max: usize,
    },
}

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum validation time
    pub max_validation_time: Duration,

    /// Maximum memory usage (approximate)
    pub max_memory_usage: usize,

    /// Maximum number of parallel validators
    pub max_parallel_validators: usize,

    /// Maximum cache memory
    pub max_cache_memory: usize,

    /// Maximum expression evaluation time
    pub max_expression_time: Duration,

    /// Maximum number of validation errors to collect
    pub max_validation_errors: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_validation_time: Duration::from_secs(30),
            max_memory_usage: 1_000_000_000, // 1GB
            max_parallel_validators: 100,
            max_cache_memory: 100_000_000, // 100MB
            max_expression_time: Duration::from_secs(1),
            max_validation_errors: 1000,
        }
    }
}

impl ResourceLimits {
    /// Create resource limits from LinkML service configuration
    pub fn from_service_config(
        config: &linkml_core::configuration_v2::SecurityLimitsConfig,
    ) -> Self {
        Self {
            max_validation_time: Duration::from_millis(config.max_validation_time_ms),
            max_memory_usage: config.max_memory_usage_bytes,
            max_parallel_validators: config.max_parallel_validators,
            max_cache_memory: config.max_cache_memory_bytes,
            max_expression_time: Duration::from_millis(config.max_expression_time_ms),
            max_validation_errors: config.max_validation_errors,
        }
    }
}

/// Resource monitor for tracking usage
#[derive(Debug)]
pub struct ResourceMonitor {
    limits: ResourceLimits,
    start_time: Instant,
    memory_used: AtomicUsize,
    parallel_ops: AtomicUsize,
    cache_memory: AtomicUsize,
    validation_errors: AtomicUsize,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            limits,
            start_time: Instant::now(),
            memory_used: AtomicUsize::new(0),
            parallel_ops: AtomicUsize::new(0),
            cache_memory: AtomicUsize::new(0),
            validation_errors: AtomicUsize::new(0),
        }
    }

    /// Check if validation has timed out
    pub fn check_timeout(&self) -> Result<(), ResourceError> {
        let elapsed = self.start_time.elapsed();
        if elapsed > self.limits.max_validation_time {
            return Err(ResourceError::Timeout {
                elapsed: elapsed.as_secs_f64(),
                max: self.limits.max_validation_time.as_secs_f64(),
            });
        }
        Ok(())
    }

    /// Check if expression evaluation has timed out
    pub fn check_expression_timeout(&self, start: Instant) -> Result<(), ResourceError> {
        let elapsed = start.elapsed();
        if elapsed > self.limits.max_expression_time {
            return Err(ResourceError::Timeout {
                elapsed: elapsed.as_secs_f64(),
                max: self.limits.max_expression_time.as_secs_f64(),
            });
        }
        Ok(())
    }

    /// Track memory allocation
    pub fn allocate_memory(&self, bytes: usize) -> Result<(), ResourceError> {
        let new_total = self.memory_used.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if new_total > self.limits.max_memory_usage {
            self.memory_used.fetch_sub(bytes, Ordering::Relaxed);
            return Err(ResourceError::MemoryExceeded {
                used: new_total,
                max: self.limits.max_memory_usage,
            });
        }
        Ok(())
    }

    /// Release tracked memory
    pub fn release_memory(&self, bytes: usize) {
        self.memory_used.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Start a parallel operation
    pub fn start_parallel_op(&self) -> Result<ParallelOpGuard, ResourceError> {
        let current = self.parallel_ops.fetch_add(1, Ordering::Relaxed) + 1;
        if current > self.limits.max_parallel_validators {
            self.parallel_ops.fetch_sub(1, Ordering::Relaxed);
            return Err(ResourceError::TooManyParallelOps {
                current,
                max: self.limits.max_parallel_validators,
            });
        }
        Ok(ParallelOpGuard { monitor: self })
    }

    /// Track cache memory usage
    pub fn allocate_cache_memory(&self, bytes: usize) -> Result<(), ResourceError> {
        let new_total = self.cache_memory.fetch_add(bytes, Ordering::Relaxed) + bytes;
        if new_total > self.limits.max_cache_memory {
            self.cache_memory.fetch_sub(bytes, Ordering::Relaxed);
            return Err(ResourceError::CacheMemoryExceeded {
                used: new_total,
                max: self.limits.max_cache_memory,
            });
        }
        Ok(())
    }

    /// Release cache memory
    pub fn release_cache_memory(&self, bytes: usize) {
        self.cache_memory.fetch_sub(bytes, Ordering::Relaxed);
    }

    /// Track validation error count
    pub fn add_validation_error(&self) -> bool {
        let count = self.validation_errors.fetch_add(1, Ordering::Relaxed) + 1;
        count <= self.limits.max_validation_errors
    }

    /// Get current resource usage
    pub fn current_usage(&self) -> ResourceUsage {
        ResourceUsage {
            elapsed: self.start_time.elapsed(),
            memory_used: self.memory_used.load(Ordering::Relaxed),
            parallel_ops: self.parallel_ops.load(Ordering::Relaxed),
            cache_memory: self.cache_memory.load(Ordering::Relaxed),
            validation_errors: self.validation_errors.load(Ordering::Relaxed),
        }
    }
}

/// RAII guard for parallel operations
pub struct ParallelOpGuard<'a> {
    monitor: &'a ResourceMonitor,
}

impl<'a> Drop for ParallelOpGuard<'a> {
    fn drop(&mut self) {
        self.monitor.parallel_ops.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Current resource usage snapshot
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Time elapsed during the operation
    pub elapsed: Duration,
    /// Amount of memory used in bytes
    pub memory_used: usize,
    /// Number of parallel operations executed
    pub parallel_ops: usize,
    /// Amount of cache memory used in bytes
    pub cache_memory: usize,
    /// Number of validation errors encountered
    pub validation_errors: usize,
}

impl ResourceUsage {
    /// Format usage as a human-readable string
    pub fn format_summary(&self) -> String {
        format!(
            "Elapsed: {:.2}s, Memory: {:.2}MB, Parallel Ops: {}, Cache: {:.2}MB, Errors: {}",
            self.elapsed.as_secs_f64(),
            self.memory_used as f64 / 1_048_576.0,
            self.parallel_ops,
            self.cache_memory as f64 / 1_048_576.0,
            self.validation_errors
        )
    }
}

/// Shared resource monitor for global tracking
pub type SharedResourceMonitor = Arc<ResourceMonitor>;

/// Create a new shared resource monitor
pub fn create_monitor(limits: ResourceLimits) -> SharedResourceMonitor {
    Arc::new(ResourceMonitor::new(limits))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_timeout_check() {
        let limits = ResourceLimits {
            max_validation_time: Duration::from_millis(100),
            ..Default::default()
        };
        let monitor = ResourceMonitor::new(limits);

        // Should not timeout immediately
        assert!(monitor.check_timeout().is_ok());

        // Sleep and check timeout
        thread::sleep(Duration::from_millis(150));
        assert!(matches!(
            monitor.check_timeout(),
            Err(ResourceError::Timeout { .. })
        ));
    }

    #[test]
    fn test_memory_tracking() {
        let monitor = ResourceMonitor::new(ResourceLimits::default());

        // Allocate some memory
        assert!(monitor.allocate_memory(1000).is_ok());
        assert_eq!(monitor.current_usage().memory_used, 1000);

        // Release memory
        monitor.release_memory(500);
        assert_eq!(monitor.current_usage().memory_used, 500);
    }

    #[test]
    fn test_parallel_ops() {
        let limits = ResourceLimits {
            max_parallel_validators: 2,
            ..Default::default()
        };
        let monitor = Arc::new(ResourceMonitor::new(limits));

        // Start two ops (should succeed)
        let _guard1 = monitor.start_parallel_op()?;
        let _guard2 = monitor.start_parallel_op()?;

        // Third should fail
        assert!(matches!(
            monitor.start_parallel_op(),
            Err(ResourceError::TooManyParallelOps { .. })
        ));

        // Drop one guard
        drop(_guard1);

        // Now we can start another
        let _guard3 = monitor.start_parallel_op()?;
    }
}
