//! Caching infrastructure for compiled rules
//!
//! This module provides caching capabilities for compiled rules to improve
//! performance by avoiding repeated compilation of the same rules.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::types::CompiledRule;

/// Cache entry for compiled rules
#[derive(Debug, Clone)]
struct CacheEntry {
    rules: Vec<CompiledRule>,
    /// When this entry was created
    created_at: Instant,
    /// Last access time
    last_accessed: Instant,
    /// Number of times accessed
    access_count: usize,
}

/// Configuration for the rule cache
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// TTL for cache entries
    pub ttl: Duration,
    /// Whether to use LRU eviction
    pub use_lru: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl: Duration::from_secs(3600), // 1 hour
            use_lru: true,
        }
    }
}

impl CacheConfig {
    /// Create cache config from `LinkML` service configuration
    #[must_use]
    pub fn from_service_config(config: &linkml_core::configuration_v2::RuleCacheConfig) -> Self {
        Self {
            max_entries: config.max_entries,
            ttl: Duration::from_secs(config.ttl_seconds),
            use_lru: true, // Default to LRU for rule cache
        }
    }
}

/// Cache for compiled rules
pub struct RuleCache {
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Cache configuration
    config: CacheConfig,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: usize,
    /// Total number of cache misses
    pub misses: usize,
    /// Total number of evictions
    pub evictions: usize,
    /// Current number of entries
    pub entries: usize,
}

impl Default for RuleCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RuleCache {
    /// Create a new rule cache with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new rule cache with custom configuration
    #[must_use]
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get compiled rules from the cache
    #[must_use]
    pub fn get(&self, class_name: &str) -> Option<Vec<CompiledRule>> {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        if let Some(entry) = cache.get_mut(class_name) {
            // Check if entry is still valid
            if entry.created_at.elapsed() > self.config.ttl {
                // Entry expired
                cache.remove(class_name);
                stats.entries = cache.len();
                stats.misses += 1;
                return None;
            }

            // Update access time and count
            entry.last_accessed = Instant::now();
            entry.access_count += 1;

            stats.hits += 1;
            Some(entry.rules.clone())
        } else {
            stats.misses += 1;
            None
        }
    }

    /// Put compiled rules into the cache
    pub fn put(&self, class_name: String, rules: Vec<CompiledRule>) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        // Check if we need to evict entries
        if cache.len() >= self.config.max_entries && !cache.contains_key(&class_name) {
            self.evict_entry(&mut cache, &mut stats);
        }

        // Insert new entry
        let entry = CacheEntry {
            rules,
            created_at: Instant::now(),
            last_accessed: Instant::now(),
            access_count: 0,
        };

        cache.insert(class_name, entry);
        stats.entries = cache.len();
    }

    /// Clear the entire cache
    pub fn clear(&self) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        cache.clear();
        stats.entries = 0;
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Evict an entry based on the eviction policy
    fn evict_entry(&self, cache: &mut HashMap<String, CacheEntry>, stats: &mut CacheStats) {
        if cache.is_empty() {
            return;
        }

        let key_to_evict = if self.config.use_lru {
            // Find least recently used entry
            cache
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone())
        } else {
            // Find oldest entry
            cache
                .iter()
                .min_by_key(|(_, entry)| entry.created_at)
                .map(|(key, _)| key.clone())
        };

        if let Some(key) = key_to_evict {
            cache.remove(&key);
            stats.evictions += 1;
        }
    }

    /// Remove expired entries
    pub fn cleanup_expired(&self) {
        let mut cache = self.cache.write();
        let mut stats = self.stats.write();

        let expired_keys: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| entry.created_at.elapsed() > self.config.ttl)
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            cache.remove(&key);
            stats.evictions += 1;
        }

        stats.entries = cache.len();
    }
}

/// A thread-safe, shared rule cache
pub type SharedRuleCache = Arc<RuleCache>;

/// Create a new shared rule cache
#[must_use]
pub fn create_shared_cache() -> SharedRuleCache {
    Arc::new(RuleCache::new())
}

/// Create a new shared rule cache with configuration
#[must_use]
pub fn create_shared_cache_with_config(config: CacheConfig) -> SharedRuleCache {
    Arc::new(RuleCache::with_config(config))
}

impl CacheStats {
    /// Calculate the cache hit rate
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

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::Rule;

    fn create_test_rule() -> Result<CompiledRule, Box<dyn std::error::Error>> {
        Ok(CompiledRule::compile(
            Rule {
                description: Some("Test rule".to_string()),
                ..Default::default()
            },
            "TestClass".to_string(),
        )?)
    }

    #[test]
    fn test_cache_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
        let cache = RuleCache::new();
        let rules = vec![create_test_rule()?];

        // Test miss
        assert!(cache.get("TestClass").is_none());
        assert_eq!(cache.stats().misses, 1);

        // Test put and hit
        cache.put("TestClass".to_string(), rules.clone());
        assert_eq!(cache.stats().entries, 1);

        let retrieved = cache
            .get("TestClass")
            .ok_or_else(|| anyhow::anyhow!("should retrieve cached rule"))?;
        assert_eq!(retrieved.len(), 1);
        assert_eq!(cache.stats().hits, 1);
        Ok(())
    }

    #[test]
    fn test_cache_expiration() -> Result<(), Box<dyn std::error::Error>> {
        let config = CacheConfig {
            ttl: Duration::from_millis(100),
            ..Default::default()
        };
        let cache = RuleCache::with_config(config);
        let rules = vec![create_test_rule()?];

        cache.put("TestClass".to_string(), rules);

        // Should hit initially
        assert!(cache.get("TestClass").is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Should miss after expiration
        assert!(cache.get("TestClass").is_none());
        assert_eq!(cache.stats().entries, 0);
        Ok(())
    }

    #[test]
    fn test_cache_eviction() -> Result<(), Box<dyn std::error::Error>> {
        let config = CacheConfig {
            max_entries: 2,
            use_lru: true,
            ..Default::default()
        };
        let cache = RuleCache::with_config(config);

        // Fill cache
        cache.put("Class1".to_string(), vec![create_test_rule()?]);
        cache.put("Class2".to_string(), vec![create_test_rule()?]);

        // Access Class2 to make it more recently used
        let _ = cache.get("Class2");

        // Add third entry, should evict Class1 (LRU)
        cache.put("Class3".to_string(), vec![create_test_rule()?]);

        assert_eq!(cache.stats().entries, 2);
        assert_eq!(cache.stats().evictions, 1);
        assert!(cache.get("Class1").is_none());
        assert!(cache.get("Class2").is_some());
        assert!(cache.get("Class3").is_some());
        Ok(())
    }
}
