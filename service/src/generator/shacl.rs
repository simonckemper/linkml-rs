//! SHACL (Shapes Constraint Language) generator for `LinkML` schemas
//!
//! This module generates SHACL shapes from `LinkML` schemas for RDF validation.
//! SHACL is a W3C standard for validating RDF graphs against a set of conditions.

use linkml_core::types::{ClassDefinition, PermissibleValue, SchemaDefinition, SlotDefinition};
use std::collections::HashMap;
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult};
use linkml_core::error::LinkMLError;

/// SHACL generator for RDF validation
pub struct ShaclGenerator {
    /// Generator options
    options: GeneratorOptions,
    /// Namespace prefixes
    prefixes: HashMap<String, String>,
}

impl ShaclGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new SHACL generator
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();

        // Standard prefixes
        prefixes.insert("sh".to_string(), "http://www.w3.org/ns/shacl#".to_string());
        prefixes.insert(
            "rdf".to_string(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string(),
        );
        prefixes.insert(
            "rdfs".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#".to_string(),
        );
        prefixes.insert(
            "xsd".to_string(),
            "http://www.w3.org/2001/XMLSchema#".to_string(),
        );
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );

        Self {
            options: GeneratorOptions::default(),
            prefixes,
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Generate prefixes section
    fn generate_prefixes(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Standard prefixes
        for (prefix, uri) in &self.prefixes {
            writeln!(&mut output, "@prefix {prefix}: <{uri}> .")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Schema-specific prefix
        let schema_prefix = Self::to_snake_case(&schema.name);
        writeln!(&mut output, "@prefix {}: <{}#> .", schema_prefix, schema.id)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate header comments
    fn generate_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "# SHACL Shapes generated from LinkML schema: {}",
            schema.name
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "# Schema ID: {}", schema.id)
            .map_err(Self::fmt_error_to_generator_error)?;
        if let Some(version) = &schema.version {
            writeln!(&mut output, "# Version: {version}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "# Description: {desc}")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate SHACL shape for a class
    fn generate_class_shape(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = Self::to_snake_case(&schema.name);
        let shape_name = format!("{}:{}Shape", schema_prefix, Self::to_pascal_case(name));

        // Shape declaration
        writeln!(&mut output, "{shape_name}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    a sh:NodeShape ;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    sh:targetClass {}:{} ;",
            schema_prefix,
            Self::to_pascal_case(name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &class.description {
            writeln!(&mut output, "    rdfs:comment \"{desc}\" ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Collect all slots (including inherited)
        let all_slots = Self::collect_all_slots(class, schema);

        // Generate property shapes
        let mut property_shapes = Vec::new();
        for slot_name in &all_slots {
            if let Some(slot) = schema.slots.get(slot_name) {
                let prop_shape = self.generate_property_shape(slot_name, slot, schema)?;
                property_shapes.push(prop_shape);
            }
        }

        // Add property references
        if property_shapes.is_empty() {
            writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(&mut output, "    sh:property").map_err(Self::fmt_error_to_generator_error)?;
            for (i, _) in property_shapes.iter().enumerate() {
                if i == 0 {
                    writeln!(
                        &mut output,
                        " {}-{} ,",
                        shape_name,
                        Self::to_snake_case(&all_slots[i])
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else if i < property_shapes.len() - 1 {
                    writeln!(
                        &mut output,
                        "                {}-{} ,",
                        shape_name,
                        Self::to_snake_case(&all_slots[i])
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(
                        &mut output,
                        "                {}-{} .",
                        shape_name,
                        Self::to_snake_case(&all_slots[i])
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate the property shapes themselves
        for (slot_name, prop_shape) in all_slots.iter().zip(property_shapes.iter()) {
            writeln!(
                &mut output,
                "{}-{}",
                shape_name,
                Self::to_snake_case(slot_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            write!(&mut output, "{prop_shape}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate property shape for a slot
    fn generate_property_shape(
        &self,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = Self::to_snake_case(&schema.name);

        writeln!(&mut output, "    a sh:PropertyShape ;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    sh:path {}:{} ;",
            schema_prefix,
            Self::to_snake_case(slot_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &slot.description {
            writeln!(&mut output, "    sh:description \"{desc}\" ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Datatype or class reference
        if let Some(range) = &slot.range {
            if let Some(datatype) = Self::get_xsd_datatype(range) {
                writeln!(&mut output, "    sh:datatype {datatype} ;")
                    .map_err(Self::fmt_error_to_generator_error)?;
            } else if schema.classes.contains_key(range) {
                writeln!(
                    &mut output,
                    "    sh:class {}:{} ;",
                    schema_prefix,
                    Self::to_pascal_case(range)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            } else if schema.enums.contains_key(range) {
                // For enums, we'll use sh:in constraint
                if let Some(enum_def) = schema.enums.get(range) {
                    write!(&mut output, "    sh:in (")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    for (i, pv) in enum_def.permissible_values.iter().enumerate() {
                        let value = match pv {
                            PermissibleValue::Simple(s) => s,
                            PermissibleValue::Complex { text, .. } => text,
                        };
                        if i < enum_def.permissible_values.len() - 1 {
                            write!(&mut output, "\"{value}\" ")
                                .map_err(Self::fmt_error_to_generator_error)?;
                        } else {
                            write!(&mut output, "\"{value}\"")
                                .map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }
                    writeln!(&mut output, ") ;").map_err(Self::fmt_error_to_generator_error)?;
                }
            } else if let Some(type_def) = schema.types.get(range) {
                // Custom type - use base type but merge constraints
                let mut merged_slot = slot.clone();
                merged_slot.range.clone_from(&type_def.base_type);

                // Merge pattern constraint if not already present
                if merged_slot.pattern.is_none() && type_def.pattern.is_some() {
                    merged_slot.pattern.clone_from(&type_def.pattern);
                }

                // Merge min/max constraints if not already present
                if merged_slot.minimum_value.is_none() && type_def.minimum_value.is_some() {
                    merged_slot
                        .minimum_value
                        .clone_from(&type_def.minimum_value);
                }
                if merged_slot.maximum_value.is_none() && type_def.maximum_value.is_some() {
                    merged_slot
                        .maximum_value
                        .clone_from(&type_def.maximum_value);
                }

                return self.generate_property_shape(slot_name, &merged_slot, schema);
            }
        }

        // Cardinality constraints
        if slot.required == Some(true) {
            writeln!(&mut output, "    sh:minCount 1 ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if slot.multivalued == Some(true) {
            // No max count by default for multivalued
        } else {
            writeln!(&mut output, "    sh:maxCount 1 ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Pattern constraint
        if let Some(pattern) = &slot.pattern {
            writeln!(&mut output, "    sh:pattern \"{pattern}\" ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Value constraints
        if let Some(min) = &slot.minimum_value
            && let Some(num) = min.as_f64()
        {
            writeln!(&mut output, "    sh:minInclusive {num} ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(max) = &slot.maximum_value
            && let Some(num) = max.as_f64()
        {
            writeln!(&mut output, "    sh:maxInclusive {num} ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Remove trailing semicolon and add period
        if output.ends_with(
            " ;
",
        ) {
            output.truncate(output.len() - 3);
            writeln!(&mut output, " .").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();

        // First, get slots from parent if any
        if let Some(parent_name) = &class.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            all_slots.extend(Self::collect_all_slots(parent_class, schema));
        }

        // Then add direct slots
        all_slots.extend(class.slots.clone());

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        all_slots.retain(|slot| seen.insert(slot.clone()));

        all_slots
    }

    /// Get XSD datatype for `LinkML` range
    fn get_xsd_datatype(range: &str) -> Option<String> {
        match range {
            "string" | "str" => Some("xsd:string".to_string()),
            "integer" | "int" => Some("xsd:integer".to_string()),
            "float" | "double" => Some("xsd:double".to_string()),
            "decimal" => Some("xsd:decimal".to_string()),
            "boolean" | "bool" => Some("xsd:boolean".to_string()),
            "date" => Some("xsd:date".to_string()),
            "datetime" => Some("xsd:dateTime".to_string()),
            "time" => Some("xsd:time".to_string()),
            "uri" => Some("xsd:anyURI".to_string()),
            _ => None,
        }
    }

    /// Convert to `snake_case`
    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_upper = ch.is_uppercase();
        }

        result
    }

    /// Convert to `PascalCase`
    fn to_pascal_case(s: &str) -> String {
        s.split(['_', '-'])
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
}

impl Default for ShaclGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for ShaclGenerator {
    fn name(&self) -> &'static str {
        "shacl"
    }

    fn description(&self) -> &'static str {
        "Generates SHACL shapes for RDF validation from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".shacl", ".ttl"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for SHACL generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let mut output = String::new();

        // Generate header
        output.push_str(&self.generate_header(schema)?);

        // Generate prefixes
        output.push_str(&self.generate_prefixes(schema)?);

        // Generate shapes for each class
        for (name, class) in &schema.classes {
            let shape = self
                .generate_class_shape(name, class, schema)
                .map_err(|e| GeneratorError::Generation(format!("class {name}: {e}")))?;
            output.push_str(&shape);
        }

        // Create output
        Ok(output)
    }

    fn get_file_extension(&self) -> &'static str {
        "ttl"
    }

    fn get_default_filename(&self) -> &'static str {
        "shapes"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xsd_datatype_mapping() {
        assert_eq!(
            ShaclGenerator::get_xsd_datatype("string"),
            Some("xsd:string".to_string())
        );
        assert_eq!(
            ShaclGenerator::get_xsd_datatype("integer"),
            Some("xsd:integer".to_string())
        );
        assert_eq!(
            ShaclGenerator::get_xsd_datatype("boolean"),
            Some("xsd:boolean".to_string())
        );
        assert_eq!(
            ShaclGenerator::get_xsd_datatype("datetime"),
            Some("xsd:dateTime".to_string())
        );
        assert_eq!(ShaclGenerator::get_xsd_datatype("CustomType"), None);
    }

    #[test]
    fn test_case_conversion() {
        assert_eq!(ShaclGenerator::to_snake_case("PersonName"), "person_name");
        assert_eq!(ShaclGenerator::to_pascal_case("person_name"), "PersonName");
    }
}
