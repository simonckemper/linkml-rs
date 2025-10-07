//! Factory functions for creating LinkML service instances

use std::sync::Arc;

use crate::config_helpers::load_and_validate_configuration;
use crate::service::LinkMLServiceImpl;
use linkml_core::{config::LinkMLConfig, error::Result};

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::ObjectSafeErrorHandler;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use random_core::RandomService;
use task_management_core::TaskManagementService;
use timeout_core::TimeoutService;
use timestamp_core::{TimestampError, TimestampService};

/// Create a new `LinkML` service instance with all dependencies
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
/// - `D`: `DBMSService` implementation
/// - `O`: `TimeoutService` implementation
/// - `R`: `RandomService` implementation
///
/// # Errors
///
/// Returns an error if service initialization fails
#[allow(clippy::too_many_arguments)]
pub async fn create_linkml_service<T, E, C, O, R>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    config_service: Arc<C>,
    dbms_service: Arc<dyn DBMSService<Error = dbms_core::DBMSError>>,
    timeout_service: Arc<O>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    random_service: Arc<R>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, O, R>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ObjectSafeErrorHandler + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Load configuration from configuration service and convert to core config
    let service_config = load_and_validate_configuration(&config_service).await?;

    // Convert to core config
    let core_config = crate::config_helpers::convert_service_to_core_config(&service_config);

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
        random_service,
    };
    let service = LinkMLServiceImpl::with_config(core_config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(Arc::new(service))
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
/// let service = create_minimal_linkml_service<R>();
///
/// // New pattern (recommended):
/// use _service::wiring::wire_;
/// let service = wire_()?.into_arc();
/// ```
#[deprecated(
    since = "0.2.0",
    note = "Use \`_service::wiring::wire_()\` instead. \\
            See docs/guides/facade-migration-guide.md for migration instructions."
)]
/// Create a minimal `LinkML` service for CLI usage with limited dependencies.
///
/// This creates a basic `LinkML` service suitable for CLI operations that only
/// requires timestamp and random services.
///
/// # Errors
///
/// Returns an error if service initialization fails
pub fn create_minimal_linkml_service<R>(
    _timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    _random_service: Arc<R>,
) -> Result<Arc<crate::service::MinimalLinkMLServiceImpl>>
where
    R: RandomService + Send + Sync + 'static,
{
    use crate::service::MinimalLinkMLServiceImpl;

    // Create minimal service that only uses available dependencies
    let service = MinimalLinkMLServiceImpl::new()?;
    Ok(Arc::new(service))
}

/// Service dependencies for `LinkML` service creation
///
/// This structure groups all service dependencies required to create a `LinkML` service instance.
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
/// - `O`: `TimeoutService` implementation
/// - `R`: `RandomService` implementation
///
/// `DBMSService` is dyn-compatible and uses `Arc<dyn DBMSService>` pattern
pub struct LinkMLServiceDependencies<T, E, C, O, R> {
    /// Logger service
    pub logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    /// Timestamp service
    pub timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    /// Task manager
    pub task_manager: Arc<T>,
    /// Error handler
    pub error_handler: Arc<E>,
    /// Configuration service
    pub config_service: Arc<C>,
    /// DBMS service
    pub dbms_service: Arc<dyn DBMSService<Error = dbms_core::DBMSError>>,
    /// Timeout service
    pub timeout_service: Arc<O>,
    /// Cache service
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    /// Monitoring service
    pub monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    /// Random service
    pub random_service: Arc<R>,
}

/// Create a `LinkML` service with custom configuration
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
/// - `D`: `DBMSService` implementation
/// - `O`: `TimeoutService` implementation
/// - `R`: `RandomService` implementation
///
/// # Parameters
///
/// * `config` - Custom `LinkML` configuration
/// * `deps` - Service dependencies struct
///
/// # Errors
///
/// Returns an error if service initialization fails
pub async fn create_linkml_service_with_config<T, E, C, O, R>(
    config: LinkMLConfig,
    deps: LinkMLServiceDependencies<T, E, C, O, R>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C, O, R>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ObjectSafeErrorHandler + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Create service with custom config
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(Arc::new(service))
}
