//! Builder pattern generation

use super::base::collect_all_slots;
use super::core::RustGenerator;
use super::traits::{GeneratorOptions, GeneratorResult, IndentStyle};
use linkml_core::prelude::*;
use std::fmt::Write;

impl RustGenerator {
    /// Generate builder pattern for a struct
    pub(super) fn generate_builder(
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let builder_name = format!("{struct_name}Builder");

        // Documentation
        if options.include_docs {
            writeln!(output, "/// Builder for {struct_name}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Builder struct definition
        writeln!(output, "#[derive(Debug, Default)]")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "pub struct {builder_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate builder fields (all optional)
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = Self::convert_field_name(slot_name);
                let field_type = Self::get_rust_type(slot, schema);

                writeln!(
                    output,
                    "{}pub {}: Option<{}>,",
                    indent.single(),
                    field_name,
                    field_type
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate builder implementation
        Self::generate_builder_impl(
            output,
            struct_name,
            &builder_name,
            class,
            schema,
            options,
            indent,
        )?;

        Ok(())
    }

    /// Generate builder implementation
    #[allow(clippy::too_many_arguments)]
    fn generate_builder_impl(
        output: &mut String,
        struct_name: &str,
        builder_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(output, "impl {builder_name} {{").map_err(Self::fmt_error_to_generator_error)?;

        // Generate new() method
        if options.include_docs {
            writeln!(output, "{}/// Create a new builder", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(output, "{}pub fn new() -> Self {{", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}Self::default()", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate setter methods
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                Self::generate_builder_setter(output, slot_name, slot, schema, options, indent)?;
            }
        }

        // Generate build() method
        Self::generate_build_method(output, struct_name, class, schema, options, indent)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate setter method for builder
    fn generate_builder_setter(
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let field_name = Self::convert_field_name(slot_name);
        let field_type = Self::get_rust_type(slot, schema);

        // Documentation
        if options.include_docs {
            writeln!(output, "{}/// Set {}", indent.single(), slot_name)
                .map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &slot.description {
                writeln!(output, "{}/// {}", indent.single(), desc)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Method signature
        writeln!(
            output,
            "{}pub fn {}(mut self, value: {}) -> Self {{",
            indent.single(),
            field_name,
            field_type
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            output,
            "{}self.{} = Some(value);",
            indent.to_string(2),
            field_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}self", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate `build()` method
    fn generate_build_method(
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        if options.include_docs {
            writeln!(output, "{}/// Build the final struct", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            output,
            "{}pub fn build(self) -> Result<{}, ValidationError> {{",
            indent.single(),
            struct_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            output,
            "{}let instance = {} {{",
            indent.to_string(2),
            struct_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate field assignments
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = Self::convert_field_name(slot_name);

                // Handle required fields validation
                if slot.required.unwrap_or(false) {
                    writeln!(
                        output,
                        "{}{}: self.{}.ok_or_else(|| ValidationError::RequiredField {{ field: \"{}\" }})?,",
                        indent.to_string(3),
                        field_name,
                        field_name,
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(
                        output,
                        "{}{}: self.{},",
                        indent.to_string(3),
                        field_name,
                        field_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output, "{}}};", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Validate the built instance
        writeln!(output, "{}instance.validate()?;", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}Ok(instance)", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }
}
