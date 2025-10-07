//! # LinkML Service Wiring Module
//!
//! Rust-idiomatic dependency injection wiring for LinkML schema validation and management service.
//! This module provides wiring functions that replace factory patterns with explicit dependency wiring.
//!
//! ## Architecture
//!
//! LinkML service has extensive dependencies across multiple RootReal services:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │              wire_linkml_service()                      │
//! │                      ↓                                  │
//! │          LinkMLServiceImpl<T, E, C, O, R>               │
//! │                                                         │
//! │ Dependencies:                                           │
//! │  - Logger (dyn-compatible)                              │
//! │  - Timestamp (dyn-compatible)                           │
//! │  - TaskManagement (generic T)                           │
//! │  - ErrorHandler (generic E)                             │
//! │  - Configuration (generic C)                            │
//! │  - DBMS (dyn-compatible)                                │
//! │  - Timeout (generic O)                                  │
//! │  - Cache (dyn-compatible)                               │
//! │  - Monitoring (dyn-compatible)                          │
//! │  - Random (generic R)                                   │
//! └─────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────┐
//! │           wire_minimal_linkml_service()                 │
//! │                      ↓                                  │
//! │            MinimalLinkMLServiceImpl                     │
//! │                                                         │
//! │ Minimal Dependencies (CLI usage):                       │
//! │  - Timestamp (dyn-compatible)                           │
//! │  - Random (generic R)                                   │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Benefits
//!
//! 1. **Explicit Dependencies**: All 10 dependencies visible in function signatures
//! 2. **Testability**: Easy to inject test doubles for any dependency
//! 3. **Type Safety**: Compile-time validation of complex dependency relationships
//! 4. **Flexibility**: Minimal variant for CLI, full variant for production
//!
//! ## Migration from Factory Pattern
//!
//! **Before (Factory)**:
//! ```rust,ignore
//! use linkml_service::factory::{create_linkml_service, LinkMLServiceDependencies};
//!
//! let deps = LinkMLServiceDependencies {
//!     logger, timestamp, task_manager, error_handler, config_service,
//!     dbms_service, timeout_service, cache, monitor, random_service,
//! };
//! let service = create_linkml_service(...deps...).await?;
//! ```
//!
//! **After (Wiring)**:
//! ```rust,ignore
//! use linkml_service::wiring::wire_linkml_service;
//!
//! let service = wire_linkml_service(
//!     logger, timestamp, task_manager, error_handler, config_service,
//!     dbms_service, timeout_service, cache, monitor, random_service,
//! ).await?;
//! ```

use crate::handle::LinkMLHandle;
use crate::service::{LinkMLServiceImpl, MinimalLinkMLServiceImpl};
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::ObjectSafeErrorHandler;
use linkml_core::{config::LinkMLConfig, error::Result};
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use random_core::RandomService;
use std::sync::Arc;
use task_management_core::TaskManagementService;
use timeout_core::TimeoutService;
use timestamp_core::{TimestampError, TimestampService};

