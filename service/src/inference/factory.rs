//! Factory functions for inference service creation
//!
//! This module provides factory functions following RootReal patterns for creating
//! inference engines and introspectors with proper dependency injection.

#![allow(deprecated)]

use crate::inference::engine::InferenceEngine;
use crate::inference::introspectors::{CsvIntrospector, JsonIntrospector, XmlIntrospector};
use crate::inference::traits::InferenceResult;
use crate::inference::types::InferenceConfig;
use format_identification_service::create_format_identification_service;
use logger_core::{LoggerError, LoggerService};
use logger_service::factory::create_logger_service;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};
use timestamp_service::wiring::wire_timestamp;

/// Factory function to create a  service instance
///
/// # Deprecated
///
/// This factory function is deprecated in favor of the new wiring-based pattern.
/// Use `_service::wiring::wire_()` instead for better composability.
///
/// See the [Facade Migration Guide](https://docs.rootreal.com/guides/facade-migration-guide.md)
/// for complete migration instructions.
///
/// # Migration Example
///
/// ```rust,ignore
/// // Old pattern (deprecated):
/// let service = create_xml_introspector();
///
/// // New pattern (recommended):
/// use _service::wiring::wire_;
/// let service = wire_()?.into_arc();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use `_service::wiring::wire_()` instead.
            See docs/guides/facade-migration-guide.md for migration instructions."
)]
/// Create a fully-configured XML introspector
///
/// # Arguments
///
/// * `logger` - Logger service instance
/// * `timestamp` - Timestamp service instance
///
/// # Returns
///
/// * `Arc<XmlIntrospector>` - Configured XML introspector
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::create_xml_introspector;
/// use logger_service::test_utils::create_test_logger_service;
/// use timestamp_service::create_timestamp_service;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = create_logger_service()?;
/// let timestamp = wire_timestamp()?;
/// let introspector = create_xml_introspector(logger, timestamp)?;
/// # Ok(())
/// # }
/// ```
pub fn create_xml_introspector(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> InferenceResult<Arc<XmlIntrospector>> {
    Ok(Arc::new(XmlIntrospector::new(logger, timestamp)))
}

/// Factory function to create a  service instance
///
/// # Deprecated
///
/// This factory function is deprecated in favor of the new wiring-based pattern.
/// Use `_service::wiring::wire_()` instead for better composability.
///
/// See the [Facade Migration Guide](https://docs.rootreal.com/guides/facade-migration-guide.md)
/// for complete migration instructions.
///
/// # Migration Example
///
/// ```rust,ignore
/// // Old pattern (deprecated):
/// let service = create_json_introspector();
///
/// // New pattern (recommended):
/// use _service::wiring::wire_;
/// let service = wire_()?.into_arc();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use `_service::wiring::wire_()` instead.
            See docs/guides/facade-migration-guide.md for migration instructions."
)]
/// Create a fully-configured JSON introspector
///
/// # Arguments
///
/// * `logger` - Logger service instance
/// * `timestamp` - Timestamp service instance
///
/// # Returns
///
/// * `Arc<JsonIntrospector>` - Configured JSON introspector
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::create_json_introspector;
/// use logger_service::test_utils::create_test_logger_service;
/// use timestamp_service::create_timestamp_service;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = create_logger_service()?;
/// let timestamp = wire_timestamp()?;
/// let introspector = create_json_introspector(logger, timestamp)?;
/// # Ok(())
/// # }
/// ```
pub fn create_json_introspector(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> InferenceResult<Arc<JsonIntrospector>> {
    Ok(Arc::new(JsonIntrospector::new(logger, timestamp)))
}

/// Factory function to create a  service instance
///
/// # Deprecated
///
/// This factory function is deprecated in favor of the new wiring-based pattern.
/// Use `_service::wiring::wire_()` instead for better composability.
///
/// See the [Facade Migration Guide](https://docs.rootreal.com/guides/facade-migration-guide.md)
/// for complete migration instructions.
///
/// # Migration Example
///
/// ```rust,ignore
/// // Old pattern (deprecated):
/// let service = create_csv_introspector();
///
/// // New pattern (recommended):
/// use _service::wiring::wire_;
/// let service = wire_()?.into_arc();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use `_service::wiring::wire_()` instead.
            See docs/guides/facade-migration-guide.md for migration instructions."
)]
/// Create a fully-configured CSV introspector
///
/// # Arguments
///
/// * `logger` - Logger service instance
/// * `timestamp` - Timestamp service instance
///
/// # Returns
///
/// * `Arc<CsvIntrospector>` - Configured CSV introspector
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::create_csv_introspector;
/// use logger_service::test_utils::create_test_logger_service;
/// use timestamp_service::create_timestamp_service;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = create_logger_service()?;
/// let timestamp = wire_timestamp()?;
/// let introspector = create_csv_introspector(logger, timestamp)?;
/// # Ok(())
/// # }
/// ```
pub fn create_csv_introspector(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> InferenceResult<Arc<CsvIntrospector>> {
    Ok(Arc::new(CsvIntrospector::new(logger, timestamp)))
}

/// Create a fully-configured inference engine with all services
///
/// This factory function creates an InferenceEngine with complete service integration:
/// - Format Identification Service for automatic format detection
/// - All introspectors (XML, JSON, CSV) with Parse Service integration
/// - Logger Service for comprehensive audit trails
/// - Timestamp Service for schema versioning
///
/// # Returns
///
/// * `InferenceResult<Arc<InferenceEngine>>` - Configured inference engine or error
///
/// # Errors
///
/// Returns error if any service creation fails
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::create_inference_engine;
/// use std::path::Path;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let engine = create_inference_engine().await?;
/// let schema = engine.infer_from_file_auto(Path::new("data.xml")).await?;
/// println!("Generated schema: {}", schema.name);
/// # Ok(())
/// # }
/// ```
pub async fn create_inference_engine() -> InferenceResult<Arc<InferenceEngine>> {
    // Create core services
    let logger = create_logger_service()
        .await
        .map_err(|e| crate::inference::traits::InferenceError::ServiceError(e.to_string()))?;

    let timestamp = wire_timestamp();

    create_inference_engine_with_services(logger, timestamp.into_inner()).await
}

/// Create inference engine with provided services
///
/// This factory function allows callers to provide their own service instances,
/// useful for testing or when services are already initialized elsewhere.
///
/// # Arguments
///
/// * `logger` - Logger service instance
/// * `timestamp` - Timestamp service instance
///
/// # Returns
///
/// * `InferenceResult<Arc<InferenceEngine>>` - Configured inference engine or error
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::create_inference_engine_with_services;
/// use logger_service::test_utils::create_test_logger_service;
/// use timestamp_service::create_timestamp_service;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = create_logger_service()?;
/// let timestamp = wire_timestamp()?;
/// let engine = create_inference_engine_with_services(logger, timestamp).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_inference_engine_with_services(
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> InferenceResult<Arc<InferenceEngine>> {
    // Create format identification service
    let format_identifier = create_format_identification_service().await.map_err(|e| {
        crate::inference::traits::InferenceError::FormatIdentificationFailed(e.to_string())
    })?;

    // Create introspectors using direct instantiation (avoiding deprecated factory functions)
    let xml_introspector = Arc::new(XmlIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    let json_introspector = Arc::new(JsonIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    let csv_introspector = Arc::new(CsvIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    // Use default configuration
    let config = InferenceConfig::default();

    // Create inference engine
    Ok(Arc::new(InferenceEngine::new(
        format_identifier,
        xml_introspector,
        json_introspector,
        csv_introspector,
        config,
        logger,
        timestamp,
    )))
}

/// Create inference engine with custom configuration
///
/// # Arguments
///
/// * `config` - Custom inference configuration
/// * `logger` - Logger service instance
/// * `timestamp` - Timestamp service instance
///
/// # Returns
///
/// * `InferenceResult<Arc<InferenceEngine>>` - Configured inference engine or error
///
/// # Example
///
/// ```rust,no_run
/// use linkml_service::inference::{create_inference_engine_with_config, InferenceConfig};
/// use logger_service::test_utils::create_test_logger_service;
/// use timestamp_service::create_timestamp_service;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let logger = create_logger_service()?;
/// let timestamp = wire_timestamp()?;
///
/// let config = InferenceConfig {
///     min_samples_for_type_inference: 10,
///     type_inference_confidence_threshold: 0.9,
///     generate_descriptions: true,
///     use_heuristic_naming: true,
///     max_nesting_depth: 15,
///     sample_size: Some(500),
/// };
///
/// let engine = create_inference_engine_with_config(config, logger, timestamp).await?;
/// # Ok(())
/// # }
/// ```
pub async fn create_inference_engine_with_config(
    config: InferenceConfig,
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
) -> InferenceResult<Arc<InferenceEngine>> {
    // Create format identification service
    let format_identifier = create_format_identification_service().await.map_err(|e| {
        crate::inference::traits::InferenceError::FormatIdentificationFailed(e.to_string())
    })?;

    // Create introspectors using direct instantiation (avoiding deprecated factory functions)
    let xml_introspector = Arc::new(XmlIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    let json_introspector = Arc::new(JsonIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    let csv_introspector = Arc::new(CsvIntrospector::new(
        Arc::clone(&logger),
        Arc::clone(&timestamp),
    ));

    // Create inference engine with custom config
    Ok(Arc::new(InferenceEngine::new(
        format_identifier,
        xml_introspector,
        json_introspector,
        csv_introspector,
        config,
        logger,
        timestamp,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_introspectors() {
        let logger = logger_service::test_utils::create_test_logger_service()
            .expect("Failed to create logger");
        let timestamp = timestamp_service::wire_timestamp().expect("Failed to create timestamp");

        // Test introspector creation using direct instantiation
        let xml = Arc::new(XmlIntrospector::new(
            Arc::clone(&logger),
            Arc::clone(&timestamp.into_inner()),
        ));
        assert!(xml.introspect_bytes(b"<root/>").await.is_ok());

        let json = Arc::new(JsonIntrospector::new(
            Arc::clone(&logger),
            Arc::clone(&timestamp.into_inner()),
        ));
        assert!(json.introspect_bytes(b"{}").await.is_ok());

        let csv = Arc::new(CsvIntrospector::new(
            Arc::clone(&logger),
            Arc::clone(&timestamp.into_inner()),
        ));
        assert!(csv.introspect_bytes(b"a,b,c\n1,2,3").await.is_ok());
    }

    #[tokio::test]
    async fn test_create_inference_engine() {
        // This will fail if any service is not properly configured
        let result = create_inference_engine().await;
        assert!(
            result.is_ok(),
            "Failed to create inference engine: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_create_inference_engine_with_custom_config() {
        let logger = logger_service::test_utils::create_test_logger_service()
            .expect("Failed to create logger");
        let timestamp = timestamp_service::wire_timestamp().expect("Failed to create timestamp");

        let config = InferenceConfig {
            min_samples_for_type_inference: 10,
            type_inference_confidence_threshold: 0.95,
            generate_descriptions: false,
            use_heuristic_naming: false,
            max_nesting_depth: 20,
            sample_size: Some(1000),
        };

        let result = create_inference_engine_with_config(config, logger, timestamp).await;
        assert!(result.is_ok());
    }
}
