//! Python dataclass code generator for LinkML schemas

use super::base::{
    collect_all_slots, get_default_value_str, is_optional_slot, BaseCodeFormatter, ImportManager,
    TypeMapper,
};
use super::traits::{
    CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use super::options::{GeneratorOptions, IndentStyle};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// Python dataclass generator
pub struct PythonDataclassGenerator {
    name: String,
    description: String,
}

impl Default for PythonDataclassGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonDataclassGenerator {
    /// Create a new Python dataclass generator
    pub fn new() -> Self {
        Self {
            name: "python-dataclass".to_string(),
            description: "Generate Python dataclasses from LinkML schemas".to_string(),
        }
    }

    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(err: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Formatting error: {}", err),
        ))
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
        if options.include_docs && (class.description.is_some() || options.include_examples) {
            writeln!(&mut output, "@dataclass").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "class {}:", class_name).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;
            
            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 72, "    ");
                writeln!(&mut output, "    {}", wrapped).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            if options.include_examples {
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    Examples:").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        >>> person = {}(", class_name).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ...     name=\"John Doe\",").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ...     age=30").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        ... )").map_err(Self::fmt_error_to_generator_error)?;
            }
            
            writeln!(&mut output, "    \"\"\"").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, "@dataclass").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "class {}:", class_name).map_err(Self::fmt_error_to_generator_error)?;
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
            if options.get_custom("generate_validation") == Some("true") {
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                self.generate_post_init(&mut output, &slots, schema, &options.indent)?;
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
            writeln!(&mut final_output, "{}", import_block).map_err(Self::fmt_error_to_generator_error)?;
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
        let _indent = options.indent.single();

        // Add field documentation
        if options.include_docs {
            if let Some(ref desc) = slot.description {
                writeln!(output, "    # {}", desc).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Determine the type
        let base_type = self.get_field_type(slot, schema, imports)?;
        
        // Handle optional and multivalued
        let field_type = if slot.multivalued.unwrap_or(false) {
            imports.add_import("typing", "List");
            format!("List[{}]", base_type)
        } else {
            base_type
        };

        let final_type = if is_optional_slot(slot) && !slot.multivalued.unwrap_or(false) {
            imports.add_import("typing", "Optional");
            format!("Optional[{}]", field_type)
        } else {
            field_type
        };

        // Get default value
        let default_str = get_default_value_str(slot, "python");

        // Write the field
        write!(output, "{}    {}: {}", "", slot_name, final_type).map_err(Self::fmt_error_to_generator_error)?;
        
        if let Some(default) = default_str {
            write!(output, " = {}", default).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Get the Python type for a field
    fn get_field_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        imports: &mut ImportManager,
    ) -> GeneratorResult<String> {
        // Check if it's an enum
        if !slot.permissible_values.is_empty() {
            imports.add_import("enum", "Enum");
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
            if let Some(type_def) = schema.types.get(range) {
                if let Some(ref base_type) = type_def.base_type {
                    return Ok(TypeMapper::to_python(base_type).to_string());
                }
            }

            // Otherwise map as primitive
            Ok(TypeMapper::to_python(range).to_string())
        } else {
            Ok("Any".to_string())
        }
    }

    /// Generate __post_init__ method for validation
    fn generate_post_init(
        &self,
        output: &mut String,
        slots: &[String],
        schema: &SchemaDefinition,
        _indent: &IndentStyle,
    ) -> GeneratorResult<()> {
        writeln!(output, "    def __post_init__(self):").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        \"\"\"Validate fields after initialization.\"\"\"").map_err(Self::fmt_error_to_generator_error)?;

        let mut has_validation = false;

        for slot_name in slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    if !has_validation {
                        writeln!(output, "        import re").map_err(Self::fmt_error_to_generator_error)?;
                        has_validation = true;
                    }
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{} is not None and not re.match(r\"{}\", self.{}):",
                        slot_name, pattern, slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{} does not match pattern: {}\")",
                        slot_name, pattern
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Range validation for numbers
                if let Some(min) = &slot.minimum_value {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{} is not None and self.{} < {}:",
                        slot_name, slot_name, min
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{} must be >= {}\")",
                        slot_name, min
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    has_validation = true;
                }

                if let Some(max) = &slot.maximum_value {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "        if self.{} is not None and self.{} > {}:",
                        slot_name, slot_name, max
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "            raise ValueError(f\"{} must be <= {}\")",
                        slot_name, max
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

#[async_trait]
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

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        self.validate_schema(schema).await?;

        let mut outputs = Vec::new();

        // Generate a single file with all classes
        let mut content = String::new();
        let mut imports = ImportManager::new();

        // File header
        writeln!(&mut content, "\"\"\"").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut content, "Generated from LinkML schema: {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        if let Some(ref desc) = schema.description {
            writeln!(&mut content).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut content, "{}", desc).map_err(Self::fmt_error_to_generator_error)?;
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
            let class_code = self.generate_class(class_name, class_def, schema, options)?;
            writeln!(&mut class_content).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut class_content).map_err(Self::fmt_error_to_generator_error)?;
            class_content.push_str(&class_code);
        }

        // Combine everything
        let mut final_content = String::new();
        
        // Imports
        let import_block = imports.python_imports();
        if !import_block.is_empty() {
            writeln!(&mut final_content, "{}", import_block).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Add generated content marker
        writeln!(&mut final_content, "# Generated by LinkML Python Dataclass Generator").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;

        // Enums
        if !enum_content.is_empty() {
            final_content.push_str(&enum_content);
            writeln!(&mut final_content).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Classes
        final_content.push_str(&class_content);

        outputs.push(GeneratedOutput {
            content: final_content,
            filename: format!("{}.py", schema.name.to_lowercase().replace('-', "_")),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("generator".to_string(), self.name.clone());
                meta.insert("schema".to_string(), schema.name.clone());
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
        writeln!(output, "class {}(Enum):", enum_name).map_err(Self::fmt_error_to_generator_error)?;
        
        if let Some(ref desc) = slot.description {
            writeln!(output, "    \"\"\"{}\"\"\"", desc).map_err(Self::fmt_error_to_generator_error)?;
        }

        for value in &slot.permissible_values {
            match value {
                PermissibleValue::Simple(text) => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    writeln!(output, "    {} = \"{}\"", const_name, text).map_err(Self::fmt_error_to_generator_error)?;
                }
                PermissibleValue::Complex { text, .. } => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    writeln!(output, "    {} = \"{}\"", const_name, text).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }
}

impl CodeFormatter for PythonDataclassGenerator {
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let indent_str = indent.to_string(level);
        let lines: Vec<&str> = doc.lines().collect();
        
        if lines.len() == 1 {
            format!("{}\"\"\"{}\"\"\"", indent_str, lines[0])
        } else {
            let mut result = format!("{}\"\"\"", indent_str);
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

    #[tokio::test]
    async fn test_basic_generation() {
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

        let generator = PythonDataclassGenerator::new();
        let options = GeneratorOptions::new();

        let outputs = generator.generate(&schema, &options).await.map_err(Self::fmt_error_to_generator_error)?;
        assert_eq!(outputs.len(), 1);

        let output = &outputs[0];
        assert!(output.content.contains("@dataclass"));
        assert!(output.content.contains("class Person:"));
        assert!(output.content.contains("name: str"));
        assert!(output.content.contains("age: Optional[int] = None"));
    }
}