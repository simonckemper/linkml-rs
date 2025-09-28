//! Dynamic plugin loading
//!
//! This module handles loading plugins from various sources including
//! native libraries, Python modules, JavaScript, and WebAssembly.

use super::{EntryPoint, LinkMLError, Plugin, PluginManifest, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use timeout_core::{OperationComplexity, TimeoutContext, TimeoutService};
use timestamp_core::SyncTimestampService;

use toml;

/// Type alias for plugin loading future
type PluginLoadFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Plugin>>> + Send>>;

/// Plugin loader interface
pub trait PluginLoader: Send + Sync {
    /// Load plugin metadata from manifest
    ///
    /// # Errors
    ///
    /// Returns error if manifest file cannot be read or parsed.
    fn load_metadata(&self, path: &Path) -> Result<PluginManifest>;

    /// Load a plugin from the given path
    fn load_plugin(&self, path: &Path, manifest: &PluginManifest) -> PluginLoadFuture;
}

/// Dynamic plugin loader supporting multiple plugin types
pub struct DynamicLoader {
    /// Built-in plugin registry
    builtin_registry: super::BuiltinPluginRegistry,
    /// Native library loader (disabled for safety)
    native_loader: NativeLoader,
    /// Python plugin loader (requires feature flag)
    python_loader: PythonLoader,
    /// JavaScript plugin loader (requires feature flag)
    js_loader: JavaScriptLoader,
    /// WebAssembly plugin loader (requires feature flag)
    wasm_loader: WasmLoader,
}

impl Default for DynamicLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicLoader {
    /// Create a new dynamic loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            builtin_registry: super::BuiltinPluginRegistry::new(),
            native_loader: NativeLoader::new(),
            python_loader: PythonLoader::new(),
            js_loader: JavaScriptLoader::new(),
            wasm_loader: WasmLoader::new(),
        }
    }

    /// Get a built-in plugin by name
    #[must_use]
    pub fn get_builtin_plugin(&self, name: &str) -> Option<&dyn Plugin> {
        self.builtin_registry.get_plugin(name)
    }

    /// List all available built-in plugins
    #[must_use]
    pub fn list_builtin_plugins(&self) -> Vec<String> {
        self.builtin_registry.list_plugins()
    }

    /// Load plugin metadata from a manifest file
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::IoError` if the manifest file cannot be read
    /// Returns `LinkMLError::ParseError` if the manifest format is invalid
    pub fn load_metadata(&self, path: &Path) -> Result<PluginManifest> {
        let content = fs::read_to_string(path)?;
        let manifest: PluginManifest =
            toml::from_str(&content).map_err(|e| LinkMLError::ParseError {
                message: format!("Invalid plugin manifest: {e}"),
                location: Some(path.to_string_lossy().to_string()),
            })?;

        Ok(manifest)
    }

    /// Load a plugin based on its entry point type
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::PluginError` if the plugin cannot be loaded or initialized
    pub fn load_plugin(
        &self,
        path: &Path,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn Plugin>> {
        let base_dir = path
            .parent()
            .ok_or_else(|| LinkMLError::other("Invalid plugin path"))?;

        match &manifest.entry_point {
            EntryPoint::Native { library, symbol } => {
                self.native_loader
                    .load_plugin(base_dir, library, symbol.as_deref())
            }
            EntryPoint::Python { module, class } => self.python_loader.load_plugin(module, class),
            EntryPoint::JavaScript { module, export } => {
                self.js_loader
                    .load_plugin(base_dir, module, export.as_deref())
            }
            EntryPoint::Wasm { module, config } => {
                self.wasm_loader
                    .load_plugin(base_dir, module, config.as_ref())
            }
        }
    }
}

/// Native plugin loader for Rust/C/C++ plugins
struct NativeLoader;

impl NativeLoader {
    fn new() -> Self {
        Self
    }

    fn load_plugin(
        &self,
        base_dir: &Path,
        library: &str,
        symbol: Option<&str>,
    ) -> Result<Box<dyn Plugin>> {
        let lib_path = base_dir.join(library);
        let _ = symbol; // Unused but kept for API compatibility

        // Native plugin loading would require unsafe code, which is forbidden
        // by RootReal's core principles. Plugins must be compiled into the
        // application at build time or loaded through safe mechanisms.
        Err(LinkMLError::ServiceError(format!(
            "Native plugin loading is not supported due to safety requirements. \
                 Plugin at '{}' must be compiled into the application at build time \
                 or implemented using safe plugin mechanisms.",
            lib_path.display()
        )))
    }
}

