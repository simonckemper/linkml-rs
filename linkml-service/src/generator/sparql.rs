//! SPARQL query generator for LinkML schemas
//!
//! This module generates SPARQL queries from LinkML schemas to enable
//! querying RDF data that conforms to the schema structure.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

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
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    /// Create a new SPARQL generator
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        
        // Standard prefixes
        prefixes.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
        prefixes.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
        prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
        prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        
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
        let schema_prefix = self.to_snake_case(&schema.name);
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
    fn generate_select_queries(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL SELECT queries for {}", schema.name.as_deref().unwrap_or("schema")).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue; // Skip abstract classes
            }
            
            if self.options.include_comments {
                writeln!(output, "# Query to retrieve all instances of {}", class_name).map_err(Self::fmt_error_to_generator_error)?;
                if let Some(desc) = &class_def.description {
                    writeln!(output, "# {}", desc).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            
            // Generate prefixes
            self.write_prefixes(output)?;
            
            // SELECT clause
            write!(output, "SELECT").map_err(Self::fmt_error_to_generator_error)?;
            let vars = self.collect_query_variables(class_name, class_def, schema);
            for var in &vars {
                write!(output, " ?{}", var).map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
            
            // WHERE clause
            writeln!(output, "WHERE {{").map_err(Self::fmt_error_to_generator_error)?;
            
            // Type assertion
            let class_uri = self.get_class_uri(class_name, schema);
            writeln!(output, "  ?instance a {} .", class_uri).map_err(Self::fmt_error_to_generator_error)?;
            
            // Generate triple patterns for slots
            self.generate_triple_patterns(output, "instance", class_name, class_def, schema)?;
            
            // Add filters if enabled
            if self.options.include_filters {
                self.generate_filters(output, class_name, class_def, schema)?;
            }
            
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            
            // Add modifiers
            if let Some(limit) = self.options.limit {
                writeln!(output, "LIMIT {}", limit).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        Ok(())
    }
    
    /// Generate CONSTRUCT queries
    fn generate_construct_queries(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL CONSTRUCT queries for {}", schema.name.as_deref().unwrap_or("schema")).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            if self.options.include_comments {
                writeln!(output, "# Construct {} instances with all properties", class_name).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            self.write_prefixes(output)?;
            
            writeln!(output, "CONSTRUCT {{").map_err(Self::fmt_error_to_generator_error)?;
            
            // Construct template
            let class_uri = self.get_class_uri(class_name, schema);
            writeln!(output, "  ?instance a {} .", class_uri).map_err(Self::fmt_error_to_generator_error)?;
            
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let prop_uri = self.get_property_uri(slot_name, schema);
                    
                    if slot_def.multivalued.unwrap_or(false) {
                        writeln!(output, "  ?instance {} ?{} .", prop_uri, self.to_var_name(slot_name)).map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(output, "  ?instance {} ?{} .", prop_uri, self.to_var_name(slot_name)).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
            
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "WHERE {{").map_err(Self::fmt_error_to_generator_error)?;
            
            // Where patterns (same as construct template)
            writeln!(output, "  ?instance a {} .", class_uri).map_err(Self::fmt_error_to_generator_error)?;
            self.generate_triple_patterns(output, "instance", class_name, class_def, schema)?;
            
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        Ok(())
    }
    
    /// Generate ASK queries for validation
    fn generate_ask_queries(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL ASK queries for validation").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            // Check if instance exists
            if self.options.include_comments {
                writeln!(output, "# Check if any {} instances exist", class_name).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            self.write_prefixes(output)?;
            
            writeln!(output, "ASK {{").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "  ?instance a {} .", self.get_class_uri(class_name, schema)).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
            
            // Validation queries for required slots
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    if slot_def.required.unwrap_or(false) {
                        if self.options.include_comments {
                            writeln!(output, "# Check if all {} instances have required property {}", class_name, slot_name).map_err(Self::fmt_error_to_generator_error)?;
                        }
                        
                        self.write_prefixes(output)?;
                        
                        writeln!(output, "ASK {{").map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "  ?instance a {} .", self.get_class_uri(class_name, schema)).map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "  FILTER NOT EXISTS {{").map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "    ?instance {} ?value .", self.get_property_uri(slot_name, schema)).map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "  }}").map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate INSERT DATA queries
    fn generate_insert_queries(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL INSERT DATA templates").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            if self.options.include_comments {
                writeln!(output, "# Template for inserting {} instances", class_name).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            self.write_prefixes(output)?;
            
            writeln!(output, "INSERT DATA {{").map_err(Self::fmt_error_to_generator_error)?;
            
            // Example instance URI
            let instance_uri = format!("<{}{}/example>", self.options.base_uri, self.to_snake_case(class_name));
            
            writeln!(output, "  {} a {} .", instance_uri, self.get_class_uri(class_name, schema)).map_err(Self::fmt_error_to_generator_error)?;
            
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let prop_uri = self.get_property_uri(slot_name, schema);
                    
                    // Show example values
                    let example_value = self.get_example_value(&slot_def.range);
                    
                    if slot_def.required.unwrap_or(false) {
                        writeln!(output, "  {} {} {} .", instance_uri, prop_uri, example_value).map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(output, "  # {} {} {} . # optional", instance_uri, prop_uri, example_value).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
            
            writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        Ok(())
    }
    
    /// Generate DELETE WHERE queries
    fn generate_delete_queries(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "# SPARQL DELETE WHERE templates").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            if self.options.include_comments {
                writeln!(output, "# Delete all {} instances", class_name).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            self.write_prefixes(output)?;
            
            writeln!(output, "DELETE WHERE {{").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "  ?instance a {} .", self.get_class_uri(class_name, schema)).map_err(Self::fmt_error_to_generator_error)?;
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
        schema: &SchemaDefinition
    ) -> GeneratorResult<()> {
        let all_slots = self.collect_all_slots(class_name, class_def, schema);
        
        for slot_name in &all_slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let prop_uri = self.get_property_uri(slot_name, schema);
                let var_name = self.to_var_name(slot_name);
                
                if slot_def.required.unwrap_or(false) || !self.options.include_optional {
                    // Required property
                    writeln!(output, "  ?{} {} ?{} .", subject_var, prop_uri, var_name).map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    // Optional property
                    writeln!(output, "  OPTIONAL {{ ?{} {} ?{} . }}", subject_var, prop_uri, var_name).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                // Add type constraints for object properties
                if let Some(range) = &slot_def.range {
                    if schema.classes.contains_key(range) {
                        let range_uri = self.get_class_uri(range, schema);
                        writeln!(output, "  OPTIONAL {{ ?{} a {} . }}", var_name, range_uri).map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate FILTER constraints
    fn generate_filters(
        &self,
        output: &mut String,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition
    ) -> GeneratorResult<()> {
        let all_slots = self.collect_all_slots(_class_name, class_def, schema);
        
        for slot_name in &all_slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let var_name = self.to_var_name(slot_name);
                
                // Pattern constraint
                if let Some(pattern) = &slot_def.pattern {
                    writeln!(output, "  FILTER(REGEX(?{}, \"{}\"))", var_name, pattern).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                // Range constraints
                if let Some(min) = &slot_def.minimum_value {
                    writeln!(output, "  FILTER(?{} >= {})", var_name, min).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                if let Some(max) = &slot_def.maximum_value {
                    writeln!(output, "  FILTER(?{} <= {})", var_name, max).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                // Enum constraints
                if let Some(range) = &slot_def.range {
                    if let Some(enum_def) = schema.enums.get(range) {
                        write!(output, "  FILTER(?{} IN (", var_name).map_err(Self::fmt_error_to_generator_error)?;
                        
                        let values: Vec<String> = enum_def.permissible_values.iter()
                            .map(|pv| {
                                let value = match pv {
                                    PermissibleValue::Simple(s) => s,
                                    PermissibleValue::Complex { text, .. } => text,
                                };
                                format!("\"{}\"", value)
                            })
                            .collect();
                        
                        write!(output, "{}", values.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(output, "))").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Write namespace prefixes
    fn write_prefixes(&self, output: &mut String) -> GeneratorResult<()> {
        for (prefix, uri) in &self.prefixes {
            writeln!(output, "PREFIX {}: <{}>", prefix, uri).map_err(Self::fmt_error_to_generator_error)?;
        }
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }
    
    /// Add schema-specific prefix
    fn add_schema_prefix(&self, prefix: &str, schema: &SchemaDefinition) {
        let uri = format!("{}#", schema.id.as_deref().unwrap_or(&self.options.base_uri));
        // Note: In real implementation, would need mutable access to prefixes
    }
    
    /// Get URI for a class
    fn get_class_uri(&self, class_name: &str, schema: &SchemaDefinition) -> String {
        let prefix = self.to_snake_case(&schema.name);
        format!("{}:{}", prefix, self.to_pascal_case(class_name))
    }
    
    /// Get URI for a property
    fn get_property_uri(&self, slot_name: &str, _schema: &SchemaDefinition) -> String {
        let prefix = self.to_snake_case(&_schema.name);
        format!("{}:{}", prefix, self.to_snake_case(slot_name))
    }
    
    /// Collect query variables for a class
    fn collect_query_variables(&self, class_name: &str, class_def: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut vars = vec!["instance".to_string()];
        
        let all_slots = self.collect_all_slots(class_name, class_def, schema);
        for slot_name in &all_slots {
            vars.push(self.to_var_name(slot_name));
        }
        
        vars
    }
    
    /// Collect all slots including inherited ones
    fn collect_all_slots(&self, _class_name: &str, class_def: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();
        
        // First, get slots from parent if any
        if let Some(parent_name) = &class_def.is_a {
            if let Some(parent_class) = schema.classes.get(parent_name) {
                let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
                for slot in parent_slots {
                    if seen.insert(slot.clone()) {
                        all_slots.push(slot);
                    }
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
    fn get_example_value(&self, range: &Option<String>) -> &'static str {
        match range.as_deref() {
            Some("string") => "\"example string\"",
            Some("integer") => "42",
            Some("float") | Some("double") => "3.14",
            Some("boolean") => "true",
            Some("date") => "\"2024-01-01\"^^xsd:date",
            Some("datetime") => "\"2024-01-01T00:00:00Z\"^^xsd:dateTime",
            Some("uri") => "<http://example.org/resource>",
            _ => "\"value\"",
        }
    }
    
    /// Convert to variable name
    fn to_var_name(&self, name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
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
}

impl Default for SparqlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for SparqlGenerator {
    fn name(&self) -> &str {
        "sparql"
    }
    
    fn description(&self) -> &str {
        "Generates SPARQL queries from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".sparql", ".rq"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let content = self.generate_sparql(schema)?;
        
        let filename = format!("{}.sparql", 
            schema.name.as_deref().unwrap_or("schema"));
        
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "sparql".to_string());
        metadata.insert("query_type".to_string(), format!("{:?}", self.options.query_type).to_lowercase());
        
        Ok(vec![GeneratedOutput {
            filename,
            content,
            metadata,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        schema.id = Some("http://example.org/test".to_string());
        
        // Person class
        let mut person_class = ClassDefinition::default();
        person_class.slots = vec!["name".to_string(), "age".to_string(), "email".to_string()];
        schema.classes.insert("Person".to_string(), person_class);
        
        // Define slots
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.range = Some("integer".to_string());
        age_slot.minimum_value = Some(serde_json::json!(0));
        age_slot.maximum_value = Some(serde_json::json!(150));
        schema.slots.insert("age".to_string(), age_slot);
        
        let mut email_slot = SlotDefinition::default();
        email_slot.range = Some("string".to_string());
        email_slot.pattern = Some(r"^\S+@\S+\.\S+$".to_string());
        schema.slots.insert("email".to_string(), email_slot);
        
        schema
    }
    
    #[tokio::test]
    async fn test_select_query_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate queries");
        assert_eq!(result.len(), 1);
        
        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.sparql");
        
        // Check content
        assert!(output.content.contains("SELECT"));
        assert!(output.content.contains("WHERE"));
        assert!(output.content.contains("?instance a"));
        assert!(output.content.contains("PREFIX"));
    }
    
    #[tokio::test]
    async fn test_construct_query_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new()
            .with_query_type(SparqlQueryType::Construct);
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate queries");
        let output = &result[0];
        
        assert!(output.content.contains("CONSTRUCT"));
    }
    
    #[tokio::test]
    async fn test_filter_generation() {
        let schema = create_test_schema();
        let generator = SparqlGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.expect("should generate queries");
        let output = &result[0];
        
        // Should contain filters for constraints
        assert!(output.content.contains("FILTER"));
        assert!(output.content.contains("REGEX")); // For email pattern
        assert!(output.content.contains(">=")); // For age minimum
        assert!(output.content.contains("<=")); // For age maximum
    }
}