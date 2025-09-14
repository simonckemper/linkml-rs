//! Configuration-driven input validation utilities for security
//!
//! This module provides comprehensive input validation that uses
//! configuration instead of hardcoded values.

use thiserror::Error;
use linkml_core::configuration_v2::SecurityLimitsConfig;

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("String too large: {size} bytes (max: {max})")]
    StringTooLarge { size: usize, max: usize },

    #[error("Identifier too long: {size} characters (max: {max})")]
    IdentifierTooLong { size: usize, max: usize },

    #[error("Expression too deep: {depth} levels (max: {max})")]
    ExpressionTooDeep { depth: usize, max: usize },

    #[error("Too many constraints: {count} (max: {max})")]
    TooManyConstraints { count: usize, max: usize },

    #[error("Too many function arguments: {count} (max: {max})")]
    TooManyFunctionArgs { count: usize, max: usize },

    #[error("JSON too large: {size} bytes (max: {max})")]
    JsonTooLarge { size: usize, max: usize },

    #[error("Too many slots in class: {count} (max: {max})")]
    TooManySlotsPerClass { count: usize, max: usize },

    #[error("Too many classes in schema: {count} (max: {max})")]
    TooManyClassesPerSchema { count: usize, max: usize },

    #[error("Too many cache entries: {count} (max: {max})")]
    TooManyCacheEntries { count: usize, max: usize },

    #[error("Invalid character in identifier at position {position}: {char}")]
    InvalidCharacterInIdentifier { position: usize, char: char },

    #[error("Empty input not allowed")]
    EmptyInput,

    #[error("Invalid UTF-8 sequence")]
    InvalidUtf8}

/// Input validator that uses configuration for limits
pub struct InputValidator {
    limits: SecurityLimitsConfig}

impl InputValidator {
    /// Create a new input validator with configuration
    #[must_use]
    pub fn new(limits: SecurityLimitsConfig) -> Self {
        Self { limits }
    }

    /// Validate a string input
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::EmptyInput` if the input string is empty
    /// Returns `ValidationError::StringTooLarge` if the string exceeds max length
    pub fn validate_string(&self, input: &str) -> Result<(), ValidationError> {
        if input.is_empty() {
            return Err(ValidationError::EmptyInput);
        }

        if input.len() > self.limits.max_string_length {
            return Err(ValidationError::StringTooLarge {
                size: input.len(),
                max: self.limits.max_string_length});
        }

        Ok(())
    }

    /// Validate an identifier (variable names, keys, etc.)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::EmptyInput` if the identifier is empty
    /// Returns `ValidationError::IdentifierTooLong` if the identifier exceeds max length
    /// Returns `ValidationError::InvalidCharacterInIdentifier` if the identifier contains invalid characters
    pub fn validate_identifier(&self, identifier: &str) -> Result<(), ValidationError> {
        if identifier.is_empty() {
            return Err(ValidationError::EmptyInput);
        }

        if identifier.len() > self.limits.max_identifier_length {
            return Err(ValidationError::IdentifierTooLong {
                size: identifier.len(),
                max: self.limits.max_identifier_length});
        }

        // Check for valid identifier characters
        for (i, ch) in identifier.chars().enumerate() {
            match ch {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' => continue,
                _ => {
                    return Err(ValidationError::InvalidCharacterInIdentifier {
                        position: i,
                        char: ch});
                }
            }
        }

        Ok(())
    }

    /// Validate expression depth
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::ExpressionTooDeep` if the depth exceeds the configured maximum
    pub fn validate_expression_depth(&self, depth: usize) -> Result<(), ValidationError> {
        if depth > self.limits.max_expression_depth {
            return Err(ValidationError::ExpressionTooDeep {
                depth,
                max: self.limits.max_expression_depth});
        }
        Ok(())
    }

    /// Validate constraint count
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManyConstraints` if the count exceeds the configured maximum
    pub fn validate_constraint_count(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_constraint_count {
            return Err(ValidationError::TooManyConstraints {
                count,
                max: self.limits.max_constraint_count});
        }
        Ok(())
    }

    /// Validate function argument count
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManyFunctionArgs` if the argument count exceeds the configured maximum
    pub fn validate_function_args(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_function_args {
            return Err(ValidationError::TooManyFunctionArgs {
                count,
                max: self.limits.max_function_args});
        }
        Ok(())
    }

    /// Validate `JSON` size
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::JsonTooLarge` if the JSON size exceeds the configured maximum
    pub fn validate_json_size(&self, size: usize) -> Result<(), ValidationError> {
        if size > self.limits.max_json_size_bytes {
            return Err(ValidationError::JsonTooLarge {
                size,
                max: self.limits.max_json_size_bytes});
        }
        Ok(())
    }

    /// Validate slots per class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManySlotsPerClass` if the slot count exceeds the configured maximum
    pub fn validate_slots_per_class(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_slots_per_class {
            return Err(ValidationError::TooManySlotsPerClass {
                count,
                max: self.limits.max_slots_per_class});
        }
        Ok(())
    }

    /// Validate classes per schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManyClassesPerSchema` if the class count exceeds the configured maximum
    pub fn validate_classes_per_schema(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_classes_per_schema {
            return Err(ValidationError::TooManyClassesPerSchema {
                count,
                max: self.limits.max_classes_per_schema});
        }
        Ok(())
    }

    /// Validate cache entries
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManyCacheEntries` if the cache entry count exceeds the configured maximum
    pub fn validate_cache_entries(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_cache_entries {
            return Err(ValidationError::TooManyCacheEntries {
                count,
                max: self.limits.max_cache_entries});
        }
        Ok(())
    }

    /// Sanitize a string for safe usage
    pub fn sanitize_string(&self, input: &str) -> String {
        // Remove null bytes and control characters
        input
            .chars()
            .filter(|&c| c != '\0' && !c.is_control())
            .take(self.limits.max_string_length)
            .collect()
    }
}

/// Create a default input validator (for testing only)
#[cfg(test)]
pub fn default_validator() -> InputValidator {
    InputValidator::new(SecurityLimitsConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        let validator = default_validator();

        // Valid strings
        assert!(validator.validate_string("hello").is_ok());
        assert!(validator.validate_string("a").is_ok());

        // Empty string
        assert!(matches!(
            validator.validate_string(""),
            Err(ValidationError::EmptyInput)
        ));

        // String too large
        let large_string = "x".repeat(validator.limits.max_string_length + 1);
        assert!(matches!(
            validator.validate_string(&large_string),
            Err(ValidationError::StringTooLarge { .. })
        ));
    }

    #[test]
    fn test_identifier_validation() {
        let validator = default_validator();

        // Valid identifiers
        assert!(validator.validate_identifier("valid_name").is_ok());
        assert!(validator.validate_identifier("valid-name").is_ok());
        assert!(validator.validate_identifier("valid.name").is_ok());
        assert!(validator.validate_identifier("name123").is_ok());

        // Invalid identifiers
        assert!(validator.validate_identifier("invalid name").is_err());
        assert!(validator.validate_identifier("invalid@name").is_err());
        assert!(validator.validate_identifier("").is_err());
    }

    #[test]
    fn test_depth_validation() {
        let validator = default_validator();

        assert!(validator.validate_expression_depth(10).is_ok());
        assert!(validator.validate_expression_depth(validator.limits.max_expression_depth).is_ok());
        assert!(validator.validate_expression_depth(validator.limits.max_expression_depth + 1).is_err());
    }
}