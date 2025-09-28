//! Class/struct generation

use super::base::BaseCodeFormatter;
use super::core::RustGenerator;
use super::traits::{GeneratorOptions, GeneratorResult, IndentStyle};
use linkml_core::prelude::*;
use std::fmt::Write;

impl RustGenerator {
    /// Generate Rust code for a class
    pub(super) fn generate_class_rust(
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Check if we should generate traits for polymorphism
        let generate_traits = options
            .get_custom("generate_traits")
            .map(std::string::String::as_str)
            == Some("true");

        // Generate trait first if this class has children or is abstract
        if generate_traits
            && (class.abstract_.unwrap_or(false) || Self::has_subclasses(class_name, schema))
        {
            output.push_str(&Self::generate_trait_for_class(
                class_name, class, schema, options, indent,
            )?);
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Skip struct generation for abstract classes unless explicitly requested
        if class.abstract_.unwrap_or(false)
            && options
                .get_custom("skip_abstract_structs")
                .map(std::string::String::as_str)
                == Some("true")
        {
            return Ok(output);
        }

        let struct_name = BaseCodeFormatter::to_pascal_case(class_name);

        // Documentation
        if options.include_docs {
            writeln!(&mut output, "/// {class_name}")
                .map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &class.description {
                writeln!(&mut output, "/// {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Struct definition with derives
        writeln!(
            &mut output,
            "#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Add serde rename if class name differs from struct name
        if class_name != struct_name.to_lowercase() {
            writeln!(&mut output, "#[serde(rename = \"{class_name}\")]")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "pub struct {struct_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate fields
        Self::generate_fields(&mut output, class, schema, options, indent)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Always generate new() and validate() methods
        Self::generate_impl(&mut output, &struct_name, class, schema, options, indent)?;

        // Generate builder if requested
        if options
            .get_custom("generate_builder")
            .map(std::string::String::as_str)
            == Some("true")
        {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            Self::generate_builder(&mut output, &struct_name, class, schema, options, indent)?;
        }

        // Generate trait implementation if needed
        if generate_traits
            && (class.abstract_.unwrap_or(false)
                || Self::has_subclasses(class_name, schema)
                || class.is_a.is_some())
        {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            Self::generate_trait_impl(
                &mut output,
                &struct_name,
                class_name,
                class,
                schema,
                options,
                indent,
            )?;
        }

        // Generate enum for polymorphic types if this is a parent class
        if generate_traits && Self::has_subclasses(class_name, schema) {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            Self::generate_polymorphic_enum(&mut output, class_name, schema, options)?;
        }

        Ok(output)
    }

    /// Generate implementation block for a struct
    pub(super) fn generate_impl(
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(output, "impl {struct_name} {{").map_err(Self::fmt_error_to_generator_error)?;

        // Generate new() method
        Self::generate_new_method(output, struct_name, class, schema, options, indent)?;

        // Generate validate() method
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Self::generate_validate_method(output, struct_name, class, schema, options, indent)?;

        // Generate additional utility methods
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Self::generate_utility_methods(output, struct_name, class, schema, options, indent)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate `new()` constructor method
    fn generate_new_method(
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        if options.include_docs {
            writeln!(
                output,
                "{}/// Create a new instance of {}",
                indent.single(),
                struct_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "{}pub fn new() -> Self {{", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "{}Self {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;

        // Initialize fields with default values
        let all_slots = super::base::collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = Self::convert_field_name(slot_name);
                let default_value = Self::get_default_value(slot, schema);

                writeln!(
                    output,
                    "{}{}: {},",
                    indent.to_string(3),
                    field_name,
                    default_value
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate utility methods
    fn generate_utility_methods(
        output: &mut String,
        _struct_name: &str,
        _class: &ClassDefinition,
        _schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Generate from_json method
        if options.include_docs {
            writeln!(output, "{}/// Create from `JSON` string", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(
            output,
            "{}pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {{",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}serde_json::from_str(json)", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate to_json method
        if options.include_docs {
            writeln!(output, "{}/// Convert to `JSON` string", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(
            output,
            "{}pub fn to_json(&self) -> Result<String, serde_json::Error> {{",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}serde_json::to_string(self)", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }
}
