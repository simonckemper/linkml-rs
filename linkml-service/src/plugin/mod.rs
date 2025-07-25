//! Plugin architecture for LinkML
//!
//! This module provides a comprehensive plugin system for extending LinkML functionality
//! with custom generators, validators, loaders, and other components.

use async_trait::async_trait;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use linkml_core::prelude::*;
use linkml_core::error::{Result, LinkMLError};

pub mod discovery;
pub mod loader;
pub mod api;
pub mod registry;
pub mod compatibility;

pub use discovery::{PluginDiscovery, DiscoveryStrategy};
pub use loader::{PluginLoader, DynamicLoader};
pub use api::{PluginApi, PluginCapability, PluginMetadata};
pub use registry::{PluginRegistry, PluginRegistration};
pub use compatibility::{VersionCompatibility, CompatibilityChecker};

/// Plugin type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginType {
    /// Code generator plugin
    Generator,
    /// Data validator plugin
    Validator,
    /// Data loader plugin
    Loader,
    /// Data dumper plugin
    Dumper,
    /// Schema transformer plugin
    Transformer,
    /// Custom function provider
    Function,
    /// Analysis tool plugin
    Analyzer,
}

/// Plugin information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Unique plugin identifier
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Plugin description
    pub description: String,
    /// Plugin version
    pub version: Version,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Author information
    pub author: Option<String>,
    /// License
    pub license: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Required LinkML version
    pub linkml_version: VersionReq,
    /// Plugin dependencies
    pub dependencies: Vec<PluginDependency>,
    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,
}

/// Plugin dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// Dependency plugin ID
    pub id: String,
    /// Required version range
    pub version: VersionReq,
    /// Whether the dependency is optional
    pub optional: bool,
}

/// Plugin context for execution
#[derive(Debug, Clone)]
pub struct PluginContext {
    /// Plugin configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Working directory
    pub working_dir: PathBuf,
    /// Temporary directory
    pub temp_dir: PathBuf,
    /// Logger service
    pub logger: Arc<dyn LoggerService<Error = LoggerError>>,
}

/// Core plugin trait
#[async_trait]
pub trait Plugin: Send + Sync {
    /// Get plugin information
    fn info(&self) -> &PluginInfo;
    
    /// Initialize the plugin
    async fn initialize(&mut self, context: PluginContext) -> Result<()>;
    
    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<()>;
    
    /// Validate plugin configuration
    fn validate_config(&self, config: &HashMap<String, serde_json::Value>) -> Result<()>;
    
    /// Get plugin status
    fn status(&self) -> PluginStatus;
}

/// Plugin status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginStatus {
    /// Plugin is not initialized
    Uninitialized,
    /// Plugin is initializing
    Initializing,
    /// Plugin is ready
    Ready,
    /// Plugin encountered an error
    Error,
    /// Plugin is shutting down
    ShuttingDown,
    /// Plugin is shut down
    Shutdown,
}

/// Generator plugin trait
#[async_trait]
pub trait GeneratorPlugin: Plugin {
    /// Get supported output formats
    fn supported_formats(&self) -> Vec<String>;
    
    /// Generate code from schema
    async fn generate(
        &self,
        schema: &Schema,
        format: &str,
        options: HashMap<String, serde_json::Value>,
    ) -> Result<String>;
    
    /// Get generator-specific options
    fn options_schema(&self) -> serde_json::Value;
}

/// Validator plugin trait
#[async_trait]
pub trait ValidatorPlugin: Plugin {
    /// Validate data against schema
    async fn validate(
        &self,
        schema: &Schema,
        data: &serde_json::Value,
        options: HashMap<String, serde_json::Value>,
    ) -> Result<ValidationResult>;
    
    /// Get validator-specific options
    fn options_schema(&self) -> serde_json::Value;
}

/// Loader plugin trait
#[async_trait]
pub trait LoaderPlugin: Plugin {
    /// Get supported input formats
    fn supported_formats(&self) -> Vec<String>;
    
    /// Load data from source
    async fn load(
        &self,
        source: &str,
        format: &str,
        options: HashMap<String, serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>>;
    
    /// Get loader-specific options
    fn options_schema(&self) -> serde_json::Value;
}

/// Dumper plugin trait
#[async_trait]
pub trait DumperPlugin: Plugin {
    /// Get supported output formats
    fn supported_formats(&self) -> Vec<String>;
    
    /// Dump data to destination
    async fn dump(
        &self,
        data: &[serde_json::Value],
        destination: &str,
        format: &str,
        options: HashMap<String, serde_json::Value>,
    ) -> Result<()>;
    
