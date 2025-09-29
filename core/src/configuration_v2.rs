//! Enhanced configuration structures for LinkML service
//!
//! All configuration values must be externalized through RootReal's Configuration Service.
//! NO HARDCODED VALUES are allowed per RootReal architecture standards.

use configuration_core::{ConfigurationError, Validate};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main `LinkML` service configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LinkMLServiceConfig {
    /// `TypeDB` configuration
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

    /// Security limits configuration
    pub security_limits: SecurityLimitsConfig,
}

/// `TypeDB` specific configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TypeDBConfig {
    /// `TypeDB` server address (e.g., "localhost:1729")
    pub server_address: String,

    /// Default database name
    pub default_database: String,

    /// Batch size for operations
    pub batch_size: usize,

    /// Connection timeout in milliseconds
    pub connection_timeout_ms: u64,

    /// Query timeout in milliseconds
    pub query_timeout_ms: u64,

    /// Maximum retry attempts
    pub max_retries: u32,

    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,

    /// Connection pool size
    pub pool_size: usize,

    /// Include inferred attributes
    pub include_inferred: bool,
}

/// Parser configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ParserConfig {
    /// Maximum recursion depth for imports
    pub max_recursion_depth: u32,

    /// Enable schema caching
    pub enable_cache: bool,

    /// Schema cache TTL in seconds
    pub cache_ttl_seconds: u64,

    /// Maximum file size in bytes
    pub max_file_size_bytes: u64,

    /// Supported schema formats
    pub supported_formats: Vec<String>,

    /// Maximum import depth
    pub max_import_depth: usize,
}

/// Validator configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ValidatorConfig {
    /// Enable parallel validation
    pub enable_parallel: bool,

    /// Number of validation threads
    pub thread_count: usize,

    /// Maximum validation batch size
    pub batch_size: usize,

    /// Validation timeout in milliseconds
    pub timeout_ms: u64,

    /// Maximum validation errors to collect
    pub max_errors: usize,

    /// Stop on first error
    pub fail_fast: bool,

    /// Compiled validator cache size
    pub compiled_cache_size: usize,
}

/// Generator configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneratorConfig {
    /// Default output directory
    pub output_directory: String,

    /// Enable code formatting
    pub enable_formatting: bool,

    /// Include documentation comments
    pub include_docs: bool,

    /// Generator-specific options
    pub generator_options: HashMap<String, GeneratorOptions>,
}

/// Generator-specific options
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneratorOptions {
    /// Custom template path
    pub template_path: Option<String>,

    /// Additional generator settings
    pub settings: HashMap<String, serde_json::Value>,
}

/// Cache configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    /// Maximum cache entries
    pub max_entries: usize,

    /// Cache TTL in seconds
    pub ttl_seconds: u64,

    /// Enable cache compression
    pub enable_compression: bool,

    /// Cache eviction policy
    pub eviction_policy: EvictionPolicy,

    /// Expression cache configuration
    pub expression_cache: ExpressionCacheConfig,

    /// Rule cache configuration
    pub rule_cache: RuleCacheConfig,
}

/// Cache eviction policy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
    Ttl,
}

/// Expression cache configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExpressionCacheConfig {
    /// Maximum cached expressions
    pub max_entries: usize,

    /// Expression cache TTL in seconds
    pub ttl_seconds: u64,
}

/// Rule cache configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleCacheConfig {
    /// Maximum cached rules
    pub max_entries: usize,

    /// Rule cache TTL in seconds
    pub ttl_seconds: u64,
}

/// Performance feature flags
#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize)]
pub enum PerformanceFeature {
    /// Performance monitoring
    #[default]
    Monitoring,
    /// String interning optimization
    StringInterning,
    /// Background task processing
    BackgroundTasks,
    /// Cache warming on startup
    CacheWarming,
}

/// Performance features configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PerformanceFeatures {
    /// Enabled performance features
    pub enabled_features: Vec<PerformanceFeature>,
}

/// Performance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Performance features
    pub features: PerformanceFeatures,

    /// Memory limit in bytes
    pub memory_limit_bytes: Option<u64>,

    /// CPU limit (percentage)
    pub cpu_limit_percent: Option<u8>,

    /// String intern pool size
    pub string_pool_size: usize,

    /// Background task interval in seconds
    pub background_task_interval_secs: u64,

    /// String cache configuration
    pub string_cache: StringCacheConfig,

    /// Memory pool configuration
    pub memory_pool: MemoryPoolConfig,

    /// Cache TTL levels configuration
    pub cache_ttl_levels: CacheTtlLevelsConfig,
}

