//! RDF generator for LinkML schemas
//!
//! This generator produces plain RDF/Turtle representation of LinkML schemas,
//! focusing on the data model rather than OWL ontology features.

use super::traits::{Generator, GeneratorOptions, GeneratorResult, GeneratedOutput};
use linkml_core::prelude::*;
use async_trait::async_trait;
use std::fmt::Write;

/// RDF/Turtle generator for LinkML schemas
pub struct RdfGenerator {
    /// Base URI for the schema
    base_uri: Option<String>,
    /// Whether to include metadata properties
    include_metadata: bool,
    /// Whether to use compact Turtle syntax
    compact_syntax: bool,
    /// Whether to include LinkML-specific properties
    include_linkml_props: bool,
}

impl Default for RdfGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl RdfGenerator {
    /// Create a new RDF generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            base_uri: None,
            include_metadata: true,
            compact_syntax: true,
            include_linkml_props: true,
        }
    }

    /// Set the base URI
    #[must_use]
    pub fn with_base_uri(mut self, uri: String) -> Self {
        self.base_uri = Some(uri);
        self
    }

    /// Configure metadata inclusion
    #[must_use]
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Generate RDF/Turtle from schema
    fn generate_rdf(&self, schema: &SchemaDefinition) -> Result<String> {
        let mut output = String::new();
        
        // Determine base URI
        let base_uri = self.base_uri.as_ref()
            .map(|s| s.as_str())
            .or_else(|| schema.id.strip_suffix('/').or(Some(&schema.id)))
            .unwrap_or(&format!("https://example.org/{}", schema.name));

        // Write prefixes
        self.write_prefixes(&mut output, schema, base_uri)?;
        
        // Write schema metadata
        self.write_schema_metadata(&mut output, schema, base_uri)?;
        
        // Write classes
        for (name, class) in &schema.classes {
            self.write_class(&mut output, name, class, base_uri)?;
        }
        
        // Write slots as properties
        for (name, slot) in &schema.slots {
            self.write_slot(&mut output, name, slot, base_uri)?;
        }
        
        // Write types
        for (name, type_def) in &schema.types {
            self.write_type(&mut output, name, type_def, base_uri)?;
        }
        
        // Write enums
        for (name, enum_def) in &schema.enums {
            self.write_enum(&mut output, name, enum_def, base_uri)?;
        }
        
        Ok(output)
    }

    /// Write prefix declarations
    fn write_prefixes(&self, output: &mut String, schema: &SchemaDefinition, base_uri: &str) -> Result<()> {
        // Standard prefixes
        writeln!(output, "@prefix : <{}/> .", base_uri)?;
        writeln!(output, "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .")?;
        writeln!(output, "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .")?;
        writeln!(output, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .")?;
        writeln!(output, "@prefix dcterms: <http://purl.org/dc/terms/> .")?;
        writeln!(output, "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .")?;
        writeln!(output, "@prefix sh: <http://www.w3.org/ns/shacl#> .")?;
        
        if self.include_linkml_props {
            writeln!(output, "@prefix linkml: <https://w3id.org/linkml/> .")?;
        }
        
        // Schema-specific prefixes
        for (prefix, def) in &schema.prefixes {
            writeln!(output, "@prefix {}: <{}> .", prefix, def.prefix_reference)?;
        }
        
        writeln!(output)?; // Blank line
        Ok(())
    }

    /// Write schema metadata
    fn write_schema_metadata(&self, output: &mut String, schema: &SchemaDefinition, base_uri: &str) -> Result<()> {
        writeln!(output, "# Schema: {}", schema.name)?;
        writeln!(output, "<{}> a linkml:SchemaDefinition ;", base_uri)?;
        
        // Basic metadata
        writeln!(output, "    rdfs:label \"{}\" ;", schema.name)?;
        
        if let Some(title) = &schema.title {
            writeln!(output, "    dcterms:title \"{}\" ;", escape_literal(title))?;
        }
        
        if let Some(description) = &schema.description {
            writeln!(output, "    dcterms:description \"{}\" ;", escape_literal(description))?;
        }
        
        if let Some(version) = &schema.version {
            writeln!(output, "    dcterms:hasVersion \"{}\" ;", version)?;
        }
        
        if let Some(license) = &schema.license {
            writeln!(output, "    dcterms:license \"{}\" ;", license)?;
        }
        
        if self.include_metadata {
            if let Some(created_by) = &schema.created_by {
                writeln!(output, "    dcterms:creator \"{}\" ;", created_by)?;
            }
            
            if let Some(created_on) = &schema.created_on {
                writeln!(output, "    dcterms:created \"{}\" ;", created_on)?;
            }
            
            if let Some(modified_by) = &schema.modified_by {
                writeln!(output, "    dcterms:contributor \"{}\" ;", modified_by)?;
            }
            
            if let Some(last_updated_on) = &schema.last_updated_on {
                writeln!(output, "    dcterms:modified \"{}\" ;", last_updated_on)?;
            }
        }
        
        // Contributors
        for contributor in &schema.contributors {
            writeln!(output, "    dcterms:contributor [
        a dcterms:Agent ;
        rdfs:label \"{}\" ;", contributor.name)?;
            
            if let Some(email) = &contributor.email {
                writeln!(output, "        dcterms:identifier \"mailto:{}\" ;", email)?;
            }
            
            writeln!(output, "    ] ;")?;
        }
        
        // See also links
        for see_also in &schema.see_also {
            writeln!(output, "    rdfs:seeAlso <{}> ;", see_also)?;
        }
        
        // Close the schema definition
        writeln!(output, "    .\n")?;
        
        Ok(())
    }

    /// Write a class definition
    fn write_class(&self, output: &mut String, name: &str, class: &ClassDefinition, base_uri: &str) -> Result<()> {
        let class_uri = class.class_uri.as_deref()
            .unwrap_or(&format!("{}/{}", base_uri, name));
        
        writeln!(output, "# Class: {}", name)?;
        writeln!(output, "<{}> a rdfs:Class ;", class_uri)?;
        writeln!(output, "    rdfs:label \"{}\" ;", name)?;
        
        if let Some(description) = &class.description {
            writeln!(output, "    rdfs:comment \"{}\" ;", escape_literal(description))?;
        }
        
        // Parent class
        if let Some(is_a) = &class.is_a {
            let parent_uri = format!("{}/{}", base_uri, is_a);
            writeln!(output, "    rdfs:subClassOf <{}> ;", parent_uri)?;
        }
        
        // Mixins as additional superclasses
        for mixin in &class.mixins {
            let mixin_uri = format!("{}/{}", base_uri, mixin);
            writeln!(output, "    rdfs:subClassOf <{}> ;", mixin_uri)?;
        }
        
        // Class properties
        if let Some(abstract_) = class.abstract_ {
            if abstract_ && self.include_linkml_props {
                writeln!(output, "    linkml:abstract true ;")?;
            }
        }
        
        if let Some(mixin) = class.mixin {
            if mixin && self.include_linkml_props {
                writeln!(output, "    linkml:mixin true ;")?;
            }
        }
        
        // Slots as property restrictions
        if self.compact_syntax && !class.slots.is_empty() {
            writeln!(output, "    sh:property [")?;
            for (i, slot_name) in class.slots.iter().enumerate() {
                writeln!(output, "        sh:path :{} ;", slot_name)?;
                if i < class.slots.len() - 1 {
                    writeln!(output, "    ] , [")?;
                }
            }
            writeln!(output, "    ] ;")?;
        }
        
        writeln!(output, "    .\n")?;
        Ok(())
    }

    /// Write a slot definition as an RDF property
    fn write_slot(&self, output: &mut String, name: &str, slot: &SlotDefinition, base_uri: &str) -> Result<()> {
        let slot_uri = slot.slot_uri.as_deref()
            .unwrap_or(&format!("{}/{}", base_uri, name));
        
        writeln!(output, "# Property: {}", name)?;
        writeln!(output, "<{}> a rdf:Property ;", slot_uri)?;
        writeln!(output, "    rdfs:label \"{}\" ;", name)?;
        
        if let Some(description) = &slot.description {
            writeln!(output, "    rdfs:comment \"{}\" ;", escape_literal(description))?;
        }
        
        // Domain and range
        if let Some(domain) = &slot.domain {
            writeln!(output, "    rdfs:domain :{} ;", domain)?;
        }
        
        if let Some(range) = &slot.range {
            let range_uri = map_range_to_xsd(range);
            writeln!(output, "    rdfs:range {} ;", range_uri)?;
        }
        
        // Parent slot
        if let Some(is_a) = &slot.is_a {
            writeln!(output, "    rdfs:subPropertyOf :{} ;", is_a)?;
        }
        
        // Constraints as SHACL
        if self.compact_syntax {
            if let Some(required) = slot.required {
                if required {
                    writeln!(output, "    sh:minCount 1 ;")?;
                }
            }
            
            if let Some(multivalued) = slot.multivalued {
                if !multivalued {
                    writeln!(output, "    sh:maxCount 1 ;")?;
                }
            }
            
            if let Some(pattern) = &slot.pattern {
                writeln!(output, "    sh:pattern \"{}\" ;", escape_literal(pattern))?;
            }
        }
        
        writeln!(output, "    .\n")?;
        Ok(())
    }

    /// Write a type definition
    fn write_type(&self, output: &mut String, name: &str, type_def: &TypeDefinition, base_uri: &str) -> Result<()> {
        let type_uri = type_def.uri.as_deref()
            .unwrap_or(&format!("{}/{}", base_uri, name));
        
        writeln!(output, "# Type: {}", name)?;
        writeln!(output, "<{}> a rdfs:Datatype ;", type_uri)?;
        writeln!(output, "    rdfs:label \"{}\" ;", name)?;
        
        if let Some(description) = &type_def.description {
            writeln!(output, "    rdfs:comment \"{}\" ;", escape_literal(description))?;
        }
        
        if let Some(base) = &type_def.base {
            let base_uri = map_range_to_xsd(base);
            writeln!(output, "    rdfs:subClassOf {} ;", base_uri)?;
        }
        
        writeln!(output, "    .\n")?;
        Ok(())
    }

    /// Write an enum definition
    fn write_enum(&self, output: &mut String, name: &str, enum_def: &EnumDefinition, base_uri: &str) -> Result<()> {
        let enum_uri = enum_def.enum_uri.as_deref()
            .unwrap_or(&format!("{}/{}", base_uri, name));
        
        writeln!(output, "# Enumeration: {}", name)?;
        writeln!(output, "<{}> a rdfs:Class ;", enum_uri)?;
        writeln!(output, "    rdfs:label \"{}\" ;", name)?;
        
        if let Some(description) = &enum_def.description {
            writeln!(output, "    rdfs:comment \"{}\" ;", escape_literal(description))?;
        }
        
        // Define as enumeration using SHACL
        if self.compact_syntax && !enum_def.permissible_values.is_empty() {
            writeln!(output, "    sh:in (")?;
            for (text, _pv) in &enum_def.permissible_values {
                writeln!(output, "        \"{}\"", escape_literal(text))?;
            }
            writeln!(output, "    ) ;")?;
        }
        
        writeln!(output, "    .\n")?;
        
        // Individual permissible values as instances
        for (text, pv) in &enum_def.permissible_values {
            writeln!(output, ":{} a <{}> ;", 
                text.replace(' ', "_"), enum_uri)?;
            writeln!(output, "    rdfs:label \"{}\" ;", text)?;
            
            if let Some(description) = &pv.description {
                writeln!(output, "    rdfs:comment \"{}\" ;", escape_literal(description))?;
            }
            
            if let Some(meaning) = &pv.meaning {
                writeln!(output, "    skos:exactMatch <{}> ;", meaning)?;
            }
            
            writeln!(output, "    .\n")?;
        }
        
        Ok(())
    }
}

