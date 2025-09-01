//! `TypeQL` generation implementation for `TypeDB` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use async_trait::async_trait;
use linkml_core::error::LinkMLError;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// `TypeQL` schema generator for `TypeDB`
pub struct TypeQLGenerator {
    /// Generator name
    name: String,
}

impl TypeQLGenerator {
    /// Create a new `TypeQL` generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "typeql".to_string(),
        }
    }

    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Formatting error: {}", err),
        ))
    }

    /// Generate `TypeQL` for a class
    fn generate_class_typeql(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add documentation if requested
        if let Some(desc) = &class.description {
            writeln!(&mut output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine if this is an entity or relation
        let is_relation = self.is_relation_class(class, schema);

        if is_relation {
            self.generate_relation(&mut output, class_name, class, schema, indent)?;
        } else {
            self.generate_entity(&mut output, class_name, class, schema, indent)?;
        }

        Ok(output)
    }

    /// Generate entity type
    fn generate_entity(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let type_name = self.convert_identifier(name);

        // Entity definition
        write!(output, "{type_name} sub entity").map_err(Self::fmt_error_to_generator_error)?;

        // Add parent if specified
        if let Some(parent) = &class.is_a {
            let parent_name = self.convert_identifier(parent);
            write!(output, " sub {parent_name}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add attributes
        let attributes = self.collect_class_attributes(class, schema);
        if attributes.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;

            for (i, attr) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr)
                    .map_err(Self::fmt_error_to_generator_error)?;
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

    /// Generate relation type
    fn generate_relation(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let type_name = self.convert_identifier(name);

        // Relation definition
        write!(output, "{type_name} sub relation").map_err(Self::fmt_error_to_generator_error)?;

        // Add parent if specified
        if let Some(parent) = &class.is_a {
            let parent_name = self.convert_identifier(parent);
            write!(output, " sub {parent_name}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;

        // Add roles based on slots
        let roles = self.collect_relation_roles(class, schema)?;
        for (i, (role, _types)) in roles.iter().enumerate() {
            write!(output, "{}relates {}", indent.single(), role)
                .map_err(Self::fmt_error_to_generator_error)?;
            if i < roles.len() - 1 {
                writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Add attributes
        let attributes = self.collect_class_attributes(class, schema);
        if attributes.is_empty() {
            writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            for (i, attr) in attributes.iter().enumerate() {
                write!(output, "{}owns {}", indent.single(), attr)
                    .map_err(Self::fmt_error_to_generator_error)?;
                if i < attributes.len() - 1 {
                    writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Add role players
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        for (role, types) in roles {
            for player_type in types {
                writeln!(output, "{player_type} plays {type_name}:{role};")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate attributes from slots
    fn generate_attributes(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let mut generated_attrs = HashSet::new();

        // Collect all slots that are used as attributes
        if !schema.classes.is_empty() {
            for class in schema.classes.values() {
                if !class.slots.is_empty() {
                    for slot_name in &class.slots {
                        if !generated_attrs.contains(slot_name) {
                            if let Some(slot) = schema.slots.get(slot_name) {
                                self.generate_attribute(output, slot_name, slot)?;
                                generated_attrs.insert(slot_name.clone());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate a single attribute
    fn generate_attribute(
        &self,
        output: &mut String,
        name: &str,
        slot: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let attr_name = self.convert_identifier(name);
        let value_type = self.map_range_to_typeql(&slot.range);

        // Add documentation if present
        if let Some(desc) = &slot.description {
            writeln!(output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "{attr_name} sub attribute, value {value_type};")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate rules from schema constraints
    fn generate_rules(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Generate rules for required attributes
        if !schema.classes.is_empty() {
            for (class_name, class) in &schema.classes {
                if !class.slots.is_empty() {
                    for slot_name in &class.slots {
                        if let Some(slot) = schema.slots.get(slot_name) {
                            if slot.required == Some(true) {
                                self.generate_required_rule(output, class_name, slot_name, indent)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate a rule for required attributes
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
        writeln!(output, "}} then {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}$x has {};",
            indent.single(),
            self.convert_identifier(slot_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Check if a class represents a relation
    fn is_relation_class(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> bool {
        // A class is a relation if it has slots that reference other classes
        if !class.slots.is_empty() {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    if let Some(range) = &slot.range {
                        // Check if range is a class reference
                        if schema.classes.contains_key(range) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Collect attributes for a class
    fn collect_class_attributes(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut attributes = Vec::new();

        if !class.slots.is_empty() {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    // Only include non-relation slots as attributes
                    if let Some(range) = &slot.range {
                        if !schema.classes.contains_key(range) {
                            attributes.push(self.convert_identifier(slot_name));
                        }
                    }
                }
            }
        }

        attributes
    }

    /// Collect roles for a relation
    fn collect_relation_roles(
        &self,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<Vec<(String, Vec<String>)>> {
        let mut roles = Vec::new();

        if !class.slots.is_empty() {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    if let Some(range) = &slot.range {
                        // Check if range is a class reference
                        if schema.classes.contains_key(range) {
                            let role_name = self.convert_identifier(slot_name);
                            let player_type = self.convert_identifier(range);
                            roles.push((role_name, vec![player_type]));
                        }
                    }
                }
            }
        }

        Ok(roles)
    }

    /// Map `LinkML` range to `TypeQL` value type
    fn map_range_to_typeql(&self, range: &Option<String>) -> &'static str {
        let _ = self;
        match range.as_deref() {
            Some("string" | "str" | "uri" | "url") => "string",
            Some("integer" | "int") => "long",
            Some("float" | "double" | "decimal") => "double",
            Some("boolean" | "bool") => "boolean",
            Some("date" | "datetime") => "datetime",
            Some(_) | None => "string", // Default to string for unknown types
        }
    }
}

impl Default for TypeQLGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsyncGenerator for TypeQLGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate TypeQL schema definitions for TypeDB from LinkML schemas"
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
        AsyncGenerator::validate_schema(self, schema).await?;

        let mut output = String::new();
        let indent = &options.indent;

        // Header
        writeln!(&mut output, "# TypeQL Schema generated from LinkML")
            .map_err(Self::fmt_error_to_generator_error)?;
        if !schema.name.is_empty() {
            writeln!(&mut output, "# Schema: {}", schema.name)
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "# Description: {desc}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output, "\ndefine\n").map_err(Self::fmt_error_to_generator_error)?;

        // Generate attributes first
        self.generate_attributes(&mut output, schema, indent)?;

        // Generate classes (entities and relations)
        if !schema.classes.is_empty() {
            for (class_name, class) in &schema.classes {
                let class_output = self.generate_class_typeql(class_name, class, schema, indent)?;
                output.push_str(&class_output);
            }
        }

        // Generate rules if requested
        if options.get_custom("generate_rules") == Some("true") {
            writeln!(&mut output, "# Rules").map_err(Self::fmt_error_to_generator_error)?;
            self.generate_rules(&mut output, schema, indent)?;
        }

        // Create output
        let filename = format!(
            "{}.typeql",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("schema_name".to_string(), schema.name.clone());

        Ok(vec![GeneratedOutput {
            content: output,
            filename,
            metadata,
        }])
    }
}

// Implement the synchronous Generator trait for backward compatibility
impl Generator for TypeQLGenerator {
    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Use tokio to run the async version
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {}", e)))?;

        let options = GeneratorOptions::new();
        let outputs = runtime
            .block_on(AsyncGenerator::generate(self, schema, &options))
            .map_err(|e| LinkMLError::service(e.to_string()))?;

        // Concatenate all outputs into a single string
        Ok(outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn get_file_extension(&self) -> &str {
        "tql"
    }

    fn get_default_filename(&self) -> &str {
        "generated.tql"
    }
}

impl CodeFormatter for TypeQLGenerator {
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
        // TypeQL doesn't require special escaping for identifiers
        s.to_string()
    }

    fn convert_identifier(&self, id: &str) -> String {
        // Convert to TypeQL naming conventions (lowercase with hyphens)
        id.replace('_', "-").to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_typeql_generation() {
        let generator = TypeQLGenerator::new();

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
        let outputs = AsyncGenerator::generate(&generator, &schema, &options)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to generate TypeQL: {}", e))?;

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.contains("person sub entity"));
        assert!(outputs[0].content.contains("owns name"));
        assert!(
            outputs[0]
                .content
                .contains("name sub attribute, value string")
        );
    }

    #[test]
    fn test_identifier_conversion() {
        let generator = TypeQLGenerator::new();

        assert_eq!(generator.convert_identifier("PersonName"), "personname");
        assert_eq!(generator.convert_identifier("person_name"), "person-name");
        assert_eq!(generator.convert_identifier("PERSON_NAME"), "person-name");
    }
}

/// Create a new TypeQL generator using the factory pattern
///
/// This is the preferred way to create a TypeQL generator, ensuring proper
/// initialization and following RootReal's factory pattern standards.
///
/// # Returns
///
/// Returns a configured TypeQL generator instance
pub fn create_typeql_generator() -> TypeQLGenerator {
    TypeQLGenerator::new()
}
