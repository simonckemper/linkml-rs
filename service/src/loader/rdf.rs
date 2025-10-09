//! RDF data loader and dumper for `LinkML`
//!
//! This module provides functionality to load RDF data (Turtle, N-Triples, RDF/XML)
//! into `LinkML` data instances and dump instances back to RDF format.

use async_trait::async_trait;
use linkml_core::prelude::*;
use oxigraph::io::{RdfFormat, RdfParser, RdfSerializer};
use oxigraph::model::{BlankNode, GraphName, Literal, NamedNode, NamedOrBlankNode, Quad, Term};
use oxigraph::store::Store;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};

/// RDF serialization format
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RdfSerializationFormat {
    /// Turtle format (.ttl)
    Turtle,
    /// N-Triples format (.nt)
    NTriples,
    /// RDF/`XML` format (.rdf, .xml)
    RdfXml,
    /// N-Quads format (.nq)
    NQuads,
    /// `TriG` format (.trig)
    TriG,
}

impl RdfSerializationFormat {
    /// Convert to oxigraph `RdfFormat`
    fn to_oxigraph_format(self) -> RdfFormat {
        match self {
            Self::Turtle => RdfFormat::Turtle,
            Self::NTriples => RdfFormat::NTriples,
            Self::RdfXml => RdfFormat::RdfXml,
            Self::NQuads => RdfFormat::NQuads,
            Self::TriG => RdfFormat::TriG,
        }
    }
}

/// Skolemnization options for blank node handling
#[derive(Debug, Clone)]
pub enum SkolemnizationOptions {
    /// No skolemnization - preserve blank nodes as-is
    None,
    /// Generate deterministic URIs from blank node IDs
    Deterministic {
        /// Base URI for skolem URIs
        base_uri: String,
        /// Prefix for skolem identifiers
        prefix: String,
    },
    /// Generate `UUID`s for blank nodes
    Uuid {
        /// Base URI for skolem URIs
        base_uri: String,
    },
    /// Hash-based skolemnization using triple content
    Hash {
        /// Base URI for skolem URIs
        base_uri: String,
        /// Hash algorithm (sha256, md5, etc.)
        algorithm: String,
    },
}

impl Default for SkolemnizationOptions {
    fn default() -> Self {
        Self::None
    }
}

/// Options specific to RDF loading/dumping
#[derive(Debug, Clone)]
pub struct RdfOptions {
    /// RDF serialization format
    pub format: RdfSerializationFormat,

    /// Base IRI for relative URIs
    pub base_iri: Option<String>,

    /// Default namespace
    pub default_namespace: String,

    /// Namespace prefixes
    pub prefixes: HashMap<String, String>,

    /// Whether to generate blank node identifiers
    pub generate_blank_nodes: bool,

    /// Skolemnization settings for blank nodes
    pub skolemnization: SkolemnizationOptions,

    /// Type predicate (usually rdf:type)
    pub type_predicate: String,

    /// Whether to infer types from RDF types
    pub infer_from_rdf_type: bool,
}

impl Default for RdfOptions {
    fn default() -> Self {
        let mut prefixes = HashMap::new();
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
            format: RdfSerializationFormat::Turtle,
            base_iri: None,
            default_namespace: "http://example.org/".to_string(),
            prefixes,
            generate_blank_nodes: false,
            skolemnization: SkolemnizationOptions::None,
            type_predicate: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string(),
            infer_from_rdf_type: true,
        }
    }
}

/// RDF data loader
pub struct RdfLoader {
    options: RdfOptions,
}

