//! OWL/RDF generator for LinkML schemas
//!
//! This module generates OWL (Web Ontology Language) in RDF/Turtle format from LinkML schemas.
//! OWL is a W3C standard for creating ontologies with rich semantics.

use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, PermissibleValue};
use std::collections::HashMap;
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

/// OWL/RDF generator for semantic web ontologies
pub struct OwlRdfGenerator {
    /// Generator options
    options: GeneratorOptions,
    /// Namespace prefixes
    prefixes: HashMap<String, String>,
}

impl OwlRdfGenerator {
    /// Create a new OWL/RDF generator
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();
        
        // Standard prefixes
        prefixes.insert("owl".to_string(), "http://www.w3.org/2002/07/owl#".to_string());
        prefixes.insert("rdf".to_string(), "http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string());
        prefixes.insert("rdfs".to_string(), "http://www.w3.org/2000/01/rdf-schema#".to_string());
        prefixes.insert("xsd".to_string(), "http://www.w3.org/2001/XMLSchema#".to_string());
        prefixes.insert("skos".to_string(), "http://www.w3.org/2004/02/skos/core#".to_string());
        prefixes.insert("dcterms".to_string(), "http://purl.org/dc/terms/".to_string());
        
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
    fn generate_prefixes(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        // Standard prefixes
        for (prefix, uri) in &self.prefixes {
            writeln!(&mut output, "@prefix {}: <{}> .", prefix, uri).unwrap();
        }
        
        // Schema-specific prefix
        let schema_prefix = self.to_snake_case(&schema.name);
        writeln!(&mut output, "@prefix {}: <{}#> .", schema_prefix, schema.id).unwrap();
        writeln!(&mut output).unwrap();
        
        output
    }
    
    /// Generate ontology header
    fn generate_ontology_header(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();
        
        writeln!(&mut output, "# OWL Ontology generated from LinkML schema: {}", schema.name).unwrap();
        writeln!(&mut output).unwrap();
        
        // Ontology declaration
        writeln!(&mut output, "<{}>", schema.id).unwrap();
        writeln!(&mut output, "    a owl:Ontology ;").unwrap();
        writeln!(&mut output, "    rdfs:label \"{}\" ;", schema.name).unwrap();
        
        if let Some(version) = &schema.version {
            writeln!(&mut output, "    owl:versionInfo \"{}\" ;", version).unwrap();
        }
        
        if let Some(desc) = &schema.description {
            writeln!(&mut output, "    dcterms:description \"{}\" ;", desc).unwrap();
        }
        
        writeln!(&mut output, "    .").unwrap();
        writeln!(&mut output).unwrap();
        
        output
    }
    
    /// Generate OWL class from LinkML class
    fn generate_class(&self, name: &str, class: &ClassDefinition, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let class_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));
        
