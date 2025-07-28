//! Optimized type definitions using string interning
//!
//! This module provides memory-optimized versions of LinkML types that use
//! Arc<str> for commonly duplicated strings to reduce memory usage.

use std::sync::Arc;
use std::collections::HashMap;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::string_pool::{intern, intern_option, intern_vec};
use crate::types::{
    StructuredPattern, AnonymousSlotExpression,
    PermissibleValue, UniqueKeyDefinition, PrefixDefinition
};
use crate::annotations::Annotation;
use crate::metadata::Example;

/// Memory-optimized Schema Definition using interned strings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinitionV2 {
    // Core identification - always interned
    pub id: Arc<str>,
    pub name: Arc<str>,
    
    // Common references - always interned
    pub default_prefix: Option<Arc<str>>,
    pub default_range: Option<Arc<str>>,
    pub metamodel_version: Option<Arc<str>>,
    pub status: Option<Arc<str>>,
    
    // Lists of references - always interned
    pub imports: Vec<Arc<str>>,
    pub categories: Vec<Arc<str>>,
    pub keywords: Vec<Arc<str>>,
    pub see_also: Vec<Arc<str>>,
    
    // Potentially unique strings - not interned by default
    pub title: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub license: Option<String>,
    pub generation_date: Option<String>,
    pub source_file: Option<String>,
    
    // Complex types
    pub prefixes: IndexMap<Arc<str>, PrefixDefinitionV2>,
    pub classes: IndexMap<Arc<str>, ClassDefinitionV2>,
    pub slots: IndexMap<Arc<str>, SlotDefinitionV2>,
    pub types: IndexMap<Arc<str>, TypeDefinitionV2>,
    pub enums: IndexMap<Arc<str>, EnumDefinitionV2>,
    pub subsets: IndexMap<Arc<str>, SubsetDefinitionV2>,
    
    // Settings and metadata
    pub settings: Option<SchemaSettingsV2>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub contributors: Vec<ContributorV2>,
    
    // Other fields
    pub created_by: Option<Arc<str>>,
    pub created_on: Option<String>,
    pub last_updated_on: Option<String>,
    pub modified_by: Option<Arc<str>>,
    pub rank: Option<i32>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
}

/// Memory-optimized Class Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassDefinitionV2 {
    // Core identification
    pub name: Arc<str>,
    pub class_uri: Option<Arc<str>>,
    
    // References - always interned
    pub is_a: Option<Arc<str>>,
    pub mixins: Vec<Arc<str>>,
    pub slots: Vec<Arc<str>>,
    pub subclass_of: Vec<Arc<str>>,
    pub union_of: Vec<Arc<str>>,
    pub disjoint_with: Vec<Arc<str>>,
    
    // Potentially unique strings
    pub description: Option<String>,
    pub title: Option<String>,
    pub deprecated: Option<String>,
    
    // Lists that might be unique
    pub aliases: Vec<String>,
    pub notes: Vec<String>,
    pub comments: Vec<String>,
    pub todos: Vec<String>,
    
    // Boolean flags
    pub abstract_: Option<bool>,
    pub mixin: Option<bool>,
    pub values_from: Vec<Arc<str>>,
    pub id_prefixes: Vec<Arc<str>>,
    
    // Other fields
    pub see_also: Vec<Arc<str>>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
    pub imported_from: Option<Arc<str>>,
    pub source: Option<Arc<str>>,
    pub in_language: Option<Arc<str>>,
    pub rank: Option<i32>,
}

