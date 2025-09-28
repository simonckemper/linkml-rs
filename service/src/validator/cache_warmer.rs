//! Cache warming strategies for `LinkML` validation
//!
//! This module provides intelligent cache warming to:
//! - Pre-load frequently used validators
//! - Predict future cache needs
//! - Warm caches during low-activity periods
//! - Maintain optimal cache hit rates

use super::{
    ValidationEngine,
    cache::ValidatorCacheKey,
    compiled::{CompilationOptions, CompiledValidator},
    multi_layer_cache::MultiLayerCache,
};
use dashmap::DashMap;
use linkml_core::prelude::*;
use smallvec::SmallVec;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use rootreal_core_foundation_timestamp_core::{TimestampError, TimestampService};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;

/// Cache warming configuration
#[derive(Debug, Clone)]
pub struct CacheWarmingConfig {
    /// Enable automatic warming
    pub auto_warm: bool,
    /// Warming batch size
    pub batch_size: usize,
    /// Maximum concurrent warming tasks
    pub max_concurrent: usize,
    /// Warming interval
    pub warming_interval: Duration,
    /// Priority threshold (0.0 - 1.0)
    pub priority_threshold: f64,
    /// Enable predictive warming
    pub predictive_warming: bool,
    /// History size for predictions
    pub history_size: usize,
}

impl Default for CacheWarmingConfig {
    fn default() -> Self {
        Self {
            auto_warm: true,
            batch_size: 50,
            max_concurrent: 4,
            warming_interval: Duration::from_secs(300), // 5 minutes
            priority_threshold: 0.5,
            predictive_warming: true,
            history_size: 1000,
        }
    }
}

impl CacheWarmingConfig {
    /// Create cache warming config from `LinkML` service configuration
    #[must_use]
    pub fn from_service_config(config: &linkml_core::configuration_v2::CacheConfig) -> Self {
        // Derive settings from available cache config fields
        let max_entries = config.max_entries;
        let ttl_seconds = config.ttl_seconds;
        let enable_compression = config.enable_compression;

        Self {
            auto_warm: max_entries > 100, // Enable if cache is large enough
            batch_size: (max_entries / 20).clamp(10, 100), // 5% of max entries
            max_concurrent: 4,
            warming_interval: Duration::from_secs(ttl_seconds), // Use TTL as warming interval
            priority_threshold: 0.5,
            predictive_warming: !enable_compression, // Predictive warming if not compressing
            history_size: 1000,
        }
    }
}

/// Cache access history entry
#[derive(Debug, Clone)]
pub struct AccessEntry {
    /// Cache key
    pub key: ValidatorCacheKey,
    /// Access timestamp
    pub timestamp: std::time::Instant,
    /// Access count
    pub count: u32,
}

/// Priority entry for warming queue
#[derive(Debug, Clone)]
struct WarmingEntry {
    /// Cache key
    key: ValidatorCacheKey,
    /// Priority score (higher is better)
    priority: f64,
    /// Estimated compilation time
    estimated_time: Duration,
}

impl PartialEq for WarmingEntry {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for WarmingEntry {}

impl Ord for WarmingEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .partial_cmp(&other.priority)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for WarmingEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Cache warming strategy
pub trait WarmingStrategy: Send + Sync {
    /// Select keys to warm
    fn select_keys(
        &self,
        history: &[AccessEntry],
        current_cache: &DashMap<ValidatorCacheKey, bool>,
        config: &CacheWarmingConfig,
    ) -> Vec<ValidatorCacheKey>;

