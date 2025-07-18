//! TypeScript code generator for LinkML schemas

use super::base::{
    collect_all_slots, is_optional_slot, BaseCodeFormatter, TypeMapper,
};
use super::traits::{
    CodeFormatter, GeneratedOutput, Generator, GeneratorError, GeneratorResult,
};
use super::options::{GeneratorOptions, IndentStyle};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// TypeScript generator
pub struct TypeScriptGenerator {
    name: String,
    description: String,
}

impl Default for TypeScriptGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptGenerator {
    /// Create a new TypeScript generator
    pub fn new() -> Self {
        Self {
            name: "typescript".to_string(),
            description: "Generate TypeScript interfaces and types from LinkML schemas".to_string(),
        }
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
            writeln!(&mut output, "/**").unwrap();
            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 70, " * ");
                writeln!(&mut output, " * {}", wrapped).unwrap();
            }
            writeln!(&mut output, " * @generated from LinkML schema").unwrap();
            writeln!(&mut output, " */").unwrap();
        }

        // Check if we have inheritance
        let extends_clause = if let Some(ref parent) = class.is_a {
            format!(" extends {}", parent)
        } else {
            String::new()
        };

        writeln!(&mut output, "export interface {}{} {{", class_name, extends_clause).unwrap();

        // Collect all slots including inherited
        let slots = collect_all_slots(class, schema)?;

        // Generate fields
        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Skip if this slot is from parent
                if let Some(ref parent) = class.is_a {
                    if let Some(parent_class) = schema.classes.get(parent) {
                        let parent_slots = collect_all_slots(parent_class, schema)?;
                        if parent_slots.contains(slot_name) {
                            continue;
                        }
                    }
                }

                self.generate_field(
                    &mut output,
                    slot_name,
                    slot,
                    schema,
                    options,
                )?;
            }
        }

        writeln!(&mut output, "}}").unwrap();

        // Generate type guard
        if options.get_custom("generate_type_guards") != Some("false") {
            writeln!(&mut output).unwrap();
            self.generate_type_guard(&mut output, class_name, class, schema)?;
        }

        // Generate validator function
        if options.get_custom("generate_validators") == Some("true") {
            writeln!(&mut output).unwrap();
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
            writeln!(output, "  /**").unwrap();
            if let Some(ref desc) = slot.description {
                writeln!(output, "   * {}", desc).unwrap();
            }
            writeln!(output, "   */").unwrap();
        }

        // Determine the type
        let base_type = self.get_field_type(slot, schema)?;
        
        // Handle multivalued
        let field_type = if slot.multivalued.unwrap_or(false) {
            format!("{}[]", base_type)
        } else {
            base_type
        };

        // Handle optional
        let optional_marker = if is_optional_slot(slot) {
            "?"
        } else {
            ""
        };

        writeln!(output, "  {}{}: {};", slot_name, optional_marker, field_type).unwrap();

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
            if let Some(type_def) = schema.types.get(range) {
                if let Some(ref base_type) = type_def.base_type {
                    return Ok(TypeMapper::to_typescript(base_type).to_string());
                }
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
        writeln!(output, "/**").unwrap();
        writeln!(output, " * Type guard for {}", class_name).unwrap();
        writeln!(output, " */").unwrap();
        writeln!(output, "export function is{}(obj: unknown): obj is {} {{", class_name, class_name).unwrap();
        
        writeln!(output, "  return (").unwrap();
        writeln!(output, "    typeof obj === 'object' &&").unwrap();
        writeln!(output, "    obj !== null &&").unwrap();

        // Get direct slots (not inherited)
        let direct_slots = class.slots.clone();
        
        // Check required fields
        for (i, slot_name) in direct_slots.iter().enumerate() {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.required.unwrap_or(false) {
                    write!(output, "    '{}' in obj", slot_name).unwrap();
                    
                    // Add type check
                    let expected_type = match self.get_field_type(slot, schema)?.as_str() {
                        "string" => "string",
                        "number" => "number",
                        "boolean" => "boolean",
                        _ => "object",
                    };
                    
                    if expected_type != "object" {
                        write!(output, " &&").unwrap();
                        writeln!(output).unwrap();
                        write!(output, "    typeof (obj as any).{} === '{}'", slot_name, expected_type).unwrap();
                    }
                    
                    if i < direct_slots.len() - 1 {
                        writeln!(output, " &&").unwrap();
                    } else {
                        writeln!(output).unwrap();
                    }
                }
            }
        }

        writeln!(output, "  );").unwrap();
        writeln!(output, "}}").unwrap();

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
        writeln!(output, "/**").unwrap();
        writeln!(output, " * Validator for {}", class_name).unwrap();
        writeln!(output, " */").unwrap();
        writeln!(output, "export function validate{}(obj: unknown): ValidationResult<{}> {{", class_name, class_name).unwrap();
        writeln!(output, "  const errors: ValidationError[] = [];").unwrap();
        writeln!(output).unwrap();
        
        writeln!(output, "  if (!is{}(obj)) {{", class_name).unwrap();
        writeln!(output, "    errors.push({{ path: '', message: 'Not a valid {} object' }});", class_name).unwrap();
        writeln!(output, "    return {{ valid: false, errors }};").unwrap();
        writeln!(output, "  }}").unwrap();
        writeln!(output).unwrap();

        // Add validation for constraints
        let slots = collect_all_slots(class, schema)?;
        let mut has_validations = false;

        for slot_name in &slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    if !has_validations {
                        writeln!(output, "  // Additional validation").unwrap();
                        has_validations = true;
                    }
                    writeln!(output, "  if (obj.{} && !/{}/u.test(obj.{})) {{", slot_name, pattern, slot_name).unwrap();
                    writeln!(output, "    errors.push({{ path: '{}', message: 'Does not match pattern: {}' }});", slot_name, pattern).unwrap();
                    writeln!(output, "  }}").unwrap();
                }

                // Range validation
                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    if let Some(ref min) = slot.minimum_value {
                        writeln!(output, "  if (obj.{} !== undefined && obj.{} < {}) {{", slot_name, slot_name, min).unwrap();
                        writeln!(output, "    errors.push({{ path: '{}', message: 'Must be >= {}' }});", slot_name, min).unwrap();
                        writeln!(output, "  }}").unwrap();
                    }
                    if let Some(ref max) = slot.maximum_value {
                        writeln!(output, "  if (obj.{} !== undefined && obj.{} > {}) {{", slot_name, slot_name, max).unwrap();
                        writeln!(output, "    errors.push({{ path: '{}', message: 'Must be <= {}' }});", slot_name, max).unwrap();
                        writeln!(output, "  }}").unwrap();
                    }
                }
            }
        }

        writeln!(output).unwrap();
        writeln!(output, "  return errors.length === 0").unwrap();
        writeln!(output, "    ? {{ valid: true, data: obj }}").unwrap();
        writeln!(output, "    : {{ valid: false, errors }};").unwrap();
        writeln!(output, "}}").unwrap();

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
            writeln!(output, "/**").unwrap();
            writeln!(output, " * {}", desc).unwrap();
            writeln!(output, " */").unwrap();
        }
        
        writeln!(output, "export enum {} {{", enum_name).unwrap();

        for (i, value) in slot.permissible_values.iter().enumerate() {
            match value {
                PermissibleValue::Simple(text) => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    write!(output, "  {} = \"{}\"", const_name, text).unwrap();
                }
                PermissibleValue::Complex { text, .. } => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    write!(output, "  {} = \"{}\"", const_name, text).unwrap();
                }
            }
            if i < slot.permissible_values.len() - 1 {
                writeln!(output, ",").unwrap();
            } else {
                writeln!(output).unwrap();
            }
        }

        writeln!(output, "}}").unwrap();
        Ok(())
    }
}

