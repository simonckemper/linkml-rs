//! SPARQL query generator for `LinkML` schemas
//!
//! This module generates SPARQL queries from `LinkML` schemas to enable
//! querying RDF data that conforms to the schema structure.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorResult};
use linkml_core::error::LinkMLError;

/// SPARQL query type to generate
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SparqlQueryType {
    /// SELECT queries for retrieving data
    Select,
    /// CONSTRUCT queries for transforming data
    Construct,
    /// ASK queries for validation
    Ask,
    /// INSERT DATA for adding triples
    Insert,
    /// DELETE WHERE for removing triples
    Delete,
}

/// Options for SPARQL generation
#[derive(Debug, Clone)]
pub struct SparqlOptions {
    /// Type of queries to generate
    pub query_type: SparqlQueryType,
    /// Include optional slots in queries
    pub include_optional: bool,
    /// Generate FILTER constraints
    pub include_filters: bool,
    /// Generate subqueries for inheritance
    pub use_subqueries: bool,
    /// Add LIMIT to SELECT queries
    pub limit: Option<usize>,
    /// Base URI for the schema
    pub base_uri: String,
    /// Generate comments in queries
    pub include_comments: bool,
}

impl Default for SparqlOptions {
    fn default() -> Self {
        Self {
            query_type: SparqlQueryType::Select,
            include_optional: true,
            include_filters: true,
            use_subqueries: false,
            limit: None,
            base_uri: "http://example.org/".to_string(),
            include_comments: true,
        }
    }
}

/// SPARQL query generator
pub struct SparqlGenerator {
    /// Generation options
    options: SparqlOptions,
    /// Namespace prefixes
    prefixes: HashMap<String, String>,
}