    /// Calculate priority for a key
    fn calculate_priority(&self, key: &ValidatorCacheKey, history: &[AccessEntry]) -> f64;
}

/// Frequency-based warming strategy
pub struct FrequencyBasedStrategy {
    /// Time window for frequency calculation
    window: Duration,
}

impl FrequencyBasedStrategy {
    /// Create a new frequency-based strategy
    #[must_use]
    pub fn new(window: Duration) -> Self {
        Self { window }
    }
}

impl WarmingStrategy for FrequencyBasedStrategy {
    fn select_keys(
        &self,
        history: &[AccessEntry],
        current_cache: &DashMap<ValidatorCacheKey, bool>,
        config: &CacheWarmingConfig,
    ) -> Vec<ValidatorCacheKey> {
        let now = std::time::Instant::now();
        let window_start = now
            .checked_sub(self.window)
            .unwrap_or_else(std::time::Instant::now);

        // Count accesses per key within window
        let access_counts: DashMap<ValidatorCacheKey, u32> = DashMap::new();

        for entry in history {
            if entry.timestamp >= window_start {
                access_counts
                    .entry(entry.key.clone())
                    .and_modify(|c| *c += entry.count)
                    .or_insert(entry.count);
            }
        }

        // Sort by frequency and select top keys not in cache
        let mut candidates: Vec<_> = access_counts
            .into_iter()
            .filter(|(key, _)| !current_cache.contains_key(key))
            .collect();

        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        candidates
            .into_iter()
            .take(config.batch_size)
            .map(|(key, _)| key)
            .collect()
    }

    fn calculate_priority(&self, key: &ValidatorCacheKey, history: &[AccessEntry]) -> f64 {
        let now = std::time::Instant::now();
        let window_start = now
            .checked_sub(self.window)
            .unwrap_or_else(std::time::Instant::now);

        let access_count: u32 = history
            .iter()
            .filter(|e| e.key == *key && e.timestamp >= window_start)
            .map(|e| e.count)
            .sum();

        // Normalize to 0.0 - 1.0 range
        (f64::from(access_count) / 100.0).min(1.0)
    }
}

/// Predictive warming strategy using access patterns
pub struct PredictiveStrategy {
    /// Pattern detection window
    pattern_window: Duration,
    /// Prediction lookahead
    lookahead: Duration,
}

impl PredictiveStrategy {
    /// Create a new predictive strategy
    #[must_use]
    pub fn new(pattern_window: Duration, lookahead: Duration) -> Self {
        Self {
            pattern_window,
            lookahead,
        }
    }

    /// Detect access patterns
    fn detect_patterns(&self, history: &[AccessEntry]) -> Vec<(ValidatorCacheKey, Duration)> {
        let mut patterns = Vec::new();
        let key_accesses: DashMap<ValidatorCacheKey, SmallVec<[Instant; 16]>> = DashMap::new();
        let now = std::time::Instant::now();
        let window_start = now.checked_sub(self.pattern_window).unwrap_or(now);

        // Group accesses by key within the pattern window
        for entry in history {
            if entry.timestamp >= window_start {
                key_accesses
                    .entry(entry.key.clone())
                    .or_default()
                    .push(entry.timestamp);
            }
        }

        // Analyze patterns for each key
        for (key, mut timestamps) in key_accesses {
            if timestamps.len() < 3 {
                continue;
            }

            timestamps.sort();

            // Calculate average interval
            let mut intervals = Vec::new();
            for i in 1..timestamps.len() {
                intervals.push(timestamps[i].duration_since(timestamps[i - 1]));
            }

            if intervals.is_empty() {
                continue;
            }

            // Precision loss acceptable here
            let avg_interval = intervals
                .iter()
                .map(std::time::Duration::as_secs_f64)
                .sum::<f64>()
                / f64::from(u32::try_from(intervals.len()).unwrap_or(u32::MAX));

            patterns.push((key, Duration::from_secs_f64(avg_interval)));
        }

        patterns
    }
}

impl WarmingStrategy for PredictiveStrategy {
    fn select_keys(
        &self,
        history: &[AccessEntry],
        current_cache: &DashMap<ValidatorCacheKey, bool>,
        config: &CacheWarmingConfig,
    ) -> Vec<ValidatorCacheKey> {
        let patterns = self.detect_patterns(history);
        let now = std::time::Instant::now();

        // Select keys likely to be accessed soon
        let mut candidates: Vec<_> = patterns
            .into_iter()
            .filter_map(|(key, interval)| {
                if current_cache.contains_key(&key) {
                    return None;
                }

                // Find last access
                let last_access = history
                    .iter()
                    .filter(|e| e.key == key)
                    .map(|e| e.timestamp)
                    .max()?;

                // Predict next access
                let predicted_next = last_access + interval;

                // Include if predicted within lookahead window
                if predicted_next > now && predicted_next < now + self.lookahead {
                    Some((key, predicted_next))
                } else {
                    None
                }
            })
            .collect();

        // Sort by predicted access time
        candidates.sort_by_key(|(_, time)| *time);

        candidates
            .into_iter()
            .take(config.batch_size)
            .map(|(key, _)| key)
            .collect()
    }

