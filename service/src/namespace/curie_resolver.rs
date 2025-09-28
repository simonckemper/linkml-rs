//! CURIE (Compact URI) resolution and namespace management
//!
//! This module provides comprehensive CURIE/URI resolution matching
//! the Kapernikov `LinkML` implementation's namespace handling.

use linkml_core::prelude::*;
use regex::Regex;
use std::collections::HashMap;

/// Regular expression for valid CURIE format
static CURIE_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z][a-zA-Z0-9_]*):([^:]*)$").expect("Valid CURIE regex pattern")
});

/// Regular expression for valid URI format (matches absolute URIs with scheme://
/// or well-known schemes like mailto:, urn:, etc.)
static URI_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^([a-zA-Z][a-zA-Z0-9+.-]*://.+|mailto:.+|urn:.+|data:.+|file:.+)")
        .expect("Valid URI regex pattern")
});

/// CURIE/URI resolver for `LinkML` schemas
#[derive(Debug, Clone)]
pub struct CurieResolver {
    /// Prefix to URI mappings
    prefixes: HashMap<String, String>,

    /// URI to prefix reverse mappings (for contraction)
    uri_to_prefix: HashMap<String, String>,

    /// Default prefix for the schema
    default_prefix: Option<String>,

    /// Base URI for relative references
    base_uri: Option<String>,

    /// Strict mode - fail on unknown prefixes
    strict: bool,
}

impl Default for CurieResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CurieResolver {
    /// Create a new CURIE resolver
    #[must_use]
    pub fn new() -> Self {
        let mut resolver = Self {
            prefixes: HashMap::new(),
            uri_to_prefix: HashMap::new(),
            default_prefix: None,
            base_uri: None,
            strict: false,
        };

        // Add standard prefixes
        resolver.add_builtin_prefixes();
        resolver
    }

    /// Create from a `LinkML` schema
    #[must_use]
    pub fn from_schema(schema: &SchemaDefinition) -> Self {
        let mut resolver = Self::new();

        // Set default prefix
        resolver.default_prefix.clone_from(&schema.default_prefix);

        // Add schema prefixes
        for (prefix, definition) in &schema.prefixes {
            let uri = match definition {
                PrefixDefinition::Simple(uri) => uri.clone(),
                PrefixDefinition::Complex {
                    prefix_reference, ..
                } => prefix_reference.clone().unwrap_or_default(),
            };
            resolver.add_prefix(prefix, &uri);
        }

        // Set base URI from schema ID if available
        if !schema.id.is_empty() {
            resolver.base_uri = Some(schema.id.clone());
        }

        resolver
    }

    /// Add built-in prefixes
    fn add_builtin_prefixes(&mut self) {
        // Standard semantic web prefixes
        self.add_prefix("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#");
        self.add_prefix("rdfs", "http://www.w3.org/2000/01/rdf-schema#");
        self.add_prefix("xsd", "http://www.w3.org/2001/XMLSchema#");
        self.add_prefix("owl", "http://www.w3.org/2002/07/owl#");
        self.add_prefix("skos", "http://www.w3.org/2004/02/skos/core#");
        self.add_prefix("dcterms", "http://purl.org/dc/terms/");
        self.add_prefix("schema", "http://schema.org/");

        // LinkML specific prefixes
        self.add_prefix("linkml", "https://w3id.org/linkml/");
        self.add_prefix("biolink", "https://w3id.org/biolink/");
    }

    /// Add a prefix mapping
    pub fn add_prefix(&mut self, prefix: &str, uri: &str) {
        self.prefixes.insert(prefix.to_string(), uri.to_string());
        self.uri_to_prefix
            .insert(uri.to_string(), prefix.to_string());
    }

    /// Set the default prefix
    pub fn set_default_prefix(&mut self, prefix: &str) {
        self.default_prefix = Some(prefix.to_string());
    }

    /// Set the base URI
    pub fn set_base_uri(&mut self, uri: &str) {
        self.base_uri = Some(uri.to_string());
    }

    /// Set strict mode
    pub fn set_strict(&mut self, strict: bool) {
        self.strict = strict;
    }

    /// Check if a string is a CURIE
    pub fn is_curie(&self, s: &str) -> bool {
        CURIE_REGEX.is_match(s)
    }

    /// Check if a string is a URI
    pub fn is_uri(&self, s: &str) -> bool {
        URI_REGEX.is_match(s) || s.starts_with("http://") || s.starts_with("https://")
    }

