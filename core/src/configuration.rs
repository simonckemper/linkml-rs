//! Configuration structures for LinkML service
//!
//! All configuration values must be externalized through RootReal's Configuration Service.
//! NO HARDCODED VALUES are allowed per RootReal architecture standards.

use rootreal_core_application_config_configuration_core::{ConfigurationError, Validate};
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
}

/// Cache eviction policy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum EvictionPolicy {
    Lru,
    Lfu,
    Fifo,
    Ttl,
}

/// Performance configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PerformanceConfig {
    /// Enable performance monitoring
    pub enable_monitoring: bool,

    /// Memory limit in bytes
    pub memory_limit_bytes: Option<u64>,

    /// CPU limit (percentage)
    pub cpu_limit_percent: Option<u8>,

    /// Enable string interning
    pub enable_string_interning: bool,

    /// String intern pool size
    pub string_pool_size: usize,
}

impl Validate for LinkMLServiceConfig {
    type Error = ConfigurationError;

    fn validate(&self) -> std::result::Result<(), Self::Error> {
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
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_monitoring: true,
            memory_limit_bytes: None,
            cpu_limit_percent: None,
            enable_string_interning: true,
            string_pool_size: 10000,
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
        config.performance.enable_monitoring = true;
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
    }

    /// Production environment configuration
    #[must_use]
    pub fn production() -> Self {
        let mut config = Self::default();
        config.typedb.pool_size = 50;
        config.typedb.max_retries = 5;
        config.validator.enable_parallel = true;
        config.cache.enable_compression = true;
        config.performance.enable_string_interning = true;
        config
    }
}
