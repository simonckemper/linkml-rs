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

    // Tests for AggregatedStats
    #[test]
    fn test_aggregated_stats_creation() {
        let aggregated = AggregatedStats::new();
        assert_eq!(aggregated.document_count, 0);
        assert_eq!(aggregated.confidence, 0.0);
        assert!(aggregated.elements.is_empty());
        assert!(aggregated.source_documents.is_empty());
    }

    #[test]
    fn test_aggregated_stats_merge() {
        let mut aggregated = AggregatedStats::new();

        let mut stats1 = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats1.record_element("Person");
        stats1.record_attribute("Person", "age", "25".to_string());

        let mut stats2 = DocumentStats::new("doc2".to_string(), "xml".to_string());
        stats2.record_element("Person");
        stats2.record_attribute("Person", "age", "30".to_string());

        aggregated.merge(stats1);
        aggregated.merge(stats2);

        assert_eq!(aggregated.document_count, 2);
        assert_eq!(aggregated.source_documents.len(), 2);
        assert!(aggregated.elements.contains_key("Person"));

        let person_stats = &aggregated.elements["Person"];
        assert_eq!(person_stats.document_frequency, 2);
        assert_eq!(person_stats.total_occurrences, 2);
    }

    #[test]
    fn test_aggregated_stats_confidence_scaling() {
        let mut aggregated = AggregatedStats::new();

        // 1 document: 0.5 confidence
        aggregated.merge(DocumentStats::new("doc1".to_string(), "xml".to_string()));
        assert_eq!(aggregated.confidence, 0.5);

        // 2 documents: >0.5
        aggregated.merge(DocumentStats::new("doc2".to_string(), "xml".to_string()));
        assert!(aggregated.confidence > 0.5 && aggregated.confidence < 0.8);

        // 5 documents: 0.8
        for i in 3..=5 {
            aggregated.merge(DocumentStats::new(format!("doc{}", i), "xml".to_string()));
        }
        assert_eq!(aggregated.confidence, 0.8);

        // 10+ documents: 0.95
        for i in 6..=11 {
            aggregated.merge(DocumentStats::new(format!("doc{}", i), "xml".to_string()));
        }
        assert_eq!(aggregated.confidence, 0.95);
    }

    #[test]
    fn test_type_votes_simple_inference() {
        let mut votes = TypeVotes::new();
        votes.add_samples(&vec!["10".to_string(), "20".to_string(), "30".to_string()]);

        assert_eq!(votes.majority_type(), InferredType::Integer);
        assert_eq!(votes.confidence(), 1.0); // All samples agree
    }

    #[test]
    fn test_type_votes_mixed_types() {
        let mut votes = TypeVotes::new();
        votes.add_samples(&vec![
            "10".to_string(),
            "20".to_string(),
            "hello".to_string(),
        ]);

        // Integer has 2 votes, String has 1 vote
        assert_eq!(votes.majority_type(), InferredType::Integer);
        let confidence = votes.confidence();
        assert!(confidence > 0.6 && confidence < 0.7); // 2/3 ≈ 0.666
    }

    #[test]
    fn test_type_votes_boolean() {
        let mut votes = TypeVotes::new();
        votes.add_samples(&vec!["true".to_string(), "false".to_string(), "TRUE".to_string()]);

        assert_eq!(votes.majority_type(), InferredType::Boolean);
        assert_eq!(votes.confidence(), 1.0);
    }

    #[test]
    fn test_type_votes_uri() {
        let mut votes = TypeVotes::new();
        votes.add_samples(&vec![
            "https://example.com".to_string(),
            "http://test.org".to_string(),
        ]);

        assert_eq!(votes.majority_type(), InferredType::Uri);
        assert_eq!(votes.confidence(), 1.0);
    }

    #[test]
    fn test_aggregated_element_stats_cardinality_confidence() {
        let mut element = AggregatedElementStats::new("Item".to_string());

        // Create consistent occurrences
        let mut stats1 = ElementStats::new("Item".to_string());
        stats1.occurrence_count = 3;
        element.merge_element_stats(&stats1, 1);

        let mut stats2 = ElementStats::new("Item".to_string());
        stats2.occurrence_count = 3;
        element.merge_element_stats(&stats2, 2);

        // Same min/max should give 1.0 confidence
        assert_eq!(element.cardinality_confidence(), 1.0);
    }

    #[test]
    fn test_aggregated_element_stats_is_required() {
        let mut element = AggregatedElementStats::new("Item".to_string());

        element.document_frequency = 5;
        assert!(element.is_required(5)); // Appears in all 5 documents
        assert!(!element.is_required(6)); // Only appears in 5 of 6 documents
    }

    #[test]
    fn test_aggregated_element_stats_is_multivalued() {
        let mut element = AggregatedElementStats::new("Item".to_string());

        element.max_occurrences = 1;
        assert!(!element.is_multivalued());

        element.max_occurrences = 3;
        assert!(element.is_multivalued());
    }

    #[test]
    fn test_aggregated_child_stats() {
        let mut child = AggregatedChildStats::new("Address".to_string());

        let mut child_stats = ChildStats::new("Address".to_string());
        child_stats.occurrence_count = 2;
        child_stats.min_occurs = 1;
        child_stats.max_occurs = 2;
        child_stats.parent_instances_with_child = 5;
        child_stats.total_parent_instances = 5;

        child.merge_child_stats(&child_stats);

        assert_eq!(child.total_occurrences, 2);
        assert_eq!(child.parent_instances_with_child, 5);
        assert_eq!(child.total_parent_instances, 5);
        assert!(child.is_required());
        assert!(child.is_multivalued());
    }

    #[test]
    fn test_aggregated_child_stats_confidence() {
        let mut child = AggregatedChildStats::new("Address".to_string());
        child.parent_instances_with_child = 8;
        child.total_parent_instances = 10;

        let confidence = child.cardinality_confidence();
        assert_eq!(confidence, 0.8); // 8/10 = 0.8
    }

    #[test]
    fn test_type_refinement_across_documents() {
        let mut aggregated = AggregatedStats::new();

        // Document 1: Person with integer age
        let mut stats1 = DocumentStats::new("doc1".to_string(), "xml".to_string());
        stats1.record_element("Person");
        stats1.record_attribute("Person", "age", "25".to_string());

        // Document 2: Person with integer age
        let mut stats2 = DocumentStats::new("doc2".to_string(), "xml".to_string());
        stats2.record_element("Person");
        stats2.record_attribute("Person", "age", "30".to_string());

        // Document 3: Person with string age (outlier)
        let mut stats3 = DocumentStats::new("doc3".to_string(), "xml".to_string());
        stats3.record_element("Person");
        stats3.record_attribute("Person", "age", "N/A".to_string());

        aggregated.merge(stats1);
        aggregated.merge(stats2);
        aggregated.merge(stats3);

        let person_stats = &aggregated.elements["Person"];
        let age_votes = &person_stats.attribute_type_votes["age"];

        // Majority type should be Integer (2 votes vs 1 for String)
        assert_eq!(age_votes.majority_type(), InferredType::Integer);

        // Confidence should be 2/3 ≈ 0.666
        let confidence = age_votes.confidence();
        assert!(confidence > 0.6 && confidence < 0.7);
    }
}
