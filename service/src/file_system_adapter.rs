//! File system adapter for LinkML service
//!
//! This module provides a clean abstraction over file system operations,
//! preparing for future integration with a dedicated File System Service.
//! It follows RootReal's architectural patterns and provides sandboxed,
//! async file operations.

use async_trait::async_trait;
use linkml_core::{LinkMLError, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// File system operations trait
#[async_trait]
pub trait FileSystemOperations: Send + Sync {
    /// Read a file to string
    async fn read_to_string(&self, path: &Path) -> Result<String>;

    /// Write string to file
    async fn write(&self, path: &Path, contents: &str) -> Result<()>;

    /// Check if path exists
    async fn exists(&self, path: &Path) -> Result<bool>;

    /// Create directory (including parents)
    async fn create_dir_all(&self, path: &Path) -> Result<()>;

    /// Read directory entries
    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>>;

    /// Get file metadata
    async fn metadata(&self, path: &Path) -> Result<FileMetadata>;

    /// Copy file
    async fn copy(&self, from: &Path, to: &Path) -> Result<()>;

    /// Remove file
    async fn remove_file(&self, path: &Path) -> Result<()>;

    /// Remove directory (must be empty)
    async fn remove_dir(&self, path: &Path) -> Result<()>;
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: u64,
    /// Is directory
    pub is_dir: bool,
    /// Is file
    pub is_file: bool,
    /// Is symlink
    pub is_symlink: bool,
    /// Last modified time (Unix timestamp)
    pub modified: Option<u64>,
}

/// Default file system adapter using `tokio::fs`
pub struct TokioFileSystemAdapter {
    /// Optional root directory for sandboxing
    root: Option<PathBuf>,
}

impl Default for TokioFileSystemAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioFileSystemAdapter {
    /// Create new adapter
    #[must_use]
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create sandboxed adapter limited to a root directory
    #[must_use]
    pub fn sandboxed(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    /// Resolve path within sandbox
    fn resolve_path(&self, path: &Path) -> Result<PathBuf> {
        if let Some(root) = &self.root {
            // Check for obvious escape attempts
            for component in path.components() {
                if matches!(component, std::path::Component::ParentDir) {
                    return Err(LinkMLError::IoError(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Path contains '..' which could escape sandbox: {path:?}"),
                    )));
                }
            }

            // Also check if path is absolute (which would escape sandbox)
            if path.is_absolute() {
                return Err(LinkMLError::IoError(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("Absolute paths not allowed in sandbox: {path:?}"),
                )));
            }

            // Safe to join
            Ok(root.join(path))
        } else {
            Ok(path.to_path_buf())
        }
    }
}

#[async_trait]
impl FileSystemOperations for TokioFileSystemAdapter {
    async fn read_to_string(&self, path: &Path) -> Result<String> {
        let resolved = self.resolve_path(path)?;
        fs::read_to_string(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read {}: {}", resolved.display(), e),
            ))
        })
    }

    async fn write(&self, path: &Path, contents: &str) -> Result<()> {
        let resolved = self.resolve_path(path)?;

        // Ensure parent directory exists
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                LinkMLError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create parent directory: {e}"),
                ))
            })?;
        }

        fs::write(&resolved, contents).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to write {}: {}", resolved.display(), e),
            ))
        })
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        let resolved = self.resolve_path(path)?;
        match fs::metadata(&resolved).await {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to check existence: {e}"),
            ))),
        }
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        let resolved = self.resolve_path(path)?;
        fs::create_dir_all(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to create directory: {e}"),
            ))
        })
    }

    async fn read_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let resolved = self.resolve_path(path)?;
        let mut entries = Vec::new();
        let mut dir = fs::read_dir(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to read directory: {e}"),
            ))
        })?;

        while let Some(entry) = dir.next_entry().await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::other(format!(
                "Failed to read directory entry: {e}"
            )))
        })? {
            entries.push(entry.path());
        }

        Ok(entries)
    }

    async fn metadata(&self, path: &Path) -> Result<FileMetadata> {
        let resolved = self.resolve_path(path)?;
        let meta = fs::metadata(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to get metadata: {e}"),
            ))
        })?;

        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        Ok(FileMetadata {
            size: meta.len(),
            is_dir: meta.is_dir(),
            is_file: meta.is_file(),
            is_symlink: meta.is_symlink(),
            modified,
        })
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        let from_resolved = self.resolve_path(from)?;
        let to_resolved = self.resolve_path(to)?;

        // Ensure destination parent exists
        if let Some(parent) = to_resolved.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                LinkMLError::IoError(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create parent directory: {e}"),
                ))
            })?;
        }

        fs::copy(&from_resolved, &to_resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to copy file: {e}"),
            ))
        })?;

        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        let resolved = self.resolve_path(path)?;
        fs::remove_file(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to remove file: {e}"),
            ))
        })
    }

    async fn remove_dir(&self, path: &Path) -> Result<()> {
        let resolved = self.resolve_path(path)?;
        fs::remove_dir(&resolved).await.map_err(|e| {
            LinkMLError::IoError(std::io::Error::new(
                e.kind(),
                format!("Failed to remove directory: {e}"),
            ))
        })
    }
}

/// Create a sandboxed file system adapter for a specific directory
pub fn sandboxed_fs(root: impl Into<PathBuf>) -> TokioFileSystemAdapter {
    TokioFileSystemAdapter::sandboxed(root.into())
}

/// Create an unrestricted file system adapter
#[must_use]
pub fn unrestricted_fs() -> TokioFileSystemAdapter {
    TokioFileSystemAdapter::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_sandboxed_operations() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new().expect("should create temporary directory: {}");
        let fs = sandboxed_fs(temp_dir.path());

        // Test write and read
        let test_path = Path::new("test.txt");
        fs.write(test_path, "Hello, World!")
            .await
            .expect("should write file: {}");
        let content = fs
            .read_to_string(test_path)
            .await
            .expect("should read file: {}");
        assert_eq!(content, "Hello, World!");

        // Test exists
        assert!(
            fs.exists(test_path)
                .await
                .expect("should check existence: {}")
        );
        assert!(
            !fs.exists(Path::new("nonexistent.txt"))
                .await
                .expect("should check non-existence: {}")
        );

        // Test sandbox escape prevention
        let escape_path = Path::new("../escape.txt");
        assert!(fs.write(escape_path, "data").await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn test_directory_operations() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new().expect("should create temporary directory: {}");
        let fs = sandboxed_fs(temp_dir.path());

        // Create nested directories
        let dir_path = Path::new("a/b/c");
        fs.create_dir_all(dir_path)
            .await
            .expect("should create nested directories: {}");

        // Write file in nested directory
        let file_path = Path::new("a/b/c/file.txt");
        fs.write(file_path, "nested content")
            .await
            .expect("should write file in nested directory: {}");

        // Read directory
        let entries = fs
            .read_dir(Path::new("a/b/c"))
            .await
            .expect("should read directory: {}");
        assert_eq!(entries.len(), 1);
        Ok(())
    }
}
