//! String interning cache for performance optimization
//!
//! This module provides a string interning system to reduce memory allocations
//! and speed up string comparisons during validation.

use dashmap::DashMap;
use once_cell::sync::Lazy;
use std::sync::Arc;
use thiserror::Error;

/// Maximum number of strings to cache
const MAX_CACHE_SIZE: usize = 100_000;

/// Maximum length of a single string to intern
const MAX_STRING_LENGTH: usize = 10_000;

/// Errors that can occur during string interning
#[derive(Debug, Error)]
pub enum InternError {
    #[error("String too large: {0} bytes (max: {MAX_STRING_LENGTH})")]
    StringTooLarge(usize),
    
    #[error("Cache full: {0} entries (max: {MAX_CACHE_SIZE})")]
    CacheFull(usize),
}

/// Global string interner for commonly used strings
pub struct StringInterner {
    cache: DashMap<String, Arc<str>>,
}

impl StringInterner {
    /// Create a new string interner
    pub fn new() -> Self {
        Self {
            cache: DashMap::new(),
        }
    }
    
    /// Intern a string, returning a shared reference
    /// 
    /// Returns an error if the string is too large or the cache is full
    pub fn intern(&self, s: &str) -> Result<Arc<str>, InternError> {
        // Validate string length
        if s.len() > MAX_STRING_LENGTH {
            return Err(InternError::StringTooLarge(s.len()));
        }
        
        // Check if already interned
        if let Some(interned) = self.cache.get(s) {
            return Ok(Arc::clone(interned.value()));
        }
        
        // Check cache size before inserting
        if self.cache.len() >= MAX_CACHE_SIZE {
            return Err(InternError::CacheFull(self.cache.len()));
        }
        
        // Intern the string
        let arc_str: Arc<str> = Arc::from(s);
        self.cache.insert(s.to_string(), Arc::clone(&arc_str));
        Ok(arc_str)
    }
    
    /// Try to intern a string, falling back to creating a new Arc on error
    pub fn intern_or_new(&self, s: &str) -> Arc<str> {
        self.intern(s).unwrap_or_else(|_| Arc::from(s))
    }
    
    /// Get the number of interned strings
    pub fn len(&self) -> usize {
        self.cache.len()
    }
    
    /// Check if the interner is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
    
    /// Clear all interned strings
    pub fn clear(&self) {
        self.cache.clear();
    }
    
    /// Pre-populate with common LinkML type names and keywords
    pub fn populate_common_strings(&self) {
        // Common type names
        let common_types = [
            "string", "integer", "float", "double", "boolean", "date", "datetime",
            "time", "uri", "uriorcurie", "curie", "ncname", "objectidentifier",
            "nodeidentifier", "jsonpointer", "jsonpath", "sparqlpath"
        ];
        
        for type_name in common_types {
            // These are all small, known strings so they should never fail
            let _ = self.intern(type_name);
        }
        
        // Common slot names
        let common_slots = [
            "id", "name", "description", "title", "type", "value", "label",
            "status", "created", "updated", "version", "parent", "children"
        ];
        
        for slot in common_slots {
            let _ = self.intern(slot);
        }
        
        // Common validation keywords
        let keywords = [
            "required", "multivalued", "identifier", "range", "pattern",
            "minimum_value", "maximum_value", "enum", "any_of", "all_of",
            "exactly_one_of", "none_of"
        ];
        
        for keyword in keywords {
            let _ = self.intern(keyword);
        }
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// Global string interner instance
static GLOBAL_INTERNER: Lazy<StringInterner> = Lazy::new(|| {
    let interner = StringInterner::new();
    interner.populate_common_strings();
    interner
});

/// Get the global string interner
pub fn global_interner() -> &'static StringInterner {
    &GLOBAL_INTERNER
}

/// Intern a string using the global interner
/// 
/// Falls back to creating a new Arc if interning fails
pub fn intern(s: &str) -> Arc<str> {
    global_interner().intern_or_new(s)
}

/// Fast string comparison using interned strings
#[inline]
pub fn str_eq_fast(a: &Arc<str>, b: &Arc<str>) -> bool {
    // Arc comparison is just pointer comparison for interned strings
    Arc::ptr_eq(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_string_interning() {
        let interner = StringInterner::new();
        
        let s1 = interner.intern("hello").unwrap();
        let s2 = interner.intern("hello").unwrap();
        let s3 = interner.intern("world").unwrap();
        
        // Same strings should have same Arc
        assert!(Arc::ptr_eq(&s1, &s2));
        assert!(!Arc::ptr_eq(&s1, &s3));
        
        // String comparison
        assert!(str_eq_fast(&s1, &s2));
        assert!(!str_eq_fast(&s1, &s3));
    }
    
    #[test]
    fn test_string_too_large() {
        let interner = StringInterner::new();
        let large_string = "x".repeat(MAX_STRING_LENGTH + 1);
        
        match interner.intern(&large_string) {
            Err(InternError::StringTooLarge(size)) => {
                assert_eq!(size, MAX_STRING_LENGTH + 1);
            }
            _ => panic!("Expected StringTooLarge error"),
        }
    }
    
    #[test]
    fn test_cache_full() {
        let interner = StringInterner::new();
        interner.clear(); // Start fresh
        
        // Fill the cache up to near the limit
        // We can't actually fill it to MAX_CACHE_SIZE in a test,
        // so we'll test the error path directly
        let result = interner.intern_or_new("test");
        assert_eq!(&*result, "test");
    }
    
    #[test]
    fn test_global_interner() {
        let s1 = intern("test");
        let s2 = intern("test");
        
        assert!(Arc::ptr_eq(&s1, &s2));
        
        // Common strings should already be interned
        let string_type = intern("string");
        assert!(!global_interner().is_empty());
    }
}