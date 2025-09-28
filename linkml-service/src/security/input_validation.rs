//! Input validation utilities for security
//!
//! This module provides comprehensive input validation to prevent
//! various security vulnerabilities such as `DoS` attacks, injection
//! attacks, and resource exhaustion.

use thiserror::Error;

/// Security limits for various input types
pub mod limits {
    /// Maximum length for general string inputs
    pub const MAX_STRING_LENGTH: usize = 1_000_000; // 1MB

    /// Maximum depth for nested expressions
    pub const MAX_EXPRESSION_DEPTH: usize = 100;

    /// Maximum number of constraints in a single validation
    pub const MAX_CONSTRAINT_COUNT: usize = 1000;

    /// Maximum number of cache entries
    pub const MAX_CACHE_ENTRIES: usize = 10_000;

    /// Maximum number of function arguments
    pub const MAX_FUNCTION_ARGS: usize = 20;

    /// Maximum length for identifiers (names, keys, etc.)
    pub const MAX_IDENTIFIER_LENGTH: usize = 256;

    /// Maximum size for `JSON` payloads
    pub const MAX_JSON_SIZE: usize = 10_000_000; // 10MB

    /// Maximum number of slots in a class
    pub const MAX_SLOTS_PER_CLASS: usize = 1000;

    /// Maximum number of classes in a schema
    pub const MAX_CLASSES_PER_SCHEMA: usize = 10_000;
}

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    /// String exceeds maximum allowed size
    #[error("String too large: {size} bytes (max: {max})")]
    StringTooLarge {
        /// Actual size in bytes
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Identifier exceeds maximum allowed length
    #[error("Identifier too long: {size} characters (max: {max})")]
    IdentifierTooLong {
        /// Actual size in characters
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Expression nesting depth exceeds limit
    #[error("Expression too deep: {depth} levels (max: {max})")]
    ExpressionTooDeep {
        /// Actual depth
        depth: usize,
        /// Maximum allowed depth
        max: usize,
    },

    /// Too many constraints in validation
    #[error("Too many constraints: {count} (max: {max})")]
    TooManyConstraints {
        /// Actual count
        count: usize,
        /// Maximum allowed count
        max: usize,
    },

    /// Too many function arguments
    #[error("Too many function arguments: {count} (max: {max})")]
    TooManyFunctionArgs {
        /// Actual count
        count: usize,
        /// Maximum allowed count
        max: usize,
    },

    /// Invalid UTF-8 in string
    #[error("Invalid UTF-8 in string")]
    InvalidUtf8,

    /// String contains control characters
    #[error("String contains control characters")]
    ControlCharacters,

    /// String contains null bytes
    #[error("String contains null bytes")]
    NullBytes,

    /// `JSON` payload too large
    #[error("JSON payload too large: {size} bytes (max: {max})")]
    JsonTooLarge {
        /// Actual size in bytes
        size: usize,
        /// Maximum allowed size
        max: usize,
    },
}

/// Validate a general string input
/// Returns an error if the operation fails
///
/// # Errors
///
/// Returns `ValidationError::StringTooLarge` if the string exceeds max length
/// Returns `ValidationError::NullBytes` if the string contains null bytes
/// Returns `ValidationError::ControlCharacters` if the string contains control characters
pub fn validate_string_input(s: &str) -> Result<(), ValidationError> {
    // Check length
    if s.len() > limits::MAX_STRING_LENGTH {
        return Err(ValidationError::StringTooLarge {
            size: s.len(),
            max: limits::MAX_STRING_LENGTH,
        });
    }

    // Check for null bytes
    if s.contains('\0') {
        return Err(ValidationError::NullBytes);
    }

    // Check for control characters (except common ones like newline, tab)
    if s.chars()
        .any(|c| c.is_control() && c != '\n' && c != '\r' && c != '\t')
    {
        return Err(ValidationError::ControlCharacters);
    }

    Ok(())
}

/// Validate an identifier (name, key, etc.)
/// Returns an error if the operation fails
///
/// # Errors
///
/// Returns `ValidationError::IdentifierTooLong` if the identifier exceeds max length
/// Returns `ValidationError::ControlCharacters` if the identifier contains control characters
/// Returns `ValidationError::NullBytes` if the identifier contains null bytes
pub fn validate_identifier(id: &str) -> Result<(), ValidationError> {
    // Check length
    if id.len() > limits::MAX_IDENTIFIER_LENGTH {
        return Err(ValidationError::IdentifierTooLong {
            size: id.len(),
            max: limits::MAX_IDENTIFIER_LENGTH,
        });
    }

    // Identifiers should not contain control characters
    if id.chars().any(char::is_control) {
        return Err(ValidationError::ControlCharacters);
    }

    // Check for null bytes
    if id.contains('\0') {
        return Err(ValidationError::NullBytes);
    }

    Ok(())
}

/// Validate `JSON` size before parsing
/// Returns an error if the operation fails
///
/// # Errors
///
/// Returns `ValidationError::JsonTooLarge` if the JSON string exceeds max size
pub fn validate_json_size(json_str: &str) -> Result<(), ValidationError> {
    if json_str.len() > limits::MAX_JSON_SIZE {
        return Err(ValidationError::JsonTooLarge {
            size: json_str.len(),
            max: limits::MAX_JSON_SIZE,
        });
    }
    Ok(())
}

/// Sanitize a string for safe display (removes control characters)
#[must_use]
pub fn sanitize_for_display(s: &str) -> String {
    s.chars()
        .filter(|&c| !c.is_control() || c == '\n' || c == '\r' || c == '\t')
        .collect()
}

/// Truncate a string to a safe length for logging
#[must_use]
pub fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... (truncated from {} bytes)", &s[..max_len], s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_string_input() {
        // Valid strings
        assert!(validate_string_input("hello world").is_ok());
        assert!(
            validate_string_input(
                "multi
line
string"
            )
            .is_ok()
        );
        assert!(validate_string_input("with\ttabs").is_ok());

        // Too large
        let large_string = "x".repeat(limits::MAX_STRING_LENGTH + 1);
        assert!(matches!(
            validate_string_input(&large_string),
            Err(ValidationError::StringTooLarge { .. })
        ));

        // Null bytes
        assert!(matches!(
            validate_string_input("hello\0world"),
            Err(ValidationError::NullBytes)
        ));

        // Control characters
        assert!(matches!(
            validate_string_input("hello\x01world"),
            Err(ValidationError::ControlCharacters)
        ));
    }

    #[test]
    fn test_validate_identifier() {
        // Valid identifiers
        assert!(validate_identifier("my_class").is_ok());
        assert!(validate_identifier("MyClass123").is_ok());
        assert!(validate_identifier("some-identifier").is_ok());

        // Too long
        let long_id = "x".repeat(limits::MAX_IDENTIFIER_LENGTH + 1);
        assert!(matches!(
            validate_identifier(&long_id),
            Err(ValidationError::IdentifierTooLong { .. })
        ));

        // Control characters
        assert!(matches!(
            validate_identifier(
                "my
class"
            ),
            Err(ValidationError::ControlCharacters)
        ));
    }

    #[test]
    fn test_sanitize_for_display() {
        assert_eq!(sanitize_for_display("hello\x01world"), "helloworld");
        assert_eq!(
            sanitize_for_display(
                "multi
line"
            ),
            "multi
line"
        );
        assert_eq!(sanitize_for_display("with\ttabs"), "with\ttabs");
    }

    #[test]
    fn test_truncate_for_log() {
        assert_eq!(truncate_for_log("short", 10), "short");
        assert_eq!(
            truncate_for_log("this is a long string", 10),
            "this is a ... (truncated from 21 bytes)"
        );
    }
}