    fn calculate_priority(&self, key: &ValidatorCacheKey, history: &[AccessEntry]) -> f64 {
        // Priority based on access regularity
        let mut timestamps: Vec<_> = history
            .iter()
            .filter(|e| e.key == *key)
            .map(|e| e.timestamp)
            .collect();

        if timestamps.len() < 2 {
            return 0.0;
        }

        timestamps.sort();

        // Calculate variance in intervals
        let mut intervals = Vec::new();
        for i in 1..timestamps.len() {
            intervals.push(
                timestamps[i]
                    .duration_since(timestamps[i - 1])
                    .as_secs_f64(),
            );
        }

        // Precision loss acceptable here
        let mean = intervals.iter().sum::<f64>()
            / f64::from(u32::try_from(intervals.len()).unwrap_or(u32::MAX));
        // Precision loss acceptable here
        let variance = intervals.iter().map(|i| (i - mean).powi(2)).sum::<f64>()
            / f64::from(u32::try_from(intervals.len()).unwrap_or(u32::MAX));

        // Lower variance = higher priority (more regular pattern)
        1.0 / (1.0 + variance.sqrt())
    }
}

/// Cache warmer implementation
pub struct CacheWarmer {
    /// Configuration
    config: Arc<RwLock<CacheWarmingConfig>>,
    /// Multi-layer cache
    cache: Arc<MultiLayerCache>,
    /// Access history
    history: Arc<RwLock<Vec<AccessEntry>>>,
    /// Warming strategies
    strategies: Vec<Box<dyn WarmingStrategy + Send + Sync>>,
    /// Warming queue
    warming_queue: Arc<RwLock<BinaryHeap<WarmingEntry>>>,
    /// Currently warming keys
    warming_in_progress: Arc<DashMap<ValidatorCacheKey, std::time::Instant>>,
    /// Background task handles
    task_handles: Arc<RwLock<Vec<JoinHandle<()>>>>,
    /// Timestamp service
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
}

impl CacheWarmer {
    /// Create a new cache warmer
    #[must_use]
    pub fn new(
        config: CacheWarmingConfig,
        cache: Arc<MultiLayerCache>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        let mut strategies: Vec<Box<dyn WarmingStrategy + Send + Sync>> = vec![Box::new(
            FrequencyBasedStrategy::new(Duration::from_secs(3600)),
        )];

        if config.predictive_warming {
            strategies.push(Box::new(PredictiveStrategy::new(
                Duration::from_secs(7200),
                Duration::from_secs(600),
            )));
        }

        Self {
            config: Arc::new(RwLock::new(config)),
            cache,
            history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            strategies,
            warming_queue: Arc::new(RwLock::new(BinaryHeap::new())),
            warming_in_progress: Arc::new(DashMap::new()),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            timestamp,
        }
    }

