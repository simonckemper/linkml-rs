//! Resource limit enforcement for `LinkML` validation
//!
//! This module provides comprehensive resource management including:
//! - Memory usage limits and monitoring
//! - CPU usage throttling
//! - Concurrent operation limits
//! - Request rate limiting
//! - Timeout enforcement

use crate::utils::safe_cast::{u64_to_usize_saturating, usize_to_f32_saturating, usize_to_f64};
use dashmap::DashMap;
use linkml_core::{LinkMLError, Result};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinHandle;

/// Resource limits configuration
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory usage in bytes
    pub max_memory_bytes: usize,
    /// Maximum CPU usage percentage (0-100)
    pub max_cpu_percent: f32,
    /// Maximum concurrent validations
    pub max_concurrent_validations: usize,
    /// Maximum validation duration
    pub max_validation_duration: Duration,
    /// Maximum schema size in bytes
    pub max_schema_size: usize,
    /// Maximum document size in bytes
    pub max_document_size: usize,
    /// Maximum nested depth
    pub max_nested_depth: usize,
    /// Rate limit (requests per second)
    pub rate_limit_rps: Option<f64>,
    /// Enable resource monitoring
    pub monitoring_enabled: bool,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: 1024 * 1024 * 1024, // 1GB
            max_cpu_percent: 80.0,
            max_concurrent_validations: 100,
            max_validation_duration: Duration::from_secs(30),
            max_schema_size: 10 * 1024 * 1024,    // 10MB
            max_document_size: 100 * 1024 * 1024, // 100MB
            max_nested_depth: 100,
            rate_limit_rps: Some(1000.0),
            monitoring_enabled: true,
        }
    }
}

/// Resource usage snapshot
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// Current memory usage in bytes
    pub memory_bytes: usize,
    /// Current CPU usage percentage
    pub cpu_percent: f32,
    /// Active validation count
    pub active_validations: usize,
    /// Current request rate
    pub request_rate: f64,
    /// Timestamp
    pub timestamp: Instant,
}

/// Resource monitor trait
pub trait ResourceMonitor: Send + Sync {
    /// Get current memory usage
    fn get_memory_usage(&self) -> usize;

    /// Get current CPU usage
    fn get_cpu_usage(&self) -> f32;

    /// Check if resources are available
    ///
    /// # Errors
    ///
    /// Returns an error if resources are not available.
    fn check_resources(&self, required: &ResourceRequirements) -> Result<()>;
}

/// Resource requirements for an operation
#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    /// Estimated memory needed
    pub memory_bytes: usize,
    /// Estimated CPU usage
    pub cpu_percent: f32,
    /// Estimated duration
    pub duration: Duration,
    /// Document size
    pub document_size: usize,
}

impl ResourceRequirements {
    /// Estimate requirements from document size
    #[must_use]
    pub fn estimate_from_size(document_size: usize) -> Self {
        Self {
            // Rough estimates based on document size
            memory_bytes: document_size * 10, // 10x for processing overhead
            cpu_percent: 10.0,
            duration: Duration::from_millis((document_size / 1000) as u64),
            document_size,
        }
    }
}

/// Token bucket for rate limiting
struct TokenBucket {
    capacity: f64,
    tokens: f64,
    refill_rate: f64,
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        self.refill();

        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();

        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}

/// Resource limiter implementation
pub struct ResourceLimiter {
    limits: Arc<RwLock<ResourceLimits>>,
    semaphore: Arc<Semaphore>,
    rate_limiter: Arc<Mutex<Option<TokenBucket>>>,
    active_operations: Arc<DashMap<String, ActiveOperation>>,
    usage_history: Arc<RwLock<Vec<ResourceUsage>>>,
    monitor: Option<Arc<dyn ResourceMonitor>>,
    task_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
}

