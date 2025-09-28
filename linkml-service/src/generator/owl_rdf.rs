//! RDF generator for LinkML schemas
//!
//! This module generates RDF representations from LinkML schemas in multiple formats:
//! - OWL ontologies (default, with rich semantics)
//! - Simple RDF Schema (RDFS)
//! - Pure RDF triples
//!
//! Supported output formats:
//! - Turtle (.ttl) - default
//! - RDF/XML (.rdf)
//! - N-Triples (.nt)
//! - JSON-LD (.jsonld)

use linkml_core::{
    error::LinkMLError,
    types::{ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition}};
use serde_json;
use std::collections::HashMap;
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult};

/// RDF output format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RdfFormat {
    /// Turtle format (default)
    Turtle,
    /// RDF/`XML` format
    RdfXml,
    /// N-Triples format
    NTriples,
    /// `JSON`-LD format
    JsonLd}

/// RDF generation mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RdfMode {
    /// Full OWL ontology with restrictions
    Owl,
    /// Simple RDFS schema
    Rdfs,
    /// Plain RDF triples
    Simple}

/// RDF generator for semantic web representations
pub struct RdfGenerator {
    /// Generator options
    options: GeneratorOptions,
    /// Namespace prefixes
    prefixes: HashMap<String, String>,
    /// Output format
    format: RdfFormat,
    /// Generation mode
    mode: RdfMode}

/// Alias for backward compatibility
pub type OwlRdfGenerator = RdfGenerator;

impl RdfGenerator {
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new RDF generator (defaults to OWL mode, Turtle format)
    #[must_use]
    pub fn new() -> Self {
        let mut prefixes = HashMap::new();

        // Standard prefixes
        prefixes.insert(
            "owl".to_string(),
            "http://www.w3.org/2002/07/owl#".to_string(),
        );
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
            "skos".to_string(),
            "http://www.w3.org/2004/02/skos/core#".to_string(),
        );
        prefixes.insert(
            "dcterms".to_string(),
            "http://purl.org/dc/terms/".to_string(),
        );

