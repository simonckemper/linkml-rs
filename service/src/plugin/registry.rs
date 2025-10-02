//! Plugin registry for managing loaded plugins
//!
//! This module provides a centralized registry for all loaded plugins
//! with support for lookup, lifecycle management, and dependency resolution.

use super::{
    LinkMLError, Plugin, PluginContext, PluginMetadata, PluginSDK, PluginStatus, PluginType,
    Result, Version, VersionReq,
};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};
use timestamp_core::TimestampService;

/// Plugin registry for managing all loaded plugins
pub struct PluginRegistry {
    /// Registered plugins by ID
    plugins: Arc<RwLock<HashMap<String, PluginRegistration>>>,
    /// Plugins by type
    by_type: Arc<RwLock<HashMap<PluginType, HashSet<String>>>>,
    /// Dependency graph
    dep_graph: Arc<RwLock<DiGraph<String, ()>>>,
    /// Node index mapping
    node_map: Arc<RwLock<HashMap<String, NodeIndex>>>,
    /// Timestamp service for registration timestamps
    timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
}

/// Plugin registration entry
pub struct PluginRegistration {
    /// The registered plugin instance wrapped for thread-safe access
    pub plugin: Arc<Mutex<Box<dyn Plugin>>>,
    /// Registration timestamp
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Initialization status
    pub initialized: bool,
    /// Plugin metadata
    pub metadata: PluginMetadata,
}

impl PluginRegistry {
    /// Create a new plugin registry
    #[must_use]
    pub fn new(
        timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    ) -> Self {
        Self {
            plugins: Arc::new(RwLock::new(HashMap::new())),
            by_type: Arc::new(RwLock::new(HashMap::new())),
            dep_graph: Arc::new(RwLock::new(DiGraph::new())),
            node_map: Arc::new(RwLock::new(HashMap::new())),
            timestamp_service,
        }
    }