impl RdfLoader {
    /// Create a new RDF loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: RdfOptions::default(),
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: RdfOptions) -> Self {
        Self { options }
    }

    /// Create with specific format
    #[must_use]
    pub fn with_format(format: RdfSerializationFormat) -> Self {
        Self {
            options: RdfOptions {
                format,
                ..Default::default()
            },
        }
    }

    /// Parse RDF data into a store
    fn parse_rdf(&self, data: &[u8]) -> LoaderResult<Store> {
        let store = Store::new().map_err(|e| {
            LoaderError::Io(std::io::Error::other(format!(
                "Failed to create store: {e}"
            )))
        })?;

        let format = self.options.format.to_oxigraph_format();
        let parser = RdfParser::from_format(format);

        let parser = if let Some(base) = &self.options.base_iri {
            parser
                .with_base_iri(base)
                .map_err(|e| LoaderError::Configuration(format!("Invalid base IRI: {e}")))?
        } else {
            parser
        };

        let cursor = Cursor::new(data);
        let quads: Vec<_> = parser
            .for_reader(cursor)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| LoaderError::Parse(format!("Failed to parse RDF: {e}")))?;

        for quad in quads {
            store.insert(&quad).map_err(|e| {
                LoaderError::Io(std::io::Error::other(format!("Failed to insert quad: {e}")))
            })?;
        }

        Ok(store)
    }

    /// Extract instances from RDF store
    fn extract_instances(
        &self,
        store: &Store,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let mut instances = Vec::new();
        let mut instance_map: HashMap<String, DataInstance> = HashMap::new();

        // Find all subjects that have a type
        let type_predicate = NamedNode::new(&self.options.type_predicate)
            .map_err(|e| LoaderError::Configuration(format!("Invalid type predicate: {e}")))?;

        // Get all typed subjects
        let typed_subjects: Vec<NamedOrBlankNode> = store
            .quads_for_pattern(None, Some((&type_predicate).into()), None, None)
            .filter_map(std::result::Result::ok)
            .map(|quad| quad.subject)
            .collect();

        // Process each subject
        for subject in typed_subjects {
            let subject_str = self.subject_to_string(&subject);

            // Skip if already processed
            if instance_map.contains_key(&subject_str) {
                continue;
            }

            // Get the type(s) of this subject
            let types: Vec<String> = store
                .quads_for_pattern(
                    Some((&subject).into()),
                    Some((&type_predicate).into()),
                    None,
                    None,
                )
                .filter_map(std::result::Result::ok)
                .filter_map(|quad| match &quad.object {
                    Term::NamedNode(n) => Some(n.as_str().to_string()),
                    _ => None,
                })
                .collect();

            // Determine the LinkML class
            let class_name = if let Some(target) = &options.target_class {
                target.clone()
            } else if self.options.infer_from_rdf_type {
                self.infer_class_from_types(&types, schema)?
            } else {
                continue;
            };

            // Create instance
            let mut data = HashMap::new();

            // Get all properties for this subject
            for quad_result in store.quads_for_pattern(Some((&subject).into()), None, None, None) {
                let quad = quad_result
                    .map_err(|e| LoaderError::Parse(format!("Failed to read quad: {e}")))?;

                // Skip type predicates
                if quad.predicate == type_predicate {
                    continue;
                }

                let property = self.predicate_to_property(&quad.predicate);
                let value = Self::term_to_json(&quad.object)?;

                // Handle multivalued properties
                if let Some(existing) = data.get_mut(&property) {
                    match existing {
                        JsonValue::Array(arr) => arr.push(value),
                        other => {
                            let old_value = other.clone();
                            *other = JsonValue::Array(vec![old_value, value]);
                        }
                    }
                } else {
                    data.insert(property, value);
                }
            }

            let instance = DataInstance {
                class_name,
                data,
                id: Some(subject_str.clone()),
                metadata: HashMap::new(),
            };

            instance_map.insert(subject_str, instance);
        }

        // Also handle subjects without explicit types if requested
        if options.infer_types {
            for quad_result in store {
                let quad = quad_result
                    .map_err(|e| LoaderError::Parse(format!("Failed to read quad: {e}")))?;

                let subject_str = match &quad.subject {
                    NamedOrBlankNode::NamedNode(node) => {
                        self.subject_to_string(&NamedOrBlankNode::NamedNode(node.clone()))
                    }
                    NamedOrBlankNode::BlankNode(node) => {
                        self.subject_to_string(&NamedOrBlankNode::BlankNode(node.clone()))
                    }
                };

                // Skip if already processed
                if instance_map.contains_key(&subject_str) {
                    continue;
                }

                // Try to infer class from properties
                let subject_node = match &quad.subject {
                    NamedOrBlankNode::NamedNode(node) => NamedOrBlankNode::NamedNode(node.clone()),
                    NamedOrBlankNode::BlankNode(node) => NamedOrBlankNode::BlankNode(node.clone()),
                };
                if let Some(class_name) =
                    self.infer_class_from_properties(&subject_node, store, schema)
                {
                    let mut data = HashMap::new();

                    // Get all properties
                    for prop_quad_result in
                        store.quads_for_pattern(Some((&quad.subject).into()), None, None, None)
                    {
                        let prop_quad = prop_quad_result
                            .map_err(|e| LoaderError::Parse(format!("Failed to read quad: {e}")))?;

                        let property = self.predicate_to_property(&prop_quad.predicate);
                        let value = Self::term_to_json(&prop_quad.object)?;

                        data.insert(property, value);
                    }

                    let instance = DataInstance {
                        class_name,
                        data,
                        id: Some(subject_str.clone()),
                        metadata: HashMap::new(),
                    };

                    instance_map.insert(subject_str, instance);
                }
            }
        }

        // Apply limit if specified
        instances.extend(instance_map.into_values());

        if let Some(limit) = options.limit {
            instances.truncate(limit);
        }

        Ok(instances)
    }

    /// Convert subject to string, applying skolemnization if configured
    fn subject_to_string(&self, subject: &NamedOrBlankNode) -> String {
        match subject {
            NamedOrBlankNode::NamedNode(n) => n.as_str().to_string(),
            NamedOrBlankNode::BlankNode(b) => self.skolemnize_blank_node(b),
        }
    }

    /// Skolemnize a blank node according to configuration
    fn skolemnize_blank_node(&self, blank_node: &BlankNode) -> String {
        match &self.options.skolemnization {
            SkolemnizationOptions::None => {
                // Keep as blank node identifier
                format!("_:{}", blank_node.as_str())
            }
            SkolemnizationOptions::Deterministic { base_uri, prefix } => {
                // Generate deterministic URI from blank node ID
                format!("{}/{}_{}", base_uri, prefix, blank_node.as_str())
            }
            SkolemnizationOptions::Uuid { base_uri } => {
                // Generate UUID-based URI
                let uuid = uuid::Uuid::new_v4();
                format!("{base_uri}/skolem/{uuid}")
            }
            SkolemnizationOptions::Hash {
                base_uri,
                algorithm,
            } => {
                // Generate hash-based URI from blank node content
                // Currently only sha256 is supported, but the algorithm parameter
                // is preserved for future extensibility
                use sha2::{Digest, Sha256};
                let hash = if algorithm.as_str() == "sha256" {
                    let mut hasher = Sha256::new();
                    hasher.update(blank_node.as_str().as_bytes());
                    format!("{:x}", hasher.finalize())
                } else {
                    // Default to sha256 for unknown algorithms
                    let mut hasher = Sha256::new();
                    hasher.update(blank_node.as_str().as_bytes());
                    format!("{:x}", hasher.finalize())
                };
                format!("{base_uri}/skolem/{hash}")
            }
        }
    }

    /// Convert term to string (reserved for future RDF-star support)
    fn _term_to_string(term: &Term) -> String {
        match term {
            Term::NamedNode(n) => n.as_str().to_string(),
            Term::BlankNode(b) => format!("_:{}", b.as_str()),
            Term::Literal(l) => l.value().to_string(),
            // TODO: Handle RDF-star Triple terms when oxigraph updates API
            // Term::Triple(triple) => {
            //     // RDF-star support: convert triple to reified statement representation
            //     let subj = self._term_to_string(&triple.subject.clone().into());
            //     let pred = triple.predicate.as_str();
            //     let obj = self._term_to_string(&triple.object.clone());
            //     format!("<<{subj} {pred} {obj}>>")
            // }
        }
    }

    /// Convert predicate to property name
    fn predicate_to_property(&self, predicate: &NamedNode) -> String {
        let uri = predicate.as_str();

        // Try to use prefixed name
        for (prefix, namespace) in &self.options.prefixes {
            if uri.starts_with(namespace) {
                let local = &uri[namespace.len()..];
                return format!("{prefix}:{local}");
            }
        }

        // Otherwise use local name
        if let Some(pos) = uri.rfind(['#', '/']) {
            uri[pos + 1..].to_string()
        } else {
            uri.to_string()
        }
    }

    /// Convert RDF term to `JSON` value
    fn term_to_json(term: &Term) -> LoaderResult<JsonValue> {
        match term {
            Term::NamedNode(n) => Ok(JsonValue::String(n.as_str().to_string())),
            Term::BlankNode(b) => Ok(JsonValue::String(format!("_:{}", b.as_str()))),
            Term::Literal(l) => {
                let value = l.value();

                // Check datatype
                match l.datatype().as_str() {
                    "http://www.w3.org/2001/XMLSchema#integer" => value
                        .parse::<i64>()
                        .map(|n| JsonValue::Number(n.into()))
                        .map_err(|_| {
                            LoaderError::TypeConversion(format!(
                                "Cannot parse '{value}' as integer"
                            ))
                        }),
                    "http://www.w3.org/2001/XMLSchema#decimal"
                    | "http://www.w3.org/2001/XMLSchema#double"
                    | "http://www.w3.org/2001/XMLSchema#float" => value
                        .parse::<f64>()
                        .map(|n| {
                            JsonValue::Number(
                                serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()),
                            )
                        })
                        .map_err(|_| {
                            LoaderError::TypeConversion(format!("Cannot parse '{value}' as float"))
                        }),
                    "http://www.w3.org/2001/XMLSchema#boolean" => match value {
                        "true" | "1" => Ok(JsonValue::Bool(true)),
                        "false" | "0" => Ok(JsonValue::Bool(false)),
                        _ => Err(LoaderError::TypeConversion(format!(
                            "Cannot parse '{value}' as boolean"
                        ))),
                    },
                    _ => Ok(JsonValue::String(value.to_string())),
                }
            } // TODO: Handle RDF-star Triple terms when oxigraph updates API
              // Term::Triple(triple) => {
              //     // RDF-star support: convert triple to a nested JSON object representation
              //     let subj_term: Term = triple.subject.clone().into();
              //     let obj_term = triple.object.clone();
              //     let subj_value = self.term_to_json(&subj_term)?;
              //     let pred_name = self.predicate_to_property(&triple.predicate);
              //     let obj_value = self.term_to_json(&obj_term)?;
              //
              //     // Represent as a reified statement object
              //     let mut triple_obj = serde_json::Map::new();
              //     triple_obj.insert(
              //         "@type".to_string(),
              //         JsonValue::String("rdf:Statement".to_string()),
              //     );
              //     triple_obj.insert("rdf:subject".to_string(), subj_value);
              //     triple_obj.insert("rdf:predicate".to_string(), JsonValue::String(pred_name));
              //     triple_obj.insert("rdf:object".to_string(), obj_value);
              //
              //     Ok(JsonValue::Object(triple_obj))
              // }
        }
    }

    /// Infer `LinkML` class from RDF types
    fn infer_class_from_types(
        &self,
        types: &[String],
        schema: &SchemaDefinition,
    ) -> LoaderResult<String> {
        // Try to find a matching class
        for rdf_type in types {
            // Extract local name
            let local_name = if let Some(pos) = rdf_type.rfind(['#', '/']) {
                &rdf_type[pos + 1..]
            } else {
                rdf_type
            };

            // Check if this matches a LinkML class
            if schema.classes.contains_key(local_name) {
                return Ok(local_name.to_string());
            }
        }

        Err(LoaderError::SchemaValidation(format!(
            "Could not find LinkML class for RDF types: {types:?}"
        )))
    }

    /// Infer class from properties
    fn infer_class_from_properties(
        &self,
        subject: &NamedOrBlankNode,
        store: &Store,
        schema: &SchemaDefinition,
    ) -> Option<String> {
        // Get all properties
        let property_list: Vec<String> = store
            .quads_for_pattern(Some(subject.into()), None, None, None)
            .filter_map(std::result::Result::ok)
            .map(|quad| self.predicate_to_property(&quad.predicate))
            .collect();

        // Find best matching class
        let mut best_match = None;
        let mut best_score = 0;

        for (class_name, class_def) in &schema.classes {
            let mut match_score = 0;
            let all_slots = self.collect_all_slots(class_name, class_def, schema);

            for prop in &property_list {
                if all_slots.contains(prop) {
                    match_score += 1;
                }
            }

            if match_score > best_score {
                best_score = match_score;
                best_match = Some(class_name.clone());
            }
        }

        best_match
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut all_slots = Vec::new();

        // Add inherited slots
        if let Some(parent_name) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
            all_slots.extend(parent_slots);
        }

        // Add direct slots
        all_slots.extend(class_def.slots.clone());

        // Add attributes
        all_slots.extend(class_def.attributes.keys().cloned());

        all_slots
    }

    /// Check for circular inheritance that could cause issues in RDF processing
    fn check_inheritance_cycles(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        use std::collections::HashSet;

        for (class_name, _class_def) in &schema.classes {
            let mut visited = HashSet::new();
            let mut stack = vec![class_name.clone()];

            while let Some(current_class) = stack.pop() {
                if visited.contains(&current_class) {
                    continue;
                }
                visited.insert(current_class.clone());

                if let Some(current_def) = schema.classes.get(&current_class)
                    && let Some(parent) = &current_def.is_a
                {
                    if parent == class_name {
                        return Err(LoaderError::SchemaValidation(format!(
                            "Circular inheritance detected: class '{class_name}' inherits from itself"
                        )));
                    }
                    if !visited.contains(parent) {
                        stack.push(parent.clone());
                    }
                }
            }
        }

        Ok(())
    }
}

