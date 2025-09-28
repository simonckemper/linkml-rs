//! Rust trait generation for polymorphic types

use super::base::BaseCodeFormatter;
use super::core::RustGenerator;
use super::traits::{GeneratorOptions, GeneratorResult, IndentStyle};
use linkml_core::prelude::*;
use std::fmt::Write;

impl RustGenerator {
    /// Generate trait for a class (Kapernikov-style polymorphism)
    pub(super) fn generate_trait_for_class(
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        _indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let trait_name = format!("{}Trait", BaseCodeFormatter::to_pascal_case(class_name));

        // Documentation
        if options.include_docs {
            writeln!(&mut output, "/// Trait for {class_name} and its subclasses")
                .map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &class.description {
                writeln!(&mut output, "/// {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(
            &mut output,
            "pub trait {trait_name}: std::fmt::Debug + Send + Sync {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Add common methods that all implementations should have
        writeln!(&mut output, "    /// Get the type name of this instance")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    fn type_name(&self) -> &'static str;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "    /// Validate this instance")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    fn validate(&self) -> Result<(), ValidationError>;"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Add serialization support
        writeln!(&mut output, "    /// Serialize to `JSON`")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    fn to_json(&self) -> Result<String, serde_json::Error>;"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Add getter methods for key slots from schema
        let all_slots = super::base::collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name)
                && (slot.identifier == Some(true) || slot.required == Some(true))
            {
                let field_name = BaseCodeFormatter::to_snake_case(slot_name);
                let return_type = Self::get_rust_type(slot, schema);
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    /// Get the {field_name} field")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    fn {field_name}(&self) -> &{return_type};")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate trait implementation for a struct
    #[allow(clippy::too_many_arguments)]
    pub(super) fn generate_trait_impl(
        output: &mut String,
        struct_name: &str,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
        _indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Find the trait to implement (could be from parent class)
        let trait_class =
            if class.abstract_.unwrap_or(false) || Self::has_subclasses(class_name, schema) {
                class_name
            } else if let Some(parent) = &class.is_a {
                parent
            } else {
                return Ok(());
            };

        let trait_name = format!("{}Trait", BaseCodeFormatter::to_pascal_case(trait_class));

        writeln!(output, "impl {trait_name} for {struct_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Implement type_name
        writeln!(output, "    fn type_name(&self) -> &'static str {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        \"{struct_name}\"")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Implement validate
        writeln!(
            output,
            "    fn validate(&self) -> Result<(), ValidationError> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        self.validate()").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Implement to_json
        writeln!(
            output,
            "    fn to_json(&self) -> Result<String, serde_json::Error> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        serde_json::to_string(self)")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate polymorphic enum for a parent class
    pub(super) fn generate_polymorphic_enum(
        output: &mut String,
        class_name: &str,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        let enum_name = format!("{}Variants", BaseCodeFormatter::to_pascal_case(class_name));
        let subclasses = Self::get_subclasses(class_name, schema);

        if subclasses.is_empty() {
            return Ok(());
        }

        // Documentation
        if options.include_docs {
            writeln!(
                output,
                "/// Polymorphic enum for {class_name} and its subclasses"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "#[derive(Debug, Clone, Serialize, Deserialize)]")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "#[serde(tag = \"type\")]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "pub enum {enum_name} {{").map_err(Self::fmt_error_to_generator_error)?;

        // Add base class variant
        let base_struct_name = BaseCodeFormatter::to_pascal_case(class_name);
        writeln!(output, "    {base_struct_name}({base_struct_name}),")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Add subclass variants
        for subclass in &subclasses {
            let variant_name = BaseCodeFormatter::to_pascal_case(subclass);
            writeln!(output, "    {variant_name}({variant_name}),")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate implementation for the enum
        Self::generate_enum_impl(output, &enum_name, class_name, &subclasses)?;

        Ok(())
    }

    /// Generate implementation for polymorphic enum
    fn generate_enum_impl(
        output: &mut String,
        enum_name: &str,
        class_name: &str,
        subclasses: &[String],
    ) -> GeneratorResult<()> {
        let trait_name = format!("{}Trait", BaseCodeFormatter::to_pascal_case(class_name));

        writeln!(output, "impl {trait_name} for {enum_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Implement type_name
        writeln!(output, "    fn type_name(&self) -> &'static str {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        match self {{").map_err(Self::fmt_error_to_generator_error)?;

        let base_variant = BaseCodeFormatter::to_pascal_case(class_name);
        writeln!(
            output,
            "            {enum_name}::{base_variant}(inner) => inner.type_name(),"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        for subclass in subclasses {
            let variant_name = BaseCodeFormatter::to_pascal_case(subclass);
            writeln!(
                output,
                "            {enum_name}::{variant_name}(inner) => inner.type_name(),"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Implement validate
        writeln!(
            output,
            "    fn validate(&self) -> Result<(), ValidationError> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        match self {{").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(
            output,
            "            {enum_name}::{base_variant}(inner) => inner.validate(),"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        for subclass in subclasses {
            let variant_name = BaseCodeFormatter::to_pascal_case(subclass);
            writeln!(
                output,
                "            {enum_name}::{variant_name}(inner) => inner.validate(),"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Implement to_json
        writeln!(
            output,
            "    fn to_json(&self) -> Result<String, serde_json::Error> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        serde_json::to_string(self)")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }
}
