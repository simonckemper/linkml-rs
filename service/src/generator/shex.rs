//! `ShEx` (Shape Expressions) generator for `LinkML` schemas
//!
//! This module generates Shape Expressions from `LinkML` schemas for RDF validation.
//! `ShEx` is a language for describing RDF graph structures as sets of constraints.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorResult};
use linkml_core::error::LinkMLError;

/// `ShEx` generation style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShExStyle {
    /// Compact syntax (`ShExC`)
    Compact,
    /// `JSON` representation (`ShExJ`)
    Json,
    /// RDF representation (`ShExR`)
    Rdf,
}

/// Options for `ShEx` generation
#[derive(Debug, Clone)]
pub struct ShExOptions {
    /// Output style
    pub style: ShExStyle,
    /// Generate closed shapes (no extra properties allowed)
    pub closed_shapes: bool,
    /// Include inheritance in shapes
    pub expand_inheritance: bool,
    /// Add shape labels
    pub include_labels: bool,
    /// Add comments from descriptions
    pub include_comments: bool,
    /// Base URI for shapes
    pub base_uri: String,
    /// Strict cardinality (use exact cardinality when min=max)
    pub strict_cardinality: bool,
}

impl Default for ShExOptions {
    fn default() -> Self {
        Self {
            style: ShExStyle::Compact,
            closed_shapes: false,
            expand_inheritance: true,
            include_labels: true,
            include_comments: true,
            base_uri: "http://example.org/shapes/".to_string(),
            strict_cardinality: true,
        }
    }
}

/// `ShEx` generator for RDF validation
pub struct ShExGenerator {
    /// Generation options
    options: ShExOptions,
    /// Namespace prefixes
    prefixes: HashMap<String, String>,
}

