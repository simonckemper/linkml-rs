//! Python dataclass code generator for `LinkML` schemas

use super::base::{
    BaseCodeFormatter, ImportManager, TypeMapper, collect_all_slots, get_default_value_str,
    is_optional_slot,
};
use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};
use linkml_core::prelude::*;
use std::fmt::Write;

/// Python dataclass generator
pub struct PythonDataclassGenerator {
    name: String,
    description: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for PythonDataclassGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonDataclassGenerator {
    /// Create a new Python dataclass generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "python-dataclass".to_string(),
            description: "Generate Python dataclasses from LinkML schemas".to_string(),
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

    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(format!("Formatting error: {err}")))
    }

    /// Check if a string is a valid Python identifier
    fn is_valid_python_identifier(name: &str) -> bool {
        if name.is_empty() {
            return false;
        }

        // Must start with letter or underscore
        let mut chars = name.chars();
        if let Some(first) = chars.next()
            && !first.is_ascii_alphabetic()
            && first != '_'
        {
            return false;
        }

        // Rest must be letters, numbers, or underscores
        for ch in chars {
            if !ch.is_ascii_alphanumeric() && ch != '_' {
                return false;
            }
        }

        // Check it's not a Python keyword
        !matches!(
            name,
            "False"
                | "None"
                | "True"
                | "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "try"
                | "while"
                | "with"
                | "yield"
        )
    }

    /// Generate code for a single class
    fn generate_class(
        &self,
        class_name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let mut imports = ImportManager::new();

        // Always need dataclass
        imports.add_import("dataclasses", "dataclass");

        // Generate class documentation
        if options.include_docs
            && (class.description.is_some()
                || options
                    .get_custom("include_examples")
                    .is_some_and(|v| v == "true"))
        {
            writeln!(&mut output, "@dataclass").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "class {class_name}:")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;

            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 72, "    ");
                writeln!(&mut output, "    {wrapped}")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            if options
                .get_custom("include_examples")
                .is_some_and(|v| v == "true")
            {
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    Examples:")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        >>> person = {class_name}(")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ...     name=\"John Doe\",")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ...     age=30")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ... )")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, "@dataclass").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "class {class_name}:")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Collect all slots including inherited
        let slots = collect_all_slots(class, schema)?;

        if slots.is_empty() {
            writeln!(&mut output, "    pass").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            // Generate fields
            for slot_name in &slots {
                if let Some(slot) = schema.slots.get(slot_name) {
                    self.generate_field(
                        &mut output,
                        slot_name,
                        slot,
                        schema,
                        &mut imports,
                        options,
                    )?;
                }
            }

            // Generate __post_init__ if we need validation
            if options
                .get_custom("generate_validation")
                .map(std::string::String::as_str)
                == Some("true")
            {
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                Self::generate_post_init(&mut output, &slots, schema, &options.indent)?;
            }
        }

        // Add field import if needed
        if output.contains("field(") {
            imports.add_import("dataclasses", "field");
        }

        // Combine imports and class
        let mut final_output = String::new();
        let import_block = imports.python_imports();
        if !import_block.is_empty() {
            writeln!(&mut final_output, "{import_block}")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut final_output).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut final_output).map_err(Self::fmt_error_to_generator_error)?;
        }
        final_output.push_str(&output);

        Ok(final_output)
    }

    /// Generate a single field with advanced type annotations and validation
    fn generate_field(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        imports: &mut ImportManager,
        options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        let _indent = options.indent.single();

        // Add field documentation
        if options.include_docs
            && let Some(ref desc) = slot.description
        {
            writeln!(output, "    # {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine the type
        let base_type = Self::get_field_type(slot, schema, imports);

        // Handle optional and multivalued with advanced type annotations
        let field_type = if slot.multivalued.unwrap_or(false) {
            // Use more specific collection types based on constraints
            if !slot.unique_keys.is_empty() || slot.unique.unwrap_or(false) {
                imports.add_import("typing", "Set");
                format!("Set[{base_type}]")
            } else if slot.ordered.unwrap_or(false) {
                imports.add_import("typing", "List");
                format!("List[{base_type}]")
            } else {
                imports.add_import("typing", "Sequence");
                format!("Sequence[{base_type}]")
            }
        } else {
            base_type
        };

        let final_type = if is_optional_slot(slot) && !slot.multivalued.unwrap_or(false) {
            imports.add_import("typing", "Optional");
            format!("Optional[{field_type}]")
        } else {
            field_type
        };

        // Get default value
        let default_str = get_default_value_str(slot, "python");

        // Write the field
        write!(output, "    {slot_name}: {final_type}")
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(default) = default_str {
            write!(output, " = {default}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get the Python type for a field
    fn get_field_type(
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        imports: &mut ImportManager,
    ) -> String {
        // Check if it's an enum
        if !slot.permissible_values.is_empty() {
            imports.add_import("enum", "Enum");
            let enum_name = BaseCodeFormatter::to_pascal_case(&slot.name);
            return enum_name;
        }

        // Check range
        if let Some(ref range) = slot.range {
            // Check if it's a class
            if schema.classes.contains_key(range) {
                return range.clone();
            }

            // Check if it's a type
            if let Some(type_def) = schema.types.get(range)
                && let Some(ref base_type) = type_def.base_type
            {
                return TypeMapper::to_python(base_type).to_string();
            }

            // Otherwise map as primitive
            TypeMapper::to_python(range).to_string()
        } else {
            "Any".to_string()
        }
    }

    /// Generate __`post_init`__ method for validation
    fn generate_post_init(
        output: &mut String,
        slots: &[String],
        schema: &SchemaDefinition,
        _indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(output, "    def __post_init__(self):")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            output,
            "        \"\"\"Validate fields after initialization.\"\"\""
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        let mut has_validation = false;

        for slot_name in slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    if !has_validation {
                        writeln!(output, "        import re")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        has_validation = true;
                    }
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{slot_name} is not None and not re.match(r\"{pattern}\", self.{slot_name}):"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{slot_name} does not match pattern: {pattern}\")"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Range validation for numbers
                if let Some(min) = &slot.minimum_value {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{slot_name} is not None and self.{slot_name} < {min}:"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{slot_name} must be >= {min}\")"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    has_validation = true;
                }

                if let Some(max) = &slot.maximum_value {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{slot_name} is not None and self.{slot_name} > {max}:"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{slot_name} must be <= {max}\")"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    has_validation = true;
                }

                // Add enum validation
                if !slot.permissible_values.is_empty() {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
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
                        "        if self.{} is not None and self.{} not in [{}]:",
                        slot_name,
                        slot_name,
                        values.join(", ")
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{} must be one of: {}\")",
                        slot_name,
                        values.join(", ")
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    has_validation = true;
                }

                // Add required field validation
                if slot.required.unwrap_or(false) {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        if self.{slot_name} is None:")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(\"{slot_name} is required\")"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    has_validation = true;
                }
            }
        }

        if !has_validation {
            writeln!(output, "        pass").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }
}

impl Generator for PythonDataclassGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["py"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Validate schema has required fields for Python generation
        if schema.name.is_empty() {
            return Err(LinkMLError::SchemaValidationError {
                message: "Schema must have a name for Python generation".to_string(),
                element: Some("schema.name".to_string()),
            });
        }

        // Validate classes have valid Python identifiers
        for (class_name, _class_def) in &schema.classes {
            if !Self::is_valid_python_identifier(class_name) {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!("Class name '{class_name}' is not a valid Python identifier"),
                    element: Some(format!("class.{class_name}")),
                });
            }
        }

        // Validate slots have valid Python identifiers
        for (slot_name, _slot_def) in &schema.slots {
            if !Self::is_valid_python_identifier(slot_name) {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!("Slot name '{slot_name}' is not a valid Python identifier"),
                    element: Some(format!("slot.{slot_name}")),
                });
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> Result<String> {
        self.validate_schema(schema)?;

        // Generate a single file with all classes
        let mut content = String::new();
        let mut imports = ImportManager::new();

        // File header
        writeln!(&mut content, "\"\"\"").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut content,
            "Generated from LinkML schema: {}",
            schema.name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(ref desc) = schema.description {
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "{desc}").map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut content, "\"\"\"").map_err(Self::fmt_error_to_generator_error)?;

        // Generate enums first
        let mut enum_content = String::new();
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                self.generate_enum(&mut enum_content, slot_name, slot, &mut imports)?;
            }
        }

        // Generate classes
        let mut class_content = String::new();
        for (class_name, class_def) in &schema.classes {
            let class_code =
                self.generate_class(class_name, class_def, schema, &GeneratorOptions::default())?;
            writeln!(&mut class_content).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut class_content).map_err(Self::fmt_error_to_generator_error)?;
            class_content.push_str(&class_code);
        }

        // Combine everything
        let mut final_content = String::new();

        // Imports
        let import_block = imports.python_imports();
        if !import_block.is_empty() {
            writeln!(&mut final_content, "{import_block}")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add generated content marker
        writeln!(
            &mut final_content,
            "# Generated by LinkML Python Dataclass Generator"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;

        // Enums
        if !enum_content.is_empty() {
            final_content.push_str(&enum_content);
            writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Classes
        final_content.push_str(&class_content);

        Ok(final_content)
    }

    fn get_file_extension(&self) -> &'static str {
        "py"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema_dataclass"
    }
}

