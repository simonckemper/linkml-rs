//! Optimized expression cache using HashMap utilities
//!
//! This module provides an optimized version of the expression cache that
//! minimizes cloning and uses more efficient collection operations.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use std::collections::HashMap;

use linked_hash_map::LinkedHashMap;
use linkml_core::{
    error::Result,
    hashmap_utils::{collect_keys_for_removal, ArcCache},
    string_pool::intern,
};

use super::{Expression, ParsedExpression};

/// Cached expression entry with metadata
#[derive(Clone)]
struct CacheEntryV2 {
    /// The parsed expression (using Arc for sharing)
    expression: Arc<ParsedExpression>,
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
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub entries: usize,
    pub size_bytes: usize,
}

impl CacheStats {
    /// Calculate hit rate
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
    /// The actual cache storage (LRU)
    cache: Arc<RwLock<LinkedHashMap<Arc<str>, CacheEntryV2>>>,
    /// Maximum number of entries
    capacity: usize,
    /// Maximum age for entries
    max_age: Duration,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl ExpressionCacheV2 {
    /// Create a new expression cache
    pub fn new(capacity: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(LinkedHashMap::with_capacity(capacity))),
            capacity,
            max_age: Duration::from_secs(3600), // 1 hour default
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }
    
    /// Set maximum age for cache entries
    pub fn with_max_age(mut self, max_age: Duration) -> Self {
        self.max_age = max_age;
        self
    }
    
    /// Get a parsed expression from cache
    pub fn get(&self, expression: &str) -> Option<Arc<ParsedExpression>> {
        let key = intern(expression);
        let mut cache = self.cache.write().ok()?;
        let mut stats = self.stats.write().ok()?;
        
        if let Some(entry) = cache.get_mut(&key) {
            // Update access time and count
            entry.last_accessed = Instant::now();
            entry.hit_count += 1;
            stats.hits += 1;
            
            // Move to end (most recently used)
            let cloned = entry.clone();
            cache.remove(&key);
            cache.insert(Arc::clone(&key), cloned);
            
            Some(Arc::clone(&entry.expression))
        } else {
            stats.misses += 1;
            None
        }
    }
    
    /// Store a parsed expression in cache
    pub fn put(&self, expression: &str, parsed: ParsedExpression) {
        let key = intern(expression);
        let now = Instant::now();
        
        let mut cache = self.cache.write().map_err(|e| anyhow::anyhow!("cache lock poisoned": {}, e))?;
        let mut stats = self.stats.write().map_err(|e| anyhow::anyhow!("stats lock poisoned": {}, e))?;
        
        // Check if we need to evict
        if cache.len() >= self.capacity {
            // Remove least recently used
            if let Some((old_key, _)) = cache.pop_front() {
                stats.evictions += 1;
                drop(old_key); // Release the Arc<str>
            }
        }
        
        // Insert new entry
        let entry = CacheEntryV2 {
            expression: Arc::new(parsed),
            created: now,
            last_accessed: now,
            hit_count: 0,
        };
        
        cache.insert(key, entry);
        stats.entries = cache.len();
        stats.size_bytes = stats.entries * std::mem::size_of::<CacheEntryV2>();
    }
    
    /// Get or compute and cache an expression
    pub fn get_or_compute<F>(&self, expression: &str, compute: F) -> Result<Arc<ParsedExpression>>
    where
        F: FnOnce() -> Result<ParsedExpression>,
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
            .ok_or_else(|| linkml_core::error::LinkMLError::internal("Cache put/get mismatch"))
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.write().map_err(|e| anyhow::anyhow!("cache lock poisoned": {}, e))?;
        let mut stats = self.stats.write().map_err(|e| anyhow::anyhow!("stats lock poisoned": {}, e))?;
        
