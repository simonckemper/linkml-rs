//! Core type definitions for LinkML schemas and data

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Schema definition - the root of a LinkML schema
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SchemaDefinition {
    /// Unique identifier for the schema
    pub id: String,
    
    /// Name of the schema
    pub name: String,
    
    /// Human-readable title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    
    /// Description of the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Version of the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    
    /// License information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    
    /// Default prefix for the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_prefix: Option<String>,
    
    /// Prefix declarations
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub prefixes: IndexMap<String, PrefixDefinition>,
    
    /// Import statements
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<String>,
    
    /// Class definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub classes: IndexMap<String, ClassDefinition>,
    
    /// Slot definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub slots: IndexMap<String, SlotDefinition>,
    
    /// Type definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub types: IndexMap<String, TypeDefinition>,
    
    /// Enum definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub enums: IndexMap<String, EnumDefinition>,
    
    /// Subset definitions
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub subsets: IndexMap<String, SubsetDefinition>,
    
    /// Default range for slots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_range: Option<String>,
    
    /// Generation date
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_date: Option<String>,
    
    /// Source file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_file: Option<String>,
    
    /// Metamodel version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metamodel_version: Option<String>,
}

/// Class definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassDefinition {
    /// Name of the class
    pub name: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Is this class abstract?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub abstract_: Option<bool>,
    
    /// Is this a mixin?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mixin: Option<bool>,
    
    /// Parent class (single inheritance)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_a: Option<String>,
    
    /// Mixin classes (multiple inheritance via composition)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mixins: Vec<String>,
    
    /// Slots used by this class
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<String>,
    
    /// Slot usage overrides
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub slot_usage: IndexMap<String, SlotDefinition>,
    
    /// Attributes (inline slots)
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<String, SlotDefinition>,
    
    /// Class URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_uri: Option<String>,
    
    /// Subclass of URIs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subclass_of: Vec<String>,
    
    /// Tree root flag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_root: Option<bool>,
}

/// Slot definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SlotDefinition {
    /// Name of the slot
    pub name: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Range (type) of the slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,
    
    /// Is this slot required?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    
    /// Is this slot multivalued?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multivalued: Option<bool>,
    
    /// Is this slot an identifier?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<bool>,
    
    /// Pattern for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_value: Option<Value>,
    
    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_value: Option<Value>,
    
    /// Permissible values
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissible_values: Vec<PermissibleValue>,
    
    /// Slot URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_uri: Option<String>,
    
    /// Aliases
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    
    /// Is inherited (is_a)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_a: Option<String>,
    
    /// Mixins
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mixins: Vec<String>,
    
    /// Inverse relationship
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inverse: Option<String>,
    
    /// Default value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    
    /// Is this slot inlined?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined: Option<bool>,
    
    /// Is this slot inlined as list?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_as_list: Option<bool>,
}

/// Type definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TypeDefinition {
    /// Name of the type
    pub name: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Base type
    #[serde(skip_serializing_if = "Option::is_none", rename = "typeof")]
    pub base_type: Option<String>,
    
    /// Type URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    
    /// Pattern constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    
    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_value: Option<Value>,
    
    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_value: Option<Value>,
}

/// Enum definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumDefinition {
    /// Name of the enum
    pub name: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Permissible values
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissible_values: Vec<PermissibleValue>,
    
    /// Code set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_set: Option<String>,
    
    /// Code set tag
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_set_tag: Option<String>,
    
    /// Code set version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_set_version: Option<String>,
}

/// Permissible value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissibleValue {
    /// Simple string value
    Simple(String),
    /// Complex value with metadata
    Complex {
        /// The text value
        text: String,
        /// Description
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<String>,
        /// Meaning URI
        #[serde(skip_serializing_if = "Option::is_none")]
        meaning: Option<String>,
    },
}

/// Prefix definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PrefixDefinition {
    /// Simple string expansion
    Simple(String),
    /// Complex prefix with reference
    Complex {
        /// Prefix expansion
        prefix_prefix: String,
        /// Reference URL
        #[serde(skip_serializing_if = "Option::is_none")]
        prefix_reference: Option<String>,
    },
}

/// Subset definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SubsetDefinition {
    /// Name of the subset
    pub name: String,
    
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Validation report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Is the data valid?
    pub valid: bool,
    
    /// Validation errors
    #[serde(default)]
    pub errors: Vec<ValidationError>,
    
    /// Validation warnings
    #[serde(default)]
    pub warnings: Vec<ValidationWarning>,
    
    /// Validation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    
    /// Schema used for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_id: Option<String>,
}

/// Validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Error message
    pub message: String,
    
    /// Path to the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Expected value/type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    
    /// Actual value found
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    
    /// Error severity
    #[serde(default)]
    pub severity: Severity,
}

/// Validation warning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// Warning message
    pub message: String,
    
    /// Path to the warning
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    
    /// Suggestion for fixing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Error level (default)
    #[default]
    Error,
    /// Warning level
    Warning,
    /// Info level
    Info,
}

/// Named captures from pattern matching
pub type NamedCaptures = HashMap<String, String>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_serialization() {
        let schema = SchemaDefinition {
            id: "https://example.org/test".to_string(),
            name: "test_schema".to_string(),
            ..Default::default()
        };
        
        let json = serde_json::to_string(&schema).unwrap();
        assert!(json.contains("test_schema"));
        
        let parsed: SchemaDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test_schema");
    }

    #[test]
    fn test_permissible_value() {
        let simple = PermissibleValue::Simple("test".to_string());
        let json = serde_json::to_string(&simple).unwrap();
        assert_eq!(json, r#""test""#);
        
        let complex = PermissibleValue::Complex {
            text: "test".to_string(),
            description: Some("A test value".to_string()),
            meaning: None,
        };
        let json = serde_json::to_string(&complex).unwrap();
        assert!(json.contains("description"));
    }
}