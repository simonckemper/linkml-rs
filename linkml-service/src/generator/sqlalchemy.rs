//! SQLAlchemy ORM model generator for LinkML schemas
//!
//! This module generates Python SQLAlchemy ORM models from LinkML schemas,
//! enabling database persistence with full ORM capabilities.

use crate::error::LinkMLError;
use crate::generator::traits::{Generator, GeneratorConfig};
use linkml_core::schema::{ClassDefinition, EnumDefinition, Schema, SlotDefinition, TypeDefinition};
use std::collections::{HashMap, HashSet};

/// SQLAlchemy generator configuration
#[derive(Debug, Clone)]
pub struct SQLAlchemyGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// SQLAlchemy version to target (2.0 by default)
    pub sqlalchemy_version: String,
    /// Whether to generate type annotations
    pub use_type_annotations: bool,
    /// Whether to generate relationships
    pub generate_relationships: bool,
    /// Whether to generate indexes
    pub generate_indexes: bool,
    /// Whether to generate constraints
    pub generate_constraints: bool,
    /// Whether to use declarative base or mapped classes
    pub use_declarative_base: bool,
    /// Custom base class name
    pub base_class: String,
    /// Table name prefix
    pub table_prefix: String,
    /// Whether to generate alembic migration support
    pub alembic_support: bool,
}

impl Default for SQLAlchemyGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            sqlalchemy_version: "2.0".to_string(),
            use_type_annotations: true,
            generate_relationships: true,
            generate_indexes: true,
            generate_constraints: true,
            use_declarative_base: true,
            base_class: "Base".to_string(),
            table_prefix: String::new(),
            alembic_support: false,
        }
    }
}

/// SQLAlchemy ORM model generator
pub struct SQLAlchemyGenerator {
    config: SQLAlchemyGeneratorConfig,
}

impl SQLAlchemyGenerator {
    /// Create a new SQLAlchemy generator
    pub fn new(config: SQLAlchemyGeneratorConfig) -> Self {
        Self { config }
    }
    
    /// Generate imports section
    fn generate_imports(&self) -> String {
        let mut imports = vec![
            "from datetime import datetime, date".to_string(),
            "from decimal import Decimal".to_string(),
            "from typing import Optional, List, Dict, Any, Union".to_string(),
            "from enum import Enum".to_string(),
        ];
        
        // SQLAlchemy imports based on version
        if self.config.sqlalchemy_version.starts_with("2.") {
            imports.push("from sqlalchemy import Column, String, Integer, Float, Boolean, DateTime, Date, Text, JSON, ForeignKey, Table, UniqueConstraint, Index, CheckConstraint".to_string());
            imports.push("from sqlalchemy.orm import declarative_base, relationship, mapped_column, Mapped".to_string());
            imports.push("from sqlalchemy.ext.hybrid import hybrid_property".to_string());
        } else {
            imports.push("from sqlalchemy import Column, String, Integer, Float, Boolean, DateTime, Date, Text, JSON, ForeignKey, Table".to_string());
            imports.push("from sqlalchemy.ext.declarative import declarative_base".to_string());
            imports.push("from sqlalchemy.orm import relationship".to_string());
        }
        
        if self.config.alembic_support {
            imports.push("from alembic import op".to_string());
            imports.push("import sqlalchemy as sa".to_string());
        }
        
        imports.join("\n")
    }
    
    /// Generate base class declaration
    fn generate_base(&self) -> String {
        format!("{} = declarative_base()", self.config.base_class)
    }
    
    /// Map LinkML type to SQLAlchemy column type
    fn map_type_to_column(&self, type_name: &str, type_def: Option<&TypeDefinition>) -> String {
        // Check if we have a type definition with a base
        if let Some(td) = type_def {
            if let Some(base) = &td.typeof {
                return self.map_type_to_column(base, None);
            }
        }
        
        // Map based on type name
        match type_name {
            "string" | "str" => "String",
            "integer" | "int" => "Integer",
            "float" | "double" | "decimal" => "Float",
            "boolean" | "bool" => "Boolean",
            "date" => "Date",
            "datetime" => "DateTime",
            "time" => "Time",
            "uri" | "uriorcurie" | "curie" => "String(512)",
            "ncname" => "String(255)",
            _ => "String",
        }.to_string()
    }
    
