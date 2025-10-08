//! Core type definitions for LinkML schemas and data

use crate::annotations::{Annotatable, Annotations};
use crate::settings::SchemaSettings;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Schema definition - the root of a `LinkML` schema
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

    /// Schema settings for controlling validation and generation behavior
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<SchemaSettings>,

    /// Annotations for the schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Contributors to this schema
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<crate::metadata::Contributor>,

    /// Schema status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Schema categories
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,

    /// Schema keywords
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// See also references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,
}

/// Options for handling recursive class references
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecursionOptions {
    /// Use Box for recursive references to prevent infinite size
    pub use_box: bool,
    /// Maximum recursion depth for validation
    pub max_depth: Option<usize>,
}

/// Class definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassDefinition {
    /// Name of the class
    #[serde(default)]
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Is this class abstract?
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
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

    /// Rules for class-level validation
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<Rule>,

    /// Conditional requirements (`if_required/then_required`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_required: Option<IndexMap<String, ConditionalRequirement>>,

    /// Unique key constraints
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub unique_keys: IndexMap<String, UniqueKeyDefinition>,

    /// Annotations for the class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    /// Recursion handling options for self-referential classes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursion_options: Option<RecursionOptions>,

    // Metadata fields
    /// Alternative names
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,

    /// See also references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,

    /// Examples
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<crate::metadata::Example>,

    /// Whether deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,

    /// Todos
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<String>,

    /// Notes
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    /// Comments
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,

    // Mapping fields
    /// Exact mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_mappings: Vec<String>,

    /// Close mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub close_mappings: Vec<String>,

    /// Related mappings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_mappings: Vec<String>,

    /// Narrow mappings (more specific terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub narrow_mappings: Vec<String>,

    /// Broad mappings (more general terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broad_mappings: Vec<String>,
}

/// Action to take when a slot value is absent
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IfAbsentAction {
    /// Set to the slot name
    SlotName,
    /// Set to the class name plus slot name
    ClassSlotCurie,
    /// Set to the class name
    ClassName,
    /// Generate a unique identifier
    Bnode,
    /// Use the default value
    DefaultValue,
    /// Use a custom string value
    String(String),
    /// Use the current date
    Date,
    /// Use the current datetime
    Datetime,
    /// Generate an integer
    Int(i64),
    /// Evaluate an expression
    Expression(String),
}

// Custom deserialization for IfAbsentAction to handle both tagged and untagged formats
impl<'de> serde::Deserialize<'de> for IfAbsentAction {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Visitor};

        struct IfAbsentActionVisitor;

        impl<'de> Visitor<'de> for IfAbsentActionVisitor {
            type Value = IfAbsentAction;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an ifabsent action (string or tagged enum)")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<IfAbsentAction, E>
            where
                E: de::Error,
            {
                // Try to match known variants first
                match value {
                    "slot_name" => Ok(IfAbsentAction::SlotName),
                    "class_slot_curie" => Ok(IfAbsentAction::ClassSlotCurie),
                    "class_name" => Ok(IfAbsentAction::ClassName),
                    "bnode" => Ok(IfAbsentAction::Bnode),
                    "default_value" => Ok(IfAbsentAction::DefaultValue),
                    "date" => Ok(IfAbsentAction::Date),
                    "datetime" => Ok(IfAbsentAction::Datetime),
                    // For any other string, treat as a string expression
                    _ => Ok(IfAbsentAction::String(value.to_string())),
                }
            }

            fn visit_i64<E>(self, value: i64) -> std::result::Result<IfAbsentAction, E>
            where
                E: de::Error,
            {
                Ok(IfAbsentAction::Int(value))
            }

            fn visit_map<M>(self, map: M) -> std::result::Result<IfAbsentAction, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                // For tagged format like { "string": "value" } or { "int": 42 }
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                enum Tagged {
                    SlotName,
                    ClassSlotCurie,
                    ClassName,
                    Bnode,
                    DefaultValue,
                    String(String),
                    Date,
                    Datetime,
                    Int(i64),
                    Expression(String),
                }

                let tagged = Tagged::deserialize(de::value::MapAccessDeserializer::new(map))?;
                Ok(match tagged {
                    Tagged::SlotName => IfAbsentAction::SlotName,
                    Tagged::ClassSlotCurie => IfAbsentAction::ClassSlotCurie,
                    Tagged::ClassName => IfAbsentAction::ClassName,
                    Tagged::Bnode => IfAbsentAction::Bnode,
                    Tagged::DefaultValue => IfAbsentAction::DefaultValue,
                    Tagged::String(s) => IfAbsentAction::String(s),
                    Tagged::Date => IfAbsentAction::Date,
                    Tagged::Datetime => IfAbsentAction::Datetime,
                    Tagged::Int(i) => IfAbsentAction::Int(i),
                    Tagged::Expression(e) => IfAbsentAction::Expression(e),
                })
            }
        }

        deserializer.deserialize_any(IfAbsentActionVisitor)
    }
}