#[async_trait]
impl Generator for TypeScriptGenerator {
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
        self.validate_schema(schema).await?;

        let mut outputs = Vec::new();
        let mut content = String::new();

        // File header
        writeln!(&mut content, "/**").unwrap();
        writeln!(&mut content, " * Generated from LinkML schema: {}", schema.name).unwrap();
        if let Some(ref desc) = schema.description {
            writeln!(&mut content, " * {}", desc).unwrap();
        }
        writeln!(&mut content, " */").unwrap();
        writeln!(&mut content).unwrap();

        // Generate validation types if needed
        if options.get_custom("generate_validators") == Some("true") {
            writeln!(&mut content, "// Validation types").unwrap();
            writeln!(&mut content, "export interface ValidationError {{").unwrap();
            writeln!(&mut content, "  path: string;").unwrap();
            writeln!(&mut content, "  message: string;").unwrap();
            writeln!(&mut content, "}}").unwrap();
            writeln!(&mut content).unwrap();
            writeln!(&mut content, "export type ValidationResult<T> =").unwrap();
            writeln!(&mut content, "  | {{ valid: true; data: T }}").unwrap();
            writeln!(&mut content, "  | {{ valid: false; errors: ValidationError[] }};").unwrap();
            writeln!(&mut content).unwrap();
        }

        // Generate enums first
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                self.generate_enum(&mut content, slot_name, slot)?;
                writeln!(&mut content).unwrap();
            }
        }

        // Generate interfaces
        for (class_name, class_def) in &schema.classes {
            let interface_code = self.generate_interface(class_name, class_def, schema, options)?;
            content.push_str(&interface_code);
            writeln!(&mut content).unwrap();
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
    fn format_doc(&self, doc: &str, indent: &IndentStyle, level: usize) -> String {
        let indent_str = indent.to_string(level);
        let lines: Vec<&str> = doc.lines().collect();
        
        let mut result = format!("{}/**", indent_str);
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

        let generator = TypeScriptGenerator::new();
        let options = GeneratorOptions::new();

        let outputs = generator.generate(&schema, &options).await.unwrap();
        assert_eq!(outputs.len(), 1);

        let output = &outputs[0];
        assert!(output.content.contains("export interface Person"));
        assert!(output.content.contains("name: string;"));
        assert!(output.content.contains("age?: number;"));
        assert!(output.content.contains("export function isPerson"));
    }
}