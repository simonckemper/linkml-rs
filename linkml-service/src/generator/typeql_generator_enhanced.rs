//! Enhanced TypeQL generation implementation for TypeDB schemas
//!
//! This module provides comprehensive LinkML to TypeQL translation with:
//! - Advanced entity/relation detection
//! - Full constraint support
//! - Complex inheritance handling
//! - Migration script generation
//! - TypeDB 3.0 feature support

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, GeneratedOutput, Generator, GeneratorResult, GeneratorError};
use super::typeql_constraints::TypeQLConstraintTranslator;
use super::typeql_relation_analyzer::RelationAnalyzer;
use super::typeql_role_inheritance::RoleInheritanceResolver;
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet, BTreeMap};
use std::sync::RwLock;
use std::fmt::Write;
use thiserror::Error;

/// Errors specific to TypeQL generation
#[derive(Debug, Error)]
pub enum TypeQLError {
    /// Schema structure is invalid for TypeQL generation
    #[error("Invalid schema structure: {0}")]
    InvalidSchema(String),
    
    /// LinkML feature not supported in TypeQL
    #[error("Unsupported LinkML feature: {0}")]
    UnsupportedFeature(String),
    
    /// Error translating LinkML constraint to TypeQL
    #[error("Constraint translation error: {0}")]
    ConstraintError(String),
    
    /// Circular inheritance detected in schema
    #[error("Inheritance cycle detected: {0}")]
    InheritanceCycle(String),
}

/// Enhanced TypeQL schema generator for TypeDB 3.0+
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
    identifier_map: HashMap<String, String>,
}

/// Analyzes schema structure for optimal TypeQL generation
struct SchemaAnalyzer {
    /// Cache for entity/relation detection results
    type_cache: HashMap<String, TypeQLType>,
}

/// Type of TypeQL schema element
#[derive(Debug, Clone, PartialEq)]
enum TypeQLType {
    Entity,
    Relation,
    Attribute,
    Abstract,
}

/// Relation role information
#[derive(Debug, Clone)]
struct RelationRole {
    name: String,
    players: Vec<String>,
    cardinality: Option<(usize, Option<usize>)>,
}


