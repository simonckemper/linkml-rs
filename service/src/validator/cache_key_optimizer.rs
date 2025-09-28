//! Optimized cache key generation for `LinkML` validation
//!
//! This module provides efficient cache key generation with:
//! - Minimal allocations through careful string handling
//! - Fast hashing using xxHash for non-cryptographic use cases
//! - Hierarchical key structure for efficient invalidation
//! - Zero-copy operations where possible

use super::cache::ValidatorCacheKey;
use dashmap::DashMap;
use linkml_core::types::SchemaDefinition;
use smallvec::SmallVec;
use std::borrow::Cow;
use std::convert::Infallible;
use std::str::FromStr;

/// Fast non-cryptographic hash for cache keys
pub use xxhash_rust::xxh3::xxh3_64 as fast_hash;

/// Small vector optimization for typical key components (usually < 8)
type KeyComponents = SmallVec<[Cow<'static, str>; 8]>;

/// Optimized cache key builder with minimal allocations
#[derive(Debug, Clone)]
pub struct CacheKeyBuilder {
    /// Pre-allocated buffer for key construction
    buffer: String,
    /// Components to be joined
    components: KeyComponents,
    /// Separator character
    separator: char,
}

impl Default for CacheKeyBuilder {
    fn default() -> Self {
        Self {
            buffer: String::with_capacity(128), // Pre-allocate typical key size
            components: KeyComponents::new(),
            separator: ':',
        }
    }
}

impl CacheKeyBuilder {
    /// Create a new cache key builder
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with custom initial capacity
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            components: KeyComponents::new(),
            separator: ':',
        }
    }

    /// Add a static component (zero allocation)
    pub fn add_static(&mut self, component: &'static str) -> &mut Self {
        self.components.push(Cow::Borrowed(component));
        self
    }

    /// Add an owned component
    pub fn add_owned(&mut self, component: String) -> &mut Self {
        self.components.push(Cow::Owned(component));
        self
    }

    /// Add a component that might be static or owned
    pub fn add<'a>(&mut self, component: impl Into<Cow<'a, str>>) -> &mut Self
    where
        'a: 'static,
    {
        self.components.push(component.into().into_owned().into());
        self
    }

    /// Add a numeric component (efficient formatting)
    pub fn add_num(&mut self, num: impl std::fmt::Display) -> &mut Self {
        self.components.push(Cow::Owned(num.to_string()));
        self
    }

    /// Build the final key with minimal allocations
    pub fn build(&mut self) -> String {
        self.buffer.clear();

        for (i, component) in self.components.iter().enumerate() {
            if i > 0 {
                self.buffer.push(self.separator);
            }
            self.buffer.push_str(component);
        }

        // Clear components for reuse
        self.components.clear();

        // Return the built key
        std::mem::take(&mut self.buffer)
    }

    /// Build and compute hash in one operation
    pub fn build_hash(&mut self) -> (String, u64) {
        let key = self.build();
        let hash = fast_hash(key.as_bytes());
        (key, hash)
    }
}

/// Hierarchical cache key for efficient invalidation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HierarchicalCacheKey {
    /// Full key
    pub full_key: String,
    /// Key hash for fast comparison
    pub key_hash: u64,
    /// Hierarchy levels for invalidation
    pub hierarchy: SmallVec<[String; 4]>,
}

impl HierarchicalCacheKey {
    /// Create a new hierarchical cache key
    #[must_use]
    pub fn new(schema_id: &str, class_name: &str, slot_name: Option<&str>) -> Self {
        let mut builder = CacheKeyBuilder::new();
        let mut hierarchy = SmallVec::new();

        // Build hierarchy
        builder.add_static("linkml:validator");
        hierarchy.push("linkml:validator".to_string());

        builder.add_owned(schema_id.to_string());
        hierarchy.push(format!("linkml:validator:{schema_id}"));

        builder.add_owned(class_name.to_string());
        hierarchy.push(format!("linkml:validator:{schema_id}:{class_name}"));

        if let Some(slot) = slot_name {
            builder.add_owned(slot.to_string());
            hierarchy.push(format!("linkml:validator:{schema_id}:{class_name}:{slot}"));
        }

        let (full_key, key_hash) = builder.build_hash();

        Self {
            full_key,
            key_hash,
            hierarchy,
        }
    }

