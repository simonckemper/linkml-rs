//! Core types for schema inference from structured data
//!
//! This module defines the fundamental types used throughout the schema
//! inference system for collecting statistics and metadata from data sources.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistics collected from analyzing a single document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStats {
    /// Document identifier
    pub document_id: String,

    /// Format of the analyzed document
    pub format: String,

    /// Element/field statistics
    pub elements: HashMap<String, ElementStats>,

    /// Schema metadata
    pub metadata: SchemaMetadata,

    /// Document-level statistics
    pub document_metrics: DocumentMetrics,
}

impl DocumentStats {
    /// Create a new DocumentStats instance
    pub fn new(document_id: String, format: String) -> Self {
        Self {
            document_id,
            format,
            elements: HashMap::new(),
            metadata: SchemaMetadata::default(),
            document_metrics: DocumentMetrics::default(),
        }
    }

    /// Record an element occurrence
    pub fn record_element(&mut self, name: &str) {
        self.elements
            .entry(name.to_string())
            .or_insert_with(|| ElementStats::new(name.to_string()))
            .occurrence_count += 1;
    }

    /// Add a text sample to an element
    pub fn add_text_sample(&mut self, element_name: &str, text: String) {
        if let Some(element) = self.elements.get_mut(element_name) {
            if element.text_samples.len() < 100 {
                // Limit samples
                element.text_samples.push(text);
            }
        }
    }

    /// Record an attribute for an element
    pub fn record_attribute(
        &mut self,
        element_name: &str,
        attr_name: &str,
        attr_value: String,
    ) {
        if let Some(element) = self.elements.get_mut(element_name) {
            element
                .attributes
                .entry(attr_name.to_string())
                .or_insert_with(|| AttributeStats::new(attr_name.to_string()))
                .record_value(attr_value);
        }
    }

    /// Record a child relationship
    pub fn record_child(&mut self, parent_name: &str, child_name: &str) {
        if let Some(element) = self.elements.get_mut(parent_name) {
            element
                .children
                .entry(child_name.to_string())
                .or_insert_with(|| ChildStats::new(child_name.to_string()))
                .occurrence_count += 1;
        }
    }
}

/// Statistics for a single element/field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElementStats {
    /// Element name
    pub name: String,

    /// Number of times this element appears
    pub occurrence_count: usize,

    /// Attributes of this element
    pub attributes: HashMap<String, AttributeStats>,

    /// Child elements
    pub children: HashMap<String, ChildStats>,

    /// Sample text content values
    pub text_samples: Vec<String>,

    /// Maximum nesting depth where this element appears
    pub max_depth: usize,
}

impl ElementStats {
    /// Create a new ElementStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            occurrence_count: 0,
            attributes: HashMap::new(),
            children: HashMap::new(),
            text_samples: Vec::new(),
            max_depth: 0,
        }
    }
}

/// Statistics for an attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeStats {
    /// Attribute name
    pub name: String,

    /// Number of times this attribute appears
    pub occurrence_count: usize,

    /// Sample values
    pub value_samples: Vec<String>,

    /// Unique value count (approximate)
    pub unique_values: usize,
}

impl AttributeStats {
    /// Create a new AttributeStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            occurrence_count: 0,
            value_samples: Vec::new(),
            unique_values: 0,
        }
    }

    /// Record a value occurrence
    pub fn record_value(&mut self, value: String) {
        self.occurrence_count += 1;
        if self.value_samples.len() < 100 && !self.value_samples.contains(&value) {
            self.value_samples.push(value);
            self.unique_values = self.value_samples.len();
        }
    }
}

/// Statistics for child elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildStats {
    /// Child element name
    pub name: String,

    /// Number of times this child appears
    pub occurrence_count: usize,
}

impl ChildStats {
    /// Create a new ChildStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            occurrence_count: 0,
        }
    }
}

/// Schema metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema identifier
    pub schema_id: Option<String>,

    /// Schema name
    pub schema_name: Option<String>,

    /// Schema version
    pub version: Option<String>,

    /// Generation timestamp
    pub generated_at: Option<DateTime<Utc>>,

    /// Generator information
    pub generator: Option<String>,

    /// Source file information
    pub source_files: Vec<String>,
}

/// Document-level metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DocumentMetrics {
    /// Total number of elements in the document
    pub total_elements: usize,

    /// Total number of attributes in the document
    pub total_attributes: usize,

    /// Maximum nesting depth
    pub max_nesting_depth: usize,

    /// Number of unique element names
    pub unique_element_names: usize,

    /// Document size in bytes
    pub document_size_bytes: usize,
}

