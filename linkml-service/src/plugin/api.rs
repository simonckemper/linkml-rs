//! Plugin API contracts and interfaces
//!
//! This module defines the API contracts that plugins must implement
//! and provides helper utilities for plugin development.

use super::*;
use std::any::Any;

/// Plugin API version
pub const PLUGIN_API_VERSION: u32 = 1;

/// Plugin capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginCapability {
    /// Can generate code
    CodeGeneration,
    /// Can validate data
    DataValidation,
    /// Can load data
    DataLoading,
    /// Can dump data
    DataDumping,
    /// Can transform schemas
    SchemaTransformation,
    /// Can provide custom functions
    CustomFunctions,
    /// Can analyze schemas
    SchemaAnalysis,
    /// Supports async operations
    AsyncOperations,
    /// Supports streaming data
    StreamingData,
    /// Supports batch operations
    BatchOperations,
    /// Can be configured at runtime
    RuntimeConfiguration,
    /// Supports hot reload
    HotReload,
}

/// Plugin metadata for runtime introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    /// API version this plugin was built against
    pub api_version: u32,
    /// Plugin SDK version
    pub sdk_version: String,
    /// Build timestamp
    pub build_time: String,
    /// Build host
    pub build_host: Option<String>,
    /// Custom metadata
    pub custom: HashMap<String, serde_json::Value>,
}

/// Plugin lifecycle events
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    /// Before initialization
    PreInit,
    /// After initialization
    PostInit,
    /// Before shutdown
    PreShutdown,
    /// After shutdown
    PostShutdown,
    /// Configuration changed
    ConfigChanged,
    /// Error occurred
    Error,
}

/// Plugin event handler
#[async_trait]
pub trait PluginEventHandler: Send + Sync {
    /// Handle a lifecycle event
    async fn handle_event(&mut self, event: LifecycleEvent, data: Option<&dyn Any>) -> Result<()>;
}

/// Plugin configuration schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSchema {
    /// JSON Schema for configuration validation
    pub schema: serde_json::Value,
    /// Default configuration values
    pub defaults: HashMap<String, serde_json::Value>,
    /// Configuration examples
    pub examples: Vec<HashMap<String, serde_json::Value>>,
}

/// Plugin health check interface
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Check if the plugin is healthy
    async fn check_health(&self) -> HealthStatus;

    /// Get detailed health metrics
    async fn health_metrics(&self) -> HealthMetrics;
}

/// Plugin health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Overall health state
    pub status: HealthState,
    /// Health check message
    pub message: Option<String>,
    /// Component health details
    pub components: HashMap<String, HealthState>,
}

/// Health state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthState {
    /// Plugin is healthy
    Healthy,
    /// Plugin is degraded but functional
    Degraded,
    /// Plugin is unhealthy
    Unhealthy,
    /// Plugin health is unknown
    Unknown,
}

/// Detailed health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Uptime in seconds
    pub uptime: u64,
    /// Total requests processed
    pub requests_total: u64,
    /// Failed requests
    pub requests_failed: u64,
    /// Average response time in milliseconds
    pub response_time_avg: f64,
    /// Memory usage in bytes
    pub memory_usage: usize,
    /// Custom metrics
    pub custom: HashMap<String, f64>,
}

/// Plugin extension points
#[async_trait]
pub trait PluginExtension: Send + Sync {
    /// Get extension type identifier
    fn extension_type(&self) -> &str;

    /// Execute extension functionality
    async fn execute(&self, input: ExtensionInput) -> Result<ExtensionOutput>;
}

/// Extension input data
#[derive(Debug, Clone)]
pub struct ExtensionInput {
    /// Operation to perform
    pub operation: String,
    /// Input parameters
    pub params: HashMap<String, serde_json::Value>,
    /// Optional schema context
    pub schema: Option<SchemaDefinition>,
    /// Optional data context
    pub data: Option<serde_json::Value>,
}