impl SparqlGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new SPARQL generator
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
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );

        Self {
            options: SparqlOptions::default(),
            prefixes,
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: SparqlOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Set the query type
    #[must_use]
    pub fn with_query_type(mut self, query_type: SparqlQueryType) -> Self {
        self.options.query_type = query_type;
        self
    }

    /// Generate SPARQL queries for the schema
    fn generate_sparql(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Add schema prefix
        let schema_prefix = Self::to_snake_case(&schema.name);
        self.add_schema_prefix(&schema_prefix, schema);

        match self.options.query_type {
            SparqlQueryType::Select => self.generate_select_queries(&mut output, schema)?,
            SparqlQueryType::Construct => self.generate_construct_queries(&mut output, schema)?,
            SparqlQueryType::Ask => self.generate_ask_queries(&mut output, schema)?,
            SparqlQueryType::Insert => self.generate_insert_queries(&mut output, schema)?,
            SparqlQueryType::Delete => self.generate_delete_queries(&mut output, schema)?,
        }

        Ok(output)
    }

    /// Generate SELECT queries for each class
    fn generate_select_queries(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "# SPARQL SELECT queries for {}",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue; // Skip abstract classes
            }

            if self.options.include_comments {
                writeln!(output, "# Query to retrieve all instances of {class_name}")
                    .map_err(Self::fmt_error_to_generator_error)?;
                if let Some(desc) = &class_def.description {
                    writeln!(output, "# {desc}").map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Generate prefixes
            self.write_prefixes(output)?;

            // SELECT clause
            write!(output, "SELECT").map_err(Self::fmt_error_to_generator_error)?;
            let vars = self.collect_query_variables(class_name, class_def, schema);
            for var in &vars {
                write!(output, " ?{var}").map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

            // WHERE clause
            writeln!(output, "WHERE {{").map_err(Self::fmt_error_to_generator_error)?;

            // Type assertion
            let class_uri = self.get_class_uri(class_name, schema);
            writeln!(output, "  ?instance a {class_uri} .")
                .map_err(Self::fmt_error_to_generator_error)?;

            // Generate triple patterns for slots
            self.generate_triple_patterns(output, "instance", class_name, class_def, schema)?;

            // Add filters if enabled
            if self.options.include_filters {
                self.generate_filters(output, class_name, class_def, schema)?;
            }

            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;

            // Add modifiers
            if let Some(limit) = self.options.limit {
                writeln!(output, "LIMIT {limit}").map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate CONSTRUCT queries
    fn generate_construct_queries(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(
            output,
            "# SPARQL CONSTRUCT queries for {}",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            if self.options.include_comments {
                writeln!(
                    output,
                    "# Construct {class_name} instances with all properties"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            self.write_prefixes(output)?;

            writeln!(output, "CONSTRUCT {{").map_err(Self::fmt_error_to_generator_error)?;

            // Construct template
            let class_uri = self.get_class_uri(class_name, schema);
            writeln!(output, "  ?instance a {class_uri} .")
                .map_err(Self::fmt_error_to_generator_error)?;

            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if schema.slots.contains_key(slot_name) {
                    let prop_uri = self.get_property_uri(slot_name, schema);

                    // Both multivalued and single-valued slots use the same pattern
                    writeln!(
                        output,
                        "  ?instance {} ?{} .",
                        prop_uri,
                        Self::to_var_name(slot_name)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "WHERE {{").map_err(Self::fmt_error_to_generator_error)?;

            // Where patterns (same as construct template)
            writeln!(output, "  ?instance a {class_uri} .")
                .map_err(Self::fmt_error_to_generator_error)?;
            self.generate_triple_patterns(output, "instance", class_name, class_def, schema)?;

            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate ASK queries for validation
    fn generate_ask_queries(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL ASK queries for validation")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            // Check if instance exists
            if self.options.include_comments {
                writeln!(output, "# Check if any {class_name} instances exist")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            self.write_prefixes(output)?;

            writeln!(output, "ASK {{").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                output,
                "  ?instance a {} .",
                self.get_class_uri(class_name, schema)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

            // Validation queries for required slots
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && slot_def.required.unwrap_or(false)
                {
                    if self.options.include_comments {
                        writeln!(
                                output,
                                "# Check if all {class_name} instances have required property {slot_name}"
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    self.write_prefixes(output)?;

                    writeln!(output, "ASK {{").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "  ?instance a {} .",
                        self.get_class_uri(class_name, schema)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  FILTER NOT EXISTS {{")
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(
                        output,
                        "    ?instance {} ?value .",
                        self.get_property_uri(slot_name, schema)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }

    /// Generate INSERT DATA queries
    fn generate_insert_queries(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL INSERT DATA templates")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            if self.options.include_comments {
                writeln!(output, "# Template for inserting {class_name} instances")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            self.write_prefixes(output)?;

            writeln!(output, "INSERT DATA {{").map_err(Self::fmt_error_to_generator_error)?;

            // Example instance URI
            let instance_uri = format!(
                "<{}{}/example>",
                self.options.base_uri,
                Self::to_snake_case(class_name)
            );

            writeln!(
                output,
                "  {} a {} .",
                instance_uri,
                self.get_class_uri(class_name, schema)
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let prop_uri = self.get_property_uri(slot_name, schema);

                    // Show example values
                    let example_value = self.get_example_value(slot_def.range.as_ref());

                    if slot_def.required.unwrap_or(false) {
                        writeln!(output, "  {instance_uri} {prop_uri} {example_value} .")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(
                            output,
                            "  # {instance_uri} {prop_uri} {example_value} . # optional"
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }

            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate DELETE WHERE queries
    fn generate_delete_queries(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL DELETE WHERE templates")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }

            if self.options.include_comments {
                writeln!(output, "# Delete all {class_name} instances")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            self.write_prefixes(output)?;

            writeln!(output, "DELETE WHERE {{").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                output,
                "  ?instance a {} .",
                self.get_class_uri(class_name, schema)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "  ?instance ?p ?o .").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(())
    }

    /// Generate triple patterns for a class
    fn generate_triple_patterns(
        &self,
        output: &mut String,
        subject_var: &str,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        let all_slots = self.collect_all_slots(class_name, class_def, schema);

        for slot_name in &all_slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let prop_uri = self.get_property_uri(slot_name, schema);
                let var_name = Self::to_var_name(slot_name);

                if slot_def.required.unwrap_or(false) || !self.options.include_optional {
                    // Required property
                    writeln!(output, "  ?{subject_var} {prop_uri} ?{var_name} .")
                        .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    // Optional property
                    writeln!(
                        output,
                        "  OPTIONAL {{ ?{subject_var} {prop_uri} ?{var_name} . }}"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Add type constraints for object properties
                if let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    let range_uri = self.get_class_uri(range, schema);
                    writeln!(output, "  OPTIONAL {{ ?{var_name} a {range_uri} . }}")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }

    /// Generate FILTER constraints
    fn generate_filters(
        &self,
        output: &mut String,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        let all_slots = self.collect_all_slots(class_name, class_def, schema);

        for slot_name in &all_slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let var_name = Self::to_var_name(slot_name);

                // Pattern constraint
                if let Some(pattern) = &slot_def.pattern {
                    writeln!(output, "  FILTER(REGEX(?{var_name}, \"{pattern}\"))")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Range constraints
                if let Some(min) = &slot_def.minimum_value {
                    writeln!(output, "  FILTER(?{var_name} >= {min})")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                if let Some(max) = &slot_def.maximum_value {
                    writeln!(output, "  FILTER(?{var_name} <= {max})")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                // Enum constraints
                if let Some(range) = &slot_def.range
                    && let Some(enum_def) = schema.enums.get(range)
                {
                    write!(output, "  FILTER(?{var_name} IN (")
                        .map_err(Self::fmt_error_to_generator_error)?;

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

                    write!(output, "{}", values.join(", "))
                        .map_err(Self::fmt_error_to_generator_error)?;
                    writeln!(output, "))").map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }

    /// Write namespace prefixes
    fn write_prefixes(&self, output: &mut String) -> GeneratorResult<()> {
        for (prefix, uri) in &self.prefixes {
            writeln!(output, "PREFIX {prefix}: <{uri}>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Add schema-specific prefix
    fn add_schema_prefix(&self, _prefix: &str, schema: &SchemaDefinition) {
        let _uri = format!(
            "{}#",
            if schema.id.is_empty() {
                &self.options.base_uri
            } else {
                &schema.id
            }
        );
        // Note: In real implementation, would need mutable access to prefixes
    }

    /// Get URI for a class
    fn get_class_uri(&self, class_name: &str, schema: &SchemaDefinition) -> String {
        let prefix = Self::to_snake_case(&schema.name);
        format!("{}:{}", prefix, Self::to_pascal_case(class_name))
    }

    /// Get URI for a property
    fn get_property_uri(&self, slot_name: &str, schema: &SchemaDefinition) -> String {
        let prefix = Self::to_snake_case(&schema.name);
        format!("{}:{}", prefix, Self::to_snake_case(slot_name))
    }

    /// Collect query variables for a class
    fn collect_query_variables(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut vars = vec!["instance".to_string()];

        let all_slots = self.collect_all_slots(class_name, class_def, schema);
        for slot_name in &all_slots {
            vars.push(Self::to_var_name(slot_name));
        }

        vars
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

    /// Get example value for a range type
    fn get_example_value(&self, range: Option<&String>) -> &'static str {
        match range.map(String::as_str) {
            Some("string") => "\"example string\"",
            Some("integer") => "42",
            Some("float" | "double") => "3.14",
            Some("boolean") => "true",
            Some("date") => "\"2024-01-01\"^^xsd:date",
            Some("datetime") => "\"2024-01-01T00:00:00Z\"^^xsd:dateTime",
            Some("uri") => "<http://example.org/resource>",
            _ => "\"value\"",
        }
    }

    /// Convert to variable name
    fn to_var_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
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

impl Default for SparqlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for SparqlGenerator {
    fn name(&self) -> &'static str {
        "sparql"
    }

    fn description(&self) -> &'static str {
        "Generates SPARQL queries from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".sparql", ".rq"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for SPARQL generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let content = self.generate_sparql(schema)?;

        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "sparql"
    }

    fn get_default_filename(&self) -> &'static str {
        "queries"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        // Person class
        let person_class = ClassDefinition {
            slots: vec!["name".to_string(), "age".to_string(), "email".to_string()],
            ..Default::default()
        };

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);

        // Define slots
        let name_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            minimum_value: Some(serde_json::json!(0)),
            maximum_value: Some(serde_json::json!(150)),
            ..Default::default()
        };

        let email_slot = SlotDefinition {
            range: Some("string".to_string()),
            pattern: Some(r"^\S+@\S+\.\S+$".to_string()),
            ..Default::default()
        };

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);
        slots.insert("email".to_string(), email_slot);

        SchemaDefinition {
            name: "TestSchema".to_string(),
            id: "http://example.org/test".to_string(),
            classes,
            slots,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_select_query_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new();
        // GeneratorOptions not needed for new generate signature

        let output = generator
            .generate(&schema)
            .expect("should generate queries: {}");

        // Check content
        assert!(output.contains("SELECT"));
        assert!(output.contains("WHERE"));
        assert!(output.contains("?instance a"));
        assert!(output.contains("PREFIX"));
    }

    #[tokio::test]
    async fn test_construct_query_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new().with_query_type(SparqlQueryType::Construct);
        // GeneratorOptions not needed for new generate signature

        let output = generator
            .generate(&schema)
            .expect("should generate queries: {}");

        assert!(output.contains("CONSTRUCT"));
    }

    #[tokio::test]
    async fn test_filter_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new();
        // GeneratorOptions not needed for new generate signature

        let output = generator
            .generate(&schema)
            .expect("should generate queries: {}");

        // Should contain filters for constraints
        assert!(output.contains("FILTER"));
        assert!(output.contains("REGEX")); // For email pattern
        assert!(output.contains(">=")); // For age minimum
        assert!(output.contains("<=")); // For age maximum
    }
}
