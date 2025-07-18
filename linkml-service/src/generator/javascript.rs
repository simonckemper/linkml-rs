//! JavaScript (ES6) code generator for LinkML schemas

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

/// JavaScript generator
pub struct JavaScriptGenerator {
    name: String,
    description: String,
}

impl Default for JavaScriptGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptGenerator {
    /// Create a new JavaScript generator
    pub fn new() -> Self {
        Self {
            name: "javascript".to_string(),
            description: "Generate JavaScript ES6 classes from LinkML schemas".to_string(),
        }
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

        // Generate class documentation
        if options.include_docs {
            writeln!(&mut output, "/**").unwrap();
            if let Some(ref desc) = class.description {
                let wrapped = BaseCodeFormatter::wrap_text(desc, 70, " * ");
                writeln!(&mut output, " * {}", wrapped).unwrap();
            }
            writeln!(&mut output, " * @generated from LinkML schema").unwrap();
            writeln!(&mut output, " */").unwrap();
        }

        // Generate class definition
        let extends_clause = if let Some(ref parent) = class.is_a {
            format!(" extends {}", parent)
        } else {
            String::new()
        };

        writeln!(&mut output, "export class {}{} {{", class_name, extends_clause).unwrap();

        // Generate constructor JSDoc
        writeln!(&mut output, "  /**").unwrap();
        writeln!(&mut output, "   * @param {{Object}} data - Initialization data").unwrap();

        // Collect all slots including inherited
        let all_slots = collect_all_slots(class, schema)?;
        
        // Get direct slots only for constructor params
        let direct_slots: Vec<String> = if let Some(ref parent) = class.is_a {
            let parent_slots = if let Some(parent_class) = schema.classes.get(parent) {
                collect_all_slots(parent_class, schema)?
            } else {
                vec![]
            };
            all_slots.into_iter()
                .filter(|slot| !parent_slots.contains(slot))
                .collect()
        } else {
            all_slots
        };

        // Document constructor parameters
        for slot_name in &direct_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let type_str = self.get_jsdoc_type(slot, schema)?;
                let optional = if is_optional_slot(slot) { "[" } else { "" };
                let optional_close = if is_optional_slot(slot) { "]" } else { "" };
                
                write!(&mut output, "   * @param {{{}{}{}}} ", optional, type_str, optional_close).unwrap();
                if is_optional_slot(slot) {
                    write!(&mut output, "[data.{}] - ", slot_name).unwrap();
                } else {
                    write!(&mut output, "data.{} - ", slot_name).unwrap();
                }
                if let Some(ref desc) = slot.description {
                    write!(&mut output, "{}", desc).unwrap();
                } else {
                    write!(&mut output, "{} value", BaseCodeFormatter::to_pascal_case(&slot_name)).unwrap();
                }
                writeln!(&mut output).unwrap();
            }
        }
        writeln!(&mut output, "   */").unwrap();

        // Generate constructor
        writeln!(&mut output, "  constructor(data = {{}}) {{").unwrap();
        
        // Call super if has parent
        if class.is_a.is_some() {
            writeln!(&mut output, "    super(data);").unwrap();
        }

        // Validate data
        writeln!(&mut output, "    this.#validate(data);").unwrap();

        // Initialize fields
        for slot_name in &direct_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.multivalued.unwrap_or(false) {
                    writeln!(&mut output, "    this.{} = data.{} || [];", slot_name, slot_name).unwrap();
                } else if is_optional_slot(slot) {
                    writeln!(&mut output, "    this.{} = data.{} || null;", slot_name, slot_name).unwrap();
                } else {
                    writeln!(&mut output, "    this.{} = data.{};", slot_name, slot_name).unwrap();
                }
            }
        }

        writeln!(&mut output, "  }}").unwrap();
        writeln!(&mut output).unwrap();

        // Generate private validation method
        self.generate_validation_method(&mut output, &direct_slots, schema)?;

        // Generate static fromJSON method
        writeln!(&mut output, "  /**").unwrap();
        writeln!(&mut output, "   * Create from JSON").unwrap();
        writeln!(&mut output, "   * @param {{string}} json - JSON string").unwrap();
        writeln!(&mut output, "   * @returns {{{}}}", class_name).unwrap();
        writeln!(&mut output, "   */").unwrap();
        writeln!(&mut output, "  static fromJSON(json) {{").unwrap();
        writeln!(&mut output, "    return new {}(JSON.parse(json));", class_name).unwrap();
        writeln!(&mut output, "  }}").unwrap();
        writeln!(&mut output).unwrap();

        // Generate toObject method
        writeln!(&mut output, "  /**").unwrap();
        writeln!(&mut output, "   * Convert to plain object").unwrap();
        writeln!(&mut output, "   * @returns {{Object}}").unwrap();
        writeln!(&mut output, "   */").unwrap();
        writeln!(&mut output, "  toObject() {{").unwrap();
        
        if class.is_a.is_some() {
            writeln!(&mut output, "    const parentData = super.toObject();").unwrap();
            writeln!(&mut output, "    return {{").unwrap();
            writeln!(&mut output, "      ...parentData,").unwrap();
        } else {
            writeln!(&mut output, "    return {{").unwrap();
        }
        
        for (i, slot_name) in direct_slots.iter().enumerate() {
            if let Some(slot) = schema.slots.get(slot_name) {
                if slot.multivalued.unwrap_or(false) {
                    write!(&mut output, "      {}: [...this.{}]", slot_name, slot_name).unwrap();
                } else {
                    write!(&mut output, "      {}: this.{}", slot_name, slot_name).unwrap();
                }
                if i < direct_slots.len() - 1 {
                    writeln!(&mut output, ",").unwrap();
                } else {
                    writeln!(&mut output).unwrap();
                }
            }
        }
        
        writeln!(&mut output, "    }};").unwrap();
        writeln!(&mut output, "  }}").unwrap();
        writeln!(&mut output).unwrap();

        // Generate toJSON method
        writeln!(&mut output, "  /**").unwrap();
        writeln!(&mut output, "   * Convert to JSON string").unwrap();
        writeln!(&mut output, "   * @returns {{string}}").unwrap();
        writeln!(&mut output, "   */").unwrap();
        writeln!(&mut output, "  toJSON() {{").unwrap();
        writeln!(&mut output, "    return JSON.stringify(this.toObject());").unwrap();
        writeln!(&mut output, "  }}").unwrap();

        writeln!(&mut output, "}}").unwrap();

        Ok(output)
    }

    /// Generate validation method
    fn generate_validation_method(
        &self,
        output: &mut String,
        slots: &[String],
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "  #validate(data) {{").unwrap();
        
        let mut has_validations = false;

        for slot_name in slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                // Required field validation
                if slot.required.unwrap_or(false) {
                    writeln!(output, "    if (!data.{} || typeof data.{} !== '{}') {{", 
                        slot_name, 
                        slot_name,
                        self.get_js_type_check(&slot.range)
                    ).unwrap();
                    writeln!(output, "      throw new TypeError('{} must be a non-empty {}');", 
                        slot_name,
                        self.get_js_type_name(&slot.range)
                    ).unwrap();
                    writeln!(output, "    }}").unwrap();
                    has_validations = true;
                }

                // Pattern validation
                if let Some(ref pattern) = slot.pattern {
                    writeln!(output, "    if (data.{} && !/{}/u.test(data.{})) {{", 
                        slot_name, pattern, slot_name
                    ).unwrap();
                    writeln!(output, "      throw new TypeError('{} does not match pattern: {}');", 
                        slot_name, pattern
                    ).unwrap();
                    writeln!(output, "    }}").unwrap();
                    has_validations = true;
                }

                // Range validation
                if slot.minimum_value.is_some() || slot.maximum_value.is_some() {
                    if let Some(ref min) = slot.minimum_value {
                        writeln!(output, "    if (typeof data.{} === 'number' && data.{} < {}) {{", 
                            slot_name, slot_name, min
                        ).unwrap();
                        writeln!(output, "      throw new RangeError('{} must be >= {}');", 
                            slot_name, min
                        ).unwrap();
                        writeln!(output, "    }}").unwrap();
                        has_validations = true;
                    }
                    if let Some(ref max) = slot.maximum_value {
                        writeln!(output, "    if (typeof data.{} === 'number' && data.{} > {}) {{", 
                            slot_name, slot_name, max
                        ).unwrap();
                        writeln!(output, "      throw new RangeError('{} must be <= {}');", 
                            slot_name, max
                        ).unwrap();
                        writeln!(output, "    }}").unwrap();
                        has_validations = true;
                    }
                }

                // Array validation
                if slot.multivalued.unwrap_or(false) {
                    writeln!(output, "    if (data.{} && !Array.isArray(data.{})) {{", 
                        slot_name, slot_name
                    ).unwrap();
                    writeln!(output, "      throw new TypeError('{} must be an array');", 
                        slot_name
                    ).unwrap();
                    writeln!(output, "    }}").unwrap();
                    has_validations = true;
                }
            }
        }

        if !has_validations {
            writeln!(output, "    // No validation required").unwrap();
        }

        writeln!(output, "  }}").unwrap();
        Ok(())
    }

    /// Get JSDoc type annotation
    fn get_jsdoc_type(
        &self,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
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
            Ok(format!("{}[]", base_type))
        } else {
            Ok(base_type)
        }
    }

    /// Get JavaScript typeof check string
    fn get_js_type_check(&self, range: &Option<String>) -> &'static str {
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
    fn get_js_type_name(&self, range: &Option<String>) -> &'static str {
        if let Some(r) = range {
            match r.as_str() {
                "string" | "str" => "string",
                "integer" | "int" => "number",
                "float" | "double" | "decimal" => "number",
                "boolean" | "bool" => "boolean",
                _ => "object",
            }
        } else {
            "value"
        }
    }

    /// Generate enum constants
    fn generate_enum(
        &self,
        output: &mut String,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);
        
        writeln!(output, "/**").unwrap();
        if let Some(ref desc) = slot.description {
            writeln!(output, " * {}", desc).unwrap();
        }
        writeln!(output, " * @readonly").unwrap();
        writeln!(output, " * @enum {{string}}").unwrap();
        writeln!(output, " */").unwrap();
        writeln!(output, "export const {} = Object.freeze({{", enum_name).unwrap();

        for value in &slot.permissible_values {
            match value {
                PermissibleValue::Simple(text) => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    writeln!(output, "  {}: \"{}\",", const_name, text).unwrap();
                }
                PermissibleValue::Complex { text, .. } => {
                    let const_name = text.to_uppercase().replace(' ', "_").replace('-', "_");
                    writeln!(output, "  {}: \"{}\",", const_name, text).unwrap();
                }
            }
        }

        writeln!(output, "}});").unwrap();
        Ok(())
    }
}