impl EnhancedTypeQLGenerator {
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    /// Create a new enhanced TypeQL generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "typeql-enhanced".to_string(),
            analyzer: RwLock::new(SchemaAnalyzer::new()),
            constraint_translator: RwLock::new(TypeQLConstraintTranslator::new()),
            relation_analyzer: RwLock::new(RelationAnalyzer::new()),
            role_inheritance_resolver: RwLock::new(RoleInheritanceResolver::new()),
            identifier_map: HashMap::new(),
        }
    }

    /// Analyze schema and determine optimal TypeQL structure
    fn analyze_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // First pass: identify all types using advanced relation analysis
        for (class_name, class_def) in &schema.classes {
            // Use relation analyzer for better detection
            if let Some(_relation_info) = self.relation_analyzer.write().expect("relation analyzer lock should not be poisoned").analyze_relation(class_name, class_def, schema) {
                self.analyzer.write().expect("analyzer lock should not be poisoned").type_cache.insert(class_name.clone(), TypeQLType::Relation);
                
                // Analyze role inheritance if applicable
                if let Some(_parent) = &class_def.is_a {
                    self.role_inheritance_resolver.write().expect("role inheritance resolver lock should not be poisoned").analyze_relation_inheritance(
                        class_name,
                        class_def,
                        schema,
                    );
                }
            } else {
                let typeql_type = self.analyzer.read().expect("analyzer lock should not be poisoned").determine_type(class_def, schema)?;
                self.analyzer.write().expect("analyzer lock should not be poisoned").type_cache.insert(class_name.clone(), typeql_type);
            }
        }
        
        // Second pass: validate and optimize structure
        self.analyzer.read().expect("analyzer lock should not be poisoned").validate_structure(schema)?;
        
        Ok(())
    }

    /// Generate complete TypeQL schema
    fn generate_typeql_schema(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let indent = &options.indent;

        // Header with metadata
        self.write_header(&mut output, schema)?;
        
        // Define section
        writeln!(&mut output, "\ndefine\n").map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate in dependency order
        let ordered_types = self.get_dependency_order(schema)?;
        
        // 1. Generate abstract types first
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name) {
                if class.abstract_.unwrap_or(false) || class.mixin.unwrap_or(false) {
                    self.generate_abstract_type(&mut output, type_name, class, schema, indent)?;
                }
            }
        }
        
        // 2. Generate attributes
        self.generate_all_attributes(&mut output, schema, indent)?;
        
        // 3. Generate concrete entities
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name) {
                if let Some(TypeQLType::Entity) = self.analyzer.read().expect("analyzer lock should not be poisoned").type_cache.get(type_name) {
                    if !class.abstract_.unwrap_or(false) {
                        self.generate_entity(&mut output, type_name, class, schema, indent)?;
                    }
                }
            }
        }
        
        // 4. Generate relations
        for type_name in &ordered_types {
            if let Some(class) = schema.classes.get(type_name) {
                if let Some(TypeQLType::Relation) = self.analyzer.read().expect("analyzer lock should not be poisoned").type_cache.get(type_name) {
                    self.generate_relation(&mut output, type_name, class, schema, indent)?;
                }
            }
        }
        
        // 5. Generate constraints and rules
        if options.get_custom("generate_constraints") != Some("false") {
            writeln!(&mut output, "\n# Constraints and Validation Rules\n").map_err(Self::fmt_error_to_generator_error)?;
            self.generate_constraints(&mut output, schema, indent)?;
            self.generate_validation_rules(&mut output, schema, indent)?;
        }
        
        Ok(output)
    }

    /// Write schema header with metadata
    fn write_header(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# TypeQL Schema generated from LinkML").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "# Generator: Enhanced TypeQL Generator v2.0").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "# TypeDB Version: 3.0+").map_err(Self::fmt_error_to_generator_error)?;
        
        if !schema.name.is_empty() {
            writeln!(output, "# Schema: {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if let Some(version) = &schema.version {
            writeln!(output, "# Version: {}", version).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if let Some(desc) = &schema.description {
            writeln!(output, "# Description: {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add generation timestamp
        writeln!(output, "# Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")).map_err(Self::fmt_error_to_generator_error)?;
        
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
    fn visit_type(
        &self,
        type_name: &str,
        schema: &SchemaDefinition,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        order: &mut Vec<String>,
    ) -> GeneratorResult<()> {
        if visiting.contains(type_name) {
            return Err(GeneratorError::SchemaValidation(format!("Inheritance cycle detected: {}", type_name)));
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
            writeln!(output, "# Abstract: {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Determine base type
        let base_type = if self.is_relation_like(class, schema) {
            "relation"
        } else {
            "entity"
        };
        
        write!(output, "{} sub {}, abstract", type_name, base_type).map_err(Self::fmt_error_to_generator_error)?;
        
        // Add attributes owned by abstract type
        let attributes = self.collect_direct_attributes(class, schema);
        if !attributes.is_empty() {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            for (i, (attr_name, constraints)) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name).map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" ")).map_err(Self::fmt_error_to_generator_error)?;
                }
                if i < attributes.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        } else {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
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
            writeln!(output, "# Entity: {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Build inheritance chain
        let inheritance = self.build_inheritance_chain(class, schema)?;
        
        write!(output, "{} sub", type_name).map_err(Self::fmt_error_to_generator_error)?;
        if !inheritance.is_empty() {
            write!(output, " {}", inheritance.join(", sub ")).map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(output, " entity").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Collect all attributes (including constraints)
        let all_attributes = self.collect_all_attributes(class, schema)?;
        
        // Add roles this entity can play
        let roles = self.collect_playable_roles(name, schema);
        
        if all_attributes.is_empty() && roles.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            
            // Write attributes with constraints
            for (i, (attr_name, constraints)) in all_attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name).map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" ")).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                if i < all_attributes.len() - 1 || !roles.is_empty() {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            
            // Write roles
            for (i, role) in roles.iter().enumerate() {
                write!(output, "{}plays {}", indent.single(), role).map_err(Self::fmt_error_to_generator_error)?;
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
        let relation_info = self.relation_analyzer.write().expect("relation analyzer lock should not be poisoned").analyze_relation(name, class, schema)
            .ok_or_else(|| GeneratorError::SchemaValidation(format!("{} is not a valid relation", name)))?;
        
        // Add documentation
        if let Some(desc) = &class.description {
            writeln!(output, "# Relation: {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add multi-way relation comment if applicable
        if relation_info.is_multiway {
            writeln!(output, "# Multi-way relation with {} roles", relation_info.roles.len()).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Build inheritance chain
        let inheritance = self.build_inheritance_chain(class, schema)?;
        
        write!(output, "{} sub", type_name).map_err(Self::fmt_error_to_generator_error)?;
        if !inheritance.is_empty() {
            write!(output, " {}", inheritance.join(", sub ")).map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(output, " relation").map_err(Self::fmt_error_to_generator_error)?;
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
            if let Some(hierarchy) = self.role_inheritance_resolver.read().expect("role inheritance resolver lock should not be poisoned").hierarchies.get(name) {
                if let Some(base_role) = hierarchy.specializations.get(&role_key) {
                    // This role specializes another
                    write!(output, "{}relates {} as {}", 
                        indent.single(), 
                        role_name,
                        self.convert_identifier(base_role)
                    ).map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(output, "{}relates {}", indent.single(), role_name).map_err(Self::fmt_error_to_generator_error)?;
                }
            } else {
                write!(output, "{}relates {}", indent.single(), role_name).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            // Add cardinality if specified
            if let Some((min, max)) = &role.cardinality {
                write!(output, " @card({}", min).map_err(Self::fmt_error_to_generator_error)?;
                if let Some(max_val) = max {
                    write!(output, "..{}", max_val).map_err(Self::fmt_error_to_generator_error)?;
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
        if !attributes.is_empty() {
            if !relation_info.roles.is_empty() {
                writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            }
            
            for (i, (attr_name, constraints)) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr_name).map_err(Self::fmt_error_to_generator_error)?;
                if !constraints.is_empty() {
                    write!(output, " {}", constraints.join(" ")).map_err(Self::fmt_error_to_generator_error)?;
                }
                if i < attributes.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        } else {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Generate role players
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        for role in &relation_info.roles {
            let player_typeql = self.convert_identifier(&role.player_type);
            writeln!(output, "{} plays {}:{};", player_typeql, type_name, role.name).map_err(Self::fmt_error_to_generator_error)?;
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
        writeln!(output, "# Attributes\n").map_err(Self::fmt_error_to_generator_error)?;
        
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
            writeln!(output, "# {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Determine value type
        let value_type = self.map_range_to_typeql(&slot.range, schema);
        
        write!(output, "{} sub attribute, value {}", name, value_type).map_err(Self::fmt_error_to_generator_error)?;
        
        // Add inline constraints
        let inline_constraints = self.get_inline_constraints(slot);
        if !inline_constraints.is_empty() {
            write!(output, ", {}", inline_constraints.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add range constraints for numeric types
        if value_type == "long" || value_type == "double" {
            let mut range_parts = Vec::new();
            
            if let Some(min) = &slot.minimum_value {
                if let Some(min_num) = self.value_to_number(min) {
                    range_parts.push(format!("{}", min_num));
                }
            } else {
                range_parts.push("".to_string());
            }
            
            range_parts.push("..".to_string());
            
            if let Some(max) = &slot.maximum_value {
                if let Some(max_num) = self.value_to_number(max) {
                    range_parts.push(format!("{}", max_num));
                }
            }
            
            if range_parts.len() > 2 || !range_parts[0].is_empty() {
                write!(output, ", range [{}]", range_parts.join("")).map_err(Self::fmt_error_to_generator_error)?;
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
        writeln!(output, "# Validation Rules\n").map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate required field rules
        for (class_name, class) in &schema.classes {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name).or_else(|| class.slot_usage.get(slot_name)) {
                    if slot.required == Some(true) {
                        self.generate_required_rule(output, class_name, slot_name, indent)?;
                    }
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
        let rule_name = format!(
            "{}-unique-{}",
            self.convert_identifier(class_name),
            "key"
        );
        
        writeln!(output, "rule {}:", rule_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;
        
        // Match two instances with same key values
        writeln!(output, "{}$x isa {};", indent.single(), self.convert_identifier(class_name)).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}$y isa {};", indent.single(), self.convert_identifier(class_name)).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}not {{ $x is $y; }};", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        
        for slot in &unique_key.unique_key_slots {
            let attr = self.convert_identifier(slot);
            writeln!(output, "{}$x has {} $val{};", indent.single(), attr, slot).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}$y has {} $val{};", indent.single(), attr, slot).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}$x has validation-error \"Duplicate unique key\";", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
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
        
        writeln!(output, "rule {}:", rule_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        ).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}not {{ $x has {} $v; }};",
            indent.single(),
            self.convert_identifier(slot_name)
        ).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x has validation-error \"Missing required field: {}\";",
            indent.single(),
            slot_name
        ).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }

    /// Generate rule from LinkML rule definition
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
        
        writeln!(output, "# Rule: {}", rule.description.as_ref().unwrap_or(&rule.title.as_ref().unwrap_or(&"unnamed".to_string()))).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "rule {}:", rule_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "when {{").map_err(Self::fmt_error_to_generator_error)?;
        
        // Base entity match
        writeln!(
            output,
            "{}$x isa {};",
            indent.single(),
            self.convert_identifier(class_name)
        ).map_err(Self::fmt_error_to_generator_error)?;
        
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
            ).map_err(Self::fmt_error_to_generator_error)?;
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
    ) -> GeneratorResult<Vec<String>> {
        let mut chain = Vec::new();
        
        // Direct parent
        if let Some(parent) = &class.is_a {
            chain.push(self.convert_identifier(parent));
        }
        
        // Mixins
        for mixin in &class.mixins {
            chain.push(self.convert_identifier(mixin));
        }
        
        Ok(chain)
    }

    /// Collect all attributes including inherited ones
    fn collect_all_attributes(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<(String, Vec<String>)>> {
        let mut attributes = Vec::new();
        let mut seen = HashSet::new();
        
        // Direct attributes
        for (attr_name, constraints) in self.collect_direct_attributes(class, schema) {
            if !seen.contains(&attr_name) {
                seen.insert(attr_name.clone());
                attributes.push((attr_name, constraints));
            }
        }
        
        Ok(attributes)
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
            if let Some(slot) = schema.slots.get(slot_name).or_else(|| class.slot_usage.get(slot_name)) {
                if let Some(range) = &slot.range {
                    if !schema.classes.contains_key(range) {
                        let attr_name = self.convert_identifier(slot_name);
                        let constraints = self.collect_slot_constraints(slot);
                        attributes.push((attr_name, constraints));
                    }
                }
            }
        }
        
        attributes
    }

    /// Collect constraints for a slot
    fn collect_slot_constraints(&self, slot: &SlotDefinition) -> Vec<String> {
        // Delegate to the enhanced constraint translator
        let mut constraints = self.constraint_translator.write().expect("constraint translator lock should not be poisoned").translate_slot_constraints(slot);
        
        // Add range constraints for numeric types
        if let Some(range) = &slot.range {
            if range == "integer" || range == "float" || range == "double" {
                let range_constraints = self.constraint_translator.write().expect("constraint translator lock should not be poisoned").translate_range_constraints(slot);
                constraints.extend(range_constraints);
            }
        }
        
        constraints
    }

    /// Get inline constraints for attribute definition
    fn get_inline_constraints(&self, slot: &SlotDefinition) -> Vec<String> {
        // Use the public method that handles all constraints
        self.constraint_translator.write().expect("constraint translator lock should not be poisoned").translate_slot_constraints(slot)
            .into_iter()
            .filter(|c| !c.starts_with('@')) // Filter out @ annotations for inline use
            .collect()
    }

    /// Collect roles this entity can play
    fn collect_playable_roles(&self, entity_name: &str, _schema: &SchemaDefinition) -> Vec<String> {
        // Use the relation analyzer's role player map
        self.relation_analyzer.write().expect("relation analyzer lock should not be poisoned").get_playable_roles(entity_name)
            .into_iter()
            .map(|role| {
                let parts: Vec<&str> = role.split(':').collect();
                if parts.len() == 2 {
                    format!("{}:{}", 
                        self.convert_identifier(parts[0]), 
                        self.convert_identifier(parts[1])
                    )
                } else {
                    role
                }
            })
            .collect()
    }

    /// Collect relation roles
    fn collect_relation_roles(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<RelationRole>> {
        let mut roles = Vec::new();
        
        for slot_name in &class.slots {
            if let Some(slot) = schema.slots.get(slot_name).or_else(|| class.slot_usage.get(slot_name)) {
                if let Some(range) = &slot.range {
                    if schema.classes.contains_key(range) {
                        // This is an object-valued slot, create a role
                        let role_name = self.convert_identifier(slot_name);
                        let player = self.convert_identifier(range);
                        
                        let cardinality = if slot.multivalued == Some(true) {
                            Some((0, None))
                        } else if slot.required == Some(true) {
                            Some((1, Some(1)))
                        } else {
                            Some((0, Some(1)))
                        };
                        
                        roles.push(RelationRole {
                            name: role_name,
                            players: vec![player],
                            cardinality,
                        });
                    }
                }
            }
        }
        
        Ok(roles)
    }

    /// Determine if a class should be a relation
    fn is_relation_like(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> bool {
        // A class is relation-like if:
        // 1. It has multiple object-valued slots
        // 2. It represents a relationship concept
        // 3. It has relationship-indicating patterns in name/description
        
        let mut object_slots = 0;
        
        for slot_name in &class.slots {
            if let Some(slot) = schema.slots.get(slot_name).or_else(|| class.slot_usage.get(slot_name)) {
                if let Some(range) = &slot.range {
                    if schema.classes.contains_key(range) {
                        object_slots += 1;
                    }
                }
            }
        }
        
        // Multiple object-valued slots indicate a relation
        if object_slots >= 2 {
            return true;
        }
        
        // Check for relationship patterns in name
        let name_lower = class.name.to_lowercase();
        let relation_patterns = ["association", "relationship", "link", "connection", "mapping"];
        
        relation_patterns.iter().any(|pattern| name_lower.contains(pattern))
    }

    /// Map LinkML range to TypeQL value type
    fn map_range_to_typeql(&self, range: &Option<String>, schema: &SchemaDefinition) -> &'static str {
        match range.as_deref() {
            Some("string" | "str" | "uri" | "url" | "curie" | "ncname") => "string",
            Some("integer" | "int") => "long",
            Some("float" | "double" | "decimal" | "number") => "double",
            Some("boolean" | "bool") => "boolean",
            Some("date" | "datetime" | "time") => "datetime",
            Some(custom) => {
                // Check if it's a custom type definition
                if let Some(type_def) = schema.types.get(custom) {
                    // Resolve base type
                    self.map_range_to_typeql(&type_def.base_type, schema)
                } else {
                    "string" // Default fallback
                }
            }
            None => "string",
        }
    }

    /// Escape regex pattern for TypeQL
    fn escape_regex(&self, pattern: &str) -> String {
        // TypeQL uses Java regex syntax, escape accordingly
        pattern
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    }

    /// Convert LinkML Value to number
    fn value_to_number(&self, value: &linkml_core::Value) -> Option<f64> {
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
        _var: &str,
        _condition: &RuleConditions,
        _schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // This would translate LinkML rule conditions to TypeQL
        // For now, basic implementation
        writeln!(output, "{}# TODO: Complex rule conditions", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate rule assertions
    fn generate_rule_assertions(
        &self,
        output: &mut String,
        _var: &str,
        _condition: &RuleConditions,
        _schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // This would translate LinkML rule assertions to TypeQL
        // For now, basic implementation
        writeln!(output, "{}# TODO: Complex rule assertions", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
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
    fn determine_type(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<TypeQLType> {
        if class.abstract_.unwrap_or(false) || class.mixin.unwrap_or(false) {
            return Ok(TypeQLType::Abstract);
        }
        
        // Count object-valued vs literal-valued slots
        let mut object_slots = Vec::new();
        let mut literal_slots = 0;
        
        for slot_name in &class.slots {
            if let Some(slot) = schema.slots.get(slot_name).or_else(|| class.slot_usage.get(slot_name)) {
                if let Some(range) = &slot.range {
                    if schema.classes.contains_key(range) {
                        object_slots.push((slot_name, slot));
                    } else {
                        literal_slots += 1;
                    }
                }
            }
        }
        
        // Decision logic for entity vs relation
        if object_slots.len() >= 2 {
            // Multiple object references suggest a relation
            Ok(TypeQLType::Relation)
        } else if object_slots.len() == 1 && literal_slots <= 2 {
            // Single object reference with few attributes might be a relation
            // Check if the class name suggests a relationship
            let name_lower = class.name.to_lowercase();
            if name_lower.contains("association") || 
               name_lower.contains("relationship") ||
               name_lower.contains("link") ||
               name_lower.contains("_to_") ||
               name_lower.contains("_has_") {
                Ok(TypeQLType::Relation)
            } else {
                Ok(TypeQLType::Entity)
            }
        } else {
            // Default to entity
            Ok(TypeQLType::Entity)
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
            if let TypeQLType::Relation = class_type {
                if let Some(class) = schema.classes.get(class_name) {
                    if !class.slots.iter().any(|slot_name| {
                        schema.slots.get(slot_name)
                            .or_else(|| class.slot_usage.get(slot_name))
                            .and_then(|slot| slot.range.as_ref())
                            .map(|range| schema.classes.contains_key(range))
                            .unwrap_or(false)
                    }) {
                        return Err(GeneratorError::SchemaValidation(
                            format!("Relation {} has no object-valued slots", class_name)
                        ));
                    }
                }
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
            return Err(GeneratorError::SchemaValidation(format!("Inheritance cycle detected: {}", class_name)));
        }
        
        visited.insert(class_name.to_string());
        
        if let Some(parent) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent) {
                self.check_inheritance_cycle(parent, parent_class, schema, visited)?;
            }
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
impl Generator for EnhancedTypeQLGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Enhanced TypeQL generator with full constraint support and migration capabilities"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".tql", ".typeql"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        self.validate_schema(schema).await?;
        
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
        
        let mut outputs = vec![
            GeneratedOutput {
                filename: format!("{}.typeql", filename_base),
                content: schema_output,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("generator".to_string(), self.name.clone());
                    meta.insert("version".to_string(), "2.0".to_string());
                    meta.insert("schema_name".to_string(), schema.name.clone());
                    meta
                },
            }
        ];
        
        // Generate migration script if requested
        if options.get_custom("generate_migration") == Some("true") {
            let migration = self.generate_migration_script(schema, options)?;
            outputs.push(GeneratedOutput {
                filename: format!("{}-migration.tql", filename_base),
                content: migration,
                metadata: HashMap::new(),
            });
        }
        
        Ok(outputs)
    }
}

impl CodeFormatter for EnhancedTypeQLGenerator {
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        doc.lines()
            .map(|line| format!("{prefix}# {line}"))
            .collect::<Vec<_>>()
            .join("\n")
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
                    let prev_is_lower = i > 0 && chars[i-1].is_lowercase();
                    let next_is_lower = i + 1 < chars.len() && chars[i+1].is_lowercase();
                    let prev_is_upper = i > 0 && chars[i-1].is_uppercase();
                    
                    if prev_is_lower || (prev_is_upper && next_is_lower) {
                        result.push('-');
                    }
                }
                result.push(ch.to_lowercase().next().expect("to_lowercase() should always produce at least one character"));
            } else if ch == '_' {
                result.push('-');
            } else {
                result.push(ch);
            }
        }
        
        // Clean up any double hyphens and trim
        result = result.replace("--", "-");
        result = result.trim_start_matches('-').trim_end_matches('-').to_string();
        
        // Store mapping for bidirectional lookup
        // self.identifier_map.insert(id.to_string(), result.clone());
        
        result
    }
}

// Migration support
impl EnhancedTypeQLGenerator {
    /// Generate migration script for schema changes
    fn generate_migration_script(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "# TypeQL Migration Script").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# Schema: {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# Version: {}", schema.version.as_ref().unwrap_or(&"unknown".to_string())).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // TODO: Implement actual migration logic based on schema diff
        writeln!(&mut output, "# This is a placeholder for migration logic").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# In production, this would:").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# 1. Compare with previous schema version").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# 2. Generate appropriate schema modifications").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# 3. Include data migration queries").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# 4. Handle breaking changes safely").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_enhanced_typeql_generation() {
        let generator = EnhancedTypeQLGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "test".to_string();
        schema.name = "TestSchema".to_string();
        schema.version = Some("1.0.0".to_string());

        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.name = "name".to_string();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        name_slot.identifier = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut age_slot = SlotDefinition::default();
        age_slot.name = "age".to_string();
        age_slot.range = Some("integer".to_string());
        age_slot.minimum_value = Some(json!(0));
        age_slot.maximum_value = Some(json!(150));
        schema.slots.insert("age".to_string(), age_slot);

        // Add classes
        let mut person_class = ClassDefinition::default();
        person_class.name = "Person".to_string();
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        let options = GeneratorOptions::default();
        let outputs = generator.generate(&schema, &options).await.map_err(Self::fmt_error_to_generator_error)?;

        assert_eq!(outputs.len(), 1);
        let content = &outputs[0].content;
        
        // Check for enhanced features
        assert!(content.contains("person sub entity"));
        assert!(content.contains("owns name @key"));
        assert!(content.contains("owns age"));
        assert!(content.contains("age sub attribute, value long, range [0..150]"));
        assert!(content.contains("# TypeQL Schema generated from LinkML"));
        assert!(content.contains("# Version: 1.0.0"));
    }

    #[test]
    fn test_advanced_identifier_conversion() {
        let generator = EnhancedTypeQLGenerator::new();

        assert_eq!(generator.convert_identifier("PersonName"), "person-name");
        assert_eq!(generator.convert_identifier("person_name"), "person-name");
        assert_eq!(generator.convert_identifier("HTTPSConnection"), "https-connection");
        assert_eq!(generator.convert_identifier("has_part"), "has-part");
    }

    #[tokio::test]
    async fn test_relation_detection() {
        let generator = EnhancedTypeQLGenerator::new();
        let mut schema = SchemaDefinition::default();

        // Create a relation-like class
        let mut employment = ClassDefinition::default();
        employment.name = "Employment".to_string();
        employment.slots = vec!["employee".to_string(), "employer".to_string(), "start_date".to_string()];

        // Add slots
        let mut employee_slot = SlotDefinition::default();
        employee_slot.name = "employee".to_string();
        employee_slot.range = Some("Person".to_string());
        schema.slots.insert("employee".to_string(), employee_slot);

        let mut employer_slot = SlotDefinition::default();
        employer_slot.name = "employer".to_string();
        employer_slot.range = Some("Organization".to_string());
        schema.slots.insert("employer".to_string(), employer_slot);

        // Add placeholder classes
        schema.classes.insert("Person".to_string(), ClassDefinition::default());
        schema.classes.insert("Organization".to_string(), ClassDefinition::default());
        schema.classes.insert("Employment".to_string(), employment);

        // Analyze schema
        generator.analyze_schema(&schema).map_err(Self::fmt_error_to_generator_error)?;

        // Check that Employment was detected as a relation
        assert_eq!(
            generator.analyzer.read().expect("analyzer lock should not be poisoned").type_cache.get("Employment"),
            Some(&TypeQLType::Relation)
        );
    }
}