//! Configuration types for LinkML services

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration for LinkML services
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LinkMLConfig {
    /// Schema loading configuration
    pub schema: SchemaConfig,
    
    /// Validation configuration
    pub validation: ValidationConfig,
    
    /// Performance configuration
    pub performance: PerformanceConfig,
    
    /// Code generation configuration
    pub generation: GenerationConfig,
    
    /// Integration configuration
    pub integration: IntegrationConfig,
}

impl Default for LinkMLConfig {
    fn default() -> Self {
        Self {
            schema: SchemaConfig::default(),
            validation: ValidationConfig::default(),
            performance: PerformanceConfig::default(),
            generation: GenerationConfig::default(),
            integration: IntegrationConfig::default(),
        }
    }
}

/// Schema loading configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SchemaConfig {
    /// Base directories for schema resolution
    pub search_paths: Vec<PathBuf>,
    
    /// Enable schema caching
    pub enable_cache: bool,
    
    /// Cache directory
    pub cache_dir: PathBuf,
    
    /// Schema import timeout
    #[serde(with = "humantime_serde")]
    pub import_timeout: Duration,
    
    /// Maximum import depth
    pub max_import_depth: usize,
    
    /// Validate schemas on load
    pub validate_on_load: bool,
}

impl Default for SchemaConfig {
    fn default() -> Self {
        Self {
            search_paths: vec![PathBuf::from("schemas")],
            enable_cache: true,
            cache_dir: PathBuf::from(".linkml_cache"),
            import_timeout: Duration::from_secs(30),
            max_import_depth: 10,
            validate_on_load: true,
        }
    }
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ValidationConfig {
    /// Enable strict mode
    pub strict_mode: bool,
    
    /// Enable pattern validation
    pub enable_patterns: bool,
    
    /// Enable instance validation
    pub enable_instances: bool,
    
    /// Instance file search paths
    pub instance_paths: Vec<PathBuf>,
    
    /// Maximum validation errors to collect
    pub max_errors: usize,
    
    /// Validation timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
    
    /// Enable type coercion
    pub enable_coercion: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            enable_patterns: true,
            enable_instances: true,
            instance_paths: vec![PathBuf::from("instances")],
            max_errors: 100,
            timeout: Duration::from_secs(60),
            enable_coercion: true,
        }
    }
}

/// Performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerformanceConfig {
    /// Enable compiled validators
    pub enable_compilation: bool,
    
    /// Thread pool size for parallel validation
    pub thread_pool_size: usize,
    
    /// Maximum concurrent validations
    pub max_concurrent_validations: usize,
    
    /// Buffer size for streaming operations
    pub stream_buffer_size: usize,
    
    /// Enable memory mapping for large files
    pub enable_mmap: bool,
    
    /// Cache size in MB
    pub cache_size_mb: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_compilation: true,
            thread_pool_size: num_cpus::get(),
            max_concurrent_validations: 100,
            stream_buffer_size: 8192,
            enable_mmap: true,
            cache_size_mb: 256,
        }
    }
}

/// Code generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerationConfig {
    /// Output directory for generated code
    pub output_dir: PathBuf,
    
    /// Enable TypeQL generation
    pub enable_typeql: bool,
    
    /// Enable Rust generation
    pub enable_rust: bool,
    
    /// Enable GraphQL generation
    pub enable_graphql: bool,
    
    /// Enable documentation generation
    pub enable_docs: bool,
    
    /// Documentation format
    pub doc_format: String,
    
    /// Include source location comments
    pub include_source_info: bool,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("generated"),
            enable_typeql: true,
            enable_rust: true,
            enable_graphql: true,
            enable_docs: true,
            doc_format: "markdown".to_string(),
            include_source_info: true,
        }
    }
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IntegrationConfig {
    /// Enable Iceberg integration
    pub enable_iceberg: bool,
    
    /// Iceberg service endpoint
    pub iceberg_endpoint: Option<String>,
    
    /// Enable TypeDB integration
    pub enable_typedb: bool,
    
    /// TypeDB connection string
    pub typedb_connection: Option<String>,
    
    /// Enable monitoring
    pub enable_monitoring: bool,
    
    /// Monitoring endpoint
    pub monitoring_endpoint: Option<String>,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            enable_iceberg: true,
            iceberg_endpoint: None,
            enable_typedb: true,
            typedb_connection: None,
            enable_monitoring: true,
            monitoring_endpoint: None,
        }
    }
}

/// Validation options for runtime configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationOptions {
    /// Target class for validation
    pub target_class: Option<String>,
    
    /// Enable strict validation
    pub strict: bool,
    
    /// Collect all errors (don't fail fast)
    pub collect_all: bool,
    
    /// Maximum errors to collect
    pub max_errors: Option<usize>,
    
    /// Include warnings
    pub include_warnings: bool,
}

/// Generation options for runtime configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationOptions {
    /// Include private fields
    pub include_private: bool,
    
    /// Include documentation
    pub include_docs: bool,
    
    /// Custom template path
    pub template_path: Option<PathBuf>,
    
    /// Additional context data
    pub context: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LinkMLConfig::default();
        assert!(config.schema.enable_cache);
        assert_eq!(config.validation.max_errors, 100);
        assert_eq!(config.performance.thread_pool_size, num_cpus::get());
    }

    #[test]
    fn test_config_serialization() -> Result<(), Box<dyn std::error::Error>> {
        let config = LinkMLConfig::default();
        let yaml = serde_yaml::to_string(&config)?;
        assert!(yaml.contains("enable_cache"));
        
        let parsed: LinkMLConfig = serde_yaml::from_str(&yaml)?;
        assert_eq!(parsed.schema.enable_cache, config.schema.enable_cache);
        Ok(())
    }
}