#[async_trait]
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

        // Use strict mode
        writeln!(&mut content, "'use strict';").unwrap();
        writeln!(&mut content).unwrap();

        // Generate enums first
        for (slot_name, slot) in &schema.slots {
            if !slot.permissible_values.is_empty() {
                self.generate_enum(&mut content, slot_name, slot)?;
                writeln!(&mut content).unwrap();
            }
        }

        // Generate classes
        for (class_name, class_def) in &schema.classes {
            let class_code = self.generate_class(class_name, class_def, schema, options)?;
            content.push_str(&class_code);
            writeln!(&mut content).unwrap();
        }

        // Generate CommonJS exports if requested
        if options.get_custom("module_type") == Some("commonjs") {
            writeln!(&mut content, "// CommonJS exports").unwrap();
            writeln!(&mut content, "if (typeof module !== 'undefined' && module.exports) {{").unwrap();
            
            // Export enums
            for (slot_name, slot) in &schema.slots {
                if !slot.permissible_values.is_empty() {
                    let enum_name = BaseCodeFormatter::to_pascal_case(slot_name);
                    writeln!(&mut content, "  module.exports.{} = {};", enum_name, enum_name).unwrap();
                }
            }
            
            // Export classes
            for class_name in schema.classes.keys() {
                writeln!(&mut content, "  module.exports.{} = {};", class_name, class_name).unwrap();
            }
            
            writeln!(&mut content, "}}").unwrap();
        }

        let extension = if options.get_custom("module_type") == Some("commonjs") {
            "js"
        } else {
            "mjs"
        };

        outputs.push(GeneratedOutput {
            content,
            filename: format!("{}.{}", schema.name.to_lowercase().replace('-', "_"), extension),
            metadata: {
                let mut meta = HashMap::new();
                meta.insert("generator".to_string(), self.name.clone());
                meta.insert("schema".to_string(), schema.name.clone());
                meta.insert("module_type".to_string(), 
                    options.get_custom("module_type").unwrap_or("esm").to_string());
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

impl CodeFormatter for JavaScriptGenerator {
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
        // JavaScript identifiers are typically camelCase
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

        let generator = JavaScriptGenerator::new();
        let options = GeneratorOptions::new();

        let outputs = generator.generate(&schema, &options).await.unwrap();
        assert_eq!(outputs.len(), 1);

        let output = &outputs[0];
        assert!(output.content.contains("export class Person"));
        assert!(output.content.contains("constructor(data = {})"));
        assert!(output.content.contains("#validate(data)"));
        assert!(output.content.contains("static fromJSON(json)"));
        assert!(output.content.contains("toObject()"));
        assert!(output.content.contains("toJSON()"));
    }
}