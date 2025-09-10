//! Main parser v2 with file system adapter support
//!
//! This version uses FileSystemOperations instead of direct file system access,
//! and provides both sync and async interfaces.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition};
use std::path::Path;
use std::sync::Arc;

use crate::file_system_adapter::FileSystemOperations;
use super::{AsyncSchemaParser, YamlParserV2, JsonParserV2};

/// Main parser that uses file system adapter and delegates to format-specific parsers
pub struct ParserV2<F: FileSystemOperations> {
    yaml_parser: YamlParserV2<F>,
    json_parser: JsonParserV2<F>,
    /// Whether to automatically resolve imports
    auto_resolve_imports: bool}

impl<F: FileSystemOperations> ParserV2<F> {
    /// Create a new parser with file system adapter
    pub fn new(fs: Arc<F>) -> Self {
        Self {
            yaml_parser: YamlParserV2::new(fs.clone()),
            json_parser: JsonParserV2::new(fs),
            auto_resolve_imports: true}
    }

    /// Set whether to automatically resolve imports
    pub fn with_auto_resolve_imports(mut self, auto_resolve: bool) -> Self {
        self.auto_resolve_imports = auto_resolve;
        self
    }

    /// Parse schema from string with explicit format
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn parse_str(&self, content: &str, format: &str) -> Result<SchemaDefinition> {
        match format.to_lowercase().as_str() {
            "yaml" | "yml" => self.yaml_parser.parse_str(content).await,
            "json" => self.json_parser.parse_str(content).await,
            _ => Err(LinkMLError::invalid_format(format))}
    }

    /// Parse schema from file, detecting format from extension
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let format = path
            .extension()
            .and_then(|ext| ext.to_str())
            .ok_or_else(|| LinkMLError::invalid_format("no file extension"))?;

        match format.to_lowercase().as_str() {
            "yaml" | "yml" => self.yaml_parser.parse_file(path).await,
            "json" => self.json_parser.parse_file(path).await,
            _ => Err(LinkMLError::invalid_format(format))}
    }

    /// Parse with explicit format
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn parse_with_format(
        &self,
        content: &str,
        format: Option<&str>
    ) -> Result<SchemaDefinition> {
        let fmt = format.unwrap_or("yaml");
        self.parse_str(content, fmt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_system_adapter::TokioFileSystemAdapter;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_parser_v2_yaml() {
        let temp_dir = TempDir::new()?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(temp_dir.path().to_path_buf());
        let parser = ParserV2::new(fs.clone());

        let schema_content = r#"
id: https://example.org/test
name: TestSchema
classes:
  Person:
    attributes:
      name:
        range: string
"#;

        // Test parse_str
        let schema = parser.parse_str(schema_content, "yaml").await?;
        assert_eq!(schema.name, "TestSchema");

        // Test parse_file
        let schema_path = temp_dir.path().join("test.yaml");
        fs.write(&schema_path, schema_content).await?;
        let schema = parser.parse_file(&schema_path).await?;
        assert_eq!(schema.name, "TestSchema");
    }

    #[tokio::test]
    async fn test_parser_v2_json() {
        let temp_dir = TempDir::new()?;
        let fs = Arc::new(TokioFileSystemAdapter::sandboxed(temp_dir.path().to_path_buf());
        let parser = ParserV2::new(fs.clone());

        let schema_content = r#"{
  "id": "https://example.org/test",
  "name": "TestSchema",
  "classes": {
    "Person": {
      "attributes": {
        "name": {
          "range": "string"
        }
      }
    }
  }
}"#;

        // Test parse_str
        let schema = parser.parse_str(schema_content, "json").await?;
        assert_eq!(schema.name, "TestSchema");

        // Test parse_file
        let schema_path = temp_dir.path().join("test.json");
        fs.write(&schema_path, schema_content).await?;
        let schema = parser.parse_file(&schema_path).await?;
        assert_eq!(schema.name, "TestSchema");
    }
}