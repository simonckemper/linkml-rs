//! JSON parser v2 using file system adapter
//!
//! This version uses the `FileSystemOperations` trait instead of direct `std::fs` access,
//! following `RootReal`'s architectural patterns.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::path::Path;
use std::sync::Arc;

use super::{AsyncSchemaParser, SchemaParser};
use crate::file_system_adapter::FileSystemOperations;

/// `JSON` parser implementation with file system adapter
pub struct JsonParserV2<F: FileSystemOperations> {
    fs: Arc<F>,
}

impl<F: FileSystemOperations> JsonParserV2<F> {
    /// Create a new `JSON` parser with file system adapter
    pub fn new(fs: Arc<F>) -> Self {
        Self { fs }
    }
}

impl<F: FileSystemOperations> SchemaParser for JsonParserV2<F> {
    fn parse_str(&self, content: &str) -> Result<SchemaDefinition> {
        serde_json::from_str(content).map_err(|e| {
            LinkMLError::parse_at(
                format!("JSON parsing error: {e}"),
                format!("line {}, column {}", e.line(), e.column()),
            )
        })
    }

    fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        // Note: This is a sync trait method, but we need to use async fs operations
        // In a real implementation, we'd need to refactor the trait to be async
        let content = tokio::runtime::Handle::current().block_on(self.fs.read_to_string(path))?;

        <Self as SchemaParser>::parse_str(self, &content).map_err(|e| match e {
            LinkMLError::ParseError { message, location } => LinkMLError::ParseError {
                message: format!("{message} in file {}", path.display()),
                location,
            },
            other => other,
        })
    }
}

#[async_trait::async_trait]
impl<F: FileSystemOperations> AsyncSchemaParser for JsonParserV2<F> {
    async fn parse_str(&self, content: &str) -> Result<SchemaDefinition> {
        serde_json::from_str(content).map_err(|e| {
            LinkMLError::parse_at(
                format!("JSON parsing error: {e}"),
                format!("line {}, column {}", e.line(), e.column()),
            )
        })
    }

    async fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = self.fs.read_to_string(path).await?;

        <Self as AsyncSchemaParser>::parse_str(self, &content)
            .await
            .map_err(|e| match e {
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
    async fn test_json_parser_v2() -> std::result::Result<(), anyhow::Error> {
        let temp_dir = TempDir::new()?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(
            temp_dir.path().to_path_buf(),
        ));
        let parser = JsonParserV2::new(fs.clone());

        // Create a test schema
        let schema_content = r#"{
  "id": "https://example.org/test",
  "name": "TestSchema",
  "description": "A test schema",
  "classes": {
    "Person": {
      "name": "Person",
      "description": "A person",
      "attributes": {
        "name": {
          "name": "name",
          "range": "string",
          "required": true
        },
        "age": {
          "name": "age",
          "range": "integer"
        }
      }
    }
  }
}"#;

        // Write to file using relative path within sandbox
        let schema_path = Path::new("test_schema.json");
        fs.write(schema_path, schema_content).await?;

        // Parse using async trait - explicitly use AsyncSchemaParser trait
        let schema = <JsonParserV2<TokioFileSystemAdapter> as AsyncSchemaParser>::parse_file(
            &parser,
            schema_path,
        )
        .await?;
        assert_eq!(schema.name, "TestSchema");
        assert!(schema.classes.contains_key("Person"));
        Ok(())
    }
}
