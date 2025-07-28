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
        .map_err(|e| LinkMLError::Io(e))?;
    
    // Substitute environment variables
    let substituted = substitute_env_vars(&contents);
    
    // Parse YAML
    serde_yaml::from_str(&substituted)
        .map_err(|e| LinkMLError::configuration(format!("Failed to parse YAML config: {}", e)))
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
    pub server_address: String,
    pub default_database: String,
    pub batch_size: usize,
    pub connection_timeout_ms: u64,
    pub query_timeout_ms: u64,
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub pool_size: usize,
    pub include_inferred: bool,
}

/// Parser configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    pub max_recursion_depth: usize,
    pub enable_cache: bool,
    pub cache_ttl_seconds: u64,
    pub max_file_size_bytes: u64,
    pub supported_formats: Vec<String>,
    pub max_import_depth: usize,
}

/// Validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorConfig {
    pub enable_parallel: bool,
    pub thread_count: usize,
    pub batch_size: usize,
    pub timeout_ms: u64,
    pub max_errors: usize,
    pub fail_fast: bool,
    pub compiled_cache_size: usize,
}

/// Generator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorConfig {
    pub output_directory: String,
    pub enable_formatting: bool,
    pub include_docs: bool,
    pub generator_options: HashMap<String, GeneratorOptions>,
}

/// Generator-specific options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratorOptions {
    pub template_path: Option<String>,
    pub settings: HashMap<String, serde_json::Value>,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub max_entries: usize,
    pub ttl_seconds: u64,
    pub enable_compression: bool,
    pub eviction_policy: String,
    pub expression_cache: CacheSettings,
    pub rule_cache: CacheSettings,
}

/// Cache settings for specific components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    pub max_entries: usize,
    pub ttl_seconds: u64,
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enable_monitoring: bool,
    pub memory_limit_bytes: u64,
    pub cpu_limit_percent: u8,
    pub enable_string_interning: bool,
    pub string_pool_size: usize,
    pub enable_background_tasks: bool,
    pub enable_cache_warming: bool,
    pub background_task_interval_secs: u64,
    pub string_cache: StringCacheConfig,
    pub memory_pool: MemoryPoolConfig,
    pub cache_ttl_levels: CacheTtlLevels,
}

/// String cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringCacheConfig {
    pub max_entries: usize,
    pub max_string_length: usize,
}

/// Memory pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolConfig {
    pub max_size_bytes: u64,
    pub chunk_size_bytes: usize,
}

/// Cache TTL levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheTtlLevels {
    pub l1_seconds: u64,
    pub l2_seconds: u64,
    pub l3_seconds: u64,
    pub min_ttl_seconds: u64,
    pub max_ttl_seconds: u64,
}

/// Security limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityLimits {
    pub max_string_length: usize,
    pub max_expression_depth: usize,
    pub max_constraint_count: usize,
    pub max_cache_entries: usize,
    pub max_function_args: usize,
    pub max_identifier_length: usize,
    pub max_json_size_bytes: u64,
    pub max_slots_per_class: usize,
    pub max_classes_per_schema: usize,
    pub max_validation_time_ms: u64,
    pub max_memory_usage_bytes: u64,
    pub max_parallel_validators: usize,
    pub max_cache_memory_bytes: u64,
    pub max_expression_time_ms: u64,
    pub max_validation_errors: usize,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub default_host: String,
    pub default_port: u16,
    pub api_timeout_seconds: u64,
}

/// Expression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionConfig {
    pub enable_cache: bool,
    pub enable_compilation: bool,
    pub cache_size: usize,
    pub timeout_seconds: u64,
    pub max_recursion_depth: usize,
}

/// Pattern validator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternValidatorConfig {
    pub default_cache_size: usize,
}

/// Multi-layer cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiLayerCacheConfig {
    pub l3_max_size_bytes: u64,
}

/// Background services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundServicesConfig {
    pub cache_ttl_check_interval_secs: u64,
    pub memory_cleanup_interval_secs: u64,
    pub panic_recovery_timeout_secs: u64,
    pub error_recovery_timeout_secs: u64,
}

/// CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    pub default_iterations: usize,
    pub progress_bar_template: String,
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
        env::set_var("TEST_VAR", "test_value");
        
        let content = "server: ${TEST_VAR:-default}";
        let result = substitute_env_vars(content);
        assert_eq!(result, "server: test_value");
        
        let content = "server: ${NONEXISTENT:-default_value}";
        let result = substitute_env_vars(content);
        assert_eq!(result, "server: default_value");
        
        env::remove_var("TEST_VAR");
    }
}