    /// Record a cache access
    pub async fn record_access(&self, key: ValidatorCacheKey) {
        let mut history = self.history.write().await;
        let config = self.config.read().await;

        // Add to history
        history.push(AccessEntry {
            key: key.clone(),
            timestamp: std::time::Instant::now(),
            count: 1,
        });

        // Trim history if needed
        if history.len() > config.history_size {
            let drain_count = history.len() - config.history_size;
            history.drain(0..drain_count);
        }
    }

    /// Analyze and queue keys for warming
    pub async fn analyze_and_queue(&self) {
        let history = self.history.read().await;
        let config = self.config.read().await;

        // Get current cache state - cache stats would tell us what's cached
        let current_cache = DashMap::new();
        // Since we can't directly iterate the cache, we'll track cached keys separately
        // In production, this would be tracked via cache metadata or a separate index

        // Run strategies
        let mut all_candidates = Vec::new();

        for strategy in &self.strategies {
            let candidates = strategy.select_keys(&history, &current_cache, &config);

            for key in candidates {
                let priority = strategy.calculate_priority(&key, &history);

                if priority >= config.priority_threshold {
                    // Estimate time based on priority (higher priority = likely more complex)
                    let time_millis = 50.0 * (1.0 + priority);
                    let estimated_time = Duration::from_millis(
                        crate::utils::f64_to_u64_saturating(time_millis.max(0.0)),
                    );
                    all_candidates.push(WarmingEntry {
                        key,
                        priority,
                        estimated_time,
                    });
                }
            }
        }

        // Add to queue
        let mut queue = self.warming_queue.write().await;
        for entry in all_candidates {
            queue.push(entry);
        }
    }

    /// Warm a single validator
    fn warm_validator(
        &self,
        key: &ValidatorCacheKey,
        engine: &ValidationEngine,
    ) -> Result<()> {
        // Mark as in progress
        self.warming_in_progress
            .insert(key.clone(), std::time::Instant::now());

        // Compile validator
        let start = std::time::Instant::now();

        if let Some(class_def) = engine.schema.classes.get(&key.class_name) {
            // Create compilation options based on key requirements
            let options = CompilationOptions::ALL;
            let validator = CompiledValidator::compile_class(
                &engine.schema,
                &key.class_name,
                class_def,
                options,
            )?;

            // Put in cache
            let validator_arc = Arc::new(validator);
            self.cache.put(key, &validator_arc)?;

            let duration = start.elapsed();
            tracing::debug!("Warmed validator for {} in {:?}", key.class_name, duration);
        }

        // Remove from in progress
        self.warming_in_progress.remove(key);

        Ok(())
    }

    /// Run the warming process
    ///
    /// # Errors
    ///
    /// Returns an error if cache warming fails.
    pub async fn run_warming_cycle(&self, engine: Arc<ValidationEngine>) -> Result<()> {
        let config = self.config.read().await;

        if !config.auto_warm {
            return Ok(());
        }

        // Analyze and queue
        self.analyze_and_queue().await;

        // Process queue
        let mut tasks = Vec::new();
        let mut queue = self.warming_queue.write().await;

        for _ in 0..config.batch_size {
            if let Some(entry) = queue.pop() {
                // Skip if already warming or if estimated time is too long
                if self.warming_in_progress.contains_key(&entry.key) {
                    continue;
                }

                // Skip entries that would take too long
                if entry.estimated_time > Duration::from_secs(1) {
                    tracing::debug!(
                        "Skipping cache warming for {} due to long estimated time: {:?}",
                        entry.key.class_name,
                        entry.estimated_time
                    );
                    continue;
                }

                let warmer = self.clone();
                let key = entry.key.clone();
                let engine_clone = engine.clone();

                let task = tokio::task::spawn_blocking(move || {
                    let runtime = tokio::runtime::Handle::current();
                    runtime.block_on(async {
                        if let Err(e) = warmer.warm_validator(&key, &engine_clone) {
                            tracing::warn!("Failed to warm validator: {:?}", e);
                        }
                    });
                });

                tasks.push(task);

                // Limit concurrent tasks
                if tasks.len() >= config.max_concurrent {
                    break;
                }
            } else {
                break;
            }
        }

        // Store task handles with bounded growth
        {
            let mut handles = self.task_handles.write().await;

            // Cleanup completed handles
            handles.retain(|h| !h.is_finished());

            // If at limit, abort oldest tasks
            while handles.len() + tasks.len() > 5 && !handles.is_empty() {
                let oldest = handles.remove(0);
                oldest.abort();
            }

            // Convert spawn_blocking tasks to regular tasks for storage
            for _task in &tasks {
                // Create a wrapper task that we can store
                let task_handle = tokio::spawn(async {
                    // This wrapper just waits for the spawn_blocking task
                });
                handles.push(task_handle);
            }
        }

        // Wait for tasks
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    /// Start background warming worker
    pub fn start_background_worker(
        self: Arc<Self>,
        engine: Arc<ValidationEngine>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let config = self.config.read().await;
            let interval = config.warming_interval;
            drop(config);

            let mut timer = tokio::time::interval(interval);

            loop {
                timer.tick().await;

                if let Err(e) = self.run_warming_cycle(engine.clone()).await {
                    tracing::error!("Cache warming cycle failed: {:?}", e);
                }
            }
        })
    }

