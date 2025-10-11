//! TypeScript code generator for `LinkML` schemas

use super::base::{BaseCodeFormatter, TypeMapper, collect_all_slots, is_optional_slot};
use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{
    AsyncGenerator, CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use async_trait::async_trait;
use linkml_core::error::LinkMLError;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// TypeScript generator
pub struct TypeScriptGenerator {
    name: String,
    description: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for TypeScriptGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// Implement the synchronous Generator trait for backward compatibility
impl Generator for TypeScriptGenerator {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn description(&self) -> &'static str {
        "Generate TypeScript interfaces and types from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for typescript generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Use tokio to run the async version
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {e}")))?;

        let options = GeneratorOptions::new();
        let outputs = runtime
            .block_on(AsyncGenerator::generate(self, schema, &options))
            .map_err(|e| LinkMLError::service(e.to_string()))?;

        // Concatenate all outputs into a single string
        Ok(outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join(
                "
",
            ))
    }

    fn get_file_extension(&self) -> &'static str {
        "ts"
    }

    fn get_default_filename(&self) -> &'static str {
        "generated.ts"
    }
}

impl TypeScriptGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new TypeScript generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "typescript".to_string(),
            description: "Generate TypeScript interfaces and types from LinkML schemas".to_string(),
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

    /// Generate code for a single interface
    fn generate_interface(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Generate interface documentation
        if options.include_docs && class.description.is_some() {
            writeln!(&mut output, "/**").map_err(Self::fmt_error_to_generator_error)?;
            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 70, " * ");
                writeln!(&mut output, " * {wrapped}")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(&mut output, " * @generated from LinkML schema")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, " */").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Check if we have inheritance
        let extends_clause = if let Some(ref parent) = class.is_a {
            format!(" extends {parent}")
        } else {
            String::new()
        };

        writeln!(
            &mut output,
            "export interface {class_name}{extends_clause} {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Collect all slots including inherited
        let slots = collect_all_slots(class, schema)?;

        // Generate fields
        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Skip if this slot is from parent
                if let Some(ref parent) = class.is_a
                    && let Some(parent_class) = schema.classes.get(parent)
                {
                    let parent_slots = collect_all_slots(parent_class, schema)?;
                    if parent_slots.contains(slot_name) {
                        continue;
                    }
                }

                self.generate_field(&mut output, slot_name, slot, schema, options)?;
            }
        }

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        // Generate type guard
        if options
            .get_custom("generate_type_guards")
            .map(std::string::String::as_str)
            != Some("false")
        {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_type_guard(&mut output, class_name, class, schema)?;
        }

        // Generate validator function
        if options
            .get_custom("generate_validators")
            .map(std::string::String::as_str)
            == Some("true")
        {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_validator(&mut output, class_name, class, schema)?;
        }

        Ok(output)
    }

    /// Generate a single field
    fn generate_field(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        // Add field documentation
        if options.include_docs && slot.description.is_some() {
            writeln!(output, "  /**").map_err(Self::fmt_error_to_generator_error)?;
            if let Some(ref desc) = slot.description {
                writeln!(output, "   * {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(output, "   */").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine the type
        let base_type = self.get_field_type(slot, schema)?;

        // Handle multivalued with advanced collection types
        let field_type = if slot.multivalued.unwrap_or(false) {
            if slot.unique.unwrap_or(false) {
                format!("Set<{base_type}>")
            } else if slot.ordered.unwrap_or(false) {
                format!("{base_type}[]")
            } else {
                format!("ReadonlyArray<{base_type}>")
            }
        } else {
            base_type
        };

        // Handle optional
        let optional_marker = if is_optional_slot(slot) { "?" } else { "" };

        writeln!(output, "  {slot_name}{optional_marker}: {field_type};")
            .map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get the TypeScript type for a field
    fn get_field_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        // Check if it's an enum
        if !slot.permissible_values.is_empty() {
            let enum_name = BaseCodeFormatter::to_pascal_case(&slot.name);
            return Ok(enum_name);
        }

        // Check range
        if let Some(ref range) = slot.range {
            // Check if it's a class
            if schema.classes.contains_key(range) {
                return Ok(range.clone());
            }

            // Check if it's a type
            if let Some(type_def) = schema.types.get(range)
                && let Some(ref base_type) = type_def.base_type
            {
                return Ok(TypeMapper::to_typescript(base_type).to_string());
            }

            // Otherwise map as primitive
            Ok(TypeMapper::to_typescript(range).to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    /// Generate a type guard function
    fn generate_type_guard(
        &self,
        output: &mut String,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " * Type guard for {class_name}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "export function is{class_name}(obj: unknown): obj is {class_name} {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "  return (").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    typeof obj === 'object' &&")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    obj !== null &&").map_err(Self::fmt_error_to_generator_error)?;

        // Get direct slots (not inherited)
        let direct_slots = class.slots.clone();

        // Check required fields
        for (i, slot_name) in direct_slots.iter().enumerate() {
            if let Some(slot) = schema.slots.get(slot_name)
                && slot.required.unwrap_or(false)
            {
                write!(output, "    '{slot_name}' in obj")
                    .map_err(Self::fmt_error_to_generator_error)?;

                // Add type check
                let expected_type = match self.get_field_type(slot, schema)?.as_str() {
                    "string" => "string",
                    "number" => "number",
                    "boolean" => "boolean",
                    _ => "object",
                };

                if expected_type != "object" {
                    write!(output, " &&").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    write!(
                        output,
                        "    typeof (obj as any).{slot_name} === '{expected_type}'"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                if i < direct_slots.len() - 1 {
                    writeln!(output, " &&").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output, "  );").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate a validator function
    fn generate_validator(
        &self,
        output: &mut String,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " * Validator for {class_name}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "export function validate{class_name}(obj: unknown): ValidationResult<{class_name}> {{"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  const errors: ValidationError[] = [];")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        writeln!(output, "  if (!is{class_name}(obj)) {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "    errors.push({{ path: '', message: 'Not a valid {class_name} object' }});"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    return {{ valid: false, errors }};")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        // Add comprehensive validation for constraints
        let slots = collect_all_slots(class, schema)?;
        let mut has_validations = false;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Required field validation
                if slot.required.unwrap_or(false) {
                    if !has_validations {
                        writeln!(output, "  // Field validation")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        has_validations = true;
                    }
                    writeln!(
                        output,
                        "  if (obj.{slot_name} === undefined || obj.{slot_name} === null) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    errors.push({{ path: '{slot_name}', message: '{slot_name} is required' }});"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }

                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    if !has_validations {
                        writeln!(output, "  // Pattern validation")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        has_validations = true;
                    }
                    writeln!(
                        output,
                        "  if (obj.{slot_name} && !/{pattern}/test(String(obj.{slot_name}))) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    errors.push({{ path: '{slot_name}', message: '{slot_name} does not match pattern: {pattern}' }});"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }

                // Range validation for numbers
                if let Some(min) = slot.minimum_value.as_ref() {
                    if !has_validations {
                        writeln!(output, "  // Range validation")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        has_validations = true;
                    }
                    writeln!(
                        output,
                        "  if (obj.{slot_name} !== undefined && typeof obj.{slot_name} === 'number' && obj.{slot_name} < {min}) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    errors.push({{ path: '{slot_name}', message: '{slot_name} must be >= {min}' }});"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(max) = slot.maximum_value.as_ref() {
                    writeln!(
                        output,
                        "  if (obj.{slot_name} !== undefined && typeof obj.{slot_name} === 'number' && obj.{slot_name} > {max}) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    errors.push({{ path: '{slot_name}', message: '{slot_name} must be <= {max}' }});"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }

                // Enum validation
                if !slot.permissible_values.is_empty() {
                    let values: Vec<String> = slot
                        .permissible_values
                        .iter()
                        .map(|pv| {
                            let text = match pv {
                                linkml_core::types::PermissibleValue::Simple(s) => s,
                                linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                            };
                            format!("'{text}'")
                        })
                        .collect();
                    writeln!(
                        output,
                        "  if (obj.{} !== undefined && ![{}].includes(obj.{})) {{",
                        slot_name,
                        values.join(", "),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    errors.push({{ path: '{}', message: '{} must be one of: {}' }});",
                        slot_name,
                        slot_name,
                        values.join(", ")
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }

                // Range validation
                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    if let Some(ref min) = slot.minimum_value {
                        writeln!(
                            output,
                            "  if (obj.{slot_name} !== undefined && obj.{slot_name} < {min}) {{"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "    errors.push({{ path: '{slot_name}', message: 'Must be >= {min}' }});"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    }
                    if let Some(ref max) = slot.maximum_value {
                        writeln!(
                            output,
                            "  if (obj.{slot_name} !== undefined && obj.{slot_name} > {max}) {{"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "    errors.push({{ path: '{slot_name}', message: 'Must be <= {max}' }});"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  return errors.length === 0")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    ? {{ valid: true, data: obj }}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    : {{ valid: false, errors }};")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate enum from permissible values
    fn generate_enum(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);

        if let Some(ref desc) = slot.description {
            writeln!(output, "/**").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, " * {desc}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, " */").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "export enum {enum_name} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        for (i, value) in slot.permissible_values.iter().enumerate() {
            let text = match value {
                PermissibleValue::Simple(text) | PermissibleValue::Complex { text, .. } => text,
            };
            let const_name = text.to_uppercase().replace([' ', '-'], "_");
            write!(output, "  {const_name} = \"{text}\"")
                .map_err(Self::fmt_error_to_generator_error)?;
            if i < slot.permissible_values.len() - 1 {
                writeln!(output, ",").map_err(Self::fmt_error_to_generator_error)?;
            } else {
                writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }
}

#[async_trait]
impl AsyncGenerator for TypeScriptGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["ts"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        AsyncGenerator::validate_schema(self, schema).await?;

        let mut outputs = Vec::new();
        let mut content = String::new();

        // File header
        writeln!(&mut content, "/**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut content,
            " * Generated from LinkML schema: {}",
            schema.name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(ref desc) = schema.description {
            writeln!(&mut content, " * {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut content, " */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;

        // Generate validation types if needed
        if options
            .get_custom("generate_validators")
            .map(std::string::String::as_str)
            == Some("true")
        {
            writeln!(&mut content, "// Validation types")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "export interface ValidationError {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "  path: string;")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "  message: string;")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "export type ValidationResult<T> =")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "  | {{ valid: true; data: T }}")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut content,
                "  | {{ valid: false; errors: ValidationError[] }};"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate enums first
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                self.generate_enum(&mut content, slot_name, slot)?;
                writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Generate interfaces
        for (class_name, class_def) in &schema.classes {
            let interface_code = self.generate_interface(class_name, class_def, schema, options)?;
            content.push_str(&interface_code);
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
        }

        outputs.push(GeneratedOutput {
            content,
            filename: format!("{}.ts", schema.name.to_lowercase().replace('-', "_")),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("generator".to_string(), self.name.clone());
                meta.insert("schema".to_string(), schema.name.clone());
                meta.insert("typescript_version".to_string(), "5.0".to_string());
                meta
            },
        });

        Ok(outputs)
    }

    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        if schema.name.is_empty() {
            return Err(GeneratorError::SchemaValidation(
                "Schema must have a name".to_string(),
            ));
        }

        if schema.classes.is_empty() {
            return Err(GeneratorError::SchemaValidation(
                "Schema must have at least one class".to_string(),
            ));
        }

        Ok(())
    }
}

impl CodeFormatter for TypeScriptGenerator {
    fn name(&self) -> &'static str {
        "typescript"
    }

    fn description(&self) -> &'static str {
        "Code formatter for typescript output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["ts", "tsx"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // Basic formatting - just ensure consistent indentation
        let mut formatted = String::new();
        let indent = "    ";
        let mut indent_level: usize = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add proper indentation
            formatted.push_str(&indent.repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after opening braces
            if trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(') {
                indent_level += 1;
            }
        }

        Ok(formatted)
    }
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let indent_str = indent.to_string(level);
        let lines: Vec<&str> = doc.lines().collect();

        let mut result = format!("{indent_str}/**");
        for line in lines {
            result.push('\n');
            result.push_str(&indent_str);
            result.push_str(" * ");
            result.push_str(line);
        }
        result.push('\n');
        result.push_str(&indent_str);
        result.push_str(" */");
        result
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        indent: &IndentStyle,
        level: usize,
        separator: &str,
    ) -> String {
        items
            .iter()
            .map(|item| format!("{}{}", indent.to_string(level), item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        BaseCodeFormatter::escape_js_string(s)
    }

    fn convert_identifier(&self, id: &str) -> String {
        // TypeScript identifiers are typically camelCase
        BaseCodeFormatter::to_camel_case(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[tokio::test]
    async fn test_basic_generation() {
        let mut schema = SchemaDefinition {
            name: "test_schema".to_string(),
            ..Default::default()
        };

        let person_class = ClassDefinition {
            name: "Person".to_string(),
            description: Some("A person".to_string()),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };

        schema.classes.insert("Person".to_string(), person_class);

        let name_slot = SlotDefinition {
            name: "name".to_string(),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };

        let age_slot = SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            ..Default::default()
        };

        schema.slots.insert("name".to_string(), name_slot);
        schema.slots.insert("age".to_string(), age_slot);

        let generator = TypeScriptGenerator::new();
        let options = GeneratorOptions::new();

        let outputs = AsyncGenerator::generate(&generator, &schema, &options)
            .await
            .expect("should generate TypeScript output: {}");
        assert_eq!(outputs.len(), 1);

        let output = &outputs[0];
        assert!(output.content.contains("export interface Person"));
        assert!(output.content.contains("name: string;"));
        assert!(output.content.contains("age?: number;"));
        assert!(output.content.contains("export function isPerson"));
    }
}