    /// Get parent key at specific level
    #[must_use]
    pub fn parent_at_level(&self, level: usize) -> Option<&str> {
        self.hierarchy.get(level).map(std::string::String::as_str)
    }

    /// Check if this key is a child of another key
    #[must_use]
    pub fn is_child_of(&self, parent: &str) -> bool {
        self.hierarchy.iter().any(|h| h.starts_with(parent))
    }
}

/// Optimized validator cache key with zero-copy support
pub struct OptimizedCacheKey<'a> {
    /// Schema ID (can be borrowed or owned)
    pub schema_id: Cow<'a, str>,
    /// Schema hash (pre-computed)
    pub schema_hash: u64,
    /// Class name (can be borrowed or owned)
    pub class_name: Cow<'a, str>,
    /// Options hash (pre-computed)
    pub options_hash: u64,
}

impl<'a> OptimizedCacheKey<'a> {
    /// Create from borrowed data (zero allocation)
    #[must_use]
    pub fn borrowed(
        schema_id: &'a str,
        schema_hash: u64,
        class_name: &'a str,
        options_hash: u64,
    ) -> Self {
        Self {
            schema_id: Cow::Borrowed(schema_id),
            schema_hash,
            class_name: Cow::Borrowed(class_name),
            options_hash,
        }
    }

    /// Convert to owned key
    #[must_use]
    pub fn into_owned(self) -> ValidatorCacheKey {
        ValidatorCacheKey {
            schema_id: self.schema_id.into_owned(),
            schema_hash: self.schema_hash.to_string(),
            class_name: self.class_name.into_owned(),
            options_hash: self.options_hash.to_string(),
        }
    }

    /// Fast comparison using hashes
    #[must_use]
    pub fn fast_eq(&self, other: &Self) -> bool {
        self.schema_hash == other.schema_hash
            && self.options_hash == other.options_hash
            && self.schema_id == other.schema_id
            && self.class_name == other.class_name
    }
}

/// Cache key optimizer for efficient key generation
pub struct CacheKeyOptimizer {
    /// Cache for schema hashes
    schema_hash_cache: DashMap<String, u64>,
}

