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
use tokio::sync::RwLock;

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
    /// Create cache warming config from LinkML service configuration
    pub fn from_service_config(config: &linkml_core::configuration_v2::CacheConfig) -> Self {
        Self {
            auto_warm: config.enable_cache_warming,
            batch_size: 50,  // Could be added to config
            max_concurrent: 4,  // Could be added to config
            warming_interval: Duration::from_secs(config.warming_interval_seconds),
            priority_threshold: 0.5,  // Could be added to config
            predictive_warming: true,  // Could be added to config
            history_size: 1000,  // Could be added to config
        }
    }
}

/// Cache access history entry
#[derive(Debug, Clone)]
pub struct AccessEntry {
    /// Cache key
    pub key: ValidatorCacheKey,
    /// Access timestamp
    pub timestamp: Instant,
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
    _estimated_time: Duration,
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
        let now = Instant::now();
        let window_start = now.checked_sub(self.window).unwrap_or(Instant::now());

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
        let now = Instant::now();
        let window_start = now.checked_sub(self.window).unwrap_or(Instant::now());

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
    _pattern_window: Duration,
    /// Prediction lookahead
    lookahead: Duration,
}

impl PredictiveStrategy {
    /// Create a new predictive strategy
    #[must_use]
    pub fn new(pattern_window: Duration, lookahead: Duration) -> Self {
        Self {
            _pattern_window: pattern_window,
            lookahead,
        }
    }

    /// Detect access patterns
    fn detect_patterns(&self, history: &[AccessEntry]) -> Vec<(ValidatorCacheKey, Duration)> {
        let _ = self;
        let mut patterns = Vec::new();
        let key_accesses: DashMap<ValidatorCacheKey, SmallVec<[Instant; 16]>> = DashMap::new();

        // Group accesses by key
        for entry in history {
            key_accesses
                .entry(entry.key.clone())
                .or_default()
                .push(entry.timestamp);
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

            #[allow(clippy::cast_precision_loss)]
            let avg_interval = intervals
                .iter()
                .map(std::time::Duration::as_secs_f64)
                .sum::<f64>()
                / intervals.len() as f64;

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
        let now = Instant::now();

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

        #[allow(clippy::cast_precision_loss)]
        let mean = intervals.iter().sum::<f64>() / intervals.len() as f64;
        #[allow(clippy::cast_precision_loss)]
        let variance =
            intervals.iter().map(|i| (i - mean).powi(2)).sum::<f64>() / intervals.len() as f64;

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
    warming_in_progress: Arc<DashMap<ValidatorCacheKey, Instant>>,
}

impl CacheWarmer {
    /// Create a new cache warmer
    #[must_use]
    pub fn new(config: CacheWarmingConfig, cache: Arc<MultiLayerCache>) -> Self {
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
        }
    }

    /// Record a cache access
    pub async fn record_access(&self, key: ValidatorCacheKey) {
        let mut history = self.history.write().await;
        let config = self.config.read().await;

        // Add to history
        history.push(AccessEntry {
            key: key.clone(),
            timestamp: Instant::now(),
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

        // Get current cache state
        let current_cache = DashMap::new();
        // TODO: Query actual cache state

        // Run strategies
        let mut all_candidates = Vec::new();

        for strategy in &self.strategies {
            let candidates = strategy.select_keys(&history, &current_cache, &config);

            for key in candidates {
                let priority = strategy.calculate_priority(&key, &history);

                if priority >= config.priority_threshold {
                    all_candidates.push(WarmingEntry {
                        key,
                        priority,
                        _estimated_time: Duration::from_millis(50), // TODO: Estimate based on schema
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
    async fn warm_validator(
        &self,
        key: &ValidatorCacheKey,
        engine: &ValidationEngine,
    ) -> Result<()> {
        // Mark as in progress
        self.warming_in_progress.insert(key.clone(), Instant::now());

        // Compile validator
        let start = Instant::now();

        if let Some(class_def) = engine.schema.classes.get(&key.class_name) {
            let options = CompilationOptions::default(); // TODO: Parse from key
            let validator = CompiledValidator::compile_class(
                &engine.schema,
                &key.class_name,
                class_def,
                &options,
            )?;

            // Put in cache
            self.cache.put(key.clone(), Arc::new(validator)).await?;

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
                // Skip if already warming
                if self.warming_in_progress.contains_key(&entry.key) {
                    continue;
                }

                let warmer = self.clone();
                let key = entry.key.clone();
                let engine_clone = engine.clone();

                let task = tokio::task::spawn_blocking(move || {
                    let runtime = tokio::runtime::Handle::current();
                    runtime.block_on(async {
                        if let Err(e) = warmer.warm_validator(&key, &engine_clone).await {
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

        // Wait for tasks
        for task in tasks {
            let _ = task.await;
        }

        Ok(())
    }

    /// Start background warming worker
    #[must_use]
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
        let now = Instant::now();

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
            _estimated_time: Duration::from_millis(50),
        };

        let entry2 = WarmingEntry {
            key: ValidatorCacheKey {
                schema_id: "test".to_string(),
                schema_hash: "hash".to_string(),
                class_name: "Class2".to_string(),
                options_hash: "opts".to_string(),
            },
            priority: 0.9,
            _estimated_time: Duration::from_millis(50),
        };

        assert!(entry2 > entry1);
    }
}
