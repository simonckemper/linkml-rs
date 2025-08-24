//! Multi-layer caching system for `LinkML` validation
//!
//! This module implements a hierarchical caching system with multiple layers:
//! - L1: In-memory cache (fastest, limited size)
//! - L2: Distributed cache via `RootReal`'s `CacheService`
//! - L3: Persistent disk cache (optional, for large schemas)

use super::{cache::ValidatorCacheKey, compiled::CompiledValidator};
use cache_core::{CacheError, CacheKey, CacheService, CacheTtl, CacheValue};
use linkml_core::error::{LinkMLError, Result};
use lru::LruCache;
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Configuration for multi-layer cache
#[derive(Debug, Clone)]
pub struct MultiLayerCacheConfig {
    /// L1 cache size (number of validators)
    pub l1_max_validators: usize,
    /// L1 time-to-live
    pub l1_ttl: Duration,
    /// L2 cache time-to-live
    pub l2_ttl: Duration,
    /// Enable L3 disk cache
    pub l3_enabled: bool,
    /// L3 cache directory
    pub l3_directory: Option<String>,
    /// L3 cache max size in bytes
    pub l3_max_size_bytes: usize,
    /// Cache warming on startup
    pub warm_on_startup: bool,
    /// Prefetch related validators
    pub prefetch_related: bool,
}

impl Default for MultiLayerCacheConfig {
    fn default() -> Self {
        Self {
            l1_max_validators: 100,
            l1_ttl: Duration::from_secs(300),  // 5 minutes
            l2_ttl: Duration::from_secs(3600), // 1 hour
            l3_enabled: false,
            l3_directory: None,
            l3_max_size_bytes: 100 * 1024 * 1024, // 100MB
            warm_on_startup: false,
            prefetch_related: true,
        }
    }
}

/// Entry in L1 cache with timestamp
struct L1Entry {
    validator: Arc<CompiledValidator>,
    inserted_at: Instant,
}

/// Multi-layer cache implementation
pub struct MultiLayerCache {
    /// Configuration
    config: MultiLayerCacheConfig,
    /// L1: Fast in-memory LRU cache
    l1_cache: Arc<Mutex<LruCache<ValidatorCacheKey, L1Entry>>>,
    /// L2: Distributed cache service
    l2_cache: Option<Arc<dyn CacheService<Error = CacheError> + Send + Sync>>,
    /// L3: Disk cache
    l3_cache: Option<Arc<DiskCache>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Background tasks handle  
    #[allow(dead_code)]
    background_handle: Option<Arc<tokio::task::JoinHandle<()>>>,
}

/// Cache statistics across all layers
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// L1 hits
    pub l1_hits: u64,
    /// L1 misses
    pub l1_misses: u64,
    /// L2 hits
    pub l2_hits: u64,
    /// L2 misses
    pub l2_misses: u64,
    /// L3 hits
    pub l3_hits: u64,
    /// L3 misses
    pub l3_misses: u64,
    /// Total gets
    pub total_gets: u64,
    /// Total puts
    pub total_puts: u64,
    /// Average get latency in microseconds
    pub avg_get_latency_us: u64,
    /// Average put latency in microseconds
    pub avg_put_latency_us: u64,
}

impl CacheStats {
    /// Calculate overall hit rate
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn hit_rate(&self) -> f64 {
        let total_hits = self.l1_hits + self.l2_hits + self.l3_hits;
        let total_accesses = self.total_gets;
        if total_accesses == 0 {
            0.0
        } else {
            (total_hits as f64) / (total_accesses as f64)
        }
    }
}

