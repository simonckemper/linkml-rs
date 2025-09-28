//! Plugin discovery mechanisms
//!
//! This module provides various strategies for discovering plugins in the filesystem.

use super::{Deserialize, HashMap, LinkMLError, PluginInfo, Serialize};
use glob::glob;
use linkml_core::error::Result;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Plugin discovery service
pub struct PluginDiscovery {
    /// File patterns to search for
    patterns: Vec<String>,
    /// Directories to exclude
    exclude_dirs: Vec<String>,
}

/// Discovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryStrategy {
    /// Search only in the specified directory
    Shallow,
    /// Search recursively in subdirectories
    Recursive,
    /// Search using glob patterns
    Glob,
    /// Search in system plugin directories
    System,
}

impl Default for PluginDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginDiscovery {
    /// Create a new plugin discovery service
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: vec![
                "linkml-plugin-*.toml".to_string(),
                "plugin.toml".to_string(),
                "*.linkml-plugin".to_string(),
            ],
            exclude_dirs: vec![
                "node_modules".to_string(),
                "target".to_string(),
                ".git".to_string(),
                "dist".to_string(),
                "build".to_string(),
            ],
        }
    }

    /// Add a file pattern to search for
    pub fn add_pattern(&mut self, pattern: String) {
        self.patterns.push(pattern);
    }

    /// Add a directory to exclude
    pub fn exclude_dir(&mut self, dir: String) {
        self.exclude_dirs.push(dir);
    }

    /// Discover plugins using the specified strategy
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError::IoError` if directory traversal fails
    /// Returns `LinkMLError::PluginError` if plugin discovery fails
    pub fn discover(&self, path: &Path, strategy: DiscoveryStrategy) -> Result<Vec<PathBuf>> {
        match strategy {
            DiscoveryStrategy::Shallow => self.discover_shallow(path),
            DiscoveryStrategy::Recursive => self.discover_recursive(path),
            DiscoveryStrategy::Glob => self.discover_glob(path),
            DiscoveryStrategy::System => self.discover_system(),
        }
    }

    /// Discover plugins in a single directory
    fn discover_shallow(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();

        if !path.is_dir() {
            return Err(LinkMLError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Path is not a directory: {path:?}"),
            )));
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if self.is_plugin_manifest(&path) {
                plugins.push(path);
            }
        }

        Ok(plugins)
    }

    /// Discover plugins recursively
    fn discover_recursive(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();

        for entry in WalkDir::new(path)
            .follow_links(true)
            .into_iter()
            .filter_entry(|e| !self.should_exclude(e.path()))
        {
            let entry = entry.map_err(|e| {
                LinkMLError::IoError(std::io::Error::other(format!(
                    "Failed to read directory entry: {e}"
                )))
            })?;
            let path = entry.path();

            if path.is_file() && self.is_plugin_manifest(path) {
                plugins.push(path.to_path_buf());
            }
        }

        Ok(plugins)
    }

    /// Discover plugins using glob patterns
    fn discover_glob(&self, base_path: &Path) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();

        for pattern in &self.patterns {
            let full_pattern = base_path.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            for entry in glob(&pattern_str).map_err(|e| {
                LinkMLError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid glob pattern: {e}"),
                ))
            })? {
                let path = entry.map_err(|e| {
                    LinkMLError::IoError(std::io::Error::other(format!(
                        "Failed to read glob entry: {e}"
                    )))
                })?;
                if path.is_file() {
                    plugins.push(path);
                }
            }
        }

        Ok(plugins)
    }

    /// Discover plugins in system directories
    fn discover_system(&self) -> Result<Vec<PathBuf>> {
        let mut plugins = Vec::new();

        // Check common system plugin directories
        let system_dirs = self.get_system_plugin_dirs();

        for dir in system_dirs {
            if dir.exists() {
                let mut dir_plugins = self.discover_recursive(&dir)?;
                plugins.append(&mut dir_plugins);
            }
        }

        Ok(plugins)
    }

    /// Get system plugin directories
    fn get_system_plugin_dirs(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        // User home directory
        if let Some(home) = dirs::home_dir() {
            dirs.push(home.join(".linkml").join("plugins"));
            dirs.push(
                home.join(".local")
                    .join("share")
                    .join("linkml")
                    .join("plugins"),
            );
        }

        // System directories
        #[cfg(unix)]
        {
            dirs.push(PathBuf::from("/usr/local/share/linkml/plugins"));
            dirs.push(PathBuf::from("/usr/share/linkml/plugins"));
            dirs.push(PathBuf::from("/opt/linkml/plugins"));
        }

        #[cfg(windows)]
        {
            if let Ok(program_files) = std::env::var("ProgramFiles") {
                dirs.push(PathBuf::from(program_files).join("LinkML").join("plugins"));
            }
            if let Ok(app_data) = std::env::var("APPDATA") {
                dirs.push(PathBuf::from(app_data).join("LinkML").join("plugins"));
            }
        }

        // Environment variable override
        if let Ok(plugin_path) = std::env::var("LINKML_PLUGIN_PATH") {
            for path in plugin_path.split(':') {
                dirs.push(PathBuf::from(path));
            }
        }

        dirs
    }

    /// Check if a path should be excluded
    fn should_exclude(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            self.exclude_dirs.iter().any(|ex| name_str == *ex)
        } else {
            false
        }
    }

    /// Check if a file is a plugin manifest
    fn is_plugin_manifest(&self, path: &Path) -> bool {
        if !path.is_file() {
            return false;
        }

        self.matches_pattern(path)
    }

    /// Check if a path matches plugin patterns (for testing)
    fn matches_pattern(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            self.patterns.iter().any(|pattern| {
                if pattern.contains('*') {
                    // Simple glob matching
                    let regex_pattern = pattern.replace('*', ".*");
                    if let Ok(re) = regex::Regex::new(&format!("^{regex_pattern}$")) {
                        re.is_match(&name_str)
                    } else {
                        false
                    }
                } else {
                    name_str == *pattern
                }
            })
        } else {
            false
        }
    }
}

/// Plugin manifest structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin metadata
    pub plugin: PluginInfo,
    /// Entry point
    pub entry_point: EntryPoint,
    /// Build configuration
    pub build: Option<BuildConfig>,
    /// Installation requirements
    pub requirements: Option<Requirements>,
}

/// Plugin entry point specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EntryPoint {
    /// Native Rust plugin (.so/.dll/.dylib)
    Native {
        /// Library file path relative to manifest
        library: String,
        /// Symbol name to load
        symbol: Option<String>,
    },
    /// Python plugin
    Python {
        /// Module path
        module: String,
        /// Class name
        class: String,
    },
    /// JavaScript/Node.js plugin
    JavaScript {
        /// Module file
        module: String,
        /// Export name
        export: Option<String>,
    },
    /// WebAssembly plugin
    Wasm {
        /// WASM module file
        module: String,
        /// Instance configuration
        config: Option<serde_json::Value>,
    },
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Build command
    pub command: Option<String>,
    /// Build directory
    pub directory: Option<String>,
    /// Environment variables
    pub env: Option<HashMap<String, String>>,
}

/// Installation requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirements {
    /// System dependencies
    pub system: Option<Vec<String>>,
    /// Python packages
    pub python: Option<Vec<String>>,
    /// Node.js packages
    pub npm: Option<Vec<String>>,
    /// Rust crates
    pub cargo: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn test_plugin_discovery_shallow() -> std::result::Result<(), LinkMLError> {
        let temp_dir = TempDir::new().map_err(|e| LinkMLError::IoError(e))?;
        let plugin_file = temp_dir.path().join("plugin.toml");
        fs::write(&plugin_file, "# Plugin manifest").map_err(|e| LinkMLError::IoError(e))?;

        let discovery = PluginDiscovery::new();
        let plugins = discovery.discover(temp_dir.path(), DiscoveryStrategy::Shallow)?;

        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0], plugin_file);
        Ok(())
    }

    #[test]
    fn test_pattern_matching() {
        let discovery = PluginDiscovery::new();
        let path = Path::new("linkml-plugin-test.toml");
        assert!(discovery.matches_pattern(path));

        let path = Path::new("something.linkml-plugin");
        assert!(discovery.matches_pattern(path));

        let path = Path::new("not-a-plugin.txt");
        assert!(!discovery.matches_pattern(path));
    }
}