    /// Register a plugin
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::ServiceError` if the plugin is already registered or the registry lock is poisoned
    pub async fn register(&self, plugin: Box<dyn Plugin>) -> Result<()> {
        let info = plugin.info();
        let id = info.id.clone();
        let plugin_type = info.plugin_type;

        // Check if already registered
        {
            let plugins = self.plugins.read().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            if plugins.contains_key(&id) {
                return Err(LinkMLError::ServiceError(format!(
                    "Plugin error: Plugin '{id}' is already registered"
                )));
            }
        }

        // Add to dependency graph
        let _node_idx = {
            let mut graph = self.dep_graph.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin dependency graph lock poisoned".to_string())
            })?;
            let mut node_map = self.node_map.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin node map lock poisoned".to_string())
            })?;

            let idx = graph.add_node(id.clone());
            node_map.insert(id.clone(), idx);

            // Add edges for dependencies
            for dep in &info.dependencies {
                if let Some(&dep_idx) = node_map.get(&dep.id) {
                    graph.add_edge(idx, dep_idx, ());
                } else if !dep.optional {
                    return Err(LinkMLError::ServiceError(format!(
                        "Plugin error: Required dependency '{}' not found for plugin '{}'",
                        dep.id, id
                    )));
                }
            }

            idx
        };

        // Create registration
        let registered_at = self.timestamp_service.now_utc().await.map_err(|e| {
            LinkMLError::ServiceError(format!("Failed to get current timestamp: {e}"))
        })?;
        let metadata = PluginSDK::metadata(self.timestamp_service.clone())
            .await
            .map_err(|e| {
                LinkMLError::ServiceError(format!("Failed to create plugin metadata: {e}"))
            })?;

        let registration = PluginRegistration {
            plugin: Arc::new(Mutex::new(plugin)),
            registered_at,
            initialized: false,
            metadata,
        };

        // Register the plugin
        {
            let mut plugins = self.plugins.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            plugins.insert(id.clone(), registration);
        }

        // Update type index
        {
            let mut by_type = self.by_type.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin type index lock poisoned".to_string())
            })?;
            by_type
                .entry(plugin_type)
                .or_insert_with(HashSet::new)
                .insert(id);
        }

        Ok(())
    }

    /// Unregister a plugin
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::ServiceError` if the plugin is not found or the registry lock is poisoned
    pub fn unregister(&self, id: &str) -> Result<()> {
        // Remove from main registry
        let registration = {
            let mut plugins = self.plugins.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            plugins.remove(id).ok_or_else(|| {
                LinkMLError::ServiceError(format!("Plugin error: Plugin '{id}' not found"))
            })?
        };

        // Remove from type index
        {
            let mut by_type = self.by_type.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin type index lock poisoned".to_string())
            })?;
            let plugin = registration
                .plugin
                .lock()
                .map_err(|_| LinkMLError::ServiceError("Plugin mutex poisoned".to_string()))?;
            let plugin_type = plugin.info().plugin_type;
            if let Some(type_set) = by_type.get_mut(&plugin_type) {
                type_set.remove(id);
                if type_set.is_empty() {
                    by_type.remove(&plugin_type);
                }
            }
        }

        // Remove from dependency graph
        {
            let mut graph = self.dep_graph.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin dependency graph lock poisoned".to_string())
            })?;
            let mut node_map = self.node_map.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin node map lock poisoned".to_string())
            })?;

            if let Some(idx) = node_map.remove(id) {
                graph.remove_node(idx);
            }
        }

        Ok(())
    }

    /// Get a plugin by ID
    #[must_use]
    pub fn get(&self, id: &str) -> Option<Arc<Mutex<Box<dyn Plugin>>>> {
        let plugins = self.plugins.read().ok()?;
        plugins.get(id).map(|reg| Arc::clone(&reg.plugin))
    }

    /// Get all plugins of a specific type
    #[must_use]
    pub fn get_by_type(&self, plugin_type: PluginType) -> Vec<Arc<Mutex<Box<dyn Plugin>>>> {
        let Ok(by_type) = self.by_type.read() else {
            return Vec::new();
        };
        let Ok(plugins) = self.plugins.read() else {
            return Vec::new();
        };

        by_type
            .get(&plugin_type)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| plugins.get(id).map(|reg| Arc::clone(&reg.plugin)))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all registered plugins
    #[must_use]
    pub fn get_all(&self) -> Vec<Arc<Mutex<Box<dyn Plugin>>>> {
        let Ok(plugins) = self.plugins.read() else {
            return Vec::new();
        };
        plugins
            .values()
            .map(|reg| Arc::clone(&reg.plugin))
            .collect()
    }

    /// Get plugin registration info
    #[must_use]
    pub fn get_registration(&self, id: &str) -> Option<PluginRegistrationInfo> {
        let plugins = self.plugins.read().ok()?;
        plugins.get(id).and_then(|reg| {
            let plugin = reg.plugin.lock().ok()?;
            Some(PluginRegistrationInfo {
                id: id.to_string(),
                plugin_type: plugin.info().plugin_type,
                version: plugin.info().version.clone(),
                registered_at: reg.registered_at,
                initialized: reg.initialized,
                status: plugin.status(),
            })
        })
    }

    /// Initialize all plugins in dependency order
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn initialize_all(&self, context: PluginContext) -> Result<()> {
        // Get initialization order
        let init_order = self.get_initialization_order()?;

        // Initialize plugins in order
        for id in init_order {
            self.initialize_plugin(&id, context.clone()).await?;
        }

        Ok(())
    }

    /// Initialize a specific plugin
    async fn initialize_plugin(&self, id: &str, context: PluginContext) -> Result<()> {
        let plugin = {
            let plugins = self.plugins.read().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            plugins
                .get(id)
                .map(|reg| Arc::clone(&reg.plugin))
                .ok_or_else(|| {
                    LinkMLError::ServiceError(format!("Plugin error: Plugin '{id}' not found"))
                })?
        };

        // Initialize the plugin
        // Note: Using std::sync::Mutex across await points is not ideal, but required here
        // due to the plugin trait design. Consider using tokio::sync::Mutex in future versions.
        #[allow(clippy::await_holding_lock)]
        {
            let mut plugin_guard = plugin
                .lock()
                .map_err(|_| LinkMLError::ServiceError("Plugin mutex poisoned".to_string()))?;
            plugin_guard.initialize(context).await?;
        }

        // Mark as initialized
        {
            let mut plugins = self.plugins.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            if let Some(reg) = plugins.get_mut(id) {
                reg.initialized = true;
            }
        }

        Ok(())
    }

    /// Shutdown all plugins in reverse dependency order
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn shutdown_all(&self) -> Result<()> {
        // Get initialization order and reverse it
        let mut shutdown_order = self.get_initialization_order()?;
        shutdown_order.reverse();

        // Shutdown plugins in order
        for id in shutdown_order {
            self.shutdown_plugin(&id).await?;
        }

        Ok(())
    }

    /// Shutdown a specific plugin
    async fn shutdown_plugin(&self, id: &str) -> Result<()> {
        let plugin = {
            let plugins = self.plugins.read().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            plugins
                .get(id)
                .map(|reg| Arc::clone(&reg.plugin))
                .ok_or_else(|| {
                    LinkMLError::ServiceError(format!("Plugin error: Plugin '{id}' not found"))
                })?
        };

        // Shutdown the plugin
        // Note: Using std::sync::Mutex across await points is not ideal, but required here
        // due to the plugin trait design. Consider using tokio::sync::Mutex in future versions.
        #[allow(clippy::await_holding_lock)]
        {
            let mut plugin_guard = plugin
                .lock()
                .map_err(|_| LinkMLError::ServiceError("Plugin mutex poisoned".to_string()))?;
            plugin_guard.shutdown().await?;
        }

        // Mark as not initialized
        {
            let mut plugins = self.plugins.write().map_err(|_| {
                LinkMLError::ServiceError("Plugin registry lock poisoned".to_string())
            })?;
            if let Some(reg) = plugins.get_mut(id) {
                reg.initialized = false;
            }
        }

        Ok(())
    }

    /// Get plugin initialization order based on dependencies
    fn get_initialization_order(&self) -> Result<Vec<String>> {
        let graph = self.dep_graph.read().map_err(|_| {
            LinkMLError::ServiceError("Plugin dependency graph lock poisoned".to_string())
        })?;
        let node_map = self
            .node_map
            .read()
            .map_err(|_| LinkMLError::ServiceError("Plugin node map lock poisoned".to_string()))?;

        // Perform topological sort
        match toposort(&*graph, None) {
            Ok(order) => {
                // Map node indices back to plugin IDs
                let mut id_order = Vec::new();
                for idx in order {
                    if let Some(id) = node_map
                        .iter()
                        .find(|&(_, &i)| i == idx)
                        .map(|(id, _)| id.clone())
                    {
                        id_order.push(id);
                    }
                }
                Ok(id_order)
            }
            Err(_) => Err(LinkMLError::ServiceError(
                "Plugin error: Circular dependency detected in plugins".to_string(),
            )),
        }
    }

    /// Check if all plugin dependencies are satisfied
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::ServiceError` if the registry lock is poisoned
    pub fn check_dependencies(&self) -> Result<Vec<DependencyError>> {
        let mut errors = Vec::new();
        let plugins = self
            .plugins
            .read()
            .map_err(|_| LinkMLError::ServiceError("Plugin registry lock poisoned".to_string()))?;

        for (id, reg) in plugins.iter() {
            let plugin = match reg.plugin.lock() {
                Ok(p) => p,
                Err(_) => continue, // Skip if mutex is poisoned
            };
            let info = plugin.info();

            for dep in &info.dependencies {
                if !dep.optional && !plugins.contains_key(&dep.id) {
                    errors.push(DependencyError {
                        plugin_id: id.clone(),
                        dependency_id: dep.id.clone(),
                        required_version: dep.version.clone(),
                        found_version: None,
                        reason: "Dependency not found".to_string(),
                    });
                } else if let Some(dep_reg) = plugins.get(&dep.id)
                    && let Ok(dep_plugin) = dep_reg.plugin.lock()
                {
                    let dep_version = &dep_plugin.info().version;
                    if !dep.version.matches(dep_version) {
                        errors.push(DependencyError {
                            plugin_id: id.clone(),
                            dependency_id: dep.id.clone(),
                            required_version: dep.version.clone(),
                            found_version: Some(dep_version.clone()),
                            reason: "Version mismatch".to_string(),
                        });
                    }
                }
            }
        }

        Ok(errors)
    }
}

/// Plugin registration information
#[derive(Debug, Clone)]
pub struct PluginRegistrationInfo {
    /// Plugin ID
    pub id: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Plugin version
    pub version: Version,
    /// Registration timestamp
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Whether the plugin is initialized
    pub initialized: bool,
    /// Current plugin status
    pub status: PluginStatus,
}

/// Dependency error information
#[derive(Debug, Clone)]
pub struct DependencyError {
    /// Plugin with the dependency
    pub plugin_id: String,
    /// Missing or incompatible dependency
    pub dependency_id: String,
    /// Required version
    pub required_version: VersionReq,
    /// Found version (if any)
    pub found_version: Option<Version>,
    /// Error reason
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::PluginInfo;
    use async_trait::async_trait;

    // Mock plugin for testing
    struct MockPlugin {
        info: PluginInfo,
    }

    #[async_trait]
    impl Plugin for MockPlugin {
        fn info(&self) -> &PluginInfo {
            &self.info
        }

        async fn initialize(&mut self, _context: PluginContext) -> Result<()> {
            Ok(())
        }

        async fn shutdown(&mut self) -> Result<()> {
            Ok(())
        }

        fn validate_config(&self, _config: &HashMap<String, serde_json::Value>) -> Result<()> {
            Ok(())
        }

        fn status(&self) -> PluginStatus {
            PluginStatus::Ready
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }

    // Mock logger for testing
    struct TestLogger;

    #[async_trait]
    impl logger_core::LoggerService for TestLogger {
        type Error = logger_core::LoggerError;

        async fn debug(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn info(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn warn(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn error(&self, _message: &str) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn log(
            &self,
            _level: logger_core::LogLevel,
            _message: &str,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn log_entry(
            &self,
            _entry: &logger_core::LogEntry,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn set_level(
            &self,
            _level: logger_core::LogLevel,
        ) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn flush(&self) -> std::result::Result<(), Self::Error> {
            Ok(())
        }

        async fn shutdown(&self) -> std::result::Result<(), Self::Error> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_plugin_registration() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let timestamp_service = timestamp_service::factory::create_timestamp_service();
        let registry = PluginRegistry::new(timestamp_service);

        let plugin = Box::new(MockPlugin {
            info: PluginInfo {
                id: "test-plugin".to_string(),
                name: "Test Plugin".to_string(),
                description: "Test".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Generator,
                author: None,
                license: None,
                homepage: None,
                linkml_version: VersionReq::parse("*")?,
                dependencies: vec![],
                capabilities: vec![],
            },
        });

        registry.register(plugin).await?;

        assert!(registry.get("test-plugin").is_some());
        assert_eq!(registry.get_by_type(PluginType::Generator).len(), 1);

        // Test that we can access the plugin through the mutex
        if let Some(plugin_mutex) = registry.get("test-plugin") {
            let plugin = plugin_mutex
                .lock()
                .expect("plugin mutex should not be poisoned");
            assert_eq!(plugin.info().id, "test-plugin");
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_plugin_lifecycle() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let timestamp_service = timestamp_service::factory::create_timestamp_service();
        let registry = PluginRegistry::new(timestamp_service);

        let plugin = Box::new(MockPlugin {
            info: PluginInfo {
                id: "lifecycle-test".to_string(),
                name: "Lifecycle Test Plugin".to_string(),
                description: "Test".to_string(),
                version: Version::new(1, 0, 0),
                plugin_type: PluginType::Generator,
                author: None,
                license: None,
                homepage: None,
                linkml_version: VersionReq::parse("*")?,
                dependencies: vec![],
                capabilities: vec![],
            },
        });

        registry.register(plugin).await?;

        // Create a test context
        let context = PluginContext {
            config: HashMap::new(),
            working_dir: std::env::current_dir()?,
            temp_dir: std::env::temp_dir(),
            logger: Arc::new(TestLogger),
        };

        // Test initialization
        registry.initialize_all(context.clone()).await?;

        // Verify plugin is initialized
        let reg_info = registry
            .get_registration("lifecycle-test")
            .ok_or("Plugin not found")?;
        assert!(reg_info.initialized);

        // Test shutdown
        registry.shutdown_all().await?;

        // Verify plugin is shutdown
        let reg_info = registry
            .get_registration("lifecycle-test")
            .ok_or("Plugin not found")?;
        assert!(!reg_info.initialized);
        Ok(())
    }
}
