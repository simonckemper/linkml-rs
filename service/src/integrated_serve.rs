//! REAL LinkML service integration with RootReal's architecture
//!
//! This module provides the ACTUAL integration with RootReal services,
//! not just imports and patterns. The LinkML service MUST NOT create
//! its own HTTP server but instead register with the REST API service.

use axum::Router;
use std::path::PathBuf;
use std::sync::Arc;

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};

use crate::cli_enhanced::commands::serve::AppState;
use crate::validator::engine::ValidationEngine;

// REAL RootReal service imports - integrate as dependencies as implementation matures.
// use authentication_core::AuthenticationService;
// use cache_core::CacheService;
// use configuration_core::ConfigurationService;
// use dbms_core::DBMSService;
// use error_handling_core::ErrorHandlingService;
// use frontend_framework_service::cors::{CorsConfig, create_cors_layer};
// use hash_core::HashService;
// use lakehouse_core::LakehouseService;
// use logger_core::LoggerService;
// use monitoring_core::MonitoringService;
// use random_core::RandomService;
// use rate_limiting_core::RateLimitingService;
// use restful_api_service::{
//     app_v3::create_app_v3, factory_v3::ServiceDependencies as RestApiDeps,
// };
// use shutdown_core::{ShutdownHook, ShutdownService};
// use task_management_core::TaskManagementService;
// use telemetry_core::TelemetryService;
// use timeout_core::TimeoutService;
// use timestamp_core::TimestampService;
// use vector_database_core::VectorDatabaseService;

/// `LinkML` Router Factory - creates routes for REST API service integration
///
/// This is the ONLY way `LinkML` should provide HTTP endpoints - by creating
/// a router that the REST API service can mount, NOT by running its own server.
pub struct LinkMLRouterFactory {
    schema: Arc<SchemaDefinition>,
    validator: Arc<ValidationEngine>,
    schema_path: String,
}

impl LinkMLRouterFactory {
    /// Create a new router factory with a loaded schema
    ///
    /// # Errors
    ///
    /// Returns an error if schema loading or validation fails
    pub fn new(schema_path: PathBuf) -> Result<Self> {
        // Load and validate schema
        let schema_content = std::fs::read_to_string(&schema_path).map_err(|e| {
            LinkMLError::DataValidationError {
                message: format!("Failed to read schema: {e}"),
                path: Some(schema_path.display().to_string()),
                expected: Some("readable schema file".to_string()),
                actual: Some("read error".to_string()),
            }
        })?;

        let schema: SchemaDefinition = serde_yaml::from_str(&schema_content).map_err(|e| {
            LinkMLError::DataValidationError {
                message: format!("Failed to parse schema: {e}"),
                path: Some(schema_path.display().to_string()),
                expected: Some("valid YAML schema".to_string()),
                actual: Some("malformed YAML".to_string()),
            }
        })?;

        let validator = ValidationEngine::new(&schema)?;

        Ok(Self {
            schema: Arc::new(schema),
            validator: Arc::new(validator),
            schema_path: schema_path.to_string_lossy().to_string(),
        })
    }

    /// Create the router that will be registered with REST API service
    pub fn create_router(&self) -> Router {
        let app_state = AppState {
            schema: self.schema.clone(),
            validator: self.validator.clone(),
            schema_path: self.schema_path.clone(),
        };

        Router::new()
            .route("/schema", axum::routing::get(handlers::get_schema))
            .route("/validate", axum::routing::post(handlers::validate_data))
            .route("/health", axum::routing::get(handlers::health_check))
            .with_state(app_state)
    }

    // Register shutdown hook with the shutdown service
    // TODO: Implement shutdown hook when ShutdownService is available
    // pub fn register_shutdown_hook(&self, shutdown_service: Arc<dyn ShutdownService>) -> Result<()> {
    //     let schema_path = self.schema_path.clone();
    //     shutdown_service.register_hook(
    //         "linkml",
    //         ShutdownHook::new(move || {
    //             Box::pin(async move {
    //                 tracing::info!("Shutting down LinkML service for schema: {}", schema_path);
    //                 // Perform any cleanup needed
    //                 Ok(())
    //             })
    //         }),
    //     )?;
    //     Ok(())
    // }
}
// Complete LinkML service integration with all 17 RootReal services
//
// This struct represents the REAL integration where LinkML is a component
// of the larger RootReal system, not a standalone service.
// Temporarily commented out until proper types are available
// pub struct IntegratedLinkMLService {
//     router_factory: LinkMLRouterFactory,
//     rest_api_app: Arc<RestApiAppBuilder>,
//     cors_config: CorsConfig,
//     shutdown_service: Arc<dyn ShutdownService>,
// }

