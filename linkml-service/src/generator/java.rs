//! Java code generator for LinkML schemas
//!
//! This module generates Java classes from LinkML schemas with full
//! validation support and builder patterns.

use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, PermissibleValue};
use std::collections::HashMap;
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

/// Java class generator
pub struct JavaGenerator {
    /// Generator options
    options: GeneratorOptions,
    /// Type mapping from LinkML to Java
    type_map: HashMap<String, String>,
}

impl JavaGenerator {
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    /// Create a new Java generator
    #[must_use]
    pub fn new() -> Self {
        let mut type_map = HashMap::new();
        
        // Basic type mappings
        type_map.insert("string".to_string(), "String".to_string());
        type_map.insert("str".to_string(), "String".to_string());
        type_map.insert("integer".to_string(), "Long".to_string());
        type_map.insert("int".to_string(), "Long".to_string());
        type_map.insert("float".to_string(), "Double".to_string());
        type_map.insert("double".to_string(), "Double".to_string());
        type_map.insert("decimal".to_string(), "BigDecimal".to_string());
        type_map.insert("boolean".to_string(), "Boolean".to_string());
        type_map.insert("bool".to_string(), "Boolean".to_string());
        type_map.insert("date".to_string(), "LocalDate".to_string());
        type_map.insert("datetime".to_string(), "LocalDateTime".to_string());
        type_map.insert("time".to_string(), "LocalTime".to_string());
        type_map.insert("uri".to_string(), "URI".to_string());
        type_map.insert("uriorcurie".to_string(), "String".to_string());
        type_map.insert("curie".to_string(), "String".to_string());
        type_map.insert("ncname".to_string(), "String".to_string());
        
        Self {
            options: GeneratorOptions::default(),
            type_map,
        }
    }
    
    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }
    
    /// Generate package and imports
    fn generate_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        // Package declaration
        let package_name = self.to_snake_case(&schema.name);
        writeln!(&mut output, "package com.example.{};", package_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Standard imports
        writeln!(&mut output, "import java.math.BigDecimal;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.net.URI;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.time.LocalDate;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.time.LocalDateTime;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.time.LocalTime;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.util.*;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import java.util.regex.Pattern;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "import javax.validation.constraints.*;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Schema documentation
        writeln!(&mut output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, " * Generated from LinkML schema: {}", schema.name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, " * Schema ID: {}", schema.id).map_err(Self::fmt_error_to_generator_error)?;
        if let Some(version) = &schema.version {
            writeln!(&mut output, " * Version: {}", version).map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(desc) = &schema.description {
            writeln!(&mut output, " * ").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, " * {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output, " */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate enum class
    fn generate_enum(&self, name: &str, enum_def: &EnumDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        // Javadoc
        writeln!(&mut output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        if let Some(desc) = &enum_def.description {
            writeln!(&mut output, " * {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, " * Enumeration: {}", name).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output, " */").map_err(Self::fmt_error_to_generator_error)?;
        
        writeln!(&mut output, "public enum {} {{", self.to_pascal_case(name)).map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate enum values
        let values_count = enum_def.permissible_values.len();
        for (index, pv) in enum_def.permissible_values.iter().enumerate() {
            let (text, description) = match pv {
                PermissibleValue::Simple(s) => (s.as_str(), None),
                PermissibleValue::Complex { text, description, .. } => (text.as_str(), description.as_ref()),
            };
            
            if let Some(desc) = description {
                writeln!(&mut output, "    /**").map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "     * {}", desc).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "     */").map_err(Self::fmt_error_to_generator_error)?;
            }
            
            let enum_name = self.to_screaming_snake_case(text);
            let comma = if index < values_count - 1 { "," } else { ";" };
            writeln!(&mut output, "    {}{}", enum_name, comma).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate class
    fn generate_class(&self, name: &str, class_def: &ClassDefinition, schema: &SchemaDefinition, options: &GeneratorOptions) -> GeneratorResult<String> {
        let mut output = String::new();
        
        // Javadoc
        writeln!(&mut output, "/**").map_err(Self::fmt_error_to_generator_error)?;
        if let Some(desc) = &class_def.description {
            writeln!(&mut output, " * {}", desc).map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(&mut output, " * Class: {}", name).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output, " */").map_err(Self::fmt_error_to_generator_error)?;
        
        // Class declaration with inheritance
        let extends = if let Some(parent) = &class_def.is_a {
            format!(" extends {}", self.to_pascal_case(parent))
        } else {
            String::new()
        };
        
        writeln!(&mut output, "public class {}{} {{", self.to_pascal_case(name), extends).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Collect all slots (direct only, parent slots inherited via extends)
        let slots: Vec<_> = class_def.slots.iter()
            .filter_map(|slot_name| schema.slots.get(slot_name).map(|s| (slot_name, s)))
            .collect();
        
        // Generate fields
        for (slot_name, slot) in &slots {
            self.write_field(&mut output, slot_name, slot, schema)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Default constructor
        writeln!(&mut output, "    /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "     * Default constructor").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "     */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    public {}() {{", self.to_pascal_case(name)).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        // Initialize collections").map_err(Self::fmt_error_to_generator_error)?;
        for (slot_name, slot) in &slots {
            if slot.multivalued.unwrap_or(false) {
                let field_name = self.to_camel_case(slot_name);
                writeln!(&mut output, "        this.{} = new ArrayList<>();", field_name).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate getters and setters
        for (slot_name, slot) in &slots {
            self.write_getter(&mut output, slot_name, slot, schema)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            self.write_setter(&mut output, slot_name, slot, schema)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Generate builder if requested in options
        if options.get_custom("generate_builder") == Some("true") {
            self.write_builder(&mut output, name, &slots, schema)?;
        }
        
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Write field declaration
    fn write_field(&self, output: &mut String, slot_name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Javadoc
        if let Some(desc) = &slot.description {
            writeln!(output, "    /**").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "     * {}", desc).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "     */").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Validation annotations
        if slot.required.unwrap_or(false) {
            writeln!(output, "    @NotNull").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if let Some(pattern) = &slot.pattern {
            writeln!(output, "    @Pattern(regexp = \"{}\")", pattern).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if let Some(min) = &slot.minimum_value {
            if let Some(num) = min.as_f64() {
                writeln!(output, "    @Min({})", num as i64).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        if let Some(max) = &slot.maximum_value {
            if let Some(num) = max.as_f64() {
                writeln!(output, "    @Max({})", num as i64).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        // Field declaration
        let java_type = self.get_java_type(&slot.range, slot.multivalued.unwrap_or(false), schema)?;
        let field_name = self.to_camel_case(slot_name);
        writeln!(output, "    private {} {};", java_type, field_name).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Write getter method
    fn write_getter(&self, output: &mut String, slot_name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<()> {
        let java_type = self.get_java_type(&slot.range, slot.multivalued.unwrap_or(false), schema)?;
        let field_name = self.to_camel_case(slot_name);
        let method_name = format!("get{}", self.to_pascal_case(slot_name));
        
        writeln!(output, "    public {} {}() {{", java_type, method_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        return {};", field_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Write setter method
    fn write_setter(&self, output: &mut String, slot_name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<()> {
        let java_type = self.get_java_type(&slot.range, slot.multivalued.unwrap_or(false), schema)?;
        let field_name = self.to_camel_case(slot_name);
        let method_name = format!("set{}", self.to_pascal_case(slot_name));
        
        writeln!(output, "    public void {}({} {}) {{", method_name, java_type, field_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        this.{} = {};", field_name, field_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Write builder pattern
    fn write_builder(&self, output: &mut String, class_name: &str, slots: &[(&String, &SlotDefinition)], schema: &SchemaDefinition) -> GeneratorResult<()> {
        let class_pascal = self.to_pascal_case(class_name);
        
        writeln!(output, "    /**").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "     * Builder for {}", class_pascal).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "     */").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    public static class Builder {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        private final {} instance = new {}();", class_pascal, class_pascal).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Builder methods for each field
        for (slot_name, slot) in slots {
            let java_type = self.get_java_type(&slot.range, slot.multivalued.unwrap_or(false), schema)?;
            let field_name = self.to_camel_case(slot_name);
            let method_name = format!("with{}", self.to_pascal_case(slot_name));
            
            writeln!(output, "        public Builder {}({} {}) {{", method_name, java_type, field_name).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "            instance.set{}({});", self.to_pascal_case(slot_name), field_name).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "            return this;").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Build method
        writeln!(output, "        public {} build() {{", class_pascal).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "            // TODO: Add validation").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "            return instance;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Static builder factory method
        writeln!(output, "    public static Builder builder() {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "        return new Builder();").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Get Java type for a LinkML range
    fn get_java_type(&self, range: &Option<String>, multivalued: bool, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let base_type = match range {
            Some(r) => {
                if let Some(java_type) = self.type_map.get(r) {
                    java_type.clone()
                } else if let Some(type_def) = schema.types.get(r) {
                    // Custom type, resolve to base type
                    return self.get_java_type(&type_def.base_type, multivalued, schema);
                } else {
                    // Assume it's a class or enum
                    self.to_pascal_case(r)
                }
            }
            None => "String".to_string(),
        };
        
        if multivalued {
            Ok(format!("List<{}>", base_type))
        } else {
            Ok(base_type)
        }
    }
    
    /// Convert to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;
        
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().expect("char to_lowercase always produces at least one char"));
            prev_upper = ch.is_uppercase();
        }
        
        result
    }
    
    /// Convert to camelCase
    fn to_camel_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;
        
        for (i, ch) in s.chars().enumerate() {
            if ch == '_' || ch == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_uppercase().next().expect("char to_uppercase always produces at least one char"));
                capitalize_next = false;
            } else if i == 0 {
                result.push(ch.to_lowercase().next().expect("char to_lowercase always produces at least one char"));
            } else {
                result.push(ch);
            }
        }
        
        result
    }
    
    /// Convert to PascalCase
    fn to_pascal_case(&self, s: &str) -> String {
        s.split(|c| c == '_' || c == '-')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
    
    /// Convert to SCREAMING_SNAKE_CASE
    fn to_screaming_snake_case(&self, s: &str) -> String {
        self.to_snake_case(s).to_uppercase()
    }
}

impl Default for JavaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for JavaGenerator {
    fn name(&self) -> &str {
        "java"
    }
    
    fn description(&self) -> &str {
        "Generates Java classes with validation annotations from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".java"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let mut outputs = Vec::new();
        
        // Generate header content (package and imports)
        let header = self.generate_header(schema)?;
        
        // Generate enums
        for (name, enum_def) in &schema.enums {
            let mut content = header.clone();
            let enum_code = self.generate_enum(name, enum_def)
                .map_err(|e| GeneratorError::Generation {
                    context: format!("enum {}", name),
                    message: e.to_string(),
                })?;
            content.push_str(&enum_code);
            
            let filename = format!("{}.java", self.to_pascal_case(name));
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "enum".to_string());
            metadata.insert("name".to_string(), name.clone());
            
            outputs.push(GeneratedOutput {
                filename,
                content,
                metadata,
            });
        }
        
        // Generate classes
        for (name, class_def) in &schema.classes {
            let mut content = header.clone();
            let class_code = self.generate_class(name, class_def, schema, options)
                .map_err(|e| GeneratorError::Generation {
                    context: format!("class {}", name),
                    message: e.to_string(),
                })?;
            content.push_str(&class_code);
            
            let filename = format!("{}.java", self.to_pascal_case(name));
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "class".to_string());
            metadata.insert("name".to_string(), name.clone());
            
            outputs.push(GeneratedOutput {
                filename,
                content,
                metadata,
            });
        }
        
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_case_conversion() {
        let generator = JavaGenerator::new();
        
        // snake_case
        assert_eq!(generator.to_snake_case("PersonName"), "person_name");
        assert_eq!(generator.to_snake_case("HTTPRequest"), "httprequest");
        
        // camelCase
        assert_eq!(generator.to_camel_case("person_name"), "personName");
        assert_eq!(generator.to_camel_case("http_request"), "httpRequest");
        assert_eq!(generator.to_camel_case("is-active"), "isActive");
        
        // PascalCase
        assert_eq!(generator.to_pascal_case("person_name"), "PersonName");
        assert_eq!(generator.to_pascal_case("http-request"), "HttpRequest");
        
        // SCREAMING_SNAKE_CASE
        assert_eq!(generator.to_screaming_snake_case("personName"), "PERSON_NAME");
        assert_eq!(generator.to_screaming_snake_case("HTTPMethod"), "HTTPMETHOD");
    }
}