//! Common service initialization for examples
//!
//! Provides real service initialization for LinkML examples
//! using actual service factories instead of mocks.

use linkml_core::config::LinkMLConfig;
use linkml_service::factory::LinkMLServiceDependencies;
use linkml_service::service::LinkMLServiceImpl;
use std::sync::Arc;

// Import real service factories
use logger_service::factory::create_development_logger;
use timestamp_service::factory::create_timestamp_service;
use task_management_service::factory::create_task_management_service;
use configuration_service::factory::create_configuration_service;
use error_handling_service::factory::create_error_handling_service;
use cache_service::factory::create_cache_service;
use monitoring_service::factory::create_monitoring_service;
use timeout_service::factory::create_timeout_service;
use dbms_service::factory::create_dbms_service;

/// Initialize LinkML service with real dependencies
pub async fn init_linkml_service_with_real_deps(
) -> Result<Arc<LinkMLServiceImpl>, Box<dyn std::error::Error>> {
    // Create all real services
    let logger = Arc::new(create_development_logger().await?);
    let timestamp = create_timestamp_service();
    let task_manager = Arc::new(create_task_management_service()?);
    let config_service = create_configuration_service(task_manager.clone());
    let error_handler = create_error_handling_service(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone()
    )?;
    let cache = create_cache_service(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone(),
        error_handler.clone(),
        None
    ).await?;
    let monitoring = create_monitoring_service(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone()
    ).await?;
    let timeout = create_timeout_service(task_manager.clone());
    
    // Create DBMS service (TypeDB)
    let dbms = create_dbms_service(
        logger.clone(),
        config_service.clone(),
        error_handler.clone(),
        monitoring.clone(),
    ).await?;

    // Create LinkML config
    let config = LinkMLConfig::default();

    // Create dependencies
    let deps = LinkMLServiceDependencies {
        logger,
        timestamp,
        config_service,
        error_handler,
        task_manager,
        cache,
        monitoring,
        timeout,
        dbms,
    };

    // Initialize LinkML service
    let service = Arc::new(LinkMLServiceImpl::new(deps, config).await?);

    Ok(service)
}