/// Check if a type name represents a valid RDF datatype
fn is_valid_rdf_datatype(type_name: &str) -> bool {
    matches!(
        type_name,
        "string"
            | "boolean"
            | "integer"
            | "float"
            | "double"
            | "decimal"
            | "date"
            | "datetime"
            | "time"
            | "uri"
            | "uriorcurie"
            | "ncname"
            | "nodeidentifier"
            | "jsonpointer"
            | "jsonpath"
            | "sparqlpath"
            | "curie"
            // XSD datatypes
            | "xsd:string"
            | "xsd:boolean"
            | "xsd:integer"
            | "xsd:int"
            | "xsd:long"
            | "xsd:short"
            | "xsd:byte"
            | "xsd:unsignedInt"
            | "xsd:unsignedLong"
            | "xsd:unsignedShort"
            | "xsd:unsignedByte"
            | "xsd:positiveInteger"
            | "xsd:nonNegativeInteger"
            | "xsd:negativeInteger"
            | "xsd:nonPositiveInteger"
            | "xsd:float"
            | "xsd:double"
            | "xsd:decimal"
            | "xsd:date"
            | "xsd:dateTime"
            | "xsd:time"
            | "xsd:gYear"
            | "xsd:gYearMonth"
            | "xsd:gMonth"
            | "xsd:gMonthDay"
            | "xsd:gDay"
            | "xsd:duration"
            | "xsd:dayTimeDuration"
            | "xsd:yearMonthDuration"
            | "xsd:anyURI"
            | "xsd:base64Binary"
            | "xsd:hexBinary"
            | "xsd:normalizedString"
            | "xsd:token"
            | "xsd:language"
            | "xsd:NMTOKEN"
            | "xsd:NMTOKENS"
            | "xsd:Name"
            | "xsd:NCName"
            | "xsd:ID"
            | "xsd:IDREF"
            | "xsd:IDREFS"
            | "xsd:ENTITY"
            | "xsd:ENTITIES"
            | "xsd:QName"
            | "xsd:NOTATION"
    )
}

