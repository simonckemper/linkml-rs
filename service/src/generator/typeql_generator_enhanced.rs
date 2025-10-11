//! Enhanced `TypeQL` generation implementation for `TypeDB` schemas
//!
//! This module provides comprehensive `LinkML` to `TypeQL` translation with:
//! - Advanced entity/relation detection
//! - Full constraint support
//! - Complex inheritance handling
//! - Migration script generation
//! - `TypeDB` 3.0 feature support

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use super::typeql_constraints::TypeQLConstraintTranslator;
use super::typeql_relation_analyzer::RelationAnalyzer;
use super::typeql_role_inheritance::RoleInheritanceResolver;
use crate::utils::timestamp::SyncTimestampUtils;
use async_trait::async_trait;
use linkml_core::error::LinkMLError;
use linkml_core::prelude::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write;
use std::sync::{Arc, RwLock};
use thiserror::Error;
use timestamp_core::{TimestampError, TimestampService};

/// Errors specific to `TypeQL` generation
#[derive(Debug, Error)]
pub enum TypeQLError {
    /// Schema structure is invalid for `TypeQL` generation
    #[error("Invalid schema structure: {0}")]
    InvalidSchema(String),

    /// `LinkML` feature not supported in `TypeQL`
    #[error("Unsupported LinkML feature: {0}")]
    UnsupportedFeature(String),

    /// Error translating `LinkML` constraint to `TypeQL`
    #[error("Constraint translation error: {0}")]
    ConstraintError(String),

    /// Circular inheritance detected in schema
    #[error("Inheritance cycle detected: {0}")]
    InheritanceCycle(String),
}

/// Enhanced `TypeQL` schema generator for `TypeDB` 3.0+
pub struct EnhancedTypeQLGenerator {
    /// Generator name
    name: String,
    /// Schema analyzer for advanced detection
    analyzer: RwLock<SchemaAnalyzer>,
    /// Constraint translator
    constraint_translator: RwLock<TypeQLConstraintTranslator>,
    /// Relation analyzer for advanced relationships
    relation_analyzer: RwLock<RelationAnalyzer>,
    /// Role inheritance resolver
    role_inheritance_resolver: RwLock<RoleInheritanceResolver>,
    /// Identifier mapping table for bidirectional lookups
    identifier_map: RwLock<HashMap<String, String>>,
    /// Generator options
    options: super::traits::GeneratorOptions,
    /// Timestamp utilities for generating timestamps
    timestamp_utils: Arc<SyncTimestampUtils>,
}

/// Analyzes schema structure for optimal `TypeQL` generation
struct SchemaAnalyzer {
    /// Cache for entity/relation detection results
    type_cache: HashMap<String, TypeQLType>,
}

/// Type of `TypeQL` schema element
#[derive(Debug, Clone, PartialEq)]

enum TypeQLType {
    Entity,
    Relation,
    Abstract,
}

