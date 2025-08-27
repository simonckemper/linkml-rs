//! Plugin system for custom generators

use super::registry::GeneratorRegistry;
use super::traits::{Generator, GeneratorError, GeneratorResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

/// Plugin error types
#[derive(Debug, Error)]
pub enum PluginError {
    /// Plugin loading failed
    #[error("Failed to load plugin: {0}")]
    LoadError(String),

    /// Plugin initialization failed
    #[error("Plugin initialization failed: {0}")]
    InitError(String),

    /// Plugin configuration error
    #[error("Plugin configuration error: {0}")]
    ConfigError(String),

    /// Plugin not found
    #[error("Plugin not found: {0}")]
    NotFound(String),
}

/// Plugin metadata
#[derive(Debug, Clone)]
pub struct PluginMetadata {
    /// Plugin name
    pub name: String,

    /// Plugin version
    pub version: String,

    /// Plugin description
    pub description: String,

    /// Plugin author
    pub author: String,

    /// Generator names provided by this plugin
    pub generators: Vec<String>,
}

/// Plugin configuration
#[derive(Debug, Clone)]
pub struct PluginConfig {
    /// Plugin directory path
    pub plugin_dir: PathBuf,

    /// Enable plugin auto-discovery
    pub auto_discover: bool,

    /// Plugin-specific configuration
    pub settings: HashMap<String, serde_json::Value>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            plugin_dir: PathBuf::from("./plugins"),
            auto_discover: true,
            settings: HashMap::new(),
        }
    }
}

/// Generator plugin trait
#[async_trait]
pub trait GeneratorPlugin: Send + Sync {
    /// Get plugin metadata
    fn metadata(&self) -> &PluginMetadata;

    /// Initialize the plugin
    async fn initialize(&mut self, config: &PluginConfig) -> Result<(), PluginError>;

    /// Register generators with the registry
    async fn register_generators(&self, registry: &Arc<GeneratorRegistry>) -> GeneratorResult<()>;

    /// Shutdown the plugin
    async fn shutdown(&mut self) -> Result<(), PluginError>;
}

/// Plugin manager for loading and managing generator plugins
pub struct PluginManager {
    /// Plugin configuration
    config: PluginConfig,

    /// Loaded plugins
    plugins: HashMap<String, Box<dyn GeneratorPlugin>>,

    /// Generator registry
    registry: Arc<GeneratorRegistry>,
}

impl PluginManager {
    /// Create a new plugin manager
    pub fn new(config: PluginConfig, registry: Arc<GeneratorRegistry>) -> Self {
        Self {
            config,
            plugins: HashMap::new(),
            registry,
        }
    }

    /// Load all plugins from the plugin directory
    pub async fn load_plugins(&mut self) -> GeneratorResult<()> {
        if self.config.auto_discover {
            self.discover_plugins()?;
        }

        Ok(())
    }

    /// Discover plugins in the plugin directory
    fn discover_plugins(&mut self) -> GeneratorResult<()> {
        let plugin_dir = &self.config.plugin_dir;

        if !plugin_dir.exists() {
            // Create plugin directory if it doesn't exist
            std::fs::create_dir_all(plugin_dir).map_err(|e| {
                GeneratorError::Plugin(format!("Failed to create plugin directory: {e}"))
            })?;
            return Ok(());
        }

        // For now, we'll support loading plugins from shared libraries
        // In a real implementation, this would use dynamic loading
        // For RootReal's purposes, we'll use a registration approach

        Ok(())
    }

    /// Register a plugin manually
    pub async fn register_plugin(
        &mut self,
        mut plugin: Box<dyn GeneratorPlugin>,
    ) -> GeneratorResult<()> {
        let metadata = plugin.metadata();
        let name = metadata.name.clone();

        // Initialize the plugin
        plugin.initialize(&self.config).await.map_err(|e| {
            GeneratorError::Plugin(format!("Plugin {name} initialization failed: {e}"))
        })?;

        // Register generators
        plugin.register_generators(&self.registry).await?;

        // Store the plugin
        self.plugins.insert(name, plugin);

        Ok(())
    }

    /// Get a loaded plugin by name
    #[must_use]
    pub fn get_plugin(&self, name: &str) -> Option<&dyn GeneratorPlugin> {
        self.plugins.get(name).map(std::convert::AsRef::as_ref)
    }

    /// List all loaded plugins
    #[must_use]
    pub fn list_plugins(&self) -> Vec<&PluginMetadata> {
        self.plugins.values().map(|p| p.metadata()).collect()
    }

    /// Shutdown all plugins
    pub async fn shutdown(&mut self) -> GeneratorResult<()> {
        for (name, mut plugin) in self.plugins.drain() {
            if let Err(e) = plugin.shutdown().await {
                eprintln!("Failed to shutdown plugin {name}: {e}");
            }
        }

        Ok(())
    }
}

/// Example custom generator plugin
pub struct CustomGeneratorPlugin {
    metadata: PluginMetadata,
    generators: Vec<Arc<dyn Generator>>,
}

impl CustomGeneratorPlugin {
    /// Create a new custom generator plugin
    #[must_use]
    pub fn new(name: String, version: String, generators: Vec<Arc<dyn Generator>>) -> Self {
        let generator_names: Vec<String> =
            generators.iter().map(|g| g.name().to_string()).collect();

        Self {
            metadata: PluginMetadata {
                name,
                version,
                description: "Custom generator plugin".to_string(),
                author: "RootReal".to_string(),
                generators: generator_names,
            },
            generators,
        }
    }
}

#[async_trait]
impl GeneratorPlugin for CustomGeneratorPlugin {
    fn metadata(&self) -> &PluginMetadata {
        &self.metadata
    }

    async fn initialize(&mut self, _config: &PluginConfig) -> Result<(), PluginError> {
        // Perform any initialization needed
        Ok(())
    }

    async fn register_generators(&self, registry: &Arc<GeneratorRegistry>) -> GeneratorResult<()> {
        for generator in &self.generators {
            registry.register(generator.clone()).await?;
        }

        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), PluginError> {
        // Perform any cleanup needed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::{GeneratedOutput, GeneratorOptions};
    use linkml_core::prelude::*;

    /// Test generator for plugin system
    struct TestGenerator {
        name: String,
    }

    #[async_trait]
    impl Generator for TestGenerator {
        fn name(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &'static str {
            "Test generator for plugin system"
        }

        fn file_extensions(&self) -> Vec<&str> {
            vec![".test"]
        }

        fn generate(&self, _schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
            Ok(String::new())
        }

        fn get_file_extension(&self) -> &str {
            "test"
        }

        fn get_default_filename(&self) -> &str {
            "test.test"
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() {
        let registry = Arc::new(GeneratorRegistry::new());
        let config = PluginConfig::default();
        let mut manager = PluginManager::new(config, registry.clone());

        // Create a test generator
        let test_gen = Arc::new(TestGenerator {
            name: "test".to_string(),
        });

        // Create a plugin with the test generator
        let plugin = Box::new(CustomGeneratorPlugin::new(
            "test-plugin".to_string(),
            "1.0.0".to_string(),
            vec![test_gen],
        ));

        // Register the plugin
        manager
            .register_plugin(plugin)
            .await
            .map_err(|e| anyhow::anyhow!("should register plugin": {}, e))?;

        // Verify plugin is loaded
        assert_eq!(manager.list_plugins().len(), 1);

        // Verify generator is registered
        let generators = registry.list_generators().await;
        assert!(generators.iter().any(|name| name == "test"));
    }
}
