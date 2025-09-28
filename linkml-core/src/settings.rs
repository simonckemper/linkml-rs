//! Schema settings for LinkML
//!
//! This module defines settings that control schema processing behavior,
//! validation rules, and code generation options.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Schema settings that control processing behavior
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SchemaSettings {
    /// Validation settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<ValidationSettings>,

    /// Code generation settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationSettings>,

    /// Import resolution settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imports: Option<ImportSettings>,

    /// Default values for various schema elements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defaults: Option<DefaultSettings>,

    /// Naming convention settings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub naming: Option<NamingSettings>,

    /// Custom settings as key-value pairs
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Validation-related settings
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ValidationSettings {
    /// Whether to perform strict validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,

    /// Whether to validate permissible values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_permissibles: Option<bool>,

    /// Whether to validate identifier uniqueness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_identifiers: Option<bool>,

    /// Whether to fail on first error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_fast: Option<bool>,

    /// Maximum validation depth for recursive structures
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<usize>,

    /// Whether to allow additional properties not defined in schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_additional_properties: Option<bool>,

    /// Whether to fail on warnings (treat warnings as errors)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_warning: Option<bool>,

    /// Whether to validate default values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_defaults: Option<bool>,

    /// Whether to coerce types when possible
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_coercion: Option<bool>,
}

/// Code generation settings
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct GenerationSettings {
    /// Whether to generate builders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_builders: Option<bool>,

    /// Whether to generate validation code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_validation: Option<bool>,

    /// Whether to include examples in generated code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_examples: Option<bool>,

    /// Whether to generate documentation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_docs: Option<bool>,

    /// Target language-specific settings
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub language_options: HashMap<String, LanguageOptions>,

    /// Whether to use generic types where applicable
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_generics: Option<bool>,

    /// Whether to generate serialization code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_serialization: Option<bool>,
}

/// Language-specific generation options
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct LanguageOptions {
    /// Package/module name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,

    /// Import statements to include
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<String>,

    /// Custom type mappings
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub type_mappings: HashMap<String, String>,

    /// Language-specific features to enable
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,

    /// Custom settings
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom: HashMap<String, serde_json::Value>,
}

/// Import resolution settings
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ImportSettings {
    /// Base directories for import resolution
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub search_paths: Vec<String>,

    /// Base URL for URL imports
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Whether to follow imports recursively
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_imports: Option<bool>,

    /// Maximum import depth
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_import_depth: Option<usize>,

    /// Import aliases
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aliases: HashMap<String, String>,

    /// Whether to cache imported schemas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_imports: Option<bool>,

    /// Import resolution strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_strategy: Option<ImportResolutionStrategy>,
}

/// Import resolution strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ImportResolutionStrategy {
    /// Resolve imports relative to the importing file
    Relative,
    /// Resolve imports from search paths
    Absolute,
    /// Try relative first, then absolute
    Mixed,
}

/// Default value settings
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DefaultSettings {
    /// Default range for slots without explicit range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_range: Option<String>,

    /// Default cardinality for slots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot_cardinality: Option<CardinalityDefault>,

    /// Whether slots are required by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots_required: Option<bool>,

    /// Whether to inline objects by default
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_objects: Option<bool>,

    /// Default string serialization
    #[serde(skip_serializing_if = "Option::is_none")]
    pub string_serialization: Option<StringSerialization>,
}

/// Default cardinality settings
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CardinalityDefault {
    /// Single-valued (0..1)
    Single,
    /// Required single (1..1)
    RequiredSingle,
    /// Optional multiple (0..*)
    Multiple,
    /// Required multiple (1..*)
    RequiredMultiple,
}

/// String serialization format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StringSerialization {
    /// Use exact string values
    Exact,
    /// Convert to lowercase
    Lowercase,
    /// Convert to uppercase
    Uppercase,
    /// Use camelCase
    CamelCase,
    /// Use `snake_case`
    SnakeCase,
    /// Use `PascalCase`
    PascalCase,
}

/// Naming convention settings
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NamingSettings {
    /// Naming convention for classes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub classes: Option<NamingConvention>,

    /// Naming convention for slots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slots: Option<NamingConvention>,

    /// Naming convention for enums
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enums: Option<NamingConvention>,

    /// Naming convention for types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub types: Option<NamingConvention>,

    /// Whether to validate naming conventions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validate_names: Option<bool>,

    /// Whether to auto-correct names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_correct: Option<bool>,
}

/// Naming convention
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NamingConvention {
    /// `snake_case`
    SnakeCase,
    /// camelCase
    CamelCase,
    /// `PascalCase`
    PascalCase,
    /// kebab-case
    KebabCase,
    /// `SCREAMING_SNAKE_CASE`
    ScreamingSnakeCase,
    /// No convention enforced
    Any,
}

