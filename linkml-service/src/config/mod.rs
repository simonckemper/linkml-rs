//! Configuration loading for LinkML service
//!
//! This module provides configuration loading from YAML files with
//! environment variable substitution support.

pub mod validation;
pub mod hot_reload;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::Path;
use linkml_core::error::{LinkMLError, Result};

/// Load configuration from YAML file with environment variable substitution
pub fn load_config<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    // Read the file
    let contents = std::fs::read_to_string(path)
        .map_err(|e| LinkMLError::IoError(e))?;
    
    // Substitute environment variables
    let substituted = substitute_env_vars(&contents);
    
    // Parse YAML
    serde_yaml::from_str(&substituted)
        .map_err(|e| LinkMLError::ConfigError(format!("Failed to parse YAML config: {}", e)))
}

/// Substitute environment variables in the format ${VAR:-default}
fn substitute_env_vars(content: &str) -> String {
    let re = regex::Regex::new(r"\$\{([^}:]+)(?::(-)?([^}]*))?\}")
        .expect("regex should be valid");
    
    re.replace_all(content, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let default_value = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        
        env::var(var_name).unwrap_or_else(|_| default_value.to_string())
    }).to_string()
}

/// Complete LinkML service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMLConfig {
    /// TypeDB configuration
    pub typedb: TypeDBConfig,
    /// Parser configuration
    pub parser: ParserConfig,
    /// Validator configuration
    pub validator: ValidatorConfig,
    /// Generator configuration
    pub generator: GeneratorConfig,
    /// Cache configuration
    pub cache: CacheConfig,
    /// Performance configuration
    pub performance: PerformanceConfig,
    /// Security limits
    pub security_limits: SecurityLimits,
    /// Network configuration
    pub network: NetworkConfig,
    /// Expression configuration
    pub expression: ExpressionConfig,
    /// Pattern validator configuration
    pub pattern_validator: PatternValidatorConfig,
    /// Multi-layer cache configuration
    pub multi_layer_cache: MultiLayerCacheConfig,
    /// Background services configuration
    pub background_services: BackgroundServicesConfig,
    /// CLI configuration
    pub cli: CliConfig,
}

/// TypeDB configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDBConfig {
    /// Server address (e.g., "localhost:1729")
    pub server_address: String,
    /// Default database name
    pub default_database: String,
    /// Batch size for bulk operations
    pub batch_size: usize,
    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,
    /// Query timeout in milliseconds
    pub query_timeout_ms: u64,
    /// Maximum number of retries for failed operations
    pub max_retries: u32,
    /// Delay between retries in milliseconds
    pub retry_delay_ms: u64,
    /// Connection pool size
    pub pool_size: usize,
    /// Whether to include inferred facts in query results
    pub include_inferred: bool,
}

/// Parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Maximum recursion depth for nested structures
    pub max_recursion_depth: usize,
    /// Whether to enable parser cache
    pub enable_cache: bool,
    /// Cache time-to-live in seconds
    pub cache_ttl_seconds: u64,
    /// Maximum file size in bytes
    pub max_file_size_bytes: u64,
    /// List of supported file formats
    pub supported_formats: Vec<String>,
    /// Maximum import depth for schema imports
    pub max_import_depth: usize,
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    /// Whether to enable parallel validation
    pub enable_parallel: bool,
    /// Number of threads for parallel validation
    pub thread_count: usize,
    /// Batch size for validation operations
    pub batch_size: usize,
    /// Validation timeout in milliseconds
    pub timeout_ms: u64,
    /// Maximum number of errors to collect
    pub max_errors: usize,
    /// Whether to stop on first error
    pub fail_fast: bool,
    /// Size of compiled validator cache
    pub compiled_cache_size: usize,
}

/// Generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    /// Output directory for generated files
    pub output_directory: String,
    /// Whether to enable code formatting
    pub enable_formatting: bool,
    /// Whether to include documentation in generated code
    pub include_docs: bool,
    /// Generator-specific options by generator name
    pub generator_options: HashMap<String, GeneratorOptions>,
}

/// Generator-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOptions {
    /// Path to custom template file
    pub template_path: Option<String>,
    /// Additional generator settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Maximum number of cache entries
    pub max_entries: usize,
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
    /// Whether to enable cache compression
    pub enable_compression: bool,
    /// Cache eviction policy (e.g., "lru", "lfu")
    pub eviction_policy: String,
    /// Expression cache settings
    pub expression_cache: CacheSettings,
    /// Rule cache settings
    pub rule_cache: CacheSettings,
}

/// Cache settings for specific components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Time-to-live in seconds
    pub ttl_seconds: u64,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Whether to enable performance monitoring
    pub enable_monitoring: bool,
    /// Memory limit in bytes
    pub memory_limit_bytes: u64,
    /// CPU limit as percentage (0-100)
    pub cpu_limit_percent: u8,
    /// Whether to enable string interning
    pub enable_string_interning: bool,
    /// Size of string pool for interning
    pub string_pool_size: usize,
    /// Whether to enable background tasks
    pub enable_background_tasks: bool,
    /// Whether to enable cache warming
    pub enable_cache_warming: bool,
    /// Background task interval in seconds
    pub background_task_interval_secs: u64,
    /// String cache configuration
    pub string_cache: StringCacheConfig,
    /// Memory pool configuration
    pub memory_pool: MemoryPoolConfig,
    /// Cache TTL levels configuration
    pub cache_ttl_levels: CacheTtlLevels,
}