/// Memory-optimized Slot Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotDefinitionV2 {
    // Core identification
    pub name: Arc<str>,
    pub slot_uri: Option<Arc<str>>,
    
    // Type references - always interned
    pub range: Option<Arc<str>>,
    pub is_a: Option<Arc<str>>,
    pub mixins: Vec<Arc<str>>,
    pub inverse: Option<Arc<str>>,
    pub domain: Option<Arc<str>>,
    pub subproperty_of: Option<Arc<str>>,
    pub symmetric: Option<Arc<str>>,
    
    // Patterns and expressions - often repeated
    pub pattern: Option<Arc<str>>,
    pub equals_expression: Option<Arc<str>>,
    pub equals_string_in: Option<Vec<Arc<str>>>,
    
    // Potentially unique strings
    pub description: Option<String>,
    pub title: Option<String>,
    pub deprecated: Option<String>,
    
    // Lists that might be unique
    pub aliases: Vec<String>,
    pub notes: Vec<String>,
    pub comments: Vec<String>,
    pub todos: Vec<String>,
    
    // Boolean and numeric properties
    pub required: Option<bool>,
    pub recommended: Option<bool>,
    pub multivalued: Option<bool>,
    pub inlined: Option<bool>,
    pub inlined_as_list: Option<bool>,
    pub key: Option<bool>,
    pub identifier: Option<bool>,
    pub designates_type: Option<bool>,
    pub alias: Option<bool>,
    pub owner: Option<Arc<str>>,
    pub readonly: Option<String>,
    pub ifabsent: Option<String>,
    pub list_elements_unique: Option<bool>,
    pub list_elements_ordered: Option<bool>,
    pub shared: Option<bool>,
    pub locally_defined: Option<bool>,
    pub asymmetric: Option<bool>,
    pub reflexive: Option<bool>,
    pub irreflexive: Option<bool>,
    pub transitive: Option<bool>,
    
    // Numeric constraints
    pub minimum_value: Option<Value>,
    pub maximum_value: Option<Value>,
    pub minimum_cardinality: Option<i32>,
    pub maximum_cardinality: Option<i32>,
    
    // Other references
    pub see_also: Vec<Arc<str>>,
    pub values_from: Vec<Arc<str>>,
    pub id_prefixes: Vec<Arc<str>>,
    
    // Complex types
    pub structured_pattern: Option<StructuredPattern>,
    pub examples: Vec<Example>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
    pub imported_from: Option<Arc<str>>,
    pub source: Option<Arc<str>>,
    pub in_language: Option<Arc<str>>,
    pub rank: Option<i32>,
}

/// Memory-optimized Type Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDefinitionV2 {
    pub name: Arc<str>,
    pub uri: Option<Arc<str>>,
    pub base: Option<Arc<str>>,
    #[serde(rename = "typeof")]
    pub base_type: Option<Arc<str>>,
    pub description: Option<String>,
    pub pattern: Option<Arc<str>>,
    pub minimum_value: Option<Value>,
    pub maximum_value: Option<Value>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
    pub imported_from: Option<Arc<str>>,
    pub rank: Option<i32>,
}

/// Memory-optimized Enum Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumDefinitionV2 {
    pub name: Arc<str>,
    pub enum_uri: Option<Arc<str>>,
    pub code_set: Option<Arc<str>>,
    pub code_set_tag: Option<Arc<str>>,
    pub code_set_version: Option<Arc<str>>,
    pub pv_formula: Option<Arc<str>>,
    pub description: Option<String>,
    pub title: Option<String>,
    pub deprecated: Option<String>,
    pub permissible_values: IndexMap<String, PermissibleValue>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
    pub rank: Option<i32>,
}

/// Memory-optimized Subset Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubsetDefinitionV2 {
    pub name: Arc<str>,
    pub description: Option<String>,
    pub annotations: Option<HashMap<String, Annotation>>,
    pub extensions: HashMap<String, Value>,
    pub from_schema: Option<Arc<str>>,
    pub rank: Option<i32>,
}

/// Memory-optimized Prefix Definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixDefinitionV2 {
    pub prefix_prefix: Arc<str>,
    pub prefix_reference: Arc<str>,
}

/// Memory-optimized Schema Settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaSettingsV2 {
    pub search_paths: Vec<Arc<str>>,
    pub base_url: Option<Arc<str>>,
    pub aliases: HashMap<Arc<str>, Arc<str>>,
    pub slot_range: Option<Arc<str>>,
    pub package_name: Option<Arc<str>>,
    pub imports: Vec<Arc<str>>,
    pub type_mappings: HashMap<Arc<str>, Arc<str>>,
    pub features: Vec<Arc<str>>,
}

/// Memory-optimized Contributor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorV2 {
    pub name: Arc<str>,
    pub email: Option<Arc<str>>,
    pub github: Option<Arc<str>>,
    pub orcid: Option<Arc<str>>,
    pub role: Option<Arc<str>>,
}

