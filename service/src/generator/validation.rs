//! Validation method generation

use super::base::collect_all_slots;
use super::core::RustGenerator;
use super::traits::{GeneratorOptions, GeneratorResult, IndentStyle};
use linkml_core::prelude::*;
use std::fmt::Write;

impl RustGenerator {
    /// Generate `validate()` method
    pub(super) fn generate_validate_method(
        output: &mut String,
        _struct_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        if options.include_docs {
            writeln!(output, "{}/// Validate this instance", indent.single())
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            output,
            "{}pub fn validate(&self) -> Result<(), ValidationError> {{",
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

        // Generate validation for each field
        let all_slots = collect_all_slots(class, schema)?;
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                Self::generate_field_validation(output, slot_name, slot, schema, indent)?;
            }
        }

        // Return result
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}if errors.is_empty() {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}Ok(())", indent.to_string(3))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}} else {{", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}Err(errors.into_iter().next().expect(\\\"iterator should have next item\\\"))",
            indent.to_string(3)
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.single()).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate validation for a specific field
    fn generate_field_validation(
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        _schema: &SchemaDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let field_name = Self::convert_field_name(slot_name);

        // Required field validation
        if slot.required.unwrap_or(false) {
            if slot.multivalued.unwrap_or(false) {
                // Required multivalued field - check if empty
                writeln!(
                    output,
                    "{}// Required multivalued field: {}",
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
            } else if slot.range.as_deref() == Some("string") {
                // Required string field - check if empty
                writeln!(
                    output,
                    "{}// Required string field: {}",
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
        }

        // Pattern validation
        if let Some(pattern) = &slot.pattern {
            writeln!(
                output,
                "{}// Pattern validation for field: {}",
                indent.to_string(2),
                slot_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if slot.multivalued.unwrap_or(false) {
                // Validate each item in the vector
                writeln!(
                    output,
                    "{}for item in &self.{} {{",
                    indent.to_string(2),
                    field_name
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                Self::generate_pattern_check(
                    output,
                    slot_name,
                    pattern,
                    "item",
                    &indent.to_string(3),
                )?;
                writeln!(output, "{}}}", indent.to_string(2))
                    .map_err(Self::fmt_error_to_generator_error)?;
            } else if slot.required.unwrap_or(false) {
                // Validate required field directly
                Self::generate_pattern_check(
                    output,
                    slot_name,
                    pattern,
                    &format!("&self.{field_name}"),
                    &indent.to_string(2),
                )?;
            } else {
                // Validate optional field if present
                writeln!(
                    output,
                    "{}if let Some(ref value) = self.{} {{",
                    indent.to_string(2),
                    field_name
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                Self::generate_pattern_check(
                    output,
                    slot_name,
                    pattern,
                    "value",
                    &indent.to_string(3),
                )?;
                writeln!(output, "{}}}", indent.to_string(2))
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Range validation for numeric types
        if let Some(range) = &slot.range
            && matches!(range.as_str(), "integer" | "int" | "float" | "double")
            && let (Some(min_val), Some(max_val)) = (&slot.minimum_value, &slot.maximum_value)
        {
            Self::generate_range_validation(
                output,
                slot_name,
                &field_name,
                min_val,
                max_val,
                slot,
                indent,
            )?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate pattern validation check
    fn generate_pattern_check(
        output: &mut String,
        slot_name: &str,
        pattern: &str,
        value_expr: &str,
        indent_str: &str,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "{}let regex = regex::Regex::new(r\"{}\").map_err(|_| {{",
            indent_str,
            pattern.replace('"', "\\\"")
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{indent_str}    ValidationError::InvalidValue {{ field: \"{slot_name}\", message: \"Invalid regex pattern\".to_string() }}"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{indent_str}}})?;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{indent_str}if !regex.is_match({value_expr}) {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{indent_str}    errors.push(ValidationError::PatternValidation {{ field: \"{slot_name}\" }});"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{indent_str}}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate range validation for numeric fields
    #[allow(clippy::too_many_arguments)]
    fn generate_range_validation(
        output: &mut String,
        slot_name: &str,
        field_name: &str,
        min_val: &serde_json::Value,
        max_val: &serde_json::Value,
        slot: &SlotDefinition,
        indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        let check_expr = if slot.required.unwrap_or(false) {
            format!("self.{field_name}")
        } else {
            "*value".to_string()
        };

        let validation_code = format!(
            "{}if {} < {} || {} > {} {{",
            indent.to_string(2),
            check_expr,
            min_val,
            check_expr,
            max_val
        );

        if !slot.required.unwrap_or(false) {
            writeln!(
                output,
                "{}if let Some(ref value) = self.{} {{",
                indent.to_string(2),
                field_name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "{validation_code}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "{}errors.push(ValidationError::InvalidValue {{ field: \"{}\", message: format!(\"Value must be between {} and {}\", {}, {}) }});",
            indent.to_string(3),
            slot_name,
            min_val,
            max_val,
            min_val,
            max_val
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "{}}}", indent.to_string(2))
            .map_err(Self::fmt_error_to_generator_error)?;

        if !slot.required.unwrap_or(false) {
            writeln!(output, "{}}}", indent.to_string(2))
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }
}
