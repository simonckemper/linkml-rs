//! Optimized expression cache using `HashMap` utilities
//!
//! This module provides an optimized version of the expression cache that
//! minimizes cloning and uses more efficient collection operations.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use linkml_core::{
    error::Result,
    hashmap_utils::{ArcCache, collect_keys_for_removal},
    string_pool::intern,
};

use super::Expression;

/// Cached expression entry with metadata
#[derive(Clone)]
struct CacheEntryV2 {
    expression: Arc<Expression>,
    /// When this entry was created
    created: Instant,
    /// When this entry was last accessed
    last_accessed: Instant,
    /// Number of times accessed
    hit_count: u64,
}

/// Expression cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of cache hits (successful lookups)
    pub hits: u64,
    /// Number of cache misses (unsuccessful lookups)
    pub misses: u64,
    /// Number of entries evicted from cache (due to capacity or expiry)
    pub evictions: u64,
    /// Current number of entries in the cache
    pub entries: usize,
    /// Approximate memory usage in bytes
    pub size_bytes: usize,
}

impl CacheStats {
    /// Calculate hit rate
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Optimized expression cache using Arc and efficient eviction
pub struct ExpressionCacheV2 {
    cache: Arc<RwLock<HashMap<Arc<str>, CacheEntryV2>>>,
    /// LRU order tracking
    lru_order: Arc<RwLock<VecDeque<Arc<str>>>>,
    /// Maximum number of entries
    capacity: usize,
    /// Maximum age for entries
    max_age: Duration,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ExpressionCacheV2 {
    /// Create a new expression cache
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::with_capacity(capacity))),
            lru_order: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            capacity,
            max_age: Duration::from_secs(3600), // 1 hour default
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Set maximum age for cache entries
    #[must_use]
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Get a parsed expression from cache
    #[must_use]
    pub fn get(&self, expression: &str) -> Option<Arc<Expression>> {
        let key = intern(expression);
        let mut cache = self.cache.write().ok()?;
        let mut lru_order = self.lru_order.write().ok()?;
        let mut stats = self.stats.write().ok()?;

        if let Some(entry) = cache.get_mut(&key) {
            let now = Instant::now();

            // Check if entry has expired based on creation time
            if now.duration_since(entry.created) > self.max_age {
                // Entry has expired, remove it
                cache.remove(&key);
                if let Some(pos) = lru_order.iter().position(|k| k == &key) {
                    lru_order.remove(pos);
                }
                stats.evictions += 1;
                stats.misses += 1;
                stats.entries = cache.len();
                return None;
            }

            // Update access time and count
            entry.last_accessed = now;
            entry.hit_count += 1;
            stats.hits += 1;

            // Move to end of LRU order (most recently used)
            if let Some(pos) = lru_order.iter().position(|k| k == &key) {
                lru_order.remove(pos);
            }
            lru_order.push_back(Arc::clone(&key));

            Some(Arc::clone(&entry.expression))
        } else {
            stats.misses += 1;
            None
        }
    }

    /// Store a parsed expression in cache
    pub fn put(&self, expression: &str, parsed: Expression) {
        let key = intern(expression);
        let now = Instant::now();

        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut lru_order = self
            .lru_order
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        // Check if we need to evict
        if cache.len() >= self.capacity {
            // Remove least recently used
            if let Some(old_key) = lru_order.pop_front() {
                cache.remove(&old_key);
                stats.evictions += 1;
            }
        }

        // Insert new entry
        let entry = CacheEntryV2 {
            expression: Arc::new(parsed),
            created: now,
            last_accessed: now,
            hit_count: 0,
        };

        cache.insert(Arc::clone(&key), entry);
        lru_order.push_back(key);
        stats.entries = cache.len();
        stats.size_bytes = stats.entries * std::mem::size_of::<CacheEntryV2>();
    }

    /// Get or compute and cache an expression
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_or_compute<F>(&self, expression: &str, compute: F) -> Result<Arc<Expression>>
    where
        F: FnOnce() -> Result<Expression>,
    {
        // Check cache first
        if let Some(parsed) = self.get(expression) {
            return Ok(parsed);
        }

        // Compute and cache
        let parsed = compute()?;
        self.put(expression, parsed.clone());

        // Return the Arc we just stored
        self.get(expression)
            .ok_or_else(|| linkml_core::LinkMLError::parse("Cache put/get mismatch".to_string()))
    }

    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        cache.clear();
        *stats = CacheStats::default();
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Clean up old entries (optimized version)
    pub fn cleanup(&self) {
        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let now = Instant::now();

        // Collect keys to remove without cloning during iteration
        let to_remove = collect_keys_for_removal(
            &cache
                .iter()
                .map(|(k, v)| (k.clone(), v))
                .collect::<HashMap<_, _>>(),
            |_key, entry| now.duration_since(entry.last_accessed) > self.max_age,
        );

        // Remove old entries
        for key in to_remove {
            cache.remove(&key);
            stats.evictions += 1;
        }

        stats.entries = cache.len();
        stats.size_bytes = stats.entries * std::mem::size_of::<CacheEntryV2>();
    }
}