/// Slot definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SlotDefinition {
    /// Name of the slot
    #[serde(default)]
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

    /// Is this slot a key (unique within its container)?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<bool>,

    /// Is this slot readonly (cannot be modified after creation)?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readonly: Option<bool>,

    /// Pattern for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_value: Option<Value>,

    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_value: Option<Value>,

    /// Minimum length for string values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,

    /// Maximum length for string values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,

    /// Permissible values
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissible_values: Vec<PermissibleValue>,

    /// Slot URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_uri: Option<String>,

    /// Default value or action when value is absent
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifabsent: Option<IfAbsentAction>,

    /// Aliases
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,

    /// Domain - the class(es) that this slot can be applied to
    /// Defines the type of the subject of the slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,

    /// Is inherited (`is_a`)
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

    /// `any_of` constraint - at least one must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,

    /// `all_of` constraint - all must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,

    /// `exactly_one_of` constraint - exactly one must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,

    /// `none_of` constraint - none can be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<AnonymousSlotExpression>>,

    /// Expression that computes the value of this slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_expression: Option<String>,

    /// Expression that must evaluate to true for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,

    /// List of allowed string values (`equals_string_in` constraint)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_string_in: Option<Vec<String>>,

    /// Structured pattern validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_pattern: Option<StructuredPattern>,

    /// Annotations for the slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,

    // Metadata fields
    /// See also references
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,

    /// Examples of valid values
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<crate::metadata::Example>,

    /// Whether this slot is deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,

    /// Todos for this slot
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<String>,

    /// Notes about this slot
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    /// Comments about this slot
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,

    /// Rank for ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i32>,

    /// Whether values in this slot must be unique
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique: Option<bool>,

    /// Whether values in this slot are ordered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ordered: Option<bool>,

    /// Unique key constraints for this slot
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unique_keys: Vec<String>,

    // Mapping fields
    /// Exact mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_mappings: Vec<String>,

    /// Close mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub close_mappings: Vec<String>,

    /// Related mappings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_mappings: Vec<String>,

    /// Narrow mappings (more specific terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub narrow_mappings: Vec<String>,

    /// Broad mappings (more general terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broad_mappings: Vec<String>,
}

/// Structured pattern for advanced pattern matching
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructuredPattern {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Whether to interpolate variables in the pattern
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolated: Option<bool>,

    /// Partial match allowed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_match: Option<bool>,
}

/// Type definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TypeDefinition {
    /// Name of the type
    #[serde(default)]
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

    /// Annotations for the type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Enum definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EnumDefinition {
    /// Name of the enum
    #[serde(default)]
    pub name: String,

    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Permissible values
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_permissible_values"
    )]
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

    /// Annotations for the enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Permissible value metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PermissibleValueMetadata {
    /// Description of this permissible value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Meaning URI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meaning: Option<String>,
    /// Title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

/// Permissible value (legacy enum, kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum PermissibleValue {
    /// Simple string value
    Simple(String),
    /// Complex value with metadata
    Complex {
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
    #[serde(default)]
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

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "Validation error at {}: {}", path, self.message)
        } else {
            write!(f, "Validation error: {}", self.message)
        }
    }
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

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "Validation warning at {}: {}", path, self.message)
        } else {
            write!(f, "Validation warning: {}", self.message)
        }
    }
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

/// Anonymous slot expression for boolean constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct AnonymousSlotExpression {
    /// Range constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,

    /// Pattern constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_value: Option<Value>,

    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_value: Option<Value>,

    /// Minimum cardinality
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_cardinality: Option<i32>,

    /// Maximum cardinality
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_cardinality: Option<i32>,

    /// Required constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Recommended constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recommended: Option<bool>,

    /// Multivalued constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multivalued: Option<bool>,

    /// Permissible values
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissible_values: Vec<PermissibleValue>,

    /// Inlined constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined: Option<bool>,

    /// Inlined as list constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inlined_as_list: Option<bool>,

    /// Nested `any_of` constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,

    /// Nested `all_of` constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,

    /// Nested `exactly_one_of` constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,

    /// Nested `none_of` constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<AnonymousSlotExpression>>,
}

/// Rule definition for class-level validation
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Rule {
    /// Human-readable description of the rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Title for the rule
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Whether this rule is deactivated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deactivated: Option<bool>,

    /// Priority for rule execution (higher = earlier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i32>,

    /// Conditions that must be met for rule to apply (IF)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preconditions: Option<RuleConditions>,

    /// Conditions that must be satisfied when preconditions match (THEN)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postconditions: Option<RuleConditions>,

    /// Alternative conditions when preconditions don't match (ELSE)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub else_conditions: Option<RuleConditions>,
}

