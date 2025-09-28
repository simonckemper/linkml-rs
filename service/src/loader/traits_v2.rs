//! Version 2 traits for data loading and dumping with file system adapter support
//!
//! This module provides updated traits that use `FileSystemOperations` instead
//! of direct file system access.

use async_trait::async_trait;
use linkml_core::prelude::*;
use std::path::Path;
use std::sync::Arc;

use super::traits::{DataInstance, DumperError, LoaderError};
use crate::file_system_adapter::FileSystemOperations;

/// Result type for loader operations
pub type LoaderResult<T> = std::result::Result<T, LoaderError>;

/// Result type for dumper operations
pub type DumperResult<T> = std::result::Result<T, DumperError>;

/// Trait for data loaders that use file system adapter
#[async_trait]
pub trait DataLoaderV2: Send + Sync {
    /// Load data instances from a file
    async fn load_file<F: FileSystemOperations>(
        &mut self,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> LoaderResult<Vec<DataInstance>>;

    /// Load data instances from string content
    async fn load_str(
        &mut self,
        content: &str,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>>;

    /// Get the name of this loader
    fn name(&self) -> &'static str;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&'static str>;
}

/// Trait for data dumpers that use file system adapter
#[async_trait]
pub trait DataDumperV2: Send + Sync {
    /// Dump data instances to a file
    async fn dump_file<F: FileSystemOperations>(
        &mut self,
        instances: Vec<DataInstance>,
        path: &Path,
        schema: &SchemaDefinition,
        fs: Arc<F>,
    ) -> DumperResult<()>;

    /// Dump data instances to string
    async fn dump_str(
        &mut self,
        instances: Vec<DataInstance>,
        schema: &SchemaDefinition,
    ) -> DumperResult<String>;

    /// Get the name of this dumper
    fn name(&self) -> &'static str;

    /// Get supported file extensions
    fn supported_extensions(&self) -> Vec<&'static str>;
}

/// Options for configuring loaders and dumpers
#[derive(Debug, Clone, Default)]
pub struct LoaderOptionsV2 {
    /// Whether to validate data during loading
    pub validate: bool,

    /// Whether to use strict mode
    pub strict: bool,

    /// Target class name for validation
    pub target_class: Option<String>,

    /// Maximum number of errors to collect
    pub max_errors: usize,

    /// Custom options for specific loaders
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl LoaderOptionsV2 {
    /// Create new loader options
    #[must_use]
    pub fn new() -> Self {
        Self {
            validate: true,
            strict: false,
            target_class: None,
            max_errors: 100,
            custom: Default::default(),
        }
    }

    /// Set validation enabled
    #[must_use]
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }

    /// Set strict mode
    #[must_use]
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }

    /// Set target class
    #[must_use]
    pub fn with_target_class(mut self, class: String) -> Self {
        self.target_class = Some(class);
        self
    }
}