impl SchemaSettings {
    /// Create new schema settings with defaults
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create settings optimized for strict validation
    #[must_use]
    pub fn strict() -> Self {
        Self {
            validation: Some(ValidationSettings {
                strict: Some(true),
                check_permissibles: Some(true),
                check_identifiers: Some(true),
                fail_fast: Some(false),
                allow_additional_properties: Some(false),
                validate_defaults: Some(true),
                type_coercion: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Create settings optimized for code generation
    #[must_use]
    pub fn for_generation() -> Self {
        Self {
            generation: Some(GenerationSettings {
                generate_builders: Some(true),
                generate_validation: Some(true),
                include_examples: Some(true),
                generate_docs: Some(true),
                use_generics: Some(true),
                generate_serialization: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Merge two settings, with other taking precedence
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            validation: other.validation.or(self.validation),
            generation: other.generation.or(self.generation),
            imports: other.imports.or(self.imports),
            defaults: other.defaults.or(self.defaults),
            naming: other.naming.or(self.naming),
            custom: {
                let mut merged = self.custom;
                merged.extend(other.custom);
                merged
            },
        }
    }

    /// Get a custom setting value
    ///
    /// # Errors
    ///
    /// Returns an error if the value exists but cannot be deserialized to the target type.
    pub fn get_custom<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        match self.custom.get(key) {
            Some(value) => Ok(Some(serde_json::from_value(value.clone())?)),
            None => Ok(None),
        }
    }

    /// Set a custom setting value
    ///
    /// # Errors
    ///
    /// Returns an error if the value cannot be serialized to `JSON`.
    pub fn set_custom<T: serde::Serialize>(
        &mut self,
        key: &str,
        value: T,
    ) -> std::result::Result<(), serde_json::Error> {
        self.custom
            .insert(key.to_string(), serde_json::to_value(value)?);
        Ok(())
    }
}

impl ValidationSettings {
    /// Check if strict validation is enabled
    #[must_use]
    pub fn is_strict(&self) -> bool {
        self.strict.unwrap_or(false)
    }

    /// Check if additional properties are allowed
    #[must_use]
    pub fn allows_additional_properties(&self) -> bool {
        self.allow_additional_properties.unwrap_or(true)
    }
}

impl GenerationSettings {
    /// Get language-specific options
    #[must_use]
    pub fn get_language_options(&self, language: &str) -> Option<&LanguageOptions> {
        self.language_options.get(language)
    }

    /// Set language-specific options
    pub fn set_language_options(&mut self, language: &str, options: LanguageOptions) {
        self.language_options.insert(language.to_string(), options);
    }
}

impl ImportSettings {
    /// Check if imports should be followed
    #[must_use]
    pub fn should_follow_imports(&self) -> bool {
        self.follow_imports.unwrap_or(true)
    }

    /// Get the resolution strategy
    #[must_use]
    pub fn get_resolution_strategy(&self) -> ImportResolutionStrategy {
        self.resolution_strategy
            .unwrap_or(ImportResolutionStrategy::Mixed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_settings_default() {
        let settings = SchemaSettings::new();
        assert!(settings.validation.is_none());
        assert!(settings.generation.is_none());
        assert!(settings.custom.is_empty());
    }

    #[test]
    fn test_strict_settings() {
        let settings = SchemaSettings::strict();
        let Some(validation) = settings.validation else {
            panic!("validation settings should be present in strict settings - test invariant")
        };
        assert_eq!(validation.strict, Some(true));
        assert_eq!(validation.check_permissibles, Some(true));
        assert_eq!(validation.allow_additional_properties, Some(false));
    }

    #[test]
    fn test_generation_settings() {
        let settings = SchemaSettings::for_generation();
        let Some(generation) = settings.generation else {
            panic!("generation settings should be present")
        };
        assert_eq!(generation.generate_builders, Some(true));
        assert_eq!(generation.generate_validation, Some(true));
        assert_eq!(generation.generate_docs, Some(true));
    }

    #[test]
    fn test_custom_settings() -> crate::Result<()> {
        let mut settings = SchemaSettings::new();

        // Set a custom value
        settings.set_custom("max_items", 100).map_err(|e| {
            anyhow::anyhow!("setting custom value with valid data should not fail in test: {e}")
        })?;

        // Get the custom value
        let max_items: Option<i32> = settings.get_custom("max_items").map_err(|e| {
            anyhow::anyhow!("getting custom value with valid data should not fail in test: {e}")
        })?;
        assert_eq!(max_items, Some(100));
        Ok(())
    }

    #[test]
    fn test_settings_merge() {
        let base = SchemaSettings {
            validation: Some(ValidationSettings {
                strict: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        };

        let override_settings = SchemaSettings {
            generation: Some(GenerationSettings {
                generate_builders: Some(false),
                ..Default::default()
            }),
            ..Default::default()
        };

        let merged = base.merge(override_settings);

        assert!(merged.validation.is_some());
        assert!(merged.generation.is_some());
        let Some(ref generation) = merged.generation else {
            panic!("generation settings should be present after merge - test invariant")
        };
        assert_eq!(generation.generate_builders, Some(false));
    }

    #[test]
    fn test_serialization() -> crate::Result<()> {
        let settings = SchemaSettings::strict();
        let json = serde_json::to_string_pretty(&settings).map_err(|e| {
            anyhow::anyhow!("serialization of valid settings should not fail in test: {e}")
        })?;

        let deserialized: SchemaSettings = serde_json::from_str(&json).map_err(|e| {
            anyhow::anyhow!("deserialization of valid JSON should not fail in test: {e}")
        })?;
        assert_eq!(settings, deserialized);
        Ok(())
    }
}
