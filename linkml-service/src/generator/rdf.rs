//! RDF generator for `LinkML` schemas
//!
//! This generator produces plain RDF/Turtle representation of `LinkML` schemas,
//! focusing on the data model rather than OWL ontology features.

use super::traits::Generator;
use linkml_core::prelude::*;
use std::fmt::Write;

/// Helper macro to convert `fmt::Error` to `LinkML`Error with newline
macro_rules! writeln_rdf {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).map_err(|e| LinkMLError::ServiceError(format!("Failed to write RDF: {e}")))
    };
}

/// RDF/Turtle generator for `LinkML` schemas
pub struct RdfGenerator {
    /// Base URI for the schema
    base_uri: Option<String>,
    /// Whether to include metadata properties
    include_metadata: bool,
    /// Whether to use compact Turtle syntax
    compact_syntax: bool,
    /// Whether to include `LinkML`-specific properties
    include_linkml_props: bool,
    /// Generator options
    options: super::traits::GeneratorOptions,
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
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Create a new RDF generator in RDFS mode
    #[must_use]
    pub fn rdfs() -> Self {
        Self {
            base_uri: None,
            include_metadata: true,
            compact_syntax: false,
            include_linkml_props: false,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create a new RDF generator in simple mode
    #[must_use]
    pub fn simple() -> Self {
        Self {
            base_uri: None,
            include_metadata: false,
            compact_syntax: true,
            include_linkml_props: false,
            options: super::traits::GeneratorOptions::default(),
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
        let default_uri = format!("https://example.org/{}", schema.name);
        let base_uri = self
            .base_uri
            .as_deref()
            .or_else(|| schema.id.strip_suffix('/').or(Some(&schema.id)))
            .unwrap_or(&default_uri);

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
            self.write_slot(&mut output, name, slot, schema, base_uri)?;
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
    fn write_prefixes(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        base_uri: &str,
    ) -> Result<()> {
        // Standard prefixes
        writeln_rdf!(output, "@prefix : <{}/> .", base_uri)?;
        writeln_rdf!(
            output,
            "@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> ."
        )?;
        writeln_rdf!(
            output,
            "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> ."
        )?;
        writeln_rdf!(output, "@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .")?;
        writeln_rdf!(output, "@prefix dcterms: <http://purl.org/dc/terms/> .")?;
        writeln_rdf!(
            output,
            "@prefix skos: <http://www.w3.org/2004/02/skos/core#> ."
        )?;
        writeln_rdf!(output, "@prefix sh: <http://www.w3.org/ns/shacl#> .")?;

        if self.include_linkml_props {
            writeln_rdf!(output, "@prefix linkml: <https://w3id.org/linkml/> .")?;
        }

        // Schema-specific prefixes
        for (prefix, def) in &schema.prefixes {
            let reference = match def {
                PrefixDefinition::Simple(url) => url.clone(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.clone().unwrap_or_default(),
            };
            writeln_rdf!(output, "@prefix {}: <{}> .", prefix, reference)?;
        }

        writeln_rdf!(output, "")?; // Blank line
        Ok(())
    }

    /// Write schema metadata
    fn write_schema_metadata(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
        base_uri: &str,
    ) -> Result<()> {
        writeln_rdf!(output, "# Schema: {}", schema.name)?;
        writeln_rdf!(output, "<{}> a linkml:SchemaDefinition ;", base_uri)?;

        // Basic metadata
        writeln_rdf!(output, "    rdfs:label \"{}\" ;", schema.name)?;

        if let Some(title) = &schema.title {
            writeln_rdf!(output, "    dcterms:title \"{}\" ;", escape_literal(title))?;
        }

        if let Some(description) = &schema.description {
            writeln_rdf!(
                output,
                "    dcterms:description \"{}\" ;",
                escape_literal(description)
            )?;
        }

        if let Some(version) = &schema.version {
            writeln_rdf!(output, "    dcterms:hasVersion \"{}\" ;", version)?;
        }

        if let Some(license) = &schema.license {
            writeln_rdf!(output, "    dcterms:license \"{}\" ;", license)?;
        }

        if self.include_metadata {
            // Add available metadata fields
            if let Some(generation_date) = &schema.generation_date {
                writeln_rdf!(output, "    dcterms:created \"{}\" ;", generation_date)?;
            }

            if let Some(source_file) = &schema.source_file {
                writeln_rdf!(output, "    dcterms:source \"{}\" ;", source_file)?;
            }

            if let Some(metamodel_version) = &schema.metamodel_version {
                writeln_rdf!(
                    output,
                    "    linkml:metamodelVersion \"{}\" ;",
                    metamodel_version
                )?;
            }

            if let Some(status) = &schema.status {
                writeln_rdf!(output, "    dcterms:conformsTo \"{}\" ;", status)?;
            }

            // Categories as subject classification
            for category in &schema.categories {
                writeln_rdf!(output, "    dcterms:subject \"{}\" ;", category)?;
            }

            // Keywords
            for keyword in &schema.keywords {
                writeln_rdf!(output, "    dcat:keyword \"{}\" ;", keyword)?;
            }
        }

        // Contributors
        for contributor in &schema.contributors {
            writeln_rdf!(
                output,
                "    dcterms:contributor [
        a dcterms:Agent ;
        rdfs:label \"{}\" ;",
                contributor.name
            )?;

            if let Some(email) = &contributor.email {
                writeln_rdf!(output, "        dcterms:identifier \"mailto:{}\" ;", email)?;
            }

            writeln_rdf!(output, "    ] ;")?;
        }

        // See also links
        for see_also in &schema.see_also {
            writeln_rdf!(output, "    rdfs:seeAlso <{}> ;", see_also)?;
        }

        // Close the schema definition
        writeln_rdf!(
            output, "    .
"
        )?;

        Ok(())
    }

    /// Write a class definition
    fn write_class(
        &self,
        output: &mut String,
        name: &str,
        class: &ClassDefinition,
        base_uri: &str,
    ) -> Result<()> {
        let default_uri = format!("{base_uri}/{name}");
        let class_uri = class.class_uri.as_deref().unwrap_or(&default_uri);

        writeln_rdf!(output, "# Class: {}", name)?;
        writeln_rdf!(output, "<{}> a rdfs:Class ;", class_uri)?;
        writeln_rdf!(output, "    rdfs:label \"{}\" ;", name)?;

        if let Some(description) = &class.description {
            writeln_rdf!(
                output,
                "    rdfs:comment \"{}\" ;",
                escape_literal(description)
            )?;
        }

        // Parent class
        if let Some(is_a) = &class.is_a {
            let parent_uri = format!("{base_uri}/{is_a}");
            writeln_rdf!(output, "    rdfs:subClassOf <{}> ;", parent_uri)?;
        }

        // Mixins as additional superclasses
        for mixin in &class.mixins {
            let mixin_uri = format!("{base_uri}/{mixin}");
            writeln_rdf!(output, "    rdfs:subClassOf <{}> ;", mixin_uri)?;
        }

        // Class properties
        if let Some(abstract_) = class.abstract_
            && abstract_
            && self.include_linkml_props
        {
            writeln_rdf!(output, "    linkml:abstract true ;")?;
        }

        if let Some(mixin) = class.mixin
            && mixin
            && self.include_linkml_props
        {
            writeln_rdf!(output, "    linkml:mixin true ;")?;
        }

        // Slots as property restrictions
        if self.compact_syntax && !class.slots.is_empty() {
            writeln_rdf!(output, "    sh:property [")?;
            for (i, slot_name) in class.slots.iter().enumerate() {
                writeln_rdf!(output, "        sh:path :{} ;", slot_name)?;
                if i < class.slots.len() - 1 {
                    writeln_rdf!(output, "    ] , [")?;
                }
            }
            writeln_rdf!(output, "    ] ;")?;
        }

        writeln_rdf!(
            output, "    .
"
        )?;
        Ok(())
    }

    /// Write a slot definition as an RDF property
    fn write_slot(
        &self,
        output: &mut String,
        name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        base_uri: &str,
    ) -> Result<()> {
        let default_uri = format!("{base_uri}/{name}");
        let slot_uri = slot.slot_uri.as_deref().unwrap_or(&default_uri);

        writeln_rdf!(output, "# Property: {}", name)?;
        writeln_rdf!(output, "<{}> a rdf:Property ;", slot_uri)?;
        writeln_rdf!(output, "    rdfs:label \"{}\" ;", name)?;

        if let Some(description) = &slot.description {
            writeln_rdf!(
                output,
                "    rdfs:comment \"{}\" ;",
                escape_literal(description)
            )?;
        }

        // Domain - compute from classes that use this slot
        let mut domains = Vec::new();
        for (class_name, class) in &schema.classes {
            if class.slots.contains(&name.to_string()) {
                domains.push(class_name.clone());
            }
        }

        if domains.len() == 1 {
            // Single domain
            writeln_rdf!(output, "    rdfs:domain :{} ;", domains[0])?;
        } else if domains.len() > 1 {
            // Multiple domains - use owl:unionOf
            writeln_rdf!(output, "    rdfs:domain [")?;
            writeln_rdf!(output, "        a owl:Class ;")?;
            writeln_rdf!(output, "        owl:unionOf (")?;
            for domain in &domains {
                writeln_rdf!(output, "            :{}", domain)?;
            }
            writeln_rdf!(output, "        )")?;
            writeln_rdf!(output, "    ] ;")?;
        }

        if let Some(range) = &slot.range {
            let range_uri = map_range_to_xsd(range);
            writeln_rdf!(output, "    rdfs:range {} ;", range_uri)?;
        }

        // Parent slot
        if let Some(is_a) = &slot.is_a {
            writeln_rdf!(output, "    rdfs:subPropertyOf :{} ;", is_a)?;
        }

        // Constraints as SHACL
        if self.compact_syntax {
            if let Some(required) = slot.required
                && required
            {
                writeln_rdf!(output, "    sh:minCount 1 ;")?;
            }

            if let Some(multivalued) = slot.multivalued
                && !multivalued
            {
                writeln_rdf!(output, "    sh:maxCount 1 ;")?;
            }

            if let Some(pattern) = &slot.pattern {
                writeln_rdf!(output, "    sh:pattern \"{}\" ;", escape_literal(pattern))?;
            }
        }

        writeln_rdf!(
            output, "    .
"
        )?;
        Ok(())
    }

    /// Write a type definition
    fn write_type(
        &self,
        output: &mut String,
        name: &str,
        type_def: &TypeDefinition,
        base_uri: &str,
    ) -> Result<()> {
        let default_uri = format!("{base_uri}/{name}");
        let type_uri = type_def.uri.as_deref().unwrap_or(&default_uri);

        writeln_rdf!(output, "# Type: {}", name)?;
        writeln_rdf!(output, "<{}> a rdfs:Datatype ;", type_uri)?;
        writeln_rdf!(output, "    rdfs:label \"{}\" ;", name)?;

        if let Some(description) = &type_def.description {
            writeln_rdf!(
                output,
                "    rdfs:comment \"{}\" ;",
                escape_literal(description)
            )?;
        }

        if let Some(base_type) = &type_def.base_type {
            let base_uri = map_range_to_xsd(base_type);
            writeln_rdf!(output, "    rdfs:subClassOf {} ;", base_uri)?;
        }

        writeln_rdf!(
            output, "    .
"
        )?;
        Ok(())
    }

    /// Write an enum definition
    fn write_enum(
        &self,
        output: &mut String,
        name: &str,
        enum_def: &EnumDefinition,
        base_uri: &str,
    ) -> Result<()> {
        // Generate enum URI - use code_set if available, otherwise construct from base_uri
        let enum_uri = if let Some(code_set) = &enum_def.code_set {
            // If code_set is a full URI, use it directly
            if code_set.starts_with("http://") || code_set.starts_with("https://") {
                code_set.clone()
            } else {
                // Otherwise construct URI from base and code_set
                format!("{base_uri}/{code_set}")
            }
        } else {
            // Default to base_uri + enum name
            format!("{base_uri}/enums/{name}")
        };

        writeln_rdf!(output, "# Enumeration: {}", name)?;
        writeln_rdf!(output, "<{}> a rdfs:Class ;", enum_uri)?;
        writeln_rdf!(output, "    rdfs:label \"{}\" ;", name)?;

        if let Some(description) = &enum_def.description {
            writeln_rdf!(
                output,
                "    rdfs:comment \"{}\" ;",
                escape_literal(description)
            )?;
        }

        // Define as enumeration using SHACL
        if self.compact_syntax && !enum_def.permissible_values.is_empty() {
            writeln_rdf!(output, "    sh:in (")?;
            for pv in &enum_def.permissible_values {
                let text = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                writeln_rdf!(output, "        \"{}\"", escape_literal(text))?;
            }
            writeln_rdf!(output, "    ) ;")?;
        }

        writeln_rdf!(
            output, "    .
"
        )?;

        // Individual permissible values as instances
        for pv in &enum_def.permissible_values {
            let (text, description, meaning) = match pv {
                PermissibleValue::Simple(s) => (s.as_str(), None, None),
                PermissibleValue::Complex {
                    text,
                    description,
                    meaning,
                } => (text.as_str(), description.as_deref(), meaning.as_deref()),
            };
            writeln_rdf!(output, ":{} a <{}> ;", text.replace(' ', "_"), enum_uri)?;
            writeln_rdf!(output, "    rdfs:label \"{}\" ;", text)?;

            if let Some(desc) = description {
                writeln_rdf!(output, "    rdfs:comment \"{}\" ;", escape_literal(desc))?;
            }

            if let Some(mean) = meaning {
                writeln_rdf!(output, "    skos:exactMatch <{}> ;", mean)?;
            }

            writeln_rdf!(
                output, "    .
"
            )?;
        }

        Ok(())
    }
}

/// Map `LinkML` range to XSD datatype
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
        "uri" | "uriorcurie" => "xsd:anyURI".to_string(),
        _ => format!(":{range}"), // Assume it's a class reference
    }
}

/// Escape literal values for Turtle
fn escape_literal(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(
            '\n', "\
",
        )
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

impl Generator for RdfGenerator {
    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for rdf generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        self.generate_rdf(schema)
    }

    fn name(&self) -> &'static str {
        "rdf"
    }

    fn description(&self) -> &'static str {
        "Generate RDF/Turtle representation of LinkML schema"
    }

    fn get_file_extension(&self) -> &'static str {
        "ttl"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema.ttl"
    }
}