impl Default for RdfLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for RdfLoader {
    fn name(&self) -> &str {
        match self.options.format {
            RdfSerializationFormat::Turtle => "turtle",
            RdfSerializationFormat::NTriples => "ntriples",
            RdfSerializationFormat::RdfXml => "rdfxml",
            RdfSerializationFormat::NQuads => "nquads",
            RdfSerializationFormat::TriG => "trig",
        }
    }

    fn description(&self) -> &'static str {
        "Loads data from RDF files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        match self.options.format {
            RdfSerializationFormat::Turtle => vec![".ttl", ".turtle"],
            RdfSerializationFormat::NTriples => vec![".nt", ".ntriples"],
            RdfSerializationFormat::RdfXml => vec![".rdf", ".xml"],
            RdfSerializationFormat::NQuads => vec![".nq", ".nquads"],
            RdfSerializationFormat::TriG => vec![".trig"],
        }
    }

    async fn load_file(
        &self,
        path: &Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let data = tokio::fs::read(path).await?;
        self.load_bytes(&data, schema, options).await
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        self.load_bytes(content.as_bytes(), schema, options).await
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let store = self.parse_rdf(data)?;
        self.extract_instances(&store, schema, options)
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that schema is compatible with RDF loading

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema name is required for RDF loading".to_string(),
            ));
        }

        // Validate that classes can be represented as RDF resources
        for (class_name, class_def) in &schema.classes {
            // Check for RDF-incompatible characters in class names
            if class_name.contains(|c: char| {
                c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
            }) {
                return Err(LoaderError::SchemaValidation(format!(
                    "Class name '{class_name}' contains RDF-incompatible characters"
                )));
            }

            // Validate slots for RDF property compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                // Check for RDF-incompatible characters in slot names
                if slot_name.contains(|c: char| {
                    c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
                }) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot name '{slot_name}' in class '{class_name}' contains RDF-incompatible characters"
                    )));
                }
            }

            // Validate attributes for RDF compatibility
            for (attr_name, _attr_def) in &class_def.attributes {
                if attr_name.contains(|c: char| {
                    c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
                }) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Attribute name '{attr_name}' in class '{class_name}' contains RDF-incompatible characters"
                    )));
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a
                && !schema.classes.contains_key(parent_name)
            {
                return Err(LoaderError::SchemaValidation(format!(
                    "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                )));
            }
        }

        // Validate slot definitions for RDF compatibility
        for (slot_name, slot_def) in &schema.slots {
            // Check if range is valid
            if let Some(range) = &slot_def.range {
                // Verify range refers to valid class or datatype
                if !schema.classes.contains_key(range)
                    && !schema.enums.contains_key(range)
                    && !is_valid_rdf_datatype(range)
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or RDF datatype"
                    )));
                }
            }

            // Validate domain constraints
            if let Some(domain) = &slot_def.domain {
                if !schema.classes.contains_key(domain) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                    )));
                }
            }
        }

        // Validate enums for RDF compatibility
        for (enum_name, enum_def) in &schema.enums {
            if enum_name.contains(|c: char| {
                c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
            }) {
                return Err(LoaderError::SchemaValidation(format!(
                    "Enum name '{enum_name}' contains RDF-incompatible characters"
                )));
            }

            // Check enum values for RDF compatibility
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }

                // Check for RDF-unsafe characters in enum values
                if pv_text.contains(['<', '>', '"']) {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Enum value '{pv_text}' in enum '{enum_name}' contains RDF-unsafe characters"
                    )));
                }
            }
        }

        // Check for circular inheritance that could cause issues in RDF
        self.check_inheritance_cycles(schema)?;

        Ok(())
    }
}