impl Default for CacheKeyOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheKeyOptimizer {
    /// Create a new cache key optimizer
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema_hash_cache: DashMap::new(),
        }
    }

    /// Generate an optimized cache key
    pub fn generate_key(
        &mut self,
        schema: &SchemaDefinition,
        class_name: &str,
        options_hash: u64,
    ) -> ValidatorCacheKey {
        // Check if we have a cached schema hash
        let schema_hash = if let Some(hash) = self.schema_hash_cache.get(&schema.id) {
            *hash
        } else {
            // Compute schema hash efficiently
            let hash = Self::compute_schema_hash(schema);
            self.schema_hash_cache.insert(schema.id.clone(), hash);
            hash
        };

        ValidatorCacheKey {
            schema_id: schema.id.clone(),
            schema_hash: schema_hash.to_string(),
            class_name: class_name.to_string(),
            options_hash: options_hash.to_string(),
        }
    }

    /// Generate a hierarchical cache key
    pub fn generate_hierarchical_key(
        &mut self,
        schema_id: &str,
        class_name: &str,
        slot_name: Option<&str>,
    ) -> HierarchicalCacheKey {
        HierarchicalCacheKey::new(schema_id, class_name, slot_name)
    }

    /// Compute schema hash efficiently
    fn compute_schema_hash(schema: &SchemaDefinition) -> u64 {
        // Use xxHash for fast, non-cryptographic hashing
        let mut hasher_input = Vec::with_capacity(1024);

        // Include key schema elements
        hasher_input.extend_from_slice(schema.id.as_bytes());
        hasher_input.extend_from_slice(&[0]); // Separator

        if let Some(version) = &schema.version {
            hasher_input.extend_from_slice(version.as_bytes());
        }
        hasher_input.extend_from_slice(&[0]); // Separator

        // Include class count and names for change detection
        hasher_input.extend_from_slice(
            &u32::try_from(schema.classes.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        for (name, _) in &schema.classes {
            hasher_input.extend_from_slice(name.as_bytes());
            hasher_input.extend_from_slice(&[0]);
        }

        fast_hash(&hasher_input)
    }

    /// Batch generate keys for multiple classes (efficient)
    pub fn generate_batch_keys(
        &mut self,
        schema: &SchemaDefinition,
        class_names: &[String],
        options_hash: u64,
    ) -> Vec<ValidatorCacheKey> {
        // Pre-compute schema hash once
        let schema_hash = if let Some(hash) = self.schema_hash_cache.get(&schema.id) {
            *hash
        } else {
            let hash = Self::compute_schema_hash(schema);
            self.schema_hash_cache.insert(schema.id.clone(), hash);
            hash
        };

        // Generate keys with minimal allocations
        class_names
            .iter()
            .map(|class_name| ValidatorCacheKey {
                schema_id: schema.id.clone(),
                schema_hash: schema_hash.to_string(),
                class_name: class_name.clone(),
                options_hash: options_hash.to_string(),
            })
            .collect()
    }
}

/// Cache key patterns for efficient matching
#[derive(Debug, Clone)]
pub struct CacheKeyPattern {
    /// Pattern components
    components: Vec<PatternComponent>,
}

#[derive(Debug, Clone)]
enum PatternComponent {
    /// Exact match
    Exact(String),
    /// Wildcard match
    Wildcard,
    /// Prefix match
    Prefix(String),
}

impl CacheKeyPattern {
    /// Create a pattern from string
    #[must_use]
    pub fn parse(pattern: &str) -> Self {
        let components = pattern
            .split(':')
            .map(|part| {
                if part == "*" {
                    PatternComponent::Wildcard
                } else if let Some(prefix) = part.strip_suffix('*') {
                    PatternComponent::Prefix(prefix.to_string())
                } else {
                    PatternComponent::Exact(part.to_string())
                }
            })
            .collect();

        Self { components }
    }

    /// Check if a key matches this pattern
    #[must_use]
    pub fn matches(&self, key: &str) -> bool {
        let key_parts: Vec<&str> = key.split(':').collect();

        if self.components.len() != key_parts.len() {
            return false;
        }

        for (pattern, key_part) in self.components.iter().zip(key_parts.iter()) {
            match pattern {
                PatternComponent::Exact(s) => {
                    if s != key_part {
                        return false;
                    }
                }
                PatternComponent::Wildcard => {
                    // Always matches
                }
                PatternComponent::Prefix(prefix) => {
                    if !key_part.starts_with(prefix) {
                        return false;
                    }
                }
            }
        }

        true
    }
}

impl FromStr for CacheKeyPattern {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_builder() {
        let mut builder = CacheKeyBuilder::new();
        let key = builder
            .add_static("linkml")
            .add_static("validator")
            .add_owned("schema123".to_string())
            .add_num(42)
            .build();

        assert_eq!(key, "linkml:validator:schema123:42");
    }

    #[test]
    fn test_hierarchical_key() {
        let key = HierarchicalCacheKey::new("schema1", "Person", Some("name"));

        assert_eq!(key.hierarchy.len(), 4);
        assert!(key.is_child_of("linkml:validator:schema1"));
        assert!(!key.is_child_of("linkml:validator:schema2"));
    }

    #[test]
    fn test_cache_key_pattern() {
        let pattern = CacheKeyPattern::parse("linkml:validator:*:Person");

        assert!(pattern.matches("linkml:validator:schema1:Person"));
        assert!(pattern.matches("linkml:validator:schema2:Person"));
        assert!(!pattern.matches("linkml:validator:schema1:Animal"));
    }

    #[test]
    fn test_optimized_cache_key() {
        let key1 = OptimizedCacheKey::borrowed("schema1", 12345, "Person", 67890);
        let key2 = OptimizedCacheKey::borrowed("schema1", 12345, "Person", 67890);
        let key3 = OptimizedCacheKey::borrowed("schema2", 12345, "Person", 67890);

        assert!(key1.fast_eq(&key2));
        assert!(!key1.fast_eq(&key3));
    }
}
