//! Field generation and type mapping

use super::base::{BaseCodeFormatter, collect_all_slots};
use super::core::RustGenerator;
use super::traits::{GeneratorOptions, GeneratorResult, IndentStyle};
use linkml_core::prelude::*;
use linkml_core::types::PermissibleValue;
use std::fmt::Write;

impl RustGenerator {
    /// Generate struct fields
    pub(super) fn generate_fields(
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
                let rust_name = Self::convert_field_name(slot_name);
                let mut attrs = Vec::new();

                // Documentation
                if options.include_docs
                    && let Some(desc) = &slot.description
                {
                    writeln!(output, "{}/// {}", indent.single(), desc)
                        .map_err(Self::fmt_error_to_generator_error)?;
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
                let field_type = Self::get_rust_type(slot, schema);
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

    /// Get Rust type for a slot
    pub(super) fn get_rust_type(slot: &SlotDefinition, schema: &SchemaDefinition) -> String {
        let base_type = Self::get_base_type(slot.range.as_ref(), schema);

        if slot.multivalued.unwrap_or(false) {
            format!("Vec<{base_type}>")
        } else if slot.required.unwrap_or(false) {
            base_type
        } else {
            format!("Option<{base_type}>")
        }
    }

    /// Get base Rust type for a range
    pub(super) fn get_base_type(range: Option<&String>, schema: &SchemaDefinition) -> String {
        match range {
            Some(range_name) => {
                // Check if it's a built-in type
                match range_name.as_str() {
                    "string" | "str" | "uri" | "uriorcurie" => "String".to_string(),
                    "integer" | "int" => "i64".to_string(),
                    "float" | "double" => "f64".to_string(),
                    "boolean" | "bool" => "bool".to_string(),
                    "date" => "chrono::NaiveDate".to_string(),
                    "datetime" => "chrono::DateTime<chrono::Utc>".to_string(),
                    "time" => "chrono::NaiveTime".to_string(),
                    "decimal" => "rust_decimal::Decimal".to_string(),
                    _ => {
                        // Check if it's a class in the schema
                        if schema.classes.contains_key(range_name)
                            || schema.enums.contains_key(range_name)
                        {
                            BaseCodeFormatter::to_pascal_case(range_name)
                        } else {
                            // Unknown type, default to String with a comment
                            "String".to_string() // Default fallback
                        }
                    }
                }
            }
            None => "String".to_string(),
        }
    }

    /// Get default value for a field
    pub(super) fn get_default_value(slot: &SlotDefinition, schema: &SchemaDefinition) -> String {
        // Multivalued fields always use Vec::new() as default
        if slot.multivalued.unwrap_or(false) {
            "Vec::new()".to_string()
        } else if slot.required.unwrap_or(false) {
            match Self::get_base_type(slot.range.as_ref(), schema).as_str() {
                "String" => "String::new()".to_string(),
                "i64" => "0".to_string(),
                "f64" => "0.0".to_string(),
                "bool" => "false".to_string(),
                _ => "Default::default()".to_string(),
            }
        } else {
            "None".to_string()
        }
    }

    /// Generate enum from `LinkML` enum definition
    pub(super) fn generate_enum_rust(
        enum_name: &str,
        enum_def: &EnumDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let struct_name = BaseCodeFormatter::to_pascal_case(enum_name);

        // Documentation
        if options.include_docs {
            writeln!(&mut output, "/// {enum_name}").map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &enum_def.description {
                writeln!(&mut output, "/// {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Enum definition
        writeln!(
            &mut output,
            "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "pub enum {struct_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate variants
        if !enum_def.permissible_values.is_empty() {
            let permissible_values = &enum_def.permissible_values;
            for value_def in permissible_values {
                let (value_name, description) = match value_def {
                    PermissibleValue::Simple(text) => (text.as_str(), None),
                    PermissibleValue::Complex {
                        text, description, ..
                    } => (text.as_str(), description.as_deref()),
                };

                if options.include_docs
                    && let Some(desc) = description
                {
                    writeln!(&mut output, "{}/// {}", indent.single(), desc)
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                let variant_name = BaseCodeFormatter::to_pascal_case(value_name);
                writeln!(&mut output, "{}{},", indent.single(), variant_name)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate implementation
        writeln!(&mut output, "impl {struct_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // to_string method
        writeln!(
            &mut output,
            "{}/// Convert to string representation",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "{}pub fn as_str(&self) -> &'static str {{",
            indent.single()
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}match self {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;

        if !enum_def.permissible_values.is_empty() {
            let permissible_values = &enum_def.permissible_values;
            for value_def in permissible_values {
                let value_name = match value_def {
                    PermissibleValue::Simple(text) | PermissibleValue::Complex { text, .. } => {
                        text.as_str()
                    }
                };
                let variant_name = BaseCodeFormatter::to_pascal_case(value_name);
                writeln!(
                    &mut output,
                    "{}{}::{} => \"{}\",",
                    indent.to_string(3),
                    struct_name,
                    variant_name,
                    value_name
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(&mut output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}}}", indent.single())
            .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}
