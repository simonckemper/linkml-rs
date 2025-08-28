//! Configuration for the cargo-linkml plugin

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// LinkML configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkMLConfig {
    /// Schema configuration
    pub schema: SchemaConfig,

    /// Generation configuration
    pub generate: GenerateConfig,

    /// Validation configuration
    pub validate: ValidateConfig,
}

/// Schema configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaConfig {
    /// Directory containing schemas
    pub directory: PathBuf,

    /// Include patterns
    pub include: Vec<String>,

    /// Exclude patterns
    pub exclude: Vec<String>,
}

/// Generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateConfig {
    /// Output directory
    pub output_directory: PathBuf,

    /// Add serde derives
    pub serde: bool,

    /// Add Debug derive
    pub debug: bool,

    /// Add Clone derive
    pub clone: bool,

    /// Additional derives
    pub derives: Vec<String>,

    /// Module structure (flat or nested)
    pub module_structure: ModuleStructure,

    /// Validate before generating
    pub validate_first: bool,
}

/// Module structure options
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleStructure {
    /// All types in a single module
    Flat,
    /// Separate modules for classes, enums, etc.
    Nested,
}

/// Validation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateConfig {
    /// Fail on validation errors
    pub fail_on_error: bool,

    /// Show warnings
    pub show_warnings: bool,

    /// Strict mode
    pub strict: bool,
}

impl Default for LinkMLConfig {
    fn default() -> Self {
        Self {
            schema: SchemaConfig {
                directory: PathBuf::from("src/schemas"),
                include: vec![
                    "**/*.linkml.yaml".to_string(),
                    "**/*.linkml.yml".to_string(),
                    "**/*.linkml".to_string(),
                ],
                exclude: vec![],
            },
            generate: GenerateConfig {
                output_directory: PathBuf::from("src/generated"),
                serde: true,
                debug: true,
                clone: true,
                derives: vec![],
                module_structure: ModuleStructure::Nested,
                validate_first: true,
            },
            validate: ValidateConfig {
                fail_on_error: true,
                show_warnings: true,
                strict: false,
            },
        }
    }
}

impl LinkMLConfig {
    /// Load configuration from file
    pub fn load() -> anyhow::Result<Self> {
        let config_path = PathBuf::from("linkml.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let config: Self = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Self::default())
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> anyhow::Result<()> {
        let config_path = PathBuf::from("linkml.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&config_path, content)?;
        Ok(())
    }
}
