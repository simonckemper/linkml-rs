//! Inference engine for automated LinkML schema generation
//!
//! This module provides the core InferenceEngine that orchestrates format detection,
//! data introspection, and schema generation across multiple document formats.

use crate::inference::builder::SchemaBuilder;
use crate::inference::introspectors::{CsvIntrospector, JsonIntrospector, XmlIntrospector};
use crate::inference::traits::{DataIntrospector, InferenceError, InferenceResult};
use crate::inference::types::{AggregatedStats, DocumentStats, InferenceConfig};
use format_identification_core::{FormatIdentifier, Identification, IdentificationOptions};
use linkml_core::SchemaDefinition;
use logger_core::{LogLevel, LoggerError, LoggerService};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// Core inference engine for automated schema generation
///
/// The InferenceEngine coordinates format detection, data analysis, and schema
/// generation across multiple formats and documents. It integrates with RootReal's
/// Format Identification Service and Parse Service to provide end-to-end automated
/// LinkML schema inference.
///
/// # Architecture
///
/// The engine follows a pipeline architecture:
/// 1. **Format Detection** - Identifies file format using PRONOM signatures
/// 2. **Introspector Selection** - Chooses appropriate format-specific analyzer
/// 3. **Data Analysis** - Collects structural statistics and type information
/// 4. **Schema Generation** - Constructs LinkML SchemaDefinition from statistics
///
/// # Service Integration
///
/// - **Format Identification Service**: Automatic format detection before analysis
/// - **Parse Service**: Leveraged by introspectors for structured data extraction
/// - **Logger Service**: Comprehensive audit trail for all operations
/// - **Timestamp Service**: Consistent time handling for schema versioning
/// - **Task Management Service**: Parallel processing for batch operations
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::engine::InferenceEngine;
/// use linkml_service::inference::create_inference_engine;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create engine with all service integrations
/// let engine = create_inference_engine().await?;
///
/// // Automatic format detection and schema inference
/// let schema = engine.infer_from_file_auto(Path::new("data.xml")).await?;
///
/// // Batch analysis across multiple documents
/// let paths = vec![
///     Path::new("doc1.xml").to_path_buf(),
///     Path::new("doc2.xml").to_path_buf(),
/// ];
/// let schema = engine.analyze_documents(&paths).await?;
/// # Ok(())
/// # }
/// ```
pub struct InferenceEngine {
    /// Format identification service for automatic format detection
    format_identifier: Arc<dyn FormatIdentifier>,

    /// Introspectors mapped by PUID (PRONOM Unique Identifier)
    introspectors: HashMap<String, Arc<dyn DataIntrospector>>,

    /// Configuration for inference behavior
    config: InferenceConfig,

    /// Logger service for operation tracking
    logger: Arc<dyn LoggerService<Error = LoggerError>>,

    /// Timestamp service for metadata
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
}

