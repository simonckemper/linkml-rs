//! Factory functions for creating LinkML service instances

use std::sync::Arc;

use crate::service::LinkMLServiceImpl;
use linkml_core::{config::LinkMLConfig, error::Result};

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

/// Create a new `LinkML` service instance with all dependencies
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
/// - `D`: `DBMSService` implementation
/// - `O`: `TimeoutService` implementation
///
/// # Errors
///
/// Returns an error if service initialization fails
#[allow(clippy::too_many_arguments)]
pub async fn create_linkml_service<T, E, C, D, O>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    config_service: Arc<C>,
    dbms_service: Arc<D>,
    timeout_service: Arc<O>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, D, O>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
{
    // Create service
    let deps = LinkMLServiceDependencies {
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        dbms_service,
        timeout_service,
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
pub struct LinkMLServiceDependencies<T, E, C, D, O> {
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
    /// DBMS service
    pub dbms_service: Arc<D>,
    /// Timeout service
    pub timeout_service: Arc<O>,
    /// Cache service
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    /// Monitoring service
    pub monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
}

/// # Errors
///
/// Returns an error if service initialization fails
pub async fn create_linkml_service_with_config<T, E, C, D, O>(
    config: LinkMLConfig,
    deps: LinkMLServiceDependencies<T, E, C, D, O>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, D, O>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    D: DBMSService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
{
    // Create service with custom config
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(Arc::new(service))
}
