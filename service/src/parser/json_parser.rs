//! JSON parser for `LinkML` schemas

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::fs;
use std::path::Path;

use super::SchemaParser;

/// `JSON` parser implementation
#[derive(Default)]
pub struct JsonParser;

impl JsonParser {
    /// Create a new `JSON` parser
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl SchemaParser for JsonParser {
    fn parse_str(&self, content: &str) -> Result<SchemaDefinition> {
        serde_json::from_str(content)
            .map_err(|e| LinkMLError::parse(format!("JSON parsing error: {e}")))
    }

    fn parse_file(&self, path: &Path) -> Result<SchemaDefinition> {
        let content = fs::read_to_string(path).map_err(LinkMLError::IoError)?;

        self.parse_str(&content).map_err(|e| match e {
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

    #[test]
    fn test_parse_minimal_schema() -> std::result::Result<(), anyhow::Error> {
        let json = r#"{
            "id": "https://example.org/test",
            "name": "test_schema"
        }"#;

        let parser = JsonParser::new();
        let schema = parser.parse_str(json)?;

        assert_eq!(schema.id, "https://example.org/test");
        assert_eq!(schema.name, "test_schema");
        Ok(())
    }

    #[test]
    fn test_parse_invalid_json() {
        let json = r#"{"invalid": json content"#;

        let parser = JsonParser::new();
        let result = parser.parse_str(json);

        assert!(result.is_err());
        if let Err(LinkMLError::ParseError { message, .. }) = result {
            assert!(message.contains("JSON parsing error"));
        } else {
            panic!("Expected ParseError");
        }
    }
}