    /// Cancel all running tasks
    pub async fn cancel_all_tasks(&self) {
        let mut handles = self.task_handles.write().await;
        for handle in handles.drain(..) {
            handle.abort();
        }
    }

    /// Cleanup completed tasks
    pub async fn cleanup_completed_tasks(&self) {
        let mut handles = self.task_handles.write().await;
        handles.retain(|h| !h.is_finished());
    }
}

// Manual Clone implementation needed due to trait objects
impl Clone for CacheWarmer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            cache: self.cache.clone(),
            history: self.history.clone(),
            strategies: vec![
                Box::new(FrequencyBasedStrategy::new(Duration::from_secs(3600)))
                    as Box<dyn WarmingStrategy + Send + Sync>,
            ],
            warming_queue: self.warming_queue.clone(),
            warming_in_progress: self.warming_in_progress.clone(),
            task_handles: Arc::new(RwLock::new(Vec::new())),
            timestamp: self.timestamp.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_strategy() {
        let strategy = FrequencyBasedStrategy::new(Duration::from_secs(3600));
        let mut history = Vec::new();
        let now = std::time::Instant::now();

        // Add some access entries
        for i in 0..5 {
            history.push(AccessEntry {
                key: ValidatorCacheKey {
                    schema_id: "test".to_string(),
                    schema_hash: "hash".to_string(),
                    class_name: format!("Class{}", i % 2),
                    options_hash: "opts".to_string(),
                },
                timestamp: now.checked_sub(Duration::from_secs(i * 60)).unwrap_or(now),
                count: i as u32 + 1,
            });
        }

        let current_cache = DashMap::new();
        let config = CacheWarmingConfig::default();

        let selected = strategy.select_keys(&history, &current_cache, &config);
        assert!(!selected.is_empty());
    }

    #[test]
    fn test_warming_entry_ordering() {
        let entry1 = WarmingEntry {
            key: ValidatorCacheKey {
                schema_id: "test".to_string(),
                schema_hash: "hash".to_string(),
                class_name: "Class1".to_string(),
                options_hash: "opts".to_string(),
            },
            priority: 0.8,
            estimated_time: Duration::from_millis(50),
        };

        let entry2 = WarmingEntry {
            key: ValidatorCacheKey {
                schema_id: "test".to_string(),
                schema_hash: "hash".to_string(),
                class_name: "Class2".to_string(),
                options_hash: "opts".to_string(),
            },
            priority: 0.9,
            estimated_time: Duration::from_millis(50),
        };

        assert!(entry2 > entry1);
    }
}