impl MultiLayerCache {
    /// Create a new multi-layer cache
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn new(
        config: MultiLayerCacheConfig,
        cache_service: Option<Arc<dyn CacheService<Error = CacheError> + Send + Sync>>,
    ) -> Result<Self> {
        // Initialize L1 cache
        let l1_cache = Arc::new(Mutex::new(LruCache::<ValidatorCacheKey, L1Entry>::new(
            std::num::NonZeroUsize::new(config.l1_max_validators)
                .ok_or_else(|| LinkMLError::service("L1 cache size must be > 0"))?,
        )));

        // Initialize L3 disk cache if enabled
        let l3_cache = if config.l3_enabled {
            let dir = config
                .l3_directory
                .as_ref()
                .ok_or_else(|| LinkMLError::service("L3 cache directory required when enabled"))?;
            Some(Arc::new(DiskCache::new(dir, config.l3_max_size_bytes)?))
        } else {
            None
        };

        // Start background tasks for cache maintenance
        let background_handle =
            if config.warm_on_startup || config.l1_ttl < Duration::from_secs(3600) {
                let l1_clone = l1_cache.clone();
                let l1_ttl = config.l1_ttl;

                Some(tokio::spawn(async move {
                    let mut interval = tokio::time::interval(Duration::from_secs(60));
                    loop {
                        interval.tick().await;

                        // Evict expired entries from L1
                        let now = Instant::now();
                        let mut cache = l1_clone.lock();

                        // Collect expired keys
                        let expired_keys: Vec<_> = cache
                            .iter()
                            .filter(|(_, entry)| now.duration_since(entry.inserted_at) > l1_ttl)
                            .map(|(key, _)| key.clone())
                            .collect();

                        // Remove expired entries
                        for key in expired_keys {
                            cache.pop(&key);
                        }

                        drop(cache);
                    }
                }))
            } else {
                None
            };

        Ok(Self {
            config,
            l1_cache,
            l2_cache: cache_service,
            l3_cache,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            background_handle: background_handle.map(Arc::new),
        })
    }

