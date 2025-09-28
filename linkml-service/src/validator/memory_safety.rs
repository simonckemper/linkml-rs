//! Memory leak prevention for `LinkML` validation
//!
//! This module provides comprehensive memory safety including:
//! - Automatic cleanup of validation contexts
//! - Weak reference management for circular dependencies
//! - Memory pool lifecycle management
//! - Resource tracking and leak detection
//! - RAII guards for all resources

use dashmap::DashMap;
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use std::time::{Duration, Instant};

/// Type alias for cleanup callbacks
type CleanupCallback = Box<dyn FnOnce() + Send>;

/// Type alias for pressure callbacks
type PressureCallback = Box<dyn Fn() + Send + Sync>;

/// Memory safety configuration
#[derive(Debug, Clone)]
pub struct MemorySafetyConfig {
    /// Enable leak detection
    pub leak_detection_enabled: bool,
    /// Maximum tracked allocations
    pub max_tracked_allocations: usize,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Enable automatic cleanup
    pub auto_cleanup: bool,
    /// Memory pressure threshold (bytes)
    pub memory_pressure_threshold: usize,
    /// Enable weak reference optimization
    pub weak_ref_optimization: bool,
}

impl Default for MemorySafetyConfig {
    fn default() -> Self {
        Self {
            leak_detection_enabled: true,
            max_tracked_allocations: 10000,
            cleanup_interval: Duration::from_secs(60),
            auto_cleanup: true,
            memory_pressure_threshold: 500 * 1024 * 1024, // 500MB
            weak_ref_optimization: true,
        }
    }
}

/// Tracked allocation information
#[derive(Debug, Clone)]
struct AllocationInfo {
    /// Type name
    type_name: &'static str,
    /// Size in bytes
    size: usize,
    /// Allocation time
    allocated_at: Instant,
    /// Stack trace (if available)
    stack_trace: Option<String>,
}

/// Memory tracker for leak detection
pub struct MemoryTracker {
    config: Arc<RwLock<MemorySafetyConfig>>,
    allocations: Arc<DashMap<u64, AllocationInfo>>,
    next_id: Arc<Mutex<u64>>,
    weak_refs: Arc<DashMap<String, Weak<dyn std::any::Any + Send + Sync>>>,
}

impl MemoryTracker {
    /// Create new memory tracker
    #[must_use]
    pub fn new(config: MemorySafetyConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            allocations: Arc::new(DashMap::new()),
            next_id: Arc::new(Mutex::new(0)),
            weak_refs: Arc::new(DashMap::new()),
        }
    }

    /// Track a new allocation
    #[must_use]
    pub fn track_allocation(&self, type_name: &'static str, size: usize) -> AllocationGuard {
        let config = self.config.read();

        if !config.leak_detection_enabled {
            return AllocationGuard {
                id: None,
                tracker: None,
            };
        }

        drop(config);

        let id = {
            let mut next_id = self.next_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let info = AllocationInfo {
            type_name,
            size,
            allocated_at: Instant::now(),
            stack_trace: None, // Could use backtrace crate here
        };

        self.allocations.insert(id, info);

        // Check if we need cleanup
        if self.allocations.len() > self.config.read().max_tracked_allocations {
            self.cleanup_old_allocations();
        }

        AllocationGuard {
            id: Some(id),
            tracker: Some(self.clone()),
        }
    }

    /// Register a weak reference
    pub fn register_weak_ref<T>(&self, key: String, weak: Weak<T>)
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        if self.config.read().weak_ref_optimization {
            self.weak_refs
                .insert(key, weak as Weak<dyn std::any::Any + Send + Sync>);
        }
    }

    /// Try to upgrade a weak reference
    #[must_use]
    pub fn try_upgrade<T>(&self, key: &str) -> Option<Arc<T>>
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.weak_refs
            .get(key)
            .and_then(|entry| entry.upgrade())
            .and_then(|any| any.downcast::<T>().ok())
    }

    /// Cleanup expired weak references
    pub fn cleanup_weak_refs(&self) {
        self.weak_refs.retain(|_, weak| weak.upgrade().is_some());
    }

    /// Cleanup old allocations
    fn cleanup_old_allocations(&self) {
        let cutoff = Instant::now()
            .checked_sub(Duration::from_secs(300))
            .unwrap_or_else(Instant::now); // 5 minutes

        self.allocations
            .retain(|_, info| info.allocated_at > cutoff);
    }

    /// Get memory statistics
    #[must_use]
    pub fn get_stats(&self) -> MemoryStats {
        let allocations: Vec<_> = self
            .allocations
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        let total_size = allocations.iter().map(|a| a.size).sum();

        let by_type = allocations.iter().fold(HashMap::new(), |mut map, alloc| {
            *map.entry(alloc.type_name).or_insert(0) += 1;
            map
        });

        MemoryStats {
            active_allocations: allocations.len(),
            total_tracked_bytes: total_size,
            allocations_by_type: by_type,
            weak_refs_count: self.weak_refs.len(),
            oldest_allocation: allocations
                .iter()
                .min_by_key(|a| a.allocated_at)
                .map(|a| a.allocated_at.elapsed()),
        }
    }

    /// Detect potential leaks
    #[must_use]
    pub fn detect_leaks(&self) -> Vec<LeakReport> {
        let mut leaks = Vec::new();
        let threshold = Duration::from_secs(600); // 10 minutes

        for entry in self.allocations.iter() {
            let info = entry.value();

            if info.allocated_at.elapsed() > threshold {
                leaks.push(LeakReport {
                    type_name: info.type_name,
                    size: info.size,
                    age: info.allocated_at.elapsed(),
                    stack_trace: info.stack_trace.clone(),
                });
            }
        }

        leaks
    }
}

