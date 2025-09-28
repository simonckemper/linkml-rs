//! Expression caching for performance optimization
//!
//! This module provides caching mechanisms for both parsed expressions
//! and compiled bytecode to avoid redundant parsing and compilation.

use super::ast::Expression;
use super::compiler::CompiledExpression;
use lru::LruCache;
use std::hash::Hash;
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Key for caching expressions
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ExpressionKey {
    /// The source expression text used as the primary cache key
    pub source: String,
    /// Optional context schema for validation
    pub schema_id: Option<String>,
}

/// Cached expression entry
#[derive(Clone)]
pub struct CachedExpression {
    /// Parsed AST
    pub ast: Expression,
    /// Compiled bytecode
    pub compiled: Option<Arc<CompiledExpression>>,
    /// When this entry was last accessed
    pub last_accessed: Instant,
    /// Number of times this expression has been used
    pub hit_count: u64,
}

/// Statistics about cache performance
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of evictions
    pub evictions: u64,
    /// Total time saved by cache hits (estimated)
    pub time_saved_ms: u64,
    /// Current number of entries
    pub entries: usize,
    /// Current cache size in bytes (estimated)
    pub size_bytes: usize,
}

/// Expression cache with LRU eviction
pub struct ExpressionCache {
    cache: Arc<RwLock<LruCache<ExpressionKey, CachedExpression>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Maximum age for cache entries
    max_age: Duration,
    /// Whether to cache compiled bytecode
    cache_compiled: bool,
}