/// Python plugin loader using `PyO3`
struct PythonLoader;

impl PythonLoader {
    fn new() -> Self {
        Self
    }

    /// Load a Python plugin
    ///
    /// # Errors
    ///
    /// Returns error because Python plugin support requires `PyO3` integration
    /// and the 'python' feature to be enabled.
    fn load_plugin(&self, _module: &str, _class: &str) -> Result<Box<dyn Plugin>> {
        // Python integration would require PyO3
        // For now, return an error indicating Python plugins need PyO3 integration
        Err(LinkMLError::ServiceError(
            "Python plugin support requires PyO3 integration. \
             Please enable the 'python' feature."
                .to_string(),
        ))
    }
}

/// JavaScript plugin loader
struct JavaScriptLoader;

impl JavaScriptLoader {
    fn new() -> Self {
        Self
    }

    fn load_plugin(
        &self,
        _base_dir: &Path,
        _module: &str,
        _export: Option<&str>,
    ) -> Result<Box<dyn Plugin>> {
        // JavaScript integration would require a JS runtime like deno_core
        Err(LinkMLError::ServiceError(
            "JavaScript plugin support requires JS runtime integration. \
             Please enable the 'javascript' feature."
                .to_string(),
        ))
    }
}

/// WebAssembly plugin loader
struct WasmLoader;

impl WasmLoader {
    fn new() -> Self {
        Self
    }

    fn load_plugin(
        &self,
        _base_dir: &Path,
        _module: &str,
        _config: Option<&serde_json::Value>,
    ) -> Result<Box<dyn Plugin>> {
        // WASM integration would require wasmtime or wasmer
        Err(LinkMLError::ServiceError(
            "WebAssembly plugin support requires WASM runtime integration. \
             Please enable the 'wasm' feature."
                .to_string(),
        ))
    }
}

/// Plugin sandbox for secure execution
#[derive(Debug, Clone)]
pub struct PluginSandbox {
    /// Resource limits
    pub limits: ResourceLimits,
    /// Allowed capabilities
    pub capabilities: Vec<String>,
}

impl PluginSandbox {
    /// Create a new plugin sandbox with default limits
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: ResourceLimits::default(),
            capabilities: Vec::new(),
        }
    }

    /// Create a sandbox with custom resource limits
    #[must_use]
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            limits,
            capabilities: Vec::new(),
        }
    }

    /// Add a capability to the sandbox
    pub fn add_capability(&mut self, capability: String) {
        self.capabilities.push(capability);
    }

    /// Check if a capability is allowed
    #[must_use]
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.iter().any(|c| c == capability)
    }
}

impl Default for PluginSandbox {
    fn default() -> Self {
        Self::new()
    }
}

/// Resource limits for plugins
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory: usize,
    /// Maximum CPU time in milliseconds
    pub max_cpu_time: u64,
    /// Maximum file size in bytes
    pub max_file_size: usize,
    /// Maximum number of open files
    pub max_open_files: usize,
    /// Network access allowed
    pub allow_network: bool,
    /// File system access mode
    pub fs_access: FsAccessMode,
}

/// File system access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsAccessMode {
    /// No file system access
    None,
    /// Read-only access to specific directories
    ReadOnly,
    /// Read-write access to temporary directory only
    TempOnly,
    /// Full read-write access (dangerous!)
    Full,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 256 * 1024 * 1024,    // 256 MB
            max_cpu_time: 30_000,             // 30 seconds
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_open_files: 100,
            allow_network: false,
            fs_access: FsAccessMode::TempOnly,
        }
    }
}

/// Plugin wrapper that enforces sandboxing
pub struct SandboxedPlugin<O: TimeoutService> {
    plugin: Box<dyn Plugin>,
    /// Sandbox configuration
    sandbox: PluginSandbox,
    /// Timeout service for managing execution timeouts
    timeout_service: Arc<O>,
    /// Timestamp service for timing operations
    timestamp_service: Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>>,
}

impl<O: TimeoutService> SandboxedPlugin<O> {
    /// Create a new sandboxed plugin with timeout service
    pub fn new(plugin: Box<dyn Plugin>, sandbox: PluginSandbox, timeout_service: Arc<O>) -> Self {
        Self {
            plugin,
            sandbox,
            timeout_service,
            timestamp_service: timestamp_service::factory::create_sync_timestamp_service(),
        }
    }