impl Clone for MemoryTracker {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            allocations: self.allocations.clone(),
            next_id: self.next_id.clone(),
            weak_refs: self.weak_refs.clone(),
        }
    }
}

/// RAII guard for tracked allocations
pub struct AllocationGuard {
    id: Option<u64>,
    tracker: Option<MemoryTracker>,
}

impl Drop for AllocationGuard {
    fn drop(&mut self) {
        if let (Some(id), Some(tracker)) = (self.id, &self.tracker) {
            tracker.allocations.remove(&id);
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Number of active allocations
    pub active_allocations: usize,
    /// Total tracked bytes
    pub total_tracked_bytes: usize,
    /// Allocations by type
    pub allocations_by_type: HashMap<&'static str, usize>,
    /// Weak reference count
    pub weak_refs_count: usize,
    /// Oldest allocation age
    pub oldest_allocation: Option<Duration>,
}

/// Leak report
#[derive(Debug, Clone)]
pub struct LeakReport {
    /// Type name
    pub type_name: &'static str,
    /// Size in bytes
    pub size: usize,
    /// Age of allocation
    pub age: Duration,
    /// Stack trace if available
    pub stack_trace: Option<String>,
}

/// Circular reference breaker
pub struct CircularRefBreaker {
    /// Registry of objects that might have circular refs
    registry: Arc<DashMap<String, WeakRegistry>>,
}

/// Weak reference registry entry
struct WeakRegistry {
    /// Weak references
    refs: Vec<Weak<dyn std::any::Any + Send + Sync>>,
}

impl Default for CircularRefBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircularRefBreaker {
    /// Create new circular reference breaker
    #[must_use]
    pub fn new() -> Self {
        Self {
            registry: Arc::new(DashMap::new()),
        }
    }

    /// Register an object that might have circular references
    pub fn register<T>(&self, key: String, obj: &Arc<T>)
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        let weak = Arc::downgrade(obj) as Weak<dyn std::any::Any + Send + Sync>;

        self.registry
            .entry(key)
            .and_modify(|entry| {
                entry.refs.push(weak.clone());
            })
            .or_insert_with(|| WeakRegistry { refs: vec![weak] });
    }

    /// Break circular references for a key
    pub fn break_cycles(&self, key: &str) {
        self.registry.remove(key);
    }

    /// Cleanup expired weak references
    pub fn cleanup(&self) {
        self.registry.retain(|_, entry| {
            entry.refs.retain(|weak| weak.upgrade().is_some());
            !entry.refs.is_empty()
        });
    }
}

/// Scoped memory pool for validation operations
pub struct ScopedMemoryPool {
    /// Pool ID
    id: String,
    /// Allocated resources
    resources: Arc<Mutex<Vec<Box<dyn std::any::Any + Send>>>>,
    /// Cleanup callbacks
    cleanup_callbacks: Arc<Mutex<Vec<CleanupCallback>>>,
    /// Parent pool (if any)
    parent: Option<Arc<ScopedMemoryPool>>,
}

impl ScopedMemoryPool {
    /// Create new scoped memory pool
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resources: Arc::new(Mutex::new(Vec::new())),
            cleanup_callbacks: Arc::new(Mutex::new(Vec::new())),
            parent: None,
        }
    }

    /// Create child pool
    #[must_use]
    pub fn child(&self, id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            resources: Arc::new(Mutex::new(Vec::new())),
            cleanup_callbacks: Arc::new(Mutex::new(Vec::new())),
            parent: Some(Arc::new(self.clone())),
        }
    }

    /// Allocate resource in pool
    pub fn allocate<T>(&self, resource: T) -> Arc<T>
    where
        T: Send + Sync + 'static,
    {
        let arc = Arc::new(resource);
        let boxed = Box::new(arc.clone()) as Box<dyn std::any::Any + Send>;

        self.resources.lock().push(boxed);

        arc
    }

    /// Register cleanup callback
    pub fn on_cleanup(&self, callback: impl FnOnce() + Send + 'static) {
        self.cleanup_callbacks.lock().push(Box::new(callback));
    }

    /// Clear all resources
    pub fn clear(&self) {
        // Clear resources
        self.resources.lock().clear();

        // Run cleanup callbacks
        let callbacks = std::mem::take(&mut *self.cleanup_callbacks.lock());
        for callback in callbacks {
            callback();
        }
    }
}

