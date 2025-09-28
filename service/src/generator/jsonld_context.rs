//! JSON-LD Context generator for `LinkML` schemas
//!
//! This module generates JSON-LD @context definitions from `LinkML` schemas,
//! enabling semantic web integration and linked data capabilities.

use crate::generator::traits::{Generator, GeneratorConfig};
use bitflags::bitflags;
use linkml_core::error::LinkMLError;
use linkml_core::types::{ClassDefinition, PrefixDefinition, SchemaDefinition, SlotDefinition};
use serde::Serialize;
use serde_json::{Map, Value, json};

bitflags! {
    /// Feature flags for JSON-LD context generation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct JsonLdFeatures: u8 {
        /// Include type coercion in context
        const TYPE_COERCION = 1 << 0;
        /// Include language maps in context
        const LANGUAGE_MAPS = 1 << 1;
        /// Use compact IRIs (CURIEs)
        const USE_CURIES = 1 << 2;
        /// Include container mappings
        const CONTAINERS = 1 << 3;

        /// Default feature set for typical usage
        const DEFAULT = Self::TYPE_COERCION.bits() | Self::USE_CURIES.bits() | Self::CONTAINERS.bits();
    }
}

/// `JSON`-LD Context generator configuration
#[derive(Debug, Clone)]
pub struct JsonLdContextGeneratorConfig {
    /// Base generator configuration
    pub base: GeneratorConfig,
    /// Base URI for the schema
    pub base_uri: Option<String>,
    /// Feature flags controlling context generation
    pub features: JsonLdFeatures,
    /// Default language for string values
    pub default_language: Option<String>,
}

impl Default for JsonLdContextGeneratorConfig {
    fn default() -> Self {
        Self {
            base: GeneratorConfig::default(),
            base_uri: None,
            features: JsonLdFeatures::DEFAULT,
            default_language: None,
        }
    }
}

/// `JSON`-LD Context generator
pub struct JsonLdContextGenerator {
    config: JsonLdContextGeneratorConfig,
    /// Additional generator options for customization
    options: super::traits::GeneratorOptions,
}

impl JsonLdContextGenerator {
    /// Create a new `JSON`-LD Context generator
    #[must_use]
    pub fn new(config: JsonLdContextGeneratorConfig) -> Self {
        Self {
            config,
            options: super::traits::GeneratorOptions::default(),
        }
    }

    /// Create generator with custom options
    #[must_use]
    pub fn with_options(
        config: JsonLdContextGeneratorConfig,
        options: super::traits::GeneratorOptions,
    ) -> Self {
        Self { config, options }
    }

    /// Get custom option value
    fn get_custom_option(&self, key: &str) -> Option<&String> {
        self.options.custom.get(key)
    }

    /// Generate the context object
    fn generate_context(&self, schema: &SchemaDefinition) -> Result<Value, LinkMLError> {
        let mut context = Map::new();

        // Add base URI if provided
        if let Some(base) = &self.config.base_uri {
            context.insert("@base".to_string(), json!(base));
        }

        // Add default language if specified
        if let Some(lang) = &self.config.default_language {
            context.insert("@language".to_string(), json!(lang));
        }

        // Add prefix mappings
        if !schema.prefixes.is_empty() {
            for (prefix, expansion) in &schema.prefixes {
                if prefix != "@base" && prefix != "@language" {
                    let reference = match expansion {
                        PrefixDefinition::Simple(url) => url.clone(),
                        PrefixDefinition::Complex {
                            prefix_reference, ..
                        } => prefix_reference.clone().unwrap_or_default(),
                    };
                    context.insert(prefix.clone(), json!(reference));
                }
            }
        }

        // Add default prefix if available
        if let Some(default_prefix) = &schema.default_prefix
            && let Some(expansion) = schema.prefixes.get(default_prefix)
        {
            let reference = match expansion {
                PrefixDefinition::Simple(url) => url.clone(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.clone().unwrap_or_default(),
            };
            context.insert("@vocab".to_string(), json!(reference));
        }

        // Add class mappings
        if !schema.classes.is_empty() {
            for (class_name, class_def) in &schema.classes {
                self.add_class_to_context(class_name, class_def, &mut context, schema)?;
            }
        }

        // Add slot mappings
        if !schema.slots.is_empty() {
            for (slot_name, slot_def) in &schema.slots {
                self.add_slot_to_context(slot_name, slot_def, &mut context, schema)?;
            }
        }

        Ok(Value::Object(context))
    }

    /// Add a class to the context
    fn add_class_to_context(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        context: &mut Map<String, Value>,
        schema: &SchemaDefinition,
    ) -> Result<(), LinkMLError> {
        let mut class_mapping = Map::new();

        // Determine the IRI for the class
        let class_iri = self.get_iri_for_element(class_name, None, schema);
        class_mapping.insert("@id".to_string(), json!(class_iri));

        // Add type if this represents an RDF type
        if class_def.class_uri.is_some() || class_def.is_a.is_none() {
            class_mapping.insert("@type".to_string(), json!("@id"));
        }

        // Process class-specific slots
        if !class_def.slots.is_empty() {
            for slot_name in &class_def.slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    self.add_slot_to_context(slot_name, slot_def, context, schema)?;
                }
            }
        }