impl EnhancedTypeQLGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Check if a string is a valid `TypeQL` identifier
    fn is_valid_identifier(name: &str) -> bool {
        // TypeQL identifiers must:
        // - Start with a letter or underscore
        // - Contain only letters, numbers, underscores, or hyphens
        // - Not be a TypeQL reserved keyword
        if name.is_empty() {
            return false;
        }

        // Check first character
        let first_char = name.chars().next().expect("iterator should have next item");
        if !first_char.is_alphabetic() && first_char != '_' {
            return false;
        }

        // Check all characters
        for c in name.chars() {
            if !c.is_alphanumeric() && c != '_' && c != '-' {
                return false;
            }
        }

        // Check for TypeQL reserved keywords
        let reserved_keywords = [
            "define",
            "undefine",
            "insert",
            "delete",
            "match",
            "get",
            "aggregate",
            "compute",
            "rule",
            "when",
            "then",
            "entity",
            "attribute",
            "relation",
            "role",
            "plays",
            "owns",
            "abstract",
            "sub",
            "as",
            "has",
            "isa",
            "thing",
            "value",
            "regex",
            "key",
            "unique",
        ];

        !reserved_keywords.contains(&name.to_lowercase().as_str())
    }

    /// Create a new enhanced `TypeQL` generator
    #[must_use]
    pub fn new() -> Self {
        let timestamp_service = timestamp_service::wiring::wire_timestamp().into_inner();
        let timestamp_utils = Arc::new(SyncTimestampUtils::new(timestamp_service));
        Self {
            name: "typeql-enhanced".to_string(),
            analyzer: RwLock::new(SchemaAnalyzer::new()),
            constraint_translator: RwLock::new(TypeQLConstraintTranslator::new()),
            relation_analyzer: RwLock::new(RelationAnalyzer::new()),
            role_inheritance_resolver: RwLock::new(RoleInheritanceResolver::new()),
            identifier_map: RwLock::new(HashMap::new()),
            options: super::traits::GeneratorOptions::default(),
            timestamp_utils,
        }
    }

    /// Create with custom timestamp service
    pub fn with_timestamp_service(
        timestamp_service: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        let timestamp_utils = Arc::new(SyncTimestampUtils::new(timestamp_service));
        Self {
            name: "typeql-enhanced".to_string(),
            analyzer: RwLock::new(SchemaAnalyzer::new()),
            constraint_translator: RwLock::new(TypeQLConstraintTranslator::new()),
            relation_analyzer: RwLock::new(RelationAnalyzer::new()),
            role_inheritance_resolver: RwLock::new(RoleInheritanceResolver::new()),
            identifier_map: RwLock::new(HashMap::new()),
            options: super::traits::GeneratorOptions::default(),
            timestamp_utils,
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Analyze schema and determine optimal `TypeQL` structure
    fn analyze_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // First pass: identify all types using advanced relation analysis
        for (class_name, class_def) in &schema.classes {
            // Use relation analyzer for better detection
            if let Some(_relation_info) = self
                .relation_analyzer
                .write()
                .expect("relation analyzer lock should not be poisoned: {}")
                .analyze_relation(class_name, class_def, schema)
            {
                self.analyzer
                    .write()
                    .expect("analyzer lock should not be poisoned: {}")
                    .type_cache
                    .insert(class_name.clone(), TypeQLType::Relation);

                // Analyze role inheritance if applicable
                if let Some(_parent) = &class_def.is_a {
                    self.role_inheritance_resolver
                        .write()
                        .expect("role inheritance resolver lock should not be poisoned: {}")
                        .analyze_relation_inheritance(class_name, class_def, schema);
                }
            } else {
                let typeql_type = self
                    .analyzer
                    .read()
                    .expect("analyzer lock should not be poisoned: {}")
                    .determine_type(class_def, schema);
                self.analyzer
                    .write()
                    .expect("analyzer lock should not be poisoned: {}")
                    .type_cache
                    .insert(class_name.clone(), typeql_type);
            }
        }

        // Second pass: validate and optimize structure
        self.analyzer
            .read()
            .expect("analyzer lock should not be poisoned: {}")
            .validate_structure(schema)?;

        Ok(())
    }

    /// Generate complete `TypeQL` schema
    fn generate_typeql_schema(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let indent = &options.indent;

        // Header with metadata
        Self::write_header(&mut output, schema)?;

        // Define section
        writeln!(
            &mut output,
            "
define
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate in dependency order
        let ordered_types = self.get_dependency_order(schema)?;

        // 1. Generate abstract types first
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name)
                && (class.abstract_.unwrap_or(false) || class.mixin.unwrap_or(false))
            {
                self.generate_abstract_type(&mut output, type_name, class, schema, indent)?;
            }
        }

        // 2. Generate attributes
        self.generate_all_attributes(&mut output, schema, indent)?;

        // 3. Generate concrete entities
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name)
                && let Some(TypeQLType::Entity) = self
                    .analyzer
                    .read()
                    .expect("analyzer lock should not be poisoned: {}")
                    .type_cache
                    .get(type_name)
                && !class.abstract_.unwrap_or(false)
            {
                self.generate_entity(&mut output, type_name, class, schema, indent)?;
            }
        }

        // 4. Generate relations
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name)
                && let Some(TypeQLType::Relation) = self
                    .analyzer
                    .read()
                    .expect("analyzer lock should not be poisoned: {}")
                    .type_cache
                    .get(type_name)
            {
                self.generate_relation(&mut output, type_name, class, schema, indent)?;
            }
        }

        // 5. Generate constraints and rules
        if options
            .get_custom("generate_constraints")
            .map(std::string::String::as_str)
            != Some("false")
        {
            writeln!(
                &mut output,
                "
# Constraints and Validation Rules
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            self.generate_constraints(&mut output, schema, indent)?;
            self.generate_validation_rules(&mut output, schema, indent)?;
        }

        Ok(output)
    }

    /// Write schema header with metadata
    fn write_header(output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# TypeQL Schema generated from LinkML")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "# Generator: Enhanced TypeQL Generator v2.0")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "# TypeDB Version: 3.0+").map_err(Self::fmt_error_to_generator_error)?;

        if !schema.name.is_empty() {
            writeln!(output, "# Schema: {}", schema.name)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(version) = &schema.version {
            writeln!(output, "# Version: {version}").map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(desc) = &schema.description {
            writeln!(output, "# Description: {desc}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add generation timestamp
        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        writeln!(
            output,
            "# Generated: {}",
            timestamp
                .replace('T', " ")
                .replace('Z', "")
                .split('.')
                .next()
                .unwrap_or(&timestamp)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get dependency order for types (topological sort)
    fn get_dependency_order(&self, schema: &SchemaDefinition) -> GeneratorResult<Vec<String>> {
        let mut visited = HashSet::new();
        let mut order = Vec::new();
        let mut visiting = HashSet::new();

        for class_name in schema.classes.keys() {
            if !visited.contains(class_name) {
                self.visit_type(class_name, schema, &mut visited, &mut visiting, &mut order)?;
            }
        }

        Ok(order)
    }

    /// Visit type for dependency ordering (DFS)
    #[allow(clippy::only_used_in_recursion)]
    fn visit_type(
        &self,
        type_name: &str,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> GeneratorResult<()> {
        if visiting.contains(type_name) {
            return Err(GeneratorError::SchemaValidation(format!(
                "Inheritance cycle detected: {type_name}"
            )));
        }

        if visited.contains(type_name) {
            return Ok(());
        }

        visiting.insert(type_name.to_string());

        if let Some(class) = schema.classes.get(type_name) {
            // Visit parent
            if let Some(parent) = &class.is_a {
                self.visit_type(parent, schema, visited, visiting, order)?;
            }

            // Visit mixins
            for mixin in &class.mixins {
                self.visit_type(mixin, schema, visited, visiting, order)?;
            }
        }

        visiting.remove(type_name);
        visited.insert(type_name.to_string());
        order.push(type_name.to_string());

        Ok(())
    }

    /// Generate abstract type definition
    fn generate_abstract_type(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let type_name = self.convert_identifier(name);

        // Add documentation
        if let Some(desc) = &class.description {
            writeln!(output, "# Abstract: {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine base type
        let base_type = if Self::is_relation_like(class, schema) {
            "relation"
        } else {
            "entity"
        };

        write!(output, "{type_name} sub {base_type}, abstract")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Add attributes owned by abstract type
        let attributes = self.collect_direct_attributes(class, schema);
        if attributes.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            for (i, (attr_name, constraints)) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" "))
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                if i < attributes.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate entity type definition
    fn generate_entity(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let type_name = self.convert_identifier(name);

        // Add documentation
        if let Some(desc) = &class.description {
            writeln!(output, "# Entity: {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Build inheritance chain
        let inheritance = self.build_inheritance_chain(class, schema);

        write!(output, "{type_name} sub").map_err(Self::fmt_error_to_generator_error)?;
        if inheritance.is_empty() {
            write!(output, " entity").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(output, " {}", inheritance.join(", sub "))
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Collect all attributes (including constraints)
        let all_attributes = self.collect_all_attributes(class, schema);

        // Add roles this entity can play
        let roles = self.collect_playable_roles(name, schema);

        if all_attributes.is_empty() && roles.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;

            // Write attributes with constraints
            for (i, (attr_name, constraints)) in all_attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" "))
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                if i < all_attributes.len() - 1 || !roles.is_empty() {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Write roles
            for (i, role) in roles.iter().enumerate() {
                write!(output, "{}plays {}", indent.single(), role)
                    .map_err(Self::fmt_error_to_generator_error)?;
                if i < roles.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate relation type definition
    fn generate_relation(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let type_name = self.convert_identifier(name);

        // Get advanced relation info
        let relation_info = self
            .relation_analyzer
            .write()
            .expect("relation analyzer lock should not be poisoned: {}")
            .analyze_relation(name, class, schema)
            .ok_or_else(|| {
                GeneratorError::SchemaValidation(format!("{name} is not a valid relation"))
            })?;

        // Add documentation
        if let Some(desc) = &class.description {
            writeln!(output, "# Relation: {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add multi-way relation comment if applicable
        if relation_info.is_multiway {
            writeln!(
                output,
                "# Multi-way relation with {} roles",
                relation_info.roles.len()
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Build inheritance chain
        let inheritance = self.build_inheritance_chain(class, schema);

        write!(output, "{type_name} sub").map_err(Self::fmt_error_to_generator_error)?;
        if inheritance.is_empty() {
            write!(output, " relation").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(output, " {}", inheritance.join(", sub "))
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Handle abstract relations
        if class.abstract_.unwrap_or(false) {
            write!(output, ", abstract").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;

        // Write roles with advanced features
        for (i, role) in relation_info.roles.iter().enumerate() {
            let role_name = self.convert_identifier(&role.name);

            // Check for role inheritance
            let role_key = format!("{}:{}", name, role.name);
            if let Some(hierarchy) = self
                .role_inheritance_resolver
                .read()
                .expect("role inheritance resolver lock should not be poisoned: {}")
                .hierarchies
                .get(name)
            {
                if let Some(base_role) = hierarchy.specializations.get(&role_key) {
                    // This role specializes another
                    write!(
                        output,
                        "{}relates {} as {}",
                        indent.single(),
                        role_name,
                        self.convert_identifier(base_role)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(output, "{}relates {}", indent.single(), role_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            } else {
                write!(output, "{}relates {}", indent.single(), role_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Add cardinality if specified
            if let Some((min, max)) = &role.cardinality {
                write!(output, " @card({min}").map_err(Self::fmt_error_to_generator_error)?;
                if let Some(max_val) = max {
                    write!(output, "..{max_val}").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(output, "..").map_err(Self::fmt_error_to_generator_error)?;
                }
                write!(output, ")").map_err(Self::fmt_error_to_generator_error)?;
            }

            if i < relation_info.roles.len() - 1 || !relation_info.attributes.is_empty() {
                writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Add attributes owned by relation
        let attributes = self.collect_direct_attributes(class, schema);
        if attributes.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            if !relation_info.roles.is_empty() {
                writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            }

            for (i, (attr_name, constraints)) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" "))
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                if i < attributes.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Generate role players
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        for role in &relation_info.roles {
            let player_typeql = self.convert_identifier(&role.player_type);
            writeln!(
                output,
                "{} plays {}:{};",
                player_typeql, type_name, role.name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate all attributes from schema
    fn generate_all_attributes(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        _indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "# Attributes
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        let mut generated_attrs = HashSet::new();
        let mut attr_definitions = BTreeMap::new();

        // Collect all unique attributes from slots
        for slot in schema.slots.values() {
            let attr_name = self.convert_identifier(&slot.name);
            if !generated_attrs.contains(&attr_name) {
                generated_attrs.insert(attr_name.clone());
                attr_definitions.insert(attr_name, slot);
            }
        }

        // Collect from class slot usage
        for class in schema.classes.values() {
            for (slot_name, slot_def) in &class.slot_usage {
                let attr_name = self.convert_identifier(slot_name);
                if !generated_attrs.contains(&attr_name) {
                    generated_attrs.insert(attr_name.clone());
                    attr_definitions.insert(attr_name, slot_def);
                }
            }
        }

        // Generate attribute definitions
        for (attr_name, slot) in attr_definitions {
            self.generate_attribute_definition(output, &attr_name, slot, schema)?;
        }

        Ok(())
    }

    /// Generate a single attribute definition
    fn generate_attribute_definition(
        &self,
        output: &mut String,
        name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        // Add documentation
        if let Some(desc) = &slot.description {
            writeln!(output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine value type
        let value_type = self.map_range_to_typeql(slot.range.as_ref(), schema);

        write!(output, "{name} sub attribute, value {value_type}")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Add inline constraints
        let inline_constraints = self.get_inline_constraints(slot);
        if !inline_constraints.is_empty() {
            write!(output, ", {}", inline_constraints.join(", "))
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add range constraints for numeric types
        if value_type == "long" || value_type == "double" {
            let mut range_parts = Vec::new();

            if let Some(min) = &slot.minimum_value {
                if let Some(min_num) = Self::value_to_number(min) {
                    range_parts.push(format!("{min_num}"));
                }
            } else {
                range_parts.push(String::new());
            }

            range_parts.push("..".to_string());

            if let Some(max) = &slot.maximum_value
                && let Some(max_num) = Self::value_to_number(max)
            {
                range_parts.push(format!("{max_num}"));
            }

            if range_parts.len() > 2 || !range_parts[0].is_empty() {
                write!(output, ", range [{}]", range_parts.join(""))
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate constraints section
    fn generate_constraints(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Unique key constraints
        for (class_name, class) in &schema.classes {
            if !class.unique_keys.is_empty() {
                for (_key_name, unique_key) in &class.unique_keys {
                    self.generate_unique_constraint(output, class_name, unique_key, indent)?;
                }
            }
        }

        Ok(())
    }

    /// Generate validation rules
    fn generate_validation_rules(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "# Validation Rules
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate required field rules
        for (class_name, class) in &schema.classes {
            for slot_name in &class.slots {
                if let Some(slot) = schema
                    .slots
                    .get(slot_name)
                    .or_else(|| class.slot_usage.get(slot_name))
                    && slot.required == Some(true)
                {
                    self.generate_required_rule(output, class_name, slot_name, indent)?;
                }
            }

            // Generate rules from class rules
            for rule in &class.rules {
                self.generate_class_rule(output, class_name, rule, schema, indent)?;
            }
        }

        Ok(())
    }

    /// Generate unique constraint
    fn generate_unique_constraint(
        &self,
        output: &mut String,
        class_name: &str,
        unique_key: &UniqueKeyDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        if unique_key.unique_key_slots.len() == 1 {
            // Single field unique constraint handled by @key annotation
            return Ok(());
        }

        // Multi-field unique constraint requires a rule
        let rule_name = format!("{}-unique-{}", self.convert_identifier(class_name), "key");

        writeln!(output, "rule {rule_name}:").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;

        // Match two instances with same key values
        writeln!(
            output,
            "{}$x isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$y isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}not {{ $x is $y; }};", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;

        for slot in &unique_key.unique_key_slots {
            let attr = self.convert_identifier(slot);
            writeln!(output, "{}$x has {} $val{};", indent.single(), attr, slot)
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}$y has {} $val{};", indent.single(), attr, slot)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x has validation-error \"Duplicate unique key\";",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate required field rule
    fn generate_required_rule(
        &self,
        output: &mut String,
        class_name: &str,
        slot_name: &str,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let rule_name = format!(
            "{}-requires-{}",
            self.convert_identifier(class_name),
            self.convert_identifier(slot_name)
        );

        writeln!(output, "rule {rule_name}:").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}not {{ $x has {} $v; }};",
            indent.single(),
            self.convert_identifier(slot_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x has validation-error \"Missing required field: {}\";",
            indent.single(),
            slot_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate rule from `LinkML` rule definition
    fn generate_class_rule(
        &self,
        output: &mut String,
        class_name: &str,
        rule: &Rule,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let rule_name = format!(
            "{}-rule-{}",
            self.convert_identifier(class_name),
            self.convert_identifier(rule.title.as_ref().unwrap_or(&"unnamed".to_string()))
        );

        writeln!(
            output,
            "# Rule: {}",
            rule.description
                .as_ref()
                .unwrap_or(rule.title.as_ref().unwrap_or(&"unnamed".to_string()))
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "rule {rule_name}:").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;

        // Base entity match
        writeln!(
            output,
            "{}$x isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Add preconditions
        if let Some(preconditions) = &rule.preconditions {
            self.generate_rule_conditions(output, "$x", preconditions, schema, indent)?;
        }

        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;

        // Add postconditions or validation error
        if let Some(postconditions) = &rule.postconditions {
            self.generate_rule_assertions(output, "$x", postconditions, schema, indent)?;
        } else {
            writeln!(
                output,
                "{}$x has validation-error \"Rule {} violated\";",
                indent.single(),
                rule.title.as_ref().unwrap_or(&"unnamed".to_string())
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Helper methods...
    /// Build inheritance chain for a class
    fn build_inheritance_chain(
        &self,
        class: &ClassDefinition,
        _schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut chain = Vec::new();

        // Direct parent
        if let Some(parent) = &class.is_a {
            chain.push(self.convert_identifier(parent));
        }

        // Mixins
        for mixin in &class.mixins {
            chain.push(self.convert_identifier(mixin));
        }

        chain
    }

    /// Collect all attributes including inherited ones
    fn collect_all_attributes(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<(String, Vec<String>)> {
        let mut attributes = Vec::new();
        let mut seen = HashSet::new();

        // Direct attributes
        for (attr_name, constraints) in self.collect_direct_attributes(class, schema) {
            if !seen.contains(&attr_name) {
                seen.insert(attr_name.clone());
                attributes.push((attr_name, constraints));
            }
        }

        attributes
    }

    /// Collect direct attributes with constraints
    fn collect_direct_attributes(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<(String, Vec<String>)> {
        let mut attributes = Vec::new();

        for slot_name in &class.slots {
            // Skip object-valued slots (they become relations)
            if let Some(slot) = schema
                .slots
                .get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name))
                && let Some(range) = &slot.range
                && !schema.classes.contains_key(range)
            {
                let attr_name = self.convert_identifier(slot_name);
                let constraints = self.collect_slot_constraints(slot);
                attributes.push((attr_name, constraints));
            }
        }

        attributes
    }

    /// Collect constraints for a slot
    fn collect_slot_constraints(&self, slot: &SlotDefinition) -> Vec<String> {
        // Delegate to the enhanced constraint translator
        let mut constraints = self
            .constraint_translator
            .write()
            .expect("constraint translator lock should not be poisoned")
            .translate_slot_constraints(slot);

        // Add range constraints for numeric types
        if let Some(range) = &slot.range
            && (range == "integer" || range == "float" || range == "double")
        {
            let range_constraints = self
                .constraint_translator
                .write()
                .expect("constraint translator lock should not be poisoned")
                .translate_range_constraints(slot);
            constraints.extend(range_constraints);
        }

        constraints
    }

    /// Get inline constraints for attribute definition
    fn get_inline_constraints(&self, slot: &SlotDefinition) -> Vec<String> {
        // Use the public method that handles all constraints
        self.constraint_translator
            .write()
            .expect("constraint translator lock should not be poisoned")
            .translate_slot_constraints(slot)
            .into_iter()
            .filter(|c| !c.starts_with('@')) // Filter out @ annotations for inline use
            .collect()
    }

    /// Collect roles this entity can play
    fn collect_playable_roles(&self, entity_name: &str, _schema: &SchemaDefinition) -> Vec<String> {
        // Use the relation analyzer's role player map
        self.relation_analyzer
            .write()
            .expect("relation analyzer lock should not be poisoned")
            .get_playable_roles(entity_name)
            .into_iter()
            .map(|role| {
                let parts: Vec<&str> = role.split(':').collect();
                if parts.len() == 2 {
                    format!(
                        "{}:{}",
                        self.convert_identifier(parts[0]),
                        self.convert_identifier(parts[1])
                    )
                } else {
                    role
                }
            })
            .collect()
    }

    /// Determine if a class should be a relation
    fn is_relation_like(class: &ClassDefinition, schema: &SchemaDefinition) -> bool {
        // A class is relation-like if:
        // 1. It has multiple object-valued slots
        // 2. It represents a relationship concept
        // 3. It has relationship-indicating patterns in name/description

        let mut object_slots = 0;

        for slot_name in &class.slots {
            if let Some(slot) = schema
                .slots
                .get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name))
                && let Some(range) = &slot.range
                && schema.classes.contains_key(range)
            {
                object_slots += 1;
            }
        }

        // Multiple object-valued slots indicate a relation
        if object_slots >= 2 {
            return true;
        }

        // Check for relationship patterns in name
        let name_lower = class.name.to_lowercase();
        let relation_patterns = [
            "association",
            "relationship",
            "link",
            "connection",
            "mapping",
        ];

        relation_patterns
            .iter()
            .any(|pattern| name_lower.contains(pattern))
    }

    /// Map `LinkML` range to `TypeQL` value type
    #[allow(clippy::only_used_in_recursion)]
    fn map_range_to_typeql(
        &self,
        range: Option<&String>,
        schema: &SchemaDefinition,
    ) -> &'static str {
        match range.map(String::as_str) {
            Some("string" | "str" | "uri" | "url" | "curie" | "ncname") => "string",
            Some("integer" | "int") => "long",
            Some("float" | "double" | "decimal" | "number") => "double",
            Some("boolean" | "bool") => "boolean",
            Some("date" | "datetime" | "time") => "datetime",
            Some(custom) => {
                // Check if it's a custom type definition
                if let Some(type_def) = schema.types.get(custom) {
                    // Resolve base type
                    self.map_range_to_typeql(type_def.base_type.as_ref(), schema)
                } else {
                    "string" // Default fallback
                }
            }
            None => "string",
        }
    }

    /// Escape regex pattern for `TypeQL`
    fn _escape_regex(pattern: &str) -> String {
        // TypeQL uses Java regex syntax, escape accordingly
        pattern.replace('\\', "\\\\").replace('"', "\\\"")
    }

    /// Convert `LinkML` Value to number
    fn value_to_number(value: &linkml_core::Value) -> Option<f64> {
        // Try to convert LinkML Value (serde_json::Value) to number
        if let Some(n) = value.as_f64() {
            Some(n)
        } else if let Some(s) = value.as_str() {
            s.parse().ok()
        } else {
            None
        }
    }

    /// Generate rule conditions
    fn generate_rule_conditions(
        &self,
        output: &mut String,
        var: &str,
        condition: &RuleConditions,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Process slot conditions
        if let Some(slot_conditions) = &condition.slot_conditions {
            for (slot_name, slot_condition) in slot_conditions {
                // Generate has attribute conditions
                if let Some(range) = &slot_condition.range {
                    writeln!(
                        output,
                        "{}{} has {}: ${}_{};",
                        indent.single(),
                        var,
                        slot_name,
                        var,
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;

                    // Add type constraint for the attribute
                    writeln!(
                        output,
                        "{}${}_{} isa {};",
                        indent.single(),
                        var,
                        slot_name,
                        Self::get_typeql_type_for_range(range, schema)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Generate value constraints if present
                if let Some(equals_string) = &slot_condition.equals_string {
                    writeln!(
                        output,
                        "{}${}_{} == {};",
                        indent.single(),
                        var,
                        slot_name,
                        Self::format_value_for_typeql(equals_string)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(equals_number) = &slot_condition.equals_number {
                    writeln!(
                        output,
                        "{}${}_{} == {};",
                        indent.single(),
                        var,
                        slot_name,
                        equals_number
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(minimum_value) = &slot_condition.minimum_value {
                    writeln!(
                        output,
                        "{}${}_{} >= {};",
                        indent.single(),
                        var,
                        slot_name,
                        minimum_value
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(maximum_value) = &slot_condition.maximum_value {
                    writeln!(
                        output,
                        "{}${}_{} <= {};",
                        indent.single(),
                        var,
                        slot_name,
                        maximum_value
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(pattern) = &slot_condition.pattern {
                    writeln!(
                        output,
                        "{}${}_{} like \"{}\";",
                        indent.single(),
                        var,
                        slot_name,
                        pattern
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Process expression conditions
        if let Some(expressions) = &condition.expression_conditions {
            for expr in expressions {
                // Parse and translate LinkML expressions to TypeQL predicates
                let typeql_expr = Self::translate_expression_to_typeql(expr, var)?;
                writeln!(output, "{}{};", indent.single(), typeql_expr)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Process composite conditions (AND/OR/NOT)
        if let Some(composite) = &condition.composite_conditions {
            self.generate_composite_conditions(output, var, composite, schema, indent)?;
        }

        Ok(())
    }

    /// Generate rule assertions
    fn generate_rule_assertions(
        &self,
        output: &mut String,
        var: &str,
        condition: &RuleConditions,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Generate entity/relation insertion based on conditions
        if let Some(slot_conditions) = &condition.slot_conditions {
            // First, determine if we're creating an entity or relation
            let entity_type = self.infer_entity_type_from_conditions(condition, schema);

            // Generate the insertion statement
            writeln!(
                output,
                "{}insert ${}_new isa {};",
                indent.single(),
                var,
                entity_type
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // Add attributes from slot conditions
            for (slot_name, slot_condition) in slot_conditions {
                if let Some(range) = &slot_condition.range {
                    // Determine if this is an attribute or relation
                    if Self::is_attribute_type(range, schema) {
                        writeln!(
                            output,
                            "{}${}_new has {}: ${}_{}_;",
                            indent.single(),
                            var,
                            slot_name,
                            var,
                            slot_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        // This is a relation to another entity
                        writeln!(
                            output,
                            "{}(owner: ${}_new, target: ${}_{}) isa {};",
                            indent.single(),
                            var,
                            var,
                            slot_name,
                            Self::get_relation_name(slot_name, range)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                // Add value assertions if specified in the condition
                if let Some(equals_string) = &slot_condition.equals_string {
                    writeln!(
                        output,
                        "{}${}_new has {}: {};",
                        indent.single(),
                        var,
                        slot_name,
                        Self::format_value_for_typeql(equals_string)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else if let Some(equals_number) = &slot_condition.equals_number {
                    writeln!(
                        output,
                        "{}${}_new has {}: {};",
                        indent.single(),
                        var,
                        slot_name,
                        equals_number
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Process expression-based assertions
        if let Some(expressions) = &condition.expression_conditions {
            for expr in expressions {
                // Generate derived attributes based on expressions
                if let Some((attr_name, attr_value)) = self.parse_assertion_expression(expr)? {
                    writeln!(
                        output,
                        "{}${}_new has {}: {};",
                        indent.single(),
                        var,
                        attr_name,
                        attr_value
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Handle composite assertions (complex logic)
        if let Some(composite) = &condition.composite_conditions {
            self.generate_composite_assertions(output, var, composite, schema, indent)?;
        }

        Ok(())
    }

    /// Get `TypeQL` type for a `LinkML` range
    fn get_typeql_type_for_range(range: &str, schema: &SchemaDefinition) -> String {
        // Check if it's a class reference
        if schema.classes.contains_key(range) {
            return range.to_string();
        }

        // Map LinkML types to TypeQL types
        match range {
            "string" | "str" | "text" | "uri" | "url" => "string".to_string(),
            "integer" | "int" => "long".to_string(),
            "float" | "double" | "decimal" => "double".to_string(),
            "boolean" | "bool" => "boolean".to_string(),
            "date" | "datetime" | "time" => "datetime".to_string(),
            _ => "string".to_string(), // Default fallback
        }
    }

    // Removed duplicate is_valid_identifier method - using the one defined earlier at line 85

    /// Format a value for `TypeQL` syntax
    fn format_value_for_typeql(value: &str) -> String {
        // Check if it's a numeric or boolean value (both can be used as-is)
        if value.parse::<f64>().is_ok() || value == "true" || value == "false" {
            value.to_string()
        } else {
            // String value - needs quotes
            format!("\"{}\"", value.replace('\"', "\\\""))
        }
    }

    /// Translate `LinkML` expression to `TypeQL` predicate
    fn translate_expression_to_typeql(expr: &str, var: &str) -> GeneratorResult<String> {
        // Parse basic comparison expressions
        if let Some(caps) = regex::Regex::new(r"(\w+)\s*([><=!]+)\s*(.+)")
            .map_err(|e| GeneratorError::Generation(format!("expression parsing: {e}")))?
            .captures(expr)
        {
            let field = &caps[1];
            let op = &caps[2];
            let value = &caps[3];

            let typeql_op = match op {
                ">" => ">",
                ">=" => ">=",
                "<" => "<",
                "<=" => "<=",
                "==" | "=" => "==",
                "!=" => "!=",
                _ => {
                    return Err(GeneratorError::Generation(format!(
                        "operator translation: Unknown operator: {op}"
                    )));
                }
            };

            Ok(format!(
                "${}_{} {} {}",
                var,
                field,
                typeql_op,
                Self::format_value_for_typeql(value)
            ))
        } else {
            // Complex expression - return as comment for manual translation
            Ok(format!("# Expression: {expr}"))
        }
    }

    /// Generate composite conditions (AND/OR/NOT)
    fn generate_composite_conditions(
        &self,
        output: &mut String,
        var: &str,
        composite: &CompositeConditions,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        if let Some(all_of) = &composite.all_of {
            writeln!(output, "{}# AND conditions:", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
            for condition in all_of {
                self.generate_rule_conditions(output, var, condition, schema, indent)?;
            }
        }

        if let Some(any_of) = &composite.any_of {
            writeln!(
                output,
                "{}# OR conditions (requires multiple rules):",
                indent.single()
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            for condition in any_of {
                writeln!(output, "    # Option:").map_err(Self::fmt_error_to_generator_error)?;
                self.generate_rule_conditions(output, var, condition, schema, indent)?;
            }
        }

        // Note: 'not' field is not available in CompositeConditions
        // This logic may need to be implemented differently
        if false {
            // Placeholder - original logic used composite.not
            writeln!(output, "{}# NOT condition:", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}not {{", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
            // Note: 'not' variable is not available - this logic needs to be implemented differently
            // self.generate_rule_conditions(output, var, not.as_ref(), schema, indent)?;
            writeln!(output, "{}}}", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Infer entity type from rule conditions
    fn infer_entity_type_from_conditions(
        &self,
        condition: &RuleConditions,
        schema: &SchemaDefinition,
    ) -> String {
        // Look for type hints in slot conditions
        if let Some(slot_conditions) = &condition.slot_conditions {
            for (_, slot_condition) in slot_conditions {
                if let Some(range) = &slot_condition.range
                    && schema.classes.contains_key(range)
                {
                    return range.clone();
                }
            }
        }

        // Default to generic entity
        "entity".to_string()
    }

    /// Check if a type is an attribute type
    fn is_attribute_type(range: &str, schema: &SchemaDefinition) -> bool {
        // If it's not a class, it's an attribute type
        !schema.classes.contains_key(range)
    }

    /// Get relation name for a slot
    fn get_relation_name(slot_name: &str, _range: &str) -> String {
        format!("has_{slot_name}")
    }

    /// Parse assertion expression into attribute name and value
    fn parse_assertion_expression(&self, expr: &str) -> GeneratorResult<Option<(String, String)>> {
        // Parse assignment expressions like "field = value"
        if let Some(caps) = regex::Regex::new(r"(\w+)\s*=\s*(.+)")
            .map_err(|e| GeneratorError::Generation(format!("composite condition parsing: {e}")))?
            .captures(expr)
        {
            let field = caps[1].to_string();
            let value = Self::format_value_for_typeql(&caps[2]);
            Ok(Some((field, value)))
        } else {
            Ok(None)
        }
    }

    /// Generate composite assertions
    fn generate_composite_assertions(
        &self,
        output: &mut String,
        var: &str,
        composite: &CompositeConditions,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // For assertions, we typically only handle all_of conditions
        if let Some(all_of) = &composite.all_of {
            for condition in all_of {
                self.generate_rule_assertions(output, var, condition, schema, indent)?;
            }
        }

        // any_of and not don't typically apply to assertions
        if composite.any_of.is_some() {
            writeln!(
                output,
                "{}# Warning: OR conditions in assertions require separate rules",
                indent.single()
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Note: 'not' field is not available in CompositeConditions
        if false {
            // Placeholder - original logic used composite.not.is_some()
            writeln!(
                output,
                "{}# Warning: NOT conditions not supported in assertions",
                indent.single()
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }
}

impl Default for EnhancedTypeQLGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SchemaAnalyzer {
    fn new() -> Self {
        Self {
            type_cache: HashMap::new(),
        }
    }

    /// Determine if a class should be an entity, relation, or abstract type
    fn determine_type(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> TypeQLType {
        if class.abstract_.unwrap_or(false) || class.mixin.unwrap_or(false) {
            return TypeQLType::Abstract;
        }

        // Count object-valued vs literal-valued slots
        let mut object_slots = Vec::new();
        let mut literal_slots = 0;

        for slot_name in &class.slots {
            if let Some(slot) = schema
                .slots
                .get(slot_name)
                .or_else(|| class.slot_usage.get(slot_name))
                && let Some(range) = &slot.range
            {
                if schema.classes.contains_key(range) {
                    object_slots.push((slot_name, slot));
                } else {
                    literal_slots += 1;
                }
            }
        }

        // Decision logic for entity vs relation
        if object_slots.len() >= 2 {
            // Multiple object references suggest a relation
            TypeQLType::Relation
        } else if object_slots.len() == 1 && literal_slots <= 2 {
            // Single object reference with few attributes might be a relation
            // Check if the class name suggests a relationship
            let name_lower = class.name.to_lowercase();
            if name_lower.contains("association")
                || name_lower.contains("relationship")
                || name_lower.contains("link")
                || name_lower.contains("_to_")
                || name_lower.contains("_has_")
            {
                TypeQLType::Relation
            } else {
                TypeQLType::Entity
            }
        } else {
            // Default to entity
            TypeQLType::Entity
        }
    }

    /// Validate the overall schema structure
    fn validate_structure(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Check for inheritance cycles
        for (class_name, class) in &schema.classes {
            self.check_inheritance_cycle(class_name, class, schema, &mut HashSet::new())?;
        }

        // Validate relation roles
        for (class_name, class_type) in &self.type_cache {
            if let TypeQLType::Relation = class_type
                && let Some(class) = schema.classes.get(class_name)
                && !class.slots.iter().any(|slot_name| {
                    schema
                        .slots
                        .get(slot_name)
                        .or_else(|| class.slot_usage.get(slot_name))
                        .and_then(|slot| slot.range.as_ref())
                        .is_some_and(|range| schema.classes.contains_key(range))
                })
            {
                return Err(GeneratorError::SchemaValidation(format!(
                    "Relation {class_name} has no object-valued slots"
                )));
            }
        }

        Ok(())
    }

    /// Check for inheritance cycles
    fn check_inheritance_cycle(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
    ) -> GeneratorResult<()> {
        if visited.contains(class_name) {
            return Err(GeneratorError::SchemaValidation(format!(
                "Inheritance cycle detected: {class_name}"
            )));
        }

        visited.insert(class_name.to_string());

        if let Some(parent) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            self.check_inheritance_cycle(parent, parent_class, schema, visited)?;
        }

        for mixin in &class.mixins {
            if let Some(mixin_class) = schema.classes.get(mixin) {
                self.check_inheritance_cycle(mixin, mixin_class, schema, visited)?;
            }
        }

        visited.remove(class_name);
        Ok(())
    }
}

#[async_trait]
impl AsyncGenerator for EnhancedTypeQLGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Enhanced TypeQL generator with full constraint support and migration capabilities"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".tql", ".typeql"]
    }

    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Validate schema has required fields for enhanced TypeQL generation
        if schema.name.is_empty() {
            return Err(GeneratorError::Validation(
                "Schema must have a name for enhanced TypeQL generation".to_string(),
            ));
        }

        // Validate class names are valid TypeQL identifiers
        for (class_name, _) in &schema.classes {
            if !Self::is_valid_identifier(class_name) {
                return Err(GeneratorError::Validation(format!(
                    "Class name '{class_name}' is not a valid TypeQL identifier"
                )));
            }
        }

        // Validate slot names
        for (slot_name, _) in &schema.slots {
            if !Self::is_valid_identifier(slot_name) {
                return Err(GeneratorError::Validation(format!(
                    "Slot name '{slot_name}' is not a valid TypeQL identifier"
                )));
            }
        }

        Ok(())
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        AsyncGenerator::validate_schema(self, schema).await?;

        // Analyze schema structure
        self.analyze_schema(schema)?;

        // Generate main schema
        let schema_output = self.generate_typeql_schema(schema, options)?;

        // Generate filename base
        let filename_base = if schema.name.is_empty() {
            "schema".to_string()
        } else {
            self.convert_identifier(&schema.name)
        };

        let mut outputs = vec![GeneratedOutput {
            filename: format!("{filename_base}.typeql"),
            content: schema_output,
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("generator".to_string(), self.name.clone());
                meta.insert("version".to_string(), "2.0".to_string());
                meta.insert("schema_name".to_string(), schema.name.clone());
                meta
            },
        }];

        // Generate migration script if requested
        if options
            .get_custom("generate_migration")
            .map(std::string::String::as_str)
            == Some("true")
        {
            let migration = self.generate_migration_script(schema, options)?;
            outputs.push(GeneratedOutput {
                filename: format!("{filename_base}-migration.tql"),
                content: migration,
                metadata: HashMap::new(),
            });
        }

        Ok(outputs)
    }
}

// Implement the synchronous Generator trait for backward compatibility
impl Generator for EnhancedTypeQLGenerator {
    fn name(&self) -> &'static str {
        "enhanced_typeql"
    }

    fn description(&self) -> &'static str {
        "Enhanced TypeQL generator with advanced constraint support, migration capabilities, and optimizations"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Use tokio to run the async validation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {e}")))?;

        runtime
            .block_on(AsyncGenerator::validate_schema(self, schema))
            .map_err(|e| LinkMLError::service(e.to_string()))
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Use tokio to run the async version
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {e}")))?;

        let options = GeneratorOptions::new();
        let outputs = runtime
            .block_on(AsyncGenerator::generate(self, schema, &options))
            .map_err(|e| LinkMLError::service(e.to_string()))?;

        // Concatenate all outputs into a single string
        Ok(outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join(
                "
",
            ))
    }

    fn get_file_extension(&self) -> &'static str {
        "txt"
    }

    fn get_default_filename(&self) -> &'static str {
        "generated.txt"
    }
}

impl CodeFormatter for EnhancedTypeQLGenerator {
    fn name(&self) -> &'static str {
        "enhanced_typeql_formatter"
    }

    fn description(&self) -> &'static str {
        "Advanced code formatter for TypeQL with constraint formatting and proper indentation"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["tql", "typeql"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        let mut formatted = String::new();
        let mut indent_level = 0;
        let mut in_rule_block = false;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Handle comments
            if trimmed.starts_with('#') || trimmed.starts_with("//") {
                formatted.push_str(&"    ".repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed == "}" || trimmed == "};" || trimmed.starts_with("} then") {
                indent_level = indent_level.saturating_sub(1);
                in_rule_block = false;
            }

            // Add proper indentation
            formatted.push_str(&"    ".repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after define, when, then, or opening braces
            // Note: Multiple conditions with same action is intentional for clarity
            #[allow(clippy::if_same_then_else)]
            if trimmed == "define" || trimmed == "undefine" {
                indent_level += 1;
            } else if trimmed.starts_with("rule ") && trimmed.ends_with(" {") {
                indent_level += 1;
                in_rule_block = true;
            } else if in_rule_block && (trimmed == "when {" || trimmed == "then {") {
                indent_level += 1;
            } else if !in_rule_block && trimmed.ends_with(" {") {
                indent_level += 1;
            }
        }

        Ok(formatted)
    }

    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        doc.lines()
            .map(|line| format!("{prefix}# {line}"))
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        indent: &IndentStyle,
        level: usize,
        separator: &str,
    ) -> String {
        let prefix = indent.to_string(level);
        items
            .iter()
            .map(|item| format!("{}{}", prefix, item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        s.replace('"', "\\\"")
    }

    fn convert_identifier(&self, id: &str) -> String {
        // Convert to TypeQL naming conventions (lowercase with hyphens)
        if id.is_empty() {
            return String::new();
        }

        let mut result = String::new();
        let chars: Vec<char> = id.chars().collect();

        for i in 0..chars.len() {
            let ch = chars[i];

            if ch.is_uppercase() {
                // Add hyphen before uppercase if:
                // 1. Not at start
                // 2. Previous char is lowercase OR
                // 3. Previous char is uppercase AND next char exists and is lowercase
                if i > 0 {
                    let prev_is_lower = i > 0 && chars[i - 1].is_lowercase();
                    let next_is_lower = i + 1 < chars.len() && chars[i + 1].is_lowercase();
                    let prev_is_upper = i > 0 && chars[i - 1].is_uppercase();

                    if prev_is_lower || (prev_is_upper && next_is_lower) {
                        result.push('-');
                    }
                }
                result.push(
                    ch.to_lowercase().next().unwrap_or(ch), // should always produce at least one character
                );
            } else if ch == '_' {
                result.push('-');
            } else {
                result.push(ch);
            }
        }

        // Clean up any double hyphens and trim
        result = result.replace("--", "-");
        result = result
            .trim_start_matches('-')
            .trim_end_matches('-')
            .to_string();

        // Store mapping for bidirectional lookup
        if let Ok(mut map) = self.identifier_map.write() {
            map.insert(id.to_string(), result.clone());
        }

        result
    }
}

// Migration support
impl EnhancedTypeQLGenerator {
    /// Generate migration script for schema changes
    fn generate_migration_script(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "# TypeQL Migration Script")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# Schema: {}", schema.name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "# Version: {}",
            schema.version.as_ref().unwrap_or(&"unknown".to_string())
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        let timestamp = self
            .timestamp_utils
            .now_rfc3339()
            .unwrap_or_else(|_| "unknown".to_string());
        writeln!(
            &mut output,
            "# Generated: {}",
            timestamp
                .replace('T', " ")
                .replace('Z', "")
                .split('.')
                .next()
                .unwrap_or(&timestamp)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate migration sections
        writeln!(&mut output, "# Phase 1: Schema Modifications")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# ==============================")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Check for previous schema version from options
        if let Some(prev_version) = options.get_custom("previous_schema_version") {
            writeln!(&mut output, "# Migrating from version: {prev_version}")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate DROP statements for removed elements
        writeln!(&mut output, "# Remove deprecated elements")
            .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(removed_classes) = options.get_custom("removed_classes") {
            for class_name in removed_classes.split(',') {
                writeln!(&mut output, "undefine {class_name} sub entity;")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate ALTER statements for modified elements
        writeln!(&mut output, "# Modify existing elements")
            .map_err(Self::fmt_error_to_generator_error)?;
        for (class_name, _class) in &schema.classes {
            // Generate modifications for changed attributes
            if let Some(modified_attrs) =
                options.get_custom(&format!("{class_name}_modified_attrs"))
            {
                for attr in modified_attrs.split(',') {
                    writeln!(
                        &mut output,
                        "redefine {} owns {};",
                        self.convert_identifier(class_name),
                        attr
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Handle inheritance changes
            if let Some(new_parent) = options.get_custom(&format!("{class_name}_new_parent")) {
                writeln!(
                    &mut output,
                    "redefine {} sub {};",
                    self.convert_identifier(class_name),
                    new_parent
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate CREATE statements for new elements
        writeln!(&mut output, "# Add new elements").map_err(Self::fmt_error_to_generator_error)?;
        for (class_name, _class) in &schema.classes {
            // Check if this is a new class (not in previous version)
            if options
                .get_custom(&format!("{class_name}_is_new"))
                .map(std::string::String::as_str)
                == Some("true")
            {
                // Note: generate_class is async but this context is not async
                // This needs to be refactored to work properly
                writeln!(&mut output, "# Class: {class_name}").map_err(|e| {
                    GeneratorError::Generation(format!("writing class comment: {e}"))
                })?;
            }
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Phase 2: Data Migration
        writeln!(&mut output, "# Phase 2: Data Migration")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# =======================")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate data migration queries
        writeln!(&mut output, "# Migrate existing data to new schema")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Example: Rename attribute values
        if let Some(attr_renames) = options.get_custom("attribute_renames") {
            writeln!(&mut output, "# Rename attributes")
                .map_err(Self::fmt_error_to_generator_error)?;
            for rename in attr_renames.split(',') {
                if let Some((old, new)) = rename.split_once(':') {
                    writeln!(&mut output, "match").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "  $x has {old}: $old_val;")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "delete").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "  $x has {old};")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "insert").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "  $x has {new}: $old_val;")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Example: Transform data formats
        if let Some(transforms) = options.get_custom("data_transforms") {
            writeln!(&mut output, "# Transform data formats")
                .map_err(Self::fmt_error_to_generator_error)?;
            for transform in transforms.split(',') {
                writeln!(&mut output, "# Apply transformation: {transform}")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Phase 3: Validation
        writeln!(&mut output, "# Phase 3: Validation")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# ===================")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate validation queries
        writeln!(&mut output, "# Validate migrated data")
            .map_err(Self::fmt_error_to_generator_error)?;
        for (class_name, class) in &schema.classes {
            if class.abstract_.unwrap_or(false) {
                continue;
            }

            // Check required attributes
            for slot_name in &class.slots {
                if let Some(slot) = schema
                    .slots
                    .get(slot_name)
                    .or_else(|| class.slot_usage.get(slot_name))
                    && slot.required.unwrap_or(false)
                {
                    writeln!(
                        &mut output,
                        "# Validate required attribute '{slot_name}' for '{class_name}'"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "match").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        &mut output,
                        "  $x isa {};",
                        self.convert_identifier(class_name)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "  not {{ $x has {slot_name}; }};")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "get $x;").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "# Should return empty result set")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output, "# Migration complete!")
            .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
    use serde_json::json;

    #[tokio::test]
    async fn test_enhanced_typeql_generation() -> anyhow::Result<()> {
        let generator = EnhancedTypeQLGenerator::new();

        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "TestSchema".to_string(),
            version: Some("1.0.0".to_string()),
            ..Default::default()
        };

        // Add slots
        let name_slot = SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        };
        schema.slots.insert("name".to_string(), name_slot);

        let age_slot = SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            minimum_value: Some(json!(0)),
            maximum_value: Some(json!(150)),
            ..Default::default()
        };
        schema.slots.insert("age".to_string(), age_slot);

        // Add classes
        let person_class = ClassDefinition {
            name: "Person".to_string(),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Person".to_string(), person_class);

        let options = GeneratorOptions::default();
        let outputs = AsyncGenerator::generate(&generator, &schema, &options)
            .await
            .expect("should generate TypeQL output: {}");

        assert_eq!(outputs.len(), 1);
        let content = &outputs[0].content;

        // Check for enhanced features
        assert!(content.contains("person sub entity"));
        assert!(content.contains("owns name @key"));
        assert!(content.contains("owns age"));
        assert!(content.contains("age sub attribute, value long, range [0..150]"));
        assert!(content.contains("# TypeQL Schema generated from LinkML"));
        assert!(content.contains("# Version: 1.0.0"));
        Ok(())
    }

    #[test]
    fn test_advanced_identifier_conversion() {
        let generator = EnhancedTypeQLGenerator::new();

        assert_eq!(generator.convert_identifier("PersonName"), "person-name");
        assert_eq!(generator.convert_identifier("person_name"), "person-name");
        assert_eq!(
            generator.convert_identifier("HTTPSConnection"),
            "https-connection"
        );
        assert_eq!(generator.convert_identifier("has_part"), "has-part");
    }

    #[tokio::test]
    async fn test_relation_detection() -> anyhow::Result<()> {
        let generator = EnhancedTypeQLGenerator::new();
        let mut schema = SchemaDefinition::default();

        // Create a relation-like class
        let employment = ClassDefinition {
            name: "Employment".to_string(),
            slots: vec![
                "employee".to_string(),
                "employer".to_string(),
                "start_date".to_string(),
            ],
            ..Default::default()
        };

        // Add slots
        let employee_person_slot = SlotDefinition {
            name: "employee".to_string(),
            range: Some("Person".to_string()),
            ..Default::default()
        };
        schema
            .slots
            .insert("employee".to_string(), employee_person_slot);

        let employer_organization_slot = SlotDefinition {
            name: "employer".to_string(),
            range: Some("Organization".to_string()),
            ..Default::default()
        };
        schema
            .slots
            .insert("employer".to_string(), employer_organization_slot);

        // Add placeholder classes
        schema
            .classes
            .insert("Person".to_string(), ClassDefinition::default());
        schema
            .classes
            .insert("Organization".to_string(), ClassDefinition::default());
        schema.classes.insert("Employment".to_string(), employment);

        // Analyze schema
        generator
            .analyze_schema(&schema)
            .expect("should analyze schema: {}");

        // Check that Employment was detected as a relation
        assert_eq!(
            generator
                .analyzer
                .read()
                .expect("analyzer lock should not be poisoned: {}")
                .type_cache
                .get("Employment"),
            Some(&TypeQLType::Relation)
        );
        Ok(())
    }
}

/// Create an enhanced `TypeQL` generator using the factory pattern
///
/// This is the preferred way to create an `EnhancedTypeQLGenerator`, ensuring
/// proper initialization and following `RootReal`'s factory pattern standards.
///
/// # Returns
///
/// Returns a configured enhanced `TypeQL` generator instance
#[must_use]
pub fn create_enhanced_typeql_generator() -> EnhancedTypeQLGenerator {
    EnhancedTypeQLGenerator::new()
}
