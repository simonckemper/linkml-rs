//! Core Rust generator implementation

use super::GeneratorOptions;
use super::base::BaseCodeFormatter;
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};

use linkml_core::prelude::*;
use std::fmt::Write;

/// Rust code generator for `LinkML` schemas
pub struct RustGenerator {
    /// Generator name
    pub name: String,
    /// Generator description
    pub description: String,
    /// Generator options
    pub options: super::traits::GeneratorOptions,
}

impl RustGenerator {
    /// Create a new Rust generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "rust".to_string(),
            description: "Generate idiomatic Rust code from LinkML schemas with serde support, comprehensive validation, and zero-tolerance for unwrap()".to_string(),
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

    /// Get the generator name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the generator description
    #[must_use]
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Check if a class has subclasses
    pub(super) fn has_subclasses(class_name: &str, schema: &SchemaDefinition) -> bool {
        schema
            .classes
            .values()
            .any(|c| c.is_a.as_deref() == Some(class_name))
    }

    /// Get all direct subclasses of a class
    pub(super) fn get_subclasses(class_name: &str, schema: &SchemaDefinition) -> Vec<String> {
        schema
            .classes
            .iter()
            .filter(|(_, c)| c.is_a.as_deref() == Some(class_name))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Convert field name to Rust naming convention
    pub(super) fn convert_field_name(name: &str) -> String {
        BaseCodeFormatter::to_snake_case(name)
    }

    /// Convert error from `fmt::Error` to `GeneratorError`
    pub(super) fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Generation(format!("Format error: {err}"))
    }

    /// Map `LinkML` type to Rust type
    pub(super) fn linkml_type_to_rust(linkml_type: &str) -> &str {
        match linkml_type {
            "integer" | "int" => "i64",
            "float" | "double" | "decimal" => "f64",
            "boolean" | "bool" => "bool",
            "date" => "chrono::NaiveDate",
            "datetime" => "chrono::DateTime<chrono::Utc>",
            "time" => "chrono::NaiveTime",
            // All string-like types map to String
            _ => "String",
        }
    }

