//! `SQLAlchemy` ORM model generator for `LinkML` schemas
//!
//! This module generates Python `SQLAlchemy` ORM models from `LinkML` schemas,
//! enabling database persistence with full ORM capabilities.

use crate::generator::traits::{Generator, GeneratorConfig};
use indexmap::IndexMap;
use linkml_core::error::LinkMLError;
use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
    TypeDefinition,
};
use std::collections::HashSet;

/// `SQL`Alchemy generator configuration
#[derive(Debug, Clone)]
pub struct SQLAlchemyGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// `SQL`Alchemy version to target (2.0 by default)
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

/// `SQL`Alchemy ORM model generator
pub struct SQLAlchemyGenerator {
    config: SQLAlchemyGeneratorConfig,
    /// Additional generator options for customization
    options: super::traits::GeneratorOptions,
}

impl SQLAlchemyGenerator {
    /// Create a new `SQL`Alchemy generator
    #[must_use]
    pub fn new(config: SQLAlchemyGeneratorConfig) -> Self {
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with custom options
    #[must_use]
    pub fn with_options(
        config: SQLAlchemyGeneratorConfig,
        options: super::traits::GeneratorOptions,
    ) -> Self {
        Self { config, options }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
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
            imports.push(
                "from sqlalchemy.orm import declarative_base, relationship, mapped_column, Mapped"
                    .to_string(),
            );
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

        // Add custom imports from options
        if let Some(custom_imports) = self.get_custom_option("custom_imports") {
            imports.push(custom_imports.clone());
        }

        imports.join(
            "
",
        )
    }

    /// Generate base class declaration
    fn generate_base(&self) -> String {
        format!("{} = declarative_base()", self.config.base_class)
    }

    /// Map `LinkML` type to `SQLAlchemy` column type
    fn map_type_to_column(&self, type_name: &str, type_def: Option<&TypeDefinition>) -> String {
        // Check if we have a type definition with a base
        if let Some(td) = type_def
            && let Some(base) = &td.base_type
        {
            return self.map_type_to_column(base, None);
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
        }
        .to_string()
    }

    /// Generate enum class
    fn generate_enum(&self, name: &str, enum_def: &EnumDefinition) -> String {
        let mut lines = vec![];

        // Generate Python enum
        lines.push(format!("class {name}(str, Enum):"));
        lines.push(format!(
            "    \"\"\"{}\"\"\"",
            enum_def.description.as_deref().unwrap_or("An enumeration")
        ));

        if !enum_def.permissible_values.is_empty() {
            for value in &enum_def.permissible_values {
                let value_name = match value {
                    PermissibleValue::Simple(name) => name,
                    PermissibleValue::Complex { text, .. } => text,
                };
                let safe_name = self.to_python_name(value_name);
                lines.push(format!(
                    "    {} = \"{}\"",
                    safe_name.to_uppercase(),
                    value_name
                ));
            }
        }

        if enum_def.permissible_values.is_empty() {
            lines.push("    pass".to_string());
        }

        lines.join(
            "
",
        )
    }

    /// Generate association table for many-to-many relationships
    fn generate_association_table(
        &self,
        class_name: &str,
        slot_name: &str,
        target_class: &str,
    ) -> String {
        let table_name = format!(
            "{}{}_{}_{}",
            self.config.table_prefix,
            self.to_snake_case(class_name),
            self.to_snake_case(slot_name),
            self.to_snake_case(target_class)
        );

        format!(
            "{}_association = Table(
    '{}',
    {}.metadata,
    Column('{}_id', Integer, ForeignKey('{}{}.id'), primary_key=True),
    Column('{}_id', Integer, ForeignKey('{}{}.id'), primary_key=True)
)",
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
    fn generate_class(
        &self,
        name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> String {
        let mut lines = vec![];
        let table_name = format!("{}{}", self.config.table_prefix, self.to_snake_case(name));

        // Class declaration
        let parent = if let Some(is_a) = &class_def.is_a {
            self.to_class_name(is_a)
        } else {
            self.config.base_class.clone()
        };

        lines.push(format!("class {name}({parent}):"));

        // Docstring
        if let Some(desc) = &class_def.description {
            lines.push(format!("    \"\"\"{desc}\"\"\""));
        }

        // Table name
        if class_def.is_a.is_none() {
            lines.push(format!("    __tablename__ = '{table_name}'"));
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
        if !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let column_def = self.generate_column(slot_name, slot_def, schema);
                    if !column_def.is_empty() {
                        if !has_content {
                            lines.push("    ".to_string());
                        }
                        lines.push(format!("    {column_def}"));
                        has_content = true;
                    }
                }
            }
        }

        // Process attributes
        if !class_def.attributes.is_empty() {
            for (attr_name, attr_def) in &class_def.attributes {
                let column_def = self.generate_column(attr_name, attr_def, schema);
                if !column_def.is_empty() {
                    if !has_content {
                        lines.push("    ".to_string());
                    }
                    lines.push(format!("    {column_def}"));
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
                lines.push(format!("    {rel}"));
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
                    lines.push(format!("        {constraint},"));
                }
                lines.push("    )".to_string());
                has_content = true;
            }
        }

        if !has_content {
            lines.push("    pass".to_string());
        }

        lines.join(
            "
",
        )
    }

    /// Generate column definition
    fn generate_column(
        &self,
        name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> String {
        let column_name = self.to_snake_case(name);
        let mut column_args = vec![];

        // Determine column type
        let column_type = if let Some(range) = &slot.range {
            // Check if it's an enum
            if schema.enums.contains_key(range) {
                format!("Enum({range})")
            } else if schema.classes.contains_key(range) {
                // This is a foreign key
                return self.generate_foreign_key_column(name, slot, range);
            } else {
                // It's a type
                let type_def = schema.types.get(range);
                self.map_type_to_column(range, type_def)
            }
        } else {
            "String".to_string()
        };

        // Add column arguments
        if let Some(desc) = &slot.description {
            column_args.push(format!("comment='{}'", desc.replace('\'', "\\'")));
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
            format!("{column_name}: Mapped[{type_annotation}] = mapped_column({column_type}{args})")
        } else {
            let args = if column_args.is_empty() {
                String::new()
            } else {
                format!(", {}", column_args.join(", "))
            };
            format!("{column_name} = Column({column_type}{args})")
        }
    }

    /// Generate foreign key column
    fn generate_foreign_key_column(
        &self,
        name: &str,
        slot: &SlotDefinition,
        target_class: &str,
    ) -> String {
        let column_name = format!("{}_id", self.to_snake_case(name));
        let target_table = format!(
            "{}{}",
            self.config.table_prefix,
            self.to_snake_case(target_class)
        );

        let nullable = if slot.required == Some(true) {
            "False"
        } else {
            "True"
        };

        if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations {
            format!(
                "{column_name}: Mapped[Optional[int]] = mapped_column(ForeignKey('{target_table}.id'), nullable={nullable})"
            )
        } else {
            format!(
                "{column_name} = Column(Integer, ForeignKey('{target_table}.id'), nullable={nullable})"
            )
        }
    }

    /// Generate relationships
    fn generate_relationships(
        &self,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut relationships = vec![];

        if !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    let rel = self.generate_relationship(slot_name, slot_def, range);
                    relationships.push(rel);
                }
            }
        }

