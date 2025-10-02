// Copyright (C) 2025 Simon C. Kemper
// Licensed under Creative Commons BY-NC 4.0
//
// Core trait definitions for data introspection and schema inference

use async_trait::async_trait;
use linkml_core::SchemaDefinition;
use std::path::Path;
use thiserror::Error;

use crate::inference::types::DocumentStats;

/// Error types for inference operations
#[derive(Debug, Error)]
pub enum InferenceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Format identification failed: {0}")]
    FormatIdentificationFailed(String),

    #[error("Parse service error: {0}")]
    ParseServiceError(String),

    #[error("Logger service error: {0}")]
    LoggerError(String),

    #[error("Service error: {0}")]
    ServiceError(String),

    #[error("Unsupported format: {puid} ({format_name})")]
    UnsupportedFormat { puid: String, format_name: String },

    #[error("Invalid data structure: {0}")]
    InvalidDataStructure(String),

    #[error("Schema generation failed: {0}")]
    SchemaGenerationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Type inference error: {0}")]
    TypeInferenceError(String),
}

/// Result type for inference operations
pub type InferenceResult<T> = Result<T, InferenceError>;

/// Trait for format-specific data introspection
///
/// This trait defines the interface for analyzing structured data formats
/// and extracting statistics needed for LinkML schema generation.
///
/// # Design Principles
/// - Single Responsibility: Each introspector handles one format (XML, JSON, CSV)
/// - Open-Closed: New introspectors can be added without modifying existing code
/// - Liskov Substitution: All introspectors are interchangeable
/// - Interface Segregation: Clean trait with minimal required methods
/// - Dependency Inversion: Depend on abstractions, not concrete types
#[async_trait]
pub trait DataIntrospector: Send + Sync {
    /// Analyze a file and collect structure statistics
    ///
    /// # Arguments
    /// * `path` - Path to the file to analyze
    ///
    /// # Returns
    /// * `InferenceResult<DocumentStats>` - Collected statistics or error
    ///
    /// # Errors
    /// Returns error if:
    /// - File cannot be read
    /// - File format is invalid
    /// - Analysis fails for structural reasons
    async fn analyze_file(&self, path: &Path) -> InferenceResult<DocumentStats>;

    /// Analyze raw data bytes
    ///
    /// # Arguments
    /// * `data` - Raw data bytes to analyze
    ///
    /// # Returns
    /// * `InferenceResult<DocumentStats>` - Collected statistics or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Data format is invalid
    /// - Analysis fails for structural reasons
    async fn analyze_bytes(&self, data: &[u8]) -> InferenceResult<DocumentStats>;

    /// Get the format this introspector handles
    ///
    /// # Returns
    /// * `&str` - Format name (e.g., "xml", "json", "csv")
    fn format_name(&self) -> &str;

    /// Generate LinkML schema from collected statistics
    ///
    /// # Arguments
    /// * `stats` - Document statistics collected from analysis
    /// * `schema_id` - Unique identifier for the schema
    ///
    /// # Returns
    /// * `InferenceResult<SchemaDefinition>` - Generated LinkML schema or error
    ///
    /// # Errors
    /// Returns error if:
    /// - Statistics are insufficient for schema generation
    /// - Schema construction fails
    /// - Required metadata is missing
    async fn generate_schema(
        &self,
        stats: &DocumentStats,
        schema_id: &str,
    ) -> InferenceResult<SchemaDefinition>;
}

/// Trait for type inference from sample values
///
/// This trait defines the interface for detecting data types from string samples.
/// Implementations should try to infer the most specific type possible while
/// maintaining accuracy.
pub trait TypeInferencer: Send + Sync {
    /// Infer data type from sample values
    ///
    /// # Arguments
    /// * `samples` - Collection of string samples to analyze
    ///
    /// # Returns
    /// * `InferredType` - Most specific type that matches all samples
    ///
    /// # Type Priority
    /// 1. Boolean (true/false)
    /// 2. Integer (can be parsed as i64)
    /// 3. Float (can be parsed as f64)
    /// 4. DateTime (ISO 8601 format)
    /// 5. Date (ISO 8601 date only)
    /// 6. Time (ISO 8601 time only)
    /// 7. Uri (starts with http://, https://, ftp://)
    /// 8. Email (contains @ with valid format)
    /// 9. String (default fallback)
    fn infer_from_samples(&self, samples: &[String]) -> InferredType;