    /// Generate an enum from `LinkML` enum definition
    pub(super) fn generate_enum(
        enum_name: &str,
        enum_def: &EnumDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add documentation if available
        if let Some(ref desc) = enum_def.description {
            writeln!(
                &mut output,
                "
/// {desc}"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(
                &mut output,
                "
/// {enum_name} enum"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(
            &mut output,
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "pub enum {enum_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate enum variants
        for pv in &enum_def.permissible_values {
            match pv {
                PermissibleValue::Simple(value) => {
                    // Convert to PascalCase for Rust enum variant
                    let variant_name = BaseCodeFormatter::to_pascal_case(value);
                    writeln!(&mut output, "    #[serde(rename = \"{value}\")]")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "    {variant_name},")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                PermissibleValue::Complex {
                    text, description, ..
                } => {
                    if let Some(desc) = description {
                        writeln!(&mut output, "    /// {desc}")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    let variant_name = BaseCodeFormatter::to_pascal_case(text);
                    writeln!(&mut output, "    #[serde(rename = \"{text}\")]")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(&mut output, "    {variant_name},")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(
            &mut output,
            "}}
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        Ok(output)
    }

    /// Generate a class/struct from `LinkML` class definition
    pub(super) fn generate_class(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add documentation
        if let Some(ref desc) = class_def.description {
            writeln!(
                &mut output,
                "
/// {desc}"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(
                &mut output,
                "
/// {class_name}"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add derive macros
        writeln!(
            &mut output,
            "#[derive(Debug, Clone, Serialize, Deserialize)]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "pub struct {class_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Collect all slots for this class
        let slots = self.collect_class_slots(class_def, schema);

        if slots.is_empty() {
            writeln!(&mut output, "    // No fields defined")
                .map_err(Self::fmt_error_to_generator_error)?;
        } else {
            // Generate fields for each slot
            for slot_name in &slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    Self::generate_field(&mut output, slot_name, slot_def, schema)?;
                }
            }
        }

        writeln!(
            &mut output,
            "}}
"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        Ok(output)
    }

    /// Generate a field from a slot definition
    pub(super) fn generate_field(
        output: &mut String,
        slot_name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        // Add field documentation
        if let Some(ref desc) = slot_def.description {
            writeln!(output, "    /// {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add serde rename if needed (to preserve original casing)
        let rust_field_name = Self::convert_field_name(slot_name);
        if rust_field_name != slot_name {
            writeln!(output, "    #[serde(rename = \"{slot_name}\")]")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine field type
        let base_type = if let Some(ref range) = slot_def.range {
            // Check if it's an enum
            if schema.enums.contains_key(range) {
                range.clone()
            }
            // Check if it's a class
            else if schema.classes.contains_key(range) {
                format!("Box<{range}>") // Box to avoid infinite size for recursive types
            }
            // Otherwise treat as primitive
            else {
                Self::linkml_type_to_rust(range).to_string()
            }
        } else {
            "String".to_string() // Default type
        };

        // Handle multivalued
        let field_type = if slot_def.multivalued.unwrap_or(false) {
            format!("Vec<{base_type}>")
        } else {
            base_type
        };

        // Handle optional
        let final_type =
            if !slot_def.required.unwrap_or(false) && !slot_def.multivalued.unwrap_or(false) {
                format!("Option<{field_type}>")
            } else {
                field_type
            };

        // Write the field
        writeln!(output, "    pub {rust_field_name}: {final_type},")
            .map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Collect all slots for a class (including inherited)
    pub(super) fn collect_class_slots(
        &self,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut slots = Vec::new();

        // Add inherited slots if there's a parent class
        if let Some(ref parent) = class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent)
        {
            slots.extend(self.collect_class_slots(parent_class, schema));
        }

        // Add this class's slots
        slots.extend_from_slice(&class_def.slots);

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        slots.retain(|slot| seen.insert(slot.clone()));

        slots
    }

    /// Generate file header with imports
    pub(super) fn generate_header(schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // File header
        writeln!(
            &mut output,
            "//! Generated from LinkML schema: {}",
            if schema.name.is_empty() {
                "unnamed"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &schema.description {
            writeln!(&mut output, "//! {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Standard imports
        writeln!(&mut output, "use serde::{{Deserialize, Serialize}};")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "use std::collections::HashMap;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "use thiserror::Error;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate validation error enum
    pub(super) fn generate_validation_error() -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "/// Validation errors for generated types")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "#[derive(Debug, Error)]")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "pub enum ValidationError {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    #[error(\"Required field missing: {{field}}\")]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    RequiredField {{ field: &'static str }},")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    #[error(\"Invalid value for field {{field}}: {{message}}\")]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    InvalidValue {{ field: &'static str, message: String }},"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    #[error(\"Pattern validation failed for field {{field}}\")]"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    PatternValidation {{ field: &'static str }},"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RustGenerator {
    fn name(&self) -> &'static str {
        "rust"
    }

    fn description(&self) -> &'static str {
        "Generate Rust structs and enums from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Rust generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        let _options = GeneratorOptions::default();
        let mut output = String::new();

        // Generate header
        output.push_str(
            &Self::generate_header(schema)
                .map_err(|e| LinkMLError::data_validation(e.to_string()))?,
        );

        // Generate validation error enum
        output.push_str(
            &Self::generate_validation_error()
                .map_err(|e| LinkMLError::data_validation(e.to_string()))?,
        );

        // Generate enums first
        for (enum_name, enum_def) in &schema.enums {
            output.push_str(
                &Self::generate_enum(enum_name, enum_def)
                    .map_err(|e| LinkMLError::data_validation(e.to_string()))?,
            );
        }

        // Generate basic structs for classes
        for (class_name, class_def) in &schema.classes {
            output.push_str(
                &self
                    .generate_class(class_name, class_def, schema)
                    .map_err(|e| LinkMLError::data_validation(e.to_string()))?,
            );
        }

        Ok(output)
    }

    fn get_file_extension(&self) -> &'static str {
        "rs"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

impl CodeFormatter for RustGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["rs"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // In a real implementation, this would use rustfmt
        // For now, just return the code as-is
        Ok(code.to_string())
    }
}