        cache.clear();
        *stats = CacheStats::default();
    }
    
    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().map_err(|e| anyhow::anyhow!("stats lock poisoned": {}, e))?.clone()
    }
    
    /// Clean up old entries (optimized version)
    pub fn cleanup(&self) {
        let mut cache = self.cache.write().map_err(|e| anyhow::anyhow!("cache lock poisoned": {}, e))?;
        let mut stats = self.stats.write().map_err(|e| anyhow::anyhow!("stats lock poisoned": {}, e))?;
        
        let now = Instant::now();
        
        // Collect keys to remove without cloning during iteration
        let to_remove = collect_keys_for_removal(
            &cache.iter().map(|(k, v)| (k.clone(), v)).collect::<HashMap<_, _>>(),
            |_key, entry| now.duration_since(entry.last_accessed) > self.max_age
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

/// Global expression cache using ArcCache
pub struct GlobalExpressionCacheV2 {
    /// Primary cache for all expressions
    primary: ArcCache<Arc<str>, ParsedExpression>,
    /// Hot cache for frequently used expressions
    hot: ArcCache<Arc<str>, ParsedExpression>,
    /// Access counter for promotion
    access_counts: Arc<RwLock<HashMap<Arc<str>, u64>>>,
    /// Threshold for hot promotion
    hot_threshold: u64,
}

impl GlobalExpressionCacheV2 {
    /// Create a new global cache
    pub fn new(primary_capacity: usize, hot_capacity: usize) -> Self {
        Self {
            primary: ArcCache::with_capacity(primary_capacity),
            hot: ArcCache::with_capacity(hot_capacity),
            access_counts: Arc::new(RwLock::new(HashMap::new())),
            hot_threshold: 10,
        }
    }
    
    /// Get or compute an expression
    pub fn get_or_compute<F>(
        &mut self,
        expression: &str,
        compute: F,
    ) -> Result<Arc<ParsedExpression>>
    where
        F: FnOnce() -> Result<ParsedExpression>,
    {
        let key = intern(expression);
        
        // Check hot cache first
        if let Ok(counts) = self.access_counts.read() {
            if let Some(&count) = counts.get(&key) {
                if count >= self.hot_threshold {
                    return match compute() {
                        Ok(parsed) => Ok(self.hot.get_or_compute(&key, || parsed)),
                        Err(e) => Err(e),
                    };
                }
            }
        }
        
        // Update access count
        if let Ok(mut counts) = self.access_counts.write() {
            let count = counts.entry(Arc::clone(&key)).or_insert(0);
            *count += 1;
        }
        
        // Use primary cache
        self.primary.get_or_compute(&key, compute)
    }
    
    /// Clear all caches
    pub fn clear(&mut self) {
        self.primary.clear();
        self.hot.clear();
        self.access_counts.write().map_err(|e| anyhow::anyhow!("access_counts lock poisoned": {}, e))?.clear();
    }
}

/// Thread-safe wrapper for global cache
pub struct ThreadSafeGlobalCache {
    inner: Arc<RwLock<GlobalExpressionCacheV2>>,
}

impl ThreadSafeGlobalCache {
    /// Create new thread-safe cache
    pub fn new(primary_capacity: usize, hot_capacity: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(GlobalExpressionCacheV2::new(
                primary_capacity,
                hot_capacity,
            ))),
        }
    }
    
    /// Get or compute with thread safety
    pub fn get_or_compute<F>(
        &self,
        expression: &str,
        compute: F,
    ) -> Result<Arc<ParsedExpression>>
    where
        F: FnOnce() -> Result<ParsedExpression>,
    {
        self.inner
            .write()
            .map_err(|e| anyhow::anyhow!("inner cache lock poisoned": {}, e))?
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
        
        let parsed = ParsedExpression::default(); // Assuming default impl
        cache.put("test", parsed.clone());
        
        assert!(cache.get("test").is_some());
        
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_lru_eviction() {
        let cache = ExpressionCacheV2::new(2);
        
        cache.put("expr1", ParsedExpression::default());
        cache.put("expr2", ParsedExpression::default());
        cache.put("expr3", ParsedExpression::default()); // Should evict expr1
        
        assert!(cache.get("expr1").is_none());
        assert!(cache.get("expr2").is_some());
        assert!(cache.get("expr3").is_some());
        
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cleanup() {
        let cache = ExpressionCacheV2::new(10)
            .with_max_age(Duration::from_millis(100));
        
        cache.put("old", ParsedExpression::default());
        
        // Wait for expiry
        std::thread::sleep(Duration::from_millis(150));
        
        cache.cleanup();
        
        assert!(cache.get("old").is_none());
        let stats = cache.stats();
        assert_eq!(stats.evictions, 1);
    }
}