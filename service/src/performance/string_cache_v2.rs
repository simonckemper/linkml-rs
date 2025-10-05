//! String interning cache with configuration support
//!
//! This module provides a string interning system to reduce memory allocations
//! and speed up string comparisons during validation.

use dashmap::DashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during string interning
#[derive(Debug, Error)]
pub enum InternError {
    #[error("String too large: {size} bytes (max: {max})")]
    StringTooLarge { size: usize, max: usize },

    #[error("Cache full: {current} entries (max: {max})")]
    CacheFull { current: usize, max: usize }}

/// String interner with configurable limits
pub struct StringInterner {
    cache: DashMap<String, Arc<str>>,
    max_entries: usize,
    max_string_length: usize}

impl StringInterner {
    /// Create a new string interner with configuration
    pub fn new(max_entries: usize, max_string_length: usize) -> Self {
        Self {
            cache: DashMap::new(),
            max_entries,
            max_string_length}
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(100_000, 10_000)
    }

    /// Intern a string, returning a shared reference
    ///
    /// Returns an error if the string is too large or the cache is full
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `InternError::StringTooLarge` if the string exceeds maximum length
    /// Returns `InternError::CacheFull` if the cache capacity is exceeded
    pub fn intern(&self, s: &str) -> Result<Arc<str>, InternError> {
        // Validate string length
        if s.len() > self.max_string_length {
            return Err(InternError::StringTooLarge {
                size: s.len(),
                max: self.max_string_length});
        }

        // Check if already interned
        if let Some(interned) = self.cache.get(s) {
            return Ok(Arc::clone(&interned));
        }

        // Check cache size
        if self.cache.len() >= self.max_entries {
            return Err(InternError::CacheFull {
                current: self.cache.len(),
                max: self.max_entries});
        }

        // Intern the string
        let arc_str: Arc<str> = Arc::from(s);
        self.cache.insert(s.to_string(), Arc::clone(&arc_str));

        Ok(arc_str)
    }

    /// Try to intern a string, returning None if it can't be interned
    pub fn try_intern(&self, s: &str) -> Option<Arc<str>> {
        self.intern(s).ok()
    }

    /// Get the current number of interned strings
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Clear all interned strings
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> InternStats {
        InternStats {
            entries: self.cache.len(),
            max_entries: self.max_entries,
            max_string_length: self.max_string_length}
    }
}

/// Statistics about the string interner
#[derive(Debug, Clone)]
pub struct InternStats {
    /// Current number of entries
    pub entries: usize,
    /// Maximum allowed entries
    pub max_entries: usize,
    /// Maximum allowed string length
    pub max_string_length: usize}

impl Default for StringInterner {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Builder for StringInterner with configuration
pub struct StringInternerBuilder {
    max_entries: usize,
    max_string_length: usize}

impl StringInternerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            max_entries: 100_000,
            max_string_length: 10_000}
    }

    /// Set maximum entries
    pub fn max_entries(mut self, max: usize) -> Self {
        self.max_entries = max;
        self
    }

    /// Set maximum string length
    pub fn max_string_length(mut self, max: usize) -> Self {
        self.max_string_length = max;
        self
    }

    /// Build the interner
    pub fn build(self) -> StringInterner {
        StringInterner::new(self.max_entries, self.max_string_length)
    }
}

impl Default for StringInternerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_interning() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let interner = StringInterner::with_defaults();

        let s1 = interner.intern("hello").expect("should intern string: {}");
        let s2 = interner.intern("hello").expect("should intern same string: {}");

        // Same string should return same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        assert_eq!(interner.len(), 1);
        Ok(())
    }

    #[test]
    fn test_string_too_large() {
        let interner = StringInterner::new(100, 10);
        let large_string = "x".repeat(11);

        match interner.intern(&large_string) {
            Err(InternError::StringTooLarge { size, max }) => {
                assert_eq!(size, 11);
                assert_eq!(max, 10);
            }
            _ => panic!("Expected StringTooLarge error")}
    }

    #[test]
    fn test_cache_full() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let interner = StringInterner::new(2, 100);

        if let Err(e) = interner.intern("a") {
            panic!("Should intern first string: {e}");
        }
        if let Err(e) = interner.intern("b") {
            panic!("Should intern second string: {e}");
        }

        match interner.intern("c") {
            Err(InternError::CacheFull { current, max }) => {
                assert_eq!(current, 2);
                assert_eq!(max, 2);
            }
            _ => panic!("Expected CacheFull error")}
        Ok(())
    }

    #[test]
    fn test_builder() {
        let interner = StringInternerBuilder::new()
            .max_entries(500)
            .max_string_length(50)
            .build();

        let stats = interner.stats();
        assert_eq!(stats.max_entries, 500);
        assert_eq!(stats.max_string_length, 50);
    }
}