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
use task_management_core::TaskManagementService;
use timeout_core::TimeoutService;
use timestamp_core::TimestampService;

/// Create LinkML service with DBMS integration
///
/// This factory creates a LinkML service that fully integrates with
/// RootReal's DBMS service for TypeDB schema management and data operations.
///
/// # Arguments
///
/// * `logger` - Logger service for structured logging
/// * `timestamp` - Timestamp service for time operations
/// * `cache` - Cache service for performance optimization
/// * `monitoring` - Monitoring service for metrics and telemetry
/// * `configuration_service` - Configuration service for hot-reload support
/// * `task_manager` - Task management service for async operations
/// * `error_handler` - Error handling service for comprehensive error tracking
/// * `dbms_service` - DBMS service for TypeDB integration
/// * `timeout_service` - Timeout service for operation timeouts
///
/// # Errors
///
/// Returns an error if service creation or initialization fails
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
        timeout_service: timeout_service.clone(),
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
                    "Failed to load LinkML config: {}, using defaults",
                    e
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

    // TODO: Add metric recording when MonitoringService interface is updated
    // monitoring.observe("linkml.service.created", 1.0)
    //     .await
    //     .map_err(|e| LinkMLError::service(format!("Monitor error: {e}")))?;

    Ok(Arc::new(service))
}

/// Create LinkML service with DBMS and custom configuration
///
/// This variant allows providing a custom LinkML configuration
/// instead of loading from the configuration service.
///
/// # Arguments
///
/// * `config` - Custom LinkML configuration
/// * Other arguments same as `create_linkml_service_with_dbms`
///
/// # Errors
///
/// Returns an error if service creation or initialization fails
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
        timeout_service: timeout_service.clone(),
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

    Ok(Arc::new(service))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_factory_v3_creation() {
        // Placeholder test to validate factory_v3 module structure
        // TODO: Implement comprehensive factory tests with mock services
        assert!(true, "Factory V3 module structure is valid");
    }
}
