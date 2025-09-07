//! Built-in plugins for LinkML service
//!
//! Since dynamic plugin loading is forbidden by RootReal's safety requirements,
//! all plugins must be compiled into the application at build time.

use super::*;
use async_trait::async_trait;
use linkml_core::error::Result;
use semver::{Version, VersionReq};
use serde_json::Value;
use std::collections::HashMap;

/// Registry of built-in plugins
pub struct BuiltinPluginRegistry {
    plugins: HashMap<String, Box<dyn Plugin>>,
}

impl BuiltinPluginRegistry {
    /// Create a new registry with all built-in plugins
    pub fn new() -> Self {
        let mut registry = Self {
            plugins: HashMap::new(),
        };

        // Register all built-in plugins
        registry.register_builtin_plugins();
        registry
    }

    /// Register all built-in plugins
    fn register_builtin_plugins(&mut self) {
        // Register JSON Schema generator plugin
        self.plugins.insert(
            "json-schema-generator".to_string(),
            Box::new(JsonSchemaGeneratorPlugin::new()),
        );

        // Register SQL generator plugin
        self.plugins.insert(
            "sql-generator".to_string(),
            Box::new(SqlGeneratorPlugin::new()),
        );

        // Register TypeQL generator plugin
        self.plugins.insert(
            "typeql-generator".to_string(),
            Box::new(TypeQLGeneratorPlugin::new()),
        );

        // Register validation plugin
        self.plugins.insert(
            "enhanced-validator".to_string(),
            Box::new(EnhancedValidatorPlugin::new()),
        );
    }

    /// Get a plugin by name
    pub fn get_plugin(&self, name: &str) -> Option<&Box<dyn Plugin>> {
        self.plugins.get(name)
    }

    /// Get mutable plugin by name
    pub fn get_plugin_mut(&mut self, name: &str) -> Option<&mut Box<dyn Plugin>> {
        self.plugins.get_mut(name)
    }

    /// List all available plugins
    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }
}

/// `JSON` Schema generator plugin
struct JsonSchemaGeneratorPlugin {
    info: PluginInfo,
    status: PluginStatus,
}

impl JsonSchemaGeneratorPlugin {
    fn new() -> Self {
        Self {
            info: PluginInfo {
                id: "json-schema-generator".to_string(),
                name: "JSON Schema Generator".to_string(),
                description: "Generate JSON Schema from LinkML schemas".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Generator,
                author: Some("RootReal Team".to_string()),
                license: Some("CC BY-NC 4.0".to_string()),
                homepage: None,
                linkml_version: VersionReq::parse(">=1.0.0")
                    .expect("Valid version requirement"),
                dependencies: vec![],
                capabilities: vec![PluginCapability::CodeGeneration],
            },
            status: PluginStatus::Uninitialized,
        }
    }
}

#[async_trait]
impl Plugin for JsonSchemaGeneratorPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
        self.status = PluginStatus::Ready;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.status = PluginStatus::Shutdown;
        Ok(())
    }

    fn validate_config(&self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        self.status
    }
}

/// `SQL` generator plugin
struct SqlGeneratorPlugin {
    info: PluginInfo,
    status: PluginStatus,
}

impl SqlGeneratorPlugin {
    fn new() -> Self {
        Self {
            info: PluginInfo {
                id: "sql-generator".to_string(),
                name: "SQL Generator".to_string(),
                description: "Generate SQL DDL from LinkML schemas".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Generator,
                author: Some("RootReal Team".to_string()),
                license: Some("CC BY-NC 4.0".to_string()),
                homepage: None,
                linkml_version: VersionReq::parse(">=1.0.0")
                    .expect("Valid version requirement"),
                dependencies: vec![],
                capabilities: vec![PluginCapability::CodeGeneration],
            },
            status: PluginStatus::Uninitialized,
        }
    }
}

#[async_trait]
impl Plugin for SqlGeneratorPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
        self.status = PluginStatus::Ready;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.status = PluginStatus::Shutdown;
        Ok(())
    }

    fn validate_config(&self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        self.status
    }
}

/// TypeQL generator plugin
struct TypeQLGeneratorPlugin {
    info: PluginInfo,
    status: PluginStatus,
}

impl TypeQLGeneratorPlugin {
    fn new() -> Self {
        Self {
            info: PluginInfo {
                id: "typeql-generator".to_string(),
                name: "TypeQL Generator".to_string(),
                description: "Generate TypeQL schema from LinkML schemas".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Generator,
                author: Some("RootReal Team".to_string()),
                license: Some("CC BY-NC 4.0".to_string()),
                homepage: None,
                linkml_version: VersionReq::parse(">=1.0.0")
                    .expect("Valid version requirement"),
                dependencies: vec![],
                capabilities: vec![PluginCapability::CodeGeneration],
            },
            status: PluginStatus::Uninitialized,
        }
    }
}

#[async_trait]
impl Plugin for TypeQLGeneratorPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
        self.status = PluginStatus::Ready;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.status = PluginStatus::Shutdown;
        Ok(())
    }

    fn validate_config(&self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        self.status
    }
}

/// Enhanced validator plugin
struct EnhancedValidatorPlugin {
    info: PluginInfo,
    status: PluginStatus,
}

impl EnhancedValidatorPlugin {
    fn new() -> Self {
        Self {
            info: PluginInfo {
                id: "enhanced-validator".to_string(),
                name: "Enhanced Validator".to_string(),
                description: "Enhanced validation with custom rules".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Validator,
                author: Some("RootReal Team".to_string()),
                license: Some("CC BY-NC 4.0".to_string()),
                homepage: None,
                linkml_version: VersionReq::parse(">=1.0.0")
                    .expect("Valid version requirement"),
                dependencies: vec![],
                capabilities: vec![PluginCapability::DataValidation],
            },
            status: PluginStatus::Uninitialized,
        }
    }
}

#[async_trait]
impl Plugin for EnhancedValidatorPlugin {
    fn info(&self) -> &PluginInfo {
        &self.info
    }

    async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
        self.status = PluginStatus::Ready;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.status = PluginStatus::Shutdown;
        Ok(())
    }

    fn validate_config(&self, _config: &HashMap<String, Value>) -> Result<()> {
        Ok(())
    }

    fn status(&self) -> PluginStatus {
        self.status
    }
}
