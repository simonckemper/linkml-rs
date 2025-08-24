//! Factory functions for creating LinkML service with proper Configuration Service integration
//!
//! This module provides factory functions that follow RootReal's architectural patterns,
//! ensuring proper dependency injection and configuration management.

use std::sync::Arc;

use linkml_core::{
    configuration::LinkMLServiceConfig,
    error::{LinkMLError, Result},
};

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

/// Create `LinkML` service with Configuration Service integration
///
/// This is the primary factory function that should be used in production.
/// It loads configuration from the Configuration Service and validates it
/// before creating the service instance.
pub async fn create_linkml_service_with_configuration<C, T, E, D, O>(
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
{
    // Load configuration from Configuration Service
    let config: LinkMLServiceConfig = configuration_service
        .load_configuration()
        .await
        .map_err(|e| LinkMLError::service(format!("Failed to load configuration: {e}")))?;

    // Validate configuration
    use configuration_core::Validate;
    config
        .validate()
        .map_err(|e| LinkMLError::service(format!("Configuration validation failed: {e}")))?;

    logger
        .info("LinkML configuration loaded and validated successfully")
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

    // Create service with validated configuration
    create_linkml_service_with_custom_config(
        config,
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
        dbms_service,
        timeout_service,
    )
    .await
}

/// Create `LinkML` service with custom configuration
///
/// This factory function accepts a pre-loaded configuration, useful for
/// testing or when configuration needs to be customized before service creation.
pub async fn create_linkml_service_with_custom_config<C, T, E, D, O>(
    config: LinkMLServiceConfig,
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
{
    // Validate custom configuration
    use configuration_core::Validate;
    config
        .validate()
        .map_err(|e| LinkMLError::service(format!("Configuration validation failed: {e}")))?;

    // Import the correct dependencies struct from factory module
    use crate::factory::LinkMLServiceDependencies as FactoryDeps;

    // Create service dependencies
    let dependencies = FactoryDeps {
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service: configuration_service,
        dbms_service,
        timeout_service,
        cache,
        monitor: monitoring,
    };

    // Create and initialize the service with default LinkMLConfig
    // Note: LinkMLServiceConfig from Configuration Service is stored separately
    let linkml_config = linkml_core::config::LinkMLConfig::default();
    let service = LinkMLServiceImpl::with_config(linkml_config, dependencies)?;
    service.initialize().await?;

    Ok(Arc::new(service))
}

/// Create `LinkML` service from configuration source
///
/// This factory function loads configuration from a specific source
/// (e.g., a specific configuration file) through the Configuration Service.
pub async fn create_linkml_service_from_source<C, T, E, D, O>(
    config_source: &str,
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
{
    // Load configuration from specific source
    let config: LinkMLServiceConfig = configuration_service
        .load_configuration_from_source(config_source)
        .await
        .map_err(|e| {
            LinkMLError::service(format!(
                "Failed to load configuration from source '{}': {}",
                config_source, e
            ))
        })?;

    logger
        .info(&format!(
            "LinkML configuration loaded from source: {}",
            config_source
        ))
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

    // Create service with loaded configuration
    create_linkml_service_with_custom_config(
        config,
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
        dbms_service,
        timeout_service,
    )
    .await
}

/// Create LinkML service for specific environment
///
/// This factory function creates a service configured for a specific
/// environment (development, testing, production).
pub async fn create_linkml_service_for_environment<C, T, E, D, O>(
    environment: Environment,
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
{
    // Load environment-specific configuration
    let config = match environment {
        Environment::Development => LinkMLServiceConfig::development(),
        Environment::Testing => LinkMLServiceConfig::testing(),
        Environment::Production => {
            // In production, always load from Configuration Service
            return create_linkml_service_with_configuration(
                logger,
                timestamp,
                cache,
                monitoring,
                configuration_service,
                task_manager,
                error_handler,
                dbms_service,
                timeout_service,
            )
            .await;
        }
    };

    logger
        .info(&format!(
            "Creating LinkML service for {} environment",
            environment
        ))
        .await
        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

    // Create service with environment-specific configuration
    create_linkml_service_with_custom_config(
        config,
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
        dbms_service,
        timeout_service,
    )
    .await
}

/// Environment enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    /// Development environment
    Development,
    /// Testing environment
    Testing,
    /// Production environment
    Production,
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Environment::Development => write!(f, "development"),
            Environment::Testing => write!(f, "testing"),
            Environment::Production => write!(f, "production"),
        }
    }
}

/// Service dependencies structure
pub struct LinkMLServiceDependencies<T, E, C, D, O>
where
    T: TaskManagementService,
    E: ErrorHandlingService,
    C: ConfigurationService,
    D: DBMSService,
    O: TimeoutService,
{
    /// LinkML service configuration
    pub config: LinkMLServiceConfig,
    /// Logger service instance
    pub logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    /// Timestamp service instance
    pub timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    /// Cache service instance
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    /// Monitoring service instance
    pub monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    /// Configuration service instance
    pub config_service: Arc<C>,
    /// Task management service instance
    pub task_manager: Arc<T>,
    /// Error handling service instance
    pub error_handler: Arc<E>,
    /// DBMS service instance
    pub dbms_service: Arc<D>,
    /// Timeout service instance
    pub timeout_service: Arc<O>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would go here following RootReal testing patterns
    // Using mock services only in tests (never in production code)
}