/// Conditions used in rules
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RuleConditions {
    /// Conditions on specific slots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_conditions: Option<IndexMap<String, SlotCondition>>,

    /// Expression-based conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expression_conditions: Option<Vec<String>>,

    /// Composite conditions (boolean combinations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub composite_conditions: Option<CompositeConditions>,
}

/// Condition on a specific slot
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SlotCondition {
    /// Expected range/type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<String>,

    /// Whether the slot is required
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    /// Pattern constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,

    /// Exact string match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_string: Option<String>,

    /// Exact number match
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_number: Option<f64>,

    /// Expression that computes expected value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_expression: Option<String>,

    /// Minimum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_value: Option<Value>,

    /// Maximum value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_value: Option<Value>,

    /// `any_of` constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,

    /// `all_of` constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,

    /// `exactly_one_of` constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,

    /// `none_of` constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<AnonymousSlotExpression>>,
}

/// Composite conditions for boolean logic
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CompositeConditions {
    /// At least one condition must be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<RuleConditions>>,

    /// All conditions must be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<RuleConditions>>,

    /// Exactly one condition must be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<RuleConditions>>,

    /// No conditions can be true
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<RuleConditions>>,
}

/// Conditional requirement specification
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ConditionalRequirement {
    /// Condition that triggers the requirement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<SlotCondition>,

    /// Slots that become required when condition is met
    #[serde(skip_serializing_if = "Option::is_none")]
    pub then_required: Option<Vec<String>>,
}

/// Unique key definition for ensuring uniqueness constraints
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UniqueKeyDefinition {
    /// Description of this unique key constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// List of slot names that together form the unique key
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unique_key_slots: Vec<String>,

    /// Whether to consider null values as inequal (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consider_nulls_inequal: Option<bool>,
}

impl SchemaDefinition {
    /// Create a new schema definition with the given name
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            id: format!("https://example.org/{name}"),
            name,
            ..Default::default()
        }
    }
}

impl Annotatable for SchemaDefinition {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }

    fn annotations_mut(&mut self) -> Option<&mut Annotations> {
        self.annotations.as_mut()
    }
}

impl ClassDefinition {
    /// Create a new class definition with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

impl Annotatable for ClassDefinition {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }

    fn annotations_mut(&mut self) -> Option<&mut Annotations> {
        self.annotations.as_mut()
    }
}

impl SlotDefinition {
    /// Create a new slot definition with the given name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }
}

impl Annotatable for SlotDefinition {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }

    fn annotations_mut(&mut self) -> Option<&mut Annotations> {
        self.annotations.as_mut()
    }
}

impl Annotatable for TypeDefinition {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }

    fn annotations_mut(&mut self) -> Option<&mut Annotations> {
        self.annotations.as_mut()
    }
}

impl Annotatable for EnumDefinition {
    fn annotations(&self) -> Option<&Annotations> {
        self.annotations.as_ref()
    }

    fn annotations_mut(&mut self) -> Option<&mut Annotations> {
        self.annotations.as_mut()
    }
}

/// Custom deserializer for `permissible_values` that handles both map and sequence formats
fn deserialize_permissible_values<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<PermissibleValue>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct PermissibleValuesVisitor;

    impl<'de> Visitor<'de> for PermissibleValuesVisitor {
        type Value = Vec<PermissibleValue>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence or map of permissible values")
        }

        // Handle sequence format: ["value1", "value2"]
        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Vec<PermissibleValue>, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element::<PermissibleValue>()? {
                values.push(value);
            }
            Ok(values)
        }

        // Handle map format: { "value1": null, "value2": { "description": "..." } }
        fn visit_map<M>(self, mut map: M) -> std::result::Result<Vec<PermissibleValue>, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some((key, value)) = map.next_entry::<String, Option<PermissibleValueMetadata>>()? {
                let pv = if let Some(metadata) = value {
                    PermissibleValue::Complex {
                        text: key,
                        description: metadata.description,
                        meaning: metadata.meaning,
                    }
                } else {
                    PermissibleValue::Simple(key)
                };
                values.push(pv);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_any(PermissibleValuesVisitor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_serialization() -> crate::Result<()> {
        let schema = SchemaDefinition {
            id: "https://example.org/test".to_string(),
            name: "test_schema".to_string(),
            ..Default::default()
        };

        let json = serde_json::to_string(&schema)?;
        assert!(json.contains("test_schema"));

        let parsed: SchemaDefinition = serde_json::from_str(&json)?;
        assert_eq!(parsed.name, "test_schema");
        Ok(())
    }

    #[test]
    fn test_permissible_value() -> crate::Result<()> {
        let simple = PermissibleValue::Simple("test".to_string());
        let json = serde_json::to_string(&simple)?;
        assert_eq!(json, r#""test""#);

        let complex = PermissibleValue::Complex {
            text: "test".to_string(),
            description: Some("A test value".to_string()),
            meaning: None,
        };
        let json = serde_json::to_string(&complex)?;
        assert!(json.contains("description"));
        Ok(())
    }
}