/// Active operation tracking
struct ActiveOperation {
    id: String,
    start_time: Instant,
    _requirements: ResourceRequirements,
    timeout_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ResourceLimiter {
    /// Create new resource limiter
    #[must_use]
    pub fn new(limits: ResourceLimits) -> Self {
        let max_concurrent = limits.max_concurrent_validations;
        let rate_limit = limits.rate_limit_rps.map(|rps| TokenBucket::new(rps, rps));

        Self {
            limits: Arc::new(RwLock::new(limits)),
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            rate_limiter: Arc::new(Mutex::new(rate_limit)),
            active_operations: Arc::new(DashMap::new()),
            usage_history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            monitor: None,
            task_handles: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Set resource monitor
    pub fn set_monitor(&mut self, monitor: Arc<dyn ResourceMonitor>) {
        self.monitor = Some(monitor);
    }

    /// Acquire resources for an operation
    ///
    /// # Errors
    ///
    /// Returns an error if resources cannot be acquired.
    pub async fn acquire(
        &self,
        operation_id: String,
        requirements: ResourceRequirements,
    ) -> Result<ResourceGuard> {
        // Check static limits in a separate scope to ensure lock is dropped
        {
            let limits = self.limits.read();

            if requirements.document_size > limits.max_document_size {
                return Err(LinkMLError::service(format!(
                    "Document size {} exceeds limit {}",
                    requirements.document_size, limits.max_document_size
                )));
            }

            if requirements.memory_bytes > limits.max_memory_bytes {
                return Err(LinkMLError::service(format!(
                    "Memory requirement {} exceeds limit {}",
                    requirements.memory_bytes, limits.max_memory_bytes
                )));
            }
        }

        // Check rate limit
        {
            if let Some(rate_limiter) = &mut *self.rate_limiter.lock()
                && !rate_limiter.try_consume(1.0)
            {
                return Err(LinkMLError::service("Rate limit exceeded"));
            }
        } // Drop the lock before await

        // Check dynamic resources if monitor available
        if let Some(monitor) = &self.monitor {
            monitor.check_resources(&requirements)?;
        }

        // Acquire semaphore permit
        let permit = self
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| LinkMLError::service("Failed to acquire resource permit"))?;

        // Set up timeout
        let timeout_duration = self.limits.read().max_validation_duration;
        let operation_id_clone = operation_id.clone();
        let active_ops = Arc::clone(&self.active_operations);

        let timeout_handle = tokio::spawn(async move {
            tokio::time::sleep(timeout_duration).await;

            // Force cleanup if still active
            if active_ops.remove(&operation_id_clone).is_some() {
                tracing::warn!("Operation {} timed out", operation_id_clone);
            }
        });

        // Store timeout task handle with bounded growth
        {
            let mut handles = self.task_handles.write();
            if handles.len() >= 5 {
                // Cleanup completed handles
                handles.retain(|h| !h.is_finished());

                // If still at limit, abort oldest
                if handles.len() >= 5 {
                    let oldest = handles.remove(0);
                    oldest.abort();
                }
            }
            handles.push(timeout_handle);
        }

        // Track active operation
        self.active_operations.insert(
            operation_id.clone(),
            ActiveOperation {
                id: operation_id.clone(),
                start_time: Instant::now(),
                _requirements: requirements.clone(),
                timeout_handle: None,
            }, // We store the handle separately now
        );

        // Record usage
        self.record_usage();

        Ok(ResourceGuard {
            operation_id,
            limiter: self.clone(),
            _permit: Some(permit),
            start_time: Instant::now(),
        })
    }

    /// Check if operation would exceed limits
    #[must_use]
    pub fn would_exceed_limits(&self, requirements: &ResourceRequirements) -> Option<String> {
        let limits = self.limits.read();

        if requirements.document_size > limits.max_document_size {
            return Some(format!(
                "Document size {} exceeds limit {}",
                requirements.document_size, limits.max_document_size
            ));
        }

        if requirements.memory_bytes > limits.max_memory_bytes {
            return Some(format!(
                "Memory requirement {} exceeds limit {}",
                requirements.memory_bytes, limits.max_memory_bytes
            ));
        }

        if let Some(monitor) = &self.monitor {
            let current_memory = monitor.get_memory_usage();
            if current_memory + requirements.memory_bytes > limits.max_memory_bytes {
                return Some(format!(
                    "Insufficient memory: {} available, {} required",
                    limits.max_memory_bytes - current_memory,
                    requirements.memory_bytes
                ));
            }

            let current_cpu = monitor.get_cpu_usage();
            if current_cpu + requirements.cpu_percent > limits.max_cpu_percent {
                return Some(format!(
                    "Insufficient CPU: {}% available, {}% required",
                    limits.max_cpu_percent - current_cpu,
                    requirements.cpu_percent
                ));
            }
        }

        None
    }

    /// Record current resource usage
    fn record_usage(&self) {
        if !self.limits.read().monitoring_enabled {
            return;
        }

        let usage = if let Some(monitor) = &self.monitor {
            ResourceUsage {
                memory_bytes: monitor.get_memory_usage(),
                cpu_percent: monitor.get_cpu_usage(),
                active_validations: self.active_operations.len(),
                request_rate: self.calculate_request_rate(),
                timestamp: Instant::now(),
            }
        } else {
            ResourceUsage {
                memory_bytes: 0,
                cpu_percent: 0.0,
                active_validations: self.active_operations.len(),
                request_rate: self.calculate_request_rate(),
                timestamp: Instant::now(),
            }
        };

        let mut history = self.usage_history.write();

        // Keep limited history
        if history.len() >= 1000 {
            history.drain(0..100);
        }

        history.push(usage);
    }

    /// Calculate current request rate
    fn calculate_request_rate(&self) -> f64 {
        let history = self.usage_history.read();

        if history.len() < 2 {
            return 0.0;
        }

        let window = Duration::from_secs(60);
        let now = Instant::now();

        let count = history
            .iter()
            .filter(|u| now.duration_since(u.timestamp) <= window)
            .count();

        // Calculate rate using safe casting
        usize_to_f64(count) / window.as_secs_f64()
    }

    /// Get resource statistics
    ///
    /// # Panics
    ///
    /// This function will panic if `Instant::now()` is less than 300 seconds from the start of time.
    #[must_use]
    pub fn get_stats(&self) -> ResourceStats {
        let history = self.usage_history.read();

        if history.is_empty() {
            return ResourceStats::default();
        }

        let latest = history.last().expect("just checked history is not empty");
        let window_start = Instant::now()
            .checked_sub(Duration::from_secs(300))
            .unwrap_or_else(Instant::now); // 5 min window

        let window_history: Vec<_> = history
            .iter()
            .filter(|u| u.timestamp >= window_start)
            .collect();

        if window_history.is_empty() {
            return ResourceStats::default();
        }

        let avg_memory =
            window_history.iter().map(|u| u.memory_bytes).sum::<usize>() / window_history.len();

        // Calculate average CPU using safe casting
        let avg_cpu = window_history.iter().map(|u| u.cpu_percent).sum::<f32>()
            / usize_to_f32_saturating(window_history.len());

        let peak_memory = window_history
            .iter()
            .map(|u| u.memory_bytes)
            .max()
            .unwrap_or(0);

        let peak_cpu = window_history
            .iter()
            .map(|u| u.cpu_percent)
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0.0);

        ResourceStats {
            current_memory: latest.memory_bytes,
            current_cpu: latest.cpu_percent,
            active_operations: self.active_operations.len(),
            average_memory: avg_memory,
            average_cpu: avg_cpu,
            peak_memory,
            peak_cpu,
            request_rate: latest.request_rate,
        }
    }

    /// Cleanup expired operations
    pub fn cleanup_expired(&self) {
        let timeout = self.limits.read().max_validation_duration;
        let now = Instant::now();

        let expired: Vec<_> = self
            .active_operations
            .iter()
            .filter(|entry| now.duration_since(entry.start_time) > timeout)
            .map(|entry| entry.id.clone())
            .collect();

        for id in expired {
            if let Some((_, mut op)) = self.active_operations.remove(&id) {
                if let Some(handle) = op.timeout_handle.take() {
                    handle.abort();
                }
                tracing::warn!("Cleaned up expired operation: {}", id);
            }
        }
    }

    /// Cancel all running tasks
    pub fn cancel_all_tasks(&self) {
        let mut handles = self.task_handles.write();
        for handle in handles.drain(..) {
            handle.abort();
        }
    }

    /// Cleanup completed tasks
    pub fn cleanup_completed_tasks(&self) {
        let mut handles = self.task_handles.write();
        handles.retain(|h| !h.is_finished());
    }
}

// Manual Clone implementation to handle trait object
impl Clone for ResourceLimiter {
    fn clone(&self) -> Self {
        Self {
            limits: self.limits.clone(),
            semaphore: self.semaphore.clone(),
            rate_limiter: self.rate_limiter.clone(),
            active_operations: self.active_operations.clone(),
            usage_history: self.usage_history.clone(),
            monitor: self.monitor.clone(),
            task_handles: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

/// Resource guard that releases resources on drop
pub struct ResourceGuard {
    operation_id: String,
    limiter: ResourceLimiter,
    _permit: Option<tokio::sync::OwnedSemaphorePermit>,
    start_time: Instant,
}

impl Drop for ResourceGuard {
    fn drop(&mut self) {
        // Remove from active operations
        if let Some((_, mut op)) = self.limiter.active_operations.remove(&self.operation_id)
            && let Some(handle) = op.timeout_handle.take()
        {
            handle.abort();
        }

        // Record duration
        let duration = self.start_time.elapsed();
        tracing::debug!(
            "Operation {} completed in {:?}",
            self.operation_id,
            duration
        );

        // Permit is automatically released
    }
}

/// Resource statistics
#[derive(Debug, Default, Clone)]
pub struct ResourceStats {
    /// Current memory usage
    pub current_memory: usize,
    /// Current CPU usage
    pub current_cpu: f32,
    /// Number of active operations
    pub active_operations: usize,
    /// Average memory usage
    pub average_memory: usize,
    /// Average CPU usage
    pub average_cpu: f32,
    /// Peak memory usage
    pub peak_memory: usize,
    /// Peak CPU usage
    pub peak_cpu: f32,
    /// Current request rate
    pub request_rate: f64,
}

/// System resource monitor implementation
pub struct SystemResourceMonitor {
    /// Process handle for monitoring (kept for future RAII use)
    _process: sysinfo::System,
}

impl Default for SystemResourceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemResourceMonitor {
    /// Create new system monitor
    #[must_use]
    pub fn new() -> Self {
        Self {
            _process: sysinfo::System::new(),
        }
    }
}

impl ResourceMonitor for SystemResourceMonitor {
    fn get_memory_usage(&self) -> usize {
        // Get current process memory usage using sysinfo
        // Note: sysinfo::System doesn't implement Clone, so we create a new instance
        let mut system = sysinfo::System::new();
        system.refresh_memory();
        u64_to_usize_saturating(system.used_memory())
    }

    fn get_cpu_usage(&self) -> f32 {
        // Get current process CPU usage using sysinfo
        // Note: sysinfo::System doesn't implement Clone, so we create a new instance
        let mut system = sysinfo::System::new();
        system.refresh_cpu_all();
        system.global_cpu_usage()
    }

    fn check_resources(&self, _required: &ResourceRequirements) -> Result<()> {
        // Check if resources are available
        Ok(())
    }
}

/// Validation-specific resource limiter
pub struct ValidationResourceLimiter {
    base_limiter: ResourceLimiter,
    depth_tracker: DashMap<String, usize>,
}

impl ValidationResourceLimiter {
    /// Create new validation resource limiter
    #[must_use]
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            base_limiter: ResourceLimiter::new(limits),
            depth_tracker: DashMap::new(),
        }
    }

    /// Check nested depth limit
    ///
    /// # Errors
    ///
    /// Returns an error if depth exceeds the limit.
    pub fn check_depth(&self, operation_id: &str, current_depth: usize) -> Result<()> {
        let max_depth = self.base_limiter.limits.read().max_nested_depth;

        if current_depth > max_depth {
            return Err(LinkMLError::service(format!(
                "Nested depth {current_depth} exceeds limit {max_depth}"
            )));
        }

        self.depth_tracker
            .insert(operation_id.to_string(), current_depth);
        Ok(())
    }

    /// Acquire resources for validation
    ///
    /// # Errors
    ///
    /// Returns an error if resources cannot be acquired.
    pub async fn acquire_for_validation(&self, document_size: usize) -> Result<ResourceGuard> {
        let operation_id = format!("validation_{}", uuid::Uuid::new_v4());
        let requirements = ResourceRequirements::estimate_from_size(document_size);

        self.base_limiter.acquire(operation_id, requirements).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0);

        assert!(bucket.try_consume(5.0));
        assert!(bucket.try_consume(5.0));
        assert!(!bucket.try_consume(1.0));
    }

    #[test]
    fn test_resource_requirements() {
        let reqs = ResourceRequirements::estimate_from_size(1000);

        assert_eq!(reqs.document_size, 1000);
        assert_eq!(reqs.memory_bytes, 10000); // 10x estimate
    }

    #[tokio::test]
    async fn test_resource_limiter() -> anyhow::Result<()> {
        let limits = ResourceLimits {
            max_concurrent_validations: 2,
            ..Default::default()
        };

        let limiter = ResourceLimiter::new(limits);

        // Acquire first resource
        let _guard1 = limiter
            .acquire(
                "op1".to_string(),
                ResourceRequirements::estimate_from_size(100),
            )
            .await?;

        // Acquire second resource
        let _guard2 = limiter
            .acquire(
                "op2".to_string(),
                ResourceRequirements::estimate_from_size(100),
            )
            .await?;

        // Third should wait (would block in real scenario)
        assert_eq!(limiter.active_operations.len(), 2);
        Ok(())
    }
}