/// Extension output data
#[derive(Debug, Clone)]
pub struct ExtensionOutput {
    /// Success status
    pub success: bool,
    /// Result data
    pub result: Option<serde_json::Value>,
    /// Error message if failed
    pub error: Option<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Plugin SDK helper for plugin developers
pub struct PluginSDK;

impl PluginSDK {
    /// Create a new plugin builder
    pub fn builder() -> PluginBuilder {
        PluginBuilder::new()
    }

    /// Get current API version
    pub fn api_version() -> u32 {
        PLUGIN_API_VERSION
    }

    /// Create plugin metadata
    pub fn metadata() -> PluginMetadata {
        PluginMetadata {
            api_version: PLUGIN_API_VERSION,
            sdk_version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: chrono::Utc::now().to_rfc3339(),
            build_host: hostname::get().ok().and_then(|h| h.into_string().ok()),
            custom: HashMap::new(),
        }
    }
}

/// Plugin builder for easier plugin creation
pub struct PluginBuilder {
    info: PluginInfo,
    capabilities: Vec<PluginCapability>,
    config_schema: Option<ConfigSchema>,
}

impl PluginBuilder {
    /// Create a new plugin builder
    pub fn new() -> Self {
        Self {
            info: PluginInfo {
                id: String::new(),
                name: String::new(),
                description: String::new(),
                version: Version::new(0, 1, 0),
                plugin_type: PluginType::Generator,
                author: None,
                license: None,
                homepage: None,
                linkml_version: VersionReq::parse("*").expect("'*' is a valid version requirement"),
                dependencies: Vec::new(),
                capabilities: Vec::new(),
            },
            capabilities: Vec::new(),
            config_schema: None,
        }
    }

    /// Set plugin ID
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.info.id = id.into();
        self
    }

    /// Set plugin name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.info.name = name.into();
        self
    }

    /// Set plugin description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.info.description = description.into();
        self
    }

    /// Set plugin version
    pub fn version(mut self, major: u64, minor: u64, patch: u64) -> Self {
        self.info.version = Version::new(major, minor, patch);
        self
    }

    /// Set plugin type
    pub fn plugin_type(mut self, plugin_type: PluginType) -> Self {
        self.info.plugin_type = plugin_type;
        self
    }

    /// Add a capability
    pub fn capability(mut self, capability: PluginCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    /// Set configuration schema
    pub fn config_schema(mut self, schema: ConfigSchema) -> Self {
        self.config_schema = Some(schema);
        self
    }

    /// Build plugin info
    pub fn build(mut self) -> PluginInfo {
        self.info.capabilities = self.capabilities;
        self.info
    }
}

/// Macro for easy plugin exports (for native plugins)
#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn create_plugin() -> *mut dyn $crate::plugin::Plugin {
            let plugin = <$plugin_type>::new();
            Box::into_raw(Box::new(plugin) as Box<dyn $crate::plugin::Plugin>)
        }

        #[no_mangle]
        pub extern "C" fn plugin_api_version() -> u32 {
            $crate::plugin::PLUGIN_API_VERSION
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_builder() {
        let info = PluginSDK::builder()
            .id("test-plugin")
            .name("Test Plugin")
            .description("A test plugin")
            .version(1, 0, 0)
            .plugin_type(PluginType::Generator)
            .capability(PluginCapability::CodeGeneration)
            .capability(PluginCapability::AsyncOperations)
            .build();

        assert_eq!(info.id, "test-plugin");
        assert_eq!(info.capabilities.len(), 2);
        assert_eq!(info.version, Version::new(1, 0, 0));
    }

    #[test]
    fn test_health_status() {
        let status = HealthStatus {
            status: HealthState::Healthy,
            message: Some("All systems operational".to_string()),
            components: HashMap::new(),
        };

        assert_eq!(status.status, HealthState::Healthy);
        assert!(status.message.is_some());
    }
}