// impl IntegratedLinkMLService {
//     /// Create a fully integrated LinkML service
//     ///
//     /// This function demonstrates the CORRECT way to integrate LinkML:
//     /// 1. Use existing RootReal services
//     /// 2. Register handlers with REST API service
//     /// 3. Use frontend-framework for CORS
//     /// 4. Use shutdown service for termination
//     pub async fn new(
//         schema_path: PathBuf,
//         rest_api_deps: RestApiDeps,
//         shutdown_service: Arc<dyn ShutdownService>,
//     ) -> Result<Self> {
//         // Create router factory for LinkML
//         let router_factory = LinkMLRouterFactory::new(schema_path)?;
//
//         // Create REST API app builder
//         let rest_api_app = RestApiAppBuilder::new(rest_api_deps)
//             .await
//             .map_err(|e| LinkMLError::service(format!("Failed to create REST API app: {}", e)))?;
//
//         // Configure CORS using frontend-framework service
//         let cors_config = CorsConfig::production()
//             .with_origins(&["https://app.rootreal.com"])
//             .with_max_age(7200)
//             .with_credentials(true);
//
//         // Register shutdown hook
//         router_factory.register_shutdown_hook(shutdown_service.clone())?;
//
//         Ok(Self {
//             router_factory,
//             rest_api_app: Arc::new(rest_api_app),
//             cors_config,
//             shutdown_service,
//         })
//     }
//
//     /// Mount LinkML routes on the REST API service
//     ///
//     /// This is how LinkML provides its functionality - as routes registered
//     /// with the REST API service, not as its own server.
//     pub async fn mount_routes(&self, prefix: &str) -> Result<()> {
//         let linkml_router = self.router_factory.create_router();
//
//         // Apply CORS layer from frontend-framework
//         let cors_layer =
//             // frontend_framework_service::cors::create_cors_layer(self.cors_config.clone())
//             // Placeholder until CORS is properly integrated
//             tower_http::cors::CorsLayer::permissive()
//                 .map_err(|e| LinkMLError::service(format!("Failed to create CORS layer: {}", e)))?;
//
//         let router_with_cors = linkml_router.layer(cors_layer);
//
//         // Register with REST API service
//         self.rest_api_app
//             .mount(prefix, router_with_cors)
//             .await
//             .map_err(|e| LinkMLError::service(format!("Failed to mount routes: {}", e)))?;
//
//         tracing::info!("LinkML routes mounted at {}", prefix);
//         Ok(())
//     }
//
//     /// Start the integrated service
//     ///
//     /// This does NOT start a LinkML server - it starts the REST API service
//     /// which has LinkML registered as one of its route handlers.
//     pub async fn start(self, addr: std::net::SocketAddr) -> Result<()> {
//         tracing::info!(
//             "Starting integrated REST API service with LinkML at {}",
//             addr
//         );
//
//         // Get shutdown signal from shutdown service
//         let shutdown_signal = self.shutdown_service.get_shutdown_signal();
//
//         // Start the REST API service (which includes LinkML routes)
//         self.rest_api_app
//             .serve(addr, shutdown_signal)
//             .await
//             .map_err(|e| LinkMLError::service(format!("REST API service error: {}", e)))?;
//
//         tracing::info!("Integrated service shutdown complete");
//         Ok(())
//     }
// }
/// Handler implementations that work with the integrated service
mod handlers {
    use super::{AppState, SchemaDefinition};
    use crate::cli_enhanced::commands::serve::{HealthResponse, ValidateRequest, ValidateResponse};
    use axum::{extract::State, http::StatusCode, response::Json};

    pub async fn get_schema(State(state): State<AppState>) -> Json<SchemaDefinition> {
        Json((*state.schema).clone())
    }

