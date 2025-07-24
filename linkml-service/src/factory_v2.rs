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
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::TaskManagementService;
use timestamp_core::TimestampService;

/// Create LinkML service with Configuration Service integration
///
/// This is the primary factory function that should be used in production.
/// It loads configuration from the Configuration Service and validates it
/// before creating the service instance.
pub async fn create_linkml_service_with_configuration<C, T, E>(
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
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
    ).await
}

/// Create LinkML service with custom configuration
///
/// This factory function accepts a pre-loaded configuration, useful for
/// testing or when configuration needs to be customized before service creation.
pub async fn create_linkml_service_with_custom_config<C, T, E>(
    config: LinkMLServiceConfig,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
{
    // Validate custom configuration
    use configuration_core::Validate;
    config.validate()
        .map_err(|e| LinkMLError::configuration(format!("Configuration validation failed: {e}")))?;
    
    // Create service dependencies with configuration
    let dependencies = LinkMLServiceDependencies {
        config: config.clone(),
        logger,
        timestamp,
        cache,
        monitoring,
        configuration_service,
        task_manager,
        error_handler,
    };
    
    // Create and initialize the service
    let service = LinkMLServiceImpl::new(dependencies)?;
    service.initialize().await?;
    
    Ok(Arc::new(service))
}

/// Create LinkML service from configuration source
///
/// This factory function loads configuration from a specific source
/// (e.g., a specific configuration file) through the Configuration Service.
pub async fn create_linkml_service_from_source<C, T, E>(
    config_source: &str,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
{
    // Load configuration from specific source
    let config: LinkMLServiceConfig = configuration_service
        .load_configuration_from_source(config_source)
        .await
        .map_err(|e| LinkMLError::configuration(
            format!("Failed to load configuration from source '{}': {}", config_source, e)
        ))?;
    
    logger
        .info(&format!("LinkML configuration loaded from source: {}", config_source))
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
    ).await
}

/// Create LinkML service for specific environment
///
/// This factory function creates a service configured for a specific
/// environment (development, testing, production).
pub async fn create_linkml_service_for_environment<C, T, E>(
    environment: Environment,
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    configuration_service: Arc<C>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
) -> Result<Arc<LinkMLServiceImpl<T, E, C>>>
where
    C: ConfigurationService + Send + Sync + 'static,
    T: TaskManagementService + Send + Sync + 'static,
    E: ErrorHandlingService + Send + Sync + 'static,
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
            ).await;
        }
    };
    
    logger
        .info(&format!("Creating LinkML service for {} environment", environment))
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
    ).await
}

/// Environment enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Testing,
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
pub struct LinkMLServiceDependencies<C, T, E>
where
    C: ConfigurationService,
    T: TaskManagementService,
    E: ErrorHandlingService,
{
    pub config: LinkMLServiceConfig,
    pub logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    pub timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    pub cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    pub monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    pub configuration_service: Arc<C>,
    pub task_manager: Arc<T>,
    pub error_handler: Arc<E>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Tests would go here following RootReal testing patterns
    // Using mock services only in tests (never in production code)
}