//! Configuration types for LinkML services

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use std::time::Duration;

/// Main configuration for `LinkML` services
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Validation modes for configuration
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum ValidationMode {
    /// Strict validation - all rules enforced
    Strict,
    /// Standard validation - most rules enforced
    #[default]
    Standard,
    /// Permissive validation - only critical rules enforced
    Permissive,
}

/// Feature flags for validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationFeatures {
    /// Enable pattern validation
    pub patterns: bool,
    /// Enable instance validation
    pub instances: bool,
    /// Enable type coercion
    pub coercion: bool,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ValidationConfig {
    /// Validation mode
    pub mode: ValidationMode,

    /// Validation features
    pub features: ValidationFeatures,

    /// Instance file search paths
    pub instance_paths: Vec<PathBuf>,

    /// Maximum validation errors to collect
    pub max_errors: usize,

    /// Validation timeout
    #[serde(with = "humantime_serde")]
    pub timeout: Duration,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            mode: ValidationMode::Standard,
            features: ValidationFeatures {
                patterns: true,
                instances: true,
                coercion: true,
            },
            instance_paths: vec![PathBuf::from("instances")],
            max_errors: 100,
            timeout: Duration::from_secs(60),
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

/// Generation target options
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum GenerationTarget {
    /// Generate `TypeQL` schemas
    TypeQL,
    /// Generate Rust code
    Rust,
    /// Generate GraphQL schemas
    GraphQL,
    /// Generate documentation
    #[default]
    Documentation,
}

/// Generation targets configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerationTargets {
    /// Enabled generation targets
    pub enabled_targets: Vec<GenerationTarget>,
}

/// Code generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GenerationConfig {
    /// Output directory for generated code
    pub output_dir: PathBuf,

    /// Generation targets
    pub targets: GenerationTargets,

    /// Documentation format
    pub doc_format: String,

    /// Include source location comments
    pub include_source_info: bool,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        use GenerationTarget::{Documentation, GraphQL, Rust, TypeQL};
        Self {
            output_dir: PathBuf::from("generated"),
            targets: GenerationTargets {
                enabled_targets: vec![TypeQL, Rust, GraphQL, Documentation],
            },
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

    /// Enable `TypeDB` integration
    pub enable_typedb: bool,

    /// `TypeDB` connection string
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
        assert!(matches!(config.validation.mode, ValidationMode::Standard));
    }

    #[test]
    fn test_config_serialization() -> crate::Result<()> {
        let config = LinkMLConfig::default();
        let yaml = serde_yaml::to_string(&config)?;
        assert!(yaml.contains("enable_cache"));

        let parsed: LinkMLConfig = serde_yaml::from_str(&yaml)?;
        assert_eq!(parsed.schema.enable_cache, config.schema.enable_cache);
        Ok(())
    }
}