        // Process attributes
        if !class_def.attributes.is_empty() {
            for (attr_name, attr_def) in &class_def.attributes {
                self.add_slot_to_context(attr_name, attr_def, context, schema)?;
            }
        }

        // Only add if there are actual mappings
        if !class_mapping.is_empty() {
            context.insert(class_name.to_string(), Value::Object(class_mapping));
        }

        Ok(())
    }

    /// Add a slot to the context
    fn add_slot_to_context(
        &self,
        slot_name: &str,
        slot_def: &SlotDefinition,
        context: &mut Map<String, Value>,
        schema: &SchemaDefinition,
    ) -> Result<(), LinkMLError> {
        // Validate slot name is not empty
        if slot_name.trim().is_empty() {
            return Err(LinkMLError::data_validation(
                "Slot name cannot be empty".to_string(),
            ));
        }

        // Validate slot name doesn't conflict with JSON-LD keywords
        if [
            "@context",
            "@id",
            "@type",
            "@value",
            "@language",
            "@index",
            "@reverse",
            "@graph",
        ]
        .contains(&slot_name)
        {
            return Err(LinkMLError::data_validation(format!(
                "Slot name '{slot_name}' conflicts with JSON-LD keyword"
            )));
        }

        // Skip if already added
        if context.contains_key(slot_name) {
            return Ok(());
        }

        let mut slot_mapping = Map::new();

        // Determine the IRI for the slot
        let slot_iri = if let Some(uri) = &slot_def.slot_uri {
            uri.clone()
        } else {
            self.get_iri_for_element(slot_name, None, schema)
        };

        // Simple string mapping if no special handling needed
        if !self.needs_complex_mapping(slot_def, schema) {
            context.insert(slot_name.to_string(), json!(slot_iri));
            return Ok(());
        }

        // Complex mapping
        slot_mapping.insert("@id".to_string(), json!(slot_iri));

        // CRITICAL: Check options for include_type_coercion override
        let include_type_coercion = self.options.custom.get("include_type_coercion").map_or(
            self.config.features.contains(JsonLdFeatures::TYPE_COERCION),
            |v| v == "true",
        );

        // Add type coercion if enabled
        if include_type_coercion && let Some(type_value) = Self::get_type_coercion(slot_def, schema)
        {
            slot_mapping.insert("@type".to_string(), type_value);
        }

        // CRITICAL: Check options for include_containers override
        let include_containers = self.options.custom.get("include_containers").map_or(
            self.config.features.contains(JsonLdFeatures::CONTAINERS),
            |v| v == "true",
        );

        // Add container mapping for multivalued slots
        if include_containers && slot_def.multivalued == Some(true) {
            // Check if custom container type is specified in options
            let container_type = self
                .get_custom_option("multivalued_container")
                .map_or("@list", std::string::String::as_str);
            slot_mapping.insert("@container".to_string(), json!(container_type));
        }

        // CRITICAL: Check options for include_language_maps override
        let include_language_maps = self.options.custom.get("include_language_maps").map_or(
            self.config.features.contains(JsonLdFeatures::LANGUAGE_MAPS),
            |v| v == "true",
        );

        // Add language mapping if applicable
        if include_language_maps && Self::is_translatable_slot(slot_def) {
            slot_mapping.insert("@container".to_string(), json!("@language"));
        }

        context.insert(slot_name.to_string(), Value::Object(slot_mapping));
        Ok(())
    }

    /// Determine if a slot needs complex mapping
    fn needs_complex_mapping(&self, slot_def: &SlotDefinition, schema: &SchemaDefinition) -> bool {
        // Needs complex mapping if:
        // - Type coercion is enabled and slot has a specific type
        // - It's multivalued and containers are enabled
        // - It's translatable and language maps are enabled
        // - It has a specific slot URI different from default

        // CRITICAL: Check options for overrides
        let include_type_coercion = self.options.custom.get("include_type_coercion").map_or(
            self.config.features.contains(JsonLdFeatures::TYPE_COERCION),
            |v| v == "true",
        );

        let include_containers = self.options.custom.get("include_containers").map_or(
            self.config.features.contains(JsonLdFeatures::CONTAINERS),
            |v| v == "true",
        );

        let include_language_maps = self.options.custom.get("include_language_maps").map_or(
            self.config.features.contains(JsonLdFeatures::LANGUAGE_MAPS),
            |v| v == "true",
        );

        if include_type_coercion && Self::get_type_coercion(slot_def, schema).is_some() {
            return true;
        }

        if include_containers && slot_def.multivalued == Some(true) {
            return true;
        }

        if include_language_maps && Self::is_translatable_slot(slot_def) {
            return true;
        }

        slot_def.slot_uri.is_some()
    }

    /// Get type coercion for a slot
    fn get_type_coercion(slot_def: &SlotDefinition, schema: &SchemaDefinition) -> Option<Value> {
        if let Some(range) = &slot_def.range {
            // Check if it's a class reference
            if schema.classes.contains_key(range) {
                return Some(json!("@id"));
            }

            // Check if it's a type
            if !schema.types.is_empty()
                && let Some(type_def) = schema.types.get(range)
            {
                // Map to XSD types
                return match type_def.base_type.as_deref() {
                    Some("integer") => Some(json!("xsd:integer")),
                    Some("float") => Some(json!("xsd:float")),
                    Some("double") => Some(json!("xsd:double")),
                    Some("boolean") => Some(json!("xsd:boolean")),
                    Some("date") => Some(json!("xsd:date")),
                    Some("datetime") => Some(json!("xsd:dateTime")),
                    Some("time") => Some(json!("xsd:time")),
                    Some("uri") => Some(json!("@id")),
                    _ => None, // Default type including "string"
                };
            }

            // Direct type mapping
            match range.as_str() {
                "integer" | "int" => Some(json!("xsd:integer")),
                "float" => Some(json!("xsd:float")),
                "double" => Some(json!("xsd:double")),
                "boolean" | "bool" => Some(json!("xsd:boolean")),
                "date" => Some(json!("xsd:date")),
                "datetime" => Some(json!("xsd:dateTime")),
                "time" => Some(json!("xsd:time")),
                "uri" | "uriorcurie" => Some(json!("@id")),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Check if a slot is translatable
    fn is_translatable_slot(slot_def: &SlotDefinition) -> bool {
        // A slot is translatable if it's a string type and marked as translatable
        if let Some(range) = &slot_def.range
            && (range == "string" || range == "text")
        {
            // In a real implementation, we'd check for a translatable annotation
            return slot_def.description.is_some();
        }
        false
    }

    /// Get IRI for an element
    fn get_iri_for_element(
        &self,
        name: &str,
        id_prefixes: Option<&Vec<String>>,
        schema: &SchemaDefinition,
    ) -> String {
        // Check if there's a specific prefix for this element
        if let Some(prefixes) = id_prefixes
            && let Some(prefix) = prefixes.first()
            && let Some(expansion) = schema.prefixes.get(prefix)
        {
            let reference = match expansion {
                PrefixDefinition::Simple(url) => url.clone(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.clone().unwrap_or_default(),
            };
            return format!("{reference}{name}");
        }

        // CRITICAL: Check options for use_curies override
        let use_curies = self.options.custom.get("use_curies").map_or(
            self.config.features.contains(JsonLdFeatures::USE_CURIES),
            |v| v == "true",
        );

        // Use CURIE if enabled and default prefix exists
        if use_curies && let Some(default_prefix) = &schema.default_prefix {
            return format!("{default_prefix}:{name}");
        }

        // CRITICAL: Check options for base_uri override
        let base_uri = self
            .options
            .custom
            .get("base_uri")
            .or(self.config.base_uri.as_ref());

        // Use base URI if available
        if let Some(base) = base_uri {
            return format!("{base}{name}");
        }

        // Fallback to just the name
        name.to_string()
    }
}

impl Generator for JsonLdContextGenerator {
    fn name(&self) -> &'static str {
        "jsonld-context"
    }

    fn description(&self) -> &'static str {
        "Generate JSON-LD context from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for jsonldcontext generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<String> {
        let context = self.generate_context(schema)?;

        let mut output = json!({
            "@context": context
        });

        // Add documentation if enabled
        if self.options.include_docs {
            if let Some(description) = &schema.description
                && let Some(obj) = output.as_object_mut()
            {
                obj.insert("description".to_string(), json!(description));
            }

            // Add custom metadata from options
            if let Some(author) = self.get_custom_option("author")
                && let Some(obj) = output.as_object_mut()
            {
                obj.insert("author".to_string(), json!(author));
            }

            if let Some(license) = self.get_custom_option("license")
                && let Some(obj) = output.as_object_mut()
            {
                obj.insert("license".to_string(), json!(license));
            }
        }

        // Format JSON according to options
        match self.options.output_format {
            super::traits::OutputFormat::JSON => {
                if self.options.indent == super::traits::IndentStyle::Tabs {
                    // Custom formatting for tabs
                    let pretty = serde_json::to_string_pretty(&output).map_err(|e| {
                        LinkMLError::ServiceError(format!(
                            "Failed to serialize JSON-LD context: {e}"
                        ))
                    })?;
                    // Replace 4 spaces with tabs
                    Ok(pretty.replace("    ", "\t"))
                } else if let super::traits::IndentStyle::Spaces(n) = self.options.indent {
                    // Custom spacing
                    let spaces = vec![b' '; n];
                    let formatter = serde_json::ser::PrettyFormatter::with_indent(&spaces);
                    let mut buf = Vec::new();
                    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
                    output.serialize(&mut ser).map_err(|e| {
                        LinkMLError::ServiceError(format!(
                            "Failed to serialize JSON-LD context: {e}"
                        ))
                    })?;
                    String::from_utf8(buf).map_err(|e| {
                        LinkMLError::ServiceError(format!("Invalid UTF-8 in serialized JSON: {e}"))
                    })
                } else {
                    serde_json::to_string_pretty(&output).map_err(|e| {
                        LinkMLError::ServiceError(format!(
                            "Failed to serialize JSON-LD context: {e}"
                        ))
                    })
                }
            }
            _ => {
                // Default pretty printing for other formats
                serde_json::to_string_pretty(&output).map_err(|e| {
                    LinkMLError::ServiceError(format!("Failed to serialize JSON-LD context: {e}"))
                })
            }
        }
    }

    fn get_file_extension(&self) -> &'static str {
        "jsonld"
    }

    fn get_default_filename(&self) -> &'static str {
        "context"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_jsonld_context_generation() -> anyhow::Result<()> {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Add prefixes
        let mut prefixes = IndexMap::new();
        prefixes.insert(
            "ex".to_string(),
            PrefixDefinition::Simple("https://example.com/".to_string()),
        );
        prefixes.insert(
            "schema".to_string(),
            PrefixDefinition::Simple("https://schema.org/".to_string()),
        );
        schema.prefixes = prefixes;
        schema.default_prefix = Some("ex".to_string());

        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.description = Some("A person".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];

        let mut classes = IndexMap::new();
        classes.insert("Person".to_string(), person_class);
        schema.classes = classes;

        // Add slots
        let mut name_slot = SlotDefinition::default();
        name_slot.description = Some("The person's name".to_string());
        name_slot.range = Some("string".to_string());

        let mut age_slot = SlotDefinition::default();
        age_slot.description = Some("The person's age".to_string());
        age_slot.range = Some("integer".to_string());

        let mut slots = IndexMap::new();
        slots.insert("name".to_string(), name_slot);
        slots.insert("age".to_string(), age_slot);
        schema.slots = slots;

        let config = JsonLdContextGeneratorConfig {
            base_uri: Some("https://example.com/".to_string()),
            ..Default::default()
        };
        let generator = JsonLdContextGenerator::new(config);

        let result = generator
            .generate(&schema)
            .expect("should generate JSON-LD context: {}");

        // Verify key elements
        assert!(result.contains("@context"));
        assert!(result.contains("@base"));
        assert!(result.contains("https://example.com/"));
        assert!(result.contains("ex"));
        assert!(result.contains("schema"));
        assert!(result.contains("xsd:integer"));
        Ok(())
    }
}