    /// Get a validator from the cache
    #[allow(clippy::await_holding_lock)]
    pub async fn get(&self, key: &ValidatorCacheKey) -> Option<Arc<CompiledValidator>> {
        let start = Instant::now();
        let mut stats = self.stats.write();
        stats.total_gets += 1;
        drop(stats);

        // Try L1 first
        {
            let mut l1 = self.l1_cache.lock();
            if let Some(entry) = l1.get(key) {
                // Check if not expired
                if start.duration_since(entry.inserted_at) <= self.config.l1_ttl {
                    let mut stats = self.stats.write();
                    stats.l1_hits += 1;
                    stats.avg_get_latency_us = (stats.avg_get_latency_us * (stats.total_gets - 1)
                        + u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
                        / stats.total_gets;
                    drop(stats);
                    return Some(entry.validator.clone());
                }
            }
        }

        let mut stats = self.stats.write();
        stats.l1_misses += 1;
        drop(stats);

        // Try L2 (distributed cache)
        if let Some(l2) = &self.l2_cache {
            let Ok(cache_key) = CacheKey::new(format!("linkml:validator:{key}")) else {
                return None; // Skip on error
            };

            if let Ok(Some(value)) = l2.get(&cache_key).await {
                if let Ok(bytes) = value.to_bytes() {
                    if let Ok(validator) = self.deserialize_validator(&bytes) {
                        let validator = Arc::new(validator);

                        // Promote to L1
                        self.promote_to_l1(key.clone(), validator.clone());

                        let mut stats = self.stats.write();
                        stats.l2_hits += 1;
                        stats.avg_get_latency_us = (stats.avg_get_latency_us
                            * (stats.total_gets - 1)
                            + u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
                            / stats.total_gets;
                        drop(stats);

                        return Some(validator);
                    }
                }
            }
        }

        let mut stats = self.stats.write();
        stats.l2_misses += 1;
        drop(stats);

        // Try L3 (disk cache)
        if let Some(l3) = &self.l3_cache {
            if let Ok(Some(validator)) = l3.get(key).await {
                let validator = Arc::new(validator);

                // Promote to L1 and L2
                self.promote_to_l1(key.clone(), validator.clone());
                let () = self.promote_to_l2(key.clone(), validator.clone()).await;

                let mut stats = self.stats.write();
                stats.l3_hits += 1;
                stats.avg_get_latency_us = (stats.avg_get_latency_us * (stats.total_gets - 1)
                    + u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
                    / stats.total_gets;
                drop(stats);

                return Some(validator);
            }
        }

        let mut stats = self.stats.write();
        stats.l3_misses += 1;
        stats.avg_get_latency_us = (stats.avg_get_latency_us * (stats.total_gets - 1)
            + u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
            / stats.total_gets;
        drop(stats);

        None
    }

    /// Put a validator into the cache
    ///
    /// # Errors
    ///
    /// Returns an error if cache operations fail.
    #[allow(clippy::unused_async)]
    pub async fn put(
        &self,
        key: ValidatorCacheKey,
        validator: Arc<CompiledValidator>,
    ) -> Result<()> {
        let start = Instant::now();
        let mut stats = self.stats.write();
        stats.total_puts += 1;
        drop(stats);

        // Always put in L1
        {
            let mut l1 = self.l1_cache.lock();
            l1.put(
                key.clone(),
                L1Entry {
                    validator: validator.clone(),
                    inserted_at: Instant::now(),
                },
            );
        }

        // Put in L2 if available
        if let Some(l2) = &self.l2_cache {
            let cache_key = CacheKey::new(format!("linkml:validator:{key}"))
                .map_err(|e| LinkMLError::service(format!("Failed to create cache key: {e}")))?;
            let serialized = self.serialize_validator(&validator)?;
            let cache_value = CacheValue::from_bytes(serialized);
            let ttl = Some(CacheTtl::Seconds(self.config.l2_ttl.as_secs()));

            // Fire and forget for async L2 write
            let l2_clone = l2.clone();
            tokio::spawn(async move {
                let _ = l2_clone.set(&cache_key, &cache_value, ttl).await;
            });
        }

        // Put in L3 if available
        if let Some(l3) = &self.l3_cache {
            // Fire and forget for async L3 write
            let l3_clone = l3.clone();
            let key_clone = key.clone();
            let validator_clone = validator.clone();
            tokio::spawn(async move {
                let _ = l3_clone.put(key_clone, &validator_clone).await;
            });
        }

        // Prefetch related validators if configured
        if self.config.prefetch_related {
            self.prefetch_related_validators(&key, &validator);
        }

        let mut stats = self.stats.write();
        stats.avg_put_latency_us = (stats.avg_put_latency_us * (stats.total_puts - 1)
            + u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
            / stats.total_puts;
        drop(stats);

        Ok(())
    }

    /// Invalidate a cache entry across all layers
    ///
    /// # Errors
    ///
    /// Returns an error if cache deletion fails.
    pub async fn invalidate(&self, key: &ValidatorCacheKey) -> Result<()> {
        // Remove from L1
        {
            let mut l1 = self.l1_cache.lock();
            l1.pop(key);
        }

        // Remove from L2
        if let Some(l2) = &self.l2_cache {
            let cache_key = CacheKey::new(format!("linkml:validator:{key}"))
                .map_err(|e| LinkMLError::service(format!("Failed to create cache key: {e}")))?;
            l2.delete(&cache_key)
                .await
                .map_err(|e| LinkMLError::service(format!("Cache delete failed: {e}")))?;
        }

        // Remove from L3
        if let Some(l3) = &self.l3_cache {
            l3.delete(key).await?;
        }

        Ok(())
    }

    /// Clear all caches
    ///
    /// # Errors
    ///
    /// Returns an error if cache clearing fails.
    pub async fn clear(&self) -> Result<()> {
        // Clear L1
        {
            let mut l1 = self.l1_cache.lock();
            l1.clear();
        }

        // Clear L2 (if pattern-based clear is supported)
        if let Some(l2) = &self.l2_cache {
            // Note: This assumes the cache service supports pattern-based deletion
            // In practice, we might need to track all keys or implement differently
            let pattern = CacheKey::new("linkml:validator:*").map_err(|e| {
                LinkMLError::service(format!("Failed to create cache pattern: {e}"))
            })?;
            let _ = l2.delete(&pattern).await;
        }

        // Clear L3
        if let Some(l3) = &self.l3_cache {
            l3.clear().await?;
        }

        Ok(())
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Warm the cache with frequently used validators
    ///
    /// # Errors
    ///
    /// Returns an error if cache warming fails.
    pub async fn warm_cache(
        &self,
        validators: Vec<(ValidatorCacheKey, Arc<CompiledValidator>)>,
    ) -> Result<()> {
        for (key, validator) in validators {
            self.put(key, validator).await?;
        }
        Ok(())
    }

    // Helper methods

    fn promote_to_l1(&self, key: ValidatorCacheKey, validator: Arc<CompiledValidator>) {
        let mut l1 = self.l1_cache.lock();
        l1.put(
            key,
            L1Entry {
                validator,
                inserted_at: Instant::now(),
            },
        );
    }

    async fn promote_to_l2(&self, key: ValidatorCacheKey, validator: Arc<CompiledValidator>) {
        if let Some(l2) = &self.l2_cache {
            if let Ok(cache_key) = CacheKey::new(format!("linkml:validator:{key}")) {
                if let Ok(serialized) = self.serialize_validator(&validator) {
                    let cache_value = CacheValue::from_bytes(serialized);
                    let ttl = Some(CacheTtl::Seconds(self.config.l2_ttl.as_secs()));
                    let _ = l2.set(&cache_key, &cache_value, ttl).await;
                }
            }
        }
    }

    fn prefetch_related_validators(
        &self,
        _key: &ValidatorCacheKey,
        _validator: &CompiledValidator,
    ) {
        let _ = self; // Placeholder for future implementation
        // TODO: Implement prefetching logic based on validator dependencies
        // For now, this is a placeholder
    }

    fn serialize_validator(&self, validator: &CompiledValidator) -> Result<Vec<u8>> {
        let _ = self;
        // Use bincode for efficient binary serialization
        bincode::serialize(validator)
            .map_err(|e| LinkMLError::service(format!("Failed to serialize validator: {e}")))
    }

    fn deserialize_validator(&self, data: &[u8]) -> Result<CompiledValidator> {
        let _ = self;
        bincode::deserialize(data)
            .map_err(|e| LinkMLError::service(format!("Failed to deserialize validator: {e}")))
    }
}

/// Simple disk cache implementation
struct DiskCache {
    directory: String,
    max_size_bytes: usize,
    current_size: Arc<RwLock<usize>>,
}

impl DiskCache {
    fn new(directory: &str, max_size_bytes: usize) -> Result<Self> {
        // Create directory if it doesn't exist
        std::fs::create_dir_all(directory)
            .map_err(|e| LinkMLError::service(format!("Failed to create cache directory: {e}")))?;

        // Calculate current size
        let current_size = Self::calculate_directory_size(directory)?;

        Ok(Self {
            directory: directory.to_string(),
            max_size_bytes,
            current_size: Arc::new(RwLock::new(current_size)),
        })
    }

    async fn get(&self, key: &ValidatorCacheKey) -> Result<Option<CompiledValidator>> {
        let path = self.key_to_path(key);

        match tokio::fs::read(&path).await {
            Ok(data) => bincode::deserialize(&data)
                .map(Some)
                .map_err(|e| LinkMLError::service(format!("Failed to deserialize from disk: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(LinkMLError::service(format!(
                "Failed to read from disk cache: {e}"
            ))),
        }
    }

    async fn put(&self, key: ValidatorCacheKey, validator: &CompiledValidator) -> Result<()> {
        let path = self.key_to_path(&key);
        let data = bincode::serialize(validator)
            .map_err(|e| LinkMLError::service(format!("Failed to serialize for disk: {e}")))?;

        // Check if we need to evict old entries
        let data_size = data.len();
        self.evict_if_needed(data_size).await?;

        // Write to disk
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                LinkMLError::service(format!("Failed to create cache subdirectory: {e}"))
            })?;
        }

        tokio::fs::write(&path, data)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to write to disk cache: {e}")))?;

        // Update size
        let mut size = self.current_size.write();
        *size += data_size;

        Ok(())
    }

    async fn delete(&self, key: &ValidatorCacheKey) -> Result<()> {
        let path = self.key_to_path(key);

        if let Ok(metadata) = tokio::fs::metadata(&path).await {
            let file_size = usize::try_from(metadata.len()).unwrap_or(usize::MAX);

            tokio::fs::remove_file(&path).await.map_err(|e| {
                LinkMLError::service(format!("Failed to delete from disk cache: {e}"))
            })?;

            let mut size = self.current_size.write();
            *size = size.saturating_sub(file_size);
        }

        Ok(())
    }

    async fn clear(&self) -> Result<()> {
        tokio::fs::remove_dir_all(&self.directory)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to clear disk cache: {e}")))?;

        tokio::fs::create_dir_all(&self.directory)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to recreate cache directory: {e}"))
            })?;

        let mut size = self.current_size.write();
        *size = 0;

        Ok(())
    }

    async fn evict_if_needed(&self, new_size: usize) -> Result<()> {
        let current = *self.current_size.read();

        if current + new_size > self.max_size_bytes {
            // Simple LRU eviction based on file modification time
            let mut entries = Vec::new();

            let mut dir = tokio::fs::read_dir(&self.directory).await.map_err(|e| {
                LinkMLError::service(format!("Failed to read cache directory: {e}"))
            })?;

            while let Some(entry) = dir
                .next_entry()
                .await
                .map_err(|e| LinkMLError::service(format!("Failed to read directory entry: {e}")))?
            {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        if let Ok(modified) = metadata.modified() {
                            entries.push((
                                entry.path(),
                                usize::try_from(metadata.len()).unwrap_or(usize::MAX),
                                modified,
                            ));
                        }
                    }
                }
            }

            // Sort by modification time (oldest first)
            entries.sort_by_key(|(_, _, modified)| *modified);

            // Evict oldest entries until we have enough space
            let mut freed = 0;
            for (path, size, _) in entries {
                if current - freed + new_size <= self.max_size_bytes {
                    break;
                }

                if let Ok(()) = tokio::fs::remove_file(&path).await {
                    freed += size;
                }
            }

            let mut size = self.current_size.write();
            *size = current - freed;
        }