/// Cache TTL levels configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheTtlLevelsConfig {
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

/// String cache configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StringCacheConfig {
    /// Maximum cached strings
    pub max_entries: usize,

    /// Maximum string length to cache
    pub max_string_length: usize,
}

/// Memory pool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryPoolConfig {
    /// Maximum pool size in bytes
    pub max_size_bytes: usize,

    /// Allocation chunk size
    pub chunk_size_bytes: usize,
}

/// Security limits configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityLimitsConfig {
    /// Maximum string length
    pub max_string_length: usize,

    /// Maximum expression depth
    pub max_expression_depth: usize,

    /// Maximum constraint count
    pub max_constraint_count: usize,

    /// Maximum cache entries
    pub max_cache_entries: usize,

    /// Maximum function arguments
    pub max_function_args: usize,

    /// Maximum identifier length
    pub max_identifier_length: usize,

    /// Maximum `JSON` size in bytes
    pub max_json_size_bytes: usize,

    /// Maximum slots per class
    pub max_slots_per_class: usize,

    /// Maximum classes per schema
    pub max_classes_per_schema: usize,

    /// Maximum validation time in milliseconds
    pub max_validation_time_ms: u64,

    /// Maximum memory usage in bytes
    pub max_memory_usage_bytes: usize,

    /// Maximum parallel validators
    pub max_parallel_validators: usize,

    /// Maximum cache memory in bytes
    pub max_cache_memory_bytes: usize,

    /// Maximum expression evaluation time in milliseconds
    pub max_expression_time_ms: u64,

    /// Maximum validation errors to collect
    pub max_validation_errors: usize,
}

impl Validate for LinkMLServiceConfig {
    type Error = ConfigurationError;

    fn validate(&self) -> Result<(), Self::Error> {
        // Validate TypeDB config
        if self.typedb.server_address.is_empty() {
            return Err(ConfigurationError::validation_error(
                "TypeDB server address cannot be empty".to_string(),
            ));
        }

        if self.typedb.batch_size == 0 {
            return Err(ConfigurationError::validation_error(
                "TypeDB batch size must be greater than 0".to_string(),
            ));
        }

        if self.typedb.pool_size == 0 {
            return Err(ConfigurationError::validation_error(
                "TypeDB connection pool size must be greater than 0".to_string(),
            ));
        }

        // Validate parser config
        if self.parser.max_recursion_depth == 0 {
            return Err(ConfigurationError::validation_error(
                "Parser max recursion depth must be greater than 0".to_string(),
            ));
        }

        if self.parser.max_file_size_bytes == 0 {
            return Err(ConfigurationError::validation_error(
                "Parser max file size must be greater than 0".to_string(),
            ));
        }

        // Validate validator config
        if self.validator.thread_count == 0 {
            return Err(ConfigurationError::validation_error(
                "Validator thread count must be greater than 0".to_string(),
            ));
        }

        // Validate cache config
        if self.cache.max_entries == 0 {
            return Err(ConfigurationError::validation_error(
                "Cache max entries must be greater than 0".to_string(),
            ));
        }

        // Validate performance config
        if let Some(cpu_limit) = self.performance.cpu_limit_percent
            && (cpu_limit == 0 || cpu_limit > 100)
        {
            return Err(ConfigurationError::validation_error(
                "CPU limit must be between 1 and 100 percent".to_string(),
            ));
        }

        // Validate security limits
        if self.security_limits.max_string_length == 0 {
            return Err(ConfigurationError::validation_error(
                "Max string length must be greater than 0".to_string(),
            ));
        }

        Ok(())
    }
}

impl Default for TypeDBConfig {
    fn default() -> Self {
        Self {
            server_address: String::from("localhost:1729"),
            default_database: String::from("linkml"),
            batch_size: 1000,
            connection_timeout_ms: 30000,
            query_timeout_ms: 10000,
            max_retries: 3,
            retry_delay_ms: 1000,
            pool_size: 10,
            include_inferred: false,
        }
    }
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            max_recursion_depth: 100,
            enable_cache: true,
            cache_ttl_seconds: 3600,
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
            supported_formats: vec!["yaml".to_string(), "yml".to_string(), "json".to_string()],
            max_import_depth: 10,
        }
    }
}

