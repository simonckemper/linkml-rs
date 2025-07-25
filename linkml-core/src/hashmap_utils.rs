//! Optimized HashMap utilities for LinkML
//!
//! This module provides efficient HashMap operations that minimize cloning
//! and leverage the Entry API for better performance.

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use indexmap::IndexMap;

/// Extension trait for HashMap with optimization utilities
pub trait HashMapExt<K, V> {
    /// Get or insert with a closure, avoiding unnecessary clones
    fn get_or_insert_with<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce() -> V;
    
    /// Merge another map efficiently without cloning if possible
    fn merge_from<'a>(&mut self, other: &'a HashMap<K, V>)
    where
        K: Clone + 'a,
        V: Clone + 'a;
    
    /// Update or insert, with access to old value
    fn update_or_insert<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce(Option<V>) -> V;
}

impl<K: Eq + Hash, V> HashMapExt<K, V> for HashMap<K, V> {
    fn get_or_insert_with<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.entry(key).or_insert_with(f)
    }
    
    fn merge_from<'a>(&mut self, other: &'a HashMap<K, V>)
    where
        K: Clone + 'a,
        V: Clone + 'a,
    {
        // Pre-allocate capacity
        self.reserve(other.len());
        
        for (k, v) in other {
            self.entry(k.clone())
                .or_insert_with(|| v.clone());
        }
    }
    
    fn update_or_insert<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce(Option<V>) -> V,
    {
        match self.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let old = e.insert(f(Some(e.remove())));
                e.insert(old)
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(f(None))
            }
        }
    }
}

/// Extension trait for IndexMap with optimization utilities
pub trait IndexMapExt<K, V> {
    /// Get or insert with a closure, avoiding unnecessary clones
    fn get_or_insert_with<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce() -> V;
    
    /// Merge from iterator efficiently
    fn merge_from_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>;
}

impl<K: Eq + Hash, V> IndexMapExt<K, V> for IndexMap<K, V> {
    fn get_or_insert_with<F>(&mut self, key: K, f: F) -> &mut V
    where
        F: FnOnce() -> V,
    {
        self.entry(key).or_insert_with(f)
    }
    
    fn merge_from_iter<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = (K, V)>,
    {
        for (k, v) in iter {
            self.entry(k).or_insert(v);
        }
    }
}

/// Efficient string key HashMap using Arc<str>
pub type StringMap<V> = HashMap<Arc<str>, V>;

/// Create a StringMap with pre-allocated capacity
pub fn string_map_with_capacity<V>(capacity: usize) -> StringMap<V> {
    HashMap::with_capacity(capacity)
}

/// Convert a HashMap<String, V> to HashMap<Arc<str>, V> efficiently
pub fn intern_keys<V>(map: HashMap<String, V>) -> HashMap<Arc<str>, V> {
    use crate::string_pool::intern;
    
    map.into_iter()
        .map(|(k, v)| (intern(&k), v))
        .collect()
}

/// Merge two HashMaps with Arc<str> keys efficiently
pub fn merge_arc_maps<V: Clone>(
    base: &HashMap<Arc<str>, V>,
    override_map: &HashMap<Arc<str>, V>,
) -> HashMap<Arc<str>, V> {
    let mut result = HashMap::with_capacity(base.len() + override_map.len());
    
    // Clone base map
    for (k, v) in base {
        result.insert(Arc::clone(k), v.clone());
    }
    
    // Override with new values
    for (k, v) in override_map {
        result.insert(Arc::clone(k), v.clone());
    }
    
    result
}

/// Collect keys that need removal without cloning during iteration
pub fn collect_keys_for_removal<K: Clone, V, F>(
    map: &HashMap<K, V>,
    predicate: F,
) -> Vec<K>
where
    F: Fn(&K, &V) -> bool,
{
    map.iter()
        .filter_map(|(k, v)| {
            if predicate(k, v) {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect()
}

/// Efficient HashMap builder with pre-allocation
pub struct HashMapBuilder<K, V> {
    map: HashMap<K, V>,
}

impl<K: Eq + Hash, V> HashMapBuilder<K, V> {
    /// Create new builder with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity),
        }
    }
    
    /// Insert a key-value pair
    pub fn insert(mut self, key: K, value: V) -> Self {
        self.map.insert(key, value);
        self
    }
    
    /// Insert if key doesn't exist
    pub fn insert_if_absent(mut self, key: K, value: V) -> Self {
        self.map.entry(key).or_insert(value);
        self
    }
    
    /// Build the HashMap
    pub fn build(self) -> HashMap<K, V> {
        self.map
    }
}

/// Cache for compiled patterns or expressions using Arc
pub struct ArcCache<K, V> {
    cache: HashMap<K, Arc<V>>,
    capacity: usize,
}

impl<K: Eq + Hash + Clone, V> ArcCache<K, V> {
    /// Create new cache with capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(capacity),
            capacity,
        }
    }
    
    /// Get or compute and cache a value
    pub fn get_or_compute<F>(&mut self, key: &K, compute: F) -> Arc<V>
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.cache.get(key) {
            return Arc::clone(value);
        }
        
        // Evict oldest if at capacity (simple FIFO)
        if self.cache.len() >= self.capacity {
            if let Some(first_key) = self.cache.keys().next().cloned() {
                self.cache.remove(&first_key);
            }
        }
        
        let value = Arc::new(compute());
        self.cache.insert(key.clone(), Arc::clone(&value));
        value
    }
    
    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hashmap_ext() {
        let mut map = HashMap::new();
        
        // Test get_or_insert_with
        let value = map.get_or_insert_with("key", || "value".to_string());
        assert_eq!(value, "value");
        
        // Should not call closure again
        let value2 = map.get_or_insert_with("key", || panic!("Should not be called"));
        assert_eq!(value2, "value");
    }

    #[test]
    fn test_arc_cache() {
        let mut cache = ArcCache::with_capacity(2);
        
        let v1 = cache.get_or_compute(&"key1", || "value1".to_string());
        let v2 = cache.get_or_compute(&"key1", || panic!("Should use cache"));
        
        assert!(Arc::ptr_eq(&v1, &v2));
    }

    #[test]
    fn test_merge_arc_maps() {
        let mut base = HashMap::new();
        base.insert(Arc::from("key1"), "value1");
        base.insert(Arc::from("key2"), "value2");
        
        let mut override_map = HashMap::new();
        override_map.insert(Arc::from("key2"), "new_value2");
        override_map.insert(Arc::from("key3"), "value3");
        
        let merged = merge_arc_maps(&base, &override_map);
        
        assert_eq!(merged.get(&Arc::from("key1")).unwrap(), &"value1");
        assert_eq!(merged.get(&Arc::from("key2")).unwrap(), &"new_value2");
        assert_eq!(merged.get(&Arc::from("key3")).unwrap(), &"value3");
    }
}