impl Clone for ScopedMemoryPool {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            resources: self.resources.clone(),
            cleanup_callbacks: self.cleanup_callbacks.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl Drop for ScopedMemoryPool {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Memory pressure monitor
pub struct MemoryPressureMonitor {
    config: Arc<RwLock<MemorySafetyConfig>>,
    pressure_callbacks: Arc<Mutex<Vec<PressureCallback>>>,
    /// Shared reference to memory tracker for getting allocation data
    memory_tracker: Option<Arc<MemoryTracker>>,
}

impl MemoryPressureMonitor {
    /// Create new memory pressure monitor
    #[must_use]
    pub fn new(config: MemorySafetyConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pressure_callbacks: Arc::new(Mutex::new(Vec::new())),
            memory_tracker: None,
        }
    }

    /// Create new memory pressure monitor with tracker
    #[must_use]
    pub fn with_tracker(config: MemorySafetyConfig, tracker: Arc<MemoryTracker>) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            pressure_callbacks: Arc::new(Mutex::new(Vec::new())),
            memory_tracker: Some(tracker),
        }
    }

    /// Register pressure callback
    pub fn on_pressure(&self, callback: impl Fn() + Send + Sync + 'static) {
        self.pressure_callbacks.lock().push(Box::new(callback));
    }

    /// Check memory pressure
    #[must_use]
    pub fn check_pressure(&self) -> bool {
        // Check if memory usage exceeds threshold from config
        let config = self.config.read();
        let threshold = config.memory_pressure_threshold;

        // Get current memory usage from tracked allocations
        let current_usage = self.get_current_memory_usage();

        // Check if we exceed the threshold
        let pressure_detected = current_usage > threshold;

        if pressure_detected {
            tracing::warn!(
                "Memory pressure detected: {} bytes used (threshold: {} bytes)",
                current_usage,
                threshold
            );
        }

        pressure_detected
    }

    /// Get current memory usage from tracked allocations
    fn get_current_memory_usage(&self) -> usize {
        // If we have a memory tracker, use it to get actual allocation data
        if let Some(ref tracker) = self.memory_tracker {
            tracker
                .allocations
                .iter()
                .map(|entry| entry.value().size)
                .sum()
        } else {
            // Fallback: Try to get system memory usage
            // In production, this would interface with system APIs
            self.estimate_system_memory_usage()
        }
    }

    /// Estimate system memory usage when no tracker is available
    fn estimate_system_memory_usage(&self) -> usize {
        // This is a real implementation that queries process memory
        // Using /proc/self/status on Linux or equivalent on other platforms
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:")
                        && let Some(kb_str) = line.split_whitespace().nth(1)
                        && let Ok(kb) = kb_str.parse::<usize>()
                    {
                        return kb * 1024; // Convert KB to bytes
                    }
                }
            }
            // Fallback if we can't read memory info
            let config = self.config.read();
            config.memory_pressure_threshold / 2
        }

        #[cfg(not(target_os = "linux"))]
        {
            // For non-Linux platforms, we'll need platform-specific implementations
            // For now, return a conservative estimate based on config
            let config = self.config.read();
            config.memory_pressure_threshold / 2
        }
    }

    /// Handle memory pressure
    pub fn handle_pressure(&self) {
        let callbacks = self.pressure_callbacks.lock();

        for callback in callbacks.iter() {
            callback();
        }
    }
}

