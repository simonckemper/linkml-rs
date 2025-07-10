//! Factory functions for creating LinkML service instances

use std::sync::Arc;

use crate::service::LinkMLServiceImpl;
use linkml_core::{error::Result, config::LinkMLConfig};

// RootReal service dependencies
use logger_core::LoggerService;
use timestamp_core::TimestampService;
use task_management_core::TaskManagementService;
use error_handling_core::ErrorHandlingService;
use configuration_core::ConfigurationService;
use cache_core::CacheService;
use monitoring_core::MonitoringService;

/// Create a new LinkML service instance with all dependencies
/// 
/// Generic parameters for non-dyn-compatible services:
/// - `T`: TaskManagementService implementation
/// - `E`: ErrorHandlingService implementation
/// - `C`: ConfigurationService implementation
pub async fn create_linkml_service<T, E, C>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    config_service: Arc<C>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
{
    // Create service
    let service = LinkMLServiceImpl::new(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    )?;
    
    // Initialize
    service.initialize().await?;
    
    Ok(Arc::new(service))
}

/// Create a LinkML service with custom configuration
/// 
/// Generic parameters for non-dyn-compatible services:
/// - `T`: TaskManagementService implementation
/// - `E`: ErrorHandlingService implementation
/// - `C`: ConfigurationService implementation
pub async fn create_linkml_service_with_config<T, E, C>(
    config: LinkMLConfig,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    config_service: Arc<C>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
{
    // Create service with custom config
    let service = LinkMLServiceImpl::with_config(
        config,
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    )?;
    
    // Initialize
    service.initialize().await?;
    
    Ok(Arc::new(service))
}