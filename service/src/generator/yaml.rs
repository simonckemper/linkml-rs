//! YAML generator for `LinkML` schemas
//!
//! This generator serializes `LinkML` schemas back to YAML format,
//! preserving structure and optionally comments.

use super::traits::Generator;
use indexmap::IndexMap;
use linkml_core::metadata::Contributor;
use linkml_core::prelude::*;
use linkml_core::types::PrefixDefinition;
use serde_yaml;

/// `YAML` schema generator
pub struct YamlGenerator {
    /// Whether to include generated metadata
    include_metadata: bool,
    /// Whether to sort keys alphabetically
    sort_keys: bool,
    /// Whether to inline simple definitions
    inline_simple: bool,
    /// Whether to include null values in output
    include_nulls: bool,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl Default for YamlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl YamlGenerator {
    /// Create a new `YAML` generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            include_metadata: true,
            sort_keys: false,
            inline_simple: true,
            include_nulls: false,
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

    /// Configure metadata inclusion
    #[must_use]
    pub fn with_metadata(mut self, include: bool) -> Self {
        self.include_metadata = include;
        self
    }

    /// Configure key sorting
    #[must_use]
    pub fn with_sorted_keys(mut self, sort: bool) -> Self {
        self.sort_keys = sort;
        self
    }

    /// Configure inline simple definitions
    #[must_use]
    pub fn with_inline_simple(mut self, inline: bool) -> Self {
        self.inline_simple = inline;
        self
    }

    /// Configure null value inclusion
    #[must_use]
    pub fn with_include_nulls(mut self, include: bool) -> Self {
        self.include_nulls = include;
        self
    }

    /// Generate `YAML` from schema
    fn generate_yaml(&self, schema: &SchemaDefinition) -> Result<String> {
        // Create ordered map for consistent output
        let mut root = IndexMap::new();

        // Core metadata
        root.insert(
            "id".to_string(),
            serde_yaml::Value::String(schema.id.clone()),
        );
        root.insert(
            "name".to_string(),
            serde_yaml::Value::String(schema.name.clone()),
        );

        // Handle optional fields based on include_nulls setting
        match &schema.version {
            Some(version) => root.insert(
                "version".to_string(),
                serde_yaml::Value::String(version.clone()),
            ),
            None if self.include_nulls => {
                root.insert("version".to_string(), serde_yaml::Value::Null)
            }
            None => None,
        };

        match &schema.title {
            Some(title) => root.insert(
                "title".to_string(),
                serde_yaml::Value::String(title.clone()),
            ),
            None if self.include_nulls => root.insert("title".to_string(), serde_yaml::Value::Null),
            None => None,
        };

        match &schema.description {
            Some(description) => root.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            ),
            None if self.include_nulls => {
                root.insert("description".to_string(), serde_yaml::Value::Null)
            }
            None => None,
        };

        // License and metadata
        if let Some(license) = &schema.license {
            root.insert(
                "license".to_string(),
                serde_yaml::Value::String(license.clone()),
            );
        }

        if self.include_metadata {
            // These fields are not present in the current LinkML specification
            // created_by, created_on, modified_by, last_updated_on, generation_date
            // Need to check if they should be added to core or removed from generator
        }

        // Contributors
        if !schema.contributors.is_empty() {
            let contributors: Vec<serde_yaml::Value> = schema
                .contributors
                .iter()
                .map(|c| Self::contributor_to_yaml(c))
                .collect();
            root.insert(
                "contributors".to_string(),
                serde_yaml::Value::Sequence(contributors),
            );
        }

        // Categories and keywords
        if !schema.categories.is_empty() {
            let categories = serde_yaml::Value::Sequence(
                schema
                    .categories
                    .iter()
                    .map(|s| serde_yaml::Value::String(s.clone()))
                    .collect(),
            );
            root.insert("categories".to_string(), categories);
        }

        if !schema.keywords.is_empty() {
            let keywords = serde_yaml::Value::Sequence(
                schema
                    .keywords
                    .iter()
                    .map(|s| serde_yaml::Value::String(s.clone()))
                    .collect(),
            );
            root.insert("keywords".to_string(), keywords);
        }

        // See also
        if !schema.see_also.is_empty() {
            let see_also = serde_yaml::Value::Sequence(
                schema
                    .see_also
                    .iter()
                    .map(|s| serde_yaml::Value::String(s.clone()))
                    .collect(),
            );
            root.insert("see_also".to_string(), see_also);
        }

        // Imports
        if !schema.imports.is_empty() {
            let imports = serde_yaml::Value::Sequence(
                schema
                    .imports
                    .iter()
                    .map(|s| serde_yaml::Value::String(s.clone()))
                    .collect(),
            );
            root.insert("imports".to_string(), imports);
        }

        // Prefixes
        if !schema.prefixes.is_empty() {
            let mut prefixes = IndexMap::new();
            for (name, def) in &schema.prefixes {
                let value = match def {
                    PrefixDefinition::Simple(url) => serde_yaml::Value::String(url.clone()),
                    PrefixDefinition::Complex {
                        prefix_prefix,
                        prefix_reference,
                    } => {
                        let mut prefix_map = IndexMap::new();
                        prefix_map.insert(
                            "prefix_prefix".to_string(),
                            serde_yaml::Value::String(prefix_prefix.clone()),
                        );
                        if let Some(reference) = prefix_reference {
                            prefix_map.insert(
                                "prefix_reference".to_string(),
                                serde_yaml::Value::String(reference.clone()),
                            );
                        }
                        serde_yaml::Value::Mapping(
                            prefix_map
                                .into_iter()
                                .map(|(k, v)| (serde_yaml::Value::String(k), v))
                                .collect::<serde_yaml::Mapping>(),
                        )
                    }
                };
                prefixes.insert(name.clone(), value);
            }
            root.insert(
                "prefixes".to_string(),
                serde_yaml::Value::Mapping(
                    prefixes
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Default settings
        if let Some(default_prefix) = &schema.default_prefix {
            root.insert(
                "default_prefix".to_string(),
                serde_yaml::Value::String(default_prefix.clone()),
            );
        }
        if let Some(default_range) = &schema.default_range {
            root.insert(
                "default_range".to_string(),
                serde_yaml::Value::String(default_range.clone()),
            );
        }

        // Subsets
        if !schema.subsets.is_empty() {
            let mut subsets = IndexMap::new();
            for (name, def) in &schema.subsets {
                subsets.insert(name.clone(), self.subset_to_yaml(def));
            }
            root.insert(
                "subsets".to_string(),
                serde_yaml::Value::Mapping(
                    subsets
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Types
        if !schema.types.is_empty() {
            let mut types = IndexMap::new();
            for (name, def) in &schema.types {
                types.insert(name.clone(), self.type_to_yaml(def));
            }
            root.insert(
                "types".to_string(),
                serde_yaml::Value::Mapping(
                    types
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Enums
        if !schema.enums.is_empty() {
            let mut enums = IndexMap::new();
            for (name, def) in &schema.enums {
                enums.insert(name.clone(), self.enum_to_yaml(def));
            }
            root.insert(
                "enums".to_string(),
                serde_yaml::Value::Mapping(
                    enums
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Slots
        if !schema.slots.is_empty() {
            let mut slots = IndexMap::new();
            for (name, def) in &schema.slots {
                slots.insert(name.clone(), self.slot_to_yaml(def));
            }
            root.insert(
                "slots".to_string(),
                serde_yaml::Value::Mapping(
                    slots
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Classes
        if !schema.classes.is_empty() {
            let mut classes = IndexMap::new();
            for (name, def) in &schema.classes {
                classes.insert(name.clone(), self.class_to_yaml(def));
            }
            root.insert(
                "classes".to_string(),
                serde_yaml::Value::Mapping(
                    classes
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v))
                        .collect::<serde_yaml::Mapping>(),
                ),
            );
        }

        // Settings
        if let Some(settings) = &schema.settings {
            root.insert("settings".to_string(), self.settings_to_yaml(settings));
        }

        // Convert to YAML string
        let yaml_value = serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            root.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ));
        serde_yaml::to_string(&yaml_value)
            .map_err(|e| LinkMLError::SerializationError(format!("YAML generation failed: {e}")))
    }

    /// Convert contributor to `YAML`
    fn contributor_to_yaml(contributor: &Contributor) -> serde_yaml::Value {
        let mut map = IndexMap::new();
        map.insert(
            "name".to_string(),
            serde_yaml::Value::String(contributor.name.clone()),
        );

        if let Some(email) = &contributor.email {
            map.insert(
                "email".to_string(),
                serde_yaml::Value::String(email.clone()),
            );
        }
        if let Some(github) = &contributor.github {
            map.insert(
                "github".to_string(),
                serde_yaml::Value::String(github.clone()),
            );
        }
        if let Some(orcid) = &contributor.orcid {
            map.insert(
                "orcid".to_string(),
                serde_yaml::Value::String(orcid.clone()),
            );
        }
        if let Some(role) = &contributor.role {
            map.insert("role".to_string(), serde_yaml::Value::String(role.clone()));
        }

        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            map.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ))
    }

    /// Convert subset to `YAML`
    fn subset_to_yaml(&self, subset: &SubsetDefinition) -> serde_yaml::Value {
        let mut map = IndexMap::new();

        if let Some(description) = &subset.description {
            map.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            );
        }

        if self.inline_simple && map.len() == 1 && subset.description.is_some() {
            // Return just the description for simple subsets
            serde_yaml::Value::String(
                subset
                    .description
                    .as_ref()
                    .expect("description exists after is_some check")
                    .clone(),
            )
        } else {
            serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                map.into_iter()
                    .map(|(k, v)| (serde_yaml::Value::String(k), v)),
            ))
        }
    }

    /// Convert type to `YAML`
    fn type_to_yaml(&self, type_def: &TypeDefinition) -> serde_yaml::Value {
        let mut map = IndexMap::new();

        if let Some(uri) = &type_def.uri {
            map.insert("uri".to_string(), serde_yaml::Value::String(uri.clone()));
        }
        if let Some(base_type) = &type_def.base_type {
            map.insert(
                "typeof".to_string(),
                serde_yaml::Value::String(base_type.clone()),
            );
        }
        if let Some(description) = &type_def.description {
            map.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            );
        }
        if let Some(pattern) = &type_def.pattern {
            map.insert(
                "pattern".to_string(),
                serde_yaml::Value::String(pattern.clone()),
            );
        }

        // Add min/max values if present
        if let Some(min_val) = &type_def.minimum_value {
            map.insert(
                "minimum_value".to_string(),
                serde_yaml::to_value(min_val).unwrap_or(serde_yaml::Value::Null),
            );
        }
        if let Some(max_val) = &type_def.maximum_value {
            map.insert(
                "maximum_value".to_string(),
                serde_yaml::to_value(max_val).unwrap_or(serde_yaml::Value::Null),
            );
        }

        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            map.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ))
    }

    /// Convert enum to `YAML`
    fn enum_to_yaml(&self, enum_def: &EnumDefinition) -> serde_yaml::Value {
        let mut map = IndexMap::new();

        if let Some(description) = &enum_def.description {
            map.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            );
        }

        // Permissible values
        if !enum_def.permissible_values.is_empty() {
            let mut pv_map = IndexMap::new();
            for pv in &enum_def.permissible_values {
                let text = match pv {
                    PermissibleValue::Simple(s) => s.clone(),
                    PermissibleValue::Complex { text, .. } => text.clone(),
                };
                pv_map.insert(text, self.permissible_value_to_yaml(pv));
            }
            map.insert(
                "permissible_values".to_string(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                    pv_map
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v)),
                )),
            );
        }

        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            map.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ))
    }

    /// Convert permissible value to `YAML`
    fn permissible_value_to_yaml(&self, pv: &PermissibleValue) -> serde_yaml::Value {
        match pv {
            PermissibleValue::Simple(_) => {
                // For simple values, return empty mapping or null
                serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
            }
            PermissibleValue::Complex {
                description,
                meaning,
                ..
            } => {
                let mut map = IndexMap::new();

                if let Some(desc) = description {
                    map.insert(
                        "description".to_string(),
                        serde_yaml::Value::String(desc.clone()),
                    );
                }
                if let Some(mean) = meaning {
                    map.insert(
                        "meaning".to_string(),
                        serde_yaml::Value::String(mean.clone()),
                    );
                }

                if self.inline_simple && map.len() == 1 && description.is_some() {
                    serde_yaml::Value::String(
                        description
                            .as_ref()
                            .expect("description exists after is_some check")
                            .clone(),
                    )
                } else if map.is_empty() {
                    serde_yaml::Value::Mapping(serde_yaml::Mapping::new())
                } else {
                    serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                        map.into_iter()
                            .map(|(k, v)| (serde_yaml::Value::String(k), v)),
                    ))
                }
            }
        }
    }

    /// Convert slot to `YAML`
    fn slot_to_yaml(&self, slot: &SlotDefinition) -> serde_yaml::Value {
        let mut map = IndexMap::new();

        // Basic properties
        if let Some(description) = &slot.description {
            map.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            );
        }
        if let Some(range) = &slot.range {
            map.insert(
                "range".to_string(),
                serde_yaml::Value::String(range.clone()),
            );
        }

        // Inheritance
        if let Some(is_a) = &slot.is_a {
            map.insert("is_a".to_string(), serde_yaml::Value::String(is_a.clone()));
        }
        if !slot.mixins.is_empty() {
            map.insert(
                "mixins".to_string(),
                serde_yaml::Value::Sequence(
                    slot.mixins
                        .iter()
                        .map(|s| serde_yaml::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }

        // Cardinality
        if let Some(required) = slot.required {
            map.insert("required".to_string(), serde_yaml::Value::Bool(required));
        }
        if let Some(multivalued) = slot.multivalued {
            map.insert(
                "multivalued".to_string(),
                serde_yaml::Value::Bool(multivalued),
            );
        }

        // Constraints
        if let Some(pattern) = &slot.pattern {
            map.insert(
                "pattern".to_string(),
                serde_yaml::Value::String(pattern.clone()),
            );
        }
        if let Some(min_val) = &slot.minimum_value {
            map.insert(
                "minimum_value".to_string(),
                serde_yaml::to_value(min_val).unwrap_or(serde_yaml::Value::Null),
            );
        }
        if let Some(max_val) = &slot.maximum_value {
            map.insert(
                "maximum_value".to_string(),
                serde_yaml::to_value(max_val).unwrap_or(serde_yaml::Value::Null),
            );
        }

        // Other properties
        if let Some(slot_uri) = &slot.slot_uri {
            map.insert(
                "slot_uri".to_string(),
                serde_yaml::Value::String(slot_uri.clone()),
            );
        }
        if !slot.aliases.is_empty() {
            map.insert(
                "aliases".to_string(),
                serde_yaml::Value::Sequence(
                    slot.aliases
                        .iter()
                        .map(|s| serde_yaml::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }

        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            map.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ))
    }

    /// Convert class to `YAML`
    fn class_to_yaml(&self, class: &ClassDefinition) -> serde_yaml::Value {
        let mut map = IndexMap::new();

        // Basic properties
        if let Some(description) = &class.description {
            map.insert(
                "description".to_string(),
                serde_yaml::Value::String(description.clone()),
            );
        }

        // Inheritance
        if let Some(is_a) = &class.is_a {
            map.insert("is_a".to_string(), serde_yaml::Value::String(is_a.clone()));
        }
        if !class.mixins.is_empty() {
            map.insert(
                "mixins".to_string(),
                serde_yaml::Value::Sequence(
                    class
                        .mixins
                        .iter()
                        .map(|s| serde_yaml::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }

        // Slots and attributes
        if !class.slots.is_empty() {
            map.insert(
                "slots".to_string(),
                serde_yaml::Value::Sequence(
                    class
                        .slots
                        .iter()
                        .map(|s| serde_yaml::Value::String(s.clone()))
                        .collect(),
                ),
            );
        }
        if !class.attributes.is_empty() {
            let mut attrs = IndexMap::new();
            for (name, slot) in &class.attributes {
                attrs.insert(name.clone(), self.slot_to_yaml(slot));
            }
            map.insert(
                "attributes".to_string(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                    attrs
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v)),
                )),
            );
        }

        // Slot usage
        if !class.slot_usage.is_empty() {
            let mut usage = IndexMap::new();
            for (name, slot) in &class.slot_usage {
                usage.insert(name.clone(), self.slot_to_yaml(slot));
            }
            map.insert(
                "slot_usage".to_string(),
                serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
                    usage
                        .into_iter()
                        .map(|(k, v)| (serde_yaml::Value::String(k), v)),
                )),
            );
        }

        // Class properties
        if let Some(abstract_) = class.abstract_ {
            map.insert("abstract".to_string(), serde_yaml::Value::Bool(abstract_));
        }
        if let Some(mixin) = class.mixin {
            map.insert("mixin".to_string(), serde_yaml::Value::Bool(mixin));
        }
        if let Some(class_uri) = &class.class_uri {
            map.insert(
                "class_uri".to_string(),
                serde_yaml::Value::String(class_uri.clone()),
            );
        }

        serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter(
            map.into_iter()
                .map(|(k, v)| (serde_yaml::Value::String(k), v)),
        ))
    }

    /// Convert settings to `YAML`
    fn settings_to_yaml(&self, settings: &SchemaSettings) -> serde_yaml::Value {
        let yaml_str = serde_yaml::to_string(settings).unwrap_or_default();
        serde_yaml::from_str(&yaml_str).unwrap_or(serde_yaml::Value::Null)
    }
}

impl Generator for YamlGenerator {
    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for yaml generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        self.generate_yaml(schema)
    }

    fn name(&self) -> &'static str {
        "yaml"
    }

    fn description(&self) -> &'static str {
        "Generate YAML representation of LinkML schema"
    }

    fn get_file_extension(&self) -> &'static str {
        "yaml"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema.yaml"
    }
}