        relationships
    }

    /// Generate a single relationship
    fn generate_relationship(
        &self,
        name: &str,
        slot: &SlotDefinition,
        target_class: &str,
    ) -> String {
        let relationship_name = self.to_snake_case(name);

        let back_populates = format!(
            "{}_{}_inverse",
            self.to_snake_case(&self.config.base_class),
            self.to_snake_case(name)
        );

        if slot.multivalued == Some(true) {
            if self.config.sqlalchemy_version.starts_with("2.") && self.config.use_type_annotations
            {
                format!(
                    "{relationship_name}: Mapped[List['{target_class}']] = relationship(back_populates='{back_populates}')"
                )
            } else {
                format!(
                    "{relationship_name} = relationship('{target_class}', back_populates='{back_populates}')"
                )
            }
        } else if self.config.sqlalchemy_version.starts_with("2.")
            && self.config.use_type_annotations
        {
            format!(
                "{relationship_name}: Mapped[Optional['{target_class}']] = relationship(back_populates='{back_populates}')"
            )
        } else {
            format!(
                "{relationship_name} = relationship('{target_class}', back_populates='{back_populates}')"
            )
        }
    }

    /// Generate table constraints
    fn generate_constraints(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut constraints = vec![];

        // Unique constraints
        let unique_together: Vec<String> = vec![];
        if !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(_slot_def) = schema.slots.get(slot_name) {
                    // This field is not present in the current LinkML specification
                    // if let Some(unique_keys) = &slot_def.unique_keys {
                    //     for key in unique_keys {
                    //         unique_together.push(key.clone());
                    //     }
                    // }
                }
            }
        }

        if !unique_together.is_empty() {
            let columns = unique_together
                .iter()
                .map(|k| format!("'{}'", self.to_snake_case(k)))
                .collect::<Vec<_>>()
                .join(", ");
            constraints.push(format!("UniqueConstraint({columns})"));
        }

        // Index generation
        if self.config.generate_indexes && !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(_slot_def) = schema.slots.get(slot_name) {
                    // This field is not present in the current LinkML specification
                    // if slot_def.indexed == Some(true) {
                    if false {
                        let column_name = self.to_snake_case(slot_name);
                        constraints.push(format!(
                            "Index('idx_{}_{}')",
                            self.to_snake_case(class_name),
                            column_name
                        ));
                    }
                }
            }
        }

        constraints
    }

    /// Get type annotation for `SQL`Alchemy 2.0
    fn get_type_annotation(&self, slot: &SlotDefinition, _schema: &SchemaDefinition) -> String {
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
            format!("Optional[{base_type}]")
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
        }
        .to_string()
    }

    /// Convert to `snake_case`
    fn to_snake_case(&self, name: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;

        for (i, ch) in name.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_upper = ch.is_uppercase();
        }

        result
    }
}