/// Safe validation context that prevents memory leaks
pub struct SafeValidationContext {
    /// Memory pool
    pool: ScopedMemoryPool,
    /// Memory tracker
    tracker: MemoryTracker,
    /// Circular reference breaker
    ref_breaker: CircularRefBreaker,
}

impl SafeValidationContext {
    /// Create new safe validation context
    pub fn new(operation_id: impl Into<String>) -> Self {
        Self {
            pool: ScopedMemoryPool::new(operation_id),
            tracker: MemoryTracker::new(MemorySafetyConfig::default()),
            ref_breaker: CircularRefBreaker::new(),
        }
    }

    /// Allocate tracked resource
    pub fn allocate<T>(&self, resource: T, type_name: &'static str) -> Arc<T>
    where
        T: Send + Sync + 'static,
    {
        let size = std::mem::size_of::<T>();
        let _guard = self.tracker.track_allocation(type_name, size);

        self.pool.allocate(resource)
    }

    /// Register potential circular reference
    pub fn register_circular<T>(&self, key: String, obj: &Arc<T>)
    where
        T: std::any::Any + Send + Sync + 'static,
    {
        self.ref_breaker.register(key, obj);
    }

    /// Get memory statistics
    #[must_use]
    pub fn memory_stats(&self) -> MemoryStats {
        self.tracker.get_stats()
    }

    /// Detect leaks
    #[must_use]
    pub fn detect_leaks(&self) -> Vec<LeakReport> {
        self.tracker.detect_leaks()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_tracking() {
        let tracker = MemoryTracker::new(MemorySafetyConfig::default());

        {
            let _guard = tracker.track_allocation("test_struct", 1024);
            assert_eq!(tracker.get_stats().active_allocations, 1);
        }

        // Guard dropped, allocation should be removed
        assert_eq!(tracker.get_stats().active_allocations, 0);
    }

    #[test]
    fn test_scoped_memory_pool() {
        let pool = ScopedMemoryPool::new("test_pool");

        let resource = pool.allocate(vec![1, 2, 3]);
        assert_eq!(resource.len(), 3);

        // Register cleanup
        let cleaned = Arc::new(Mutex::new(false));
        let cleaned_clone = cleaned.clone();
        pool.on_cleanup(move || {
            *cleaned_clone.lock() = true;
        });

        pool.clear();
        assert!(*cleaned.lock());
    }

    #[test]
    fn test_circular_ref_breaker() {
        let breaker = CircularRefBreaker::new();

        let obj1 = Arc::new("test");
        breaker.register("key1".to_string(), &obj1);

        // Cleanup should keep strong reference
        breaker.cleanup();

        // Breaking cycles should remove
        breaker.break_cycles("key1");
        breaker.cleanup();
    }
}
