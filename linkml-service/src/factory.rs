//! Factory functions for creating LinkML service instances

use std::sync::Arc;

use crate::service::LinkMLServiceImpl;
use linkml_core::{config::LinkMLConfig, error::Result};

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::TaskManagementService;
use timestamp_core::TimestampService;

/// Create a new `LinkML` service instance with all dependencies
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
///
/// # Errors
///
/// Returns an error if service initialization fails
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
    let deps = LinkMLServiceDependencies {
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    };
    let service = LinkMLServiceImpl::new(deps)?;

    // Initialize
    service.initialize().await?;

    Ok(Arc::new(service))
}

/// Create a `LinkML` service with custom configuration
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
///
/// Service dependencies for `LinkML` service creation
pub struct LinkMLServiceDependencies<T, E, C> {
    /// Logger service
    pub logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    /// Timestamp service
    pub timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    /// Task manager
    pub task_manager: Arc<T>,
    /// Error handler
    pub error_handler: Arc<E>,
    /// Configuration service
    pub config_service: Arc<C>,
    /// Cache service
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    /// Monitoring service
    pub monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
}

/// # Errors
///
/// Returns an error if service initialization fails
pub async fn create_linkml_service_with_config<T, E, C>(
    config: LinkMLConfig,
    deps: LinkMLServiceDependencies<T, E, C>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
{
    // Create service with custom config
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(Arc::new(service))
}
