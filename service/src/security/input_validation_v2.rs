//! Input validation utilities with configuration support
//!
//! This module provides comprehensive input validation to prevent
//! various security vulnerabilities such as DoS attacks, injection
//! attacks, and resource exhaustion.

use thiserror::Error;

/// Security limits configuration
#[derive(Debug, Clone)]
pub struct SecurityLimits {
    /// Maximum length for general string inputs
    pub max_string_length: usize,

    /// Maximum depth for nested expressions
    pub max_expression_depth: usize,

    /// Maximum number of constraints in a single validation
    pub max_constraint_count: usize,

    /// Maximum number of cache entries
    pub max_cache_entries: usize,

    /// Maximum number of function arguments
    pub max_function_args: usize,

    /// Maximum length for identifiers (names, keys, etc.)
    pub max_identifier_length: usize,

    /// Maximum size for `JSON` payloads
    pub max_json_size: usize,

    /// Maximum number of slots in a class
    pub max_slots_per_class: usize,

    /// Maximum number of classes in a schema
    pub max_classes_per_schema: usize}

impl Default for SecurityLimits {
    fn default() -> Self {
        Self {
            max_string_length: 1_000_000,      // 1MB
            max_expression_depth: 100,
            max_constraint_count: 1000,
            max_cache_entries: 10_000,
            max_function_args: 20,
            max_identifier_length: 256,
            max_json_size: 10_000_000,         // 10MB
            max_slots_per_class: 1000,
            max_classes_per_schema: 10_000}
    }
}

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

    #[error("JSON payload too large: {size} bytes (max: {max})")]
    JsonTooLarge { size: usize, max: usize },

    #[error("Too many slots in class: {count} (max: {max})")]
    TooManySlots { count: usize, max: usize },

    #[error("Too many classes in schema: {count} (max: {max})")]
    TooManyClasses { count: usize, max: usize },

    #[error("Invalid pattern: contains potential ReDoS vulnerability")]
    DangerousPattern,

    #[error("Path traversal attempt detected in: {path}")]
    PathTraversal { path: String }}

/// Input validator with configurable security limits
pub struct InputValidator {
    limits: SecurityLimits}

impl InputValidator {
    /// Create a new validator with custom limits
    #[must_use]
    pub fn new(limits: SecurityLimits) -> Self {
        Self { limits }
    }

    /// Create a validator with default limits
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(SecurityLimits::default())
    }

    /// Validate a general string input
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::StringTooLarge` if the string exceeds max length
    pub fn validate_string(&self, s: &str) -> Result<(), ValidationError> {
        if s.len() > self.limits.max_string_length {
            return Err(ValidationError::StringTooLarge {
                size: s.len(),
                max: self.limits.max_string_length});
        }
        Ok(())
    }

    /// Validate an identifier (name, key, etc.)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::IdentifierTooLong` if the identifier exceeds max length
    /// Returns `ValidationError::PathTraversal` if the identifier contains invalid characters
    pub fn validate_identifier(&self, id: &str) -> Result<(), ValidationError> {
        if id.len() > self.limits.max_identifier_length {
            return Err(ValidationError::IdentifierTooLong {
                size: id.len(),
                max: self.limits.max_identifier_length});
        }

        // Additional identifier validation (alphanumeric, underscores, etc.)
        if !id.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(ValidationError::PathTraversal {
                path: id.to_string()});
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

    /// Validate `JSON` payload size
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::JsonTooLarge` if the JSON size exceeds the configured maximum
    pub fn validate_json_size(&self, size: usize) -> Result<(), ValidationError> {
        if size > self.limits.max_json_size {
            return Err(ValidationError::JsonTooLarge {
                size,
                max: self.limits.max_json_size});
        }
        Ok(())
    }

    /// Validate number of slots in a class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManySlots` if the slot count exceeds the configured maximum
    pub fn validate_slot_count(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_slots_per_class {
            return Err(ValidationError::TooManySlots {
                count,
                max: self.limits.max_slots_per_class});
        }
        Ok(())
    }

    /// Validate number of classes in a schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::TooManyClasses` if the class count exceeds the configured maximum
    pub fn validate_class_count(&self, count: usize) -> Result<(), ValidationError> {
        if count > self.limits.max_classes_per_schema {
            return Err(ValidationError::TooManyClasses {
                count,
                max: self.limits.max_classes_per_schema});
        }
        Ok(())
    }

    /// Validate a regex pattern for ReDoS vulnerabilities
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::DangerousPattern` if the pattern contains ReDoS vulnerabilities
    /// Returns `ValidationError::StringTooLarge` if the pattern exceeds max length
    pub fn validate_pattern(&self, pattern: &str) -> Result<(), ValidationError> {
        // Check for common ReDoS patterns
        if pattern.contains(".*.*") ||
           pattern.contains("(.*)+") ||
           pattern.contains("(.+)+") ||
           pattern.contains("([^x]*)") {
            return Err(ValidationError::DangerousPattern);
        }

        // Check pattern length
        self.validate_string(pattern)?;

        Ok(())
    }

    /// Validate a file path for traversal attempts
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `ValidationError::PathTraversal` if the path contains directory traversal attempts
    pub fn validate_path(&self, path: &str) -> Result<(), ValidationError> {
        if path.contains("..") || path.contains("~") || path.starts_with('/') {
            return Err(ValidationError::PathTraversal {
                path: path.to_string()});
        }
        Ok(())
    }

    /// Get the current limits
    pub fn limits(&self) -> &SecurityLimits {
        &self.limits
    }
}

impl Default for InputValidator {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_validation() {
        let validator = InputValidator::with_defaults();

        // Valid string
        assert!(validator.validate_string("hello world").is_ok());

        // Too large string
        let large_string = "x".repeat(validator.limits().max_string_length + 1);
        assert!(matches!(
            validator.validate_string(&large_string),
            Err(ValidationError::StringTooLarge { .. })
        ));
    }

    #[test]
    fn test_identifier_validation() {
        let validator = InputValidator::with_defaults();

        // Valid identifiers
        assert!(validator.validate_identifier("valid_name").is_ok());
        assert!(validator.validate_identifier("name123").is_ok());
        assert!(validator.validate_identifier("name-with-dash").is_ok());

        // Invalid identifiers
        assert!(validator.validate_identifier("../path").is_err());
        assert!(validator.validate_identifier("name with space").is_err());
    }

    #[test]
    fn test_pattern_validation() {
        let validator = InputValidator::with_defaults();

        // Safe patterns
        assert!(validator.validate_pattern(r"\d+").is_ok());
        assert!(validator.validate_pattern(r"[a-z]+").is_ok());

        // Dangerous patterns
        assert!(validator.validate_pattern(r".*.*").is_err());
        assert!(validator.validate_pattern(r"(.*)+").is_err());
    }

    #[test]
    fn test_custom_limits() {
        let mut limits = SecurityLimits::default();
        limits.max_string_length = 100;

        let validator = InputValidator::new(limits);

        assert!(validator.validate_string("short").is_ok());
        assert!(validator.validate_string(&"x".repeat(101)).is_err());
    }
}