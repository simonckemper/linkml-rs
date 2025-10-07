//! Error types for `LinkML` operations

use thiserror::Error;
use timestamp_core;

/// Main error type for `LinkML` operations
#[derive(Error, Debug)]
pub enum LinkMLError {
    /// Schema parsing errors
    #[error("Failed to parse schema: {message}")]
    ParseError {
        /// Error message
        message: String,
        /// Location in schema if available
        location: Option<String>,
    },

    /// Schema validation errors
    #[error("Schema validation failed: {message}")]
    SchemaValidationError {
        /// Error message
        message: String,
        /// Schema element that failed
        element: Option<String>,
    },

    /// Data validation errors
    #[error("Data validation failed: {message}")]
    DataValidationError {
        /// Error message
        message: String,
        /// Path to invalid data
        path: Option<String>,
        /// Expected type or constraint
        expected: Option<String>,
        /// Actual value found
        actual: Option<String>,
    },

    /// Import resolution errors
    #[error("Failed to resolve import '{import}': {reason}")]
    ImportError {
        /// Import that failed
        import: String,
        /// Reason for failure
        reason: String,
    },

    /// Pattern matching errors
    #[error("Pattern validation failed: {message}")]
    PatternError {
        /// Error message
        message: String,
        /// Pattern that failed
        pattern: Option<String>,
        /// Value that didn't match
        value: Option<String>,
    },

    /// Type coercion errors
    #[error("Type coercion failed: cannot convert {from} to {to}")]
    CoercionError {
        /// Source type
        from: String,
        /// Target type
        to: String,
        /// Additional context
        context: Option<String>,
    },

    /// Configuration errors
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// IO errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Service integration errors
    #[error("Service error: {0}")]
    ServiceError(String),

    /// Feature not implemented
    #[error("Feature not implemented: {0}")]
    NotImplemented(String),

    /// Generic errors with context
    #[error("{message}")]
    Other {
        /// Error message
        message: String,
        /// Optional source error
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

/// Result type alias for `LinkML` operations
pub type Result<T> = std::result::Result<T, LinkMLError>;

impl LinkMLError {
    /// Create a new parse error
    #[must_use]
    pub fn parse(message: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            location: None,
        }
    }

    /// Create a new parse error with location
    #[must_use]
    pub fn parse_at(message: impl Into<String>, location: impl Into<String>) -> Self {
        Self::ParseError {
            message: message.into(),
            location: Some(location.into()),
        }
    }

    /// Create a new schema validation error
    #[must_use]
    pub fn schema_validation(message: impl Into<String>) -> Self {
        Self::SchemaValidationError {
            message: message.into(),
            element: None,
        }
    }

    /// Create a new data validation error
    #[must_use]
    pub fn data_validation(message: impl Into<String>) -> Self {
        Self::DataValidationError {
            message: message.into(),
            path: None,
            expected: None,
            actual: None,
        }
    }

    /// Create a new import error
    #[must_use]
    pub fn import(import: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ImportError {
            import: import.into(),
            reason: reason.into(),
        }
    }

    /// Create a new pattern error
    #[must_use]
    pub fn pattern(message: impl Into<String>) -> Self {
        Self::PatternError {
            message: message.into(),
            pattern: None,
            value: None,
        }
    }

    /// Create a new coercion error
    #[must_use]
    pub fn coercion(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::CoercionError {
            from: from.into(),
            to: to.into(),
            context: None,
        }
    }

    /// Create a new configuration error
    #[must_use]
    pub fn config(message: impl Into<String>) -> Self {
        Self::ConfigError(message.into())
    }

    /// Create a new service error
    #[must_use]
    pub fn service(message: impl Into<String>) -> Self {
        Self::ServiceError(message.into())
    }

    /// Create a not implemented error
    #[must_use]
    pub fn not_implemented(feature: impl Into<String>) -> Self {
        Self::NotImplemented(feature.into())
    }

    /// Create a generic error
    #[must_use]
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            source: None,
        }
    }

    /// Create an IO error from a message
    #[must_use]
    pub fn io_error(message: impl Into<String>) -> Self {
        Self::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            message.into(),
        ))
    }

    /// Create a serialization error
    #[must_use]
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::SerializationError(message.into())
    }

    /// Create a deserialization error (alias for parse error)
    #[must_use]
    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::parse(message)
    }

    /// Create a generic error with source
    #[must_use]
    pub fn other_with_source<E>(message: impl Into<String>, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Other {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }
}

// Implement conversions for common error types
impl From<serde_json::Error> for LinkMLError {
    fn from(err: serde_json::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<serde_yaml::Error> for LinkMLError {
    fn from(err: serde_yaml::Error) -> Self {
        Self::SerializationError(err.to_string())
    }
}

impl From<regex::Error> for LinkMLError {
    fn from(err: regex::Error) -> Self {
        Self::PatternError {
            message: err.to_string(),
            pattern: None,
            value: None,
        }
    }
}

// Add conversions for service-level errors
impl From<anyhow::Error> for LinkMLError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other {
            message: err.to_string(),
            source: Some(Box::new(std::io::Error::other(err))),
        }
    }
}

// Add conversion for timestamp service errors
impl From<timestamp_core::TimestampError> for LinkMLError {
    fn from(err: timestamp_core::TimestampError) -> Self {
        Self::ServiceError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = LinkMLError::parse("Invalid YAML");
        assert!(matches!(err, LinkMLError::ParseError { .. }));

        let err = LinkMLError::parse_at("Invalid syntax", "line 10");
        match err {
            LinkMLError::ParseError { location, .. } => {
                assert_eq!(location.as_deref(), Some("line 10"));
            }
            _ => panic!("Wrong error type"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = LinkMLError::import("common.yaml", "File not found");
        let display = err.to_string();
        assert!(display.contains("common.yaml"));
        assert!(display.contains("File not found"));
    }

    #[test]
    fn test_error_conversions() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let linkml_err: LinkMLError = json_err.into();
        assert!(matches!(linkml_err, LinkMLError::SerializationError(_)));
    }
}
