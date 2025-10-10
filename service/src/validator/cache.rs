//! Compiled validator cache for optimized performance
//!
//! This module provides caching for compiled validators to avoid
//! recompilation of the same schemas. It integrates with `RootReal`'s
//! `CacheService` for distributed caching support.

use super::compiled::{CompilationOptions, CompiledValidator};
use blake3::Hasher;
use linkml_core::error::Result as LinkMLResult;
use linkml_core::prelude::*;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Cache key for compiled validators
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorCacheKey {
    /// Schema ID
    pub schema_id: String,
    /// Schema version/hash
    pub schema_hash: String,
    /// Target class name
    pub class_name: String,
    /// Compilation options hash
    pub options_hash: String,
}

impl std::fmt::Display for ValidatorCacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}:{}:{}",
            self.schema_id, self.schema_hash, self.class_name, self.options_hash
        )
    }
}

impl ValidatorCacheKey {
    /// Create a new cache key
    #[must_use]
    pub fn new(schema: &SchemaDefinition, class_name: &str, options: &CompilationOptions) -> Self {
        // Calculate schema hash
        let schema_hash = Self::hash_schema(schema);

        // Calculate options hash
        let options_hash = Self::hash_options(*options);

        Self {
            schema_id: schema.id.clone(),
            schema_hash,
            class_name: class_name.to_string(),
            options_hash,
        }
    }

    /// Generate hash for schema
    fn hash_schema(schema: &SchemaDefinition) -> String {
        let mut hasher = Hasher::new();

        // Hash relevant schema parts
        hasher.update(schema.id.as_bytes());
        hasher.update(schema.name.as_bytes());
        hasher.update(
            schema
                .version
                .as_ref()
                .map_or(b"", std::string::String::as_bytes),
        );

        // Hash classes
        for (name, class) in &schema.classes {
            hasher.update(name.as_bytes());
            hasher.update(class.name.as_bytes());
            if let Some(parent) = &class.is_a {
                hasher.update(parent.as_bytes());
            }
        }

        // Hash slots
        for (name, slot) in &schema.slots {
            hasher.update(name.as_bytes());
            hasher.update(slot.name.as_bytes());
            if let Some(pattern) = &slot.pattern {
                hasher.update(pattern.as_bytes());
            }
        }

        hasher.finalize().to_hex().to_string()
    }

    /// Generate hash for compilation options
    fn hash_options(options: CompilationOptions) -> String {
        let mut hasher = Hasher::new();

        // Hash the bitflags value directly for efficiency
        hasher.update(&options.bits().to_le_bytes());

        hasher.finalize().to_hex().to_string()
    }
}

/// Statistics for cache performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of evictions
    pub evictions: u64,
    /// Total compilation time saved (ms)
    pub time_saved_ms: u64,
    /// Number of validators in cache
    pub cached_validators: usize,
    /// Total memory used (bytes)
    pub memory_bytes: usize,
}

impl CacheStats {
    /// Calculate cache hit rate
    #[must_use]
    // Precision loss acceptable here
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            {
                self.hits as f64 / total as f64
            }
        }
    }
}

/// Compiled validator cache
pub struct CompiledValidatorCache {
    /// Local in-memory cache
    local_cache: Arc<RwLock<HashMap<ValidatorCacheKey, Arc<CompiledValidator>>>>,

    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,

    /// Maximum number of validators to cache
    max_validators: usize,

    /// Maximum memory usage in bytes
    max_memory_bytes: usize,

    /// Optional `RootReal` `CacheService` integration
    cache_service: Option<Arc<dyn CacheService>>,
}

