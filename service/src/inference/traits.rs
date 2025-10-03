// Copyright (C) 2025 Simon C. Kemper
// Licensed under Creative Commons BY-NC 4.0
//
// Core trait definitions for data introspection and schema inference

use async_trait::async_trait;
use linkml_core::SchemaDefinition;
use std::path::Path;
use thiserror::Error;

use crate::inference::types::DocumentStats;

/// Comprehensive error types for LinkML schema inference operations.
///
/// This error type integrates with RootReal's centralized error handling service
/// and provides detailed context for debugging inference failures across the
/// multi-service pipeline (Format Identification → Parse → Introspection → Schema Generation).
///
/// Each variant includes contextual information to aid in root cause analysis
/// and recovery strategy selection.
#[derive(Debug, Error)]
pub enum InferenceError {
    /// File system operations failed during data access or schema writing.
    ///
    /// This typically occurs when:
    /// - Input files are missing or inaccessible
    /// - Output directories lack write permissions
    /// - Disk space is exhausted during large file processing
    ///
    /// Recovery strategy: Verify file paths and permissions before retrying.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Format Identification Service could not determine file format via PRONOM signatures.
    ///
    /// This occurs when:
    /// - File lacks recognizable magic bytes or signature patterns
    /// - File is corrupted or truncated
    /// - Format is not registered in PRONOM database
    ///
    /// Recovery strategy: Manually specify format using explicit introspector selection.
    #[error("Format identification failed: {0}")]
    FormatIdentificationFailed(
        /// Detailed error message from Format Identification Service explaining why detection failed
        String,
    ),

    /// Parse Service encountered errors extracting structured data from file.
    ///
    /// This occurs when:
    /// - XML/JSON is malformed or violates syntax rules
    /// - CSV has inconsistent column counts or encoding issues
    /// - File contains binary data in text format fields
    ///
    /// Recovery strategy: Validate input file format and repair structural issues.
    #[error("Parse service error: {0}")]
    ParseServiceError(
        /// Detailed parse error including line/column numbers for syntax errors
        String,
    ),

    /// Logger Service integration failed during inference operation logging.
    ///
    /// This is typically non-fatal but indicates monitoring gaps in:
    /// - Progress tracking for long-running batch operations
    /// - Performance metrics collection for optimization
    /// - Audit trail generation for compliance
    ///
    /// Recovery strategy: Check logger service configuration and connectivity.
    #[error("Logger service error: {0}")]
    LoggerError(
        /// Logger service error details including service state and configuration issues
        String,
    ),

    /// Generic service integration failure across any RootReal service dependency.
    ///
    /// This occurs when:
    /// - Task Management Service cannot spawn async operations
    /// - Timestamp Service is unavailable for metadata generation
    /// - Service initialization fails during engine creation
    ///
    /// Recovery strategy: Verify all service dependencies are properly initialized.
    #[error("Service error: {0}")]
    ServiceError(
        /// Generic service error message identifying which service failed and why
        String,
    ),

    /// File format identified by PRONOM but no introspector implementation exists.
    ///
    /// This occurs when:
    /// - Format is recognized (valid PUID) but introspector not yet implemented
    /// - Format is proprietary/binary with no open parsing specification
    /// - Format requires specialized libraries not yet integrated
    ///
    /// Currently supported formats: XML, JSON, CSV
    /// Recovery strategy: Implement custom introspector or convert to supported format.
    #[error("Unsupported format: {puid} ({format_name})")]
    UnsupportedFormat {
        /// PRONOM Unique Identifier for the detected format (e.g., "fmt/101" for XML)
        puid: String,
        /// Human-readable format name from PRONOM registry (e.g., "Extensible Markup Language")
        format_name: String,
    },

    /// Data structure violates expected patterns for the declared format.
    ///
    /// This occurs when:
    /// - JSON lacks consistent object structure across documents
    /// - XML has deeply nested elements exceeding practical schema depth
    /// - CSV contains variable column counts preventing schema inference
    ///
    /// Recovery strategy: Normalize data structure or provide explicit schema hints.
    #[error("Invalid data structure: {0}")]
    InvalidDataStructure(
        /// Detailed structural validation error describing inconsistency or constraint violation
        String,
    ),

    /// Schema Builder failed to construct valid LinkML schema from statistics.
    ///
    /// This occurs when:
    /// - Collected statistics are insufficient (sample size too small)
    /// - Type inference produces conflicting results across documents
    /// - Required metadata (schema ID, class names) is missing or invalid
    ///
    /// Recovery strategy: Increase sample size or provide explicit type hints.
    #[error("Schema generation failed: {0}")]
    SchemaGenerationFailed(
        /// Schema generation error including missing fields or validation failures
        String,
    ),

    /// Inference configuration contains invalid or contradictory settings.
    ///
    /// This occurs when:
    /// - Minimum confidence threshold exceeds 1.0 or is negative
    /// - Sample size limits are set to zero
    /// - Parallel processing settings exceed available resources
    ///
    /// Recovery strategy: Validate configuration against documented constraints.
    #[error("Configuration error: {0}")]
    ConfigurationError(
        /// Configuration validation error specifying invalid parameter and valid range
        String,
    ),

    /// Type Inferencer could not determine consistent data type from samples.
    ///
    /// This occurs when:
    /// - Sample values are too heterogeneous (e.g., mixed numbers and strings)
    /// - All samples are null/empty preventing type determination
    /// - Format-specific type detection fails (e.g., invalid datetime formats)
    ///
    /// Recovery strategy: Provide more consistent samples or explicit type annotations.
    #[error("Type inference error: {0}")]
    TypeInferenceError(
        /// Type inference error describing sample inconsistency or detection failure reason
        String,
    ),
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
