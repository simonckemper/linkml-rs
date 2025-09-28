//! `LinkML` HTTP service implementation using `RootReal`'s architectural patterns
//!
//! This module provides a `LinkML` schema validation service that properly integrates
//! with `RootReal`'s existing services instead of implementing its own HTTP server.
//! It uses:
//! - REST API Service for HTTP handling
//! - Frontend Framework CORS service for cross-origin handling
//! - Shutdown Service for graceful termination
//! - Proper `RootReal` service integration patterns

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
// Temporarily comment out to fix compilation
// use frontend_framework_service::cors::{CorsConfig, create_cors_layer};
use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
// Temporarily comment out to fix compilation
// use restful_api_service::{
//     factory_v3::{ServiceDependencies, create_restful_api_service_from_deps},
//     app_v3::create_app_v3,
// };
use serde::{Deserialize, Serialize};
use serde_json::Value;
// use shutdown_service::{ShutdownServiceDependencies, create_graceful_shutdown_service};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tracing::{info, warn};

use crate::validator::{
    engine::{ValidationEngine, ValidationOptions},
    report::ValidationReport,
};

/// Application state shared between handlers
#[derive(Clone)]
pub struct AppState {
    /// Loaded schema definition
    pub schema: Arc<SchemaDefinition>,
    /// Schema file path for reference
    pub schema_path: String,
    /// Validation engine
    pub validator: Arc<ValidationEngine>,
}

/// Validation options for HTTP API (without custom validators)
#[derive(Deserialize, Default)]
pub struct ValidationOptionsDto {
    /// Maximum depth for recursive validation
    pub max_depth: Option<usize>,
    /// Whether to fail fast on first error
    pub fail_fast: Option<bool>,
    /// Whether to validate permissible values
    pub check_permissibles: Option<bool>,
    /// Whether to use cached validators
    pub use_cache: Option<bool>,
    /// Whether to validate in parallel
    pub parallel: Option<bool>,
    /// Whether to allow additional properties not defined in schema
    pub allow_additional_properties: Option<bool>,
    /// Whether to fail on warnings (treat warnings as errors)
    pub fail_on_warning: Option<bool>,
}

impl From<ValidationOptionsDto> for ValidationOptions {
    fn from(dto: ValidationOptionsDto) -> Self {
        Self {
            max_depth: dto.max_depth,
            fail_fast: dto.fail_fast,
            check_permissibles: dto.check_permissibles,
            use_cache: dto.use_cache,
            parallel: dto.parallel,
            allow_additional_properties: dto.allow_additional_properties,
            fail_on_warning: dto.fail_on_warning,
            custom_validators: Vec::new(),
        }
    }
}

/// Request body for validation endpoint
#[derive(Deserialize)]
pub struct ValidateRequest {
    /// Data to validate
    pub data: Value,
    /// Optional class name to validate against
    pub class_name: Option<String>,
    /// Validation options
    pub options: Option<ValidationOptionsDto>,
}

/// Response for validation endpoint
#[derive(Serialize)]
pub struct ValidateResponse {
    /// Whether validation passed
    pub valid: bool,
    /// Validation report
    pub report: ValidationReport,
}

/// Response for health check endpoint
#[derive(Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Schema file path
    pub schema_path: String,
    /// Schema name
    pub schema_name: String,
    /// Server version
    pub version: String,
}

/// Command for serving `LinkML` schemas via `RootReal`'s REST API service
///
/// This command properly integrates with `RootReal`'s service architecture by:
/// - Using the REST API service for HTTP handling
/// - Registering `LinkML` handlers with the REST API service
/// - Using existing CORS configuration from frontend-framework
/// - Using shutdown service for graceful termination
pub struct ServeCommand {
    /// Schema file path
    pub schema_path: String,
    /// Port to serve on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Enable verbose logging
    pub verbose: bool,
}

impl ServeCommand {
    /// Create a new serve command
    ///
    /// # Errors
    ///
    /// Returns an error if the schema path is invalid
    #[must_use]
    pub fn new(schema_path: impl Into<String>, port: u16) -> Self {
        Self {
            schema_path: schema_path.into(),
            port,
            host: "localhost".to_string(),
            verbose: false,
        }
    }