/// Wire a full-featured LinkML service with all dependencies
///
/// Creates a comprehensive LinkML service for production use with schema validation,
/// DBMS integration, caching, monitoring, and error handling.
///
/// # Arguments
///
/// * `logger` - Logger service for diagnostic logging
/// * `timestamp` - Timestamp service for operation timing
/// * `task_manager` - Task management service for structured concurrency
/// * `error_handler` - Error handling service for recovery strategies
/// * `config_service` - Configuration service for hot-reloadable settings
/// * `dbms_service` - DBMS service for schema persistence
/// * `timeout_service` - Timeout service for operation timeouts
/// * `cache` - Cache service for schema caching
/// * `monitor` - Monitoring service for metrics collection
/// * `random_service` - Random service for non-deterministic operations
///
/// # Returns
///
/// A `LinkMLHandle` wrapping the full-featured LinkML service
///
/// # Errors
///
/// Returns `linkml_core::error::Error` if service initialization fails
///
/// # Type Parameters
///
/// * `T` - Task management service implementation (non-dyn-compatible)
/// * `E` - Error handler implementation (non-dyn-compatible)
/// * `C` - Configuration service implementation (non-dyn-compatible)
/// * `O` - Timeout service implementation (non-dyn-compatible)
/// * `R` - Random service implementation (non-dyn-compatible)
///
/// # Examples
///
/// ```rust,no_run
/// use linkml_service::wiring::wire_linkml_service;
/// use logger_service::wiring::wire_logger;
/// use timestamp_service::wiring::wire_timestamp;
/// use task_management_service::wiring::wire_task_management;
/// use error_handling_service::wiring::wire_error_handling;
/// use configuration_service::wiring::wire_configuration;
/// use dbms_service::wiring::wire_dbms;
/// use timeout_service::wiring::wire_timeout;
/// use cache_service::wiring::wire_cache;
/// use monitoring_service::wiring::wire_monitoring;
/// use random_service::wiring::wire_random;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Wire all dependencies
/// let timestamp = wire_timestamp();
/// let logger = wire_logger(timestamp.clone().into_arc());
/// let task_mgr = wire_task_management(timestamp.clone().into_arc())?;
/// let error_handler = wire_error_handling(logger.clone().into_arc()).await?;
/// let config = wire_configuration().await?;
/// let dbms = wire_dbms(logger.clone().into_arc()).await?;
/// let timeout = wire_timeout();
/// let cache = wire_cache(logger.clone().into_arc()).await?;
/// let monitor = wire_monitoring(logger.clone().into_arc()).await?;
/// let random = wire_random(logger.clone().into_arc())?;
///
/// // Wire LinkML service
/// let linkml = wire_linkml_service(
///     logger.into_arc(),
///     timestamp.into_arc(),
///     task_mgr.into_arc(),
///     error_handler.into_arc(),
///     config.into_arc(),
///     dbms.into_arc(),
///     timeout.into_arc(),
///     cache.into_arc(),
///     monitor.into_arc(),
///     random.into_arc(),
/// ).await?;
///
/// // Use LinkML service
/// let is_valid = linkml.validate_schema("path/to/schema.yaml").await?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub async fn wire_linkml_service<T, E, C, O, R>(
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
) -> Result<LinkMLHandle<LinkMLServiceImpl<T, E, C, O, R>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ObjectSafeErrorHandler + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Load configuration from configuration service
    let service_config = crate::config_helpers::load_and_validate_configuration(&config_service).await?;

    // Convert to core config
    let core_config = crate::config_helpers::convert_service_to_core_config(&service_config);

    // Create dependencies struct
    let deps = crate::factory::LinkMLServiceDependencies {
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

    // Create service
    let service = LinkMLServiceImpl::with_config(core_config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(LinkMLHandle::new(Arc::new(service)))
}

/// Wire a full-featured LinkML service with custom configuration
///
/// Similar to `wire_linkml_service` but accepts a custom LinkML configuration
/// instead of loading it from the configuration service.
///
/// # Arguments
///
/// * `config` - Custom LinkML configuration
/// * `logger` - Logger service for diagnostic logging
/// * `timestamp` - Timestamp service for operation timing
/// * `task_manager` - Task management service for structured concurrency
/// * `error_handler` - Error handling service for recovery strategies
/// * `config_service` - Configuration service for hot-reloadable settings
/// * `dbms_service` - DBMS service for schema persistence
/// * `timeout_service` - Timeout service for operation timeouts
/// * `cache` - Cache service for schema caching
/// * `monitor` - Monitoring service for metrics collection
/// * `random_service` - Random service for non-deterministic operations
///
/// # Returns
///
/// A `LinkMLHandle` wrapping the configured LinkML service
///
/// # Errors
///
/// Returns `linkml_core::error::Error` if service initialization fails
///
/// # Examples
///
/// ```rust,no_run
/// use linkml_service::wiring::wire_linkml_service_with_config;
/// use linkml_core::config::LinkMLConfig;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = LinkMLConfig {
///     schema_dir: "/path/to/schemas".into(),
///     cache_enabled: true,
///     validation_strict: true,
///     ..Default::default()
/// };
///
/// // Wire dependencies...
/// # let logger = todo!();
/// # let timestamp = todo!();
/// # let task_manager = todo!();
/// # let error_handler = todo!();
/// # let config_service = todo!();
/// # let dbms_service = todo!();
/// # let timeout_service = todo!();
/// # let cache = todo!();
/// # let monitor = todo!();
/// # let random_service = todo!();
///
/// let linkml = wire_linkml_service_with_config(
///     config,
///     logger,
///     timestamp,
///     task_manager,
///     error_handler,
///     config_service,
///     dbms_service,
///     timeout_service,
///     cache,
///     monitor,
///     random_service,
/// ).await?;
/// # Ok(())
/// # }
/// ```
#[allow(clippy::too_many_arguments)]
pub async fn wire_linkml_service_with_config<T, E, C, O, R>(
    config: LinkMLConfig,
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
) -> Result<LinkMLHandle<LinkMLServiceImpl<T, E, C, O, R>>>
where
    T: TaskManagementService + Send + Sync + 'static,
    E: ObjectSafeErrorHandler + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync + 'static,
    O: TimeoutService + Send + Sync + 'static,
    R: RandomService + Send + Sync + 'static,
{
    // Create dependencies struct
    let deps = crate::factory::LinkMLServiceDependencies {
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

    // Create service with custom config
    let service = LinkMLServiceImpl::with_config(config, deps)?;

    // Initialize
    service.initialize().await?;

    Ok(LinkMLHandle::new(Arc::new(service)))
}

/// Wire a minimal LinkML service for CLI usage
///
/// Creates a lightweight LinkML service suitable for command-line operations.
/// Only requires timestamp and random services, making it ideal for standalone
/// CLI tools and scripts.
///
/// # Arguments
///
/// * `timestamp` - Timestamp service for operation timing
/// * `random_service` - Random service for non-deterministic operations
///
/// # Returns
///
/// A `LinkMLHandle` wrapping the minimal LinkML service
///
/// # Errors
///
/// Returns `linkml_core::error::Error` if service initialization fails
///
/// # Type Parameters
///
/// * `R` - Random service implementation
///
/// # Examples
///
/// ```rust,no_run
/// use linkml_service::wiring::wire_minimal_linkml_service;
/// use timestamp_service::wiring::wire_timestamp;
/// use random_service::wiring::wire_random;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let timestamp = wire_timestamp();
/// let random = wire_random()?;
///
/// let linkml = wire_minimal_linkml_service(
///     timestamp.into_arc(),
///     random.into_arc(),
/// )?;
///
/// // Use minimal LinkML service for CLI operations
/// let schema_info = linkml.get_schema_info("path/to/schema.yaml")?;
/// # Ok(())
/// # }
/// ```
pub fn wire_minimal_linkml_service<R>(
    _timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    _random_service: Arc<R>,
) -> Result<LinkMLHandle<MinimalLinkMLServiceImpl>>
where
    R: RandomService + Send + Sync + 'static,
{
    let service = MinimalLinkMLServiceImpl::new()?;
    Ok(LinkMLHandle::new(Arc::new(service)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wire_minimal_linkml_service() {
        use random_service::wiring::wire_random;
        use timestamp_service::wiring::wire_timestamp;

        let timestamp = wire_timestamp();
        let random = wire_random().expect("Should create random service");

        let result = wire_minimal_linkml_service(timestamp.into_arc(), random.into_arc());
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_ownership() {
        use random_service::wiring::wire_random;
        use timestamp_service::wiring::wire_timestamp;

        let timestamp = wire_timestamp();
        let random = wire_random().expect("Should create random service");

        let handle = wire_minimal_linkml_service(timestamp.into_arc(), random.into_arc())
            .expect("Should create minimal service");

        // Test that we can extract Arc from handle
        let arc = handle.clone().into_arc();
        assert_eq!(Arc::strong_count(&arc), 2); // handle + arc
    }
}
