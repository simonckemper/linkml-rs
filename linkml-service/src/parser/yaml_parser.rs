//! YAML parser for `LinkML` schemas

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
};
use std::fs;
use std::path::Path;

use super::SchemaParser;

/// YAML parser implementation
#[derive(Default)]
pub struct YamlParser;

impl YamlParser {
    /// Create a new YAML parser
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl SchemaParser for YamlParser {
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
    fn test_parse_minimal_schema() {
        let yaml = r"
id: https://example.org/test
name: test_schema
";

        let parser = YamlParser::new();
        let schema = parser.parse_str(yaml)
            .expect("Failed to parse minimal schema YAML");

        assert_eq!(schema.id, "https://example.org/test");
        assert_eq!(schema.name, "test_schema");
    }

    #[test]
    fn test_parse_schema_with_classes() {
        let yaml = r"
id: https://example.org/test
name: test_schema
classes:
  Person:
    name: Person
    description: A human being
    slots:
      - name
      - age
slots:
  name:
    name: name
    range: string
  age:
    name: age
    range: integer
";

        let parser = YamlParser::new();
        let schema = parser.parse_str(yaml)
            .expect("Failed to parse schema with classes YAML");

        assert!(schema.classes.contains_key("Person"));
        assert_eq!(schema.classes["Person"].slots.len(), 2);
        assert!(schema.slots.contains_key("name"));
        assert!(schema.slots.contains_key("age"));
    }

    #[test]
    fn test_parse_invalid_yaml() {
        let yaml = "invalid: yaml: content:";

        let parser = YamlParser::new();
        let result = parser.parse_str(yaml);

        assert!(result.is_err());
        if let Err(LinkMLError::ParseError { message, .. }) = result {
            assert!(message.contains("YAML parsing error"));
        } else {
            panic!("Expected ParseError");
        }
    }
}
