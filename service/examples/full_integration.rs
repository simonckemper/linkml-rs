//! Full RootReal service integration example for LinkML
//!
//! This example demonstrates the proper way to integrate LinkML with all 17 RootReal services
//! and register it with the REST API service instead of running as a standalone server.

use std::sync::Arc;
use std::path::PathBuf;

// Core service imports
use logger_service::factory::create_logger_service;
use configuration_service::factory::create_configuration_service;
use timestamp_service::factory::create_timestamp_service;
use hash_service::factory::create_hash_service;
use random_service::factory::create_random_service;
use cache_service::factory::create_cache_service;
use monitoring_service::factory::create_monitoring_service;
use telemetry_service::factory::create_telemetry_service;
use error_handling_service::factory::create_error_handling_service;
use timeout_service::factory::create_timeout_service;
use task_management_service::factory::create_task_management_service;

// Data services
use dbms_service::factory::create_dbms_service;
use vector_database_service::factory::create_vector_database_service;
use lakehouse_consumer::factory::create_lakehouse_service;

// Security services
use authentication_service::factory::create_authentication_service;
use rate_limiting_service::factory::create_rate_limiting_service;

// REST API and related services
use restful_api_service::factory_v3::{create_restful_api_service, ServiceDependencies};
use frontend_framework_service::cors::{CorsConfig, create_cors_layer};
use shutdown_service::factory::create_shutdown_service;

// LinkML service
use linkml_service::{
    factory::create_linkml_service,
    cli_enhanced::commands::serve::create_linkml_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== RootReal LinkML Full Service Integration Example ===");
    println!();
    println!("This example demonstrates the proper architectural pattern for integrating");
    println!("LinkML with all 17 RootReal services and the REST API service.");
    println!();

    // Phase 1: Create core services
    println!("Phase 1: Creating core services...");
    let logger = create_logger_service().await?;
    let config = create_configuration_service().await?;
    let timestamp = create_timestamp_service();
    let hash = create_hash_service();
    let random = create_random_service();

    // Phase 2: Create infrastructure services
    println!("Phase 2: Creating infrastructure services...");
    let cache = create_cache_service(
        logger.clone(),
        config.clone(),
        timestamp.clone(),
    ).await?;

    let monitor = create_monitoring_service(
        logger.clone(),
        timestamp.clone(),
    ).await?;

    let telemetry = create_telemetry_service(
        logger.clone(),
        config.clone(),
        timestamp.clone(),
    ).await?;

    // Phase 3: Create task and error handling
    println!("Phase 3: Creating task and error handling services...");
    let task_manager = create_task_management_service(
        logger.clone(),
        monitor.clone(),
    ).await?;

    let error_handler = create_error_handling_service(
        logger.clone(),
        telemetry.clone(),
    ).await?;

    let timeout = create_timeout_service(
        logger.clone(),
        timestamp.clone(),
    ).await?;

    // Phase 4: Create data services
    println!("Phase 4: Creating data services...");
    let dbms = create_dbms_service(
        logger.clone(),
        config.clone(),
        cache.clone(),
        monitor.clone(),
    ).await?;

    let vector_db = create_vector_database_service(
        logger.clone(),
        config.clone(),
        cache.clone(),
    ).await?;

    let lakehouse = create_lakehouse_service(
        logger.clone(),
        config.clone(),
        cache.clone(),
        dbms.clone(),
    ).await?;

    // Phase 5: Create security services
    println!("Phase 5: Creating security services...");
    let auth = create_authentication_service(
        logger.clone(),
        config.clone(),
        cache.clone(),
        hash.clone(),
    ).await?;

    let rate_limiter = create_rate_limiting_service(
        logger.clone(),
        config.clone(),
        cache.clone(),
    ).await?;

    // Phase 6: Create LinkML service with all dependencies
    println!("Phase 6: Creating LinkML service...");
    let linkml = create_linkml_service(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone(),
        error_handler.clone(),
        config.clone(),
        dbms.clone(),
        timeout.clone(),
        cache.clone(),
        monitor.clone(),
        random.clone(),
    ).await?;

    // Phase 7: Create REST API service dependencies
    println!("Phase 7: Preparing REST API service...");
    let rest_api_deps = ServiceDependencies {
        logger: logger.clone(),
        config: config.clone(),
        timestamp: timestamp.clone(),
        hash: hash.clone(),
        rate_limiter: rate_limiter.clone(),
        cache: cache.clone(),
        dbms: dbms.clone(),
        vector_db: vector_db.clone(),
        lakehouse: lakehouse.clone(),
        auth: auth.clone(),
        telemetry: telemetry.clone(),
        monitor: monitor.clone(),
        error_handler: error_handler.clone(),
        task_manager: task_manager.clone(),
        timeout: timeout.clone(),
        random: random.clone(),
    };

    // Phase 8: Create REST API service
    println!("Phase 8: Creating REST API service...");
    let rest_api = create_restful_api_service(rest_api_deps).await?;

    // Phase 9: Register LinkML handlers with REST API service
    println!("Phase 9: Registering LinkML handlers with REST API...");

    // Load LinkML schema for the example
    let schema_path = PathBuf::from("examples/schema.yaml");
    if !schema_path.exists() {
        println!("Creating example schema file...");
        std::fs::write(&schema_path, r#"
id: https://example.org/schema
name: example_schema
description: Example LinkML schema for integration testing

classes:
  Person:
    description: A person with name and age
    attributes:
      name:
        description: Full name of the person
        range: string
        required: true
      age:
        description: Age in years
        range: integer
        minimum_value: 0
        maximum_value: 150
"#)?;
    }

    // Create LinkML router with handlers
    let linkml_router = linkml.create_router(schema_path)?;

    // Register LinkML routes with REST API service
    rest_api.register_router("/api/v1/linkml", linkml_router)?;

    // Phase 10: Configure CORS using frontend-framework service
    println!("Phase 10: Configuring CORS...");
    let cors_config = CorsConfig::production()
        .with_origins(&["https://app.rootreal.com"])
        .with_max_age(7200)
        .with_credentials(true);

    let cors_layer = create_cors_layer(cors_config)?;
    rest_api.add_layer(cors_layer)?;

    // Phase 11: Setup shutdown service
    println!("Phase 11: Setting up graceful shutdown...");
    let shutdown = create_shutdown_service(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone(),
    ).await?;

    // Register shutdown hooks for all services
    shutdown.register_hook("linkml", Box::new(move || {
        Box::pin(async move {
            println!("Shutting down LinkML service...");
            // LinkML cleanup logic here
            Ok(())
        })
    }))?;

    shutdown.register_hook("rest_api", Box::new(move || {
        Box::pin(async move {
            println!("Shutting down REST API service...");
            // REST API cleanup logic here
            Ok(())
        })
    }))?;

    // Phase 12: Start the integrated service
    println!("Phase 12: Starting integrated service...");
    println!();
    println!("LinkML Service is now available through REST API at:");
    println!("  GET  /api/v1/linkml/schema   - Get schema definition");
    println!("  POST /api/v1/linkml/validate - Validate data against schema");
    println!("  GET  /api/v1/linkml/health   - Health check");
    println!();
    println!("This is the CORRECT architectural pattern - LinkML is not a standalone");
    println!("HTTP server but a service integrated into RootReal's REST API service.");
    println!();
    println!("Press Ctrl+C for graceful shutdown...");

    // Start REST API service with all registered handlers
    let addr = "0.0.0.0:8080".parse()?;
    rest_api.serve(addr, shutdown.get_signal()).await?;

    println!("All services shut down gracefully.");
    Ok(())
}