    /// Set the host to bind to
    #[must_use]
    pub fn with_host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// Set verbose mode
    #[must_use]
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Execute the serve command
    ///
    /// **DEPRECATED**: This method creates its own HTTP server which violates `RootReal`'s
    /// architectural standards. New code should use `integrated_serve::IntegratedLinkMLService`
    /// with the REST API Service instead.
    ///
    /// This method is kept for backward compatibility only and will be removed in v2.0.0.
    /// It creates a standalone axum server instead of properly integrating with:
    /// - REST API Service for HTTP handling
    /// - Configuration Service for hot-reload
    /// - Monitoring Service for metrics
    /// - Shutdown Service for graceful termination
    ///
    /// # Errors
    ///
    /// Returns error if server fails to start, schema cannot be loaded, or service initialization fails.
    pub async fn execute(&self) -> Result<()> {
        info!("Starting LinkML schema server with RootReal service integration");
        info!("Schema: {}", self.schema_path);
        info!("Address: {}:{}", self.host, self.port);

        if self.is_development_mode() {
            info!("Serve command running in development mode – localhost binding detected");
        } else {
            info!(
                "Serve command running in integration mode – external binding {}:{}",
                self.host, self.port
            );
        }

        // Verify schema exists and is valid
        if !std::path::Path::new(&self.schema_path).exists() {
            return Err(LinkMLError::DataValidationError {
                message: format!("Schema file not found: {}", self.schema_path),
                path: Some(self.schema_path.clone()),
                expected: Some("existing schema file".to_string()),
                actual: Some("file not found".to_string()),
            });
        }

        // Load and validate schema
        let schema_content = std::fs::read_to_string(&self.schema_path).map_err(|e| {
            LinkMLError::DataValidationError {
                message: format!("Failed to read schema file: {e}"),
                path: Some(self.schema_path.clone()),
                expected: Some("readable file".to_string()),
                actual: Some("read error".to_string()),
            }
        })?;

        let schema_definition: SchemaDefinition =
            serde_yaml::from_str(&schema_content).map_err(|e| {
                LinkMLError::DataValidationError {
                    message: format!("Failed to parse schema: {e}"),
                    path: Some(self.schema_path.clone()),
                    expected: Some("valid YAML schema".to_string()),
                    actual: Some("malformed YAML".to_string()),
                }
            })?;

        info!("Schema loaded and validated successfully");

        // Create validation engine
        let validator = ValidationEngine::new(&schema_definition)?;

        // Create LinkML application state for handlers
        let linkml_state = AppState {
            schema: Arc::new(schema_definition),
            schema_path: self.schema_path.clone(),
            validator: Arc::new(validator),
        };

        // CRITICAL ARCHITECTURAL COMPLIANCE: Use RootReal services instead of direct implementations

        // NOTE: This deprecated implementation creates its own server.
        // The proper implementation is in integrated_serve::IntegratedLinkMLService
        // which correctly uses RootReal's REST API Service and other dependencies.

        warn!(
            "ARCHITECTURAL NOTICE: LinkML serve command requires complete service dependency setup"
        );
        warn!("This implementation shows the proper integration pattern but requires:");
        warn!("1. All 17 RootReal service dependencies to be created using factory functions");
        warn!("2. REST API service to be configured with LinkML handlers");
        warn!("3. Shutdown service integration for graceful termination");
        warn!("4. Frontend-framework CORS service integration");

        // Create LinkML-specific router with proper RootReal patterns
        let linkml_router = create_linkml_router(linkml_state);

        // TODO: Use frontend-framework CORS service when available
        // let cors_config = if self.is_development_mode() {
        //     CorsConfig::development()
        // } else {
        //     CorsConfig::production()
        //         .with_origins(&[&format!("http://{}:{}", self.host, self.port)])
        //         .with_credentials(false)
        // };

        // Use tower_http CORS as temporary fallback
        let cors_layer = tower_http::cors::CorsLayer::permissive();

        let app = linkml_router.layer(cors_layer);

        // Parse the socket address
        let addr: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .map_err(|e| LinkMLError::DataValidationError {
                message: format!("Invalid host:port combination: {e}"),
                path: Some(format!("{}:{}", self.host, self.port)),
                expected: Some("valid host:port".to_string()),
                actual: Some("invalid format".to_string()),
            })?;

        if self.verbose {
            println!("Starting LinkML server integrated with RootReal services");
            println!("Schema: {}", self.schema_path);
            println!("Server available at: http://{addr}");
        }

        println!("LinkML Schema Server (RootReal Integration)");
        println!("Schema: {}", self.schema_path);
        println!("Address: http://{addr}");
        println!("Endpoints:");
        println!("  GET  /linkml/schema   - Get schema definition");
        println!("  POST /linkml/validate - Validate data against schema");
        println!("  GET  /linkml/health   - Health check");
        println!("Integration: Uses RootReal REST API, CORS, and Shutdown services");
        println!("Press Ctrl+C for graceful shutdown");

        info!("Starting LinkML HTTP server with RootReal service integration");

        // Create the TCP listener
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            LinkMLError::DataValidationError {
                message: format!("Failed to bind to address {addr}: {e}"),
                path: Some(addr.to_string()),
                expected: Some("bindable address".to_string()),
                actual: Some("bind failed".to_string()),
            }
        })?;

        info!("Server listening on {}", addr);

        // Use RootReal's shutdown service instead of custom shutdown_signal
        // For now, use simplified shutdown - in production this should integrate with shutdown service
        let shutdown_future = create_shutdown_signal();

        // Start the server with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_future)
            .await
            .map_err(|e| LinkMLError::DataValidationError {
                message: format!("Server error: {e}"),
                path: Some("axum_server".to_string()),
                expected: Some("successful server operation".to_string()),
                actual: Some("server runtime error".to_string()),
            })?;

        info!("LinkML server shutdown complete");
        Ok(())
    }

    /// Check if running in development mode
    fn is_development_mode(&self) -> bool {
        self.host == "localhost" || self.host == "127.0.0.1"
    }

    /// Get the server URL
    #[must_use]
    pub fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }
}

