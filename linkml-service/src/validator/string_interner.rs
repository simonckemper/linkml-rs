//! String interning for memory optimization
//!
//! This module provides string interning to reduce memory usage for
//! frequently occurring strings like field names, type names, and error codes.

use crate::utils::safe_cast::usize_to_f64;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::Arc;

/// A handle to an interned string
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InternedString(u32);

impl InternedString {
    /// Get the underlying index
    #[must_use]
    pub fn index(&self) -> u32 {
        self.0
    }
}

/// Thread-safe string interner
pub struct StringInterner {
    /// Map from string to index
    string_to_index: Arc<DashMap<String, u32>>,
    /// Map from index to string
    index_to_string: Arc<RwLock<Vec<String>>>,
    /// Pre-interned common strings
    common_strings: CommonStrings,
}

/// Pre-interned common strings for fast access
pub struct CommonStrings {
    /// Common field names
    /// Interned "name"
    pub field_name: InternedString,
    /// Interned "type"
    pub field_type: InternedString,
    /// Interned "id"
    pub field_id: InternedString,
    /// Interned "value"
    pub field_value: InternedString,
    /// Interned "description"
    pub field_description: InternedString,

    /// Common type names
    /// Interned "string"
    pub type_string: InternedString,
    /// Interned "integer"
    pub type_integer: InternedString,
    /// Interned "float"
    pub type_float: InternedString,
    /// Interned "boolean"
    pub type_boolean: InternedString,
    /// Interned "date"
    pub type_date: InternedString,
    /// Interned "datetime"
    pub type_datetime: InternedString,
    /// Interned "uri"
    pub type_uri: InternedString,
    /// Interned "object"
    pub type_object: InternedString,
    /// Interned "array"
    pub type_array: InternedString,

    /// Common error codes
    /// Interned "`required_field_missing`"
    pub error_required: InternedString,
    /// Interned "`type_mismatch`"
    pub error_type_mismatch: InternedString,
    /// Interned "`pattern_mismatch`"
    pub error_pattern_mismatch: InternedString,
    /// Interned "`range_violation`"
    pub error_range_violation: InternedString,
    /// Interned "`enum_violation`"
    pub error_enum_violation: InternedString,
    /// Interned "`length_violation`"
    pub error_length_violation: InternedString,
}

impl StringInterner {
    /// Create a new string interner
    #[must_use]
    pub fn new() -> Self {
        let mut interner = Self {
            string_to_index: Arc::new(DashMap::new()),
            index_to_string: Arc::new(RwLock::new(Vec::new())),
            common_strings: CommonStrings {
                // These will be initialized below
                field_name: InternedString(0),
                field_type: InternedString(0),
                field_id: InternedString(0),
                field_value: InternedString(0),
                field_description: InternedString(0),
                type_string: InternedString(0),
                type_integer: InternedString(0),
                type_float: InternedString(0),
                type_boolean: InternedString(0),
                type_date: InternedString(0),
                type_datetime: InternedString(0),
                type_uri: InternedString(0),
                type_object: InternedString(0),
                type_array: InternedString(0),
                error_required: InternedString(0),
                error_type_mismatch: InternedString(0),
                error_pattern_mismatch: InternedString(0),
                error_range_violation: InternedString(0),
                error_enum_violation: InternedString(0),
                error_length_violation: InternedString(0),
            },
        };

        // Pre-intern common strings
        interner.common_strings = CommonStrings {
            // Field names
            field_name: interner.intern("name"),
            field_type: interner.intern("type"),
            field_id: interner.intern("id"),
            field_value: interner.intern("value"),
            field_description: interner.intern("description"),

            // Type names
            type_string: interner.intern("string"),
            type_integer: interner.intern("integer"),
            type_float: interner.intern("float"),
            type_boolean: interner.intern("boolean"),
            type_date: interner.intern("date"),
            type_datetime: interner.intern("datetime"),
            type_uri: interner.intern("uri"),
            type_object: interner.intern("object"),
            type_array: interner.intern("array"),

            // Error codes
            error_required: interner.intern("required_field_missing"),
            error_type_mismatch: interner.intern("type_mismatch"),
            error_pattern_mismatch: interner.intern("pattern_mismatch"),
            error_range_violation: interner.intern("range_violation"),
            error_enum_violation: interner.intern("enum_violation"),
            error_length_violation: interner.intern("length_violation"),
        };

        interner
    }