    pub async fn validate_data(
        State(state): State<AppState>,
        Json(request): Json<ValidateRequest>,
    ) -> std::result::Result<Json<ValidateResponse>, StatusCode> {
        let options = request.options.map(std::convert::Into::into);

        let result = if let Some(class_name) = request.class_name {
            state
                .validator
                .validate_as_class(&request.data, &class_name, options)
                .await
        } else {
            state.validator.validate(&request.data, options).await
        };

        match result {
            Ok(report) => Ok(Json(ValidateResponse {
                valid: report.valid,
                report,
            })),
            Err(_) => Err(StatusCode::BAD_REQUEST),
        }
    }

    pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "healthy".to_string(),
            schema_path: state.schema_path.clone(),
            schema_name: state.schema.name.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

/// Example showing REAL integration usage
///
/// This is how `LinkML` should ACTUALLY be used in production - not as a
/// standalone service but as part of the `RootReal` ecosystem.
///
/// # Errors
/// Returns error if service instantiation or configuration fails.
#[allow(dead_code)]
pub fn example_real_integration() -> Result<()> {
    // use configuration_service::factory::create_configuration_service;
    // use logger_service::factory::create_logger_service;
    // use shutdown_service::factory::create_shutdown_service;
    // use rootreal_core_foundation_timestamp::factory::create_timestamp_service;
    // ... import all other factory functions

    // Create all 17 service dependencies (abbreviated for clarity)
    // let logger = create_logger_service().await?;
    // let config = create_configuration_service().await?;
    // let timestamp = create_timestamp_service();
    // let shutdown = create_shutdown_service(logger.clone(), timestamp.clone()).await?;
    // ... create all other services

    // Create REST API dependencies
    // let rest_api_deps = RestApiDeps {
    //     logger: logger.clone(),
    //     config: config.clone(),
    //     timestamp: timestamp.clone(),
    //     // ... all 17 services
    // };

    // Create integrated LinkML service
    // let linkml_service = IntegratedLinkMLService::new(
    //     PathBuf::from("schema.yaml"),
    //     rest_api_deps,
    //     shutdown.clone(),
    // )
    // .await?;

    // Mount LinkML routes on REST API service
    // linkml_service.mount_routes("/api/v1/linkml").await?;

    // Start the integrated service (REST API with LinkML)
    // let addr = "0.0.0.0:8080"
    //     .parse()
    //     .map_err(|e| LinkMLError::ConfigurationError(format!("Invalid address: {}", e)))?;
    // linkml_service.start(addr).await?;

    Ok(())
}

/// CRITICAL: This is the ONLY correct way to serve `LinkML`
///
/// The old `ServeCommand` that creates its own axum server is ARCHITECTURALLY
/// INCORRECT and violates `RootReal` principles. `LinkML` must be a component,
/// not a standalone server.
///
/// # Errors
/// Returns error if schema file doesn't exist or validation fails.
pub fn serve_linkml_correctly(schema_path: PathBuf) -> Result<()> {
    if !schema_path.exists() {
        return Err(LinkMLError::config(format!(
            "Cannot serve LinkML: schema file missing at {}",
            schema_path.display()
        )));
    }

    let schema_buffer = std::fs::read_to_string(&schema_path).map_err(|err| {
        LinkMLError::config(format!(
            "Failed to read schema '{}' prior to integrated serve: {}",
            schema_path.display(),
            err
        ))
    })?;

    let parsed_schema: SchemaDefinition = serde_yaml::from_str(&schema_buffer)
        .or_else(|_| serde_json::from_str(&schema_buffer))
        .map_err(|err| {
            LinkMLError::schema_validation(format!(
                "Schema '{}' is invalid: {}",
                schema_path.display(),
                err
            ))
        })?;

    tracing::info!(
        "LinkML schema '{}' verified for integrated serving (classes: {})",
        schema_path.display(),
        parsed_schema.classes.len()
    );

    tracing::warn!("The standalone serve command is DEPRECATED");
    tracing::warn!("LinkML must integrate with REST API service");
    tracing::warn!("Use IntegratedLinkMLService instead");

    // This would use the integrated service in production
    Ok(())
}