        // Class declaration
        writeln!(&mut output, "# Class: {}", name).unwrap();
        writeln!(&mut output, "{}", class_uri).unwrap();
        writeln!(&mut output, "    a owl:Class ;").unwrap();
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name).unwrap();
        
        // Description
        if let Some(desc) = &class.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc).unwrap();
        }
        
        // Superclass (is_a)
        if let Some(parent) = &class.is_a {
            writeln!(&mut output, "    rdfs:subClassOf {}:{} ;", schema_prefix, self.to_pascal_case(parent)).unwrap();
        }
        
        // Mixins as additional superclasses
        for mixin in &class.mixins {
            writeln!(&mut output, "    rdfs:subClassOf {}:{} ;", schema_prefix, self.to_pascal_case(mixin)).unwrap();
        }
        
        // Collect all slots (including inherited)
        let all_slots = self.collect_all_slots(class, schema);
        
        // Generate property restrictions for slots
        if !all_slots.is_empty() {
            for (i, slot_name) in all_slots.iter().enumerate() {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let restriction = self.generate_property_restriction(slot_name, slot, schema)?;
                    write!(&mut output, "    rdfs:subClassOf {}", restriction).unwrap();
                    if i < all_slots.len() - 1 {
                        writeln!(&mut output, " ,").unwrap();
                    } else {
                        writeln!(&mut output, " .").unwrap();
                    }
                }
            }
        } else {
            writeln!(&mut output, "    .").unwrap();
        }
        
        writeln!(&mut output).unwrap();
        
        Ok(output)
    }
    
    /// Generate property restriction for a slot
    fn generate_property_restriction(&self, slot_name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let schema_prefix = self.to_snake_case(&schema.name);
        let property_uri = format!("{}:{}", schema_prefix, self.to_snake_case(slot_name));
        
        let mut restriction = String::new();
        writeln!(&mut restriction, "[").unwrap();
        writeln!(&mut restriction, "        a owl:Restriction ;").unwrap();
        writeln!(&mut restriction, "        owl:onProperty {} ;", property_uri).unwrap();
        
        // Cardinality constraints
        if slot.required == Some(true) {
            if slot.multivalued == Some(true) {
                writeln!(&mut restriction, "        owl:minCardinality 1").unwrap();
            } else {
                writeln!(&mut restriction, "        owl:cardinality 1").unwrap();
            }
        } else if slot.multivalued != Some(true) {
            writeln!(&mut restriction, "        owl:maxCardinality 1").unwrap();
        }
        
        write!(&mut restriction, "    ]").unwrap();
        
        Ok(restriction)
    }
    
    /// Generate OWL property from LinkML slot
    fn generate_property(&self, name: &str, slot: &SlotDefinition, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let property_uri = format!("{}:{}", schema_prefix, self.to_snake_case(name));
        
        writeln!(&mut output, "# Property: {}", name).unwrap();
        writeln!(&mut output, "{}", property_uri).unwrap();
        
        // Determine property type
        let property_type = if let Some(range) = &slot.range {
            if schema.classes.contains_key(range) {
                "owl:ObjectProperty"
            } else {
                "owl:DatatypeProperty"
            }
        } else {
            "owl:DatatypeProperty"
        };
        
        writeln!(&mut output, "    a {} ;", property_type).unwrap();
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name).unwrap();
        
        // Description
        if let Some(desc) = &slot.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc).unwrap();
        }
        
        // Domain (classes that use this property)
        let using_classes: Vec<String> = schema.classes.iter()
            .filter(|(_, class)| {
                class.slots.contains(&name.to_string()) || 
                self.collect_all_slots(class, schema).contains(&name.to_string())
            })
            .map(|(class_name, _)| format!("{}:{}", schema_prefix, self.to_pascal_case(class_name)))
            .collect();
        
        if !using_classes.is_empty() {
            if using_classes.len() == 1 {
                writeln!(&mut output, "    rdfs:domain {} ;", using_classes[0]).unwrap();
            } else {
                writeln!(&mut output, "    rdfs:domain [").unwrap();
                writeln!(&mut output, "        a owl:Class ;").unwrap();
                writeln!(&mut output, "        owl:unionOf ({})", using_classes.join(" ")).unwrap();
                writeln!(&mut output, "    ] ;").unwrap();
            }
        }
        
        // Range
        if let Some(range) = &slot.range {
            if let Some(datatype) = self.get_xsd_datatype(range) {
                writeln!(&mut output, "    rdfs:range {} ;", datatype).unwrap();
            } else if schema.classes.contains_key(range) {
                writeln!(&mut output, "    rdfs:range {}:{} ;", schema_prefix, self.to_pascal_case(range)).unwrap();
            } else if schema.enums.contains_key(range) {
                writeln!(&mut output, "    rdfs:range {}:{} ;", schema_prefix, self.to_pascal_case(range)).unwrap();
            }
        }
        
        // Functional property (not multivalued)
        if slot.multivalued != Some(true) {
            writeln!(&mut output, "    a owl:FunctionalProperty ;").unwrap();
        }
        
        // Pattern as OWL restriction
        if let Some(pattern) = &slot.pattern {
            writeln!(&mut output, "    owl:withRestrictions ([").unwrap();
            writeln!(&mut output, "        xsd:pattern \"{}\"", pattern).unwrap();
            writeln!(&mut output, "    ]) ;").unwrap();
        }
        
        // Remove trailing semicolon and add period
        if output.ends_with(" ;\n") {
            output.truncate(output.len() - 3);
            writeln!(&mut output, " .").unwrap();
        }
        
        writeln!(&mut output).unwrap();
        
        Ok(output)
    }
    
    /// Generate OWL class for enum
    fn generate_enum(&self, name: &str, enum_def: &EnumDefinition, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let enum_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));
        
        writeln!(&mut output, "# Enumeration: {}", name).unwrap();
        writeln!(&mut output, "{}", enum_uri).unwrap();
        writeln!(&mut output, "    a owl:Class ;").unwrap();
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name).unwrap();
        
        if let Some(desc) = &enum_def.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc).unwrap();
        }
        
        // Create individuals for each permissible value
        let individuals: Vec<String> = enum_def.permissible_values.iter()
            .map(|pv| {
                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                format!("{}:{}_{}", schema_prefix, self.to_pascal_case(name), self.to_pascal_case(value))
            })
            .collect();
        
        writeln!(&mut output, "    owl:equivalentClass [").unwrap();
        writeln!(&mut output, "        a owl:Class ;").unwrap();
        writeln!(&mut output, "        owl:oneOf ({})", individuals.join(" ")).unwrap();
        writeln!(&mut output, "    ] .").unwrap();
        writeln!(&mut output).unwrap();
        
        // Generate individuals
        for pv in &enum_def.permissible_values {
            let (value, desc) = match pv {
                PermissibleValue::Simple(s) => (s.clone(), None),
                PermissibleValue::Complex { text, description, .. } => (text.clone(), description.clone()),
            };
            
            let individual_uri = format!("{}:{}_{}", schema_prefix, self.to_pascal_case(name), self.to_pascal_case(&value));
            
            writeln!(&mut output, "{}", individual_uri).unwrap();
            writeln!(&mut output, "    a {} ;", enum_uri).unwrap();
            writeln!(&mut output, "    rdfs:label \"{}\" ;", value).unwrap();
            
            if let Some(desc) = desc {
                writeln!(&mut output, "    skos:definition \"{}\" ;", desc).unwrap();
            }
            
            writeln!(&mut output, "    .").unwrap();
            writeln!(&mut output).unwrap();
        }
        
        Ok(output)
    }
    
    /// Collect all slots including inherited ones
    fn collect_all_slots(&self, class: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();
        
        // First, get slots from parent if any
        if let Some(parent_name) = &class.is_a {
            if let Some(parent_class) = schema.classes.get(parent_name) {
                all_slots.extend(self.collect_all_slots(parent_class, schema));
            }
        }
        
        // Then add direct slots
        all_slots.extend(class.slots.clone());
        
        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        all_slots.retain(|slot| seen.insert(slot.clone()));
        
        all_slots
    }
    
    /// Get XSD datatype for LinkML range
    fn get_xsd_datatype(&self, range: &str) -> Option<String> {
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
    
    /// Convert to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;
        
        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
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

impl Default for OwlRdfGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for OwlRdfGenerator {
    fn name(&self) -> &str {
        "owl-rdf"
    }
    
    fn description(&self) -> &str {
        "Generates OWL ontology in RDF/Turtle format from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".owl", ".ttl"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let mut output = String::new();
        
        // Generate header
        output.push_str(&self.generate_ontology_header(schema));
        
        // Generate prefixes
        output.push_str(&self.generate_prefixes(schema));
        
        // Generate classes
        for (name, class) in &schema.classes {
            let class_def = self.generate_class(name, class, schema)
                .map_err(|e| GeneratorError::Generation {
                    context: format!("class {}", name),
                    message: e.to_string(),
                })?;
            output.push_str(&class_def);
        }
        
        // Generate properties
        for (name, slot) in &schema.slots {
            let property_def = self.generate_property(name, slot, schema)
                .map_err(|e| GeneratorError::Generation {
                    context: format!("property {}", name),
                    message: e.to_string(),
                })?;
            output.push_str(&property_def);
        }
        
        // Generate enums
        for (name, enum_def) in &schema.enums {
            let enum_class = self.generate_enum(name, enum_def, schema)
                .map_err(|e| GeneratorError::Generation {
                    context: format!("enum {}", name),
                    message: e.to_string(),
                })?;
            output.push_str(&enum_class);
        }
        
        // Create output
        let filename = format!("{}.owl", self.to_snake_case(&schema.name));
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "turtle".to_string());
        metadata.insert("schema".to_string(), schema.name.clone());
        metadata.insert("ontology".to_string(), "owl2".to_string());
        
        Ok(vec![GeneratedOutput {
            filename,
            content: output,
            metadata,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_xsd_datatype_mapping() {
        let generator = OwlRdfGenerator::new();
        
        assert_eq!(generator.get_xsd_datatype("string"), Some("xsd:string".to_string()));
        assert_eq!(generator.get_xsd_datatype("integer"), Some("xsd:integer".to_string()));
        assert_eq!(generator.get_xsd_datatype("boolean"), Some("xsd:boolean".to_string()));
        assert_eq!(generator.get_xsd_datatype("datetime"), Some("xsd:dateTime".to_string()));
        assert_eq!(generator.get_xsd_datatype("CustomType"), None);
    }
    
    #[test]
    fn test_case_conversion() {
        let generator = OwlRdfGenerator::new();
        
        assert_eq!(generator.to_snake_case("PersonName"), "person_name");
        assert_eq!(generator.to_pascal_case("person_name"), "PersonName");
    }
}