/// RDF data dumper
pub struct RdfDumper {
    options: RdfOptions,
}

impl RdfDumper {
    /// Create a new RDF dumper
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: RdfOptions::default(),
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: RdfOptions) -> Self {
        Self { options }
    }

    /// Create with specific format
    #[must_use]
    pub fn with_format(format: RdfSerializationFormat) -> Self {
        Self {
            options: RdfOptions {
                format,
                ..Default::default()
            },
        }
    }

    /// Create RDF store from instances
    fn create_store(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<Store> {
        let store = Store::new().map_err(|e| {
            DumperError::Io(std::io::Error::other(format!(
                "Failed to create store: {e}"
            )))
        })?;

        let type_predicate = NamedNode::new(&self.options.type_predicate)
            .map_err(|e| DumperError::Configuration(format!("Invalid type predicate: {e}")))?;

        for instance in instances {
            // Create subject
            let subject = if let Some(id) = &instance.id {
                if let Some(stripped) = id.strip_prefix("_:") {
                    // Blank node
                    NamedOrBlankNode::BlankNode(BlankNode::new(stripped).map_err(|e| {
                        DumperError::Serialization(format!("Invalid blank node ID: {e}"))
                    })?)
                } else if id.starts_with("http://") || id.starts_with("https://") {
                    // Already a full URI
                    NamedOrBlankNode::NamedNode(
                        NamedNode::new(id)
                            .map_err(|e| DumperError::Serialization(format!("Invalid URI: {e}")))?,
                    )
                } else {
                    // Create URI with default namespace
                    let uri = format!("{}{}", self.options.default_namespace, id);
                    NamedOrBlankNode::NamedNode(
                        NamedNode::new(&uri)
                            .map_err(|e| DumperError::Serialization(format!("Invalid URI: {e}")))?,
                    )
                }
            } else if self.options.generate_blank_nodes {
                NamedOrBlankNode::BlankNode(BlankNode::default())
            } else {
                return Err(DumperError::Serialization(
                    "Instance has no ID and blank node generation is disabled".to_string(),
                ));
            };

            // Add type triple
            let class_uri = format!("{}{}", self.options.default_namespace, instance.class_name);
            let class_node = NamedNode::new(&class_uri)
                .map_err(|e| DumperError::Serialization(format!("Invalid class URI: {e}")))?;

            let type_quad = Quad {
                subject: subject.clone(),
                predicate: type_predicate.clone(),
                object: Term::NamedNode(class_node),
                graph_name: GraphName::DefaultGraph,
            };

            store.insert(&type_quad).map_err(|e| {
                DumperError::Io(std::io::Error::other(format!(
                    "Failed to insert type quad: {e}"
                )))
            })?;

            // Add property triples
            for (property, value) in &instance.data {
                if value.is_null() {
                    continue;
                }

                let predicate = self.property_to_predicate(property, schema)?;

                if let JsonValue::Array(arr) = value {
                    for item in arr {
                        let object = self.json_to_term(item, property, schema)?;
                        let quad = Quad {
                            subject: subject.clone(),
                            predicate: predicate.clone(),
                            object,
                            graph_name: GraphName::DefaultGraph,
                        };
                        store.insert(&quad).map_err(|e| {
                            DumperError::Io(std::io::Error::other(format!(
                                "Failed to insert quad: {e}"
                            )))
                        })?;
                    }
                } else {
                    let object = self.json_to_term(value, property, schema)?;
                    let quad = Quad {
                        subject: subject.clone(),
                        predicate: predicate.clone(),
                        object,
                        graph_name: GraphName::DefaultGraph,
                    };
                    store.insert(&quad).map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert quad: {e}"
                        )))
                    })?;
                }
            }
        }

        Ok(store)
    }

    /// Convert property name to predicate
    fn property_to_predicate(
        &self,
        property: &str,
        _schema: &SchemaDefinition,
    ) -> DumperResult<NamedNode> {
        // Handle prefixed names
        if let Some(colon_pos) = property.find(':') {
            let prefix = &property[..colon_pos];
            let local = &property[colon_pos + 1..];

            if let Some(namespace) = self.options.prefixes.get(prefix) {
                let uri = format!("{namespace}{local}");
                return NamedNode::new(&uri).map_err(|e| {
                    DumperError::Serialization(format!("Invalid predicate URI: {e}"))
                });
            }
        }

        // Otherwise use default namespace
        let uri = format!("{}{}", self.options.default_namespace, property);
        NamedNode::new(&uri)
            .map_err(|e| DumperError::Serialization(format!("Invalid predicate URI: {e}")))
    }

    /// Convert `JSON` value to RDF term
    fn json_to_term(
        &self,
        value: &JsonValue,
        property: &str,
        schema: &SchemaDefinition,
    ) -> DumperResult<Term> {
        match value {
            JsonValue::Null => Err(DumperError::TypeConversion(
                "Cannot convert null to RDF".to_string(),
            )),

            JsonValue::Bool(b) => {
                let literal = Literal::new_typed_literal(
                    b.to_string(),
                    NamedNode::new("http://www.w3.org/2001/XMLSchema#boolean")
                        .expect("hardcoded XSD boolean datatype URI is valid: {}"),
                );
                Ok(Term::Literal(literal))
            }

            JsonValue::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    let literal = Literal::new_typed_literal(
                        n.to_string(),
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#integer")
                            .expect("hardcoded XSD integer datatype URI is valid: {}"),
                    );
                    Ok(Term::Literal(literal))
                } else {
                    let literal = Literal::new_typed_literal(
                        n.to_string(),
                        NamedNode::new("http://www.w3.org/2001/XMLSchema#decimal")
                            .expect("hardcoded XSD decimal datatype URI is valid: {}"),
                    );
                    Ok(Term::Literal(literal))
                }
            }

            JsonValue::String(s) => {
                // Check if it's a URI reference
                if s.starts_with("http://") || s.starts_with("https://") {
                    // Check if this property expects a URI
                    if let Some(slot) = schema.slots.get(property)
                        && (slot.range.as_deref() == Some("uri")
                            || slot.range.as_deref() == Some("uriorcurie"))
                    {
                        let node = NamedNode::new(s)
                            .map_err(|e| DumperError::Serialization(format!("Invalid URI: {e}")))?;
                        return Ok(Term::NamedNode(node));
                    }
                }

                // Check if it's a blank node reference
                if let Some(stripped) = s.strip_prefix("_:") {
                    let blank = BlankNode::new(stripped).map_err(|e| {
                        DumperError::Serialization(format!("Invalid blank node: {e}"))
                    })?;
                    return Ok(Term::BlankNode(blank));
                }

                // Otherwise create a string literal
                Ok(Term::Literal(Literal::new_simple_literal(s)))
            }

            JsonValue::Array(_) => Err(DumperError::TypeConversion(
                "Arrays should be handled at a higher level".to_string(),
            )),

            JsonValue::Object(_) => Err(DumperError::TypeConversion(
                "Cannot convert complex objects to RDF terms".to_string(),
            )),
        }
    }

    /// Serialize store to bytes
    fn serialize_store(&self, store: &Store) -> DumperResult<Vec<u8>> {
        let format = self.options.format.to_oxigraph_format();
        let mut buffer = Vec::new();

        let serializer = RdfSerializer::from_format(format);

        // Serialize all quads
        let mut writer = serializer.for_writer(&mut buffer);
        for quad_result in store {
            let quad = quad_result.map_err(|e| {
                DumperError::Io(std::io::Error::other(format!(
                    "Failed to read quad from store: {e}"
                )))
            })?;
            writer.serialize_quad(&quad).map_err(|e| {
                DumperError::Io(std::io::Error::other(format!(
                    "Failed to serialize quad: {e}"
                )))
            })?;
        }
        writer.finish().map_err(|e| {
            DumperError::Io(std::io::Error::other(format!(
                "Failed to finish RDF serialization: {e}"
            )))
        })?;

        Ok(buffer)
    }
}