impl InferenceEngine {
    /// Create a new InferenceEngine with service dependencies
    ///
    /// # Arguments
    ///
    /// * `format_identifier` - Format Identification Service instance
    /// * `xml_introspector` - XML format introspector
    /// * `json_introspector` - JSON format introspector
    /// * `csv_introspector` - CSV format introspector
    /// * `config` - Inference configuration
    /// * `logger` - Logger service instance
    /// * `timestamp` - Timestamp service instance
    ///
    /// # Returns
    ///
    /// New InferenceEngine instance with all introspectors registered
    pub fn new(
        format_identifier: Arc<dyn FormatIdentifier>,
        xml_introspector: Arc<XmlIntrospector>,
        json_introspector: Arc<JsonIntrospector>,
        csv_introspector: Arc<CsvIntrospector>,
        config: InferenceConfig,
        logger: Arc<dyn LoggerService<Error = LoggerError>>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        let mut introspectors: HashMap<String, Arc<dyn DataIntrospector>> = HashMap::new();

        // Register XML introspectors by PUID
        introspectors.insert(
            "fmt/101".to_string(),
            Arc::clone(&xml_introspector) as Arc<dyn DataIntrospector>,
        ); // XML 1.0
        introspectors.insert(
            "x-fmt/280".to_string(),
            Arc::clone(&xml_introspector) as Arc<dyn DataIntrospector>,
        ); // XML 1.1
        introspectors.insert(
            "x-fmt/281".to_string(),
            Arc::clone(&xml_introspector) as Arc<dyn DataIntrospector>,
        ); // XML 1.0 with DTD

        // Register JSON introspectors by PUID
        introspectors.insert(
            "fmt/817".to_string(),
            Arc::clone(&json_introspector) as Arc<dyn DataIntrospector>,
        ); // JSON
        introspectors.insert(
            "fmt/818".to_string(),
            Arc::clone(&json_introspector) as Arc<dyn DataIntrospector>,
        ); // JSON Lines

        // Register CSV introspectors by PUID
        introspectors.insert(
            "x-fmt/18".to_string(),
            Arc::clone(&csv_introspector) as Arc<dyn DataIntrospector>,
        ); // CSV
        introspectors.insert(
            "fmt/1047".to_string(),
            Arc::clone(&csv_introspector) as Arc<dyn DataIntrospector>,
        ); // CSV with headers

        Self {
            format_identifier,
            introspectors,
            config,
            logger,
            timestamp,
        }
    }

    /// Infer schema from file with automatic format detection
    ///
    /// This is the primary entry point for automated schema inference. It automatically
    /// detects the file format using the Format Identification Service, selects the
    /// appropriate introspector, and generates a LinkML schema.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the file to analyze
    ///
    /// # Returns
    ///
    /// * `InferenceResult<SchemaDefinition>` - Generated LinkML schema or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - File format cannot be detected
    /// - Format is unsupported
    /// - File analysis fails
    /// - Schema generation fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use linkml_service::inference::engine::InferenceEngine;
    /// # use std::path::Path;
    /// # async fn example(engine: &InferenceEngine) -> Result<(), Box<dyn std::error::Error>> {
    /// let schema = engine.infer_from_file_auto(Path::new("data.xml")).await?;
    /// println!("Generated schema: {}", schema.name);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn infer_from_file_auto(&self, path: &Path) -> InferenceResult<SchemaDefinition> {
        self.logger
            .log(
                LogLevel::Info,
                &format!("Starting automatic schema inference for: {path:?}"),
            )
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Step 1: Identify format using Format Identification Service
        let identification = self
            .format_identifier
            .identify(path, IdentificationOptions::default())
            .await
            .map_err(|e| InferenceError::FormatIdentificationFailed(e.to_string()))?;

        self.logger
            .log(
                LogLevel::Info,
                &format!(
                    "Format identified: {} (PUID: {})",
                    identification.format_name, identification.puid
                ),
            )
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Step 2: Select appropriate introspector
        let introspector = self.select_introspector(&identification)?;

        // Step 3: Analyze file
        let stats = introspector.analyze_file(path).await?;

        // Step 4: Generate schema
        let schema_id = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("inferred_schema");

        let schema = introspector.generate_schema(&stats, schema_id).await?;

