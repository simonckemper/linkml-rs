//! Documentation generation for `LinkML` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// Documentation generator for `LinkML` schemas
pub struct DocGenerator {
    /// Generator name
    name: String,
}

impl DocGenerator {
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
            writeln!(&mut output, "# Schema Documentation").unwrap();
        } else {
            writeln!(&mut output, "# {}", schema.name).unwrap();
        }
        writeln!(&mut output).unwrap();

        // Description
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "{desc}").unwrap();
            writeln!(&mut output).unwrap();
        }

        // Metadata
        writeln!(&mut output, "## Metadata").unwrap();
        writeln!(&mut output).unwrap();
        if let Some(version) = &schema.version {
            writeln!(&mut output, "- **Version**: {version}").unwrap();
        }
        if let Some(license) = &schema.license {
            writeln!(&mut output, "- **License**: {license}").unwrap();
        }
        if !schema.imports.is_empty() {
            writeln!(&mut output, "- **Imports**: {}", schema.imports.join(", ")).unwrap();
        }
        writeln!(&mut output).unwrap();

        // Table of Contents
        writeln!(&mut output, "## Table of Contents").unwrap();
        writeln!(&mut output).unwrap();
        writeln!(&mut output, "- [Classes](#classes)").unwrap();
        writeln!(&mut output, "- [Slots](#slots)").unwrap();
        writeln!(&mut output, "- [Types](#types)").unwrap();
        writeln!(&mut output, "- [Enums](#enums)").unwrap();
        writeln!(&mut output).unwrap();

        // Classes
        if !schema.classes.is_empty() {
            writeln!(&mut output, "## Classes").unwrap();
            writeln!(&mut output).unwrap();

            for (class_name, class) in &schema.classes {
                self.generate_class_doc(&mut output, class_name, class, schema)?;
            }
        }

        // Slots
        if !schema.slots.is_empty() {
            writeln!(&mut output, "## Slots").unwrap();
            writeln!(&mut output).unwrap();

            for (slot_name, slot) in &schema.slots {
                self.generate_slot_doc(&mut output, slot_name, slot)?;
            }
        }

        // Types
        if !schema.types.is_empty() {
            writeln!(&mut output, "## Types").unwrap();
            writeln!(&mut output).unwrap();

            for (type_name, type_def) in &schema.types {
                self.generate_type_doc(&mut output, type_name, type_def)?;
            }
        }

        // Enums
        if !schema.enums.is_empty() {
            writeln!(&mut output, "## Enums").unwrap();
            writeln!(&mut output).unwrap();

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
        writeln!(output, "### {class_name}").unwrap();
        writeln!(output).unwrap();

        if let Some(desc) = &class.description {
            writeln!(output, "{desc}").unwrap();
            writeln!(output).unwrap();
        }

        // Properties table
        writeln!(output, "**Properties:**").unwrap();
        writeln!(output).unwrap();

        if let Some(is_a) = &class.is_a {
            writeln!(output, "- **Inherits from**: {is_a}").unwrap();
        }

        if !class.mixins.is_empty() {
            writeln!(output, "- **Mixins**: {}", class.mixins.join(", ")).unwrap();
        }

        if class.abstract_ == Some(true) {
            writeln!(output, "- **Abstract**: Yes").unwrap();
        }

        if class.tree_root == Some(true) {
            writeln!(output, "- **Tree Root**: Yes").unwrap();
        }

        writeln!(output).unwrap();

        // Slots table
        if !class.slots.is_empty() {
            writeln!(output, "**Slots:**").unwrap();
            writeln!(output).unwrap();
            writeln!(output, "| Slot | Type | Required | Description |").unwrap();
            writeln!(output, "|------|------|----------|-------------|").unwrap();

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
                    .unwrap();
                }
            }
            writeln!(output).unwrap();
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
        writeln!(output, "### {slot_name}").unwrap();
        writeln!(output).unwrap();

        if let Some(desc) = &slot.description {
            writeln!(output, "{desc}").unwrap();
            writeln!(output).unwrap();
        }

        writeln!(output, "**Properties:**").unwrap();
        writeln!(output).unwrap();

        if let Some(range) = &slot.range {
            writeln!(output, "- **Type**: {range}").unwrap();
        }

        if slot.required == Some(true) {
            writeln!(output, "- **Required**: Yes").unwrap();
        }

        if slot.multivalued == Some(true) {
            writeln!(output, "- **Multivalued**: Yes").unwrap();
        }

        if let Some(pattern) = &slot.pattern {
            writeln!(output, "- **Pattern**: `{pattern}`").unwrap();
        }

        if let Some(min_val) = &slot.minimum_value {
            writeln!(output, "- **Minimum**: {min_val}").unwrap();
        }

        if let Some(max_val) = &slot.maximum_value {
            writeln!(output, "- **Maximum**: {max_val}").unwrap();
        }

        writeln!(output).unwrap();
        
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
        writeln!(output, "### {type_name}").unwrap();
        writeln!(output).unwrap();

        if let Some(desc) = &type_def.description {
            writeln!(output, "{desc}").unwrap();
            writeln!(output).unwrap();
        }

        if let Some(base) = &type_def.base_type {
            writeln!(output, "- **Base Type**: {base}").unwrap();
        }

        if let Some(pattern) = &type_def.pattern {
            writeln!(output, "- **Pattern**: `{pattern}`").unwrap();
        }

        writeln!(output).unwrap();
        
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
        writeln!(output, "### {enum_name}").unwrap();
        writeln!(output).unwrap();

        if let Some(desc) = &enum_def.description {
            writeln!(output, "{desc}").unwrap();
            writeln!(output).unwrap();
        }

        writeln!(output, "**Values:**").unwrap();
        writeln!(output).unwrap();

        if !enum_def.permissible_values.is_empty() {
            for value_def in &enum_def.permissible_values {
                match value_def {
                    PermissibleValue::Simple(text) => {
                        writeln!(output, "- **{text}**").unwrap();
                    }
                    PermissibleValue::Complex {
                        text, description, ..
                    } => {
                        write!(output, "- **{text}**").unwrap();
                        if let Some(desc) = description {
                            write!(output, ": {desc}").unwrap();
                        }
                        writeln!(output).unwrap();
                    }
                }
            }
        }

        writeln!(output).unwrap();
        
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

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        self.validate_schema(schema).await?;

        let mut outputs = Vec::new();

        // Generate based on format
        let format = options.get_custom("format").unwrap_or("markdown");

        match format {
            "markdown" | "md" => {
                let content = self.generate_markdown(schema)?;
                let filename = format!(
                    "{}.md",
                    if schema.name.is_empty() {
                        "schema"
                    } else {
                        &schema.name
                    }
                );

                let mut metadata = HashMap::new();
                metadata.insert("generator".to_string(), self.name.clone());
                metadata.insert("format".to_string(), "markdown".to_string());

                outputs.push(GeneratedOutput {
                    content,
                    filename,
                    metadata,
                });
            }
            _ => {
                return Err(GeneratorError::UnsupportedFeature(format!(
                    "Unsupported documentation format: {format}"
                )));
            }
        }

        Ok(outputs)
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

    #[tokio::test]
    async fn test_doc_generation() {
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

        let options = GeneratorOptions::new();
        let outputs = generator.generate(&schema, &options).await.unwrap();

        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].content.contains("# test_schema"));
        assert!(outputs[0].content.contains("### Person"));
        assert!(outputs[0].content.contains("A person entity"));
    }
}