impl Default for ValidatorConfig {
    fn default() -> Self {
        Self {
            enable_parallel: true,
            thread_count: num_cpus::get(),
            batch_size: 100,
            timeout_ms: 60000,
            max_errors: 100,
            fail_fast: false,
            compiled_cache_size: 10000,
        }
    }
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            output_directory: String::from("./generated"),
            enable_formatting: true,
            include_docs: true,
            generator_options: HashMap::new(),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            ttl_seconds: 3600,
            enable_compression: false,
            eviction_policy: EvictionPolicy::Lru,
            expression_cache: ExpressionCacheConfig::default(),
            rule_cache: RuleCacheConfig::default(),
        }
    }
}

impl Default for ExpressionCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl_seconds: 3600,
        }
    }
}

impl Default for RuleCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl_seconds: 3600,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        use PerformanceFeature::{BackgroundTasks, Monitoring, StringInterning};

        Self {
            features: PerformanceFeatures {
                enabled_features: vec![Monitoring, StringInterning, BackgroundTasks],
            },
            memory_limit_bytes: None,
            cpu_limit_percent: None,
            string_pool_size: 10000,
            background_task_interval_secs: 3600,
            string_cache: StringCacheConfig::default(),
            memory_pool: MemoryPoolConfig::default(),
            cache_ttl_levels: CacheTtlLevelsConfig::default(),
        }
    }
}

impl Default for StringCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 100_000,
            max_string_length: 10_000,
        }
    }
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024, // 10MB
            chunk_size_bytes: 4096,
        }
    }
}

impl Default for CacheTtlLevelsConfig {
    fn default() -> Self {
        Self {
            l1_seconds: 300,          // 5 minutes
            l2_seconds: 3600,         // 1 hour
            l3_seconds: 86400,        // 24 hours
            min_ttl_seconds: 60,      // 1 minute
            max_ttl_seconds: 604_800, // 7 days
        }
    }
}

impl Default for SecurityLimitsConfig {
    fn default() -> Self {
        Self {
            max_string_length: 1_000_000, // 1MB
            max_expression_depth: 100,
            max_constraint_count: 1000,
            max_cache_entries: 10_000,
            max_function_args: 20,
            max_identifier_length: 256,
            max_json_size_bytes: 10_000_000, // 10MB
            max_slots_per_class: 1000,
            max_classes_per_schema: 10_000,
            max_validation_time_ms: 30_000,        // 30 seconds
            max_memory_usage_bytes: 1_000_000_000, // 1GB
            max_parallel_validators: 100,
            max_cache_memory_bytes: 100_000_000, // 100MB
            max_expression_time_ms: 1000,        // 1 second
            max_validation_errors: 1000,
        }
    }
}

/// Environment-specific configuration presets
impl LinkMLServiceConfig {
    /// Development environment configuration
    #[must_use]
    pub fn development() -> Self {
        let mut config = Self::default();
        config.typedb.server_address = String::from("localhost:1729");
        config.validator.fail_fast = true;
        config.cache.max_entries = 100;
        // Performance monitoring is already enabled by default
        config.security_limits.max_validation_time_ms = 1000; // More lenient for dev
        config
    }

    /// Testing environment configuration
    #[must_use]
    pub fn testing() -> Self {
        let mut config = Self::default();
        config.typedb.server_address = String::from("localhost:1730");
        config.typedb.batch_size = 10;
        config.validator.thread_count = 1;
        config.cache.max_entries = 10;
        config
            .performance
            .features
            .enabled_features
            .retain(|f| !matches!(f, PerformanceFeature::BackgroundTasks));
        config
    }

    /// Production environment configuration
    #[must_use]
    pub fn production() -> Self {
        let mut config = Self::default();
        config.typedb.pool_size = 50;
        config.typedb.max_retries = 5;
        config.validator.enable_parallel = true;
        config.cache.enable_compression = true;
        // String interning and cache warming already enabled by default
        config
            .performance
            .features
            .enabled_features
            .push(PerformanceFeature::CacheWarming);
        config
    }
}
