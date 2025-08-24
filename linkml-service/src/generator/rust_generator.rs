//! Rust code generation implementation for `LinkML` schemas
//!
//! This generator produces idiomatic Rust code from LinkML schemas following
//! RootReal's strict quality standards:
//! - Zero tolerance for unwrap() or panic!()
//! - Comprehensive error handling with Result<T, E>
//! - Type-safe validation
//! - Performance optimized with zero-cost abstractions

use super::base::{BaseCodeFormatter, collect_all_slots, is_optional_slot};
use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use async_trait::async_trait;
use linkml_core::error::LinkMLError;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// Rust code generator for `LinkML` schemas
pub struct RustGenerator {
    /// Generator name
    name: String,
    /// Generator description
    description: String,
}

impl RustGenerator {
    /// Create a new Rust generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "rust".to_string(),
            description: "Generate idiomatic Rust code from LinkML schemas with serde support, comprehensive validation, and zero-tolerance for unwrap()".to_string(),
        }
    }

    /// Check if a class has subclasses
    fn has_subclasses(&self, class_name: &str, schema: &SchemaDefinition) -> bool {
        schema
            .classes
            .values()
            .any(|c| c.is_a.as_deref() == Some(class_name))
    }

    /// Get all direct subclasses of a class
    fn get_subclasses(&self, class_name: &str, schema: &SchemaDefinition) -> Vec<String> {
        schema
            .classes
            .iter()
            .filter(|(_, c)| c.is_a.as_deref() == Some(class_name))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Generate trait for a class (Kapernikov-style polymorphism)
    fn generate_trait_for_class(
        &self,
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
            writeln!(
                &mut output,
                "/// Trait for {} and its subclasses",
                class_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &class.description {
                writeln!(&mut output, "/// {}", desc)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(
            &mut output,
            "pub trait {}: std::fmt::Debug + Send + Sync {{",
            trait_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate getter methods for all slots
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let rust_name = self.convert_field_name(slot_name);
                let return_type = self.get_rust_type(slot, schema)?;

                // Documentation for the method
                if options.include_docs {
                    if let Some(desc) = &slot.description {
                        writeln!(&mut output, "    /// Get {}", desc)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                // Generate getter method
                if return_type.starts_with("Option<") {
                    writeln!(
                        &mut output,
                        "    fn {}(&self) -> {};",
                        rust_name, return_type
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else if return_type.starts_with("Vec<") {
                    writeln!(
                        &mut output,
                        "    fn {}(&self) -> &{};",
                        rust_name, return_type
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else if return_type == "String" {
                    writeln!(&mut output, "    fn {}(&self) -> &str;", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(
                        &mut output,
                        "    fn {}(&self) -> {};",
                        rust_name, return_type
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Add as_any method for downcasting
        writeln!(&mut output, "\n    /// Get self as Any for downcasting")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    fn as_any(&self) -> &dyn std::any::Any;")
            .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate trait implementation for a struct
    fn generate_trait_impl(
        &self,
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
            if class.abstract_.unwrap_or(false) || self.has_subclasses(class_name, schema) {
                class_name
            } else if let Some(parent) = &class.is_a {
                parent
            } else {
                return Ok(());
            };

        let trait_name = format!("{}Trait", BaseCodeFormatter::to_pascal_case(trait_class));

        writeln!(output, "impl {} for {} {{", trait_name, struct_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Implement getter methods
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let rust_name = self.convert_field_name(slot_name);
                let return_type = self.get_rust_type(slot, schema)?;

                if return_type.starts_with("Option<") {
                    writeln!(output, "    fn {}(&self) -> {} {{", rust_name, return_type)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        self.{}.clone()", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else if return_type.starts_with("Vec<") {
                    writeln!(output, "    fn {}(&self) -> &{} {{", rust_name, return_type)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        &self.{}", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else if return_type == "String" {
                    writeln!(output, "    fn {}(&self) -> &str {{", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        &self.{}", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output, "    fn {}(&self) -> {} {{", rust_name, return_type)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        self.{}", rust_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Implement as_any
        writeln!(output, "\n    fn as_any(&self) -> &dyn std::any::Any {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        self").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate enum for polymorphic types (similar to Kapernikov ExtensionOrSubtype)
    fn generate_polymorphic_enum(
        &self,
        output: &mut String,
        class_name: &str,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        let enum_name = format!("{}OrSubtype", BaseCodeFormatter::to_pascal_case(class_name));
        let subclasses = self.get_subclasses(class_name, schema);

        if subclasses.is_empty() {
            return Ok(());
        }

        // Documentation
        if options.include_docs {
            writeln!(
                output,
                "/// Polymorphic enum for {} and its subtypes",
                class_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "#[derive(Debug, Clone)]").map_err(Self::fmt_error_to_generator_error)?;
        if options.get_custom("derive_serde") != Some("false") {
            writeln!(output, "#[derive(Serialize, Deserialize)]")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "#[serde(untagged)]").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "pub enum {} {{", enum_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Base variant
        writeln!(
            output,
            "    {}(Box<{}>),",
            BaseCodeFormatter::to_pascal_case(class_name),
            BaseCodeFormatter::to_pascal_case(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Subclass variants
        for subclass in &subclasses {
            writeln!(
                output,
                "    {}(Box<{}>),",
                BaseCodeFormatter::to_pascal_case(subclass),
                BaseCodeFormatter::to_pascal_case(subclass)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        // Implement the trait for the enum
        let trait_name = format!("{}Trait", BaseCodeFormatter::to_pascal_case(class_name));
        writeln!(output, "\nimpl {} for {} {{", trait_name, enum_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Get all slots for delegation
        if let Some(class) = schema.classes.get(class_name) {
            let all_slots = collect_all_slots(class, schema)?;

            for slot_name in &all_slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let rust_name = self.convert_field_name(slot_name);
                    let return_type = self.get_rust_type(slot, schema)?;

                    // Generate delegating method
                    if return_type.starts_with("Option<") {
                        writeln!(output, "    fn {}(&self) -> {} {{", rust_name, return_type)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else if return_type.starts_with("Vec<") {
                        writeln!(output, "    fn {}(&self) -> &{} {{", rust_name, return_type)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else if return_type == "String" {
                        writeln!(output, "    fn {}(&self) -> &str {{", rust_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(output, "    fn {}(&self) -> {} {{", rust_name, return_type)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    writeln!(output, "        match self {{")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            {}::{}(x) => x.{}(),",
                        enum_name,
                        BaseCodeFormatter::to_pascal_case(class_name),
                        rust_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    for subclass in &subclasses {
                        writeln!(
                            output,
                            "            {}::{}(x) => x.{}(),",
                            enum_name,
                            BaseCodeFormatter::to_pascal_case(subclass),
                            rust_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    writeln!(output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Implement as_any
            writeln!(output, "\n    fn as_any(&self) -> &dyn std::any::Any {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "        self").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Formatting error: {}", err),
        ))
    }

    /// Generate Rust code for a class
    fn generate_class_rust(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Check if we should generate traits for polymorphism
        let generate_traits = options.get_custom("generate_traits") == Some("true");

        // Generate trait first if this class has children or is abstract
        if generate_traits
            && (class.abstract_.unwrap_or(false) || self.has_subclasses(class_name, schema))
        {
            output.push_str(
                &self.generate_trait_for_class(class_name, class, schema, options, indent)?,
            );
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Documentation
        if options.include_docs {
            if let Some(desc) = &class.description {
                writeln!(&mut output, "/// {}", desc)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(&mut output, "///").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "/// Generated from LinkML class: {}",
                class_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Derive macros
        let derives = self.get_derives(class, options);
        writeln!(&mut output, "#[derive({})]", derives.join(", "))
            .map_err(Self::fmt_error_to_generator_error)?;

        // Serde attributes if needed
        if derives.contains(&"Serialize".to_string())
            || derives.contains(&"Deserialize".to_string())
        {
            writeln!(&mut output, "#[serde(rename_all = \"camelCase\")]")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Struct definition
        let struct_name = BaseCodeFormatter::to_pascal_case(class_name);
        writeln!(&mut output, "pub struct {} {{", struct_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate fields
        self.generate_fields(&mut output, class, schema, options, indent)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Always generate new() and validate() methods
        self.generate_impl(&mut output, &struct_name, class, schema, options, indent)?;

        // Generate builder if requested
        if options.get_custom("generate_builder") == Some("true") {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_builder(&mut output, &struct_name, class, schema, options, indent)?;
        }

        // Generate trait implementation if needed
        if generate_traits
            && (class.abstract_.unwrap_or(false)
                || self.has_subclasses(class_name, schema)
                || class.is_a.is_some())
        {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_trait_impl(
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
        if generate_traits && self.has_subclasses(class_name, schema) {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_polymorphic_enum(&mut output, class_name, schema, options)?;
        }

        Ok(output)
    }

    /// Generate struct fields
    fn generate_fields(
        &self,
        output: &mut String,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        // Collect all slots including inherited ones using the base module function
        let all_slots = collect_all_slots(class, schema)?;

        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Documentation
                if options.include_docs {
                    if let Some(desc) = &slot.description {
                        writeln!(output, "{}/// {}", indent.single(), desc)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                // Field attributes
                let mut attrs = Vec::new();

                // Serde rename if needed
                let rust_name = self.convert_field_name(slot_name);
                if &rust_name != slot_name {
                    attrs.push(format!("#[serde(rename = \"{}\")]", slot_name));
                }

                // Skip serializing if optional
                if !slot.required.unwrap_or(false) && !slot.multivalued.unwrap_or(false) {
                    attrs.push("#[serde(skip_serializing_if = \"Option::is_none\")]".to_string());
                } else if slot.multivalued.unwrap_or(false) {
                    attrs.push(
                        "#[serde(default, skip_serializing_if = \"Vec::is_empty\")]".to_string(),
                    );
                }

                // Write attributes
                for attr in &attrs {
                    writeln!(output, "{}{}", indent.single(), attr)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Field definition
                let field_type = self.get_rust_type(slot, schema)?;
                writeln!(
                    output,
                    "{}pub {}: {},",
                    indent.single(),
                    rust_name,
                    field_type
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(())
    }

    /// Generate impl block
    fn generate_impl(
        &self,
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(output, "impl {} {{", struct_name).map_err(Self::fmt_error_to_generator_error)?;

        // Constructor for required fields only
        let all_slots = collect_all_slots(class, schema)?;
        let required_slots: Vec<_> = all_slots
            .iter()
            .filter(|slot_name| {
                schema
                    .slots
                    .get(*slot_name)
                    .map(|s| s.required.unwrap_or(false))
                    .unwrap_or(false)
            })
            .collect();

        if !required_slots.is_empty() {
            writeln!(
                output,
                "{}/// Create a new {} with required fields",
                indent.single(),
                struct_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            write!(output, "{}pub fn new(", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;

            // Parameters
            for (i, slot_name) in required_slots.iter().enumerate() {
                if let Some(slot) = schema.slots.get(*slot_name) {
                    let field_name = self.convert_field_name(slot_name);
                    let inner_type = self.get_inner_type(slot, schema)?;
                    write!(output, "{}: impl Into<{}>", field_name, inner_type)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    if i < required_slots.len() - 1 {
                        write!(output, ", ").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
            writeln!(output, ") -> Self {{").map_err(Self::fmt_error_to_generator_error)?;

            writeln!(output, "{}Self {{", indent.to_string(2))
                .map_err(Self::fmt_error_to_generator_error)?;

            // Initialize all fields
            for slot_name in &all_slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let field_name = self.convert_field_name(slot_name);
                    if required_slots.contains(&slot_name) {
                        writeln!(
                            output,
                            "{}{}: {}.into(),",
                            indent.to_string(3),
                            field_name,
                            field_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        let default_value = self.get_default_value(slot, schema)?;
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
            }

            writeln!(output, "{}}}", indent.to_string(2))
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}}}", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        } else {
            // Default constructor if no required fields
            writeln!(
                output,
                "{}/// Create a new {} with default values",
                indent.single(),
                struct_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}pub fn new() -> Self {{", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}Self::default()", indent.to_string(2))
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "{}}}", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Always generate validation method
        self.generate_validation_method(output, struct_name, class, schema, indent)?;

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate builder pattern
    fn generate_builder(
        &self,
        output: &mut String,
        struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let builder_name = format!("{struct_name}Builder");

        // Builder struct
        writeln!(output, "/// Builder for {struct_name}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "#[derive(Default)]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "pub struct {builder_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        let slots = collect_all_slots(class, schema)?;
        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = self.convert_field_name(slot_name);
                let field_type = self.get_rust_type(slot, schema)?;
                writeln!(output, "{}{}: {},", indent.single(), field_name, field_type)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Builder implementation
        writeln!(output, "impl {builder_name} {{").map_err(Self::fmt_error_to_generator_error)?;

        // Builder methods for each field
        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = self.convert_field_name(slot_name);
                let _field_type = self.get_rust_type(slot, schema)?;
                let inner_type = self.get_inner_type(slot, schema)?;

                writeln!(output, "{}/// Set {}", indent.single(), field_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    output,
                    "{}pub fn {}(mut self, value: {}) -> Self {{",
                    indent.single(),
                    field_name,
                    inner_type
                )
                .map_err(Self::fmt_error_to_generator_error)?;

                if slot.required == Some(true) {
                    writeln!(
                        output,
                        "{}self.{} = value;",
                        indent.to_string(2),
                        field_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(
                        output,
                        "{}self.{} = Some(value);",
                        indent.to_string(2),
                        field_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                writeln!(output, "{}self", indent.to_string(2))
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(output, "{}}}", indent.single())
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Build method
        writeln!(output, "{}/// Build the {}", indent.single(), struct_name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}pub fn build(self) -> {} {{",
            indent.single(),
            struct_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}{} {{", indent.to_string(2), struct_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        for slot_name in &slots {
            if let Some(_slot) = schema.slots.get(slot_name) {
                let field_name = self.convert_field_name(slot_name);
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

        writeln!(output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate validation method
    fn generate_validation_method(
        &self,
        output: &mut String,
        _struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "{}/// Validate this instance against schema constraints",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}pub fn validate(&self) -> Result<(), Vec<ValidationError>> {{",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}let mut errors = Vec::new();",
            indent.to_string(2)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        let all_slots = collect_all_slots(class, schema)?;

        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let field_name = self.convert_field_name(slot_name);

                // Required field validation
                if slot.required.unwrap_or(false) && slot.range.as_deref() == Some("string") {
                    writeln!(
                        output,
                        "{}// Required field: {}",
                        indent.to_string(2),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "{}if self.{}.is_empty() {{",
                        indent.to_string(2),
                        field_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "{}errors.push(ValidationError::RequiredField {{ field: \"{}\" }});",
                        indent.to_string(3),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "{}}}", indent.to_string(2))
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Pattern validation
                if let Some(pattern) = &slot.pattern {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "{}// Pattern validation for {}",
                        indent.to_string(2),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;

                    if slot.multivalued.unwrap_or(false) {
                        writeln!(
                            output,
                            "{}for value in &self.{} {{",
                            indent.to_string(2),
                            field_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}if !PATTERN_{}.is_match(value) {{",
                            indent.to_string(3),
                            field_name.to_uppercase()
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}errors.push(ValidationError::PatternMismatch {{",
                            indent.to_string(4)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}field: \"{}\",", indent.to_string(5), slot_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}pattern: \"{}\",",
                            indent.to_string(5),
                            pattern.replace('"', "\\\"")
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}});", indent.to_string(4))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(3))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else if slot.required.unwrap_or(false) {
                        writeln!(
                            output,
                            "{}if !PATTERN_{}.is_match(&self.{}) {{",
                            indent.to_string(2),
                            field_name.to_uppercase(),
                            field_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}errors.push(ValidationError::PatternMismatch {{",
                            indent.to_string(3)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}field: \"{}\",", indent.to_string(4), slot_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}pattern: \"{}\",",
                            indent.to_string(4),
                            pattern.replace('"', "\\\"")
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}});", indent.to_string(3))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(
                            output,
                            "{}if let Some(ref value) = self.{} {{",
                            indent.to_string(2),
                            field_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}if !PATTERN_{}.is_match(value) {{",
                            indent.to_string(3),
                            field_name.to_uppercase()
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}errors.push(ValidationError::PatternMismatch {{",
                            indent.to_string(4)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}field: \"{}\",", indent.to_string(5), slot_name)
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}pattern: \"{}\",",
                            indent.to_string(5),
                            pattern.replace('"', "\\\"")
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}});", indent.to_string(4))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(3))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                // Range validation
                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "{}// Range validation for {}",
                        indent.to_string(2),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;

                    let check_var = if slot.required.unwrap_or(false) {
                        field_name.clone()
                    } else {
                        "value".to_string()
                    };

                    if !slot.required.unwrap_or(false) {
                        writeln!(
                            output,
                            "{}if let Some(value) = self.{} {{",
                            indent.to_string(2),
                            field_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    let indent_level = if slot.required.unwrap_or(false) { 2 } else { 3 };

                    if let Some(ref min) = slot.minimum_value {
                        writeln!(
                            output,
                            "{}if {} < {} {{",
                            indent.to_string(indent_level),
                            check_var,
                            min
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}errors.push(ValidationError::RangeViolation {{",
                            indent.to_string(indent_level + 1)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}field: \"{}\",",
                            indent.to_string(indent_level + 2),
                            slot_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}value: {}.to_string(),",
                            indent.to_string(indent_level + 2),
                            check_var
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}min: Some(\"{}\".to_string()),",
                            indent.to_string(indent_level + 2),
                            min
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}max: None,", indent.to_string(indent_level + 2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}});", indent.to_string(indent_level + 1))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(indent_level))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    if let Some(ref max) = slot.maximum_value {
                        writeln!(
                            output,
                            "{}if {} > {} {{",
                            indent.to_string(indent_level),
                            check_var,
                            max
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}errors.push(ValidationError::RangeViolation {{",
                            indent.to_string(indent_level + 1)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}field: \"{}\",",
                            indent.to_string(indent_level + 2),
                            slot_name
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}value: {}.to_string(),",
                            indent.to_string(indent_level + 2),
                            check_var
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}min: None,", indent.to_string(indent_level + 2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "{}max: Some(\"{}\".to_string()),",
                            indent.to_string(indent_level + 2),
                            max
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}});", indent.to_string(indent_level + 1))
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "{}}}", indent.to_string(indent_level))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    if !slot.required.unwrap_or(false) {
                        writeln!(output, "{}}}", indent.to_string(2))
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}if errors.is_empty() {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}Ok(())", indent.to_string(3))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}} else {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}Err(errors)", indent.to_string(3))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get derive macros for a class
    fn get_derives(&self, _class: &ClassDefinition, options: &GeneratorOptions) -> Vec<String> {
        let mut derives = vec![
            "Debug".to_string(),
            "Clone".to_string(),
            "Default".to_string(),
            "PartialEq".to_string(),
        ];

        // Always include serde unless explicitly disabled
        if options.get_custom("derive_serde") != Some("false") {
            derives.push("Serialize".to_string());
            derives.push("Deserialize".to_string());
        }

        if options.get_custom("derive_eq") == Some("true") {
            derives.push("Eq".to_string());
        }

        if options.get_custom("derive_hash") == Some("true") {
            derives.push("Hash".to_string());
        }

        derives
    }

    /// Get Rust type for a slot
    fn get_rust_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        // Check if this slot has permissible values (enum)
        let mut base_type = if !slot.permissible_values.is_empty() {
            BaseCodeFormatter::to_pascal_case(&slot.name)
        } else {
            self.get_base_type(&slot.range, schema)?
        };

        // Check if this is a self-referential/recursive type that needs Box
        if let Some(range) = &slot.range {
            if let Some(class) = schema.classes.get(range) {
                // Check if the class has recursion options or if it references itself
                let needs_box = class
                    .recursion_options
                    .as_ref()
                    .map(|opts| opts.use_box)
                    .unwrap_or_else(|| {
                        // Auto-detect if this class has self-referential slots
                        class.slots.iter().any(|s| {
                            schema
                                .slots
                                .get(s)
                                .and_then(|slot_def| slot_def.range.as_ref())
                                .map(|r| r == range)
                                .unwrap_or(false)
                        })
                    });

                if needs_box {
                    base_type = format!("Box<{}>", base_type);
                }
            }
        }

        // Handle multivalued - always use Vec<T>, never Option<Vec<T>>
        if slot.multivalued.unwrap_or(false) {
            Ok(format!("Vec<{}>", base_type))
        } else if is_optional_slot(slot) {
            Ok(format!("Option<{}>", base_type))
        } else {
            Ok(base_type)
        }
    }

    /// Get inner type without Option wrapper
    fn get_inner_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        // Check if this slot has permissible values (enum)
        let base_type = if !slot.permissible_values.is_empty() {
            BaseCodeFormatter::to_pascal_case(&slot.name)
        } else {
            self.get_base_type(&slot.range, schema)?
        };

        if slot.multivalued.unwrap_or(false) {
            Ok(format!("Vec<{}>", base_type))
        } else {
            Ok(base_type)
        }
    }

    /// Get base Rust type from `LinkML` range
    fn get_base_type(
        &self,
        range: &Option<String>,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        match range.as_deref() {
            Some("string" | "str" | "uri" | "url") => Ok("String".to_string()),
            Some("integer" | "int") => Ok("i64".to_string()),
            Some("float" | "double" | "decimal") => Ok("f64".to_string()),
            Some("boolean" | "bool") => Ok("bool".to_string()),
            Some("date") => Ok("chrono::NaiveDate".to_string()),
            Some("datetime") => Ok("chrono::DateTime<chrono::Utc>".to_string()),
            Some(other) => {
                // Check if it's a class reference
                if schema.classes.contains_key(other) {
                    Ok(self.convert_identifier(other))
                } else if schema.enums.contains_key(other) {
                    Ok(self.convert_identifier(other))
                } else {
                    Ok("String".to_string()) // Default fallback
                }
            }
            None => Ok("String".to_string()),
        }
    }

    /// Get default value for a field
    ///
    /// # Errors
    ///
    /// Returns an error if the default value cannot be generated.
    fn get_default_value(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        // Multivalued fields always use Vec::new() as default
        if slot.multivalued.unwrap_or(false) {
            Ok("Vec::new()".to_string())
        } else if slot.required.unwrap_or(false) {
            match self.get_base_type(&slot.range, schema)?.as_str() {
                "String" => Ok("String::new()".to_string()),
                "i64" => Ok("0".to_string()),
                "f64" => Ok("0.0".to_string()),
                "bool" => Ok("false".to_string()),
                _ => Ok("Default::default()".to_string()),
            }
        } else {
            Ok("None".to_string())
        }
    }

    /// Generate enum from slot with permissible values
    fn generate_enum_from_slot(
        &self,
        slot_name: &str,
        slot: &SlotDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);

        // Documentation
        if options.include_docs {
            if let Some(desc) = &slot.description {
                writeln!(&mut output, "/// {}", desc)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Derive macros
        writeln!(
            &mut output,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        if options.get_custom("derive_serde") != Some("false") {
            writeln!(&mut output, "#[derive(Serialize, Deserialize)]")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "#[serde(rename_all = \"lowercase\")]")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Enum definition
        writeln!(&mut output, "pub enum {} {{", enum_name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate variants and collect mappings for Display/FromStr
        let mut variant_mappings = Vec::new();

        for value in &slot.permissible_values {
            match value {
                PermissibleValue::Simple(text) => {
                    let variant_name = self.convert_enum_variant(text);
                    variant_mappings.push((variant_name.clone(), text.clone()));

                    if &variant_name != text {
                        writeln!(&mut output, "    #[serde(rename = \"{}\")]", text)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    writeln!(&mut output, "    {},", variant_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                PermissibleValue::Complex {
                    text, description, ..
                } => {
                    if options.include_docs {
                        if let Some(desc) = description {
                            writeln!(&mut output, "    /// {}", desc)
                                .map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }

                    let variant_name = self.convert_enum_variant(text);
                    variant_mappings.push((variant_name.clone(), text.clone()));

                    if &variant_name != text {
                        writeln!(&mut output, "    #[serde(rename = \"{}\")]", text)
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    writeln!(&mut output, "    {},", variant_name)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate Display implementation
        writeln!(&mut output, "impl std::fmt::Display for {} {{", enum_name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        match self {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        for (variant, text) in &variant_mappings {
            writeln!(
                &mut output,
                "            {}::{} => write!(f, \"{}\"),",
                enum_name, variant, text
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate FromStr implementation
        writeln!(&mut output, "impl std::str::FromStr for {} {{", enum_name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    type Err = String;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    fn from_str(s: &str) -> Result<Self, Self::Err> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        match s {{").map_err(Self::fmt_error_to_generator_error)?;

        for (variant, text) in &variant_mappings {
            writeln!(
                &mut output,
                "            \"{}\" => Ok({}::{}),",
                text, enum_name, variant
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "            _ => Err(format!(\"Unknown {}: {{}}\", s)),",
            slot_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsyncGenerator for RustGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        AsyncGenerator::validate_schema(self, schema).await?;

        let mut outputs = Vec::new();
        let indent = &options.indent;

        // Generate main module file
        let mut main_output = String::new();

        // File header
        writeln!(
            &mut main_output,
            "//! Generated from LinkML schema: {}",
            if schema.name.is_empty() {
                "unnamed"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(desc) = &schema.description {
            writeln!(&mut main_output, "//! {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut main_output, "//!").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut main_output, "//! Generated by LinkML Rust Generator")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;

        // Imports
        writeln!(&mut main_output, "use serde::{{Deserialize, Serialize}};")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Check if we need chrono imports
        let needs_chrono = schema
            .slots
            .values()
            .any(|slot| matches!(slot.range.as_deref(), Some("date" | "datetime" | "time")));

        if needs_chrono {
            writeln!(
                &mut main_output,
                "use chrono::{{DateTime, NaiveDate, NaiveTime, Utc}};"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Check if we need regex imports
        let needs_regex = schema.slots.values().any(|slot| slot.pattern.is_some());

        if needs_regex {
            writeln!(&mut main_output, "use once_cell::sync::Lazy;")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "use regex::Regex;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate validation error type first if needed
        let needs_validation = schema.slots.values().any(|slot| {
            slot.required.unwrap_or(false)
                || slot.pattern.is_some()
                || slot.minimum_value.is_some()
                || slot.maximum_value.is_some()
        });

        if needs_validation {
            writeln!(&mut main_output, "/// Validation errors for this schema")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "#[derive(Debug, thiserror::Error)]")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "pub enum ValidationError {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut main_output,
                "    #[error(\"Required field missing: {{field}}\")]"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut main_output,
                "    RequiredField {{ field: &'static str }},"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut main_output,
                "    #[error(\"Field '{{field}}' does not match pattern: {{pattern}}\")]"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "    PatternMismatch {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        field: &'static str,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        pattern: &'static str,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "    }},").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "    #[error(\"Field '{{field}}' value {{value}} is outside range [{{min:?}}, {{max:?}}]\")]").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "    RangeViolation {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        field: &'static str,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        value: String,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        min: Option<String>,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "        max: Option<String>,")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "    }},").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate pattern constants
        let slots_with_patterns: Vec<_> = schema
            .slots
            .iter()
            .filter(|(_, slot)| slot.pattern.is_some())
            .collect();

        if !slots_with_patterns.is_empty() {
            writeln!(&mut main_output, "// Validation patterns")
                .map_err(Self::fmt_error_to_generator_error)?;
            for (slot_name, slot) in &slots_with_patterns {
                if let Some(ref pattern) = slot.pattern {
                    let const_name =
                        format!("PATTERN_{}", slot_name.to_uppercase().replace('-', "_"));
                    writeln!(
                        &mut main_output,
                        "static {}: Lazy<Regex> = Lazy::new(|| {{",
                        const_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        &mut main_output,
                        "    Regex::new(r\"{}\").expect(\"Invalid regex\")",
                        pattern
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut main_output, "}});")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate enums from slots with permissible values
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                let enum_output = self.generate_enum_from_slot(slot_name, slot, options)?;
                main_output.push_str(&enum_output);
                writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Generate classes
        if !schema.classes.is_empty() {
            for (class_name, class) in &schema.classes {
                let class_output =
                    self.generate_class_rust(class_name, class, schema, options, indent)?;
                main_output.push_str(&class_output);
                writeln!(&mut main_output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Create output
        let filename = format!(
            "{}.rs",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
            .to_lowercase()
            .replace('-', "_")
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("schema_name".to_string(), schema.name.clone());

        outputs.push(GeneratedOutput {
            content: main_output,
            filename,
            metadata,
        });

        // Generate tests if requested
        if options.generate_tests {
            let test_output = self.generate_tests(schema, options)?;
            outputs.push(test_output);
        }

        Ok(outputs)
    }
}

impl CodeFormatter for RustGenerator {
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let prefix = indent.to_string(level);
        doc.lines()
            .map(|line| format!("{prefix}/// {line}"))
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
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    fn convert_identifier(&self, id: &str) -> String {
        // Convert to Rust PascalCase for types
        let mut result = String::new();
        let mut capitalize_next = true;

        for ch in id.chars() {
            if ch == '_' || ch == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_uppercase().next().unwrap_or(ch));
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }
}

// Implement the synchronous Generator trait for backward compatibility
impl Generator for RustGenerator {
    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Create options with trait generation enabled for abstract classes
        // Check if any class is abstract or has subclasses
        let needs_traits = schema.classes.values().any(|c| {
            c.abstract_.unwrap_or(false)
                || schema
                    .classes
                    .values()
                    .any(|other| other.is_a.as_deref() == Some(&c.name))
        });

        let options = if needs_traits {
            GeneratorOptions::new().set_custom("generate_traits", "true")
        } else {
            GeneratorOptions::new()
        };

        // Try to use existing runtime, or create new one if needed
        let outputs = match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                // We're already in an async context, use it
                handle
                    .block_on(async { AsyncGenerator::generate(self, schema, &options).await })
                    .map_err(|e| LinkMLError::service(e.to_string()))?
            }
            Err(_) => {
                // No runtime exists, create one
                let runtime = tokio::runtime::Runtime::new().map_err(|e| {
                    LinkMLError::service(format!("Failed to create runtime: {}", e))
                })?;

                runtime
                    .block_on(AsyncGenerator::generate(self, schema, &options))
                    .map_err(|e| LinkMLError::service(e.to_string()))?
            }
        };

        // Concatenate all outputs into a single string
        Ok(outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join("\n"))
    }

    fn get_file_extension(&self) -> &str {
        "rs"
    }

    fn get_default_filename(&self) -> &str {
        "generated.rs"
    }
}

impl RustGenerator {
    /// Convert field names to `snake_case`
    fn convert_field_name(&self, name: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;

        for (i, ch) in name.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_upper = ch.is_uppercase();
        }

        // Handle reserved keywords
        match result.as_str() {
            "type" => "type_".to_string(),
            "match" => "match_".to_string(),
            "self" => "self_".to_string(),
            "super" => "super_".to_string(),
            "mod" => "mod_".to_string(),
            "use" => "use_".to_string(),
            _ => result,
        }
    }

    /// Convert enum variant names
    fn convert_enum_variant(&self, name: &str) -> String {
        // Convert to PascalCase and handle special characters
        let mut result = String::new();
        let mut capitalize_next = true;
        let mut prev_was_upper = false;

        for ch in name.chars() {
            if ch.is_alphanumeric() {
                if ch.is_uppercase() && !prev_was_upper && !result.is_empty() {
                    // Handle transitions like NOT_STARTED -> Not_Started
                    capitalize_next = true;
                }

                if capitalize_next {
                    result.push(ch.to_uppercase().next().unwrap_or(ch));
                    capitalize_next = false;
                } else {
                    result.push(ch.to_lowercase().next().unwrap_or(ch));
                }

                prev_was_upper = ch.is_uppercase();
            } else {
                // Skip non-alphanumeric and capitalize next
                capitalize_next = true;
                prev_was_upper = false;
            }
        }

        // Handle numeric starts
        if result.chars().next().map_or(false, |c| c.is_numeric()) {
            result = format!("N{}", result);
        }

        result
    }

    /// Generate test module
    fn generate_tests(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<GeneratedOutput> {
        let mut output = String::new();

        writeln!(&mut output, "#[cfg(test)]").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "mod tests {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    use super::*;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate basic tests for each class
        if !schema.classes.is_empty() {
            for (class_name, _class) in &schema.classes {
                let struct_name = self.convert_identifier(class_name);

                writeln!(&mut output, "    #[test]").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "    fn test_{}_creation() {{",
                    struct_name.to_lowercase()
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        let instance = {struct_name}::new();")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        // Add assertions here")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        let filename = format!(
            "{}_tests.rs",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
            .to_lowercase()
            .replace('-', "_")
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("type".to_string(), "tests".to_string());

        Ok(GeneratedOutput {
            content: output,
            filename,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rust_generation() {
        let generator = RustGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "https://example.org/test".to_string();
        schema.name = "test_schema".to_string();
        schema.description = Some("Test schema for Rust generation".to_string());

        // Add slots with various features
        let mut name_slot = SlotDefinition::default();
        name_slot.name = "name".to_string();
        name_slot.description = Some("Person's name".to_string());
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut email_slot = SlotDefinition::default();
        email_slot.name = "email".to_string();
        email_slot.range = Some("string".to_string());
        email_slot.pattern = Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string());
        schema.slots.insert("email".to_string(), email_slot);

        let mut age_slot = SlotDefinition::default();
        age_slot.name = "age".to_string();
        age_slot.range = Some("integer".to_string());
        age_slot.minimum_value = Some(json!(0));
        age_slot.maximum_value = Some(json!(150));
        schema.slots.insert("age".to_string(), age_slot);

        let mut status_slot = SlotDefinition::default();
        status_slot.name = "status".to_string();
        status_slot.permissible_values = vec![
            PermissibleValue::Simple("active".to_string()),
            PermissibleValue::Simple("inactive".to_string()),
            PermissibleValue::Simple("pending".to_string()),
        ];
        schema.slots.insert("status".to_string(), status_slot);

        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.name = "Person".to_string();
        person_class.description = Some("A person with attributes".to_string());
        person_class.slots = vec![
            "name".to_string(),
            "email".to_string(),
            "age".to_string(),
            "status".to_string(),
        ];
        schema.classes.insert("Person".to_string(), person_class);

        let options = GeneratorOptions::new().with_docs(true);
        let outputs = AsyncGenerator::generate(&generator, &schema, &options)
            .await
            .expect("should generate Rust output");

        assert_eq!(outputs.len(), 1);
        let output = &outputs[0].content;

        // Check imports
        assert!(output.contains("use serde::{Deserialize, Serialize};"));
        assert!(output.contains("use once_cell::sync::Lazy;"));
        assert!(output.contains("use regex::Regex;"));

        // Check validation error enum
        assert!(output.contains("pub enum ValidationError"));
        assert!(output.contains("RequiredField { field: &'static str }"));
        assert!(output.contains("PatternMismatch"));
        assert!(output.contains("RangeViolation"));

        // Check pattern constant
        assert!(output.contains("static PATTERN_EMAIL: Lazy<Regex>"));

        // Check status enum
        assert!(output.contains("pub enum Status"));
        assert!(output.contains("Active,"));
        assert!(output.contains("impl std::fmt::Display for Status"));
        assert!(output.contains("impl std::str::FromStr for Status"));

        // Check struct
        assert!(output.contains("pub struct Person"));
        assert!(
            output.contains("#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]")
        );
        assert!(output.contains("pub name: String,"));
        assert!(output.contains("pub email: Option<String>,"));
        assert!(output.contains("pub age: Option<i64>,"));
        assert!(output.contains("pub status: Option<Status>,"));

        // Check impl block
        assert!(output.contains("impl Person {"));
        assert!(output.contains("pub fn new(name: impl Into<String>) -> Self"));
        assert!(output.contains("pub fn validate(&self) -> Result<(), Vec<ValidationError>>"));
    }

    #[test]
    fn test_field_name_conversion() {
        let generator = RustGenerator::new();

        assert_eq!(generator.convert_field_name("firstName"), "first_name");
        assert_eq!(generator.convert_field_name("type"), "type_");
        assert_eq!(generator.convert_field_name("HTTPResponse"), "httpresponse");
        assert_eq!(generator.convert_field_name("self"), "self_");
    }

    #[test]
    fn test_enum_variant_conversion() {
        let generator = RustGenerator::new();

        assert_eq!(generator.convert_enum_variant("active"), "Active");
        assert_eq!(generator.convert_enum_variant("in-progress"), "InProgress");
        assert_eq!(generator.convert_enum_variant("NOT_STARTED"), "NotStarted");
        assert_eq!(generator.convert_enum_variant("404-error"), "N404Error");
    }
}