        Ok(())
    }

    fn key_to_path(&self, key: &ValidatorCacheKey) -> std::path::PathBuf {
        let hash = key.to_string();
        let (prefix, suffix) = hash.split_at(2.min(hash.len()));
        std::path::Path::new(&self.directory)
            .join(prefix)
            .join(format!("{suffix}.cache"))
    }

    fn calculate_directory_size(directory: &str) -> Result<usize> {
        let mut total_size = 0;

        for entry in walkdir::WalkDir::new(directory) {
            let entry = entry
                .map_err(|e| LinkMLError::service(format!("Failed to walk directory: {e}")))?;

            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    total_size += usize::try_from(metadata.len()).unwrap_or(usize::MAX);
                }
            }
        }

        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SchemaDefinition;

    #[tokio::test]
    async fn test_multi_layer_cache_basic() {
        let config = MultiLayerCacheConfig::default();
        let cache = MultiLayerCache::new(config, None).expect("should create cache");

        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            ..Default::default()
        };

        let key = ValidatorCacheKey::new(&schema, "TestClass", &Default::default());
        let validator = Arc::new(CompiledValidator::new());

        // Put and get
        cache
            .put(key.clone(), validator.clone())
            .await
            .expect("should put into cache");
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_some());

        // Check stats
        let stats = cache.stats();
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.total_gets, 1);
        assert_eq!(stats.total_puts, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let config = MultiLayerCacheConfig::default();
        let cache = MultiLayerCache::new(config, None).expect("should create cache");

        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            ..Default::default()
        };

        let key = ValidatorCacheKey::new(&schema, "TestClass", &Default::default());
        let validator = Arc::new(CompiledValidator::new());

        // Put, invalidate, and try to get
        cache
            .put(key.clone(), validator)
            .await
            .expect("should put into cache");
        cache
            .invalidate(&key)
            .await
            .expect("should invalidate cache");
        let retrieved = cache.get(&key).await;
        assert!(retrieved.is_none());
    }
}
