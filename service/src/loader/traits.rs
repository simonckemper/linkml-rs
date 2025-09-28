//! Common traits and types for data loaders and dumpers

use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Error type for data loading operations
#[derive(Debug, Error)]
pub enum LoaderError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidFormat(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Error: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Result type for loader operations
pub type LoaderResult<T> = std::result::Result<T, LoaderError>;

/// Error type for data dumping operations
#[derive(Debug, Error)]
pub enum DumperError {
    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Schema validation error
    #[error("Schema validation error: {0}")]
    SchemaValidation(String),

    /// Type conversion error
    #[error("Type conversion error: {0}")]
    TypeConversion(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Generic error
    #[error("Error: {0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

/// Result type for dumper operations
pub type DumperResult<T> = std::result::Result<T, DumperError>;

impl From<anyhow::Error> for DumperError {
    fn from(err: anyhow::Error) -> Self {
        DumperError::Other(Box::new(std::io::Error::other(format!("{err}"))))
    }
}

impl From<anyhow::Error> for LoaderError {
    fn from(err: anyhow::Error) -> Self {
        LoaderError::Other(Box::new(std::io::Error::other(format!("{err}"))))
    }
}

// Conversions to LinkMLError
impl From<LoaderError> for linkml_core::LinkMLError {
    fn from(err: LoaderError) -> Self {
        match err {
            LoaderError::Io(io_err) => linkml_core::LinkMLError::IoError(io_err),
            LoaderError::SchemaValidation(msg) => linkml_core::LinkMLError::schema_validation(msg),
            LoaderError::TypeConversion(msg) => linkml_core::LinkMLError::CoercionError {
                from: "unknown".to_string(),
                to: "unknown".to_string(),
                context: Some(msg),
            },
            LoaderError::MissingField(field) => linkml_core::LinkMLError::data_validation(format!(
                "Missing required field: {field}"
            )),
            LoaderError::InvalidFormat(msg) | LoaderError::Parse(msg) => {
                linkml_core::LinkMLError::parse(msg)
            }
            LoaderError::Configuration(msg) => linkml_core::LinkMLError::config(msg),
            LoaderError::Other(boxed_err) => linkml_core::LinkMLError::Other {
                message: "Loader error".to_string(),
                source: Some(boxed_err),
            },
        }
    }
}

impl From<DumperError> for linkml_core::LinkMLError {
    fn from(err: DumperError) -> Self {
        match err {
            DumperError::Io(io_err) => linkml_core::LinkMLError::IoError(io_err),
            DumperError::Serialization(msg) => linkml_core::LinkMLError::SerializationError(msg),
            DumperError::SchemaValidation(msg) => linkml_core::LinkMLError::schema_validation(msg),
            DumperError::TypeConversion(msg) => linkml_core::LinkMLError::CoercionError {
                from: "unknown".to_string(),
                to: "unknown".to_string(),
                context: Some(msg),
            },
            DumperError::Configuration(msg) => linkml_core::LinkMLError::config(msg),
            DumperError::Other(boxed_err) => linkml_core::LinkMLError::Other {
                message: "Dumper error".to_string(),
                source: Some(boxed_err),
            },
        }
    }
}

/// Represents a loaded data instance
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataInstance {
    /// Name of the LinkML class this instance represents
    pub class_name: String,

    /// The actual data fields and values for this instance
    pub data: HashMap<String, JsonValue>,

    /// Optional instance identifier
    pub id: Option<String>,

    /// Metadata about the instance
    pub metadata: HashMap<String, String>,
}

/// Options for loading data
#[derive(Debug, Clone, Default)]
pub struct LoadOptions {
    /// Target class to load data into
    pub target_class: Option<String>,

    /// Whether to validate data against schema
    pub validate: bool,

    /// Whether to infer types automatically
    pub infer_types: bool,

    /// Whether to skip invalid records
    pub skip_invalid: bool,

    /// Maximum number of records to load
    pub limit: Option<usize>,

    /// Custom field mappings
    pub field_mappings: HashMap<String, String>,
}

/// Options for dumping data
#[derive(Debug, Clone, Default)]
pub struct DumpOptions {
    /// Whether to include metadata
    pub include_metadata: bool,

    /// Whether to pretty-print output
    pub pretty_print: bool,

    /// Whether to include null values
    pub include_nulls: bool,

    /// Custom field mappings
    pub field_mappings: HashMap<String, String>,

    /// Maximum records to dump
    pub limit: Option<usize>,

    /// Classes to include in dump (None means all)
    pub include_classes: Option<Vec<String>>,
}

/// Trait for data loaders
#[async_trait]
pub trait DataLoader: Send + Sync {
    /// Name of the loader
    fn name(&self) -> &str;

    /// Description of the loader
    fn description(&self) -> &str;

    /// Supported file extensions
    fn supported_extensions(&self) -> Vec<&str>;

    /// Load data from a file
    async fn load_file(
        &self,
        path: &Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>>;

    /// Load data from a string
    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>>;

    /// Load data from bytes
    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>>;

    /// Validate that the loader can handle the given schema
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not compatible with this loader
    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()>;
}

/// Trait for data dumpers
#[async_trait]
pub trait DataDumper: Send + Sync {
    /// Name of the dumper
    fn name(&self) -> &str;

    /// Description of the dumper
    fn description(&self) -> &str;

    /// Supported file extensions
    fn supported_extensions(&self) -> Vec<&str>;

    /// Dump data to a file
    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()>;

    /// Dump data to a string
    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String>;

    /// Dump data to bytes
    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>>;

    /// Validate that the dumper can handle the given schema
    ///
    /// # Errors
    ///
    /// Returns an error if the schema is not compatible with this dumper
    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()>;
}

/// Registry for loaders and dumpers
pub struct LoaderRegistry {
    loaders: HashMap<String, Box<dyn DataLoader>>,
    dumpers: HashMap<String, Box<dyn DataDumper>>,
}

impl LoaderRegistry {
    /// Create a new registry
    #[must_use]
    pub fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            dumpers: HashMap::new(),
        }
    }

    /// Register a loader
    pub fn register_loader(&mut self, name: String, loader: Box<dyn DataLoader>) {
        self.loaders.insert(name, loader);
    }

    /// Register a dumper
    pub fn register_dumper(&mut self, name: String, dumper: Box<dyn DataDumper>) {
        self.dumpers.insert(name, dumper);
    }

    /// Get a loader by name
    #[must_use]
    pub fn get_loader(&self, name: &str) -> Option<&dyn DataLoader> {
        self.loaders.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Get a dumper by name
    #[must_use]
    pub fn get_dumper(&self, name: &str) -> Option<&dyn DataDumper> {
        self.dumpers.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Get loader for file extension
    #[must_use]
    pub fn get_loader_for_extension(&self, extension: &str) -> Option<&dyn DataLoader> {
        for loader in self.loaders.values() {
            if loader.supported_extensions().contains(&extension) {
                return Some(loader.as_ref());
            }
        }
        None
    }

    /// Get dumper for file extension
    #[must_use]
    pub fn get_dumper_for_extension(&self, extension: &str) -> Option<&dyn DataDumper> {
        for dumper in self.dumpers.values() {
            if dumper.supported_extensions().contains(&extension) {
                return Some(dumper.as_ref());
            }
        }
        None
    }
}

impl Default for LoaderRegistry {
    fn default() -> Self {
        Self::new()
    }
}
