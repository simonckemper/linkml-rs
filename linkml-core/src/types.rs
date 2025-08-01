//! Core type definitions for LinkML schemas and data

use crate::annotations::{Annotatable, Annotations};
use crate::settings::SchemaSettings;
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

/// Class definition
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ClassDefinition {
    /// Name of the class
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
    
    /// Conditional requirements (if_required/then_required)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_required: Option<IndexMap<String, ConditionalRequirement>>,
    
    /// Unique key constraints
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub unique_keys: IndexMap<String, UniqueKeyDefinition>,
    
    /// Annotations for the class
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
    
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
    
    /// any_of constraint - at least one must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// all_of constraint - all must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// exactly_one_of constraint - exactly one must be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// none_of constraint - none can be satisfied
    #[serde(skip_serializing_if = "Option::is_none")]
    pub none_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// Expression that computes the value of this slot
    #[serde(skip_serializing_if = "Option::is_none")]
    pub equals_expression: Option<String>,
    
    /// Expression that must evaluate to true for validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<String>>,
    
    /// List of allowed string values (equals_string_in constraint)
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
}

/// Structured pattern for advanced pattern matching
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StructuredPattern {
    /// The pattern syntax (e.g., "regular_expression", "glob")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub syntax: Option<String>,
    
    /// The pattern itself
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
    
    /// Annotations for the enum
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
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
    
    /// Nested any_of constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// Nested all_of constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// Nested exactly_one_of constraints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// Nested none_of constraints
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
    
    /// any_of constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub any_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// all_of constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub all_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// exactly_one_of constraint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exactly_one_of: Option<Vec<AnonymousSlotExpression>>,
    
    /// none_of constraint
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
            id: format!("https://example.org/{}", name),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_serialization() -> Result<(), Box<dyn std::error::Error>> {
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
    fn test_permissible_value() -> Result<(), Box<dyn std::error::Error>> {
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