    /// Generate enum class
    fn generate_enum(&self, name: &str, enum_def: &EnumDefinition) -> String {
        let mut lines = vec![];
        
        // Generate Python enum
        lines.push(format!("class {}(str, Enum):", name));
        lines.push(format!("    \"\"\"{}\"\"\"", 
            enum_def.description.as_deref().unwrap_or("An enumeration")));
        
        if let Some(values) = &enum_def.permissible_values {
            for (value_name, _value_def) in values {
                let safe_name = self.to_python_name(value_name);
                lines.push(format!("    {} = \"{}\"", safe_name.to_uppercase(), value_name));
            }
        }
        
        if enum_def.permissible_values.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
            lines.push("    pass".to_string());
        }
        
        lines.join("\n")
    }
    
    /// Generate association table for many-to-many relationships
    fn generate_association_table(&self, class_name: &str, slot_name: &str, target_class: &str) -> String {
        let table_name = format!("{}{}_{}_{}", 
            self.config.table_prefix,
            self.to_snake_case(class_name),
            self.to_snake_case(slot_name),
            self.to_snake_case(target_class)
        );
        
        format!(
            "{}_association = Table(\n    '{}',\n    {}.metadata,\n    Column('{}_id', Integer, ForeignKey('{}{}.id'), primary_key=True),\n    Column('{}_id', Integer, ForeignKey('{}{}.id'), primary_key=True)\n)",
            self.to_snake_case(slot_name),
            table_name,
            self.config.base_class,
            self.to_snake_case(class_name),
            self.config.table_prefix,
            self.to_snake_case(class_name),
            self.to_snake_case(target_class),
            self.config.table_prefix,
            self.to_snake_case(target_class)
        )
    }
    
    /// Generate model class
    fn generate_class(&self, name: &str, class_def: &ClassDefinition, schema: &Schema) -> String {
        let mut lines = vec![];
        let table_name = format!("{}{}", self.config.table_prefix, self.to_snake_case(name));
        
        // Class declaration
        let parent = if let Some(is_a) = &class_def.is_a {
            self.to_class_name(is_a)
        } else {
            self.config.base_class.clone()
        };
        
        lines.push(format!("class {}({}):", name, parent));
        
        // Docstring
        if let Some(desc) = &class_def.description {
            lines.push(format!("    \"\"\"{}\"\"\"", desc));
        }
        
        // Table name
        if class_def.is_a.is_none() {
            lines.push(format!("    __tablename__ = '{}'", table_name));
        }
        
        // Generate columns for slots
        let mut has_content = false;
        
        // Add primary key if this is a root class
        if class_def.is_a.is_none() {
            lines.push("    ".to_string());
            if self.config.sqlalchemy_version.starts_with("2.") {
                lines.push("    id: Mapped[int] = mapped_column(primary_key=True)".to_string());
            } else {
                lines.push("    id = Column(Integer, primary_key=True)".to_string());
            }
            has_content = true;
        }
        
        // Process slots
        if let Some(slots) = &class_def.slots {
            for slot_name in slots {
                if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                    let column_def = self.generate_column(slot_name, slot_def, schema);
                    if !column_def.is_empty() {
                        if !has_content {
                            lines.push("    ".to_string());
                        }
                        lines.push(format!("    {}", column_def));
                        has_content = true;
                    }
                }
            }
        }
        
        // Process attributes
        if let Some(attrs) = &class_def.attributes {
            for (attr_name, attr_def) in attrs {
                let column_def = self.generate_column(attr_name, attr_def, schema);
                if !column_def.is_empty() {
                    if !has_content {
                        lines.push("    ".to_string());
                    }
                    lines.push(format!("    {}", column_def));
                    has_content = true;
                }
            }
        }
        
        // Generate relationships
        if self.config.generate_relationships {
            let relationships = self.generate_relationships(class_def, schema);
            for rel in relationships {
                if !has_content {
                    lines.push("    ".to_string());
                }
                lines.push(format!("    {}", rel));
                has_content = true;
            }
        }
        
        // Generate constraints
        if self.config.generate_constraints {
            let constraints = self.generate_constraints(name, class_def, schema);
            if !constraints.is_empty() {
                lines.push("    ".to_string());
                lines.push("    __table_args__ = (".to_string());
                for constraint in constraints {
                    lines.push(format!("        {},", constraint));
                }
                lines.push("    )".to_string());
                has_content = true;
            }
        }
        
        if !has_content {
            lines.push("    pass".to_string());
        }
        
        lines.join("\n")
    }
    
    /// Generate column definition
    fn generate_column(&self, name: &str, slot: &SlotDefinition, schema: &Schema) -> String {
        let column_name = self.to_snake_case(name);
        let mut column_args = vec![];
        
        // Determine column type
        let column_type = if let Some(range) = &slot.range {
            // Check if it's an enum
            if schema.enums.as_ref().and_then(|e| e.get(range)).is_some() {
                format!("Enum({})", range)
            } else if schema.classes.as_ref().and_then(|c| c.get(range)).is_some() {
                // This is a foreign key
                return self.generate_foreign_key_column(name, slot, range);
            } else {
                // It's a type
                let type_def = schema.types.as_ref().and_then(|t| t.get(range));
                self.map_type_to_column(range, type_def)
            }
        } else {
            "String".to_string()
        };
        
        // Add column arguments
        if let Some(desc) = &slot.description {
            column_args.push(format!("comment='{}'", desc.replace("'", "\\'")));
        }
        
        if slot.required == Some(true) {
            column_args.push("nullable=False".to_string());
        }
        
        if slot.identifier == Some(true) {
            column_args.push("unique=True".to_string());
        }
        
        // Generate column definition
        if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations {
            let type_annotation = self.get_type_annotation(slot, schema);
            let args = if column_args.is_empty() {
                String::new()
            } else {
                format!(", {}", column_args.join(", "))
            };
            format!("{}: Mapped[{}] = mapped_column({}{})", column_name, type_annotation, column_type, args)
        } else {
            let args = if column_args.is_empty() {
                String::new()
            } else {
                format!(", {}", column_args.join(", "))
            };
            format!("{} = Column({}{})", column_name, column_type, args)
        }
    }
    
    /// Generate foreign key column
    fn generate_foreign_key_column(&self, name: &str, slot: &SlotDefinition, target_class: &str) -> String {
        let column_name = format!("{}_id", self.to_snake_case(name));
        let target_table = format!("{}{}", self.config.table_prefix, self.to_snake_case(target_class));
        
        let nullable = if slot.required == Some(true) { "False" } else { "True" };
        
        if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations {
            format!("{}: Mapped[Optional[int]] = mapped_column(ForeignKey('{}.id'), nullable={})",
                column_name, target_table, nullable)
        } else {
            format!("{} = Column(Integer, ForeignKey('{}.id'), nullable={})",
                column_name, target_table, nullable)
        }
    }
    
    /// Generate relationships
    fn generate_relationships(&self, class_def: &ClassDefinition, schema: &Schema) -> Vec<String> {
        let mut relationships = vec![];
        
        if let Some(slots) = &class_def.slots {
            for slot_name in slots {
                if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                    if let Some(range) = &slot_def.range {
                        if schema.classes.as_ref().and_then(|c| c.get(range)).is_some() {
                            let rel = self.generate_relationship(slot_name, slot_def, range);
                            relationships.push(rel);
                        }
                    }
                }
            }
        }
        
        relationships
    }
    
    /// Generate a single relationship
    fn generate_relationship(&self, name: &str, slot: &SlotDefinition, target_class: &str) -> String {
        let relationship_name = self.to_snake_case(name);
        
        let back_populates = format!("{}_{}_inverse", 
            self.to_snake_case(&self.config.base_class),
            self.to_snake_case(name)
        );
        
        if slot.multivalued == Some(true) {
            if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations {
                format!("{}: Mapped[List['{}']] = relationship(back_populates='{}')",
                    relationship_name, target_class, back_populates)
            } else {
                format!("{} = relationship('{}', back_populates='{}')",
                    relationship_name, target_class, back_populates)
            }
        } else {
            if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations {
                format!("{}: Mapped[Optional['{}']] = relationship(back_populates='{}')",
                    relationship_name, target_class, back_populates)
            } else {
                format!("{} = relationship('{}', back_populates='{}')",
                    relationship_name, target_class, back_populates)
            }
        }
    }
    
    /// Generate table constraints
    fn generate_constraints(&self, class_name: &str, class_def: &ClassDefinition, schema: &Schema) -> Vec<String> {
        let mut constraints = vec![];
        
        // Unique constraints
        let mut unique_together = vec![];
        if let Some(slots) = &class_def.slots {
            for slot_name in slots {
                if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                    if let Some(unique_keys) = &slot_def.unique_keys {
                        for key in unique_keys {
                            unique_together.push(key.clone());
                        }
                    }
                }
            }
        }
        
        if !unique_together.is_empty() {
            let columns = unique_together.iter()
                .map(|k| format!("'{}'", self.to_snake_case(k)))
                .collect::<Vec<_>>()
                .join(", ");
            constraints.push(format!("UniqueConstraint({})", columns));
        }
        
        // Index generation
        if self.config.generate_indexes {
            if let Some(slots) = &class_def.slots {
                for slot_name in slots {
                    if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                        if slot_def.indexed == Some(true) {
                            let column_name = self.to_snake_case(slot_name);
                            constraints.push(format!("Index('idx_{}_{}')", 
                                self.to_snake_case(class_name), column_name));
                        }
                    }
                }
            }
        }
        
        constraints
    }
    
    /// Get type annotation for SQLAlchemy 2.0
    fn get_type_annotation(&self, slot: &SlotDefinition, schema: &Schema) -> String {
        let base_type = if let Some(range) = &slot.range {
            match range.as_str() {
                "string" | "str" => "str",
                "integer" | "int" => "int",
                "float" | "double" => "float",
                "boolean" | "bool" => "bool",
                "date" => "date",
                "datetime" => "datetime",
                _ => "str",
            }
        } else {
            "str"
        };
        
        if slot.required == Some(true) {
            base_type.to_string()
        } else {
            format!("Optional[{}]", base_type)
        }
    }
    
    /// Convert to Python class name
    fn to_class_name(&self, name: &str) -> String {
        name.split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }
    
    /// Convert to Python variable name
    fn to_python_name(&self, name: &str) -> String {
        // Handle reserved words
        match name {
            "class" => "class_",
            "def" => "def_",
            "import" => "import_",
            "from" => "from_",
            "return" => "return_",
            _ => name,
        }.to_string()
    }
    
    /// Convert to snake_case
    fn to_snake_case(&self, name: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;
        
        for (i, ch) in name.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().expect("lowercase char should exist"));
            prev_upper = ch.is_uppercase();
        }
        
        result
    }
}