/// Inferred data type from sample values
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InferredType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Floating point type
    Float,
    /// Boolean type
    Boolean,
    /// Date and time type
    DateTime,
    /// Date only type
    Date,
    /// Time only type
    Time,
    /// URI/URL type
    Uri,
    /// Email address type
    Email,
    /// Unknown or mixed type
    Unknown,
}

impl InferredType {
    /// Convert to LinkML type string
    pub fn to_linkml_type(&self) -> &str {
        match self {
            InferredType::String => "string",
            InferredType::Integer => "integer",
            InferredType::Float => "float",
            InferredType::Boolean => "boolean",
            InferredType::DateTime => "datetime",
            InferredType::Date => "date",
            InferredType::Time => "time",
            InferredType::Uri => "uri",
            InferredType::Email => "string",
            InferredType::Unknown => "string",
        }
    }
}

/// Configuration for schema inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    /// Minimum samples required for type inference
    pub min_samples_for_type_inference: usize,

    /// Confidence threshold for type inference (0.0 to 1.0)
    pub type_inference_confidence_threshold: f32,

    /// Whether to generate descriptions from field names
    pub generate_descriptions: bool,

    /// Whether to use heuristic field naming improvements
    pub use_heuristic_naming: bool,

    /// Maximum depth for nested structure analysis
    pub max_nesting_depth: usize,

    /// Number of documents to sample for large batches
    pub sample_size: Option<usize>,
}

impl Default for InferenceConfig {
    fn default() -> Self {
        Self {
            min_samples_for_type_inference: 5,
            type_inference_confidence_threshold: 0.8,
            generate_descriptions: true,
            use_heuristic_naming: true,
            max_nesting_depth: 10,
            sample_size: Some(100),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_stats_creation() {
        let stats = DocumentStats::new("doc1".to_string(), "xml".to_string());
        assert_eq!(stats.document_id, "doc1");
        assert_eq!(stats.format, "xml");
        assert!(stats.elements.is_empty());
    }

    #[test]
    fn test_record_element() {
        let mut stats = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats.record_element("Person");
        stats.record_element("Person");

        assert_eq!(stats.elements.len(), 1);
        assert_eq!(stats.elements.get("Person").unwrap().occurrence_count, 2);
    }

    #[test]
    fn test_record_attribute() {
        let mut stats = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats.record_element("Person");
        stats.record_attribute("Person", "age", "25".to_string());
        stats.record_attribute("Person", "age", "30".to_string());

        let person = stats.elements.get("Person").unwrap();
        let age_attr = person.attributes.get("age").unwrap();
        assert_eq!(age_attr.occurrence_count, 2);
        assert_eq!(age_attr.value_samples.len(), 2);
    }

    #[test]
    fn test_record_child() {
        let mut stats = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats.record_element("Person");
        stats.record_element("Address");
        stats.record_child("Person", "Address");

        let person = stats.elements.get("Person").unwrap();
        assert_eq!(person.children.len(), 1);
        assert_eq!(person.children.get("Address").unwrap().occurrence_count, 1);
    }

    #[test]
    fn test_add_text_sample() {
        let mut stats = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats.record_element("Name");
        stats.add_text_sample("Name", "John Doe".to_string());
        stats.add_text_sample("Name", "Jane Smith".to_string());

        let name = stats.elements.get("Name").unwrap();
        assert_eq!(name.text_samples.len(), 2);
        assert!(name.text_samples.contains(&"John Doe".to_string()));
    }

    #[test]
    fn test_inferred_type_to_linkml() {
        assert_eq!(InferredType::String.to_linkml_type(), "string");
        assert_eq!(InferredType::Integer.to_linkml_type(), "integer");
        assert_eq!(InferredType::Float.to_linkml_type(), "float");
        assert_eq!(InferredType::Boolean.to_linkml_type(), "boolean");
        assert_eq!(InferredType::DateTime.to_linkml_type(), "datetime");
        assert_eq!(InferredType::Uri.to_linkml_type(), "uri");
    }

    #[test]
    fn test_default_inference_config() {
        let config = InferenceConfig::default();
        assert_eq!(config.min_samples_for_type_inference, 5);
        assert_eq!(config.type_inference_confidence_threshold, 0.8);
        assert!(config.generate_descriptions);
        assert_eq!(config.max_nesting_depth, 10);
    }
}
