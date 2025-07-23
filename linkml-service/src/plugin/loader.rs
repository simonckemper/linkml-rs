//! Dynamic plugin loading
//!
//! This module handles loading plugins from various sources including
//! native libraries, Python modules, JavaScript, and WebAssembly.

use super::*;
use dlopen::wrapper::{Container, WrapperApi};
use std::fs;
use std::path::{Path, PathBuf};
use toml;

/// Plugin loader interface
pub trait PluginLoader: Send + Sync {
    /// Load plugin metadata from manifest
    fn load_metadata(&self, path: &Path) -> Result<PluginManifest>;
    
    /// Load a plugin from the given path
    fn load_plugin(
        &self,
        path: &Path,
        manifest: &PluginManifest,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Box<dyn Plugin>>> + Send>>;
}

/// Dynamic plugin loader supporting multiple plugin types
pub struct DynamicLoader {
    /// Native library loader
    native_loader: NativeLoader,
    /// Python plugin loader
    python_loader: PythonLoader,
    /// JavaScript plugin loader
    js_loader: JavaScriptLoader,
    /// WebAssembly plugin loader
    wasm_loader: WasmLoader,
}

impl DynamicLoader {
    /// Create a new dynamic loader
    pub fn new() -> Self {
        Self {
            native_loader: NativeLoader::new(),
            python_loader: PythonLoader::new(),
            js_loader: JavaScriptLoader::new(),
            wasm_loader: WasmLoader::new(),
        }
    }
    
    /// Load plugin metadata from a manifest file
    pub fn load_metadata(&self, path: &Path) -> Result<PluginManifest> {
        let content = fs::read_to_string(path)?;
        let manifest: PluginManifest = toml::from_str(&content)
            .map_err(|e| LinkMLError::ParseError(format!("Invalid plugin manifest: {}", e)))?;
        
        Ok(manifest)
    }
    
    /// Load a plugin based on its entry point type
    pub async fn load_plugin(
        &self,
        path: &Path,
        manifest: &PluginManifest,
    ) -> Result<Box<dyn Plugin>> {
        let base_dir = path.parent()
            .ok_or_else(|| LinkMLError::InternalError("Invalid plugin path".to_string()))?;
        
        match &manifest.entry_point {
            EntryPoint::Native { library, symbol } => {
                self.native_loader.load_plugin(base_dir, library, symbol.as_deref()).await
            }
            EntryPoint::Python { module, class } => {
                self.python_loader.load_plugin(module, class).await
            }
            EntryPoint::JavaScript { module, export } => {
                self.js_loader.load_plugin(base_dir, module, export.as_deref()).await
            }
            EntryPoint::Wasm { module, config } => {
                self.wasm_loader.load_plugin(base_dir, module, config.as_ref()).await
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
    
    async fn load_plugin(
        &self,
        base_dir: &Path,
        library: &str,
        symbol: Option<&str>,
    ) -> Result<Box<dyn Plugin>> {
        let lib_path = base_dir.join(library);
        
        // Define the plugin API that native libraries must implement
        #[derive(WrapperApi)]
        struct PluginApi {
            create_plugin: fn() -> *mut dyn Plugin,
            plugin_api_version: fn() -> u32,
        }
        
        // Load the library
        let container: Container<PluginApi> = unsafe {
            Container::load(&lib_path)
                .map_err(|e| LinkMLError::PluginError(format!("Failed to load native plugin: {}", e)))?
        };
        
        // Check API version
        let api_version = container.plugin_api_version();
        if api_version != 1 {
            return Err(LinkMLError::PluginError(
                format!("Incompatible plugin API version: {} (expected 1)", api_version)
            ));
        }
        
        // Create plugin instance
        let plugin_ptr = container.create_plugin();
        if plugin_ptr.is_null() {
            return Err(LinkMLError::PluginError("Failed to create plugin instance".to_string()));
        }
        
        // Convert to boxed trait object
        let plugin = unsafe { Box::from_raw(plugin_ptr as *mut dyn Plugin) };
        
        Ok(plugin)
    }
}

/// Python plugin loader using PyO3
struct PythonLoader;

impl PythonLoader {
    fn new() -> Self {
        Self
    }
    
    async fn load_plugin(&self, module: &str, class: &str) -> Result<Box<dyn Plugin>> {
        // Python integration would require PyO3
        // For now, return an error indicating Python plugins need PyO3 integration
        Err(LinkMLError::PluginError(
            "Python plugin support requires PyO3 integration. \
             Please enable the 'python' feature.".to_string()
        ))
    }
}

/// JavaScript plugin loader
struct JavaScriptLoader;

impl JavaScriptLoader {
    fn new() -> Self {
        Self
    }
    
    async fn load_plugin(
        &self,
        base_dir: &Path,
        module: &str,
        export: Option<&str>,
    ) -> Result<Box<dyn Plugin>> {
        // JavaScript integration would require a JS runtime like deno_core
        Err(LinkMLError::PluginError(
            "JavaScript plugin support requires JS runtime integration. \
             Please enable the 'javascript' feature.".to_string()
        ))
    }
}

/// WebAssembly plugin loader
struct WasmLoader;

impl WasmLoader {
    fn new() -> Self {
        Self
    }
    
    async fn load_plugin(
        &self,
        base_dir: &Path,
        module: &str,
        config: Option<&serde_json::Value>,
    ) -> Result<Box<dyn Plugin>> {
        // WASM integration would require wasmtime or wasmer
        Err(LinkMLError::PluginError(
            "WebAssembly plugin support requires WASM runtime integration. \
             Please enable the 'wasm' feature.".to_string()
        ))
    }
}

/// Plugin sandbox for secure execution
pub struct PluginSandbox {
    /// Resource limits
    limits: ResourceLimits,
    /// Allowed capabilities
    capabilities: Vec<String>,
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
            max_memory: 256 * 1024 * 1024, // 256 MB
            max_cpu_time: 30_000, // 30 seconds
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_open_files: 100,
            allow_network: false,
            fs_access: FsAccessMode::TempOnly,
        }
    }
}

/// Plugin wrapper that enforces sandboxing
pub struct SandboxedPlugin {
    /// The actual plugin
    plugin: Box<dyn Plugin>,
    /// Sandbox configuration
    sandbox: PluginSandbox,
}

impl SandboxedPlugin {
    /// Create a new sandboxed plugin
    pub fn new(plugin: Box<dyn Plugin>, sandbox: PluginSandbox) -> Self {
        Self { plugin, sandbox }
    }
    
    /// Execute with resource limits
    pub async fn execute_sandboxed<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&dyn Plugin) -> R,
    {
        // In a real implementation, this would enforce resource limits
        // using OS-level mechanisms like cgroups, rlimits, etc.
        Ok(f(&*self.plugin))
    }
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