/// String cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringCacheConfig {
    /// Maximum number of cached strings
    pub max_entries: usize,
    /// Maximum length of cached strings
    pub max_string_length: usize,
}

/// Memory pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolConfig {
    /// Maximum size of memory pool in bytes
    pub max_size_bytes: u64,
    /// Size of memory chunks in bytes
    pub chunk_size_bytes: usize,
}

/// Cache TTL levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTtlLevels {
    /// L1 cache TTL in seconds
    pub l1_seconds: u64,
    /// L2 cache TTL in seconds
    pub l2_seconds: u64,
    /// L3 cache TTL in seconds
    pub l3_seconds: u64,
    /// Minimum TTL in seconds
    pub min_ttl_seconds: u64,
    /// Maximum TTL in seconds
    pub max_ttl_seconds: u64,
}

/// Security limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimits {
    /// Maximum string length in characters
    pub max_string_length: usize,
    /// Maximum expression nesting depth
    pub max_expression_depth: usize,
    /// Maximum number of constraints
    pub max_constraint_count: usize,
    /// Maximum number of cache entries
    pub max_cache_entries: usize,
    /// Maximum function argument count
    pub max_function_args: usize,
    /// Maximum identifier length
    pub max_identifier_length: usize,
    /// Maximum JSON payload size in bytes
    pub max_json_size_bytes: u64,
    /// Maximum slots per class
    pub max_slots_per_class: usize,
    /// Maximum classes per schema
    pub max_classes_per_schema: usize,
    /// Maximum validation time in milliseconds
    pub max_validation_time_ms: u64,
    /// Maximum memory usage in bytes
    pub max_memory_usage_bytes: u64,
    /// Maximum parallel validators
    pub max_parallel_validators: usize,
    /// Maximum cache memory in bytes
    pub max_cache_memory_bytes: u64,
    /// Maximum expression evaluation time in milliseconds
    pub max_expression_time_ms: u64,
    /// Maximum validation errors to collect
    pub max_validation_errors: usize,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Default host address
    pub default_host: String,
    /// Default port number
    pub default_port: u16,
    /// API timeout in seconds
    pub api_timeout_seconds: u64,
}

/// Expression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionConfig {
    /// Whether to enable expression cache
    pub enable_cache: bool,
    /// Whether to enable expression compilation
    pub enable_compilation: bool,
    /// Expression cache size
    pub cache_size: usize,
    /// Expression evaluation timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum recursion depth for expressions
    pub max_recursion_depth: usize,
}

/// Pattern validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternValidatorConfig {
    /// Default cache size for pattern validators
    pub default_cache_size: usize,
}

/// Multi-layer cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLayerCacheConfig {
    /// L3 cache maximum size in bytes
    pub l3_max_size_bytes: u64,
}

/// Background services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundServicesConfig {
    /// Cache TTL check interval in seconds
    pub cache_ttl_check_interval_secs: u64,
    /// Memory cleanup interval in seconds
    pub memory_cleanup_interval_secs: u64,
    /// Panic recovery timeout in seconds
    pub panic_recovery_timeout_secs: u64,
    /// Error recovery timeout in seconds
    pub error_recovery_timeout_secs: u64,
}

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Default number of iterations for benchmarks
    pub default_iterations: usize,
    /// Progress bar template string
    pub progress_bar_template: String,
    /// Progress bar finish template string
    pub progress_bar_finish_template: String,
}

/// Default configuration file path
pub const DEFAULT_CONFIG_PATH: &str = "config/default.yaml";

/// Production configuration file path
pub const PRODUCTION_CONFIG_PATH: &str = "config/production.yaml";

/// Load default configuration
pub fn load_default_config() -> Result<LinkMLConfig> {
    let path = Path::new(DEFAULT_CONFIG_PATH);
    load_config(path)
}

/// Load production configuration
pub fn load_production_config() -> Result<LinkMLConfig> {
    let path = Path::new(PRODUCTION_CONFIG_PATH);
    load_config(path)
}

/// Load configuration based on environment
pub fn load_environment_config() -> Result<LinkMLConfig> {
    let env = env::var("LINKML_ENV").unwrap_or_else(|_| "default".to_string());
    
    match env.as_str() {
        "production" | "prod" => load_production_config(),
        _ => load_default_config(),
    }
}

/// Get a configuration instance (singleton pattern)
static INSTANCE: std::sync::OnceLock<LinkMLConfig> = std::sync::OnceLock::new();

/// Get the global configuration instance
pub fn get_config() -> &'static LinkMLConfig {
    INSTANCE.get_or_init(|| {
        load_environment_config()
            .expect("Failed to load LinkML configuration")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_env_var_substitution() {
        // Test with default values only since we can't set env vars without unsafe
        let content = "server: ${NONEXISTENT:-default_value}";
        let result = substitute_env_vars(content);
        assert_eq!(result, "server: default_value");
        
        // Test multiple substitutions
        let content = "${VAR1:-val1} and ${VAR2:-val2}";
        let result = substitute_env_vars(content);
        assert_eq!(result, "val1 and val2");
    }
}