impl Default for RdfDumper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataDumper for RdfDumper {
    fn name(&self) -> &str {
        match self.options.format {
            RdfSerializationFormat::Turtle => "turtle",
            RdfSerializationFormat::NTriples => "ntriples",
            RdfSerializationFormat::RdfXml => "rdfxml",
            RdfSerializationFormat::NQuads => "nquads",
            RdfSerializationFormat::TriG => "trig",
        }
    }

    fn description(&self) -> &'static str {
        "Dumps data to RDF format"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        match self.options.format {
            RdfSerializationFormat::Turtle => vec![".ttl", ".turtle"],
            RdfSerializationFormat::NTriples => vec![".nt", ".ntriples"],
            RdfSerializationFormat::RdfXml => vec![".rdf", ".xml"],
            RdfSerializationFormat::NQuads => vec![".nq", ".nquads"],
            RdfSerializationFormat::TriG => vec![".trig"],
        }
    }

    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()> {
        let data = self.dump_bytes(instances, schema, options).await?;
        tokio::fs::write(path, data).await?;
        Ok(())
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String> {
        let data = self.dump_bytes(instances, schema, options).await?;
        String::from_utf8(data)
            .map_err(|e| DumperError::Serialization(format!("Invalid UTF-8: {e}")))
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        // Apply limit if specified
        let instances_to_dump: Vec<&DataInstance> = if let Some(limit) = options.limit {
            instances.iter().take(limit).collect()
        } else {
            instances.iter().collect()
        };

        let instances_slice: Vec<DataInstance> = instances_to_dump.into_iter().cloned().collect();
        let store = self.create_store(&instances_slice, schema)?;
        self.serialize_store(&store)
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate that schema is compatible with RDF dumping

        // Check if schema has basic required elements
        if schema.name.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Schema name is required for RDF dumping".to_string(),
            ));
        }

        // Validate that classes can be represented as RDF resources
        for (class_name, class_def) in &schema.classes {
            // Check for RDF-incompatible characters in class names
            if class_name.contains(|c: char| {
                c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
            }) {
                return Err(DumperError::SchemaValidation(format!(
                    "Class name '{class_name}' contains RDF-incompatible characters"
                )));
            }

            // Validate slots for RDF property compatibility
            for slot_name in &class_def.slots {
                // Check if this slot exists in schema slots
                if !schema.slots.contains_key(slot_name) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' referenced in class '{class_name}' not found in schema slots"
                    )));
                }

                // Check for RDF-incompatible characters in slot names
                if slot_name.contains(|c: char| {
                    c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
                }) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot name '{slot_name}' in class '{class_name}' contains RDF-incompatible characters"
                    )));
                }
            }

            // Validate attributes for RDF compatibility
            for (attr_name, _attr_def) in &class_def.attributes {
                if attr_name.contains(|c: char| {
                    c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
                }) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Attribute name '{attr_name}' in class '{class_name}' contains RDF-incompatible characters"
                    )));
                }
            }

            // Check inheritance chain for validity
            if let Some(parent_name) = &class_def.is_a {
                if !schema.classes.contains_key(parent_name) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Parent class '{parent_name}' for class '{class_name}' not found in schema"
                    )));
                }
            }
        }

        // Validate slot definitions for RDF compatibility
        for (slot_name, slot_def) in &schema.slots {
            // Check if range is valid
            if let Some(range) = &slot_def.range {
                // Verify range refers to valid class or datatype
                if !schema.classes.contains_key(range)
                    && !schema.enums.contains_key(range)
                    && !is_valid_rdf_datatype(range)
                {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid range '{range}' - not a valid class, enum, or RDF datatype"
                    )));
                }
            }

            // Validate domain constraints
            if let Some(domain) = &slot_def.domain {
                if !schema.classes.contains_key(domain) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot '{slot_name}' has invalid domain '{domain}' - class not found in schema"
                    )));
                }
            }
        }

        // Validate enums for RDF compatibility
        for (enum_name, enum_def) in &schema.enums {
            if enum_name.contains(|c: char| {
                c.is_whitespace() || ['<', '>', '"', '{', '}', '|', '^', '`', '\\'].contains(&c)
            }) {
                return Err(DumperError::SchemaValidation(format!(
                    "Enum name '{enum_name}' contains RDF-incompatible characters"
                )));
            }

            // Check enum values for RDF compatibility
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };

                if pv_text.is_empty() {
                    return Err(DumperError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }

                // Check for RDF-unsafe characters in enum values
                if pv_text.contains(['<', '>', '"']) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Enum value '{pv_text}' in enum '{enum_name}' contains RDF-unsafe characters"
                    )));
                }
            }
        }

        // Validate that default namespace and prefixes are valid URIs
        if !self.options.default_namespace.starts_with("http://")
            && !self.options.default_namespace.starts_with("https://")
            && !self.options.default_namespace.starts_with("urn:")
        {
            return Err(DumperError::SchemaValidation(format!(
                "Default namespace '{}' is not a valid URI",
                self.options.default_namespace
            )));
        }

        // Validate prefix mappings
        for (prefix, namespace_uri) in &self.options.prefixes {
            if prefix.is_empty() {
                return Err(DumperError::SchemaValidation(
                    "Empty prefix in namespace mapping".to_string(),
                ));
            }

            if !namespace_uri.starts_with("http://")
                && !namespace_uri.starts_with("https://")
                && !namespace_uri.starts_with("urn:")
            {
                return Err(DumperError::SchemaValidation(format!(
                    "Namespace URI '{namespace_uri}' for prefix '{prefix}' is not a valid URI"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
    use serde_json::json;

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Person class
        let mut person_class = ClassDefinition::default();
        person_class.slots = vec!["id".to_string(), "name".to_string(), "knows".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        // Define slots
        let mut id_slot = SlotDefinition::default();
        id_slot.identifier = Some(true);
        id_slot.range = Some("string".to_string());
        schema.slots.insert("id".to_string(), id_slot);

        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        schema.slots.insert("name".to_string(), name_slot);

        let mut knows_slot = SlotDefinition::default();
        knows_slot.range = Some("Person".to_string());
        knows_slot.multivalued = Some(true);
        schema.slots.insert("knows".to_string(), knows_slot);

        schema
    }

    #[tokio::test]
    async fn test_turtle_load_and_dump() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let loader = RdfLoader::new();
        let dumper = RdfDumper::new();

        let turtle_content = r#"
@prefix ex: <http://example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:alice rdf:type ex:Person ;
    ex:name "Alice" ;
    ex:knows ex:bob .

ex:bob rdf:type ex:Person ;
    ex:name "Bob" .
"#;

        // Load Turtle
        let options = LoadOptions {
            infer_types: true,
            ..Default::default()
        };

        let instances = loader
            .load_string(turtle_content, &schema, &options)
            .await
            .expect("should load valid Turtle content: {}");
        assert_eq!(instances.len(), 2);

        // Find Alice
        let alice = instances
            .iter()
            .find(|i| i.id.as_deref() == Some("http://example.org/alice"))
            .ok_or_else(|| anyhow::anyhow!("should find alice instance"))?;
        assert_eq!(alice.class_name, "Person");
        assert_eq!(alice.data.get("name"), Some(&json!("Alice")));
        assert_eq!(
            alice.data.get("knows"),
            Some(&json!("http://example.org/bob"))
        );

        // Dump back to Turtle
        let dump_options = DumpOptions::default();
        let output = dumper
            .dump_string(&instances, &schema, &dump_options)
            .await
            .expect("should dump instances to Turtle: {}");

        // Should contain the same data
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
        assert!(output.contains("knows"));
        Ok(())
    }

    #[tokio::test]
    async fn test_ntriples_format() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let loader = RdfLoader::with_format(RdfSerializationFormat::NTriples);
        let dumper = RdfDumper::with_format(RdfSerializationFormat::NTriples);

        let ntriples_content = r#"<http://example.org/charlie> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
<http://example.org/charlie> <http://example.org/name> "Charlie" .
"#;

        let options = LoadOptions::default();
        let instances = loader
            .load_string(ntriples_content, &schema, &options)
            .await
            .expect("should load valid N-Triples content: {}");
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].data.get("name"), Some(&json!("Charlie")));

        // Dump to N-Triples
        let dump_options = DumpOptions::default();
        let output = dumper
            .dump_string(&instances, &schema, &dump_options)
            .await
            .expect("should dump instances to N-Triples: {}");

        // N-Triples should have one triple per line
        let lines: Vec<&str> = output.trim().lines().collect();
        assert!(lines.len() >= 2); // At least type and name triples
        Ok(())
    }
}
