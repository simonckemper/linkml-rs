//! Resource limiting utilities
//!
//! This module provides mechanisms to limit resource usage during
//! validation and expression evaluation to prevent `DoS` attacks.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use thiserror::Error;
use timestamp_core::{TimestampError, TimestampService};

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

    /// Validation error
    #[error("Validation error: {message}")]
    ValidationError {
        /// Error message
        message: String,
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
    /// Create resource limits from `LinkML` service configuration
    #[must_use]
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
pub struct ResourceMonitor {
    limits: ResourceLimits,
    start_timestamp: i64, // Store as timestamp from service
    timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    memory_used: AtomicUsize,
    parallel_ops: AtomicUsize,
    cache_memory: AtomicUsize,
    validation_errors: AtomicUsize,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(
        limits: ResourceLimits,
        timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        // Initialize with 0, will be set when starting validation
        Self {
            limits,
            start_timestamp: 0,
            timestamp_service,
            memory_used: AtomicUsize::new(0),
            parallel_ops: AtomicUsize::new(0),
            cache_memory: AtomicUsize::new(0),
            validation_errors: AtomicUsize::new(0),
        }
    }

    /// Initialize the start timestamp for tracking
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn initialize_timestamp(&mut self) -> Result<(), ResourceError> {
        self.start_timestamp = self
            .timestamp_service
            .system_time()
            .await
            .map(|st| {
                st.duration_since(std::time::UNIX_EPOCH)
                    .map(|d| i64::try_from(d.as_secs()).unwrap_or(0))
                    .unwrap_or(0)
            })
            .map_err(|e| ResourceError::ValidationError {
                message: format!("Failed to get initial timestamp: {e}"),
            })?;
        Ok(())
    }

    /// Check if validation has timed out
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn check_timeout(&self) -> Result<(), ResourceError> {
        let current_timestamp = self.timestamp_service.system_time().await.map_or_else(
            |_| self.start_timestamp,
            |st| {
                // Timestamp values are within i64 range for reasonable time periods
                
                st
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            },
        ); // Use start if service fails

        let elapsed_ms = (current_timestamp - self.start_timestamp) as u64;
        let elapsed = Duration::from_millis(elapsed_ms);

        if elapsed > self.limits.max_validation_time {
            return Err(ResourceError::Timeout {
                elapsed: elapsed.as_secs_f64(),
                max: self.limits.max_validation_time.as_secs_f64(),
            });
        }
        Ok(())
    }

    /// Check if expression evaluation has timed out
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn check_expression_timeout(
        &self,
        start_timestamp: i64,
    ) -> Result<(), ResourceError> {
        let current_timestamp = self.timestamp_service.system_time().await.map_or_else(
            |_| start_timestamp,
            |st| {
                // Timestamp values are within i64 range for reasonable time periods
                
                st
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            },
        ); // Use start if service fails

        let elapsed_ms = (current_timestamp - start_timestamp) as u64;
        let elapsed = Duration::from_millis(elapsed_ms);

        if elapsed > self.limits.max_expression_time {
            return Err(ResourceError::Timeout {
                elapsed: elapsed.as_secs_f64(),
                max: self.limits.max_expression_time.as_secs_f64(),
            });
        }
        Ok(())
    }

    /// Track memory allocation
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ResourceError::MemoryExceeded` if the allocation would exceed maximum memory usage
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
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ResourceError::TooManyParallelOps` if the maximum number of parallel operations is exceeded
    pub fn start_parallel_op(&self) -> Result<ParallelOpGuard<'_>, ResourceError> {
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
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ResourceError::CacheMemoryExceeded` if the allocation would exceed maximum cache memory usage
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
        let current_timestamp = std::time::SystemTime::now();
        // Time difference calculation, result is always positive
        let elapsed_secs = (current_timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
            - self.start_timestamp) as u64;

        ResourceUsage {
            elapsed: std::time::Duration::from_secs(elapsed_secs),
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

impl Drop for ParallelOpGuard<'_> {
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
    #[must_use]
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
pub fn create_monitor(
    limits: ResourceLimits,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> SharedResourceMonitor {
    Arc::new(ResourceMonitor::new(limits, timestamp))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_timeout_check() {
        let limits = ResourceLimits {
            max_validation_time: Duration::from_millis(100),
            ..Default::default()
        };
        let timestamp_service = Arc::new(timestamp_service::wiring::wire_timestamp());
        let mut monitor = ResourceMonitor::new(limits, timestamp_service);

        // Initialize the timestamp
        monitor
            .initialize_timestamp()
            .await
            .expect("Failed to initialize timestamp");

        // Should not timeout immediately
        assert!(monitor.check_timeout().await.is_ok());

        // Sleep and check timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(matches!(
            monitor.check_timeout().await,
            Err(ResourceError::Timeout { .. })
        ));
    }

    #[test]
    fn test_memory_tracking() {
        let timestamp_service = Arc::new(timestamp_service::wiring::wire_timestamp());
        let monitor = ResourceMonitor::new(ResourceLimits::default(), timestamp_service);

        // Allocate some memory
        assert!(monitor.allocate_memory(1000).is_ok());
        assert_eq!(monitor.current_usage().memory_used, 1000);

        // Release memory
        monitor.release_memory(500);
        assert_eq!(monitor.current_usage().memory_used, 500);
    }

    #[test]
    fn test_parallel_ops() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let limits = ResourceLimits {
            max_parallel_validators: 2,
            ..Default::default()
        };
        let timestamp_service = Arc::new(timestamp_service::wiring::wire_timestamp());
        let monitor = Arc::new(ResourceMonitor::new(limits, timestamp_service));

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
        Ok(())
    }
}
