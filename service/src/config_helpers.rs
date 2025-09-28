//! Configuration loading and validation helpers

use crate::config::LinkMLConfig;
use configuration_core::{ConfigurationService, Validate};
use linkml_core::{LinkMLError, Result};
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;

/// Load and validate configuration from the configuration service
///
/// # Errors
///
/// Returns an error if configuration loading or validation fails
pub async fn load_and_validate_configuration<C>(
    config_service: &Arc<C>,
) -> Result<LinkMLConfig>
where
    C: ConfigurationService + Send + Sync + 'static,
{
    // Try to load configuration from service
    let service_config = match config_service
        .get_configuration::<LinkMLConfig>("linkml")
        .await
    {
        Ok(config) => {
            // Validate loaded configuration
            config.validate().map_err(|e| {
                LinkMLError::ConfigError(format!("Configuration validation failed: {e}"))
            })?;
            config
        }
        Err(config_error) => {
            // Log configuration load failure and use fallback
            eprintln!(
                "Warning: Failed to load LinkML configuration from service: {config_error}. Using fallback defaults."
            );
            create_fallback_service_config()
        }
    };

    // Additional validation checks
    validate_configuration_constraints(&service_config)?;

    Ok(service_config)
}

/// Create a fallback service configuration when loading from configuration service fails
pub fn create_fallback_service_config() -> LinkMLConfig {
    LinkMLConfig {
        typedb: crate::config::TypeDBConfig {
            server_address: "localhost:1729".to_string(),
            default_database: "linkml".to_string(),
            batch_size: 10,
            connection_timeout_ms: 10000,
            query_timeout_ms: 30000,
            max_retries: 3,
            retry_delay_ms: 1000,
            pool_size: 10,
            include_inferred: false,
        },
        parser: crate::config::ParserConfig {
            max_recursion_depth: 100,
            enable_cache: true,
            cache_ttl_seconds: 3600,
            max_file_size_bytes: 10 * 1024 * 1024,
            supported_formats: vec!["yaml".to_string(), "json".to_string()],
            max_import_depth: 10,
        },
        validator: crate::config::ValidatorConfig {
            enable_parallel: true,
            thread_count: 4,
            batch_size: 100,
            timeout_ms: 5000,
            max_errors: 100,
            fail_fast: false,
            compiled_cache_size: 100,
        },
        generator: crate::config::GeneratorConfig {
            output_directory: "./generated".to_string(),
            enable_formatting: true,
            include_docs: true,
            generator_options: Default::default(),
        },
        cache: crate::config::CacheConfig {
            max_entries: 1000,
            ttl_seconds: 3600,
            enable_compression: false,
            eviction_policy: "lru".to_string(),
            expression_cache: crate::config::CacheSettings {
                max_entries: 500,
                ttl_seconds: 1800,
            },
            rule_cache: crate::config::CacheSettings {
                max_entries: 250,
                ttl_seconds: 3600,
            },
        },
        performance: crate::config::PerformanceConfig {
            features: crate::config::PerformanceFeatures::default(),
            memory_limit_bytes: 512 * 1024 * 1024,
            cpu_limit_percent: 80,
            string_pool_size: 10000,
            background_task_interval_secs: 300,
            string_cache: crate::config::StringCacheConfig {
                max_entries: 5000,
                max_string_length: 1000,
            },
            memory_pool: crate::config::MemoryPoolConfig {
                max_size_bytes: 100 * 1024 * 1024,
                chunk_size_bytes: 4096,
            },
            cache_ttl_levels: crate::config::CacheTtlLevels {
                l1_seconds: 300,
                l2_seconds: 1800,
                l3_seconds: 7200,
                min_ttl_seconds: 60,
                max_ttl_seconds: 86400,
            },
        },
        security_limits: crate::config::SecurityLimits {
            max_string_length: 1_000_000,
            max_expression_depth: 50,
            max_constraint_count: 1000,
            max_cache_entries: 10_000,
            max_function_args: 20,
            max_identifier_length: 255,
            max_json_size_bytes: 10_485_760,
            max_slots_per_class: 100,
            max_classes_per_schema: 1000,
            max_validation_time_ms: 30_000,
            max_memory_usage_bytes: 536_870_912,
            max_parallel_validators: 10,
            max_cache_memory_bytes: 104_857_600,
            max_expression_time_ms: 5_000,
            max_validation_errors: 100,
        },
        network: crate::config::NetworkConfig {
            default_host: "localhost".to_string(),
            default_port: 8080,
            api_timeout_seconds: 30,
        },
        expression: crate::config::ExpressionConfig {
            enable_cache: true,
            enable_compilation: false,
            cache_size: 1000,
            timeout_seconds: 10,
            max_recursion_depth: 50,
        },
        pattern_validator: crate::config::PatternValidatorConfig {
            default_cache_size: 500,
        },
        multi_layer_cache: crate::config::MultiLayerCacheConfig {
            l3_max_size_bytes: 100 * 1024 * 1024,
        },
        background_services: crate::config::BackgroundServicesConfig::default(),
        cli: crate::config::CliConfig::default(),
    }
}