    /// Get dumper-specific options
    fn options_schema(&self) -> serde_json::Value;
}

/// Function plugin trait for custom expression functions
#[async_trait]
pub trait FunctionPlugin: Plugin {
    /// Get provided function names
    fn function_names(&self) -> Vec<String>;
    
    /// Execute a function
    async fn execute(
        &self,
        function: &str,
        args: Vec<serde_json::Value>,
    ) -> Result<serde_json::Value>;
    
    /// Get function signatures
    fn signatures(&self) -> HashMap<String, FunctionSignature>;
}

/// Function signature definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSignature {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Parameter definitions
    pub parameters: Vec<ParameterDef>,
    /// Return type
    pub return_type: String,
    /// Whether the function is variadic
    pub variadic: bool,
}

/// Parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: String,
    /// Whether the parameter is optional
    pub optional: bool,
    /// Default value if optional
    pub default: Option<serde_json::Value>,
}

/// Validation result from plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validation errors
    pub errors: Vec<ValidationError>,
    /// Validation warnings
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    /// JSON path to the error
    pub path: Option<String>,
    /// Error code
    pub code: Option<String>,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    /// JSON path to the warning
    pub path: Option<String>,
    /// Warning code
    pub code: Option<String>,
}

/// Plugin manager for coordinating all plugins
pub struct PluginManager {
    /// Plugin registry
    registry: PluginRegistry,
    /// Plugin loader
    loader: DynamicLoader,
    /// Discovery service
    discovery: PluginDiscovery,
    /// Compatibility checker
    compatibility: CompatibilityChecker,
    /// Logger service
    logger: Arc<dyn LoggerService<Error = LoggerError>>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(logger: Arc<dyn LoggerService<Error = LoggerError>>) -> Self {
        Self {
            registry: PluginRegistry::new(),
            loader: DynamicLoader::new(),
            discovery: PluginDiscovery::new(),
            compatibility: CompatibilityChecker::new(),
            logger,
        }
    }
    
    /// Discover and load plugins from a directory
    pub async fn discover_plugins(&mut self, path: &Path) -> Result<Vec<PluginInfo>> {
        let plugin_paths = self.discovery.discover(path, DiscoveryStrategy::Recursive)?;
        let mut loaded_plugins = Vec::new();
        
        for plugin_path in plugin_paths {
            match self.load_plugin(&plugin_path).await {
                Ok(info) => loaded_plugins.push(info),
                Err(e) => {
                    self.logger.log_warning(&format!(
                        "Failed to load plugin from {:?}: {}",
                        plugin_path, e
                    )).await?;
                }
            }
        }
        
        Ok(loaded_plugins)
    }
    
    /// Load a specific plugin
    pub async fn load_plugin(&mut self, path: &Path) -> Result<PluginInfo> {
        // Load plugin metadata
        let metadata = self.loader.load_metadata(path)?;
        
        // Check compatibility
        self.compatibility.check_compatibility(&metadata)?;
        
        // Load the plugin
        let plugin = self.loader.load_plugin(path, &metadata).await?;
        let info = plugin.info().clone();
        
        // Register the plugin
        self.registry.register(plugin)?;
        
        Ok(info)
    }
    
    /// Get a plugin by ID
    pub fn get_plugin(&self, id: &str) -> Option<Arc<dyn Plugin>> {
        self.registry.get(id)
    }
    
    /// Get all plugins of a specific type
    pub fn get_plugins_by_type(&self, plugin_type: PluginType) -> Vec<Arc<dyn Plugin>> {
        self.registry.get_by_type(plugin_type)
    }
    
    /// Initialize all plugins
    pub async fn initialize_all(&mut self, context: PluginContext) -> Result<()> {
        self.registry.initialize_all(context).await
    }
    
    /// Shutdown all plugins
    pub async fn shutdown_all(&mut self) -> Result<()> {
        self.registry.shutdown_all().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_plugin_info_serialization() {
        let info = PluginInfo {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            description: "A test plugin".to_string(),
            version: Version::new(1, 0, 0),
            plugin_type: PluginType::Generator,
            author: Some("Test Author".to_string()),
            license: Some("MIT".to_string()),
            homepage: None,
            linkml_version: VersionReq::parse(">=1.0.0").expect("valid version requirement"),
            dependencies: vec![],
            capabilities: vec![],
        };
        
        let json = serde_json::to_string(&info).expect("should serialize PluginInfo");
        let deserialized: PluginInfo = serde_json::from_str(&json).expect("should deserialize PluginInfo");
        
        assert_eq!(info.id, deserialized.id);
        assert_eq!(info.version, deserialized.version);
    }
}