    /// Expand a CURIE to a full URI.
    ///
    /// # Errors
    ///
    /// Returns an error when the prefix in the provided CURIE is unknown and
    /// the resolver is configured to operate in strict mode.
    pub fn expand_curie(&self, curie: &str) -> Result<String> {
        // Check if it's already a URI
        if self.is_uri(curie) {
            return Ok(curie.to_string());
        }

        // Check if it's a valid CURIE
        if let Some(captures) = CURIE_REGEX.captures(curie) {
            let prefix = captures
                .get(1)
                .ok_or_else(|| LinkMLError::service(format!("Invalid CURIE format: {curie}")))?
                .as_str();
            let local = captures
                .get(2)
                .ok_or_else(|| LinkMLError::service(format!("Invalid CURIE format: {curie}")))?
                .as_str();

            // Look up the prefix
            if let Some(uri_base) = self.prefixes.get(prefix) {
                return Ok(format!("{uri_base}{local}"));
            } else if self.strict {
                return Err(LinkMLError::service(format!("Unknown prefix: {prefix}")));
            }
            // In non-strict mode, return as-is
            return Ok(curie.to_string());
        }

        // Handle default prefix
        if !curie.contains(':') {
            if let Some(ref default) = self.default_prefix
                && let Some(uri_base) = self.prefixes.get(default)
            {
                return Ok(format!("{uri_base}{curie}"));
            }

            // Use base URI if available
            if let Some(ref base) = self.base_uri {
                return Ok(format!("{base}{curie}"));
            }
        }

        // Return as-is if we can't expand
        Ok(curie.to_string())
    }

    /// Contract a URI to a CURIE if possible
    #[must_use]
    pub fn contract_uri(&self, uri: &str) -> String {
        // Check if it's already a CURIE
        if self.is_curie(uri) && !self.is_uri(uri) {
            return uri.to_string();
        }

        // Try to find the longest matching prefix URI
        let mut best_match: Option<(&str, &str)> = None;
        let mut best_length = 0;

        for (uri_base, prefix) in &self.uri_to_prefix {
            if uri.starts_with(uri_base) && uri_base.len() > best_length {
                best_match = Some((prefix, uri_base));
                best_length = uri_base.len();
            }
        }

        if let Some((prefix, uri_base)) = best_match {
            let local = &uri[uri_base.len()..];
            return format!("{prefix}:{local}");
        }

        // Return as-is if we can't contract
        uri.to_string()
    }

    /// Resolve any identifier (name, CURIE, or URI) to a full URI.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier cannot be expanded with the
    /// current resolver configuration.
    pub fn resolve(&self, identifier: &str) -> Result<String> {
        // First try to expand as CURIE
        let expanded = self.expand_curie(identifier)?;

        // If it's a full URI, return it
        if self.is_uri(&expanded) {
            Ok(expanded)
        } else if let Some(ref base) = self.base_uri {
            // Otherwise, resolve against base URI
            Ok(format!("{base}/{expanded}"))
        } else {
            Ok(expanded)
        }
    }

    /// Get all registered prefixes
    #[must_use]
    pub fn prefixes(&self) -> &HashMap<String, String> {
        &self.prefixes
    }

    /// Get a specific prefix expansion
    #[must_use]
    pub fn get_prefix(&self, prefix: &str) -> Option<&str> {
        self.prefixes.get(prefix).map(std::string::String::as_str)
    }

    /// Normalize an identifier to a consistent form.
    ///
    /// # Errors
    ///
    /// Returns an error when the identifier cannot be resolved to a known
    /// namespace.
    pub fn normalize(&self, identifier: &str) -> Result<String> {
        // Expand to full URI then contract back to preferred CURIE
        let uri = self.resolve(identifier)?;
        Ok(self.contract_uri(&uri))
    }

    /// Check if two identifiers refer to the same entity.
    ///
    /// # Errors
    ///
    /// Returns an error when either identifier cannot be expanded to a fully
    /// qualified URI.
    pub fn same_entity(&self, id1: &str, id2: &str) -> Result<bool> {
        let uri1 = self.resolve(id1)?;
        let uri2 = self.resolve(id2)?;
        Ok(uri1 == uri2)
    }
}

/// Namespace context for a schema element
#[derive(Debug, Clone)]
pub struct NamespaceContext {
    /// Resolver for this context
    resolver: CurieResolver,

    /// Local prefixes specific to this context
    local_prefixes: HashMap<String, String>,

    /// Namespace of the current element
    namespace: Option<String>,
}

impl NamespaceContext {
    /// Create a new namespace context
    #[must_use]
    pub fn new(resolver: CurieResolver) -> Self {
        Self {
            resolver,
            local_prefixes: HashMap::new(),
            namespace: None,
        }
    }

    /// Create a child context with additional prefixes
    #[must_use]
    pub fn child(&self) -> Self {
        Self {
            resolver: self.resolver.clone(),
            local_prefixes: self.local_prefixes.clone(),
            namespace: self.namespace.clone(),
        }
    }

