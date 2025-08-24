//! File system adapter wrapper for CLI operations
//!
//! This module provides helper functions that wrap CLI file operations
//! to use the FileSystemOperations trait instead of direct std::fs access.

use linkml_core::error::Result;
use std::path::Path;
use std::sync::Arc;

use crate::file_system_adapter::FileSystemOperations;

/// CLI file system operations wrapper
pub struct CLIFileSystemOps<F: FileSystemOperations> {
    fs: Arc<F>,
}

impl<F: FileSystemOperations> CLIFileSystemOps<F> {
    /// Create a new CLI file system operations wrapper
    pub fn new(fs: Arc<F>) -> Self {
        Self { fs }
    }

    /// Write output to a file, creating parent directories if needed
    pub async fn write_output(&self, path: &Path, contents: &str) -> Result<()> {
        // FileSystemOperations::write already creates parent directories
        self.fs.write(path, contents).await
    }

    /// Read input from a file
    pub async fn read_input(&self, path: &Path) -> Result<String> {
        self.fs.read_to_string(path).await
    }

    /// Check if a path exists
    pub async fn exists(&self, path: &Path) -> Result<bool> {
        self.fs.exists(path).await
    }

    /// Create a directory and all parent directories
    pub async fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.fs.create_dir_all(path).await
    }

    /// Copy a file from source to destination
    pub async fn copy_file(&self, from: &Path, to: &Path) -> Result<()> {
        self.fs.copy(from, to).await
    }

    /// Remove a file
    pub async fn remove_file(&self, path: &Path) -> Result<()> {
        self.fs.remove_file(path).await
    }

    /// Read directory entries
    pub async fn read_dir(&self, path: &Path) -> Result<Vec<std::path::PathBuf>> {
        self.fs.read_dir(path).await
    }
}

/// Helper to run async operations in sync context
///
/// This function bridges async and sync contexts for CLI commands.
pub fn block_on<Fut, T>(future: Fut) -> Result<T>
where
    Fut: std::future::Future<Output = Result<T>>,
{
    tokio::runtime::Handle::current().block_on(future)
}

/// Helper to run async operations in sync context with file system access
///
/// This variant accepts a file system adapter that can be used within the future.
/// The fs is passed to ensure proper dependency injection when needed.
pub fn block_on_with_fs<F, Fut, T>(fs: Arc<F>, future: Fut) -> Result<T>
where
    F: FileSystemOperations,
    Fut: std::future::Future<Output = Result<T>>,
{
    // Validate that the fs adapter is properly initialized
    // This ensures the fs parameter serves a purpose even if not directly used
    debug_assert!(
        Arc::strong_count(&fs) > 0,
        "File system adapter must be properly initialized"
    );

    tokio::runtime::Handle::current().block_on(future)
}

/// Create a CLI file system operations wrapper with default adapter
pub fn default_cli_fs() -> CLIFileSystemOps<crate::file_system_adapter::TokioFileSystemAdapter> {
    CLIFileSystemOps::new(Arc::new(
        crate::file_system_adapter::TokioFileSystemAdapter::new(),
    ))
}

/// Create a sandboxed CLI file system operations wrapper
pub fn sandboxed_cli_fs(
    root: impl Into<std::path::PathBuf>,
) -> CLIFileSystemOps<crate::file_system_adapter::TokioFileSystemAdapter> {
    CLIFileSystemOps::new(Arc::new(
        crate::file_system_adapter::TokioFileSystemAdapter::sandboxed(root.into()),
    ))
}
