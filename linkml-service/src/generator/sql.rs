//! SQL DDL generation for `LinkML` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// SQL DDL generator for `LinkML` schemas
pub struct SQLGenerator {
    /// Generator name
    name: String,
}

impl SQLGenerator {
    /// Create a new SQL generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "sql".to_string(),
        }
    }
    
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }

    /// Generate SQL table for a class
    fn generate_table(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Skip abstract classes unless requested
        if class.abstract_ == Some(true) && options.get_custom("generate_abstract") != Some("true")
        {
            return Ok(output);
        }

        let table_name = self.convert_table_name(class_name);

        // Table comment
        if options.include_docs {
            if let Some(desc) = &class.description {
                writeln!(&mut output, "-- {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // CREATE TABLE statement
        writeln!(&mut output, "CREATE TABLE {table_name} (").map_err(Self::fmt_error_to_generator_error)?;

        // Primary key (ID column)
        writeln!(
            &mut output,
            "{}id {} PRIMARY KEY,",
            indent.single(),
            self.get_id_type(options)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate columns
        let columns = self.generate_columns(class, schema, options, indent)?;
        for (i, column) in columns.iter().enumerate() {
            write!(&mut output, "{column}").map_err(Self::fmt_error_to_generator_error)?;
            if i < columns.len() - 1 {
                writeln!(&mut output, ",").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Foreign key constraints
        let fk_constraints = self.generate_foreign_keys(class, schema, indent)?;
        if !fk_constraints.is_empty() {
            writeln!(&mut output, ",").map_err(Self::fmt_error_to_generator_error)?;
            for (i, constraint) in fk_constraints.iter().enumerate() {
                write!(&mut output, "{constraint}").map_err(Self::fmt_error_to_generator_error)?;
                if i < fk_constraints.len() - 1 {
                    writeln!(&mut output, ",").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, ");").map_err(Self::fmt_error_to_generator_error)?;

        // Create indexes
        let indexes = self.generate_indexes(&table_name, class, schema, options)?;
        if !indexes.is_empty() {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            for index in indexes {
                writeln!(&mut output, "{index}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate columns for a table
    fn generate_columns(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<Vec<String>> {
        let mut columns = Vec::new();

        // Add audit columns if requested
        if options.get_custom("add_audit_columns") == Some("true") {
            columns.push(format!(
                "{}created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP",
                indent.single()
            ));
            columns.push(format!(
                "{}updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP",
                indent.single()
            ));
            columns.push(format!("{}created_by VARCHAR(255)", indent.single()));
            columns.push(format!("{}updated_by VARCHAR(255)", indent.single()));
        }

        // Collect all slots including inherited ones
        let slots = self.collect_all_slots(class, schema)?;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let column_name = self.convert_column_name(slot_name);
                let column_type = self.get_sql_type(slot, schema, options)?;

                let mut column_def = format!("{}{} {}", indent.single(), column_name, column_type);

                // Add constraints
                if slot.required == Some(true) {
                    column_def.push_str(" NOT NULL");
                }

                // Add default value if specified
                if let Some(default) = options.get_custom(&format!("default_{slot_name}")) {
                    column_def.push_str(&format!(" DEFAULT {default}"));
                }

                // Add CHECK constraint for pattern
                if let Some(pattern) = &slot.pattern {
                    if options.get_custom("dialect") == Some("postgresql") {
                        column_def.push_str(&format!(" CHECK ({column_name} ~ '{pattern}')"));
                    }
                }

                // Add column comment if dialect supports it
                if options.include_docs && options.get_custom("dialect") == Some("postgresql") {
                    if let Some(desc) = &slot.description {
                        column_def.push_str(&format!(" -- {desc}"));
                    }
                }

                columns.push(column_def);
            }
        }

        Ok(columns)
    }

    /// Generate foreign key constraints
    fn generate_foreign_keys(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<Vec<String>> {
        let mut constraints = Vec::new();
        let slots = self.collect_all_slots(class, schema)?;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if let Some(range) = &slot.range {
                    // Check if range references another class
                    if schema.classes.contains_key(range) {
                        let column_name = self.convert_column_name(slot_name);
                        let ref_table = self.convert_table_name(range);
                        let constraint_name = format!(
                            "fk_{}_{}",
                            self.convert_table_name(&class.name),
                            column_name
                        );

                        let constraint = format!(
                            "{}CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}(id)",
                            indent.single(),
                            constraint_name,
                            column_name,
                            ref_table
                        );

                        constraints.push(constraint);
                    }
                }
            }
        }

        Ok(constraints)
    }

    /// Generate indexes for a table
    fn generate_indexes(
        &self,
        table_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<String>> {
        let mut indexes = Vec::new();

        // Add indexes for foreign keys
        let slots = self.collect_all_slots(class, schema)?;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let column_name = self.convert_column_name(slot_name);

                // Index foreign keys
                if let Some(range) = &slot.range {
                    if schema.classes.contains_key(range) {
                        let index_name = format!("idx_{table_name}_{column_name}");
                        indexes.push(format!(
                            "CREATE INDEX {index_name} ON {table_name}({column_name});"
                        ));
                    }
                }

                // Index identifier fields
                if slot.identifier == Some(true) {
                    let index_name = format!("idx_{table_name}_unique_{column_name}");
                    indexes.push(format!(
                        "CREATE UNIQUE INDEX {index_name} ON {table_name}({column_name});"
                    ));
                }
            }
        }

        // Add audit column indexes if requested
        if options.get_custom("add_audit_columns") == Some("true") {
            indexes.push(format!(
                "CREATE INDEX idx_{table_name}_created_at ON {table_name}(created_at);"
            ));
            indexes.push(format!(
                "CREATE INDEX idx_{table_name}_updated_at ON {table_name}(updated_at);"
            ));
        }

        Ok(indexes)
    }

    /// Generate junction tables for many-to-many relationships
    fn generate_junction_tables(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let mut generated = HashSet::new();

        for (class_name, class) in &schema.classes {
            if class.abstract_ == Some(true) {
                continue;
            }

            let slots = self.collect_all_slots(class, schema)?;

            for slot_name in &slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    // Check if this is a many-to-many relationship
                    if slot.multivalued == Some(true) {
                        if let Some(range) = &slot.range {
                            if schema.classes.contains_key(range) {
                                // Create junction table name
                                let table1 = self.convert_table_name(class_name);
                                let table2 = self.convert_table_name(range);
                                let junction_name = if table1 < table2 {
                                    format!("{table1}_{table2}")
                                } else {
                                    format!("{table2}_{table1}")
                                };

                                // Only generate once
                                if generated.insert(junction_name.clone()) {
                                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                                    writeln!(
                                        &mut output,
                                        "-- Junction table for {class_name} <-> {range}"
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;
                                    writeln!(&mut output, "CREATE TABLE {junction_name} (")
                                        .map_err(Self::fmt_error_to_generator_error)?;

                                    let id_type = self.get_id_type(options);

                                    writeln!(
                                        &mut output,
                                        "{}{}_id {} NOT NULL,",
                                        indent.single(),
                                        table1,
                                        id_type
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;
                                    writeln!(
                                        &mut output,
                                        "{}{}_id {} NOT NULL,",
                                        indent.single(),
                                        table2,
                                        id_type
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;

                                    writeln!(
                                        &mut output,
                                        "{}PRIMARY KEY ({}_id, {}_id),",
                                        indent.single(),
                                        table1,
                                        table2
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;

                                    writeln!(
                                        &mut output,
                                        "{}FOREIGN KEY ({}_id) REFERENCES {}(id),",
                                        indent.single(),
                                        table1,
                                        table1
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;
                                    writeln!(
                                        &mut output,
                                        "{}FOREIGN KEY ({}_id) REFERENCES {}(id)",
                                        indent.single(),
                                        table2,
                                        table2
                                    )
                                    .map_err(Self::fmt_error_to_generator_error)?;

                                    writeln!(&mut output, ");").map_err(Self::fmt_error_to_generator_error)?;

                                    // Create indexes
                                    writeln!(&mut output, "CREATE INDEX idx_{junction_name}_{table1}_id ON {junction_name}({table1}_id);").map_err(Self::fmt_error_to_generator_error)?;
                                    writeln!(&mut output, "CREATE INDEX idx_{junction_name}_{table2}_id ON {junction_name}({table2}_id);").map_err(Self::fmt_error_to_generator_error)?;
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(output)
    }

    /// Generate enum tables or CHECK constraints
    fn generate_enums(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        if schema.enums.is_empty() {
            return Ok(output);
        }

        let dialect = options.get_custom("dialect").unwrap_or("standard");

        if dialect == "postgresql" {
            // PostgreSQL native ENUM types
            writeln!(&mut output, "-- Enum Types").map_err(Self::fmt_error_to_generator_error)?;
            for (enum_name, enum_def) in &schema.enums {
                if options.include_docs {
                    if let Some(desc) = &enum_def.description {
                        writeln!(&mut output, "-- {desc}").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                let type_name = self.convert_table_name(enum_name);
                write!(&mut output, "CREATE TYPE {type_name} AS ENUM (").map_err(Self::fmt_error_to_generator_error)?;

                let values: Vec<String> = enum_def
                    .permissible_values
                    .iter()
                    .map(|v| match v {
                        PermissibleValue::Simple(text) => format!("'{text}'"),
                        PermissibleValue::Complex { text, .. } => format!("'{text}'"),
                    })
                    .collect();

                write!(&mut output, "{}", values.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, ");").map_err(Self::fmt_error_to_generator_error)?;
            }
        } else {
            // Standard SQL - create lookup tables
            writeln!(&mut output, "-- Enum Lookup Tables").map_err(Self::fmt_error_to_generator_error)?;
            for (enum_name, enum_def) in &schema.enums {
                if options.include_docs {
                    if let Some(desc) = &enum_def.description {
                        writeln!(&mut output, "-- {desc}").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                let table_name = format!("{}_enum", self.convert_table_name(enum_name));
                writeln!(&mut output, "CREATE TABLE {table_name} (").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "{}code VARCHAR(255) PRIMARY KEY,",
                    indent.single()
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "{}label VARCHAR(255) NOT NULL,",
                    indent.single()
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "{}description TEXT", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, ");").map_err(Self::fmt_error_to_generator_error)?;

                // Insert enum values
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                for value in &enum_def.permissible_values {
                    match value {
                        PermissibleValue::Simple(text) => {
                            writeln!(&mut output, 
                                "INSERT INTO {table_name} (code, label) VALUES ('{text}', '{text}');"
                            ).map_err(Self::fmt_error_to_generator_error)?;
                        }
                        PermissibleValue::Complex {
                            text, description, ..
                        } => {
                            let desc_sql = description.as_deref().map_or_else(
                                || "NULL".to_string(),
                                |d| format!("'{}'", d.replace('\'', "''")),
                            );

                            writeln!(&mut output,
                                "INSERT INTO {table_name} (code, label, description) VALUES ('{text}', '{text}', {desc_sql});"
                            ).map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }
                }
            }
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(output)
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<String>> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();

        // Add direct slots
        for slot in &class.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add inherited slots
        if let Some(parent) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent) {
                let parent_slots = self.collect_all_slots(parent_class, schema)?;
                for slot in parent_slots {
                    if seen.insert(slot.clone()) {
                        all_slots.push(slot);
                    }
                }
            }
        }

        Ok(all_slots)
    }

    /// Get SQL type for a slot
    ///
    /// # Errors
    ///
    /// Returns an error if the slot type cannot be mapped to SQL.
    fn get_sql_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let base_type = self.get_base_sql_type(&slot.range, schema, options)?;

        // Handle multivalued slots (arrays)
        if slot.multivalued == Some(true) {
            let dialect = options.get_custom("dialect").unwrap_or("standard");
            match dialect {
                "postgresql" => Ok(format!("{base_type}[]")),
                _ => Ok("TEXT".to_string()), // JSON array as text
            }
        } else {
            Ok(base_type)
        }
    }

    /// Get base SQL type from `LinkML` range
    ///
    /// # Errors
    ///
    /// Returns an error if the range type cannot be mapped to SQL.
    fn get_base_sql_type(
        &self,
        range: &Option<String>,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let dialect = options.get_custom("dialect").unwrap_or("standard");

        match range.as_deref() {
            Some("string" | "str") => Ok("VARCHAR(255)".to_string()),
            Some("integer" | "int") => Ok("INTEGER".to_string()),
            Some("float" | "double") => Ok("DOUBLE PRECISION".to_string()),
            Some("decimal") => Ok("DECIMAL(19,4)".to_string()),
            Some("boolean" | "bool") => match dialect {
                "postgresql" => Ok("BOOLEAN".to_string()),
                "mysql" => Ok("TINYINT(1)".to_string()),
                _ => Ok("BOOLEAN".to_string()),
            },
            Some("date") => Ok("DATE".to_string()),
            Some("datetime") => match dialect {
                "postgresql" => Ok("TIMESTAMP WITH TIME ZONE".to_string()),
                _ => Ok("TIMESTAMP".to_string()),
            },
            Some("uri" | "url") => Ok("TEXT".to_string()),
            Some(other) => {
                // Check if it's an enum
                if schema.enums.contains_key(other) {
                    match dialect {
                        "postgresql" => Ok(self.convert_table_name(other)),
                        _ => Ok("VARCHAR(255)".to_string()),
                    }
                } else if schema.classes.contains_key(other) {
                    // Foreign key reference
                    Ok(self.get_id_type(options))
                } else {
                    Ok("TEXT".to_string())
                }
            }
            None => Ok("TEXT".to_string()),
        }
    }

    /// Get the ID column type based on options
    fn get_id_type(&self, options: &GeneratorOptions) -> String {
        match options.get_custom("id_type") {
            Some("uuid") => match options.get_custom("dialect") {
                Some("postgresql") => "UUID DEFAULT gen_random_uuid()".to_string(),
                _ => "CHAR(36)".to_string(),
            },
            Some("serial") => match options.get_custom("dialect") {
                Some("postgresql") => "SERIAL".to_string(),
                Some("mysql") => "INTEGER AUTO_INCREMENT".to_string(),
                _ => "INTEGER".to_string(),
            },
            Some("bigserial") => match options.get_custom("dialect") {
                Some("postgresql") => "BIGSERIAL".to_string(),
                Some("mysql") => "BIGINT AUTO_INCREMENT".to_string(),
                _ => "BIGINT".to_string(),
            },
            _ => "INTEGER".to_string(),
        }
    }

    /// Convert to SQL table name
    fn convert_table_name(&self, name: &str) -> String {
        // Convert to snake_case and lowercase
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

    /// Convert to SQL column name
    fn convert_column_name(&self, name: &str) -> String {
        self.convert_table_name(name)
    }
}

impl Default for SQLGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for SQLGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate SQL DDL from LinkML schemas with support for multiple dialects"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".sql", ".ddl"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        self.validate_schema(schema).await?;

        let mut output = String::new();
        let indent = &options.indent;

        // Header
        writeln!(&mut output, "-- SQL DDL generated from LinkML schema").map_err(Self::fmt_error_to_generator_error)?;
        if !schema.name.is_empty() {
            writeln!(&mut output, "-- Schema: {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "-- Description: {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        let dialect = options.get_custom("dialect").unwrap_or("standard");
        writeln!(&mut output, "-- Dialect: {dialect}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate enum types/tables first
        let enum_output = self.generate_enums(schema, options, indent)?;
        if !enum_output.is_empty() {
            output.push_str(&enum_output);
        }

        // Generate tables
        if !schema.classes.is_empty() {
            writeln!(&mut output, "-- Tables").map_err(Self::fmt_error_to_generator_error)?;
            for (class_name, class) in &schema.classes {
                let table_output =
                    self.generate_table(class_name, class, schema, options, indent)?;
                if !table_output.is_empty() {
                    output.push_str(&table_output);
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Generate junction tables for many-to-many relationships
        let junction_output = self.generate_junction_tables(schema, options, indent)?;
        if !junction_output.is_empty() {
            writeln!(&mut output, "-- Junction Tables").map_err(Self::fmt_error_to_generator_error)?;
            output.push_str(&junction_output);
        }

        // Create output
        let filename = format!(
            "{}.sql",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("schema_name".to_string(), schema.name.clone());
        metadata.insert("dialect".to_string(), dialect.to_string());

        Ok(vec![GeneratedOutput {
            content: output,
            filename,
            metadata,
        }])
    }
}

impl CodeFormatter for SQLGenerator {
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        doc.lines()
            .map(|line| format!("{prefix}-- {line}"))
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
            .map(|item| format!("{prefix}{}", item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        s.replace('\'', "''")
    }

    fn convert_identifier(&self, id: &str) -> String {
        self.convert_table_name(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sql_generation() {
        let generator = SQLGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "test".to_string();
        schema.name = "test_schema".to_string();

        // Add a slot
        let mut slot = SlotDefinition::default();
        slot.name = "name".to_string();
        slot.range = Some("string".to_string());
        slot.required = Some(true);

        schema.slots.insert("name".to_string(), slot);

        // Add a class
        let mut class = ClassDefinition::default();
        class.name = "Person".to_string();
        class.slots = vec!["name".to_string()];

        schema.classes.insert("Person".to_string(), class);

        let options = GeneratorOptions::new();
        let outputs = generator.generate(&schema, &options).await.expect("should generate SQL output");

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.contains("CREATE TABLE person"));
        assert!(outputs[0].content.contains("name VARCHAR(255) NOT NULL"));
    }

    #[test]
    fn test_table_name_conversion() {
        let generator = SQLGenerator::new();

        assert_eq!(generator.convert_table_name("PersonName"), "person_name");
        assert_eq!(generator.convert_table_name("HTTPResponse"), "httpresponse");
        assert_eq!(generator.convert_table_name("person_name"), "person_name");
    }
}
