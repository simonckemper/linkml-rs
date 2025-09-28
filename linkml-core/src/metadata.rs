//! Metadata support for LinkML schemas
//!
//! This module provides standard metadata fields that can be attached to
//! various LinkML schema elements. This includes authorship information,
//! mappings to external ontologies, and documentation fields.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Metadata that can be attached to schema elements
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ElementMetadata {
    /// Alternative names for this element
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,

    /// List of CURIEs or URLs to related entities
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub see_also: Vec<String>,

    /// Exact mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exact_mappings: Vec<String>,

    /// Close mappings to external ontology terms
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub close_mappings: Vec<String>,

    /// Narrow mappings (more specific terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub narrow_mappings: Vec<String>,

    /// Broad mappings (more general terms)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub broad_mappings: Vec<String>,

    /// Related mappings
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_mappings: Vec<String>,

    /// Examples of usage
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<Example>,

    /// Notes about this element
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    /// TODO items for this element
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub todos: Vec<String>,

    /// Comments about this element
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,

    /// Whether this element is deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,

    /// Rank/priority for ordering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rank: Option<i32>,
}

/// An example of how to use an element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Example {
    pub value: String,

    /// Optional description of the example
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Schema-level metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SchemaMetadata {
    /// Schema title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// Schema description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Schema version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// License information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// List of contributors
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<Contributor>,

    /// Creation timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_on: Option<DateTime<Utc>>,

    /// Last modified timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modified_on: Option<DateTime<Utc>>,

    /// Schema status (e.g., "draft", "release")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,

    /// Source information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,

    /// Generation details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_date: Option<DateTime<Utc>>,

    /// Keywords for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keywords: Vec<String>,

    /// Categories this schema belongs to
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub categories: Vec<String>,
}

/// Information about a contributor
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Contributor {
    /// Contributor name
    pub name: String,

    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,

    /// GitHub username
    #[serde(skip_serializing_if = "Option::is_none")]
    pub github: Option<String>,

    /// ORCID identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,

    /// Role in the project
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Trait for elements that have metadata
pub trait HasMetadata {
    /// Get the element metadata
    fn metadata(&self) -> &ElementMetadata;

    /// Get mutable element metadata
    fn metadata_mut(&mut self) -> &mut ElementMetadata;

    /// Add an alias
    fn add_alias(&mut self, alias: impl Into<String>) {
        self.metadata_mut().aliases.push(alias.into());
    }

    /// Add an example
    fn add_example(&mut self, value: impl Into<String>, description: Option<String>) {
        self.metadata_mut().examples.push(Example {
            value: value.into(),
            description,
        });
    }

    /// Mark as deprecated
    fn deprecate(&mut self, reason: impl Into<String>) {
        self.metadata_mut().deprecated = Some(reason.into());
    }

    /// Check if deprecated
    fn is_deprecated(&self) -> bool {
        self.metadata().deprecated.is_some()
    }
}

/// Merge two sets of element metadata
#[must_use]
pub fn merge_element_metadata(
    base: &ElementMetadata,
    override_metadata: &ElementMetadata,
) -> ElementMetadata {
    ElementMetadata {
        aliases: merge_vec(&base.aliases, &override_metadata.aliases),
        see_also: merge_vec(&base.see_also, &override_metadata.see_also),
        exact_mappings: merge_vec(&base.exact_mappings, &override_metadata.exact_mappings),
        close_mappings: merge_vec(&base.close_mappings, &override_metadata.close_mappings),
        narrow_mappings: merge_vec(&base.narrow_mappings, &override_metadata.narrow_mappings),
        broad_mappings: merge_vec(&base.broad_mappings, &override_metadata.broad_mappings),
        related_mappings: merge_vec(&base.related_mappings, &override_metadata.related_mappings),
        examples: merge_vec(&base.examples, &override_metadata.examples),
        notes: merge_vec(&base.notes, &override_metadata.notes),
        todos: merge_vec(&base.todos, &override_metadata.todos),
        comments: merge_vec(&base.comments, &override_metadata.comments),
        deprecated: override_metadata
            .deprecated
            .clone()
            .or_else(|| base.deprecated.clone()),
        replaced_by: override_metadata
            .replaced_by
            .clone()
            .or_else(|| base.replaced_by.clone()),
        rank: override_metadata.rank.or(base.rank),
    }
}

/// Helper to merge two vectors, removing duplicates
fn merge_vec<T: Clone + PartialEq>(base: &[T], override_vec: &[T]) -> Vec<T> {
    let mut result = base.to_vec();
    for item in override_vec {
        if !result.contains(item) {
            result.push(item.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_metadata() {
        let mut metadata = ElementMetadata::default();

        // Add some metadata
        metadata.aliases.push("alternate_name".to_string());
        metadata
            .see_also
            .push("https://example.org/related".to_string());
        metadata.examples.push(Example {
            value: "example@email.com".to_string(),
            description: Some("A valid email address".to_string()),
        });

        assert_eq!(metadata.aliases.len(), 1);
        assert_eq!(metadata.examples.len(), 1);
        assert!(metadata.deprecated.is_none());
    }

    #[test]
    fn test_merge_metadata() {
        let mut base = ElementMetadata::default();
        base.aliases.push("name1".to_string());
        base.notes.push("Base note".to_string());
        base.rank = Some(1);

        let mut override_meta = ElementMetadata::default();
        override_meta.aliases.push("name2".to_string());
        override_meta.notes.push("Override note".to_string());
        override_meta.rank = Some(2);
        override_meta.deprecated = Some("Use new_element instead".to_string());

        let merged = merge_element_metadata(&base, &override_meta);

        assert_eq!(merged.aliases.len(), 2);
        assert_eq!(merged.notes.len(), 2);
        assert_eq!(merged.rank, Some(2)); // Override wins
        assert_eq!(
            merged.deprecated,
            Some("Use new_element instead".to_string())
        );
    }

    #[test]
    fn test_contributor() -> crate::Result<()> {
        let contributor = Contributor {
            name: "Jane Doe".to_string(),
            email: Some("jane@example.com".to_string()),
            github: Some("janedoe".to_string()),
            orcid: Some("0000-0000-0000-0000".to_string()),
            role: Some("maintainer".to_string()),
        };

        let json =
            serde_json::to_string(&contributor).map_err(|e| anyhow::anyhow!("Error: {e}"))?;
        assert!(json.contains("Jane Doe"));
        assert!(json.contains("maintainer"));
        Ok(())
    }
}