/// Global expression cache using `ArcCache`
pub struct GlobalExpressionCacheV2 {
    /// Primary cache for all expressions
    primary: ArcCache<Arc<str>, Expression>,
    /// Hot cache for frequently used expressions
    hot: ArcCache<Arc<str>, Expression>,
    /// Access counter for promotion
    access_counts: Arc<RwLock<HashMap<Arc<str>, u64>>>,
    /// Threshold for hot promotion
    hot_threshold: u64,
}

impl GlobalExpressionCacheV2 {
    /// Create a new global cache
    #[must_use]
    pub fn new(primary_capacity: usize, hot_capacity: usize) -> Self {
        Self {
            primary: ArcCache::with_capacity(primary_capacity),
            hot: ArcCache::with_capacity(hot_capacity),
            access_counts: Arc::new(RwLock::new(HashMap::new())),
            hot_threshold: 10,
        }
    }

    /// Get or compute an expression
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_or_compute<F>(&mut self, expression: &str, compute: F) -> Result<Arc<Expression>>
    where
        F: FnOnce() -> Result<Expression>,
    {
        let key = intern(expression);

        // Check hot cache first
        if let Ok(counts) = self.access_counts.read()
            && let Some(&count) = counts.get(&key)
            && count >= self.hot_threshold
        {
            return match compute() {
                Ok(parsed) => Ok(self.hot.get_or_compute(&key, || parsed)),
                Err(e) => Err(e),
            };
        }

        // Update access count
        if let Ok(mut counts) = self.access_counts.write() {
            let count = counts.entry(Arc::clone(&key)).or_insert(0);
            *count += 1;
        }

        // Use primary cache - handle Result properly
        match compute() {
            Ok(parsed) => Ok(self.primary.get_or_compute(&key, || parsed)),
            Err(e) => Err(e),
        }
    }

    /// Clear all caches
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn clear(&mut self) -> Result<()> {
        self.primary.clear();
        self.hot.clear();
        self.access_counts
            .write()
            .map_err(|e| {
                linkml_core::LinkMLError::parse(format!("access_counts lock poisoned: {e}"))
            })?
            .clear();
        Ok(())
    }
}

/// Thread-safe wrapper for global cache
pub struct ThreadSafeGlobalCache {
    inner: Arc<RwLock<GlobalExpressionCacheV2>>,
}

impl ThreadSafeGlobalCache {
    /// Create new thread-safe cache
    #[must_use]
    pub fn new(primary_capacity: usize, hot_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(GlobalExpressionCacheV2::new(
                primary_capacity,
                hot_capacity,
            ))),
        }
    }

    /// Get or compute with thread safety
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn get_or_compute<F>(&self, expression: &str, compute: F) -> Result<Arc<Expression>>
    where
        F: FnOnce() -> Result<Expression>,
    {
        self.inner
            .write()
            .map_err(|e| {
                linkml_core::LinkMLError::parse(format!("inner cache lock poisoned: {e}"))
            })?
            .get_or_compute(expression, compute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_cache_v2() {
        let cache = ExpressionCacheV2::new(2);

        // Test basic operations
        assert!(cache.get("test").is_none());

        let parsed = Expression::Null; // Use Null variant as default
        cache.put("test", parsed.clone());

        assert!(cache.get("test").is_some());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ExpressionCacheV2::new(2);

        cache.put("expr1", Expression::Null);
        cache.put("expr2", Expression::Null);
        cache.put("expr3", Expression::Null); // Should evict expr1

        assert!(cache.get("expr1").is_none());
        assert!(cache.get("expr2").is_some());
        assert!(cache.get("expr3").is_some());

        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cleanup() {
        let cache = ExpressionCacheV2::new(10).with_max_age(Duration::from_millis(100));

        cache.put("old", Expression::Null);

        // Wait for expiry
        std::thread::sleep(Duration::from_millis(150));

        cache.cleanup();

        assert!(cache.get("old").is_none());
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }
}