    /// Add a local prefix
    pub fn add_local_prefix(&mut self, prefix: &str, uri: &str) {
        self.local_prefixes
            .insert(prefix.to_string(), uri.to_string());
    }

    /// Set the namespace for this context
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = Some(namespace.to_string());
    }

    /// Resolve an identifier in this context
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn resolve(&self, identifier: &str) -> Result<String> {
        // Check local prefixes first
        if let Some(captures) = CURIE_REGEX.captures(identifier) {
            let prefix = captures
                .get(1)
                .ok_or_else(|| LinkMLError::service(format!("Invalid CURIE format: {identifier}")))?
                .as_str();
            if let Some(uri_base) = self.local_prefixes.get(prefix) {
                let local = captures
                    .get(2)
                    .ok_or_else(|| LinkMLError::service(format!("Invalid CURIE format: {identifier}")))?
                    .as_str();
                return Ok(format!("{uri_base}{local}"));
            }
        }

        // Fall back to main resolver
        self.resolver.resolve(identifier)
    }
}

/// Utilities for working with CURIEs and URIs
pub mod utils {
    use super::{CURIE_REGEX, URI_REGEX};

    /// Split a CURIE into prefix and local parts
    pub fn split_curie(curie: &str) -> Option<(&str, &str)> {
        if let Some(captures) = CURIE_REGEX.captures(curie) {
            let prefix = captures.get(1)?.as_str();
            let local = captures.get(2)?.as_str();
            Some((prefix, local))
        } else {
            None
        }
    }

    /// Create a CURIE from prefix and local parts
    #[must_use]
    pub fn make_curie(prefix: &str, local: &str) -> String {
        format!("{prefix}:{local}")
    }

    /// Extract the local part from a URI given a base
    #[must_use]
    pub fn local_from_uri(uri: &str, base: &str) -> Option<String> {
        uri.strip_prefix(base).map(std::string::ToString::to_string)
    }

    /// Check if a URI is absolute
    pub fn is_absolute_uri(uri: &str) -> bool {
        URI_REGEX.is_match(uri) || uri.starts_with("http://") || uri.starts_with("https://")
    }

    /// Join a base URI with a relative reference
    #[must_use]
    pub fn join_uri(base: &str, relative: &str) -> String {
        if is_absolute_uri(relative) {
            relative.to_string()
        } else {
            let base = base.trim_end_matches('/');
            let relative = relative.trim_start_matches('/');
            format!("{base}/{relative}")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curie_expansion() {
        let mut resolver = CurieResolver::new();
        resolver.add_prefix("ex", "http://example.org/");

        // Test CURIE expansion
        assert_eq!(
            resolver
                .expand_curie("ex:Person")
                .expect("Should expand CURIE"),
            "http://example.org/Person"
        );

        // Test URI pass-through
        assert_eq!(
            resolver
                .expand_curie("http://example.org/Person")
                .expect("Should pass through URI"),
            "http://example.org/Person"
        );

        // Test default prefix
        resolver.set_default_prefix("ex");
        assert_eq!(
            resolver
                .expand_curie("Person")
                .expect("Should expand with default prefix"),
            "http://example.org/Person"
        );
    }

    #[test]
    fn test_uri_contraction() {
        let mut resolver = CurieResolver::new();
        resolver.add_prefix("ex", "http://example.org/");
        resolver.add_prefix("schema", "http://schema.org/");

        // Test URI contraction
        assert_eq!(
            resolver.contract_uri("http://example.org/Person"),
            "ex:Person"
        );

        // Test longest match wins
        resolver.add_prefix("ex_people", "http://example.org/people/");
        assert_eq!(
            resolver.contract_uri("http://example.org/people/John"),
            "ex_people:John"
        );
    }

    #[test]
    fn test_same_entity() {
        let mut resolver = CurieResolver::new();
        resolver.add_prefix("ex", "http://example.org/");

        // Different representations of the same entity
        assert!(
            resolver
                .same_entity("ex:Person", "http://example.org/Person")
                .expect("Should check same entity")
        );

        // Different entities
        assert!(
            !resolver
                .same_entity("ex:Person", "ex:Animal")
                .expect("Should check different entities")
        );
    }

    #[test]
    fn test_namespace_context() {
        let mut resolver = CurieResolver::new();
        resolver.add_prefix("global", "http://global.org/");

        let mut context = NamespaceContext::new(resolver);
        context.add_local_prefix("local", "http://local.org/");

        // Local prefix takes precedence
        assert_eq!(
            context
                .resolve("local:Thing")
                .expect("Should resolve local prefix"),
            "http://local.org/Thing"
        );

        // Global prefix still works
        assert_eq!(
            context
                .resolve("global:Thing")
                .expect("Should resolve global prefix"),
            "http://global.org/Thing"
        );
    }
}
