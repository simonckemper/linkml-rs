//! `TypeQL` generation implementation for `TypeDB` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// `TypeQL` schema generator for `TypeDB`
pub struct TypeQLGenerator {
    /// Generator name
    name: String,
    /// Type mapping configuration
    type_mappings: HashMap<String, String>,
    /// Default type for unknown mappings
    default_type: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl TypeQLGenerator {
    /// Create a new `TypeQL` generator
    #[must_use]
    pub fn new() -> Self {
        let mut type_mappings = HashMap::new();
        // Initialize standard LinkML to TypeQL type mappings
        type_mappings.insert("string".to_string(), "string".to_string());
        type_mappings.insert("str".to_string(), "string".to_string());
        type_mappings.insert("uri".to_string(), "string".to_string());
        type_mappings.insert("url".to_string(), "string".to_string());
        type_mappings.insert("ncname".to_string(), "string".to_string());
        type_mappings.insert("curie".to_string(), "string".to_string());
        type_mappings.insert("integer".to_string(), "long".to_string());
        type_mappings.insert("int".to_string(), "long".to_string());
        type_mappings.insert("float".to_string(), "double".to_string());
        type_mappings.insert("double".to_string(), "double".to_string());
        type_mappings.insert("decimal".to_string(), "double".to_string());
        type_mappings.insert("boolean".to_string(), "boolean".to_string());
        type_mappings.insert("bool".to_string(), "boolean".to_string());
        type_mappings.insert("date".to_string(), "datetime".to_string());
        type_mappings.insert("datetime".to_string(), "datetime".to_string());
        type_mappings.insert("time".to_string(), "datetime".to_string());

        Self {
            name: "typeql".to_string(),
            type_mappings,
            default_type: "string".to_string(),
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(format!("Formatting error: {err}")))
    }

    /// Check if a string is a valid `TypeQL` identifier
    fn is_valid_typeql_identifier(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Must start with a letter
        let mut chars = name.chars();
        if let Some(first) = chars.next()
            && !first.is_ascii_alphabetic()
        {
            return false;
        }

        // Rest must be letters, numbers, or underscores
        for ch in chars {
            if !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-' {
                return false;
            }
        }

        // Check it's not a TypeQL reserved keyword
        !matches!(
            name,
            "sub"
                | "entity"
                | "relation"
                | "attribute"
                | "role"
                | "plays"
                | "owns"
                | "relates"
                | "as"
                | "abstract"
                | "rule"
                | "when"
                | "then"
                | "match"
                | "insert"
                | "delete"
                | "define"
                | "undefine"
                | "compute"
                | "get"
                | "aggregate"
                | "group"
                | "sort"
                | "limit"
                | "offset"
                | "contains"
                | "regex"
                | "key"
                | "unique"
                | "ordered"
                | "unordered"
        )
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
        let is_relation = Self::is_relation_class(class, schema);

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
        let roles = self.collect_relation_roles(class, schema);
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
        _indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let mut generated_attrs = HashSet::new();

        // Collect all slots that are used as attributes
        if !schema.classes.is_empty() {
            for class in schema.classes.values() {
                if !class.slots.is_empty() {
                    for slot_name in &class.slots {
                        if !generated_attrs.contains(slot_name)
                            && let Some(slot) = schema.slots.get(slot_name)
                        {
                            self.generate_attribute(output, slot_name, slot)?;
                            generated_attrs.insert(slot_name.clone());
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
        let value_type = self.map_range_to_typeql(slot.range.as_ref());

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
                        if let Some(slot) = schema.slots.get(slot_name)
                            && slot.required == Some(true)
                        {
                            self.generate_required_rule(output, class_name, slot_name, indent)?;
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
    fn is_relation_class(class: &ClassDefinition, schema: &SchemaDefinition) -> bool {
        // A class is a relation if it has slots that reference other classes
        if !class.slots.is_empty() {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name)
                    && let Some(range) = &slot.range
                {
                    // Check if range is a class reference
                    if schema.classes.contains_key(range) {
                        return true;
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
                    if let Some(range) = &slot.range
                        && !schema.classes.contains_key(range)
                    {
                        attributes.push(self.convert_identifier(slot_name));
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
    ) -> Vec<(String, Vec<String>)> {
        let mut roles = Vec::new();

        if !class.slots.is_empty() {
            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name)
                    && let Some(range) = &slot.range
                {
                    // Check if range is a class reference
                    if schema.classes.contains_key(range) {
                        let role_name = self.convert_identifier(slot_name);
                        let player_type = self.convert_identifier(range);
                        roles.push((role_name, vec![player_type]));
                    }
                }
            }
        }

        roles
    }

    /// Map `LinkML` range to `TypeQL` value type
    fn map_range_to_typeql(&self, range: Option<&String>) -> String {
        range
            .and_then(|r| self.type_mappings.get(r))
            .cloned()
            .unwrap_or_else(|| self.default_type.clone())
    }

    /// Add custom type mapping
    pub fn add_type_mapping(&mut self, linkml_type: String, typeql_type: String) {
        self.type_mappings.insert(linkml_type, typeql_type);
    }

    /// Set default type for unknown mappings
    pub fn set_default_type(&mut self, default_type: String) {
        self.default_type = default_type;
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

    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Validate schema has required fields for TypeQL generation
        if schema.name.is_empty() {
            return Err(GeneratorError::Validation(
                "Schema must have a name for TypeQL generation".to_string(),
            ));
        }

        // Validate classes if present
        for (class_name, class_def) in &schema.classes {
            // Check class name is valid TypeQL identifier
            if !Self::is_valid_typeql_identifier(class_name) {
                return Err(GeneratorError::Validation(format!(
                    "Class name '{class_name}' is not a valid TypeQL identifier. Must start with a letter and contain only letters, numbers, and underscores."
                )));
            }

            // Validate slots
            if !class_def.slots.is_empty() {
                let slots = &class_def.slots;
                for slot_name in slots {
                    if !Self::is_valid_typeql_identifier(slot_name) {
                        return Err(GeneratorError::Validation(format!(
                            "Slot name '{slot_name}' in class '{class_name}' is not a valid TypeQL identifier"
                        )));
                    }
                }
            }

            // Validate attributes
            if !class_def.attributes.is_empty() {
                let attributes = &class_def.attributes;
                for attr_name in attributes.keys() {
                    if !Self::is_valid_typeql_identifier(attr_name) {
                        return Err(GeneratorError::Validation(format!(
                            "Attribute name '{attr_name}' in class '{class_name}' is not a valid TypeQL identifier"
                        )));
                    }
                }
            }
        }

        // Validate slots definitions
        for (slot_name, _slot_def) in &schema.slots {
            if !Self::is_valid_typeql_identifier(slot_name) {
                return Err(GeneratorError::Validation(format!(
                    "Slot definition name '{slot_name}' is not a valid TypeQL identifier"
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
        writeln!(
            &mut output,
            "
define
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

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
        if options
            .get_custom("generate_rules")
            .map(std::string::String::as_str)
            == Some("true")
        {
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
    fn name(&self) -> &'static str {
        "typeql"
    }

    fn description(&self) -> &'static str {
        "Generate TypeQL schema definitions for TypeDB graph database from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Use tokio to run the async validation
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {e}")))?;

        runtime
            .block_on(AsyncGenerator::validate_schema(self, schema))
            .map_err(|e| LinkMLError::service(e.to_string()))
    }

    fn generate(&self, schema: &SchemaDefinition) -> Result<String> {
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
        "tql"
    }

    fn get_default_filename(&self) -> &'static str {
        "generated.tql"
    }
}

impl CodeFormatter for TypeQLGenerator {
    fn name(&self) -> &'static str {
        "typeql_formatter"
    }

    fn description(&self) -> &'static str {
        "Code formatter for TypeQL schema definitions with proper indentation and comment handling"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["tql", "typeql"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        let mut formatted = String::new();
        let mut indent_level = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Handle comments
            if trimmed.starts_with('#') {
                formatted.push_str(&"    ".repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed == "}" || trimmed.ends_with("};") {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add proper indentation
            formatted.push_str(&"    ".repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after define, when, then, or opening braces
            if trimmed == "define"
                || trimmed == "when {"
                || trimmed == "then {"
                || trimmed.ends_with(" {")
                || trimmed == "{"
            {
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
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[tokio::test]
    async fn test_typeql_generation() -> anyhow::Result<()> {
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
            .expect("Failed to generate TypeQL: {}");

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.contains("person sub entity"));
        assert!(outputs[0].content.contains("owns name"));
        assert!(
            outputs[0]
                .content
                .contains("name sub attribute, value string")
        );
        Ok(())
    }

    #[test]
    fn test_identifier_conversion() {
        let generator = TypeQLGenerator::new();

        assert_eq!(generator.convert_identifier("PersonName"), "personname");
        assert_eq!(generator.convert_identifier("person_name"), "person-name");
        assert_eq!(generator.convert_identifier("PERSON_NAME"), "person-name");
    }
}

/// Create a new `TypeQL` generator using the factory pattern
///
/// This is the preferred way to create a `TypeQL` generator, ensuring proper
/// initialization and following `RootReal`'s factory pattern standards.
///
/// # Returns
///
/// Returns a configured `TypeQL` generator instance
#[must_use]
pub fn create_typeql_generator() -> TypeQLGenerator {
    TypeQLGenerator::new()
}