impl Generator for SQLAlchemyGenerator {
    fn name(&self) -> &'static str {
        "sqlalchemy"
    }

    fn description(&self) -> &'static str {
        "Generate SQLAlchemy ORM models from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for sqlalchemy generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> Result<String, LinkMLError> {
        let mut output = vec![];

        // File header
        output.push("\"\"\"".to_string());
        output.push("SQLAlchemy ORM models generated from LinkML schema".to_string());
        if !schema.name.is_empty() {
            output.push(format!("# Schema: {}", schema.name));
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
        if !schema.enums.is_empty() {
            for (name, enum_def) in &schema.enums {
                output.push(self.generate_enum(name, enum_def));
                output.push(String::new());
            }
        }

        // Generate association tables for many-to-many relationships
        let mut association_tables = HashSet::new();
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                if !class_def.slots.is_empty() {
                    for slot_name in &class_def.slots {
                        if let Some(slot_def) = schema.slots.get(slot_name)
                            && slot_def.multivalued == Some(true)
                            && let Some(range) = &slot_def.range
                            && schema.classes.contains_key(range)
                        {
                            let table_key = format!("{class_name}-{slot_name}-{range}");
                            if !association_tables.contains(&table_key) {
                                association_tables.insert(table_key);
                                output.push(
                                    self.generate_association_table(class_name, slot_name, range),
                                );
                                output.push(String::new());
                            }
                        }
                    }
                }
            }
        }

        // Generate classes in dependency order
        if !schema.classes.is_empty() {
            let ordered_classes = self.order_classes_by_dependency(&schema.classes);
            for class_name in ordered_classes {
                if let Some(class_def) = schema.classes.get(&class_name) {
                    output.push(self.generate_class(&class_name, class_def, schema));
                    output.push(String::new());
                }
            }
        }

        // Generate Alembic migration support if requested
        if self.config.alembic_support {
            output.push(self.generate_alembic_support());
        }

        Ok(output.join(
            "
",
        ))
    }

    fn get_file_extension(&self) -> &'static str {
        "py"
    }

    fn get_default_filename(&self) -> &'static str {
        "models"
    }
}

impl SQLAlchemyGenerator {
    /// Order classes by dependency (parent classes first)
    fn order_classes_by_dependency(
        &self,
        classes: &IndexMap<String, ClassDefinition>,
    ) -> Vec<String> {
        let mut ordered = vec![];
        let mut visited = HashSet::new();

        fn visit(
            name: &str,
            classes: &IndexMap<String, ClassDefinition>,
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
        lines.push(format!(
            "    {}.metadata.create_all()",
            self.config.base_class
        ));
        lines.push(String::new());
        lines.push("def downgrade():".to_string());
        lines.push("    \"\"\"Drop all tables\"\"\"".to_string());
        lines.push(format!(
            "    {}.metadata.drop_all()",
            self.config.base_class
        ));

        lines.join(
            "
",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_sqlalchemy_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Add a simple class
        let person_class = ClassDefinition {
            description: Some("A person".to_string()),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);

        // Add slots
        let name_slot = SlotDefinition {
            description: Some("The person's name".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };

        let age_slot = SlotDefinition {
            description: Some("The person's age".to_string()),
            range: Some("integer".to_string()),
            ..Default::default()
        };

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);

        let schema = SchemaDefinition {
            name: "TestSchema".to_string(),
            classes,
            slots,
            ..Default::default()
        };

        let config = SQLAlchemyGeneratorConfig::default();
        let generator = SQLAlchemyGenerator::new(config);

        let result = generator
            .generate(&schema)
            .expect("should generate SQLAlchemy models: {}");

        // Verify key elements
        assert!(result.contains("from sqlalchemy"));
        assert!(result.contains("Base = declarative_base()"));
        assert!(result.contains("class Person(Base):"));
        assert!(result.contains("__tablename__ = 'person'"));
        assert!(result.contains("name"));
        assert!(result.contains("age"));
        Ok(())
    }
}