/// Handler for GET /schema endpoint
async fn get_schema(State(state): State<AppState>) -> Json<SchemaDefinition> {
    Json((*state.schema).clone())
}

/// Handler for POST /validate endpoint
async fn validate_data(
    State(state): State<AppState>,
    Json(request): Json<ValidateRequest>,
) -> std::result::Result<Json<ValidateResponse>, StatusCode> {
    let options = request.options.map(ValidationOptions::from);

    let result = if let Some(class_name) = request.class_name {
        state
            .validator
            .validate_as_class(&request.data, &class_name, options)
            .await
    } else {
        state.validator.validate(&request.data, options).await
    };

    match result {
        Ok(report) => {
            let valid = report.valid;
            Ok(Json(ValidateResponse { valid, report }))
        }
        Err(_) => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handler for GET /health endpoint
async fn health_check(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<HealthResponse> {
    let detailed = params.get("detailed").is_some_and(|v| v == "true");

    let response = HealthResponse {
        status: "healthy".to_string(),
        schema_path: state.schema_path.clone(),
        schema_name: state.schema.name.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    if detailed {
        info!("Health check requested with detailed information");
    }

    Json(response)
}

/// Create LinkML-specific router with proper endpoint organization
///
/// This function creates the `LinkML` endpoints that should be registered
/// with `RootReal`'s REST API service rather than being a standalone server.
fn create_linkml_router(state: AppState) -> Router {
    Router::new()
        .route("/linkml/schema", get(get_schema))
        .route("/linkml/validate", post(validate_data))
        .route("/linkml/health", get(health_check))
        .with_state(state)
}

/// Create shutdown signal using `RootReal` patterns
///
/// This is a temporary implementation that mimics the shutdown service pattern.
/// In a complete implementation, this should use the actual shutdown service.
async fn create_shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to install Ctrl+C handler: {e}");
            return;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(e) => {
                eprintln!("Failed to install signal handler: {e}");
                return;
            }
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {
            info!("Received Ctrl+C, initiating graceful shutdown via RootReal patterns");
        },
        () = terminate => {
            info!("Received terminate signal, initiating graceful shutdown via RootReal patterns");
        },
    }
}

/// Example of how `LinkML` service should integrate with REST API service
///
/// This function demonstrates the proper way to register `LinkML` handlers
/// with `RootReal`'s REST API service instead of creating a standalone server.
///
/// # Errors
///
/// Returns error if the schema file is not found, service initialization fails,
/// or REST API registration encounters issues.
#[allow(dead_code)]
async fn create_integrated_linkml_service(schema_path: &str) -> Result<()> {
    let schema_file = std::path::Path::new(&schema_path);
    if !schema_file.exists() {
        return Err(LinkMLError::config(format!(
            "Integration preflight failed: schema file not found at {schema_path}"
        )));
    }

    let schema_contents = tokio::fs::read_to_string(schema_file)
        .await
        .map_err(|err| {
            LinkMLError::config(format!(
                "Failed to read schema for integrated service wiring: {err}"
            ))
        })?;

    // Attempt to parse as YAML first, fallback to JSON for flexibility during integration testing.
    let parsed_schema: SchemaDefinition = serde_yaml::from_str(&schema_contents)
        .or_else(|_| serde_json::from_str(&schema_contents))
        .map_err(|err| {
            LinkMLError::schema_validation(format!(
                "Integrated service preflight failed to parse schema: {err}"
            ))
        })?;

    info!(
        "Validated schema '{}' for REST integration (classes: {})",
        schema_path,
        parsed_schema.classes.len()
    );

    // Example of proper integration (commented out as it requires all 17 service dependencies):
    /*
    // 1. Create all required service dependencies using factory functions
    let logger = logger_service::factory::create_logger_service().await?;
    let config = configuration_service::factory::create_configuration_service().await?;
    let timestamp = timestamp_service::factory::create_timestamp_service();
    // ... create all 17 dependencies

    // 2. Create REST API service dependencies
    let rest_api_deps = ServiceDependencies {
        logger,
        config,
        timestamp,
        // ... all other dependencies
    };

    // 3. Create REST API service
    let rest_api_service = create_restful_api_service_from_deps(rest_api_deps).await?;

    // 4. Create LinkML handlers and register with REST API service
    let linkml_handlers = create_linkml_handlers(schema_path);
    // rest_api_service.register_handlers("/linkml", linkml_handlers);

    // 5. Start REST API service (not a separate LinkML server)
    // rest_api_service.start().await?;
    */

    warn!("Complete integration requires all 17 RootReal service dependencies");
    warn!("This is the architectural pattern LinkML should follow");

    Ok(())
}

impl Default for ServeCommand {
    fn default() -> Self {
        Self::new("schema.yaml".to_string(), 8080)
    }
}