impl Generator for SQLAlchemyGenerator {
    fn generate(&self, schema: &Schema) -> Result<String, LinkMLError> {
        let mut output = vec![];
        
        // File header
        output.push("\"\"\"".to_string());
        output.push("SQLAlchemy ORM models generated from LinkML schema".to_string());
        if let Some(name) = &schema.name {
            output.push(format!("Schema: {}", name));
        }
        output.push("\"\"\"".to_string());
        output.push(String::new());
        
        // Imports
        output.push(self.generate_imports());
        output.push(String::new());
        
        // Base declaration
        output.push(self.generate_base());
        output.push(String::new());
        
        // Generate enums
        if let Some(enums) = &schema.enums {
            for (name, enum_def) in enums {
                output.push(self.generate_enum(name, enum_def));
                output.push(String::new());
            }
        }
        
        // Generate association tables for many-to-many relationships
        let mut association_tables = HashSet::new();
        if let Some(classes) = &schema.classes {
            for (class_name, class_def) in classes {
                if let Some(slots) = &class_def.slots {
                    for slot_name in slots {
                        if let Some(slot_def) = schema.slots.as_ref().and_then(|s| s.get(slot_name)) {
                            if slot_def.multivalued == Some(true) {
                                if let Some(range) = &slot_def.range {
                                    if schema.classes.as_ref().and_then(|c| c.get(range)).is_some() {
                                        let table_key = format!("{}-{}-{}", class_name, slot_name, range);
                                        if !association_tables.contains(&table_key) {
                                            association_tables.insert(table_key);
                                            output.push(self.generate_association_table(class_name, slot_name, range));
                                            output.push(String::new());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // Generate classes in dependency order
        if let Some(classes) = &schema.classes {
            let ordered_classes = self.order_classes_by_dependency(classes);
            for class_name in ordered_classes {
                if let Some(class_def) = classes.get(&class_name) {
                    output.push(self.generate_class(&class_name, class_def, schema));
                    output.push(String::new());
                }
            }
        }
        
        // Generate Alembic migration support if requested
        if self.config.alembic_support {
            output.push(self.generate_alembic_support());
        }
        
        Ok(output.join("\n"))
    }
    
    fn get_file_extension(&self) -> &str {
        "py"
    }
    
    fn get_default_filename(&self) -> &str {
        "models"
    }
}

impl SQLAlchemyGenerator {
    /// Order classes by dependency (parent classes first)
    fn order_classes_by_dependency(&self, classes: &HashMap<String, ClassDefinition>) -> Vec<String> {
        let mut ordered = vec![];
        let mut visited = HashSet::new();
        
        fn visit(
            name: &str,
            classes: &HashMap<String, ClassDefinition>,
            visited: &mut HashSet<String>,
            ordered: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            
            visited.insert(name.to_string());
            
            if let Some(class_def) = classes.get(name) {
                // Visit parent first
                if let Some(parent) = &class_def.is_a {
                    visit(parent, classes, visited, ordered);
                }
            }
            
            ordered.push(name.to_string());
        }
        
        for name in classes.keys() {
            visit(name, classes, &mut visited, &mut ordered);
        }
        
        ordered
    }
    
    /// Generate Alembic migration support
    fn generate_alembic_support(&self) -> String {
        let mut lines = vec![];
        
        lines.push("# Alembic migration support".to_string());
        lines.push("def upgrade():".to_string());
        lines.push("    \"\"\"Create all tables\"\"\"".to_string());
        lines.push(format!("    {}.metadata.create_all()", self.config.base_class));
        lines.push(String::new());
        lines.push("def downgrade():".to_string());
        lines.push("    \"\"\"Drop all tables\"\"\"".to_string());
        lines.push(format!("    {}.metadata.drop_all()", self.config.base_class));
        
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::schema::{Prefix, SchemaDefinition};
    
    #[test]
    fn test_sqlalchemy_generation() {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        
        // Add a simple class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        person_class.slots = Some(vec!["name".to_string(), "age".to_string()]);
        
        schema.classes = Some(HashMap::from([
            ("Person".to_string(), person_class),
        ]));
        
        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.description = Some("The person's name".to_string());
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.description = Some("The person's age".to_string());
        age_slot.range = Some("integer".to_string());
        
        schema.slots = Some(HashMap::from([
            ("name".to_string(), name_slot),
            ("age".to_string(), age_slot),
        ]));
        
        let config = SQLAlchemyGeneratorConfig::default();
        let generator = SQLAlchemyGenerator::new(config);
        
        let result = generator.generate(&Schema(schema)).expect("should generate SQLAlchemy models");
        
        // Verify key elements
        assert!(result.contains("from sqlalchemy"));
        assert!(result.contains("Base = declarative_base()"));
        assert!(result.contains("class Person(Base):"));
        assert!(result.contains("__tablename__ = 'person'"));
        assert!(result.contains("name"));
        assert!(result.contains("age"));
    }
}