    /// Intern a string and return its handle
    ///
    /// # Panics
    ///
    /// Panics if the number of interned strings exceeds `u32::MAX`.
    #[must_use]
    pub fn intern(&self, s: &str) -> InternedString {
        // Check if already interned
        if let Some(index) = self.string_to_index.get(s) {
            return InternedString(*index);
        }

        // Add new string
        let mut strings = self.index_to_string.write();
        let index = u32::try_from(strings.len()).expect("Too many interned strings");
        strings.push(s.to_string());
        drop(strings);

        self.string_to_index.insert(s.to_string(), index);
        InternedString(index)
    }

    /// Get the string for an interned handle
    #[must_use]
    pub fn get(&self, interned: InternedString) -> Option<String> {
        let strings = self.index_to_string.read();
        strings.get(interned.0 as usize).cloned()
    }

    /// Get a string reference (requires holding the lock)
    #[must_use]
    pub fn get_ref(&self, interned: InternedString) -> StringRef<'_> {
        StringRef {
            interner: self,
            handle: interned,
        }
    }

    /// Get common strings
    #[must_use]
    pub fn common(&self) -> &CommonStrings {
        &self.common_strings
    }

    /// Get statistics about the interner
    #[must_use]
    pub fn stats(&self) -> InternerStats {
        let strings = self.index_to_string.read();
        let total_strings = strings.len();
        let total_bytes: usize = strings.iter().map(std::string::String::len).sum();

        InternerStats {
            total_strings,
            total_bytes,
            average_length: if total_strings > 0 {
                // Calculate average length using safe casting
                usize_to_f64(total_bytes) / usize_to_f64(total_strings)
            } else {
                0.0
            },
        }
    }

    /// Clear the interner (except common strings)
    ///
    /// # Panics
    ///
    /// Panics if the common string count cannot be converted to `u32`.
    pub fn clear(&self) {
        // Don't clear the first set of common strings
        let common_count = 20; // Number of pre-interned common strings

        let mut strings = self.index_to_string.write();
        strings.truncate(common_count);
        drop(strings);

        // Remove non-common entries from map
        let common_count_u32 = u32::try_from(common_count).expect("Too many common strings");
        self.string_to_index
            .retain(|_, &mut v| v < common_count_u32);
    }
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

/// A reference to an interned string
pub struct StringRef<'a> {
    interner: &'a StringInterner,
    handle: InternedString,
}

impl StringRef<'_> {
    /// Get the string value as an owned String
    #[must_use]
    pub fn to_owned_string(&self) -> String {
        self.interner.get(self.handle).unwrap_or_default()
    }
}

impl std::fmt::Display for StringRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_owned_string())
    }
}

/// Statistics about the string interner
#[derive(Debug, Clone)]
pub struct InternerStats {
    /// Total number of interned strings
    pub total_strings: usize,
    /// Total bytes used by strings
    pub total_bytes: usize,
    /// Average string length
    pub average_length: f64,
}

/// Global string interner for the validation system
static GLOBAL_INTERNER: std::sync::LazyLock<StringInterner> =
    std::sync::LazyLock::new(StringInterner::new);

/// Get the global string interner
#[must_use]
pub fn global_interner() -> &'static StringInterner {
    &GLOBAL_INTERNER
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_interning() {
        let interner = StringInterner::new();

        // Intern same string multiple times
        let s1 = interner.intern("hello");
        let s2 = interner.intern("hello");
        let s3 = interner.intern("world");

        // Same strings get same handle
        assert_eq!(s1, s2);
        assert_ne!(s1, s3);

        // Can retrieve strings
        assert_eq!(interner.get(s1), Some("hello".to_string()));
        assert_eq!(interner.get(s3), Some("world".to_string()));
    }

    #[test]
    fn test_common_strings() {
        let interner = StringInterner::new();
        let common = interner.common();

        // Common strings are pre-interned
        assert_eq!(interner.get(common.field_name), Some("name".to_string()));
        assert_eq!(interner.get(common.type_string), Some("string".to_string()));
        assert_eq!(
            interner.get(common.error_required),
            Some("required_field_missing".to_string())
        );
    }

    #[test]
    fn test_string_ref() {
        let interner = StringInterner::new();
        let handle = interner.intern("test");

        let string_ref = interner.get_ref(handle);
        assert_eq!(string_ref.to_string(), "test");
        assert_eq!(format!("{string_ref}"), "test");
    }

    #[test]
    fn test_clear() {
        let interner = StringInterner::new();

        // Add some strings
        let _ = interner.intern("temporary1");
        let _ = interner.intern("temporary2");

        let stats_before = interner.stats();
        assert!(stats_before.total_strings > 20); // More than just common strings

        // Clear non-common strings
        interner.clear();

        let stats_after = interner.stats();
        assert_eq!(stats_after.total_strings, 20); // Only common strings remain

        // Common strings still work
        let common = interner.common();
        assert_eq!(interner.get(common.field_name), Some("name".to_string()));
    }
}