        Self {
            options: GeneratorOptions::default(),
            prefixes,
            format: RdfFormat::Turtle,
            mode: RdfMode::Owl}
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GeneratorOptions) -> Self {
        let mut generator = Self::new();
        generator.options = options;
        generator
    }

    /// Set the output format
    #[must_use]
    pub fn with_format(mut self, format: RdfFormat) -> Self {
        self.format = format;
        self
    }

    /// Set the generation mode
    #[must_use]
    pub fn with_mode(mut self, mode: RdfMode) -> Self {
        self.mode = mode;
        self
    }

    /// Create a simple RDF generator
    #[must_use]
    pub fn simple() -> Self {
        Self::new().with_mode(RdfMode::Simple)
    }

    /// Create an RDFS generator
    #[must_use]
    pub fn rdfs() -> Self {
        Self::new().with_mode(RdfMode::Rdfs)
    }

    /// Generate prefixes section based on format
    fn generate_prefixes(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.format {
            RdfFormat::Turtle => self.generate_turtle_prefixes(schema),
            RdfFormat::RdfXml => self.generate_rdfxml_prefixes(schema),
            RdfFormat::NTriples => Ok(String::new()), // N-Triples doesn't use prefixes
            RdfFormat::JsonLd => self.generate_jsonld_context(schema)}
    }

    /// Generate Turtle prefixes
    fn generate_turtle_prefixes(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Standard prefixes
        for (prefix, uri) in &self.prefixes {
            writeln!(&mut output, "@prefix {}: <{}> .", prefix, uri)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Schema-specific prefix
        let schema_prefix = self.to_snake_case(&schema.name);
        writeln!(&mut output, "@prefix {}: <{}#> .", schema_prefix, schema.id)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate RDF/`XML` namespace declarations
    fn generate_rdfxml_prefixes(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut namespaces = String::new();

        // Add standard namespaces
        write!(&mut namespaces, " xmlns:rdf=\"{}\"", self.prefixes["rdf"])
            .map_err(Self::fmt_error_to_generator_error)?;
        write!(&mut namespaces, " xmlns:rdfs=\"{}\"", self.prefixes["rdfs"])
            .map_err(Self::fmt_error_to_generator_error)?;

        if self.mode == RdfMode::Owl {
            write!(&mut namespaces, " xmlns:owl=\"{}\"", self.prefixes["owl"])
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        write!(&mut namespaces, " xmlns:xsd=\"{}\"", self.prefixes["xsd"])
            .map_err(Self::fmt_error_to_generator_error)?;
        write!(&mut namespaces, " xmlns:skos=\"{}\"", self.prefixes["skos"])
            .map_err(Self::fmt_error_to_generator_error)?;
        write!(
            &mut namespaces,
            " xmlns:dcterms=\"{}\"",
            self.prefixes["dcterms"]
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Schema namespace
        let schema_prefix = self.to_snake_case(&schema.name);
        write!(
            &mut namespaces,
            " xmlns:{}=\"{}#\"",
            schema_prefix, schema.id
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        Ok(namespaces)
    }

    /// Generate `JSON`-LD context
    fn generate_jsonld_context(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut context = serde_json::json!({
            "@context": {
                "rdf": self.prefixes["rdf"],
                "rdfs": self.prefixes["rdfs"],
                "xsd": self.prefixes["xsd"],
                "skos": self.prefixes["skos"],
                "dcterms": self.prefixes["dcterms"]}
        });

        if self.mode == RdfMode::Owl {
            context["@context"]["owl"] = serde_json::json!(self.prefixes["owl"]);
        }

        // Add schema-specific namespace
        let schema_prefix = self.to_snake_case(&schema.name);
        context["@context"][schema_prefix] = serde_json::json!(format!("{}#", schema.id));

        Ok(
            serde_json::to_string_pretty(&context).map_err(|e| GeneratorError::Generation(
                format!("json-ld context: {e}")
            ))?,
        )
    }

    /// Generate schema header based on mode and format
    fn generate_schema_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.mode {
            RdfMode::Owl => self.generate_owl_header(schema),
            RdfMode::Rdfs => self.generate_rdfs_header(schema),
            RdfMode::Simple => self.generate_simple_header(schema)}
    }

    /// Generate OWL ontology header
    fn generate_owl_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        match self.format {
            RdfFormat::Turtle => writeln!(
                &mut output,
                "# OWL Ontology generated from LinkML schema: {}",
                schema.name
            )
            .map_err(Self::fmt_error_to_generator_error)?,
            _ => {}
        }
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Ontology declaration
        writeln!(&mut output, "<{}>", schema.id).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    a owl:Ontology ;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", schema.name)
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(version) = &schema.version {
            writeln!(&mut output, "    owl:versionInfo \"{}\" ;", version)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(desc) = &schema.description {
            writeln!(&mut output, "    dcterms:description \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate RDFS schema header
    fn generate_rdfs_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        match self.format {
            RdfFormat::Turtle => writeln!(
                &mut output,
                "# RDFS Schema generated from LinkML: {}",
                schema.name
            )
            .map_err(Self::fmt_error_to_generator_error)?,
            _ => {}
        }

        // Schema declaration
        writeln!(&mut output, "<{}>
    a rdfs:Class ;", schema.id)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", schema.name)
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &schema.description {
            writeln!(&mut output, "    rdfs:comment \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate simple RDF header
    fn generate_simple_header(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        match self.format {
            RdfFormat::Turtle => writeln!(
                &mut output,
                "# RDF triples generated from LinkML: {}
",
                schema.name
            )
            .map_err(Self::fmt_error_to_generator_error)?,
            _ => {}
        }

        Ok(output)
    }

    /// Generate class based on mode
    fn generate_class(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        match self.mode {
            RdfMode::Owl => self.generate_owl_class(name, class, schema),
            RdfMode::Rdfs => self.generate_rdfs_class(name, class, schema),
            RdfMode::Simple => self.generate_simple_class(name, class, schema)}
    }

    /// Generate OWL class from `LinkML` class
    fn generate_owl_class(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let class_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));

        // Class declaration
        writeln!(&mut output, "# Class: {}", name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}", class_uri).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    a owl:Class ;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &class.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Superclass (is_a)
        if let Some(parent) = &class.is_a {
            writeln!(
                &mut output,
                "    rdfs:subClassOf {}:{} ;",
                schema_prefix,
                self.to_pascal_case(parent)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Mixins as additional superclasses
        for mixin in &class.mixins {
            writeln!(
                &mut output,
                "    rdfs:subClassOf {}:{} ;",
                schema_prefix,
                self.to_pascal_case(mixin)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Collect all slots (including inherited)
        let all_slots = self.collect_all_slots(class, schema);

        // Generate property restrictions for slots
        if !all_slots.is_empty() {
            for (i, slot_name) in all_slots.iter().enumerate() {
                if let Some(slot) = schema.slots.get(slot_name) {
                    let restriction =
                        self.generate_property_restriction(slot_name, slot, schema)?;
                    write!(&mut output, "    rdfs:subClassOf {}", restriction)
                        .map_err(Self::fmt_error_to_generator_error)?;
                    if i < all_slots.len() - 1 {
                        writeln!(&mut output, " ,").map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(&mut output, " .").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }
        } else {
            writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate property restriction for a slot
    fn generate_property_restriction(
        &self,
        slot_name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let schema_prefix = self.to_snake_case(&schema.name);
        let property_uri = format!("{}:{}", schema_prefix, self.to_snake_case(slot_name));

        let mut restriction = String::new();
        writeln!(&mut restriction, "[").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut restriction, "        a owl:Restriction ;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut restriction,
            "        owl:onProperty {} ;",
            property_uri
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Cardinality constraints
        if slot.required == Some(true) {
            if slot.multivalued == Some(true) {
                writeln!(&mut restriction, "        owl:minCardinality 1")
                    .map_err(Self::fmt_error_to_generator_error)?;
            } else {
                writeln!(&mut restriction, "        owl:cardinality 1")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        } else if slot.multivalued != Some(true) {
            writeln!(&mut restriction, "        owl:maxCardinality 1")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        write!(&mut restriction, "    ]").map_err(Self::fmt_error_to_generator_error)?;

        Ok(restriction)
    }

    /// Generate OWL property from `LinkML` slot
    fn generate_property(
        &self,
        name: &str,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let property_uri = format!("{}:{}", schema_prefix, self.to_snake_case(name));

        writeln!(&mut output, "# Property: {}", name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}", property_uri).map_err(Self::fmt_error_to_generator_error)?;

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

        writeln!(&mut output, "    a {} ;", property_type)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &slot.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Domain (classes that use this property)
        let using_classes: Vec<String> = schema
            .classes
            .iter()
            .filter(|(_, class)| {
                class.slots.contains(&name.to_string())
                    || self
                        .collect_all_slots(class, schema)
                        .contains(&name.to_string())
            })
            .map(|(class_name, _)| format!("{}:{}", schema_prefix, self.to_pascal_case(class_name)))
            .collect();

        if !using_classes.is_empty() {
            if using_classes.len() == 1 {
                writeln!(&mut output, "    rdfs:domain {} ;", using_classes[0])
                    .map_err(Self::fmt_error_to_generator_error)?;
            } else {
                writeln!(&mut output, "    rdfs:domain [")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        a owl:Class ;")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "        owl:unionOf ({})",
                    using_classes.join(" ")
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "    ] ;").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Range
        if let Some(range) = &slot.range {
            if let Some(datatype) = self.get_xsd_datatype(range) {
                writeln!(&mut output, "    rdfs:range {} ;", datatype)
                    .map_err(Self::fmt_error_to_generator_error)?;
            } else if schema.classes.contains_key(range) {
                writeln!(
                    &mut output,
                    "    rdfs:range {}:{} ;",
                    schema_prefix,
                    self.to_pascal_case(range)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            } else if schema.enums.contains_key(range) {
                writeln!(
                    &mut output,
                    "    rdfs:range {}:{} ;",
                    schema_prefix,
                    self.to_pascal_case(range)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Functional property (not multivalued)
        if slot.multivalued != Some(true) {
            writeln!(&mut output, "    a owl:FunctionalProperty ;")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Pattern as OWL restriction
        if let Some(pattern) = &slot.pattern {
            writeln!(&mut output, "    owl:withRestrictions ([")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "        xsd:pattern \"{}\"", pattern)
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    ]) ;").map_err(Self::fmt_error_to_generator_error)?;
        }

        // Remove trailing semicolon and add period
        if output.ends_with(" ;
") {
            output.truncate(output.len() - 3);
            writeln!(&mut output, " .").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate OWL class for enum
    fn generate_enum(
        &self,
        name: &str,
        enum_def: &EnumDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let enum_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));

        writeln!(&mut output, "# Enumeration: {}", name)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}", enum_uri).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    a owl:Class ;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name)
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &enum_def.description {
            writeln!(&mut output, "    skos:definition \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Create individuals for each permissible value
        let individuals: Vec<String> = enum_def
            .permissible_values
            .iter()
            .map(|pv| {
                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text};
                format!(
                    "{}:{}_{}",
                    schema_prefix,
                    self.to_pascal_case(name),
                    self.to_pascal_case(value)
                )
            })
            .collect();

        writeln!(&mut output, "    owl:equivalentClass [")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        a owl:Class ;")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        owl:oneOf ({})", individuals.join(" "))
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    ] .").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate individuals
        for pv in &enum_def.permissible_values {
            let (value, desc) = match pv {
                PermissibleValue::Simple(s) => (s.clone(), None),
                PermissibleValue::Complex {
                    text, description, ..
                } => (text.clone(), description.clone())};

            let individual_uri = format!(
                "{}:{}_{}",
                schema_prefix,
                self.to_pascal_case(name),
                self.to_pascal_case(&value)
            );

            writeln!(&mut output, "{}", individual_uri)
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    a {} ;", enum_uri)
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "    rdfs:label \"{}\" ;", value)
                .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(desc) = desc {
                writeln!(&mut output, "    skos:definition \"{}\" ;", desc)
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
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
        all_slots.retain(|slot| seen.insert(slot.clone());

        all_slots
    }

    /// Get XSD datatype for `LinkML` range
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
            _ => None}
    }

    /// Convert to snake_case
    fn to_snake_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut prev_upper = false;

        for (i, ch) in s.chars().enumerate() {
            if ch.is_uppercase() && i > 0 && !prev_upper {
                result.push('_');
            }
            result.push(
                ch.to_lowercase()
                    .next()
                    .unwrap_or(ch),
            );
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
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str()}
            })
            .collect()
    }

    /// Generate RDFS class
    fn generate_rdfs_class(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let class_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));

        // Class declaration
        writeln!(&mut output, "# Class: {}", name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}", class_uri).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    a rdfs:Class ;").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    rdfs:label \"{}\" ;", name)
            .map_err(Self::fmt_error_to_generator_error)?;

        // Description
        if let Some(desc) = &class.description {
            writeln!(&mut output, "    rdfs:comment \"{}\" ;", desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Superclass
        if let Some(parent) = &class.is_a {
            writeln!(
                &mut output,
                "    rdfs:subClassOf {}:{} ;",
                schema_prefix,
                self.to_pascal_case(parent)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "    .").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate simple RDF triples
    fn generate_simple_class(
        &self,
        name: &str,
        class: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();
        let schema_prefix = self.to_snake_case(&schema.name);
        let class_uri = format!("{}:{}", schema_prefix, self.to_pascal_case(name));

        // Basic triple
        writeln!(&mut output, "{} a rdfs:Class .", class_uri)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{} rdfs:label \"{}\" .", class_uri, name)
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(desc) = &class.description {
            writeln!(&mut output, "{} rdfs:comment \"{}\" .", class_uri, desc)
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }
}

impl Default for RdfGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for RdfGenerator {
    fn name(&self) -> &str {
        match self.mode {
            RdfMode::Owl => "owl-rdf",
            RdfMode::Rdfs => "rdfs",
            RdfMode::Simple => "rdf"}
    }

    fn description(&self) -> &str {
        match self.mode {
            RdfMode::Owl => "Generates OWL ontology in RDF format from LinkML schemas",
            RdfMode::Rdfs => "Generates RDFS schema from LinkML schemas",
            RdfMode::Simple => "Generates simple RDF triples from LinkML schemas"}
    }

    fn file_extensions(&self) -> Vec<&str> {
        match self.format {
            RdfFormat::Turtle => vec![".ttl", ".owl"],
            RdfFormat::RdfXml => vec![".rdf", ".owl"],
            RdfFormat::NTriples => vec![".nt"],
            RdfFormat::JsonLd => vec![".jsonld"]}
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let mut output = String::new();

        // Generate header
        output.push_str(&self.generate_schema_header(schema)?);

        // Generate prefixes
        output.push_str(&self.generate_prefixes(schema)?);

        // Generate classes
        for (name, class) in &schema.classes {
            let class_def = self.generate_class(name, class, schema).map_err(|e| {
                GeneratorError::Generation(
                    format!("class {}: {}", name, e)
                )
            })?;
            output.push_str(&class_def);
        }

        // Generate properties
        for (name, slot) in &schema.slots {
            let property_def = self.generate_property(name, slot, schema).map_err(|e| {
                GeneratorError::Generation(
                    format!("property {}: {}", name, e)
                )
            })?;
            output.push_str(&property_def);
        }

        // Generate enums
        for (name, enum_def) in &schema.enums {
            let enum_class = self.generate_enum(name, enum_def, schema).map_err(|e| {
                GeneratorError::Generation(
                    format!("enum {}: {}", name, e)
                )
            })?;
            output.push_str(&enum_class);
        }

        // Convert output to desired format
        let final_output = self.convert_to_format(&output, schema)?;

        Ok(final_output)
    }

    fn get_file_extension(&self) -> &str {
        match (self.format, self.mode) {
            (RdfFormat::Turtle, RdfMode::Owl) => "owl",
            (RdfFormat::Turtle, _) => "ttl",
            (RdfFormat::RdfXml, _) => "rdf",
            (RdfFormat::NTriples, _) => "nt",
            (RdfFormat::JsonLd, _) => "jsonld"}
    }

    fn get_default_filename(&self) -> &str {
        "schema"
    }
}

impl RdfGenerator {
    /// Convert Turtle output to desired format
    fn convert_to_format(
        &self,
        turtle_content: &str,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        match self.format {
            RdfFormat::Turtle => Ok(turtle_content.to_string()),
            RdfFormat::RdfXml => self.convert_to_rdfxml(turtle_content, schema),
            RdfFormat::NTriples => self.convert_to_ntriples(turtle_content, schema),
            RdfFormat::JsonLd => self.convert_to_jsonld(turtle_content, schema)}
    }

    /// Convert Turtle to RDF/`XML`
    fn convert_to_rdfxml(
        &self,
        _turtle_content: &str,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        // For now, we'll generate RDF/XML directly from schema
        // In production, you'd use an RDF library for proper conversion
        let mut output = String::new();

        writeln!(&mut output, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "<rdf:RDF{}>
",
            self.generate_rdfxml_prefixes(schema)?
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Generate RDF/XML content based on mode
        // This is a simplified version - full implementation would properly convert triples
        writeln!(&mut output, "</rdf:RDF>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Convert Turtle to N-Triples
    fn convert_to_ntriples(
        &self,
        _turtle_content: &str,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut output = String::new();

        // N-Triples format: subject predicate object .
        // Full URIs, no prefixes
        let schema_uri = &schema.id;

        for (name, _class) in &schema.classes {
            let class_uri = format!("{}#{}", schema_uri, self.to_pascal_case(name));
            writeln!(&mut output, "<{}> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://www.w3.org/2000/01/rdf-schema#Class> .", class_uri).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "<{}> <http://www.w3.org/2000/01/rdf-schema#label> \"{}\" .",
                class_uri, name
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Convert Turtle to `JSON`-LD
    fn convert_to_jsonld(
        &self,
        _turtle_content: &str,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let mut doc = serde_json::json!({
            "@context": serde_json::from_str::<serde_json::Value>(&self.generate_jsonld_context(schema)?).map_err(|e| GeneratorError::Generation(
                format!("json-ld: {e}")
            ))?["@context"].clone(),
            "@graph": []
        });

        // Add classes to graph
        for (name, class) in &schema.classes {
            let class_obj = serde_json::json!({
                "@id": format!("{}:{}", self.to_snake_case(&schema.name), self.to_pascal_case(name)),
                "@type": if self.mode == RdfMode::Owl { "owl:Class" } else { "rdfs:Class" },
                "rdfs:label": name,
                "rdfs:comment": class.description});
            doc["@graph"]
                .as_array_mut()
                .ok_or_else(|| anyhow::anyhow!("@graph should be an array"))?
                .push(class_obj);
        }

        serde_json::to_string_pretty(&doc).map_err(|e| GeneratorError::Generation(
            format!("json-ld: {e}")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};

    #[test]
    fn test_xsd_datatype_mapping() {
        let generator = RdfGenerator::new();

        assert_eq!(
            generator.get_xsd_datatype("string"),
            Some("xsd:string".to_string())
        );
        assert_eq!(
            generator.get_xsd_datatype("integer"),
            Some("xsd:integer".to_string())
        );
        assert_eq!(
            generator.get_xsd_datatype("boolean"),
            Some("xsd:boolean".to_string())
        );
        assert_eq!(
            generator.get_xsd_datatype("datetime"),
            Some("xsd:dateTime".to_string())
        );
        assert_eq!(generator.get_xsd_datatype("CustomType"), None);
    }

    #[test]
    fn test_case_conversion() {
        let generator = RdfGenerator::new();

        assert_eq!(generator.to_snake_case("PersonName"), "person_name");
        assert_eq!(generator.to_pascal_case("person_name"), "PersonName");
    }

    #[test]
    fn test_format_modes() {
        let owl_gen = RdfGenerator::new();
        assert_eq!(owl_gen.name(), "owl-rdf");

        let rdfs_gen = RdfGenerator::rdfs();
        assert_eq!(rdfs_gen.name(), "rdfs");

        let simple_gen = RdfGenerator::simple();
        assert_eq!(simple_gen.name(), "rdf");
    }
}