/// Conversion functions from V1 to V2 types
impl From<crate::types::SchemaDefinition> for SchemaDefinitionV2 {
    fn from(v1: crate::types::SchemaDefinition) -> Self {
        Self {
            id: intern(&v1.id),
            name: intern(&v1.name),
            default_prefix: intern_option(v1.default_prefix.as_deref()),
            default_range: intern_option(v1.default_range.as_deref()),
            metamodel_version: intern_option(v1.metamodel_version.as_deref()),
            status: intern_option(v1.status.as_deref()),
            imports: intern_vec(v1.imports),
            categories: intern_vec(v1.categories),
            keywords: intern_vec(v1.keywords),
            see_also: intern_vec(v1.see_also),
            
            // Keep potentially unique strings as-is
            title: v1.title,
            description: v1.description,
            version: v1.version,
            license: v1.license,
            generation_date: v1.generation_date,
            source_file: v1.source_file,
            
            // Convert complex types
            prefixes: v1.prefixes.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            classes: v1.classes.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            slots: v1.slots.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            types: v1.types.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            enums: v1.enums.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            subsets: v1.subsets.into_iter()
                .map(|(k, v)| (intern(&k), v.into()))
                .collect(),
            
            settings: v1.settings.map(Into::into),
            annotations: v1.annotations,
            contributors: v1.contributors.into_iter().map(Into::into).collect(),
            
            created_by: intern_option(v1.created_by.as_deref()),
            created_on: v1.created_on,
            last_updated_on: v1.last_updated_on,
            modified_by: intern_option(v1.modified_by.as_deref()),
            rank: v1.rank,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
        }
    }
}

impl From<crate::types::ClassDefinition> for ClassDefinitionV2 {
    fn from(v1: crate::types::ClassDefinition) -> Self {
        Self {
            name: intern(&v1.name),
            class_uri: intern_option(v1.class_uri.as_deref()),
            is_a: intern_option(v1.is_a.as_deref()),
            mixins: intern_vec(v1.mixins),
            slots: intern_vec(v1.slots),
            subclass_of: intern_vec(v1.subclass_of),
            union_of: intern_vec(v1.union_of),
            disjoint_with: intern_vec(v1.disjoint_with),
            
            description: v1.description,
            title: v1.title,
            deprecated: v1.deprecated,
            aliases: v1.aliases,
            notes: v1.notes,
            comments: v1.comments,
            todos: v1.todos,
            
            abstract_: v1.abstract_,
            mixin: v1.mixin,
            values_from: intern_vec(v1.values_from),
            id_prefixes: intern_vec(v1.id_prefixes),
            see_also: intern_vec(v1.see_also),
            
            annotations: v1.annotations,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
            imported_from: intern_option(v1.imported_from.as_deref()),
            source: intern_option(v1.source.as_deref()),
            in_language: intern_option(v1.in_language.as_deref()),
            rank: v1.rank,
        }
    }
}

impl From<crate::types::SlotDefinition> for SlotDefinitionV2 {
    fn from(v1: crate::types::SlotDefinition) -> Self {
        Self {
            name: intern(&v1.name),
            slot_uri: intern_option(v1.slot_uri.as_deref()),
            range: intern_option(v1.range.as_deref()),
            is_a: intern_option(v1.is_a.as_deref()),
            mixins: intern_vec(v1.mixins),
            inverse: intern_option(v1.inverse.as_deref()),
            domain: intern_option(v1.domain.as_deref()),
            subproperty_of: intern_option(v1.subproperty_of.as_deref()),
            symmetric: intern_option(v1.symmetric.as_deref()),
            
            pattern: intern_option(v1.pattern.as_deref()),
            equals_expression: intern_option(v1.equals_expression.as_deref()),
            equals_string_in: v1.equals_string_in.map(intern_vec),
            
            description: v1.description,
            title: v1.title,
            deprecated: v1.deprecated,
            aliases: v1.aliases,
            notes: v1.notes,
            comments: v1.comments,
            todos: v1.todos,
            
            required: v1.required,
            recommended: v1.recommended,
            multivalued: v1.multivalued,
            inlined: v1.inlined,
            inlined_as_list: v1.inlined_as_list,
            key: v1.key,
            identifier: v1.identifier,
            designates_type: v1.designates_type,
            alias: v1.alias,
            owner: intern_option(v1.owner.as_deref()),
            readonly: v1.readonly,
            ifabsent: v1.ifabsent,
            list_elements_unique: v1.list_elements_unique,
            list_elements_ordered: v1.list_elements_ordered,
            shared: v1.shared,
            locally_defined: v1.locally_defined,
            asymmetric: v1.asymmetric,
            reflexive: v1.reflexive,
            irreflexive: v1.irreflexive,
            transitive: v1.transitive,
            
            minimum_value: v1.minimum_value,
            maximum_value: v1.maximum_value,
            minimum_cardinality: v1.minimum_cardinality,
            maximum_cardinality: v1.maximum_cardinality,
            
            see_also: intern_vec(v1.see_also),
            values_from: intern_vec(v1.values_from),
            id_prefixes: intern_vec(v1.id_prefixes),
            
            structured_pattern: v1.structured_pattern,
            examples: v1.examples,
            annotations: v1.annotations,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
            imported_from: intern_option(v1.imported_from.as_deref()),
            source: intern_option(v1.source.as_deref()),
            in_language: intern_option(v1.in_language.as_deref()),
            rank: v1.rank,
        }
    }
}

