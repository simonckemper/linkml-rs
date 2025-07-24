//! Core LinkML service implementation

use async_trait::async_trait;
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

use linkml_core::{
    config::LinkMLConfig,
    error::{LinkMLError, Result},
    traits::{LinkMLService, SchemaFormat},
    types::{SchemaDefinition, ValidationReport},
};

use crate::factory::LinkMLServiceDependencies;
use crate::integration::CacheServiceAdapter;
use crate::parser::{ImportResolver, Parser};
use crate::validator::cache::CompiledValidatorCache;

use parking_lot::RwLock;
use std::collections::HashMap;

// RootReal service dependencies
use cache_core::CacheService;
use configuration_core::ConfigurationService;
use error_handling_core::ErrorHandlingService;
use logger_core::LoggerService;
use monitoring_core::MonitoringService;
use task_management_core::TaskManagementService;
use timestamp_core::TimestampService;

/// Main `LinkML` service implementation
///
/// Generic parameters for non-dyn-compatible services:
/// - `T`: `TaskManagementService` implementation
/// - `E`: `ErrorHandlingService` implementation  
/// - `C`: `ConfigurationService` implementation
pub struct LinkMLServiceImpl<T, E, C>
where
    T: TaskManagementService,
    E: ErrorHandlingService,
    C: ConfigurationService,
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

    // RootReal service dependencies
    logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>>,
    _timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    _task_manager: Arc<T>,
    _error_handler: Arc<E>,
    _config_service: Arc<C>,
    _cache: Arc<dyn CacheService<Error = cache_core::CacheError>>,
    _monitor: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
}

impl<T, E, C> LinkMLServiceImpl<T, E, C>
where
    T: TaskManagementService,
    E: ErrorHandlingService,
    C: ConfigurationService,
{
    /// Create a new `LinkML` service instance
    ///
    /// # Errors
    ///
    /// Returns an error if service creation fails
    pub fn new(deps: LinkMLServiceDependencies<T, E, C>) -> Result<Self> {
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
            logger: deps.logger,
            _timestamp: deps.timestamp,
            _task_manager: deps.task_manager,
            _error_handler: deps.error_handler,
            _config_service: deps.config_service,
            _cache: deps.cache,
            _monitor: deps.monitor,
        })
    }

    /// Create with custom configuration
    ///
    /// # Errors
    ///
    /// Returns an error if service creation fails
    pub fn with_config(
        config: LinkMLConfig,
        deps: LinkMLServiceDependencies<T, E, C>,
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
            logger: deps.logger,
            _timestamp: deps.timestamp,
            _task_manager: deps.task_manager,
            _error_handler: deps.error_handler,
            _config_service: deps.config_service,
            _cache: deps.cache,
            _monitor: deps.monitor,
        })
    }

    /// Initialize the service
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if:
    /// - Logger operations fail
    /// - Service initialization fails
    pub async fn initialize(&self) -> Result<()> {
        self.logger
            .info("Initializing LinkML service")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        // Load built-in schemas into cache
        self.load_builtin_schemas().await?;
        
        // Initialize caches with configuration
        self.initialize_caches().await?;
        
        // Register with monitoring service
        self.register_monitoring().await?;
        
        // Start background tasks if configured
        if self.config.performance.enable_background_tasks {
            self.start_background_tasks().await?;
        }

        self.logger
            .info("LinkML service initialized successfully")
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
            ("linkml:annotations", include_str!("../schemas/annotations.yaml")),
        ];
        
        for (name, content) in builtin_schemas {
            match self.parser.parse_str(content) {
                Ok(schema) => {
                    let mut cache = self.schema_cache.write();
                    cache.insert(name.to_string(), schema);
                }
                Err(e) => {
                    self.logger
                        .warn(&format!("Failed to load built-in schema {}: {}", name, e))
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
        self.validator_cache.clear().await;
        
        // Pre-warm cache if configured
        if self.config.performance.enable_cache_warming {
            self.logger
                .debug("Pre-warming validator cache")
                .await
                .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
            // Cache warming would happen here
        }
        
        Ok(())
    }
    
    /// Register with monitoring service
    async fn register_monitoring(&self) -> Result<()> {
        self.logger
            .debug("Registering with monitoring service")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        
        // Register metrics
        self._monitor
            .register_counter("linkml.schemas_loaded", "Number of schemas loaded")
            .await
            .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;
        
        self._monitor
            .register_counter("linkml.validations_performed", "Number of validations performed")
            .await
            .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;
        
        self._monitor
            .register_histogram("linkml.validation_duration_ms", "Validation duration in milliseconds")
            .await
            .map_err(|e| LinkMLError::service(format!("Monitoring error: {e}")))?;
        
        Ok(())
    }
    
    /// Start background tasks
    async fn start_background_tasks(&self) -> Result<()> {
        self.logger
            .debug("Starting background tasks")
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;
        
        // Start cache cleanup task
        let validator_cache = self.validator_cache.clone();
        let logger = self.logger.clone();
        let task_handle = self._task_manager
            .spawn_periodic(
                "linkml_cache_cleanup",
                std::time::Duration::from_secs(self.config.performance.background_task_interval_secs),
                move || {
                    let cache = validator_cache.clone();
                    let log = logger.clone();
                    Box::pin(async move {
                        if let Err(e) = log.debug("Running cache cleanup").await {
                            eprintln!("Logger error in cache cleanup: {}", e);
                        }
                        cache.cleanup_expired().await;
                    })
                },
            )
            .await
            .map_err(|e| LinkMLError::service(format!("Task management error: {e}")))?;
        
        // Store task handle for cleanup later if needed
        let _ = task_handle;
        
        Ok(())
    }
}

#[async_trait]
impl<T, E, C> LinkMLService for LinkMLServiceImpl<T, E, C>
where
    T: TaskManagementService + Send + Sync,
    E: ErrorHandlingService + Send + Sync,
    C: ConfigurationService + Send + Sync,
{
    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition> {
        self.logger
            .debug(&format!("Loading schema from: {}", path.display()))
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

        // Parse the schema
        let schema = self.parser.parse_file(path)?;

        // Resolve imports
        let schema = self.import_resolver.resolve_imports(&schema)?;

        // TODO: Validate schema against meta-schema

        // Cache the result
        {
            let mut cache = self.schema_cache.write();
            cache.insert(path_str, schema.clone());
        }

        self.logger
            .info(&format!("Successfully loaded schema: {}", schema.name))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        Ok(schema)
    }

    async fn load_schema_str(
        &self,
        content: &str,
        format: SchemaFormat,
    ) -> Result<SchemaDefinition> {
        self.logger
            .debug(&format!("Loading schema from string, format: {format:?}"))
            .await
            .map_err(|e| LinkMLError::service(format!("Logger error: {e}")))?;

        let format_str = match format {
            SchemaFormat::Yaml => "yaml",
            SchemaFormat::Json => "json",
        };

        // Parse the schema
        let schema = self.parser.parse_str(content, format_str)?;

        // Resolve imports
        let schema = self.import_resolver.resolve_imports(&schema)?;

        // TODO: Validate schema against meta-schema

        self.logger
            .info(&format!(
                "Successfully loaded schema from string: {}",
                schema.name
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
                    severity: linkml_core::types::Severity::Error,
                })
                .collect(),
            warnings: report
                .warnings()
                .map(|e| linkml_core::types::ValidationWarning {
                    message: e.message.clone(),
                    path: Some(e.path.clone()),
                    suggestion: None,
                })
                .collect(),
            timestamp: Some(chrono::Utc::now()),
            schema_id: Some(schema.id.clone()),
        })
    }

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