impl PythonDataclassGenerator {
    /// Generate an enum from permissible values
    fn generate_enum(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        imports: &mut ImportManager,
    ) -> GeneratorResult<()> {
        imports.add_import("enum", "Enum");

        let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "class {enum_name}(Enum):").map_err(Self::fmt_error_to_generator_error)?;

        if let Some(ref desc) = slot.description {
            writeln!(output, "    \"\"\"{desc}\"\"\"")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        for value in &slot.permissible_values {
            match value {
                PermissibleValue::Simple(text) | PermissibleValue::Complex { text, .. } => {
                    let const_name = text.to_uppercase().replace([' ', '-'], "_");
                    writeln!(output, "    {const_name} = \"{text}\"")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }
}

impl CodeFormatter for PythonDataclassGenerator {
    fn name(&self) -> &'static str {
        "python-dataclass-formatter"
    }

    fn description(&self) -> &'static str {
        "Python dataclass code formatter with docstring and type hint support"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["py"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // Format Python code with proper indentation
        let mut formatted = String::new();
        let mut indent_level = 0;
        let indent = "    ";

        for line in code.lines() {
            let trimmed = line.trim();

            // Decrease indent for dedent keywords
            if trimmed == "pass" || trimmed.starts_with("return") || trimmed.starts_with("raise") {
                // Keep current indent
            } else if trimmed.ends_with(':') && !trimmed.starts_with('#') {
                // Line that increases indent (class, def, if, for, etc.)
                formatted.push_str(&indent.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
                indent_level += 1;
                continue;
            } else if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            } else if indent_level > 0 && !trimmed.starts_with(' ') && !trimmed.starts_with('#') {
                // Dedent if we're not at the start of a block
                let first_word = trimmed.split_whitespace().next().unwrap_or("");
                if matches!(
                    first_word,
                    "class"
                        | "def"
                        | "if"
                        | "elif"
                        | "else"
                        | "for"
                        | "while"
                        | "with"
                        | "try"
                        | "except"
                        | "finally"
                ) {
                    indent_level = indent_level.saturating_sub(1);
                }
            }

            // Write the line with proper indentation
            if !trimmed.is_empty() {
                formatted.push_str(&indent.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
            }
        }

        Ok(formatted)
    }

    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let indent_str = indent.to_string(level);
        let lines: Vec<&str> = doc.lines().collect();

        if lines.len() == 1 {
            format!("{}\"\"\"{}\"\"\"", indent_str, lines[0])
        } else {
            let mut result = format!("{indent_str}\"\"\"");
            for line in lines {
                result.push('\n');
                result.push_str(&indent_str);
                result.push_str(line);
            }
            result.push('\n');
            result.push_str(&indent_str);
            result.push_str("\"\"\"");
            result
        }
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
            .map(|item| {
                let indent_str = indent.to_string(level);
                let item_str = item.as_ref();
                format!("{indent_str}{item_str}")
            })
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        BaseCodeFormatter::escape_python_string(s)
    }

    fn convert_identifier(&self, id: &str) -> String {
        // Python identifiers should be snake_case
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_basic_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
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

        let generator = PythonDataclassGenerator::new();

        let output = generator
            .generate(&schema)
            .expect("should generate dataclass output");
        assert!(output.contains("@dataclass"));
        assert!(output.contains("class Person:"));
        assert!(output.contains("name: str"));
        assert!(output.contains("age: Optional[int] = None"));
        Ok(())
    }
}
