//! Integration with `RootReal`'s Configuration Service
//!
//! This module replaces the standalone hot-reload implementation with proper
//! Configuration Service integration, following `RootReal`'s architectural patterns.

use super::LinkMLConfig as LinkMLServiceConfig; // Use the local config type
use async_trait::async_trait;
use configuration_core::{ConfigurationService, Validate};
use linkml_core::error::{LinkMLError, Result};
use serde::{Serialize, de::DeserializeOwned};
#[allow(unused_imports)] // False positive - used in Debug impl for MigrationResult
use std::fmt;
use std::sync::Arc;
use tokio::sync::{RwLock, watch};
use tracing::{error, info, warn};

/// Configuration manager that integrates with `RootReal`'s Configuration Service
///
/// This replaces the standalone `ConfigHotReloader` and properly uses
/// the Configuration Service for all configuration management including hot-reload.
pub struct ConfigurationManager<C: ConfigurationService + Send + Sync + 'static> {
    /// The Configuration Service instance
    config_service: Arc<C>,
    /// Current configuration cached
    current_config: Arc<RwLock<LinkMLServiceConfig>>,
    /// Configuration change notifier
    config_tx: watch::Sender<Arc<LinkMLServiceConfig>>,
    /// Configuration change receiver
    config_rx: watch::Receiver<Arc<LinkMLServiceConfig>>,
}

impl<C: ConfigurationService + Send + Sync + 'static> ConfigurationManager<C> {
    /// Create a new configuration manager
    ///
    /// # Errors
    ///
    /// Returns error if configuration cannot be loaded or validated.
    pub async fn new(config_service: Arc<C>) -> Result<Self> {
        // Load initial configuration from Configuration Service
        let config: LinkMLServiceConfig = config_service
            .load_configuration()
            .await
            .map_err(|e| LinkMLError::config(format!("Failed to load configuration: {e}")))?;

        // Validate the configuration
        config
            .validate()
            .map_err(|e| LinkMLError::config(format!("Configuration validation failed: {e}")))?;

        let config_arc = Arc::new(config);
        let (tx, rx) = watch::channel(Arc::clone(&config_arc));

        Ok(Self {
            config_service,
            current_config: Arc::new(RwLock::new((*config_arc).clone())),
            config_tx: tx,
            config_rx: rx,
        })
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> Arc<LinkMLServiceConfig> {
        let config = self.current_config.read().await;
        Arc::new(config.clone())
    }

    /// Get a configuration value by key
    ///
    /// # Errors
    /// Returns error if configuration key is not found or value cannot be deserialized.
    pub async fn get_value<T>(&self, key: &str) -> Result<T>
    where
        T: DeserializeOwned + Serialize + Clone + Send + Sync + 'static,
    {
        self.config_service
            .get_configuration(key)
            .await
            .map_err(|e| LinkMLError::config(format!("Failed to get config value '{key}': {e}")))
    }

    /// Reload configuration from the Configuration Service
    ///
    /// # Errors
    /// Returns error if configuration service reload fails.
    pub async fn reload(&self) -> Result<()> {
        info!("Reloading LinkML configuration from Configuration Service");

        // Trigger reload in Configuration Service
        self.config_service
            .reload_configuration()
            .await
            .map_err(|e| LinkMLError::config(format!("Failed to reload configuration: {e}")))?;

        // Load the new configuration
        let new_config: LinkMLServiceConfig = self
            .config_service
            .load_configuration()
            .await
            .map_err(|e| LinkMLError::config(format!("Failed to load new configuration: {e}")))?;

        // Validate the new configuration
        new_config.validate().map_err(|e| {
            LinkMLError::config(format!("New configuration validation failed: {e}"))
        })?;

        // Update current configuration
        {
            let mut current = self.current_config.write().await;
            *current = new_config.clone();
        }

        // Notify subscribers of configuration change
        let config_arc = Arc::new(new_config);
        if let Err(e) = self.config_tx.send(Arc::clone(&config_arc)) {
            warn!("Failed to notify configuration subscribers: {}", e);
        }

        info!("LinkML configuration reloaded successfully");
        Ok(())
    }

    /// Subscribe to configuration changes
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<Arc<LinkMLServiceConfig>> {
        self.config_rx.clone()
    }

    /// Update a configuration value
    ///
    /// # Errors
    ///
    /// Returns error if configuration service fails to set the value or validation fails.
    pub async fn set_value<T>(&self, key: &str, value: &T) -> Result<()>
    where
        T: Serialize + Validate + Clone + Send + Sync + 'static,
    {
        self.config_service
            .set_configuration(key, value)
            .await
            .map_err(|e| LinkMLError::config(format!("Failed to set config value '{key}': {e}")))?;

        // Reload to get the updated configuration
        self.reload().await?;
        Ok(())
    }

    /// Start monitoring for configuration changes
    ///
    /// This method sets up a background task that periodically checks
    /// for configuration changes and reloads when detected.
    ///
    /// # Errors
    ///
    /// Returns an error if the monitoring task cannot be spawned
    pub async fn start_monitoring<TM>(
        self: Arc<Self>,
        task_manager: Arc<TM>,
        check_interval_secs: u64,
    ) -> Result<task_management_core::TaskId>
    where
        TM: task_management_core::TaskManagementService + Send + Sync + 'static,
    {
        let manager = Arc::clone(&self);

        let task_id = task_manager
            .spawn_task(
                async move {
                    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(
                        check_interval_secs,
                    ));

                    loop {
                        interval.tick().await;

                        match manager.reload().await {
                            Ok(()) => info!("Configuration check completed"),
                            Err(e) => error!("Configuration reload failed: {}", e),
                        }
                    }
                },
                None,
            )
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to spawn monitoring task: {e}")))?;

        info!(
            "Configuration monitoring started with {}s interval",
            check_interval_secs
        );
        Ok(task_id)
    }
}

