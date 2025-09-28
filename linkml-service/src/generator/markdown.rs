//! Markdown documentation generator for `LinkML` schemas
//!
//! This generator creates comprehensive Markdown documentation from `LinkML` schemas,
//! including class hierarchies, slot tables, enumerations, and cross-references.

use super::traits::Generator;
use linkml_core::{error::LinkMLError, prelude::*};
use std::collections::BTreeMap;
use std::fmt::Write;

/// Markdown documentation generator
pub struct MarkdownGenerator {
    /// Whether to include table of contents
    include_toc: bool,
    /// Whether to include inheritance diagrams
    include_diagrams: bool,
    /// Whether to include examples
    include_examples: bool,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl MarkdownGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> super::traits::GeneratorError {
        super::traits::GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new Markdown generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_toc: true,
            include_diagrams: true,
            include_examples: true,
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

    /// Configure table of contents generation
    #[must_use]
    pub fn with_toc(mut self, enabled: bool) -> Self {
        self.include_toc = enabled;
        self
    }

    /// Configure diagram generation
    #[must_use]
    pub fn with_diagrams(mut self, enabled: bool) -> Self {
        self.include_diagrams = enabled;
        self
    }

    /// Configure example generation
    #[must_use]
    pub fn with_examples(mut self, enabled: bool) -> Self {
        self.include_examples = enabled;
        self
    }

    /// Generate the schema header
    fn generate_header(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "# {}",
            if schema.name.is_empty() {
                "LinkML Schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        if let Some(title) = &schema.title {
            writeln!(&mut output, "**Title**: {title}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(description) = &schema.description {
            writeln!(
                &mut output,
                "
**Description**: {description}"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(version) = &schema.version {
            writeln!(
                &mut output,
                "
**Version**: {version}"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "
---
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate table of contents
    fn generate_toc(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "## Table of Contents
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "1. [Overview](#overview)")
            .map_err(Self::fmt_error_to_generator_error)?;

        if !schema.classes.is_empty() {
            writeln!(&mut output, "2. [Classes](#classes)")
                .map_err(Self::fmt_error_to_generator_error)?;
            for class_name in schema.classes.keys() {
                let anchor = class_name.to_lowercase().replace(' ', "-");
                writeln!(&mut output, "   - [{class_name}](#{anchor})")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        if !schema.slots.is_empty() {
            writeln!(&mut output, "3. [Slots](#slots)")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if !schema.enums.is_empty() {
            writeln!(&mut output, "4. [Enumerations](#enumerations)")
                .map_err(Self::fmt_error_to_generator_error)?;
            for enum_name in schema.enums.keys() {
                let anchor = enum_name.to_lowercase().replace(' ', "-");
                writeln!(&mut output, "   - [{enum_name}](#{anchor})")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        if !schema.types.is_empty() {
            writeln!(&mut output, "5. [Types](#types)")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "
---
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate overview section
    fn generate_overview(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "## Overview
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Statistics
        writeln!(
            &mut output,
            "### Statistics
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Category | Count |")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "|----------|-------|")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Classes | {} |", schema.classes.len())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Slots | {} |", schema.slots.len())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Enums | {} |", schema.enums.len())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Types | {} |", schema.types.len())
            .map_err(Self::fmt_error_to_generator_error)?;

        // Prefixes
        if !schema.prefixes.is_empty() {
            writeln!(
                &mut output,
                "
### Prefixes
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "| Prefix | URI |")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "|--------|-----|")
                .map_err(Self::fmt_error_to_generator_error)?;
            for (prefix, uri_def) in &schema.prefixes {
                let uri = match uri_def {
                    linkml_core::types::PrefixDefinition::Simple(s) => s.as_str(),
                    linkml_core::types::PrefixDefinition::Complex { prefix_prefix, .. } => {
                        prefix_prefix.as_str()
                    }
                };
                writeln!(&mut output, "| {prefix} | {uri} |")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(
            &mut output,
            "
---
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate class documentation
    fn generate_classes(
        &self,
        schema: &SchemaDefinition,
    ) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        if schema.classes.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "## Classes
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Sort classes for consistent output
        let mut sorted_classes: Vec<_> = schema.classes.iter().collect();
        sorted_classes.sort_by_key(|(name, _)| name.as_str());

        for (class_name, class_def) in sorted_classes {
            writeln!(
                &mut output,
                "### {class_name}
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(description) = &class_def.description {
                writeln!(
                    &mut output,
                    "{description}
"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Metadata table
            writeln!(
                &mut output,
                "#### Metadata
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "| Property | Value |")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "|----------|-------|")
                .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(is_a) = &class_def.is_a {
                writeln!(
                    &mut output,
                    "| Parent Class | [{}](#{}) |",
                    is_a,
                    is_a.to_lowercase().replace(' ', "-")
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if let Some(abstract_) = class_def.abstract_ {
                writeln!(&mut output, "| Abstract | {abstract_} |")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            if let Some(mixin) = class_def.mixin {
                writeln!(&mut output, "| Mixin | {mixin} |")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            if !class_def.mixins.is_empty() {
                let mixins = class_def.mixins.join(", ");
                writeln!(&mut output, "| Uses Mixins | {mixins} |")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Slots table
            if !class_def.slots.is_empty() || !class_def.attributes.is_empty() {
                writeln!(
                    &mut output,
                    "
#### Slots
"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "| Slot | Required | Type | Description |")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "|------|----------|------|-------------|")
                    .map_err(Self::fmt_error_to_generator_error)?;

                // Collect all slots (direct and inherited)
                let mut all_slots: BTreeMap<String, SlotDefinition> = BTreeMap::new();

                // Direct slots
                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        all_slots.insert(slot_name.clone(), slot_def.clone());
                    }
                }

                // Attributes (inline slots)
                for (attr_name, attr_def) in &class_def.attributes {
                    all_slots.insert(attr_name.clone(), attr_def.clone());
                }

                // Slot usage overrides
                for (slot_name, slot_usage) in &class_def.slot_usage {
                    if let Some(base_slot) = schema.slots.get(slot_name) {
                        // Apply overrides to base slot
                        let mut overridden = base_slot.clone();
                        if slot_usage.required.is_some() {
                            overridden.required = slot_usage.required;
                        }
                        if slot_usage.range.is_some() {
                            overridden.range.clone_from(&slot_usage.range);
                        }
                        if slot_usage.description.is_some() {
                            overridden.description.clone_from(&slot_usage.description);
                        }
                        all_slots.insert(slot_name.clone(), overridden);
                    }
                }

                for (slot_name, slot_def) in &all_slots {
                    let required = slot_def.required.unwrap_or(false);
                    let range = slot_def.range.as_deref().unwrap_or("string");
                    let description = slot_def.description.as_deref().unwrap_or("");

                    writeln!(
                        &mut output,
                        "| {} | {} | {} | {} |",
                        slot_name,
                        if required { "✓" } else { "" },
                        range,
                        description
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Examples
            if self.include_examples {
                writeln!(
                    &mut output,
                    "
#### Example
"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "```yaml").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "{class_name}:")
                    .map_err(Self::fmt_error_to_generator_error)?;

                // Generate example values for each slot
                for slot_name in &class_def.slots {
                    if let Some(slot_def) = schema.slots.get(slot_name) {
                        let example_value = Self::generate_example_value(slot_def);
                        writeln!(&mut output, "  {slot_name}: {example_value}")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(
                    &mut output,
                    "```
"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(
                &mut output,
                "---
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate slot documentation
    fn generate_slots(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        if schema.slots.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "## Slots
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Slot | Type | Required | Description |")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "|------|------|----------|-------------|")
            .map_err(Self::fmt_error_to_generator_error)?;

        let mut sorted_slots: Vec<_> = schema.slots.iter().collect();
        sorted_slots.sort_by_key(|(name, _)| name.as_str());

        for (slot_name, slot_def) in sorted_slots {
            let range = slot_def.range.as_deref().unwrap_or("string");
            let required = slot_def.required.unwrap_or(false);
            let description = slot_def.description.as_deref().unwrap_or("");

            writeln!(
                &mut output,
                "| {} | {} | {} | {} |",
                slot_name,
                range,
                if required { "✓" } else { "" },
                description
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "
---
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate enumeration documentation
    fn generate_enums(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        if schema.enums.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "## Enumerations
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        let mut sorted_enums: Vec<_> = schema.enums.iter().collect();
        sorted_enums.sort_by_key(|(name, _)| name.as_str());

        for (enum_name, enum_def) in sorted_enums {
            writeln!(
                &mut output,
                "### {enum_name}
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(description) = &enum_def.description {
                writeln!(
                    &mut output,
                    "{description}
"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(
                &mut output,
                "#### Permissible Values
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "| Value | Description |")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "|-------|-------------|")
                .map_err(Self::fmt_error_to_generator_error)?;

            for pv in &enum_def.permissible_values {
                let (value, description) = match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => (s.as_str(), ""),
                    linkml_core::types::PermissibleValue::Complex {
                        text, description, ..
                    } => (text.as_str(), description.as_deref().unwrap_or("")),
                };
                writeln!(&mut output, "| {value} | {description} |")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(
                &mut output,
                "
---
"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate types documentation
    fn generate_types(schema: &SchemaDefinition) -> super::traits::GeneratorResult<String> {
        let mut output = String::new();

        if schema.types.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "## Types
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "| Type | Base | Pattern | Description |")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "|------|------|---------|-------------|")
            .map_err(Self::fmt_error_to_generator_error)?;

        let mut sorted_types: Vec<_> = schema.types.iter().collect();
        sorted_types.sort_by_key(|(name, _)| name.as_str());

        for (type_name, type_def) in sorted_types {
            let base = type_def.base_type.as_deref().unwrap_or("");
            let pattern = type_def.pattern.as_deref().unwrap_or("");
            let description = type_def.description.as_deref().unwrap_or("");

            let pattern_str = if pattern.is_empty() {
                String::new()
            } else {
                format!("`{pattern}`")
            };

            writeln!(
                &mut output,
                "| {type_name} | {base} | {pattern_str} | {description} |"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "
---
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate example value for a slot
    fn generate_example_value(slot: &SlotDefinition) -> &'static str {
        match slot.range.as_deref() {
            Some("string") => "\"example string\"",
            Some("integer") => "42",
            Some("float") => "3.14",
            Some("boolean") => "true",
            Some("date") => "2024-01-01",
            Some("datetime") => "2024-01-01T12:00:00Z",
            Some("uri") => "https://example.com",
            _ => "\"example value\"",
        }
    }
}

impl Default for MarkdownGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for MarkdownGenerator {
    fn name(&self) -> &'static str {
        "markdown"
    }

    fn description(&self) -> &'static str {
        "Generate Markdown documentation from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".md"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Markdown generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let mut content = String::new();

        // Generate sections
        content.push_str(&Self::generate_header(schema)?);

        if self.include_toc {
            content.push_str(&Self::generate_toc(schema)?);
        }

        content.push_str(&Self::generate_overview(schema)?);
        content.push_str(&self.generate_classes(schema)?);
        content.push_str(&Self::generate_slots(schema)?);
        content.push_str(&Self::generate_enums(schema)?);
        content.push_str(&Self::generate_types(schema)?);

        // Footer
        content.push_str(
            "
---

",
        );
        write!(
            content,
            "*Generated by LinkML Markdown Generator v{}*",
            env!("CARGO_PKG_VERSION")
        )
        .expect("Writing to string should never fail");

        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "md"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();
        schema.title = Some("Test Schema".to_string());
        schema.description = Some("A test schema for documentation".to_string());
        schema.version = Some("1.0.0".to_string());

        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person entity".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.description = Some("The person's name".to_string());
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut age_slot = SlotDefinition::default();
        age_slot.description = Some("The person's age".to_string());
        age_slot.range = Some("integer".to_string());
        schema.slots.insert("age".to_string(), age_slot);

        // Add an enum
        let mut status_enum = EnumDefinition::default();
        status_enum.description = Some("Employment status".to_string());

        status_enum
            .permissible_values
            .push(PermissibleValue::Complex {
                text: "ACTIVE".to_string(),
                description: Some("Currently employed".to_string()),
                meaning: None,
            });

        status_enum
            .permissible_values
            .push(PermissibleValue::Complex {
                text: "INACTIVE".to_string(),
                description: Some("Not currently employed".to_string()),
                meaning: None,
            });

        schema
            .enums
            .insert("EmploymentStatus".to_string(), status_enum);

        schema
    }

    #[test]
    fn test_markdown_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = MarkdownGenerator::new();

        let result = generator
            .generate(&schema)
            .expect("should generate markdown documentation: {}");

        // Check content includes expected sections
        assert!(result.contains("# TestSchema"));
        assert!(result.contains("## Table of Contents"));
        assert!(result.contains("## Overview"));
        assert!(result.contains("## Classes"));
        assert!(result.contains("### Person"));
        assert!(result.contains("## Enumerations"));
        assert!(result.contains("### EmploymentStatus"));
        Ok(())
    }

    #[test]
    fn test_example_generation() {
        let mut slot = SlotDefinition::default();

        slot.range = Some("string".to_string());
        assert_eq!(
            MarkdownGenerator::generate_example_value(&slot),
            "\"example string\""
        );

        slot.range = Some("integer".to_string());
        assert_eq!(MarkdownGenerator::generate_example_value(&slot), "42");

        slot.range = Some("boolean".to_string());
        assert_eq!(MarkdownGenerator::generate_example_value(&slot), "true");
    }
}
