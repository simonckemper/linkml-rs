//! Full RootReal service integration example for LinkML
//!
//! This example demonstrates the proper way to integrate LinkML with all 17 RootReal services
//! and register it with the REST API service instead of running as a standalone server.

use std::path::PathBuf;

// Core service imports
use cache_service::wiring::wire_cache;
use configuration_service::wiring::wire_configuration;
use error_handling_service::wiring::wire_error_handling;
use hash_service::wiring::wire_hash;
use logger_service::wiring::wire_logger;
use monitoring_service::wiring::wire_monitoring;
use random_service::wiring::wire_random;
use task_management_service::wiring::wire_task_management;
use telemetry_service::wiring::wire_telemetry;
use timeout_service::wiring::wire_timeout;
use timestamp_service::wiring::wire_timestamp;

// Data services
use dbms_service::wiring::wire_dbms;
use lakehouse_service::wiring::wire_lakehouse;
use vector_database_service::wiring::wire_vector_database;

// Security services
use authentication_service::wiring::wire_authentication;
use rate_limiting_service::wiring::wire_rate_limiting;

// REST API and related services
use frontend_framework_service::cors::{CorsConfig, create_cors_layer};
use restful_api_service::wiring::{ServiceDependencies, wire_restful_api};
use shutdown_service::wiring::wire_shutdown;

// LinkML service
use linkml_service::{
    cli_enhanced::commands::serve::create_linkml_router, wiring::wire_linkml,
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
    let logger = wire_logger().await?.into_arc();
    let config = wire_configuration().await?.into_arc();
    let timestamp = wire_timestamp().into_arc();
    let hash = wire_hash().into_arc();
    let random = wire_random(logger.clone(), timestamp.clone(), None).into_arc();

    // Phase 2: Create infrastructure services
    println!("Phase 2: Creating infrastructure services...");
    let task_manager = wire_task_management(timestamp.clone()).into_arc();
    let error_handler = wire_error_handling(logger.clone(), timestamp.clone(), task_manager.clone()).await?.into_arc();
    let cache = wire_cache(logger.clone(), timestamp.clone(), task_manager.clone(), error_handler.clone(), None).await?.into_arc();

    let monitor = wire_monitoring(logger.clone(), timestamp.clone(), task_manager.clone(), error_handler.clone(), None).await?.into_arc();

    let telemetry =
        wire_telemetry(logger.clone(), timestamp.clone(), task_manager.clone(), None).await?.into_arc();

    // Phase 3: Create timeout service
    println!("Phase 3: Creating timeout service...");
    let timeout = wire_timeout(logger.clone(), timestamp.clone(), task_manager.clone()).await?.into_arc();

    // Phase 4: Create data services
    println!("Phase 4: Creating data services...");
    let dbms = wire_dbms(
        logger.clone(),
        timestamp.clone(),
        task_manager.clone(),
        error_handler.clone(),
        cache.clone(),
        monitor.clone(),
        config.clone(),
    )
    .await?.into_arc();

    let vector_db =
        wire_vector_database(logger.clone(), timestamp.clone(), cache.clone(), config.clone()).await?.into_arc();

    let lakehouse =
        wire_lakehouse(logger.clone(), timestamp.clone(), task_manager.clone(), cache.clone(), dbms.clone(), config.clone())
            .await?.into_arc();

    // Phase 5: Create security services
    println!("Phase 5: Creating security services...");
    let auth =
        wire_authentication(logger.clone(), timestamp.clone(), config.clone(), cache.clone(), hash.clone())
            .await?.into_arc();

    let rate_limiter =
        wire_rate_limiting(logger.clone(), timestamp.clone(), config.clone(), cache.clone()).await?.into_arc();

    // Phase 6: Create LinkML service with all dependencies
    println!("Phase 6: Creating LinkML service...");
    let linkml = wire_linkml(
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
    )
    .await?.into_arc();

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
    let rest_api = wire_restful_api(rest_api_deps).await?.into_arc();

    // Phase 9: Register LinkML handlers with REST API service
    println!("Phase 9: Registering LinkML handlers with REST API...");

    // Load LinkML schema for the example
    let schema_path = PathBuf::from("examples/schema.yaml");
    if !schema_path.exists() {
        println!("Creating example schema file...");
        std::fs::write(
            &schema_path,
            r#"
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
"#,
        )?;
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
    let shutdown =
        wire_shutdown(logger.clone(), timestamp.clone(), task_manager.clone()).await?.into_arc();

    // Register shutdown hooks for all services
    shutdown.register_hook(
        "linkml",
        Box::new(move || {
            Box::pin(async move {
                println!("Shutting down LinkML service...");
                // LinkML cleanup logic here
                Ok(())
            })
        }),
    )?;

    shutdown.register_hook(
        "rest_api",
        Box::new(move || {
            Box::pin(async move {
                println!("Shutting down REST API service...");
                // REST API cleanup logic here
                Ok(())
            })
        }),
    )?;

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