// Implement remaining conversions...
impl From<crate::types::TypeDefinition> for TypeDefinitionV2 {
    fn from(v1: crate::types::TypeDefinition) -> Self {
        Self {
            name: intern(&v1.name),
            uri: intern_option(v1.uri.as_deref()),
            base: intern_option(v1.base.as_deref()),
            base_type: intern_option(v1.base_type.as_deref()),
            description: v1.description,
            pattern: intern_option(v1.pattern.as_deref()),
            minimum_value: v1.minimum_value,
            maximum_value: v1.maximum_value,
            annotations: v1.annotations,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
            imported_from: intern_option(v1.imported_from.as_deref()),
            rank: v1.rank,
        }
    }
}

impl From<crate::types::EnumDefinition> for EnumDefinitionV2 {
    fn from(v1: crate::types::EnumDefinition) -> Self {
        Self {
            name: intern(&v1.name),
            enum_uri: intern_option(v1.enum_uri.as_deref()),
            code_set: intern_option(v1.code_set.as_deref()),
            code_set_tag: intern_option(v1.code_set_tag.as_deref()),
            code_set_version: intern_option(v1.code_set_version.as_deref()),
            pv_formula: intern_option(v1.pv_formula.as_deref()),
            description: v1.description,
            title: v1.title,
            deprecated: v1.deprecated,
            permissible_values: v1.permissible_values,
            annotations: v1.annotations,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
            rank: v1.rank,
        }
    }
}

impl From<crate::types::SubsetDefinition> for SubsetDefinitionV2 {
    fn from(v1: crate::types::SubsetDefinition) -> Self {
        Self {
            name: intern(&v1.name),
            description: v1.description,
            annotations: v1.annotations,
            extensions: v1.extensions,
            from_schema: intern_option(v1.from_schema.as_deref()),
            rank: v1.rank,
        }
    }
}

impl From<crate::types::PrefixDefinition> for PrefixDefinitionV2 {
    fn from(v1: crate::types::PrefixDefinition) -> Self {
        match v1 {
            crate::types::PrefixDefinition::Simple(s) => Self {
                prefix_prefix: intern(&s),
                prefix_reference: None,
            },
            crate::types::PrefixDefinition::Complex { prefix_prefix, prefix_reference } => Self {
                prefix_prefix: intern(&prefix_prefix),
                prefix_reference: intern_option(prefix_reference.as_deref()),
            },
        }
    }
}

impl From<crate::settings::SchemaSettings> for SchemaSettingsV2 {
    fn from(v1: crate::settings::SchemaSettings) -> Self {
        Self {
            search_paths: intern_vec(v1.search_paths),
            base_url: intern_option(v1.base_url.as_deref()),
            aliases: v1.aliases.into_iter()
                .map(|(k, v)| (intern(&k), intern(&v)))
                .collect(),
            slot_range: intern_option(v1.slot_range.as_deref()),
            package_name: intern_option(v1.package_name.as_deref()),
            imports: intern_vec(v1.imports),
            type_mappings: v1.type_mappings.into_iter()
                .map(|(k, v)| (intern(&k), intern(&v)))
                .collect(),
            features: intern_vec(v1.features),
        }
    }
}

impl From<crate::metadata::Contributor> for ContributorV2 {
    fn from(v1: crate::metadata::Contributor) -> Self {
        Self {
            name: intern(&v1.name),
            email: intern_option(v1.email.as_deref()),
            github: intern_option(v1.github.as_deref()),
            orcid: intern_option(v1.orcid.as_deref()),
            role: intern_option(v1.role.as_deref()),
        }
    }
}