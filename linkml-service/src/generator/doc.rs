//! Documentation generation for `LinkML` schemas

use super::options::IndentStyle;
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::fmt::Write;

/// Documentation generator for `LinkML` schemas
pub struct DocGenerator {
    /// Generator name
    name: String,
}

impl DocGenerator {
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    /// Create a new documentation generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "doc".to_string(),
        }
    }

    /// Generate markdown documentation
    fn generate_markdown(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Title
        if schema.name.is_empty() {
            writeln!(&mut output, "# Schema Documentation").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, "# {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Metadata
        writeln!(&mut output, "## Metadata").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        if let Some(version) = &schema.version {
            writeln!(&mut output, "- **Version**: {version}").map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(license) = &schema.license {
            writeln!(&mut output, "- **License**: {license}").map_err(Self::fmt_error_to_generator_error)?;
        }
        if !schema.imports.is_empty() {
            writeln!(&mut output, "- **Imports**: {}", schema.imports.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Table of Contents
        writeln!(&mut output, "## Table of Contents").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "- [Classes](#classes)").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "- [Slots](#slots)").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "- [Types](#types)").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "- [Enums](#enums)").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Classes
        if !schema.classes.is_empty() {
            writeln!(&mut output, "## Classes").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

            for (class_name, class) in &schema.classes {
                self.generate_class_doc(&mut output, class_name, class, schema)?;
            }
        }

        // Slots
        if !schema.slots.is_empty() {
            writeln!(&mut output, "## Slots").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

            for (slot_name, slot) in &schema.slots {
                self.generate_slot_doc(&mut output, slot_name, slot)?;
            }
        }

        // Types
        if !schema.types.is_empty() {
            writeln!(&mut output, "## Types").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

            for (type_name, type_def) in &schema.types {
                self.generate_type_doc(&mut output, type_name, type_def)?;
            }
        }

        // Enums
        if !schema.enums.is_empty() {
            writeln!(&mut output, "## Enums").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

            for (enum_name, enum_def) in &schema.enums {
                self.generate_enum_doc(&mut output, enum_name, enum_def)?;
            }
        }

        Ok(output)
    }

    /// Generate documentation for a class
    fn generate_class_doc(
        &self,
        output: &mut String,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        let _ = self;
        writeln!(output, "### {class_name}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &class.description {
            writeln!(output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Properties table
        writeln!(output, "**Properties:**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(is_a) = &class.is_a {
            writeln!(output, "- **Inherits from**: {is_a}").map_err(Self::fmt_error_to_generator_error)?;
        }

        if !class.mixins.is_empty() {
            writeln!(output, "- **Mixins**: {}", class.mixins.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
        }

        if class.abstract_ == Some(true) {
            writeln!(output, "- **Abstract**: Yes").map_err(Self::fmt_error_to_generator_error)?;
        }

        if class.tree_root == Some(true) {
            writeln!(output, "- **Tree Root**: Yes").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Slots table
        if !class.slots.is_empty() {
            writeln!(output, "**Slots:**").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "| Slot | Type | Required | Description |").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "|------|------|----------|-------------|").map_err(Self::fmt_error_to_generator_error)?;

            for slot_name in &class.slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let slot_type = slot.range.as_deref().unwrap_or("string");
                    let required = if slot.required == Some(true) {
                        "Yes"
                    } else {
                        "No"
                    };
                    let desc = slot.description.as_deref().unwrap_or("");
                    writeln!(
                        output,
                        "| {slot_name} | {slot_type} | {required} | {desc} |"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        Ok(())
    }

    /// Generate documentation for a slot
    fn generate_slot_doc(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let _ = self;
        writeln!(output, "### {slot_name}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &slot.description {
            writeln!(output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "**Properties:**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(range) = &slot.range {
            writeln!(output, "- **Type**: {range}").map_err(Self::fmt_error_to_generator_error)?;
        }

        if slot.required == Some(true) {
            writeln!(output, "- **Required**: Yes").map_err(Self::fmt_error_to_generator_error)?;
        }

        if slot.multivalued == Some(true) {
            writeln!(output, "- **Multivalued**: Yes").map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(pattern) = &slot.pattern {
            writeln!(output, "- **Pattern**: `{pattern}`").map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(min_val) = &slot.minimum_value {
            writeln!(output, "- **Minimum**: {min_val}").map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(max_val) = &slot.maximum_value {
            writeln!(output, "- **Maximum**: {max_val}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }

    /// Generate documentation for a type
    fn generate_type_doc(
        &self,
        output: &mut String,
        type_name: &str,
        type_def: &TypeDefinition,
    ) -> GeneratorResult<()> {
        let _ = self;
        writeln!(output, "### {type_name}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &type_def.description {
            writeln!(output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(base) = &type_def.base_type {
            writeln!(output, "- **Base Type**: {base}").map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(pattern) = &type_def.pattern {
            writeln!(output, "- **Pattern**: `{pattern}`").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }

    /// Generate documentation for an enum
    fn generate_enum_doc(
        &self,
        output: &mut String,
        enum_name: &str,
        enum_def: &EnumDefinition,
    ) -> GeneratorResult<()> {
        let _ = self;
        writeln!(output, "### {enum_name}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &enum_def.description {
            writeln!(output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "**Values:**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        if !enum_def.permissible_values.is_empty() {
            for value_def in &enum_def.permissible_values {
                match value_def {
                    PermissibleValue::Simple(text) => {
                        writeln!(output, "- **{text}**").map_err(Self::fmt_error_to_generator_error)?;
                    }
                    PermissibleValue::Complex {
                        text, description, ..
                    } => {
                        write!(output, "- **{text}**").map_err(Self::fmt_error_to_generator_error)?;
                        if let Some(desc) = description {
                            write!(output, ": {desc}").map_err(Self::fmt_error_to_generator_error)?;
                        }
                        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
}

impl Default for DocGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for DocGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate documentation from LinkML schemas in various formats"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".md", ".html", ".rst"]
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Validate schema
        self.validate_schema(schema)?;

        // For now, always generate markdown
        self.generate_markdown(schema)
            .map_err(|e| LinkMLError::service(format!("Documentation generation error: {}", e)))
    }

    fn get_file_extension(&self) -> &str {
        // For now, always return markdown
        "md"
    }

    fn get_default_filename(&self) -> &str {
        "schema_documentation"
    }
}

impl CodeFormatter for DocGenerator {
    fn format_doc(&self, doc: &str, _indent: &IndentStyle, _level: usize) -> String {
        doc.to_string()
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        _indent: &IndentStyle,
        _level: usize,
        separator: &str,
    ) -> String {
        items
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        // Markdown escaping
        s.replace('\\', "\\\\")
            .replace('*', "\\*")
            .replace('_', "\\_")
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace('`', "\\`")
    }

    fn convert_identifier(&self, id: &str) -> String {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_generation() {
        let generator = DocGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "test".to_string();
        schema.name = "test_schema".to_string();
        schema.description = Some("A test schema".to_string());

        // Add a class
        let mut class = ClassDefinition::default();
        class.name = "Person".to_string();
        class.description = Some("A person entity".to_string());

        schema.classes.insert("Person".to_string(), class);

        let output = generator.generate(&schema)?;

        assert!(output.contains("# test_schema"));
        assert!(output.contains("### Person"));
        assert!(output.contains("A person entity"));
    }
}
