//! JavaScript (ES6) code generator for `LinkML` schemas

use super::base::{BaseCodeFormatter, TypeMapper, collect_all_slots, is_optional_slot};
use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};
use linkml_core::{error::LinkMLError, prelude::*};
use std::fmt::Write;

/// JavaScript generator
pub struct JavaScriptGenerator {
    name: String,
    description: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for JavaScriptGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new JavaScript generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "javascript".to_string(),
            description: "Generate JavaScript ES6 classes from LinkML schemas".to_string(),
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

    /// Generate code for a single class
    fn generate_class(
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // Generate class documentation
        if options.include_docs {
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

        // Generate class definition
        let extends_clause = if let Some(ref parent) = class.is_a {
            format!(" extends {parent}")
        } else {
            String::new()
        };

        writeln!(&mut output, "export class {class_name}{extends_clause} {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate constructor JSDoc
        writeln!(&mut output, "  /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "   * @param {{Object}} data - Initialization data"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Collect all slots including inherited
        let all_slots = collect_all_slots(class, schema)?;

        // Get direct slots only for constructor params
        let direct_slots: Vec<String> = if let Some(ref parent) = class.is_a {
            let parent_slots = if let Some(parent_class) = schema.classes.get(parent) {
                collect_all_slots(parent_class, schema)?
            } else {
                vec![]
            };
            all_slots
                .into_iter()
                .filter(|slot| !parent_slots.contains(slot))
                .collect()
        } else {
            all_slots
        };

        // Document constructor parameters
        for slot_name in &direct_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let type_str = Self::get_jsdoc_type(slot, schema);
                let optional = if is_optional_slot(slot) { "[" } else { "" };
                let optional_close = if is_optional_slot(slot) { "]" } else { "" };

                write!(
                    &mut output,
                    "   * @param {{{optional}{type_str}{optional_close}}} "
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                if is_optional_slot(slot) {
                    write!(&mut output, "[data.{slot_name}] - ")
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(&mut output, "data.{slot_name} - ")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                if let Some(ref desc) = slot.description {
                    write!(&mut output, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(
                        &mut output,
                        "{} value",
                        BaseCodeFormatter::to_pascal_case(slot_name)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        writeln!(&mut output, "   */").map_err(Self::fmt_error_to_generator_error)?;

        // Generate constructor
        writeln!(&mut output, "  constructor(data = {{}}) {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Call super if has parent
        if class.is_a.is_some() {
            writeln!(&mut output, "    super(data);")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Validate data
        writeln!(&mut output, "    this.#validate(data);")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Initialize fields
        for slot_name in &direct_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.multivalued.unwrap_or(false) {
                    writeln!(
                        &mut output,
                        "    this.{slot_name} = data.{slot_name} || [];"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else if is_optional_slot(slot) {
                    writeln!(
                        &mut output,
                        "    this.{slot_name} = data.{slot_name} || null;"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(&mut output, "    this.{slot_name} = data.{slot_name};")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate private validation method
        Self::generate_validation_method(&mut output, &direct_slots, schema)?;

        // Generate static fromJSON method
        writeln!(&mut output, "  /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * Create from JSON")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * @param {{string}} json - JSON string")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * @returns {{{class_name}}}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  static fromJSON(json) {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    return new {class_name}(JSON.parse(json));"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate toObject method
        writeln!(&mut output, "  /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * Convert to plain object")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * @returns {{Object}}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  toObject() {{").map_err(Self::fmt_error_to_generator_error)?;

        if class.is_a.is_some() {
            writeln!(&mut output, "    const parentData = super.toObject();")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    return {{").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "      ...parentData,")
                .map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, "    return {{").map_err(Self::fmt_error_to_generator_error)?;
        }

        for (i, slot_name) in direct_slots.iter().enumerate() {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.multivalued.unwrap_or(false) {
                    write!(&mut output, "      {slot_name}: [...this.{slot_name}]")
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(&mut output, "      {slot_name}: this.{slot_name}")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
                if i < direct_slots.len() - 1 {
                    writeln!(&mut output, ",").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output, "    }};").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate toJSON method
        writeln!(&mut output, "  /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * Convert to JSON string")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   * @returns {{string}}")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "   */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  toJSON() {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    return JSON.stringify(this.toObject());")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "  }}").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate validation method
    fn generate_validation_method(
        output: &mut String,
        slots: &[String],
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "  #validate(data) {{").map_err(Self::fmt_error_to_generator_error)?;

        let mut has_validations = false;

        for slot_name in slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Required field validation
                if slot.required.unwrap_or(false) {
                    writeln!(
                        output,
                        "    if (!data.{} || typeof data.{} !== '{}') {{",
                        slot_name,
                        slot_name,
                        Self::get_js_type_check(slot.range.as_ref())
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "      throw new TypeError('{} must be a non-empty {}');",
                        slot_name,
                        Self::get_js_type_name(slot.range.as_ref())
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                    has_validations = true;
                }

                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    writeln!(
                        output,
                        "    if (data.{slot_name} && !/{pattern}/u.test(data.{slot_name})) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "      throw new TypeError('{slot_name} does not match pattern: {pattern}');"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                    has_validations = true;
                }

                // Range validation
                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    if let Some(ref min) = slot.minimum_value {
                        writeln!(
                            output,
                            "    if (typeof data.{slot_name} === 'number' && data.{slot_name} < {min}) {{"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "      throw new RangeError('{slot_name} must be >= {min}');"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                        has_validations = true;
                    }
                    if let Some(ref max) = slot.maximum_value {
                        writeln!(
                            output,
                            "    if (typeof data.{slot_name} === 'number' && data.{slot_name} > {max}) {{"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            output,
                            "      throw new RangeError('{slot_name} must be <= {max}');"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                        has_validations = true;
                    }
                }

                // Array validation
                if slot.multivalued.unwrap_or(false) {
                    writeln!(
                        output,
                        "    if (data.{slot_name} && !Array.isArray(data.{slot_name})) {{"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "      throw new TypeError('{slot_name} must be an array');"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
                    has_validations = true;
                }
            }
        }

        if !has_validations {
            writeln!(output, "    // No validation required")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Get `JSDoc` type annotation
    fn get_jsdoc_type(slot: &SlotDefinition, schema: &SchemaDefinition) -> String {
        let base_type = if !slot.permissible_values.is_empty() {
            "string".to_string()
        } else if let Some(ref range) = slot.range {
            if schema.classes.contains_key(range) {
                range.clone()
            } else {
                TypeMapper::to_javascript(range).to_string()
            }
        } else {
            "*".to_string()
        };

        if slot.multivalued.unwrap_or(false) {
            format!("{base_type}[]")
        } else {
            base_type
        }
    }

    /// Get JavaScript typeof check string
    fn get_js_type_check(range: Option<&String>) -> &'static str {
        if let Some(r) = range {
            match r.as_str() {
                "string" | "str" => "string",
                "integer" | "int" | "float" | "double" | "decimal" => "number",
                "boolean" | "bool" => "boolean",
                _ => "object",
            }
        } else {
            "object"
        }
    }

    /// Get JavaScript type name for error messages
    fn get_js_type_name(range: Option<&String>) -> &'static str {
        if let Some(r) = range {
            match r.as_str() {
                "string" | "str" => "string",
                "integer" | "int" | "float" | "double" | "decimal" => "number",
                "boolean" | "bool" => "boolean",
                _ => "object",
            }
        } else {
            "value"
        }
    }

    /// Generate enum constants
    fn generate_enum(
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);

        writeln!(output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        if let Some(ref desc) = slot.description {
            writeln!(output, " * {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(output, " * @readonly").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " * @enum {{string}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, " */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "export const {enum_name} = Object.freeze({{")
            .map_err(Self::fmt_error_to_generator_error)?;

        for value in &slot.permissible_values {
            match value {
                PermissibleValue::Simple(text) | PermissibleValue::Complex { text, .. } => {
                    let const_name = text.to_uppercase().replace([' ', '-'], "_");
                    writeln!(output, "  {const_name}: \"{text}\",")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output, "}});").map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }
}

impl Generator for JavaScriptGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["js", "mjs"]
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        self.validate_schema(schema)?;

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

        // Use strict mode
        writeln!(&mut content, "'use strict';").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;

        // Generate enums first
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                Self::generate_enum(&mut content, slot_name, slot)?;
                writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Generate classes
        for (class_name, class_def) in &schema.classes {
            let class_code =
                Self::generate_class(class_name, class_def, schema, &GeneratorOptions::default())?;
            content.push_str(&class_code);
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate CommonJS exports (default to ESM)
        if false {
            // Default to ESM module type
            writeln!(&mut content, "// CommonJS exports")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut content,
                "if (typeof module !== 'undefined' && module.exports) {{"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // Export enums
            for (slot_name, slot) in &schema.slots {
                if !slot.permissible_values.is_empty() {
                    let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);
                    writeln!(&mut content, "  module.exports.{enum_name} = {enum_name};")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Export classes
            for class_name in schema.classes.keys() {
                writeln!(
                    &mut content,
                    "  module.exports.{class_name} = {class_name};"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(&mut content, "}}").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "js"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        if schema.name.is_empty() {
            return Err(LinkMLError::service("Schema must have a name".to_string()));
        }

        if schema.classes.is_empty() {
            return Err(LinkMLError::service(
                "Schema must have at least one class".to_string(),
            ));
        }

        Ok(())
    }
}

impl CodeFormatter for JavaScriptGenerator {
    fn name(&self) -> &'static str {
        "javascript"
    }

    fn description(&self) -> &'static str {
        "Code formatter for javascript output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["js", "mjs"]
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
        // JavaScript identifiers are typically camelCase
        BaseCodeFormatter::to_camel_case(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_basic_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut schema = SchemaDefinition::default();
        schema.name = "test_schema".to_string();

        let mut person_class = ClassDefinition::default();
        person_class.name = "Person".to_string();
        person_class.description = Some("A person".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];

        schema.classes.insert("Person".to_string(), person_class);

        let mut name_slot = SlotDefinition::default();
        name_slot.name = "name".to_string();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);

        let mut age_slot = SlotDefinition::default();
        age_slot.name = "age".to_string();
        age_slot.range = Some("integer".to_string());

        schema.slots.insert("name".to_string(), name_slot);
        schema.slots.insert("age".to_string(), age_slot);

        let generator = JavaScriptGenerator::new();

        let output = generator
            .generate(&schema)
            .expect("should generate JavaScript: {}");
        assert!(output.contains("export class Person"));
        assert!(output.contains("constructor(data = {})"));
        assert!(output.contains("#validate(data)"));
        assert!(output.contains("static fromJSON(json)"));
        assert!(output.contains("toObject()"));
        assert!(output.contains("toJSON()"));
        Ok(())
    }
}