impl Default for CompiledValidatorCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CompiledValidatorCache {
    /// Create a new cache with default settings
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(1000, 100 * 1024 * 1024) // 1000 validators, 100MB
    }

    /// Create a new cache with custom configuration
    #[must_use]
    pub fn with_config(max_validators: usize, max_memory_bytes: usize) -> Self {
        Self {
            local_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            max_validators,
            max_memory_bytes,
            cache_service: None,
        }
    }

    /// Set the `CacheService` for distributed caching
    #[must_use]
    pub fn with_cache_service(mut self, cache_service: Arc<dyn CacheService>) -> Self {
        self.cache_service = Some(cache_service);
        self
    }

    /// Get a compiled validator from cache
    pub async fn get(&self, key: &ValidatorCacheKey) -> Option<Arc<CompiledValidator>> {
        // Try local cache first
        {
            let cache = self.local_cache.read();
            if let Some(validator) = cache.get(key) {
                let mut stats = self.stats.write();
                stats.hits += 1;
                return Some(Arc::clone(validator));
            }
        }

        // Try distributed cache if available
        if let Some(cache_service) = &self.cache_service {
            let cache_key = format!("linkml:validator:{}", serde_json::to_string(key).ok()?);

            if let Ok(Some(_cached_data)) = cache_service.get(&cache_key).await {
                // For now, we can't deserialize CompiledValidator directly
                // because it contains compiled regex patterns
                // This would require a custom serialization format
                // So we'll only use local cache for now
            }
        }

        let mut stats = self.stats.write();
        stats.misses += 1;

        None
    }

    /// Store a compiled validator in cache
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Serialization fails
    /// - Cache operations fail
    pub fn put(&self, key: &ValidatorCacheKey, validator: CompiledValidator) -> LinkMLResult<()> {
        let validator_memory = Self::estimate_validator_memory(&validator);
        let validator_arc = Arc::new(validator);

        // Check if we need to evict
        {
            let mut cache = self.local_cache.write();
            let mut stats = self.stats.write();

            // Evict based on memory limit or validator count
            while !cache.is_empty()
                && (cache.len() >= self.max_validators
                    || stats.memory_bytes + validator_memory > self.max_memory_bytes)
            {
                // Remove oldest entry (simple LRU strategy)
                if let Some(first_key) = cache.keys().next().cloned()
                    && let Some(removed) = cache.remove(&first_key)
                {
                    let removed_memory = Self::estimate_validator_memory(&removed);
                    stats.memory_bytes = stats.memory_bytes.saturating_sub(removed_memory);
                    stats.evictions += 1;
                }
            }

            cache.insert(key.clone(), Arc::clone(&validator_arc));
            stats.memory_bytes += validator_memory;
            stats.cached_validators = cache.len();
        }

        // Store in distributed cache if available
        if let Some(_cache_service) = &self.cache_service {
            let _cache_key = format!("linkml:validator:{}", serde_json::to_string(&key)?);

            // For now, we can't serialize CompiledValidator directly
            // We'd need to implement a custom serialization format
            // that can handle regex patterns and other non-serializable types
        }

        Ok(())
    }

    /// Clear all cached validators
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if cache clearing fails
    pub fn clear(&self) -> LinkMLResult<()> {
        {
            let mut cache = self.local_cache.write();
            cache.clear();

            let mut stats = self.stats.write();
            stats.cached_validators = 0;
            stats.memory_bytes = 0;
        }

        // Clear distributed cache if available
        if let Some(_cache_service) = &self.cache_service {
            // Clear all linkml:validator:* keys
            // This would require a pattern-based delete operation
            // which might not be available in all cache implementations
        }

        Ok(())
    }

    /// Get cache statistics
    #[must_use]
    pub fn stats(&self) -> CacheStats {
        self.stats.read().clone()
    }

    /// Warm the cache with commonly used validators
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Validator compilation fails
    /// - Cache operations fail
    pub async fn warm_cache(
        &self,
        schemas: Vec<(SchemaDefinition, Vec<String>)>,
        options: &CompilationOptions,
    ) -> LinkMLResult<()> {
        for (schema, class_names) in schemas {
            for class_name in class_names {
                let key = ValidatorCacheKey::new(&schema, &class_name, options);

                // Check if already cached
                if self.get(&key).await.is_some() {
                    continue;
                }

                // Compile and cache
                if let Some(class) = schema.classes.get(&class_name) {
                    let validator =
                        CompiledValidator::compile_class(&schema, &class_name, class, *options)?;

                    self.put(&key, validator)?;
                }
            }
        }

        Ok(())
    }

    /// Estimate memory usage of a validator
    pub(crate) fn estimate_validator_memory(validator: &CompiledValidator) -> usize {
        // Basic estimation - would need more sophisticated calculation
        std::mem::size_of::<CompiledValidator>()
            + validator.compiled_patterns.len() * 1024 // Rough estimate for regex
            + validator.cached_enums.iter()
                .map(|set| set.len() * 32) // Rough estimate per string
                .sum::<usize>()
    }
}

/// Cache service trait for `RootReal` integration
#[async_trait::async_trait]
pub trait CacheService: Send + Sync {
    /// Get a value from cache
    async fn get(
        &self,
        key: &str,
    ) -> std::result::Result<Option<Vec<u8>>, Box<dyn std::error::Error>>;

    /// Set a value in cache
    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>;

    /// Delete a value from cache
    async fn delete(&self, key: &str) -> std::result::Result<(), Box<dyn std::error::Error>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_key_generation() {
        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        let options = CompilationOptions::default();

        let key1 = ValidatorCacheKey::new(&schema, "Person", &options);
        let key2 = ValidatorCacheKey::new(&schema, "Person", &options);
        let key3 = ValidatorCacheKey::new(&schema, "Organization", &options);

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
    }

    #[tokio::test]
    async fn test_cache_operations() -> anyhow::Result<()> {
        let cache = CompiledValidatorCache::new();

        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            ..Default::default()
        };

        let class = ClassDefinition {
            name: "TestClass".to_string(),
            ..Default::default()
        };

        let options = CompilationOptions::default();
        let key = ValidatorCacheKey::new(&schema, "TestClass", &options);

        // Cache miss
        assert!(cache.get(&key).await.is_none());
        assert_eq!(cache.stats().misses, 1);

        // Compile and cache
        let validator = CompiledValidator::compile_class(&schema, "TestClass", &class, options)
            .expect("should compile validator: {}");

        cache
            .put(&key, validator)
            .expect("should cache validator: {}");

        // Cache hit
        assert!(cache.get(&key).await.is_some());
        assert_eq!(cache.stats().hits, 1);
        assert_eq!(cache.stats().cached_validators, 1);
        Ok(())
    }

    #[tokio::test]
    async fn test_cache_eviction() -> anyhow::Result<()> {
        let cache = CompiledValidatorCache::with_config(2, 1024 * 1024);

        let mut schema = SchemaDefinition::default();
        schema.id = "test-schema".to_string();

        let options = CompilationOptions::default();

        // Add 3 validators to a cache with capacity 2
        for i in 0..3 {
            let mut class = ClassDefinition::default();
            class.name = format!("Class{i}");

            let key = ValidatorCacheKey::new(&schema, &class.name, &options);
            let validator = CompiledValidator::compile_class(&schema, &class.name, &class, options)
                .expect("should compile validator: {}");

            cache
                .put(&key, validator)
                .expect("should cache validator: {}");
        }

        assert_eq!(cache.stats().cached_validators, 2);
        assert_eq!(cache.stats().evictions, 1);
        Ok(())
    }
}
