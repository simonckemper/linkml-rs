//! Pydantic v2 code generator for `LinkML` schemas

use super::base::{
    BaseCodeFormatter, ImportManager, TypeMapper, collect_all_slots, is_optional_slot,
};
use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, Generator, GeneratorError, GeneratorResult};
use linkml_core::error::LinkMLError;
use linkml_core::prelude::*;
use std::fmt::Write;

/// Pydantic v2 generator
pub struct PydanticGenerator {
    name: String,
    description: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for PydanticGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PydanticGenerator {
    /// Create a new Pydantic generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "pydantic".to_string(),
            description: "Generate Pydantic v2 models from LinkML schemas".to_string(),
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
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
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

        // Always need BaseModel
        imports.add_import("pydantic", "BaseModel");

        // Check if we need inheritance
        let base_class = if let Some(ref parent) = class.is_a {
            parent.clone()
        } else {
            "BaseModel".to_string()
        };

        // Generate class definition
        writeln!(&mut output, "class {class_name}({base_class}):")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Generate class documentation
        if options.include_docs && class.description.is_some() {
            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;
            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 72, "    ");
                writeln!(&mut output, "    {wrapped}")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate model config
        writeln!(&mut output, "    model_config = {{")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        \"validate_assignment\": True,")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        \"use_enum_values\": True,")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        \"str_strip_whitespace\": True,")
            .map_err(Self::fmt_error_to_generator_error)?;

        if options
            .get_custom("include_examples")
            .is_some_and(|v| v == "true")
        {
            writeln!(&mut output, "        \"json_schema_extra\": {{")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            \"examples\": [")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "                {{")
                .map_err(Self::fmt_error_to_generator_error)?;

            // Generate example values for slots
            let slots = collect_all_slots(class, schema)?;
            for (i, slot_name) in slots.iter().enumerate() {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let example_value = self.get_example_value(slot);
                    write!(
                        &mut output,
                        "                    \"{slot_name}\": {example_value}"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    if i < slots.len() - 1 {
                        writeln!(&mut output, ",").map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }

            writeln!(&mut output, "                }}")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            ]").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

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
                    writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Generate validators if needed
            if options
                .get_custom("generate_validators")
                .map(std::string::String::as_str)
                == Some("true")
            {
                self.generate_validators(&mut output, &slots, schema, &mut imports)?;
            }
        }

        // Add Field import if needed
        if output.contains("Field(") {
            imports.add_import("pydantic", "Field");
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

    /// Generate a single field
    fn generate_field(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        imports: &mut ImportManager,
        options: &GeneratorOptions,
    ) -> GeneratorResult<()> {
        // Add field documentation as inline comment
        if options.include_docs
            && let Some(ref desc) = slot.description
        {
            writeln!(output, "    # {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Determine the type
        let base_type = self.get_field_type(slot, schema, imports);

        // Handle optional and multivalued
        let field_type = if slot.multivalued.unwrap_or(false) {
            imports.add_import("typing", "List");
            format!("List[{base_type}]")
        } else {
            base_type
        };

        let final_type = if is_optional_slot(slot) && !slot.multivalued.unwrap_or(false) {
            imports.add_import("typing", "Optional");
            format!("Optional[{field_type}]")
        } else {
            field_type
        };

        // Build Field arguments
        let mut field_args = Vec::new();

        // Required fields need ...
        if slot.required.unwrap_or(false) {
            field_args.push("...".to_string());
        } else if slot.multivalued.unwrap_or(false) {
            field_args.push("default_factory=list".to_string());
        } else {
            field_args.push("None".to_string());
        }

        // Add description
        if let Some(ref desc) = slot.description {
            field_args.push(format!(
                "description=\"{}\"",
                BaseCodeFormatter::escape_python_string(desc)
            ));
        }

        // Add pattern
        if let Some(ref pattern) = slot.pattern {
            field_args.push(format!("pattern=r\"{pattern}\""));
        }

        // Add numeric constraints
        if let Some(ref min) = slot.minimum_value {
            field_args.push(format!("ge={min}"));
        }
        if let Some(ref max) = slot.maximum_value {
            field_args.push(format!("le={max}"));
        }

        // Note: LinkML doesn't have minimum_cardinality/maximum_cardinality in SlotDefinition
        // These would be handled by pattern or custom validators if needed

        // Write the field
        write!(output, "    {slot_name}: {final_type} = Field(")
            .map_err(Self::fmt_error_to_generator_error)?;
        write!(output, "{}", field_args.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, ")").map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get the Python type for a field
    fn get_field_type(
        &self,
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
                let py_type = TypeMapper::to_python(base_type);
                // Add datetime imports if needed
                match base_type.as_str() {
                    "datetime" => {
                        imports.add_import("datetime", "datetime");
                    }
                    "date" => {
                        imports.add_import("datetime", "date");
                    }
                    "time" => {
                        imports.add_import("datetime", "time");
                    }
                    _ => {}
                }
                return py_type.to_string();
            }

            // Otherwise map as primitive
            let py_type = TypeMapper::to_python(range);

            // Add datetime imports if needed
            match range.as_str() {
                "datetime" => {
                    imports.add_import("datetime", "datetime");
                }
                "date" => {
                    imports.add_import("datetime", "date");
                }
                "time" => {
                    imports.add_import("datetime", "time");
                }
                _ => {}
            }

            if py_type == "Any" {
                imports.add_import("typing", "Any");
            }
            py_type.to_string()
        } else {
            imports.add_import("typing", "Any");
            "Any".to_string()
        }
    }

    /// Generate example value for a slot
    fn get_example_value(&self, slot: &SlotDefinition) -> String {
        if let Some(ref range) = slot.range {
            match range.as_str() {
                "string" | "str" => "\"Example text\"".to_string(),
                "integer" | "int" => "42".to_string(),
                "float" | "double" | "decimal" => "3.14".to_string(),
                "boolean" | "bool" => "true".to_string(),
                "date" => "\"2024-01-01\"".to_string(),
                "datetime" => "\"2024-01-01T12:00:00Z\"".to_string(),
                _ => {
                    if slot.permissible_values.is_empty() {
                        "null".to_string()
                    } else if let Some(PermissibleValue::Simple(val)) =
                        slot.permissible_values.first()
                    {
                        format!("\"{val}\"")
                    } else {
                        "null".to_string()
                    }
                }
            }
        } else {
            "null".to_string()
        }
    }

    /// Generate validators for slots that need them
    fn generate_validators(
        &self,
        output: &mut String,
        slots: &[String],
        schema: &SchemaDefinition,
        imports: &mut ImportManager,
    ) -> GeneratorResult<()> {
        imports.add_import("pydantic", "field_validator");

        let mut validators_added = false;

        for slot_name in slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Add custom validators for complex constraints
                if slot.minimum_value.is_some() && slot.maximum_value.is_some() {
                    if !validators_added {
                        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    }
                    validators_added = true;

                    writeln!(output, "    @field_validator('{slot_name}', mode='after')")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    @classmethod")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "    def validate_{slot_name}(cls, v):")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "        if v is None:")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "            return v")
                        .map_err(Self::fmt_error_to_generator_error)?;

                    // Add custom validation logic
                    if let Some(ref desc) = slot.description {
                        writeln!(output, "        # {desc}")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    writeln!(output, "        return v")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }
}

impl Generator for PydanticGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["py"]
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
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
            "# Generated by LinkML Pydantic Generator"
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

    fn get_file_extension(&self) -> &'static str {
        "py"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema_pydantic"
    }
}

impl PydanticGenerator {
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
        writeln!(output, "class {enum_name}(str, Enum):")
            .map_err(Self::fmt_error_to_generator_error)?;

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

impl CodeFormatter for PydanticGenerator {
    fn name(&self) -> &'static str {
        "pydantic"
    }

    fn description(&self) -> &'static str {
        "Code formatter for pydantic output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["py"]
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
            .map(|item| format!("{}{}", indent.to_string(level), item.as_ref()))
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
    use indexmap::IndexMap;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_basic_generation() {
        let person_class = ClassDefinition {
            name: "Person".to_string(),
            description: Some("A person".to_string()),
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);

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

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);

        let schema = SchemaDefinition {
            name: "test_schema".to_string(),
            classes,
            slots,
            ..Default::default()
        };

        let generator = PydanticGenerator::new();

        let output = generator
            .generate(&schema)
            .expect("should generate Pydantic output");
        assert!(output.contains("from pydantic import BaseModel"));
        assert!(output.contains("class Person(BaseModel):"));
        assert!(output.contains("name: str = Field(...)"));
        assert!(output.contains("age: Optional[int] = Field(None)"));
        assert!(output.contains("model_config ="));
    }
}