    /// Create a new sandboxed plugin with injected dependencies (factory pattern compliant)
    pub fn with_dependencies<T>(
        plugin: Box<dyn Plugin>,
        sandbox: PluginSandbox,
        timeout_service: Arc<O>,
        timestamp_service: Arc<T>,
    ) -> Self
    where
        T: SyncTimestampService<Error = timestamp_core::TimestampError> + Send + Sync + 'static,
    {
        Self {
            plugin,
            sandbox,
            timeout_service,
            timestamp_service,
        }
    }

    /// Execute with resource limits using the timeout service
    ///
    /// This implementation uses `RootReal`'s timeout service for proper timeout management
    /// with adaptive algorithms, jitter support, and monitoring integration.
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn execute_sandboxed<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&dyn Plugin) -> Result<R> + Send + 'static,
        R: Send + 'static,
    {
        use tokio::time::timeout;

        // Check if operation is allowed based on capabilities
        if !self.sandbox.has_capability("execute") && !self.sandbox.capabilities.is_empty() {
            return Err(LinkMLError::other(
                "Plugin does not have execute capability",
            ));
        }

        // Create timeout context for the plugin operation
        let context = TimeoutContext {
            system_load: None, // Could be populated from monitoring service
            complexity: Some(OperationComplexity::Variable), // Plugin complexity is variable
            network_quality: None,
            metadata: std::collections::HashMap::new(),
        };

        // Calculate adaptive timeout using the timeout service
        let operation_name = format!("plugin_{}", self.plugin.info().name);
        let timeout_value = self
            .timeout_service
            .calculate_timeout(&operation_name, Some(&context))
            .await
            .map_err(|e| LinkMLError::other(format!("Failed to calculate timeout: {e}")))?;

        // Record the start time for duration tracking
        let start_time = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::other(format!("Failed to get system time: {e}")))?;

        // Execute with the calculated timeout
        let result = timeout(timeout_value.duration, async { f(&*self.plugin) }).await;

        // Record the actual duration
        let end_time = self
            .timestamp_service
            .system_time()
            .map_err(|e| LinkMLError::other(format!("Failed to get system time: {e}")))?;
        let actual_duration = end_time
            .duration_since(start_time)
            .map_err(|e| LinkMLError::other(format!("Time calculation error: {e}")))?;
        let success = result.is_ok();

        // Report the duration back to the timeout service for adaptive learning
        let _ = self
            .timeout_service
            .record_duration(&operation_name, actual_duration, success, Some(&context))
            .await;

        if let Ok(value) = result {
            value
        } else {
            // Log the timeout for analysis
            let _ = self
                .timeout_service
                .record_duration(
                    &operation_name,
                    timeout_value.duration,
                    false,
                    Some(&context),
                )
                .await;

            Err(LinkMLError::other(format!(
                "Plugin execution timed out after {:?} (algorithm: {:?}, confidence: {:.2})",
                timeout_value.duration, timeout_value.algorithm, timeout_value.confidence
            )))
        }
    }

    /// Check if a plugin operation is allowed based on sandbox configuration
    #[must_use]
    pub fn is_allowed(&self, operation: PluginOperation) -> bool {
        match operation {
            PluginOperation::FileRead(path) => self.check_file_access(&path, false),
            PluginOperation::FileWrite(path) => self.check_file_access(&path, true),
            PluginOperation::NetworkAccess(_) => self.sandbox.limits.allow_network,
            PluginOperation::MemoryAllocation(size) => size <= self.sandbox.limits.max_memory,
        }
    }

    /// Check if file access is allowed
    fn check_file_access(&self, path: &std::path::Path, write: bool) -> bool {
        match self.sandbox.limits.fs_access {
            FsAccessMode::None => false,
            FsAccessMode::ReadOnly => !write,
            FsAccessMode::TempOnly => {
                // Check if path is in temp directory
                if let Ok(temp_dir) = std::env::temp_dir().canonicalize()
                    && let Ok(canonical_path) = path.canonicalize()
                {
                    return canonical_path.starts_with(&temp_dir);
                }
                false
            }
            FsAccessMode::Full => true,
        }
    }
}

/// Plugin operations that can be checked against sandbox policies
#[derive(Debug, Clone)]
pub enum PluginOperation {
    /// File read operation
    FileRead(std::path::PathBuf),
    /// File write operation
    FileWrite(std::path::PathBuf),
    /// Network access operation
    NetworkAccess(String),
    /// Memory allocation operation
    MemoryAllocation(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_limits_default() {
        let limits = ResourceLimits::default();
        assert_eq!(limits.max_memory, 256 * 1024 * 1024);
        assert_eq!(limits.fs_access, FsAccessMode::TempOnly);
        assert!(!limits.allow_network);
    }
}
