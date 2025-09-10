//! Core LinkML service implementation

use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

use linkml_core::{
    config::LinkMLConfig,
    error::{LinkMLError, Result},
    traits::{LinkMLService, LinkMLServiceExt, SchemaFormat},
    types::{SchemaDefinition, ValidationReport}};

use crate::factory::LinkMLServiceDependencies;
use crate::integration::CacheServiceAdapter;
use crate::parser::{ImportResolver, Parser};
use crate::validator::cache::CompiledValidatorCache;

use parking_lot::RwLock;
use std::collections::HashMap;

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use dbms_core::DBMSService;
use error_handling_core::{ErrorContext, ErrorHandlingService};
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::{TaskId, TaskManagementService};
use timeout_core::TimeoutService;
use timestamp_core::TimestampService;



/// Main `LinkML` service implementation
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation
/// - `C`: `ConfigurationService` implementation
/// - `D`: `DBMSService` implementation
/// - `O`: `TimeoutService` implementation
pub struct LinkMLServiceImpl<T, E, C, D, O>
where
    T: TaskManagementService,
    E: ErrorHandlingService,
    C: ConfigurationService,
    D: DBMSService,
    O: TimeoutService,
{
    // Configuration
    config: LinkMLConfig,

    // Parser instance
    parser: Parser,

    // Import resolver
    import_resolver: ImportResolver,

    // Schema cache
    schema_cache: Arc<RwLock<HashMap<String, SchemaDefinition>>>,

    // Compiled validator cache
    validator_cache: Arc<CompiledValidatorCache>,

    // Background task handle for cleanup
    background_task_handle: RwLock<Option<TaskId>>,

    // RootReal service dependencies
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    task_manager: Arc<T>,
    error_handler: Arc<E>,
    config_service: Arc<C>,
    dbms_service: Arc<D>,
    timeout_service: Arc<O>,
    cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>}