/// Map LinkML range to XSD datatype
fn map_range_to_xsd(range: &str) -> String {
    match range {
        "string" => "xsd:string".to_string(),
        "integer" => "xsd:integer".to_string(),
        "float" => "xsd:float".to_string(),
        "double" => "xsd:double".to_string(),
        "boolean" => "xsd:boolean".to_string(),
        "date" => "xsd:date".to_string(),
        "datetime" => "xsd:dateTime".to_string(),
        "time" => "xsd:time".to_string(),
        "uri" => "xsd:anyURI".to_string(),
        "uriorcurie" => "xsd:anyURI".to_string(),
        _ => format!(":{}", range), // Assume it's a class reference
    }
}

/// Escape literal values for Turtle
fn escape_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
     .replace('"', "\\\"")
     .replace('\n', "\\n")
     .replace('\r', "\\r")
     .replace('\t', "\\t")
}

#[async_trait]
impl Generator for RdfGenerator {
    async fn generate(&self, schema: &SchemaDefinition, _options: &GeneratorOptions) -> Result<GeneratorResult> {
        let rdf_content = self.generate_rdf(schema)?;
        
        Ok(GeneratorResult {
            outputs: vec![GeneratedOutput {
                filename: format!("{}.ttl", schema.name),
                content: rdf_content,
            }],
        })
    }

    fn name(&self) -> &'static str {
        "rdf"
    }

    fn description(&self) -> &'static str {
        "Generate RDF/Turtle representation of LinkML schema"
    }
}