/// Configuration change handler trait
///
/// Implement this trait to handle configuration changes in your service.
#[async_trait]
pub trait ConfigurationChangeHandler: Send + Sync {
    /// Called when configuration changes
    async fn on_configuration_change(&self, new_config: &LinkMLServiceConfig) -> Result<()>;
}

/// Configuration watcher that handles configuration changes
pub struct ConfigurationWatcher<C: ConfigurationService + Send + Sync + 'static> {
    manager: Arc<ConfigurationManager<C>>,
    handlers: Arc<RwLock<Vec<Box<dyn ConfigurationChangeHandler>>>>,
}

impl<C: ConfigurationService + Send + Sync + 'static> ConfigurationWatcher<C> {
    /// Create a new configuration watcher
    #[must_use]
    pub fn new(manager: Arc<ConfigurationManager<C>>) -> Self {
        Self {
            manager,
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a configuration change handler
    pub async fn register_handler(&self, handler: Box<dyn ConfigurationChangeHandler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push(handler);
    }

    /// Start watching for configuration changes.
    ///
    /// # Errors
    ///
    /// Returns an error when the task manager cannot spawn the watcher task.
    pub async fn start_watching<TM>(
        self: Arc<Self>,
        task_manager: Arc<TM>,
    ) -> Result<task_management_core::TaskId>
    where
        TM: task_management_core::TaskManagementService + Send + Sync + 'static,
    {
        let watcher = Arc::clone(&self);
        let mut rx = self.manager.subscribe();

        let task_id = task_manager
            .spawn_task(
                async move {
                    loop {
                        match rx.changed().await {
                            Ok(()) => {
                                let new_config = {
                                    let guard = rx.borrow_and_update();
                                    Arc::clone(&*guard)
                                };
                                info!("Configuration change detected");

                                let handlers = watcher.handlers.read().await;
                                for handler in handlers.iter() {
                                    if let Err(e) =
                                        handler.on_configuration_change(&new_config).await
                                    {
                                        error!("Configuration change handler failed: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Configuration watch error: {}", e);
                                break;
                            }
                        }
                    }
                },
                None,
            )
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to spawn watcher task: {e}")))?;

        info!("Configuration watcher started");
        Ok(task_id)
    }
}
/// Migration utilities to help transition from standalone hot-reload
pub mod migration {
    use super::{
        ConfigurationManager, ConfigurationService, ConfigurationWatcher, LinkMLServiceConfig,
    };
    use linkml_core::error::{LinkMLError, Result};
    use serde_json;
    use serde_yaml;
    use std::{fmt, path::Path, sync::Arc};
    use tokio::fs;
    use tracing::{info, warn};

    /// The configuration types are actually the same - `LinkMLServiceConfig` is an alias
    /// No migration needed, just use the Configuration Service for management
    /// Migration result containing the new configuration manager and any warnings
    pub struct MigrationResult<C: ConfigurationService + Send + Sync + 'static> {
        /// The new configuration manager
        pub manager: Arc<ConfigurationManager<C>>,
        /// Any warnings generated during migration
        pub warnings: Vec<String>,
        /// Whether any legacy files were found
        pub legacy_files_found: bool,
    }

    impl<C: ConfigurationService + Send + Sync + 'static> fmt::Debug for MigrationResult<C> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_struct("MigrationResult")
                .field("manager", &"<ConfigurationManager>")
                .field("warnings", &self.warnings)
                .field("legacy_files_found", &self.legacy_files_found)
                .finish()
        }
    }

    /// Migrate from file-based configuration to the `ConfigurationService`.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration manager cannot be created with
    /// the provided configuration service.
    pub async fn migrate_from_file<C>(
        config_service: Arc<C>,
        legacy_config_path: Option<&Path>,
    ) -> Result<MigrationResult<C>>
    where
        C: ConfigurationService + Send + Sync + 'static,
    {
        let mut warnings = Vec::new();
        let mut legacy_files_found = false;

        // Check for legacy configuration files
        if let Some(path) = legacy_config_path
            && path.exists()
        {
            legacy_files_found = true;
            info!("Found legacy configuration file at: {}", path.display());

            // Try to load and migrate the legacy configuration
            match load_legacy_config(path).await {
                Ok(legacy_config) => {
                    info!(
                        "Successfully loaded legacy configuration, migrating to Configuration Service"
                    );

                    // Store the configuration in the Configuration Service
                    if let Err(e) = config_service
                        .set_configuration("linkml_service", &legacy_config)
                        .await
                    {
                        warnings.push(format!(
                            "Failed to migrate legacy config to Configuration Service: {e}"
                        ));
                    } else {
                        info!(
                            "Legacy configuration successfully migrated to Configuration Service"
                        );
                    }
                }
                Err(e) => {
                    warnings.push(format!(
                        "Failed to load legacy config from {}: {}",
                        path.display(),
                        e
                    ));
                }
            }
        }

        // Create the new configuration manager
        let manager = Arc::new(ConfigurationManager::new(config_service).await?);

        Ok(MigrationResult {
            manager,
            warnings,
            legacy_files_found,
        })
    }

    /// Load legacy configuration from file (YAML or JSON)
    async fn load_legacy_config(path: &Path) -> Result<LinkMLServiceConfig> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            LinkMLError::config(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Try to parse as YAML first, then JSON
        if let Some(ext) = path.extension() {
            match ext.to_str() {
                Some("yaml" | "yml") => serde_yaml::from_str(&content)
                    .map_err(|e| LinkMLError::config(format!("Invalid YAML config: {e}"))),
                Some("json") => serde_json::from_str(&content)
                    .map_err(|e| LinkMLError::config(format!("Invalid JSON config: {e}"))),
                _ => {
                    // Try YAML first, then JSON
                    serde_yaml::from_str(&content)
                        .or_else(|_| serde_json::from_str(&content))
                        .map_err(|e| {
                            LinkMLError::config(format!(
                                "Failed to parse config as YAML or JSON: {e}"
                            ))
                        })
                }
            }
        } else {
            // No extension, try both formats
            serde_yaml::from_str(&content)
                .or_else(|_| serde_json::from_str(&content))
                .map_err(|e| {
                    LinkMLError::config(format!("Failed to parse config as YAML or JSON: {e}"))
                })
        }
    }

    /// Migrate from the hot-reload setup to `ConfigurationService` integration.
    ///
    /// # Errors
    ///
    /// Returns an error when migrating the legacy configuration fails, when the
    /// configuration manager cannot start monitoring, or when the watcher task
    /// cannot be spawned.
    pub async fn migrate_hot_reload_to_config_service<C, TM>(
        config_service: Arc<C>,
        task_manager: Arc<TM>,
        legacy_config_path: Option<&Path>,
        check_interval_secs: u64,
    ) -> Result<(Arc<ConfigurationManager<C>>, Arc<ConfigurationWatcher<C>>)>
    where
        C: ConfigurationService + Send + Sync + 'static,
        TM: task_management_core::TaskManagementService + Send + Sync + 'static,
    {
        // First migrate the configuration
        let migration_result = migrate_from_file(config_service, legacy_config_path).await?;

        // Log any warnings
        for warning in &migration_result.warnings {
            warn!("Migration warning: {}", warning);
        }

        if migration_result.legacy_files_found {
            info!("Successfully migrated from legacy file-based configuration");
        }

        // Start configuration monitoring
        let manager_clone = Arc::clone(&migration_result.manager);
        manager_clone
            .start_monitoring(task_manager.clone(), check_interval_secs)
            .await?;

        // Create and start configuration watcher
        let watcher = Arc::new(ConfigurationWatcher::new(Arc::clone(
            &migration_result.manager,
        )));
        let watcher_clone = Arc::clone(&watcher);
        watcher_clone.start_watching(task_manager).await?;

        Ok((migration_result.manager, watcher))
    }

    /// Validate migrated configuration
    ///
    /// # Errors
    ///
    /// Returns error if configuration manager cannot be accessed or validation fails.
    pub async fn validate_migration<C>(manager: &ConfigurationManager<C>) -> Result<Vec<String>>
    where
        C: ConfigurationService + Send + Sync + 'static,
    {
        let mut issues = Vec::new();

        // Get current configuration
        let config = manager.get_config().await;

        // Validate configuration completeness
        if config.generator.output_directory.is_empty() {
            issues.push("No generator output directory configured".to_string());
        }

        if config.parser.max_recursion_depth == 0 {
            issues.push("Parser max recursion depth must be greater than 0".to_string());
        }

        // Check if configuration can be successfully reloaded
        if let Err(e) = manager.reload().await {
            issues.push(format!("Configuration reload test failed: {e}"));
        }

        // Test configuration value access
        match manager.get_value::<bool>("validation_enabled").await {
            Ok(_) => info!("Validation setting successfully accessible"),
            Err(e) => issues.push(format!("Failed to access validation_enabled setting: {e}")),
        }

        Ok(issues)
    }

    /// Create a migration guide for existing code
    #[must_use]
    pub fn migration_guide() -> &'static str {
        r"
        Migration Guide: From Standalone Hot-Reload to Configuration Service
        =====================================================================

        1. Replace ConfigHotReloader with ConfigurationManager:

           OLD:
           ```rust
           use crate::config::hot_reload::{ConfigHotReloader, init_hot_reload};
           let reloader = ConfigHotReloader::new(config_path)?;
           ```

           NEW:
           ```rust
           use crate::config::configuration_integration::ConfigurationManager;
           let manager = ConfigurationManager::new(config_service).await?;
           ```

        2. Replace get_hot_config() with manager.get_config():

           OLD:
           ```rust
           let config = get_hot_config().await?;
           ```

           NEW:
           ```rust
           let config = manager.get_config().await;
           ```

        3. Use Configuration Service for all config operations:
           - Set values: manager.set_value(key, value).await?
           - Get values: manager.get_value(key).await?
           - Reload: manager.reload().await?

        4. Subscribe to changes using the watcher:
           ```rust
           let watcher = ConfigurationWatcher::new(manager);
           watcher.register_handler(handler).await;
           watcher.start_watching(task_manager).await?;
           ```

        5. Remove all direct file watching with notify crate
        "
    }
}

#[cfg(test)]
mod tests {
    // Add tests here
}
