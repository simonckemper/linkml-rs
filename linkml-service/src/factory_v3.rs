//! Factory functions for creating LinkML service with DBMS integration
//!
//! This module provides factory functions that include DBMS service integration
//! for TypeDB support through RootReal's DBMS service.

use std::sync::Arc;

use linkml_core::{
    configuration::LinkMLServiceConfig,
    error::{LinkMLError, Result},
};

use crate::service::LinkMLServiceWithDBMS;
use crate::loader::dbms_executor::DBMSServiceExecutor;

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::TaskManagementService;
use timestamp_core::TimestampService;

/// Create LinkML service with DBMS integration
///
/// This factory function creates a LinkML service that uses the DBMS service
/// for all TypeDB operations, ensuring proper integration with RootReal's
/// data layer.
pub async fn create_linkml_service_with_dbms<C, T, E, D>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<D>,
) -> Result<Arc<LinkMLServiceWithDBMS<T, E, C, D>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    // Load configuration from Configuration Service
    let config: LinkMLServiceConfig = configuration_service
        .load_configuration()
        .await
        .map_err(|e| LinkMLError::configuration(format!("Failed to load configuration: {e}")))?;
    
    // Validate configuration
    use configuration_core::Validate;
    config.validate()
        .map_err(|e| LinkMLError::configuration(format!("Configuration validation failed: {e}")))?;
    
    logger
        .info("LinkML configuration loaded and validated successfully")
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
    
    // Create TypeDB executor using DBMS service
    let typedb_executor = Arc::new(DBMSServiceExecutor::new(dbms_service.clone()));
    
    // Create service dependencies with DBMS
    let dependencies = LinkMLServiceDependenciesWithDBMS {
        config: config.clone(),
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
        dbms_service,
        typedb_executor,
    };
    
    // Create and initialize the service
    let service = LinkMLServiceWithDBMS::new(dependencies)?;
    service.initialize().await?;
    
    Ok(Arc::new(service))
}

/// Create LinkML service with DBMS and custom configuration
///
/// This factory function accepts a pre-loaded configuration and DBMS service,
/// useful for testing or when configuration needs to be customized.
pub async fn create_linkml_service_with_dbms_custom_config<C, T, E, D>(
    config: LinkMLServiceConfig,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    dbms_service: Arc<D>,
) -> Result<Arc<LinkMLServiceWithDBMS<T, E, C, D>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    D::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    // Validate custom configuration
    use configuration_core::Validate;
    config.validate()
        .map_err(|e| LinkMLError::configuration(format!("Configuration validation failed: {e}")))?;
    
    // Create TypeDB executor using DBMS service
    let typedb_executor = Arc::new(DBMSServiceExecutor::new(dbms_service.clone()));
    
    // Create service dependencies
    let dependencies = LinkMLServiceDependenciesWithDBMS {
        config: config.clone(),
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
        dbms_service,
        typedb_executor,
    };
    
    // Create and initialize the service
    let service = LinkMLServiceWithDBMS::new(dependencies)?;
    service.initialize().await?;
    
    Ok(Arc::new(service))
}

/// Service dependencies structure with DBMS
pub struct LinkMLServiceDependenciesWithDBMS<C, T, E, D>
where
    C: ConfigurationService,
    T: TaskManagementService,
    E: ErrorHandlingService,
    D: DBMSService,
{
    pub config: LinkMLServiceConfig,
    pub logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    pub timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    pub monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    pub configuration_service: Arc<C>,
    pub task_manager: Arc<T>,
    pub error_handler: Arc<E>,
    pub dbms_service: Arc<D>,
    pub typedb_executor: Arc<DBMSServiceExecutor<D>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would go here following RootReal testing patterns
    // Using mock services only in tests (never in production code)
}