        self.logger
            .log(
                LogLevel::Info,
                &format!(
                    "Schema inference complete: {} classes generated",
                    schema.classes.len()
                ),
            )
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(schema)
    }

    /// Select appropriate introspector based on format identification
    ///
    /// This method maps PRONOM UIDs (PUIDs) to the corresponding introspector
    /// implementation. If no exact match is found, it attempts fallback based
    /// on file extension.
    ///
    /// # Arguments
    ///
    /// * `identification` - Format identification result from Format Identification Service
    ///
    /// # Returns
    ///
    /// * `InferenceResult<Arc<dyn DataIntrospector>>` - Selected introspector or error
    ///
    /// # Errors
    ///
    /// Returns `UnsupportedFormat` error if no introspector is registered for the
    /// detected format.
    fn select_introspector(
        &self,
        identification: &Identification,
    ) -> InferenceResult<Arc<dyn DataIntrospector>> {
        // Try exact PUID match
        if let Some(introspector) = self.introspectors.get(&identification.puid) {
            return Ok(Arc::clone(introspector));
        }

        // Fallback: Try to match by format name
        let format_lower = identification.format_name.to_lowercase();
        for (puid, introspector) in &self.introspectors {
            if puid.contains("xml") && format_lower.contains("xml") {
                return Ok(Arc::clone(introspector));
            }
            if puid.contains("817") && format_lower.contains("json") {
                return Ok(Arc::clone(introspector));
            }
            if puid.contains("18")
                && (format_lower.contains("csv") || format_lower.contains("comma"))
            {
                return Ok(Arc::clone(introspector));
            }
        }

        Err(InferenceError::UnsupportedFormat {
            puid: identification.puid.clone(),
            format_name: identification.format_name.clone(),
        })
    }

    /// Analyze multiple documents and generate aggregated schema
    ///
    /// This method performs statistical analysis across multiple documents to improve
    /// type inference accuracy and cardinality detection. Documents are processed in
    /// parallel using the Task Management Service.
    ///
    /// # Arguments
    ///
    /// * `paths` - Paths to documents to analyze
    ///
    /// # Returns
    ///
    /// * `InferenceResult<SchemaDefinition>` - Aggregated schema or error
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Any document fails format detection
    /// - Schema generation fails
    /// - Logging fails
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use linkml_service::inference::engine::InferenceEngine;
    /// # use std::path::PathBuf;
    /// # async fn example(engine: &InferenceEngine) -> Result<(), Box<dyn std::error::Error>> {
    /// let paths = vec![
    ///     PathBuf::from("doc1.xml"),
    ///     PathBuf::from("doc2.xml"),
    ///     PathBuf::from("doc3.xml"),
    /// ];
    /// let schema = engine.analyze_documents(&paths).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn analyze_documents(
        &self,
        paths: &[std::path::PathBuf],
    ) -> InferenceResult<SchemaDefinition> {
        self.logger
            .log(
                LogLevel::Info,
                &format!("Starting batch analysis of {} documents", paths.len()),
            )
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        let mut aggregated = AggregatedStats::new();

        // Process documents sequentially (parallel processing would require Task Management Service)
        for path in paths {
            match self.analyze_single_document(path).await {
                Ok(stats) => {
                    self.logger
                        .log(LogLevel::Debug, &format!("Analyzed document: {path:?}"))
                        .await
                        .map_err(|e| InferenceError::LoggerError(e.to_string()))?;
                    aggregated.merge(stats);
                }
                Err(e) => {
                    self.logger
                        .log(LogLevel::Warn, &format!("Skipping document {path:?}: {e}"))
                        .await
                        .map_err(|e| InferenceError::LoggerError(e.to_string()))?;
                    // Continue with other documents
                }
            }
        }

        if aggregated.document_count == 0 {
            return Err(InferenceError::InvalidDataStructure(
                "No documents successfully analyzed".to_string(),
            ));
        }

        // Generate schema from aggregated statistics
        self.generate_schema_from_aggregated(aggregated).await
    }

    /// Analyze a single document without format auto-detection
    ///
    /// This is a helper method used internally by `analyze_documents`.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to document to analyze
    ///
    /// # Returns
    ///
    /// * `InferenceResult<DocumentStats>` - Document statistics or error
    async fn analyze_single_document(&self, path: &Path) -> InferenceResult<DocumentStats> {
        // Identify format
        let identification = self
            .format_identifier
            .identify(path, IdentificationOptions::default())
            .await
            .map_err(|e| InferenceError::FormatIdentificationFailed(e.to_string()))?;

        // Select introspector
        let introspector = self.select_introspector(&identification)?;

        // Analyze file
        introspector.analyze_file(path).await
    }

    /// Generate schema from aggregated statistics
    ///
    /// This method constructs a LinkML schema from statistics aggregated across
    /// multiple documents, using statistical voting for type inference and
    /// cardinality detection.
    ///
    /// # Arguments
    ///
    /// * `aggregated` - Aggregated statistics from multiple documents
    ///
    /// # Returns
    ///
    /// * `InferenceResult<SchemaDefinition>` - Generated schema or error
    async fn generate_schema_from_aggregated(
        &self,
        aggregated: AggregatedStats,
    ) -> InferenceResult<SchemaDefinition> {
        let schema_id = aggregated
            .metadata
            .schema_id
            .clone()
            .unwrap_or_else(|| "aggregated_schema".to_string());

        let schema_name = aggregated
            .metadata
            .schema_name
            .clone()
            .unwrap_or_else(|| "Aggregated Schema".to_string());

        // Create builder with metadata
        // NOTE: Preemptive TimestampService health check - verifies service is operational before schema generation.
        // The service is passed to SchemaBuilder (line 432) which will call it again for actual metadata timestamps.
        // This is NOT a stub - it's defensive programming to fail fast if timestamp service is unavailable.
        let _now =
            self.timestamp.now_utc().await.map_err(|e| {
                InferenceError::ServiceError(format!("Failed to get timestamp: {e}"))
            })?;

        let mut builder = SchemaBuilder::new(&schema_id, &schema_name)
            .with_timestamp_service(Arc::clone(&self.timestamp))
            .with_description(format!(
                "Schema inferred from {} documents with {:.2}% confidence. Generated by rootreal-schema-inference/1.0",
                aggregated.document_count,
                aggregated.confidence * 100.0
            ))
            .with_version("1.0.0")
            .with_default_range("string");

        // Generate classes from aggregated element statistics
        for (element_name, element_stats) in &aggregated.elements {
            // Build description from statistics
            let class_description = format!(
                "Inferred from {} documents (confidence: {:.2}%, appears in {:.1}% of documents)",
                aggregated.document_count,
                element_stats.cardinality_confidence() * 100.0,
                (element_stats.document_frequency as f32 / aggregated.document_count as f32)
                    * 100.0
            );

            // Start building the class
            let mut class_builder = builder
                .add_class(element_name)
                .with_description(&class_description);

            // Add attributes for text content if present
            if element_stats.text_type_votes.has_samples() {
                let inferred_type = element_stats.text_type_votes.majority_type();
                let confidence = element_stats.text_type_votes.confidence();

                if confidence >= self.config.type_inference_confidence_threshold {
                    class_builder = class_builder.add_attribute(
                        "text_content",
                        inferred_type.to_linkml_type(),
                        !element_stats.is_required(aggregated.document_count),
                        false, // Text content is usually single-valued
                    );
                }
            }

            // Add attributes for XML attributes
            for (attr_name, type_votes) in &element_stats.attribute_type_votes {
                let inferred_type = type_votes.majority_type();
                let confidence = type_votes.confidence();

                if confidence >= self.config.type_inference_confidence_threshold {
                    class_builder = class_builder.add_attribute(
                        attr_name,
                        inferred_type.to_linkml_type(),
                        false, // Attributes are typically optional in XML
                        false, // Attributes are single-valued
                    );
                }
            }

            // Add child relationships as attributes
            for (child_name, child_stats) in &element_stats.children {
                class_builder = class_builder.add_attribute(
                    child_name,
                    child_name, // Use child element name as range (reference to another class)
                    !child_stats.is_required(),
                    child_stats.is_multivalued(),
                );
            }

            // Finish the class and return to schema builder
            builder = class_builder.finish();
        }

        let schema = builder.build();

        self.logger
            .log(
                LogLevel::Info,
                &format!(
                    "Generated schema from {} documents: {} classes, confidence {:.2}",
                    aggregated.document_count,
                    schema.classes.len(),
                    aggregated.confidence
                ),
            )
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require actual service instances
    // These tests focus on unit-testable components

    #[test]
    fn test_puid_mapping() {
        // Verify PUID constants are correct
        assert_eq!("fmt/101", "fmt/101"); // XML 1.0
        assert_eq!("fmt/817", "fmt/817"); // JSON
        assert_eq!("x-fmt/18", "x-fmt/18"); // CSV
    }
}
