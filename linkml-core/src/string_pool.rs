//! String interning pool for LinkML to reduce memory usage
//!
//! This module provides a thread-safe string interning system to deduplicate
//! common strings across schema definitions, significantly reducing memory usage.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;

/// Global string pool for interning common LinkML strings
static STRING_POOL: Lazy<StringPool> = Lazy::new(StringPool::new);

/// Thread-safe string interning pool
pub struct StringPool {
    pool: RwLock<HashMap<String, Arc<str>>>,
}

impl StringPool {
    /// Create a new string pool
    pub fn new() -> Self {
        Self {
            pool: RwLock::new(HashMap::new()),
        }
    }

    /// Intern a string and return an Arc<str>
    pub fn intern(&self, s: &str) -> Arc<str> {
        // Try read lock first for common case
        {
            let pool = self.pool.read().expect("StringPool read lock poisoned");
            if let Some(interned) = pool.get(s) {
                return Arc::clone(interned);
            }
        }

        // Need write lock to insert
        let mut pool = self.pool.write().expect("StringPool write lock poisoned");
        
        // Double-check in case another thread interned while we waited
        if let Some(interned) = pool.get(s) {
            return Arc::clone(interned);
        }

        // Create new interned string
        let interned: Arc<str> = Arc::from(s);
        pool.insert(s.to_string(), Arc::clone(&interned));
        interned
    }

    /// Get current pool size for monitoring
    pub fn size(&self) -> usize {
        self.pool.read().expect("StringPool read lock poisoned").len()
    }

    /// Clear the pool (mainly for testing)
    #[cfg(test)]
    pub fn clear(&self) {
        self.pool.write().expect("StringPool write lock poisoned").clear();
    }
}

/// Convenience function to intern a string using the global pool
pub fn intern(s: &str) -> Arc<str> {
    STRING_POOL.intern(s)
}

/// Convenience function to intern an Option<String>
pub fn intern_option(s: Option<&str>) -> Option<Arc<str>> {
    s.map(intern)
}

/// Convenience function to intern a Vec<String>
pub fn intern_vec(v: Vec<String>) -> Vec<Arc<str>> {
    v.into_iter().map(|s| intern(&s)).collect()
}

/// Get the current size of the global string pool
pub fn pool_size() -> usize {
    STRING_POOL.size()
}

/// Types that can be interned
pub trait Internable {
    /// Return an interned version of self
    fn intern(&self) -> Arc<str>;
}

impl Internable for String {
    fn intern(&self) -> Arc<str> {
        intern(self)
    }
}

impl Internable for &str {
    fn intern(&self) -> Arc<str> {
        intern(self)
    }
}

impl Internable for Arc<str> {
    fn intern(&self) -> Arc<str> {
        // Already an Arc<str>, but ensure it's in the pool
        intern(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let pool = StringPool::new();
        
        let s1 = pool.intern("hello");
        let s2 = pool.intern("hello");
        let s3 = pool.intern("world");
        
        // Same string should return same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        
        // Different strings should not
        assert!(!Arc::ptr_eq(&s1, &s3));
        
        assert_eq!(pool.size(), 2);
    }

    #[test]
    fn test_global_pool() {
        // Clear to ensure clean state
        STRING_POOL.clear();
        
        let s1 = intern("test");
        let s2 = intern("test");
        let s3 = intern("different");
        
        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
        
        assert!(pool_size() >= 2);
    }

    #[test]
    fn test_option_interning() {
        let some = intern_option(Some("value"));
        let none = intern_option(None);
        
        assert_eq!(some.as_deref(), Some("value"));
        assert_eq!(none, None);
    }

    #[test]
    fn test_vec_interning() {
        let vec = vec!["one".to_string(), "two".to_string(), "one".to_string()];
        let interned = intern_vec(vec);
        
        assert_eq!(interned.len(), 3);
        assert!(Arc::ptr_eq(&interned[0], &interned[2]));
    }
}