//! YAML parser v2 using file system adapter
//!
//! This version uses the FileSystemOperations trait instead of direct std::fs access,
//! following RootReal's architectural patterns.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::path::Path;
use std::sync::Arc;

use crate::file_system_adapter::FileSystemOperations;
use super::SchemaParser;

/// YAML parser implementation with file system adapter
pub struct YamlParserV2<F: FileSystemOperations> {
    fs: Arc<F>,
}

impl<F: FileSystemOperations> YamlParserV2<F> {
    /// Create a new YAML parser with file system adapter
    pub fn new(fs: Arc<F>) -> Self {
        Self { fs }
    }
}

impl<F: FileSystemOperations> SchemaParser for YamlParserV2<F> {
    fn parse_str(&self, content: &str) -> Result<SchemaDefinition> {
        serde_yaml::from_str(content).map_err(|e| {
            LinkMLError::parse_at(
                format!("YAML parsing error: {e}"),
                e.location().map_or_else(
                    || "unknown location".to_string(),
                    |l| format!("line {}, column {}", l.line(), l.column()),
                ),
            )
        })
    }

    fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        // Note: This is a sync trait method, but we need to use async fs operations
        // In a real implementation, we'd need to refactor the trait to be async
        // For now, we'll use tokio's block_on, but this should be addressed
        let content = tokio::runtime::Handle::current()
            .block_on(self.fs.read_to_string(path))?;

        self.parse_str(&content).map_err(|e| match e {
            LinkMLError::ParseError { message, location } => LinkMLError::ParseError {
                message: format!("{message} in file {}", path.display()),
                location,
            },
            other => other,
        })
    }
}

/// Async version of the SchemaParser trait
#[async_trait::async_trait]
pub trait AsyncSchemaParser: Send + Sync {
    /// Parse schema from string content
    async fn parse_str(&self, content: &str) -> Result<SchemaDefinition>;
    
    /// Parse schema from file
    async fn parse_file(&self, path: &Path) -> Result<SchemaDefinition>;
}

#[async_trait::async_trait]
impl<F: FileSystemOperations> AsyncSchemaParser for YamlParserV2<F> {
    async fn parse_str(&self, content: &str) -> Result<SchemaDefinition> {
        serde_yaml::from_str(content).map_err(|e| {
            LinkMLError::parse_at(
                format!("YAML parsing error: {e}"),
                e.location().map_or_else(
                    || "unknown location".to_string(),
                    |l| format!("line {}, column {}", l.line(), l.column()),
                ),
            )
        })
    }
    
    async fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = self.fs.read_to_string(path).await?;
        
        self.parse_str(&content).await.map_err(|e| match e {
            LinkMLError::ParseError { message, location } => LinkMLError::ParseError {
                message: format!("{message} in file {}", path.display()),
                location,
            },
            other => other,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system_adapter::TokioFileSystemAdapter;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_yaml_parser_v2() {
        let temp_dir = TempDir::new().unwrap();
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(temp_dir.path().to_path_buf()));
        let parser = YamlParserV2::new(fs.clone());
        
        // Create a test schema
        let schema_content = r#"
id: https://example.org/test
name: TestSchema
description: A test schema
classes:
  Person:
    description: A person
    attributes:
      name:
        range: string
        required: true
      age:
        range: integer
"#;
        
        // Write to file
        let schema_path = temp_dir.path().join("test_schema.yaml");
        fs.write(&schema_path, schema_content).await.unwrap();
        
        // Parse using async trait
        let schema = parser.parse_file(&schema_path).await.unwrap();
        assert_eq!(schema.name, "TestSchema");
        assert!(schema.classes.contains_key("Person"));
    }
}