impl ShExGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new `ShEx` generator
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();

        // Standard prefixes
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
        prefixes.insert("shex".to_string(), "http://www.w3.org/ns/shex#".to_string());

        Self {
            options: ShExOptions::default(),
            prefixes,
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: ShExOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Set the output style
    #[must_use]
    pub fn with_style(mut self, style: ShExStyle) -> Self {
        self.options.style = style;
        self
    }

    /// Generate `ShEx` from schema
    fn generate_shex(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.options.style {
            ShExStyle::Compact => self.generate_shexc(schema),
            ShExStyle::Json => self.generate_shexj(schema),
            ShExStyle::Rdf => Ok(self.generate_shexr(schema)),
        }
    }

    /// Generate `ShEx` Compact syntax
    fn generate_shexc(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Header comment
        if self.options.include_comments {
            writeln!(
                &mut output,
                "# ShEx shapes for {}",
                if schema.name.is_empty() {
                    "LinkML Schema"
                } else {
                    &schema.name
                }
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            if let Some(desc) = &schema.description {
                writeln!(&mut output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Create a local copy of prefixes for this generation
        let mut prefixes = self.prefixes.clone();

        // Add schema prefix
        let schema_prefix = self.to_snake_case(&schema.name);
        let schema_uri = format!(
            "{}#",
            if schema.id.is_empty() {
                &self.options.base_uri
            } else {
                &schema.id
            }
        );
        prefixes.insert(schema_prefix.clone(), schema_uri);

        // Write prefixes
        self.write_prefixes(&mut output, &prefixes)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate shape for each class
        for (class_name, class_def) in &schema.classes {
            self.generate_class_shape(&mut output, class_name, class_def, schema)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate shapes for enumerations
        for (enum_name, enum_def) in &schema.enums {
            self.generate_enum_shape(&mut output, enum_name, enum_def)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate shape for a class
    fn generate_class_shape(
        &self,
        output: &mut String,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        let schema_prefix = self.to_snake_case(&schema.name);
        let shape_id = format!("{}:{}", schema_prefix, self.to_pascal_case(class_name));

        // Shape header
        if self.options.include_comments
            && let Some(desc) = &class_def.description
        {
            writeln!(output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        write!(output, "{shape_id}").map_err(Self::fmt_error_to_generator_error)?;

        // Add label if enabled
        if self.options.include_labels {
            write!(output, " EXTRA rdf:type").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Open or closed shape
        if self.options.closed_shapes {
            write!(output, " CLOSED").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, " {{").map_err(Self::fmt_error_to_generator_error)?;

        // Type constraint
        writeln!(
            output,
            "  a [ {}:{} ] ;",
            schema_prefix,
            self.to_pascal_case(class_name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Collect all slots (including inherited if enabled)
        let all_slots = if self.options.expand_inheritance {
            self.collect_all_slots(class_name, class_def, schema)
        } else {
            class_def.slots.clone()
        };

        // Generate triple constraints for each slot
        for (i, slot_name) in all_slots.iter().enumerate() {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                self.generate_triple_constraint(output, slot_name, slot_def, schema)?;

                // Add semicolon or nothing for last constraint
                if i < all_slots.len() - 1 {
                    writeln!(output, " ;").map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        // Generate inheritance constraints if not expanded
        if !self.options.expand_inheritance
            && class_def.is_a.is_some()
            && let Some(parent) = &class_def.is_a
        {
            writeln!(
                output,
                "  AND @{}:{}",
                schema_prefix,
                self.to_pascal_case(parent)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate triple constraint for a slot
    fn generate_triple_constraint(
        &self,
        output: &mut String,
        slot_name: &str,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        let schema_prefix = self.to_snake_case(&schema.name);
        let property_uri = format!("{}:{}", schema_prefix, self.to_snake_case(slot_name));

        write!(output, "  {property_uri}").map_err(Self::fmt_error_to_generator_error)?;

        // Value constraint
        write!(output, " ").map_err(Self::fmt_error_to_generator_error)?;
        self.generate_value_constraint(output, slot_def, schema)?;

        // Cardinality
        write!(output, " ").map_err(Self::fmt_error_to_generator_error)?;
        self.generate_cardinality(output, slot_def)?;

        // Annotations
        if self.options.include_comments && slot_def.description.is_some() {
            write!(
                output,
                " // {}",
                slot_def.description.as_ref().unwrap_or(&String::new())
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate value constraint
    fn generate_value_constraint(
        &self,
        output: &mut String,
        slot_def: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        if let Some(range) = &slot_def.range {
            // Check if it's a class reference or enum
            if schema.classes.contains_key(range) || schema.enums.contains_key(range) {
                let schema_prefix = self.to_snake_case(&schema.name);
                write!(output, "@{}:{}", schema_prefix, self.to_pascal_case(range))
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
            // Otherwise it's a datatype
            else {
                let datatype = self.get_xsd_datatype(range);
                write!(output, "{datatype}").map_err(Self::fmt_error_to_generator_error)?;

                // Add facets (constraints)
                let mut facets = Vec::new();

                if let Some(pattern) = &slot_def.pattern {
                    facets.push(format!(r#"PATTERN "{pattern}""#));
                }

                if let Some(min_val) = &slot_def.minimum_value {
                    facets.push(format!("MININCLUSIVE {min_val}"));
                }

                if let Some(max_val) = &slot_def.maximum_value {
                    facets.push(format!("MAXINCLUSIVE {max_val}"));
                }

                // Use proper min_length and max_length fields for string constraints
                if range == "string" {
                    if let Some(min_len) = slot_def.min_length {
                        facets.push(format!("MINLENGTH {min_len}"));
                    }

                    if let Some(max_len) = slot_def.max_length {
                        facets.push(format!("MAXLENGTH {max_len}"));
                    }
                }

                if !facets.is_empty() {
                    write!(output, " {}", facets.join(" "))
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        } else {
            // Default to string if no range specified
            write!(output, "xsd:string").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate cardinality constraint
    fn generate_cardinality(
        &self,
        output: &mut String,
        slot_def: &SlotDefinition,
    ) -> GeneratorResult<()> {
        let required = slot_def.required.unwrap_or(false);
        let multivalued = slot_def.multivalued.unwrap_or(false);

        let (min, max) = match (required, multivalued) {
            (true, false) => (1, Some(1)),  // exactly 1
            (false, false) => (0, Some(1)), // 0 or 1
            (true, true) => (1, None),      // 1 or more
            (false, true) => (0, None),     // 0 or more
        };

        // Cardinality constraints are enforced via required/multivalued flags
        let final_min = min;
        let final_max = match (None::<u32>, max) {
            (Some(m), _) => Some(m),
            (None, m) => m,
        };

        // Write cardinality
        match (final_min, final_max) {
            (0, Some(1)) => write!(output, "?").map_err(Self::fmt_error_to_generator_error)?,
            (1, Some(1)) => {} // No modifier for exactly one
            (0, None) => write!(output, "*").map_err(Self::fmt_error_to_generator_error)?,
            (1, None) => write!(output, "+").map_err(Self::fmt_error_to_generator_error)?,
            (min, Some(max)) if min == max && self.options.strict_cardinality => {
                write!(output, "{{{min}}}").map_err(Self::fmt_error_to_generator_error)?;
            }
            (min, Some(max)) => {
                write!(output, "{{{min},{max}}}").map_err(Self::fmt_error_to_generator_error)?;
            }
            (min, None) => {
                write!(output, "{{{min};}}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(())
    }

    /// Generate shape for an enumeration
    fn generate_enum_shape(
        &self,
        output: &mut String,
        enum_name: &str,
        enum_def: &EnumDefinition,
    ) -> GeneratorResult<()> {
        let schema_prefix = self.to_snake_case(enum_name);
        let shape_id = format!("{}:{}", schema_prefix, self.to_pascal_case(enum_name));

        if self.options.include_comments
            && let Some(desc) = &enum_def.description
        {
            writeln!(output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
        }

        write!(output, "{shape_id} [").map_err(Self::fmt_error_to_generator_error)?;

        // List all permissible values
        let values: Vec<String> = enum_def
            .permissible_values
            .iter()
            .map(|pv| {
                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                format!("\"{value}\"")
            })
            .collect();

        write!(output, " {} ]", values.join(" ")).map_err(Self::fmt_error_to_generator_error)?;

        Ok(())
    }

    /// Generate `ShExJ` (`JSON` format)
    fn generate_shexj(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        // Simplified JSON representation
        let mut shex_json = serde_json::json!({
            "@context": "http://www.w3.org/ns/shex.jsonld",
            "type": "Schema",
            "shapes": []
        });

        let shapes = shex_json["shapes"].as_array_mut().ok_or_else(|| {
            GeneratorError::Generation("ShExJ: shapes array not found in JSON".to_string())
        })?;

        // Add shapes for each class
        for (class_name, class_def) in &schema.classes {
            let shape = self.generate_json_shape(class_name, class_def, schema)?;
            shapes.push(shape);
        }

        serde_json::to_string_pretty(&shex_json)
            .map_err(|e| GeneratorError::Generation(format!("ShExJ: {e}")))
    }

    /// Generate `JSON` representation of a shape
    fn generate_json_shape(
        &self,
        class_name: &str,
        _class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<serde_json::Value> {
        let schema_prefix = self.to_snake_case(&schema.name);
        let shape_id = format!("{}:{}", schema_prefix, self.to_pascal_case(class_name));

        let mut shape = serde_json::json!({
            "type": "Shape",
            "id": shape_id,
            "expression": {
                "type": "TripleConstraint",
                "predicate": "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
                "valueExpr": {
                    "type": "NodeConstraint",
                    "values": [format!("{}#{}", if schema.id.is_empty() { "" } else { &schema.id }, class_name)]
                }
            }
        });

        if self.options.closed_shapes {
            shape["closed"] = serde_json::json!(true);
        }

        Ok(shape)
    }

    /// Generate `ShExR` (RDF format)
    fn generate_shexr(&self, _schema: &SchemaDefinition) -> String {
        // Simplified RDF representation in Turtle
        let output = r"@prefix shex: <http://www.w3.org/ns/shex#> .
@prefix ex: <http://example.org/> .

# ShExR format would represent shapes as RDF triples
# This is a simplified example
ex:MyShape a shex:Shape ;
    shex:closed true ;
    shex:expression [
        a shex:TripleConstraint ;
        shex:predicate rdf:type ;
        shex:valueExpr [ a shex:NodeConstraint ]
    ] .
";
        output.to_string()
    }

    /// Write namespace prefixes
    fn write_prefixes(
        &self,
        output: &mut String,
        prefixes: &HashMap<String, String>,
    ) -> GeneratorResult<()> {
        for (prefix, uri) in prefixes {
            writeln!(output, "PREFIX {prefix}: <{uri}>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        Ok(())
    }

    /// Get XSD datatype for `LinkML` range
    fn get_xsd_datatype(&self, range: &str) -> String {
        match range {
            "string" | "str" => "xsd:string",
            "integer" | "int" => "xsd:integer",
            "float" => "xsd:float",
            "double" => "xsd:double",
            "decimal" => "xsd:decimal",
            "boolean" | "bool" => "xsd:boolean",
            "date" => "xsd:date",
            "datetime" => "xsd:dateTime",
            "time" => "xsd:time",
            "uri" => "IRI",
            _ => "xsd:string",
        }
        .to_string()
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();

        // First, get slots from parent if any
        if let Some(parent_name) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
            for slot in parent_slots {
                if seen.insert(slot.clone()) {
                    all_slots.push(slot);
                }
            }
        }

        // Then add direct slots
        for slot in &class_def.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add attributes
        for (attr_name, _) in &class_def.attributes {
            if seen.insert(attr_name.clone()) {
                all_slots.push(attr_name.clone());
            }
        }

        all_slots
    }

    /// Convert to `snake_case`
    fn to_snake_case(&self, s: &str) -> String {
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
    fn to_pascal_case(&self, s: &str) -> String {
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

impl Default for ShExGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for ShExGenerator {
    fn name(&self) -> &str {
        match self.options.style {
            ShExStyle::Compact => "shex",
            ShExStyle::Json => "shexj",
            ShExStyle::Rdf => "shexr",
        }
    }

    fn description(&self) -> &'static str {
        "Generates Shape Expressions (ShEx) for RDF validation from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        match self.options.style {
            ShExStyle::Compact => vec![".shex", ".shexc"],
            ShExStyle::Json => vec![".shexj", ".json"],
            ShExStyle::Rdf => vec![".shexr", ".ttl"],
        }
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for ShEx generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let content = self.generate_shex(schema)?;

        Ok(content)
    }

    fn get_file_extension(&self) -> &str {
        match self.options.style {
            ShExStyle::Compact => "shex",
            ShExStyle::Json => "json",
            ShExStyle::Rdf => "ttl",
        }
    }

    fn get_default_filename(&self) -> &'static str {
        "shapes"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();
        schema.id = "http://example.org/test".to_string();

        // Person class
        let mut person_class = ClassDefinition::default();
        person_class.slots = vec!["name".to_string(), "age".to_string(), "friends".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        // Define slots
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        name_slot.pattern = Some(r"^[A-Za-z ]+$".to_string());
        schema.slots.insert("name".to_string(), name_slot);

        let mut age_slot = SlotDefinition::default();
        age_slot.range = Some("integer".to_string());
        age_slot.minimum_value = Some(serde_json::json!(0));
        age_slot.maximum_value = Some(serde_json::json!(150));
        schema.slots.insert("age".to_string(), age_slot);

        let mut friends_slot = SlotDefinition::default();
        friends_slot.range = Some("Person".to_string());
        friends_slot.multivalued = Some(true);
        schema.slots.insert("friends".to_string(), friends_slot);

        // Add an enum
        let mut status_enum = EnumDefinition::default();
        status_enum.permissible_values = vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ];
        schema.enums.insert("Status".to_string(), status_enum);

        schema
    }

    #[test]
    fn test_shex_compact_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let generator = ShExGenerator::new();

        let output = generator
            .generate(&schema)
            .expect("should generate ShEx: {}");

        // Check content
        assert!(output.contains("PREFIX"));
        assert!(output.contains("test_schema:Person"));
        assert!(output.contains("PATTERN"));
        assert!(output.contains("MININCLUSIVE"));
        Ok(())
    }

    #[test]
    fn test_cardinality_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let generator = ShExGenerator::new();

        let output = generator
            .generate(&schema)
            .expect("should generate ShEx: {}");

        // Check cardinality markers
        assert!(output.contains("?")); // optional age
        assert!(output.contains("*")); // multiple friends
        Ok(())
    }

    #[test]
    fn test_closed_shapes() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let schema = create_test_schema();
        let mut options = ShExOptions::default();
        options.closed_shapes = true;

        let generator = ShExGenerator::with_options(options);
        let output = generator
            .generate(&schema)
            .expect("should generate ShEx: {}");

        assert!(output.contains("CLOSED"));
        Ok(())
    }
}