/// Validate configuration constraints beyond basic validation
///
/// # Errors
///
/// Returns an error if configuration violates business constraints
pub fn validate_configuration_constraints(config: &LinkMLConfig) -> Result<()> {
    // Validate TypeDB configuration
    if config.typedb.server_address.is_empty() {
        return Err(LinkMLError::ConfigError(
            "TypeDB server address cannot be empty".to_string(),
        ));
    }
    if config.typedb.default_database.is_empty() {
        return Err(LinkMLError::ConfigError(
            "TypeDB default database cannot be empty".to_string(),
        ));
    }
    if config.typedb.batch_size == 0 {
        return Err(LinkMLError::ConfigError(
            "TypeDB batch size must be greater than 0".to_string(),
        ));
    }

    // Validate parser configuration
    if config.parser.max_recursion_depth == 0 {
        return Err(LinkMLError::ConfigError(
            "Parser max recursion depth must be greater than 0".to_string(),
        ));
    }
    if config.parser.max_file_size_bytes == 0 {
        return Err(LinkMLError::ConfigError(
            "Parser max file size must be greater than 0".to_string(),
        ));
    }

    // Validate validator configuration
    if config.validator.thread_count == 0 {
        return Err(LinkMLError::ConfigError(
            "Validator thread count must be greater than 0".to_string(),
        ));
    }
    if config.validator.batch_size == 0 {
        return Err(LinkMLError::ConfigError(
            "Validator batch size must be greater than 0".to_string(),
        ));
    }
    if config.validator.max_errors == 0 {
        return Err(LinkMLError::ConfigError(
            "Validator max errors must be greater than 0".to_string(),
        ));
    }

    // Validate generator configuration
    if config.generator.output_directory.is_empty() {
        return Err(LinkMLError::ConfigError(
            "Generator output directory cannot be empty".to_string(),
        ));
    }

    // Validate cache configuration
    if config.cache.max_entries == 0 {
        return Err(LinkMLError::ConfigError(
            "Cache max entries must be greater than 0".to_string(),
        ));
    }

    // Validate performance configuration
    if config.performance.cpu_limit_percent > 100 {
        return Err(LinkMLError::ConfigError(
            "CPU limit percent cannot exceed 100".to_string(),
        ));
    }
    if config.performance.memory_limit_bytes == 0 {
        return Err(LinkMLError::ConfigError(
            "Memory limit must be greater than 0".to_string(),
        ));
    }

    // Validate security limits
    if config.security_limits.max_string_length == 0 {
        return Err(LinkMLError::ConfigError(
            "Max string length must be greater than 0".to_string(),
        ));
    }
    if config.security_limits.max_expression_depth == 0 {
        return Err(LinkMLError::ConfigError(
            "Max expression depth must be greater than 0".to_string(),
        ));
    }

    Ok(())
}

/// Convert service-level configuration to core LinkML configuration
pub fn convert_service_to_core_config(
    service_config: &LinkMLConfig,
) -> linkml_core::config::LinkMLConfig {
    use linkml_core::config::{
        IntegrationConfig, GenerationConfig, GenerationTarget, GenerationTargets,
        PerformanceConfig, SchemaConfig, ValidationConfig, ValidationFeatures,
        ValidationMode
    };

    linkml_core::config::LinkMLConfig {
        schema: SchemaConfig {
            search_paths: vec![PathBuf::from("schemas")],
            enable_cache: service_config.parser.enable_cache,
            cache_dir: PathBuf::from(".linkml_cache"),
            import_timeout: Duration::from_millis(service_config.parser.cache_ttl_seconds * 1000),
            max_import_depth: service_config.parser.max_import_depth,
            validate_on_load: true,
        },
        validation: ValidationConfig {
            mode: ValidationMode::Standard,
            features: ValidationFeatures {
                patterns: true,
                instances: true,
                coercion: true,
            },
            instance_paths: vec![PathBuf::from("instances")],
            max_errors: service_config.validator.max_errors,
            timeout: Duration::from_millis(service_config.validator.timeout_ms),
        },
        performance: PerformanceConfig {
            enable_compilation: service_config.performance.features.is_enabled(
                &crate::config::PerformanceFeature::BackgroundTasks
            ),
            thread_pool_size: service_config.validator.thread_count,
            max_concurrent_validations: service_config.validator.batch_size,
            stream_buffer_size: 8192,
            enable_mmap: true,
            cache_size_mb: (service_config.cache.max_entries / 1000).max(1), // Rough conversion
        },
        generation: GenerationConfig {
            output_dir: PathBuf::from(&service_config.generator.output_directory),
            targets: GenerationTargets {
                enabled_targets: vec![
                    GenerationTarget::TypeQL,
                    GenerationTarget::Rust,
                    GenerationTarget::GraphQL,
                    GenerationTarget::Documentation,
                ],
            },
            doc_format: if service_config.generator.include_docs {
                "markdown".to_string()
            } else {
                "none".to_string()
            },
            include_source_info: service_config.generator.include_docs,
        },
        integration: IntegrationConfig {
            enable_iceberg: false,
            iceberg_endpoint: None,
            enable_typedb: true,
            typedb_connection: Some(service_config.typedb.server_address.clone()),
            enable_monitoring: service_config.performance.features.is_enabled(
                &crate::config::PerformanceFeature::Monitoring
            ),
            monitoring_endpoint: None,
        },
    }
}