impl ExpressionCache {
    /// Create a new expression cache with given capacity
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        // Ensure capacity is at least 1
        let capacity = NonZeroUsize::new(capacity.max(1)).expect("capacity.max(1) is always >= 1");
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_age: Duration::from_secs(3600), // 1 hour default
            cache_compiled: true,
        }
    }

    /// Create expression cache from `LinkML` service configuration
    #[must_use]
    pub fn from_service_config(
        config: &linkml_core::configuration_v2::ExpressionCacheConfig,
    ) -> Self {
        // Ensure capacity is at least 1
        let capacity = NonZeroUsize::new(config.max_entries.max(1))
            .expect("config.max_entries.max(1) is always >= 1");
        Self {
            cache: Arc::new(RwLock::new(LruCache::new(capacity))),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_age: Duration::from_secs(config.ttl_seconds),
            cache_compiled: true,
        }
    }

    /// Set maximum age for cache entries
    #[must_use]
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }

    /// Set whether to cache compiled bytecode
    #[must_use]
    pub fn with_compiled_caching(mut self, enabled: bool) -> Self {
        self.cache_compiled = enabled;
        self
    }

    /// Get an expression from the cache
    #[must_use]
    pub fn get(&self, key: &ExpressionKey) -> Option<CachedExpression> {
        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is too old
            if entry.last_accessed.elapsed() > self.max_age {
                cache.pop(key);
                stats.evictions += 1;
                stats.misses += 1;
                return None;
            }

            // Update access time and hit count
            entry.last_accessed = Instant::now();
            entry.hit_count += 1;

            stats.hits += 1;
            // Estimate 10ms saved per cache hit (parsing + compilation)
            stats.time_saved_ms += 10;

            Some(entry.clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    /// Insert an expression into the cache
    pub fn insert(
        &self,
        key: ExpressionKey,
        ast: Expression,
        compiled: Option<Arc<CompiledExpression>>,
    ) {
        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let entry = CachedExpression {
            ast,
            compiled: if self.cache_compiled { compiled } else { None },
            last_accessed: Instant::now(),
            hit_count: 0,
        };

        if cache.put(key, entry).is_some() {
            stats.evictions += 1;
        }

        stats.entries = cache.len();
        // Estimate cache size (very rough)
        stats.size_bytes = stats.entries * 1024; // Assume ~1KB per entry
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
        stats.entries = 0;
        stats.size_bytes = 0;
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        // If lock is poisoned, return default stats rather than panic
        self.stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Get cache hit rate (0.0 to 1.0)
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        // If lock is poisoned, return 0.0 rather than panic
        let stats = self
            .stats
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let total = stats.hits + stats.misses;
        if total == 0 {
            0.0
        } else {
            crate::utils::u64_to_f64_lossy(stats.hits) / crate::utils::u64_to_f64_lossy(total)
        }
    }

    /// Prune old entries from the cache
    pub fn prune_old_entries(&self) {
        let mut cache = self
            .cache
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut stats = self
            .stats
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let now = Instant::now();
        let mut to_remove = Vec::new();

        // Collect keys to remove (can't modify while iterating)
        for (key, entry) in cache.iter() {
            if now.duration_since(entry.last_accessed) > self.max_age {
                to_remove.push(key.clone());
            }
        }

        // Remove old entries
        for key in to_remove {
            cache.pop(&key);
            stats.evictions += 1;
        }

        stats.entries = cache.len();
        stats.size_bytes = stats.entries * 1024;
    }
}

/// Global expression cache for the entire application
pub struct GlobalExpressionCache {
    /// Cache for parsed expressions
    parse_cache: ExpressionCache,
    /// Separate cache for frequently used expressions
    hot_cache: ExpressionCache,
    /// Threshold for promoting to hot cache
    hot_threshold: u64,
}

impl GlobalExpressionCache {
    /// Create a new global cache
    #[must_use]
    pub fn new(parse_capacity: usize, hot_capacity: usize) -> Self {
        Self {
            parse_cache: ExpressionCache::new(parse_capacity),
            hot_cache: ExpressionCache::new(hot_capacity).with_max_age(Duration::from_secs(7200)), // 2 hours for hot
            hot_threshold: 10,
        }
    }

    /// Set the threshold for promoting expressions to hot cache
    #[must_use]
    pub fn with_hot_threshold(mut self, threshold: u64) -> Self {
        self.hot_threshold = threshold;
        self
    }

    /// Get an expression, checking hot cache first
    #[must_use]
    pub fn get(&self, key: &ExpressionKey) -> Option<CachedExpression> {
        // Check hot cache first
        if let Some(entry) = self.hot_cache.get(key) {
            return Some(entry);
        }

        // Check regular cache
        if let Some(entry) = self.parse_cache.get(key) {
            // Promote to hot cache if accessed frequently
            if entry.hit_count >= self.hot_threshold {
                self.hot_cache
                    .insert(key.clone(), entry.ast.clone(), entry.compiled.clone());
            }
            return Some(entry);
        }

        None
    }

    /// Insert an expression into the appropriate cache
    pub fn insert(
        &self,
        key: ExpressionKey,
        ast: Expression,
        compiled: Option<Arc<CompiledExpression>>,
    ) {
        self.parse_cache.insert(key, ast, compiled);
    }

    /// Get combined statistics from both caches
    #[must_use]
    pub fn stats(&self) -> GlobalCacheStats {
        let parse_stats = self.parse_cache.stats();
        let hot_stats = self.hot_cache.stats();

        GlobalCacheStats {
            parse_cache: parse_stats.clone(),
            hot_cache: hot_stats.clone(),
            total_hits: parse_stats.hits + hot_stats.hits,
            total_misses: parse_stats.misses,
            hot_hit_rate: self.hot_cache.hit_rate(),
            overall_hit_rate: self.overall_hit_rate(),
        }
    }

    /// Get overall hit rate
    #[must_use]
    pub fn overall_hit_rate(&self) -> f64 {
        let parse_stats = self.parse_cache.stats();
        let hot_stats = self.hot_cache.stats();

        let total_hits = parse_stats.hits + hot_stats.hits;
        let total_requests = total_hits + parse_stats.misses;

        if total_requests == 0 {
            0.0
        } else {
            crate::utils::u64_to_f64_lossy(total_hits)
                / crate::utils::u64_to_f64_lossy(total_requests)
        }
    }

    /// Clear all caches
    pub fn clear(&self) {
        self.parse_cache.clear();
        self.hot_cache.clear();
    }

    /// Prune old entries from all caches
    pub fn prune_old_entries(&self) {
        self.parse_cache.prune_old_entries();
        self.hot_cache.prune_old_entries();
    }
}

/// Combined statistics for global cache
#[derive(Clone, Debug)]
pub struct GlobalCacheStats {
    /// Stats for the main parse cache
    pub parse_cache: CacheStats,
    /// Stats for the hot cache
    pub hot_cache: CacheStats,
    /// Total hits across both caches
    pub total_hits: u64,
    /// Total misses (only counted once)
    pub total_misses: u64,
    /// Hit rate for hot cache
    pub hot_hit_rate: f64,
    /// Overall hit rate
    pub overall_hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::ast::Expression;

    #[test]
    fn test_basic_caching() {
        let cache = ExpressionCache::new(10);
        let key = ExpressionKey {
            source: "1 + 2".to_string(),
            schema_id: None,
        };

        // Cache miss
        assert!(cache.get(&key).is_none());
        assert_eq!(cache.stats().misses, 1);

        // Insert
        let ast = Expression::Add(
            Box::new(Expression::Number(1.0)),
            Box::new(Expression::Number(2.0)),
        );
        cache.insert(key.clone(), ast.clone(), None);

        // Cache hit
        let cached = cache.get(&key).expect("cached entry should exist");
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cached.hit_count, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ExpressionCache::new(2);

        // Fill cache
        for i in 0..3 {
            let key = ExpressionKey {
                source: format!("expr{i}"),
                schema_id: None,
            };
            let ast = Expression::Number(crate::utils::usize_to_f64(i));
            cache.insert(key, ast, None);
        }

        // First expression should be evicted
        let key0 = ExpressionKey {
            source: "expr0".to_string(),
            schema_id: None,
        };
        assert!(cache.get(&key0).is_none());

        // Later expressions should still be there
        let key2 = ExpressionKey {
            source: "expr2".to_string(),
            schema_id: None,
        };
        assert!(cache.get(&key2).is_some());
    }

    #[test]
    fn test_global_cache_promotion() {
        let global = GlobalExpressionCache::new(10, 5).with_hot_threshold(3);

        let key = ExpressionKey {
            source: "hot_expr".to_string(),
            schema_id: None,
        };
        let ast = Expression::Number(42.0);

        // Insert into regular cache
        global.insert(key.clone(), ast, None);

        // Access multiple times to trigger promotion
        for _ in 0..4 {
            let _ = global.get(&key);
        }

        // Should now be in hot cache
        let stats = global.stats();
        assert!(stats.hot_cache.entries > 0);
    }
}