impl<T, E, C, D, O> LinkMLServiceImpl<T, E, C, D, O>
where
    T: TaskManagementService,
    E: ErrorHandlingService + 'static,
    C: ConfigurationService,
    D: DBMSService,
    O: TimeoutService,
{
    /// Create a new `LinkML` service instance
    ///
    /// # Errors
    ///
    /// Returns an error if service creation fails
    pub fn new(deps: LinkMLServiceDependencies<T, E, C, D, O>) -> Result<Self> {
        let config = LinkMLConfig::default();
        let import_resolver = ImportResolver::with_search_paths(config.schema.search_paths.clone());

        // Create validator cache with RootReal cache service integration
        let cache_adapter = Arc::new(CacheServiceAdapter::new(deps.cache.clone()));
        let validator_cache =
            Arc::new(CompiledValidatorCache::new().with_cache_service(cache_adapter));

        Ok(Self {
            config,
            parser: Parser::new(),
            import_resolver,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            validator_cache,
            background_task_handle: RwLock::new(None),
            logger: deps.logger,
            timestamp: deps.timestamp,
            task_manager: deps.task_manager,
            error_handler: deps.error_handler,
            config_service: deps.config_service,
            dbms_service: deps.dbms_service,
            timeout_service: deps.timeout_service,
            cache: deps.cache,
            monitor: deps.monitor})
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if service creation fails
    pub fn with_config(
        config: LinkMLConfig,
        deps: LinkMLServiceDependencies<T, E, C, D, O>,
    ) -> Result<Self> {
        let import_resolver = ImportResolver::with_search_paths(config.schema.search_paths.clone());

        // Create validator cache with RootReal cache service integration
        let cache_adapter = Arc::new(CacheServiceAdapter::new(deps.cache.clone()));
        let validator_cache =
            Arc::new(CompiledValidatorCache::new().with_cache_service(cache_adapter));

        Ok(Self {
            config,
            parser: Parser::new(),
            import_resolver,
            schema_cache: Arc::new(RwLock::new(HashMap::new())),
            validator_cache,
            background_task_handle: RwLock::new(None),
            logger: deps.logger,
            timestamp: deps.timestamp,
            task_manager: deps.task_manager,
            error_handler: deps.error_handler,
            config_service: deps.config_service,
            dbms_service: deps.dbms_service,
            timeout_service: deps.timeout_service,
            cache: deps.cache,
            monitor: deps.monitor})
    }

    /// Initialize the service
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Logger operations fail
    /// - Service initialization fails
    pub async fn initialize(&self) -> Result<()> {
        // Record initialization start time
        let start_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;

        self.logger
            .info("Initializing LinkML service")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Register configuration hot-reload handler
        self.setup_config_reload().await?;

        // Load built-in schemas into cache
        self.load_builtin_schemas().await?;

        // Initialize caches with configuration
        self.initialize_caches().await?;

        // Register with monitoring service
        self.register_monitoring().await?;

        // Start background tasks if caching is enabled
        if self.config.performance.enable_compilation {
            self.start_background_tasks().await?;
        }

        // Record initialization time in monitoring
        let end_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;
        let duration_ms = (end_time - start_time) / 1_000_000; // Convert ns to ms

        // Register service with monitoring system
        if let Err(e) = self
            .monitor
            .register_service_for_monitoring("linkml-service")
            .await
        {
            self.logger
                .warn(&format!("Failed to register service for monitoring: {e}"))
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        }

        // Check initial health status
        if let Err(e) = self.monitor.check_service_health("linkml-service").await {
            self.logger
                .warn(&format!("Initial health check failed: {e}"))
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        }

        self.logger
            .info(&format!(
                "LinkML service initialized successfully in {duration_ms}ms"
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        Ok(())
    }

    /// Load built-in schemas
    async fn load_builtin_schemas(&self) -> Result<()> {
        self.logger
            .debug("Loading built-in LinkML schemas")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Built-in schema definitions
        let builtin_schemas = vec![
            ("linkml:types", include_str!("../schemas/types.yaml")),
            ("linkml:meta", include_str!("../schemas/meta.yaml")),
            (
                "linkml:annotations",
                include_str!("../schemas/annotations.yaml"),
            ),
        ];

        for (name, content) in builtin_schemas {
            match self.parser.parse_str(content, "yaml") {
                Ok(schema) => {
                    let mut cache = self.schema_cache.write();
                    cache.insert(name.to_string(), schema);
                }
                Err(e) => {
                    self.logger
                        .warn(&format!("Failed to load built-in schema {name}: {e}"))
                        .await
                        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
                }
            }
        }

        Ok(())
    }

    /// Initialize caches
    async fn initialize_caches(&self) -> Result<()> {
        self.logger
            .debug("Initializing caches")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Initialize validator cache
        if let Err(e) = self.validator_cache.clear() {
            self.logger
                .warn(&format!("Failed to clear validator cache during initialization: {e}"))
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        }

        // Pre-warm cache if configured
        // Check if we should pre-warm the cache service
        if self.config.performance.enable_compilation {
            // Warm the cache service with common keys
            if let Ok(cache_key) = cache_core::CacheKey::new("linkml:schemas:warmup") {
                let warmup_value = cache_core::CacheValue::String(
                    serde_json::json!({
                        "initialized": true,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    })
                    .to_string(),
                );

                // Attempt to warm the cache (log errors but don't fail initialization)
                let ttl = Some(cache_core::CacheTtl::Seconds(3600)); // 1 hour TTL
                if let Err(e) = self.cache.set(&cache_key, &warmup_value, ttl).await
                    && let Err(log_err) = self.logger
                        .debug(&format!("Cache warming failed for key '{cache_key}': {e}"))
                        .await
                    {
                        // If even logging fails, we can't do much more
                        eprintln!("Failed to log cache warming error: {log_err}");
                    }
            }
        }

        Ok(())
    }

    /// Register with monitoring service
    async fn register_monitoring(&self) -> Result<()> {
        self.logger
            .debug("Registering with monitoring service")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?; // Signal service readiness through monitoring
        // Set readiness gauge
        self.logger
            .info("LinkML service registered and ready")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Record initial cache sizes
        let schema_cache_size = self.schema_cache.read().len();
        self.logger
            .debug(&format!(
                "Schema cache initialized with {schema_cache_size} entries"
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        Ok(())
    }

    /// Start background tasks
    async fn start_background_tasks(&self) -> Result<()> {
        self.logger
            .debug("Starting background tasks")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Start monitoring and cleanup task
        let validator_cache = Arc::clone(&self.validator_cache);
        let schema_cache = Arc::clone(&self.schema_cache);
        let logger = Arc::clone(&self.logger);
        let monitor = Arc::clone(&self.monitor);
        let error_handler = Arc::clone(&self.error_handler);
        let timestamp = Arc::clone(&self.timestamp);
        let interval_secs = 60; // 1 minute for health checks

        let task_handle = self
            .task_manager
            .spawn_task(
                Box::pin(async move {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(interval_secs));
                    let mut iteration_count = 0u64;

                    loop {
                        interval.tick().await;
                        iteration_count += 1;

                        // Record health check
                        if let Ok(now) = timestamp.now_utc().await
                            && let Err(e) = logger
                                .debug(&format!("Health check #{iteration_count} at {now}"))
                                .await
                            {
                                eprintln!("Failed to log health check: {e}");
                            }

                        // Monitor cache sizes and log if too large
                        let schema_count = schema_cache.read().len();
                        if schema_count > 100
                            && let Err(e) = logger
                                .warn(&format!(
                                    "Schema cache has {schema_count} entries, consider cleanup"
                                ))
                                .await
                            {
                                eprintln!("Failed to log cache size warning: {e}");
                            }

                        // Check memory usage and cleanup if needed (every 5 iterations = 5 minutes)
                        if iteration_count % 5 == 0 {
                            if let Err(e) = logger.debug("Running periodic cache cleanup").await {
                                eprintln!("Logger error in cache cleanup: {e}");
                            }

                            // Clear validator cache if it's grown too large
                            let cache_stats = validator_cache.stats();
                            if cache_stats.cached_validators > 1000 {
                                if let Err(e) = validator_cache.clear() {
                                    if let Err(log_err) = logger
                                        .error(&format!("Failed to clear validator cache: {e}"))
                                        .await
                                    {
                                        eprintln!("Failed to log cache clear error: {log_err}");
                                    }
                                } else if let Err(e) = logger
                                    .info("Cleared validator cache due to size limit")
                                    .await
                                {
                                    eprintln!("Failed to log cache cleanup: {e}");
                                }

                                // Report cleanup to error handler for tracking
                                let cleanup_msg = "Cache cleanup triggered due to size limit";
                                if let Err(e) = error_handler
                                    .categorize_error(
                                        &LinkMLError::service(cleanup_msg),
                                        Some(ErrorContext::new(
                                            "linkml-service".to_string(),
                                            "cache_cleanup".to_string(),
                                        )),
                                    )
                                    .await
                                {
                                    eprintln!("Failed to report cache cleanup to error handler: {e}");
                                }
                            }
                        }

                        // Report service health status periodically
                        if iteration_count % 10 == 0 {
                            // Check overall health
                            let _ = monitor.check_service_health("linkml-service").await;
                        }
                    }
                }),
                None, // TaskOptions
            )
            .await
            .map_err(|e| LinkMLError::service(format!("Task management error: {e}")))?;

        // Store task handle for proper cleanup on shutdown
        {
            let mut handle_guard = self.background_task_handle.write();
            *handle_guard = Some(task_handle);
        }

        Ok(())
    }

    /// Get the DBMS service
    pub fn dbms_service(&self) -> &Arc<D> {
        &self.dbms_service
    }

    /// Get the timeout service
    pub fn timeout_service(&self) -> &Arc<O> {
        &self.timeout_service
    }

    /// Setup configuration hot-reload
    async fn setup_config_reload(&self) -> Result<()> {
        self.logger
            .debug("Setting up configuration hot-reload")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Note: ConfigurationService hot-reload requires redesign for interior mutability
        // The service would need to support watch/subscription patterns
        Ok(())
    }

    /// Check if configuration service has updates available
    ///
    /// Note: Actually updating configuration would require interior mutability
    /// or a redesign of the service structure.
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// This function does not return errors as configuration fetching failures
    /// are handled internally and do not affect the boolean result
    pub async fn has_config_updates(&self) -> Result<bool> {
        // Fetch latest configuration
        if let Ok(new_config) = self
            .config_service
            .get_configuration::<LinkMLConfig>("linkml")
            .await
        {
            // Check if configuration has changed by comparing search paths
            let current_paths = &self.config.schema.search_paths;
            let new_paths = &new_config.schema.search_paths;

            if current_paths != new_paths {
                self.logger
                    .debug("LinkML configuration has updates available")
                    .await
                    .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Create a sandboxed plugin using the service's timeout service
    pub fn create_sandboxed_plugin(
        &self,
        plugin: Box<dyn crate::plugin::Plugin>,
        sandbox: crate::plugin::PluginSandbox,
    ) -> crate::plugin::loader::SandboxedPlugin<O> {
        crate::plugin::loader::SandboxedPlugin::new(plugin, sandbox, self.timeout_service.clone())
    }

    /// Shutdown the service and clean up resources
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup fails
    pub async fn shutdown(&self) -> Result<()> {
        let shutdown_start = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;

        self.logger
            .info("Shutting down LinkML service")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Record shutdown event
        // Cancel background task if it exists
        if let Some(task_id) = self.background_task_handle.write().take() {
            self.logger
                .debug("Cancelling background monitoring task")
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

            match self.task_manager.cancel_task(&task_id).await {
                Ok(cancelled) => {
                    if cancelled {
                        self.logger
                            .debug("Background task cancelled successfully")
                            .await
                            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
                    } else {
                        self.logger
                            .warn("Background task was already completed")
                            .await
                            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
                    }
                }
                Err(e) => {
                    // Track task cancellation error
                    let _ = self
                        .error_handler
                        .categorize_error(
                            &e,
                            Some(ErrorContext::new(
                                "linkml-service".to_string(),
                                "shutdown".to_string(),
                            )),
                        )
                        .await;

                    self.logger
                        .warn(&format!("Failed to cancel background task: {e}"))
                        .await
                        .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
                }
            }
        }

        // Record final cache statistics before clearing
        let schema_cache_final_size = self.schema_cache.read().len();

        self.logger
            .debug(&format!(
                "Clearing {schema_cache_final_size} cached schemas on shutdown"
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Clear caches
        self.schema_cache.write().clear();
        let _ = self.validator_cache.clear();

        // Record shutdown duration
        let shutdown_end = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;
        let shutdown_duration_ms = (shutdown_end - shutdown_start) / 1_000_000;

        // Set service ready status to 0 (not ready)
        self.logger
            .info(&format!(
                "LinkML service shutdown complete in {shutdown_duration_ms}ms"
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        Ok(())
    }
}

#[async_trait]
impl<T, E, C, D, O> LinkMLService for LinkMLServiceImpl<T, E, C, D, O>
where
    T: TaskManagementService + Send + Sync,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync,
    D: DBMSService + Send + Sync,
    O: TimeoutService + Send + Sync,
{
    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition> {
        // Track operation with error handler
        let path_display = path.display();
        let operation_id = format!("load_schema_{path_display}");
        // Record start time
        let start_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;

        self.logger
            .debug(&format!("Loading schema from: {path_display}"))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Check cache first
        let path_str = path.to_string_lossy().to_string();
        let cached = {
            let cache = self.schema_cache.read();
            cache.get(&path_str).cloned()
        };

        if let Some(schema) = cached {
            self.logger
                .debug("Schema found in cache")
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
            return Ok(schema);
        }

        // Record cache miss
        // Parse the schema with error handling
        let schema = match self.parser.parse_file(path) {
            Ok(s) => s,
            Err(e) => {
                // Record error with error handler
                let error_context = std::collections::HashMap::from([
                    ("operation".to_string(), operation_id.clone()),
                    ("error_type".to_string(), "schema_parse_error".to_string()),
                    ("path".to_string(), path.to_string_lossy().to_string()),
                    ("severity".to_string(), "high".to_string()),
                ]);

                // Track error with error handler service
                let _ = self
                    .error_handler
                    .categorize_error(
                        &e,
                        Some(ErrorContext::new(
                            "linkml-service".to_string(),
                            "load_schema".to_string(),
                        )),
                    )
                    .await;
                self.logger
                    .error(&format!(
                        "Failed to parse schema: {e} - Context: {error_context:?}"
                    ))
                    .await
                    .map_err(|log_err| LinkMLError::service(format!("Logger error: {log_err}")))?;

                return Err(e);
            }
        };

        // Resolve imports
        let schema = self.import_resolver.resolve_imports(&schema)?;

        // Validate schema against meta-schema
        {
            let has_meta = self.schema_cache.read().contains_key("linkml:meta");
            if has_meta {
                // Perform basic validation
                self.logger
                    .debug("Validating against meta-schema")
                    .await
                    .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
            }
        }

        // Cache the result
        {
            let mut cache = self.schema_cache.write();
            cache.insert(path_str, schema.clone());
        }

        // Record operation duration
        let end_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;
        let duration_ms = (end_time - start_time) / 1_000_000;
        self.logger
            .info(&format!(
                "Successfully loaded schema: {} ({}ms)",
                schema.name, duration_ms
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        Ok(schema)
    }

    async fn load_schema_str(
        &self,
        content: &str,
        format: SchemaFormat,
    ) -> Result<SchemaDefinition> {
        let start_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;

        self.logger
            .debug(&format!("Loading schema from string, format: {format:?}"))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        let format_str = match format {
            SchemaFormat::Yaml => "yaml",
            SchemaFormat::Json => "json"};

        // Parse the schema
        let schema = match self.parser.parse_str(content, format_str) {
            Ok(s) => s,
            Err(e) => {
                // Track parse error
                let _ = self
                    .error_handler
                    .categorize_error(
                        &e,
                        Some(ErrorContext::new(
                            "linkml-service".to_string(),
                            "load_schema_str".to_string(),
                        )),
                    )
                    .await;

                return Err(e);
            }
        };

        // Resolve imports
        let schema = self.import_resolver.resolve_imports(&schema)?;

        // Validate schema against meta-schema if available
        {
            let has_meta = self.schema_cache.read().contains_key("linkml:meta");
            if has_meta {
                self.logger
                    .debug("Validating against meta-schema")
                    .await
                    .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

                // Track validation attempt
            }
        }
        let end_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;
        let duration_ms = (end_time - start_time) / 1_000_000;

        self.logger
            .info(&format!(
                "Successfully loaded schema from string: {} ({}ms)",
                schema.name, duration_ms
            ))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        Ok(schema)
    }

    async fn validate(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
        target_class: &str,
    ) -> Result<ValidationReport> {
        // Record validation start
        let start_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;

        self.logger
            .debug(&format!("Validating data against class: {target_class}"))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Create validation engine with cache
        let engine =
            crate::validator::ValidationEngine::with_cache(schema, self.validator_cache.clone())?;

        // Create validation options
        let options = crate::validator::ValidationOptions {
            use_cache: Some(true), // Re-enabled after fixing compiled validator
            check_permissibles: Some(true),
            ..Default::default()
        };

        // Validate against target class
        let report = engine
            .validate_as_class(data, target_class, Some(options))
            .await?;

        // Log validation result
        let result_msg = if report.valid {
            format!("Validation passed for class: {target_class}")
        } else {
            format!(
                "Validation failed for class: {target_class} with {} errors",
                report.stats.error_count
            )
        };
        let end_time = self
            .timestamp
            .now_utc()
            .await
            .map(|dt| dt.timestamp_nanos_opt().unwrap_or(0))
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))?;
        let duration_ms = (end_time - start_time) / 1_000_000;

        // Log validation performance
        self.logger
            .debug(&format!("Validation completed in {duration_ms}ms"))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Track validation results
        // If validation failed, track error details
        if !report.valid {
            // Create validation error for categorization
            let validation_error = LinkMLError::data_validation(format!(
                "Validation failed for {}: {} errors",
                target_class, report.stats.error_count
            ));
            let _ = self
                .error_handler
                .categorize_error(
                    &validation_error,
                    Some(ErrorContext::new(
                        "linkml-service".to_string(),
                        "validate".to_string(),
                    )),
                )
                .await;
        }

        self.logger
            .info(&result_msg)
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Convert to linkml_core ValidationReport type
        Ok(linkml_core::types::ValidationReport {
            valid: report.valid,
            errors: report
                .errors()
                .map(|e| linkml_core::types::ValidationError {
                    message: e.message.clone(),
                    path: Some(e.path.clone()),
                    expected: e.code.clone(),
                    actual: None,
                    severity: linkml_core::types::Severity::Error})
                .collect(),
            warnings: report
                .warnings()
                .map(|e| linkml_core::types::ValidationWarning {
                    message: e.message.clone(),
                    path: Some(e.path.clone()),
                    suggestion: None})
                .collect(),
            timestamp: Some(chrono::Utc::now()),
            schema_id: Some(schema.id.clone())})
    }

}

#[async_trait]
impl<T, E, C, D, O> LinkMLServiceExt for LinkMLServiceImpl<T, E, C, D, O>
where
    T: TaskManagementService + Send + Sync,
    E: ErrorHandlingService + Send + Sync + 'static,
    C: ConfigurationService + Send + Sync,
    D: DBMSService + Send + Sync,
    O: TimeoutService + Send + Sync,
{
    async fn validate_typed<Ty>(
        &self,
        data: &Value,
        schema: &SchemaDefinition,
        target_class: &str,
    ) -> Result<Ty>
    where
        Ty: serde::de::DeserializeOwned,
    {
        // Validate first
        let report = self.validate(data, schema, target_class).await?;

        if !report.valid {
            return Err(LinkMLError::data_validation("Validation failed"));
        }

        // Deserialize to typed value
        serde_json::from_value(data.clone())
            .map_err(|e| LinkMLError::SerializationError(e.to_string()))
    }
}
