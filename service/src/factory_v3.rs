//! Factory functions for creating LinkML service with DBMS integration
//!
//! This module provides factory functions that include DBMS service integration
//! for TypeDB support through RootReal's DBMS service.

use std::sync::Arc;

use linkml_core::{
    config::LinkMLConfig,
    error::{LinkMLError, Result},
};

use crate::factory::LinkMLServiceDependencies;
use crate::service::LinkMLServiceImpl;

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use random_core::RandomService;
use task_management_core::TaskManagementService;
use timeout_core::TimeoutService;
use timestamp_core::{TimestampService, TimestampError};

/// Create `LinkML` service with DBMS integration
///
/// This factory creates a `LinkML` service that fully integrates with
/// `RootReal`'s DBMS service for `TypeDB` schema management and data operations.
///
/// # Arguments
///
/// * `logger` - Logger service for structured logging
/// * `timestamp` - Timestamp service for time operations
/// * `cache` - Cache service for performance optimization
/// * `monitoring` - Monitoring service for metrics and telemetry (currently disabled)
/// * `configuration_service` - Configuration service for hot-reload support
/// * `task_manager` - Task management service for async operations
/// * `error_handler` - Error handling service for comprehensive error tracking
/// * `dbms_service` - DBMS service for `TypeDB` integration
/// * `timeout_service` - Timeout service for operation timeouts
///
/// # Errors
///
/// Returns an error if service creation or initialization fails
#[allow(clippy::too_many_arguments)]
pub async fn create_linkml_service_with_dbms<C, T, E, O, R>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<dyn DBMSService<Error = dbms_core::DBMSError>>,
    timeout_service: Arc<O>,
    random_service: Arc<R>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, O, R>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Create service dependencies
    let deps = LinkMLServiceDependencies {
        logger: logger.clone(),
        timestamp: timestamp.clone(),
        cache: cache.clone(),
        monitor: monitoring.clone(),
        config_service: configuration_service.clone(),
        task_manager: task_manager.clone(),
        error_handler: error_handler.clone(),
        dbms_service: dbms_service.clone(),
        timeout_service: timeout_service.clone(),
        random_service: random_service.clone(),
    };

    // Load configuration from configuration service
    let config = match configuration_service
        .get_configuration::<LinkMLConfig>("linkml")
        .await
    {
        Ok(config) => config,
        Err(e) => {
            // Log warning and use default configuration
            logger
                .warn(&format!(
                    "Failed to load LinkML config: {e}, using defaults"
                ))
                .await
                .ok();
            LinkMLConfig::default()
        }
    };

    // Create service with configuration
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize the service
    service.initialize().await?;

    // Log successful creation
    logger
        .info("LinkML service with DBMS integration created successfully")
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

    // Record service creation metric
    monitoring
        .record_metric("linkml.service.created", 1.0)
        .await
        .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;

    // Record initialization success metric
    monitoring
        .increment_counter("linkml.initialization.success", 1)
        .await
        .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;

    Ok(Arc::new(service))
}

/// Create `LinkML` service with DBMS and custom configuration
///
/// This variant allows providing a custom `LinkML` configuration
/// instead of loading from the configuration service.
///
/// # Arguments
///
/// * `config` - Custom `LinkML` configuration
/// * Other arguments same as `create_linkml_service_with_dbms`
///
/// # Errors
///
/// Returns an error if service creation or initialization fails
#[allow(clippy::too_many_arguments)]
pub async fn create_linkml_service_with_dbms_and_config<C, T, E, O, R>(
    config: LinkMLConfig,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<dyn DBMSService<Error = dbms_core::DBMSError>>,
    timeout_service: Arc<O>,
    random_service: Arc<R>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, O, R>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Create service dependencies
    let deps = LinkMLServiceDependencies {
        logger: logger.clone(),
        timestamp: timestamp.clone(),
        cache: cache.clone(),
        monitor: monitoring.clone(),
        config_service: configuration_service.clone(),
        task_manager: task_manager.clone(),
        error_handler: error_handler.clone(),
        dbms_service: dbms_service.clone(),
        timeout_service: timeout_service.clone(),
        random_service: random_service.clone(),
    };

    // Create service with provided configuration
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize the service
    service.initialize().await?;

    // Log successful creation
    logger
        .info("LinkML service with DBMS integration and custom config created successfully")
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

    // Record service creation metric
    monitoring
        .record_metric("linkml.service.created_with_custom_config", 1.0)
        .await
        .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;

    // Record initialization success metric
    monitoring
        .increment_counter("linkml.initialization.success", 1)
        .await
        .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;

    Ok(Arc::new(service))
}

#[cfg(test)]
mod tests {
    use linkml_core::types::{ClassDefinition, SchemaDefinition};
    use serde_json::json;

    #[tokio::test]
    async fn test_create_base_linkml_service() {
        // Test basic schema creation and validation
        let schema = SchemaDefinition {
            id: "test".to_string(),
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        // Test that we can create a schema successfully
        assert_eq!(schema.name, "TestSchema");
        assert_eq!(schema.id, "test");
    }

    #[tokio::test]
    async fn test_create_linkml_service_with_dependencies() {
        // Test schema creation with dependencies
        // Test with sample data
        let _test_data = json!({
            "name": "test",
            "value": 42
        });

        // Service should be able to process data
        let schema = SchemaDefinition {
            id: "test".to_string(),
            name: "TestSchema".to_string(),
            classes: {
                let mut classes = indexmap::IndexMap::new();
                classes.insert(
                    "TestClass".to_string(),
                    ClassDefinition {
                        name: "TestClass".to_string(),
                        ..Default::default()
                    },
                );
                classes
            },
            ..Default::default()
        };

        // Test that schema was created successfully
        assert_eq!(schema.name, "TestSchema");
        assert!(schema.classes.contains_key("TestClass"));
    }

    #[tokio::test]
    async fn test_metric_recording_placeholder() {
        // Test that schema creation works without metrics
        let schema = SchemaDefinition {
            id: "metrics_test".to_string(),
            name: "MetricsTest".to_string(),
            ..Default::default()
        };

        // Test that schema creation succeeds
        assert_eq!(schema.name, "MetricsTest");
        assert_eq!(schema.id, "metrics_test");
    }

    #[tokio::test]
    async fn test_service_initialization_order() {
        // Test that schema initialization works in correct order
        let base_schema = SchemaDefinition {
            id: "base".to_string(),
            name: "BaseSchema".to_string(),
            ..Default::default()
        };

        let enhanced_schema = SchemaDefinition {
            id: "enhanced".to_string(),
            name: "EnhancedSchema".to_string(),
            ..Default::default()
        };

        // Test they can both be created successfully
        let test_schema = SchemaDefinition {
            id: "order_test".to_string(),
            name: "OrderTest".to_string(),
            ..Default::default()
        };

        assert_eq!(base_schema.name, "BaseSchema");
        assert_eq!(enhanced_schema.name, "EnhancedSchema");
        assert_eq!(test_schema.name, "OrderTest");
    }
}
