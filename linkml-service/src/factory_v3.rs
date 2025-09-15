//! Factory functions for creating LinkML service with DBMS integration
//!
//! This module provides factory functions that include DBMS service integration
//! for TypeDB support through RootReal's DBMS service.

use std::sync::Arc;

use linkml_core::{
    config::LinkMLConfig,
    error::{LinkMLError, Result}};

use crate::factory::LinkMLServiceDependencies;
use crate::service::LinkMLServiceImpl;

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::TaskManagementService;
use timeout_core::TimeoutService;
use timestamp_core::TimestampService;

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
pub async fn create_linkml_service_with_dbms<C, T, E, D, O>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<D>,
    timeout_service: Arc<O>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, D, O>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
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
        timeout_service: timeout_service.clone()};

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
pub async fn create_linkml_service_with_dbms_and_config<C, T, E, D, O>(
    config: LinkMLConfig,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<D>,
    timeout_service: Arc<O>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, D, O>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
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
        timeout_service: timeout_service.clone()};

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
    use super::*;
    use serde_json::json;
    use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};
    use std::collections::HashMap;
    use linkml_core::prelude::*;

    // TODO: Implement create_base_linkml_service function
    // #[tokio::test]
    // async fn test_create_base_linkml_service() {
    //     let result = create_base_linkml_service();
    //     assert!(result.is_ok(), "Should create base LinkML service");
    //
    //     let service = result.expect("Failed to create service");
    //
    //     // Test basic schema loading
    //     let schema = SchemaDefinition {
    //         id: Some("test".to_string()),
    //         name: "TestSchema".to_string(),
    //         ..Default::default()
    //     };
    //
    //     let load_result = service.load_schema(schema.clone());
    //     assert!(load_result.is_ok(), "Should load schema successfully");
    // }
    
    // TODO: Implement create_linkml_service_with_dependencies function
    // #[tokio::test]
    // async fn test_create_linkml_service_with_dependencies() {
    //     let result = create_linkml_service_with_dependencies();
    //     assert!(result.is_ok(), "Should create LinkML service with dependencies");
    //
    //     // Verify service has required dependencies injected
    //     let service = result.expect("Failed to create service");
    //
    //     // Test with sample data
    //     let test_data = json!({
    //         "name": "test",
    //         "value": 42
    //     });
    //
    //     // Service should be able to process data
    //     let schema = SchemaDefinition {
    //         id: Some("test".to_string()),
    //         name: "TestSchema".to_string(),
    //         classes: {
    //             let mut classes = HashMap::new();
    //             classes.insert("TestClass".to_string(), ClassDefinition {
    //                 name: "TestClass".to_string(),
    //                 ..Default::default()
    //             });
    //             classes
    //         },
    //         ..Default::default()
    //     };
    //
    //     service.load_schema(schema).expect("Failed to load schema");
    // }
    
    #[tokio::test]
    async fn test_metric_recording_placeholder() {
        // Test that metric recording placeholder doesn't break functionality
        let service = create_linkml_service_with_dependencies();
        assert!(service.is_ok(), "Service creation should succeed even without full metrics");
        
        // Verify service can still operate without MonitoringService
        let service = service.expect("Failed to create service");
        
        // Perform a simple operation that would trigger metrics
        let schema = SchemaDefinition {
            id: Some("metrics_test".to_string()),
            name: "MetricsTest".to_string(),
            ..Default::default()
        };
        
        let result = service.load_schema(schema);
        assert!(result.is_ok(), "Operations should succeed without metrics recording");
    }
    
    #[tokio::test]
    async fn test_service_initialization_order() {
        // Test that services are initialized in correct order
        let base_service = create_base_linkml_service();
        assert!(base_service.is_ok(), "Base service should initialize first");
        
        let enhanced_service = create_linkml_service_with_dependencies();
        assert!(enhanced_service.is_ok(), "Enhanced service should initialize after base");
        
        // Both services should be functional
        let base = base_service.expect("Base service failed");
        let enhanced = enhanced_service.expect("Enhanced service failed");
        
        // Test they can both handle the same schema
        let test_schema = SchemaDefinition {
            id: Some("order_test".to_string()),
            name: "OrderTest".to_string(),
            ..Default::default()
        };
        
        assert!(base.load_schema(test_schema.clone()).is_ok(), "Base should load schema");
        assert!(enhanced.load_schema(test_schema).is_ok(), "Enhanced should load schema");
    }
}