    /// Infer type with confidence score
    ///
    /// # Arguments
    /// * `samples` - Collection of string samples to analyze
    ///
    /// # Returns
    /// * `(InferredType, f32)` - Inferred type and confidence score (0.0-1.0)
    fn infer_with_confidence(&self, samples: &[String]) -> (InferredType, f32);
}

/// Enum representing all possible inferred data types
///
/// This enum covers all LinkML built-in types that can be inferred
/// from string samples.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InferredType {
    /// String type (default fallback)
    String,
    /// Integer type (i64)
    Integer,
    /// Float type (f64)
    Float,
    /// Boolean type (true/false)
    Boolean,
    /// DateTime type (ISO 8601 with time)
    DateTime,
    /// Date type (ISO 8601 date only)
    Date,
    /// Time type (ISO 8601 time only)
    Time,
    /// URI type (http://, https://, ftp://)
    Uri,
    /// Email type (contains @ with valid format)
    Email,
    /// Unknown type (empty samples or indeterminate)
    Unknown,
}

impl InferredType {
    /// Convert to LinkML type string
    ///
    /// # Returns
    /// * `&str` - LinkML type name
    pub fn to_linkml_type(&self) -> &str {
        match self {
            InferredType::String => "string",
            InferredType::Integer => "integer",
            InferredType::Float => "float",
            InferredType::Boolean => "boolean",
            InferredType::DateTime => "datetime",
            InferredType::Date => "date",
            InferredType::Time => "time",
            InferredType::Uri => "uri",
            InferredType::Email => "string", // LinkML doesn't have email type, use string
            InferredType::Unknown => "string", // Default to string for unknown types
        }
    }

    /// Check if this type requires validation
    ///
    /// # Returns
    /// * `bool` - True if validation patterns should be added
    pub fn requires_validation(&self) -> bool {
        matches!(
            self,
            InferredType::Email
                | InferredType::Uri
                | InferredType::DateTime
                | InferredType::Date
                | InferredType::Time
        )
    }
}

impl std::fmt::Display for InferredType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_linkml_type())
    }
}

impl From<&InferredType> for String {
    fn from(t: &InferredType) -> Self {
        t.to_linkml_type().to_string()
    }
}

impl From<InferredType> for String {
    fn from(t: InferredType) -> Self {
        t.to_linkml_type().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inferred_type_to_linkml_type() {
        assert_eq!(InferredType::String.to_linkml_type(), "string");
        assert_eq!(InferredType::Integer.to_linkml_type(), "integer");
        assert_eq!(InferredType::Float.to_linkml_type(), "float");
        assert_eq!(InferredType::Boolean.to_linkml_type(), "boolean");
        assert_eq!(InferredType::DateTime.to_linkml_type(), "datetime");
        assert_eq!(InferredType::Date.to_linkml_type(), "date");
        assert_eq!(InferredType::Time.to_linkml_type(), "time");
        assert_eq!(InferredType::Uri.to_linkml_type(), "uri");
        assert_eq!(InferredType::Email.to_linkml_type(), "string");
        assert_eq!(InferredType::Unknown.to_linkml_type(), "string");
    }

    #[test]
    fn test_inferred_type_display() {
        assert_eq!(format!("{}", InferredType::Integer), "integer");
        assert_eq!(format!("{}", InferredType::Float), "float");
        assert_eq!(format!("{}", InferredType::Boolean), "boolean");
    }

    #[test]
    fn test_requires_validation() {
        assert!(InferredType::Email.requires_validation());
        assert!(InferredType::Uri.requires_validation());
        assert!(InferredType::DateTime.requires_validation());
        assert!(InferredType::Date.requires_validation());
        assert!(InferredType::Time.requires_validation());

        assert!(!InferredType::String.requires_validation());
        assert!(!InferredType::Integer.requires_validation());
        assert!(!InferredType::Float.requires_validation());
        assert!(!InferredType::Boolean.requires_validation());
    }

    #[test]
    fn test_inferred_type_equality() {
        assert_eq!(InferredType::String, InferredType::String);
        assert_ne!(InferredType::String, InferredType::Integer);

        let mut map = HashMap::new();
        map.insert(InferredType::Integer, "count");
        assert_eq!(map.get(&InferredType::Integer), Some(&"count"));
    }
}
