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

    /// Namespace prefixes encountered
    pub namespaces: HashMap<String, String>,
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
            namespaces: HashMap::new(),
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
    pub fn record_attribute(&mut self, element_name: &str, attr_name: &str, attr_value: String) {
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

    /// Record a namespace prefix and URI
    pub fn record_namespace(&mut self, prefix: String, uri: String) {
        self.namespaces.insert(prefix, uri);
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

    /// Namespace URI for this element
    pub namespace: Option<String>,

    /// Whether this element has mixed content (text and children)
    pub has_mixed_content: bool,
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
            namespace: None,
            has_mixed_content: false,
        }
    }

    /// Set the namespace for this element
    pub fn set_namespace(&mut self, namespace: String) {
        self.namespace = Some(namespace);
    }

    /// Mark this element as having mixed content
    pub fn mark_mixed_content(&mut self) {
        self.has_mixed_content = true;
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

    /// Minimum occurrences within a parent instance
    pub min_occurs: Option<usize>,

    /// Maximum occurrences within a parent instance
    pub max_occurs: Option<usize>,

    /// Number of parent instances that contain this child
    pub parent_instances_with_child: Option<usize>,

    /// Total number of parent instances
    pub total_parent_instances: Option<usize>,
}

impl ChildStats {
    /// Create a new ChildStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            occurrence_count: 0,
            min_occurs: None,
            max_occurs: None,
            parent_instances_with_child: None,
            total_parent_instances: None,
        }
    }

    /// Update occurrence statistics
    pub fn update_occurs(&mut self, occurs: usize, parent_instance_count: usize) {
        self.min_occurs = Some(self.min_occurs.map_or(occurs, |min| min.min(occurs)));
        self.max_occurs = Some(self.max_occurs.map_or(occurs, |max| max.max(occurs)));

        if occurs > 0 {
            self.parent_instances_with_child =
                Some(self.parent_instances_with_child.unwrap_or(0) + 1);
        }

        self.total_parent_instances = Some(parent_instance_count);
    }

    /// Check if this child is required
    pub fn is_required(&self) -> bool {
        if let (Some(with_child), Some(total)) = (
            self.parent_instances_with_child,
            self.total_parent_instances,
        ) {
            total > 0 && with_child == total
        } else {
            false
        }
    }

    /// Check if this child is multivalued
    pub fn is_multivalued(&self) -> bool {
        self.max_occurs.is_some_and(|max| max > 1)
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Aggregated statistics from multiple documents for schema inference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedStats {
    /// Number of documents analyzed
    pub document_count: usize,

    /// Confidence score for the aggregated schema (0.0 to 1.0)
    pub confidence: f32,

    /// Aggregated element statistics across all documents
    pub elements: HashMap<String, AggregatedElementStats>,

    /// Source document identifiers
    pub source_documents: Vec<String>,

    /// Format of the analyzed documents
    pub format: Option<String>,

    /// Schema metadata
    pub metadata: SchemaMetadata,
}

impl AggregatedStats {
    /// Create a new AggregatedStats instance
    pub fn new() -> Self {
        Self {
            document_count: 0,
            confidence: 0.0,
            elements: HashMap::new(),
            source_documents: Vec::new(),
            format: None,
            metadata: SchemaMetadata::default(),
        }
    }

    /// Merge statistics from a single document
    pub fn merge(&mut self, stats: DocumentStats) {
        self.document_count += 1;
        self.source_documents.push(stats.document_id.clone());

        if self.format.is_none() {
            self.format = Some(stats.format.clone());
        }

        // Merge element statistics
        for (element_name, element_stats) in stats.elements {
            self.elements
                .entry(element_name.clone())
                .or_insert_with(|| AggregatedElementStats::new(element_name))
                .merge_element_stats(&element_stats, self.document_count);
        }

        // Update confidence based on document count
        self.confidence = match self.document_count {
            1 => 0.5,
            2..=4 => 0.5 + (self.document_count as f32 - 1.0) * 0.1,
            5..=9 => 0.8,
            _ => 0.95,
        };
    }
}

impl Default for AggregatedStats {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated statistics for a single element across multiple documents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedElementStats {
    /// Element name
    pub name: String,

    /// Number of documents containing this element
    pub document_frequency: usize,

    /// Total occurrences across all documents
    pub total_occurrences: usize,

    /// Minimum occurrences in a single document
    pub min_occurrences: usize,

    /// Maximum occurrences in a single document
    pub max_occurrences: usize,

    /// Aggregated attribute statistics
    pub attributes: HashMap<String, AggregatedElementStats>,

    /// Aggregated child statistics
    pub children: HashMap<String, AggregatedChildStats>,

    /// Type votes from text samples
    pub text_type_votes: TypeVotes,

    /// Type votes for attributes
    pub attribute_type_votes: HashMap<String, TypeVotes>,

    /// Namespace URI if detected
    pub namespace: Option<String>,

    /// Whether this element has mixed content (text and child elements)
    pub has_mixed_content: bool,
}

impl AggregatedElementStats {
    /// Create a new AggregatedElementStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            document_frequency: 0,
            total_occurrences: 0,
            min_occurrences: usize::MAX,
            max_occurrences: 0,
            attributes: HashMap::new(),
            children: HashMap::new(),
            text_type_votes: TypeVotes::new(),
            attribute_type_votes: HashMap::new(),
            namespace: None,
            has_mixed_content: false,
        }
    }

    /// Merge statistics from a single element
    pub fn merge_element_stats(&mut self, stats: &ElementStats, _document_index: usize) {
        self.document_frequency += 1;
        self.total_occurrences += stats.occurrence_count;
        self.min_occurrences = self.min_occurrences.min(stats.occurrence_count);
        self.max_occurrences = self.max_occurrences.max(stats.occurrence_count);

        // Merge text samples for type inference
        if !stats.text_samples.is_empty() {
            self.text_type_votes.add_samples(&stats.text_samples);
        }

        // Merge attribute statistics
        for (attr_name, attr_stats) in &stats.attributes {
            self.attribute_type_votes
                .entry(attr_name.clone())
                .or_default()
                .add_samples(&attr_stats.value_samples);
        }

        // Merge child statistics
        for (child_name, child_stats) in &stats.children {
            self.children
                .entry(child_name.clone())
                .or_insert_with(|| AggregatedChildStats::new(child_name.clone()))
                .merge_child_stats(child_stats);
        }
    }

    /// Check if this element is required (appears in all documents)
    pub fn is_required(&self, total_documents: usize) -> bool {
        self.document_frequency == total_documents
    }

    /// Check if this element is multivalued (max occurrences > 1)
    pub fn is_multivalued(&self) -> bool {
        self.max_occurrences > 1
    }

    /// Calculate confidence in cardinality determination
    pub fn cardinality_confidence(&self) -> f32 {
        if self.min_occurrences == self.max_occurrences {
            1.0
        } else {
            let range = self.max_occurrences - self.min_occurrences;
            1.0 - (range as f32 / self.max_occurrences as f32).min(1.0)
        }
    }
}

/// Aggregated statistics for child elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedChildStats {
    /// Child element name
    pub name: String,

    /// Total occurrences across all parent instances
    pub total_occurrences: usize,

    /// Minimum occurrences within a parent
    pub min_occurrences: usize,

    /// Maximum occurrences within a parent
    pub max_occurrences: usize,

    /// Number of parent instances that contain this child
    pub parent_instances_with_child: usize,

    /// Total number of parent instances
    pub total_parent_instances: usize,
}

impl AggregatedChildStats {
    /// Create a new AggregatedChildStats instance
    pub fn new(name: String) -> Self {
        Self {
            name,
            total_occurrences: 0,
            min_occurrences: usize::MAX,
            max_occurrences: 0,
            parent_instances_with_child: 0,
            total_parent_instances: 0,
        }
    }

    /// Merge statistics from a single child
    pub fn merge_child_stats(&mut self, stats: &ChildStats) {
        self.total_occurrences += stats.occurrence_count;

        if let Some(min) = stats.min_occurs {
            self.min_occurrences = self.min_occurrences.min(min);
        }

        if let Some(max) = stats.max_occurs {
            self.max_occurrences = self.max_occurrences.max(max);
        }

        if let Some(with_child) = stats.parent_instances_with_child {
            self.parent_instances_with_child += with_child;
        }

        if let Some(total) = stats.total_parent_instances {
            self.total_parent_instances += total;
        }
    }

    /// Check if this child is required (appears in all parent instances)
    pub fn is_required(&self) -> bool {
        self.total_parent_instances > 0
            && self.parent_instances_with_child == self.total_parent_instances
    }

    /// Check if this child is multivalued (max occurrences > 1)
    pub fn is_multivalued(&self) -> bool {
        self.max_occurrences > 1
    }

    /// Calculate confidence in cardinality determination
    pub fn cardinality_confidence(&self) -> f32 {
        if self.total_parent_instances == 0 {
            0.0
        } else {
            self.parent_instances_with_child as f32 / self.total_parent_instances as f32
        }
    }
}

/// Type voting system for inferring types from multiple samples
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeVotes {
    /// Vote counts for each inferred type
    votes: HashMap<InferredType, usize>,

    /// Total number of samples
    total_samples: usize,
}

impl TypeVotes {
    /// Create a new TypeVotes instance
    pub fn new() -> Self {
        Self {
            votes: HashMap::new(),
            total_samples: 0,
        }
    }

    /// Add samples and vote for their inferred types
    pub fn add_samples(&mut self, samples: &[String]) {
        use crate::inference::type_inference::infer_type_from_value;

        for sample in samples {
            let inferred_traits_type = infer_type_from_value(sample);
            // Convert traits::InferredType to types::InferredType
            let inferred = match inferred_traits_type {
                crate::inference::traits::InferredType::String => InferredType::String,
                crate::inference::traits::InferredType::Integer => InferredType::Integer,
                crate::inference::traits::InferredType::Float => InferredType::Float,
                crate::inference::traits::InferredType::Boolean => InferredType::Boolean,
                crate::inference::traits::InferredType::DateTime => InferredType::DateTime,
                crate::inference::traits::InferredType::Date => InferredType::Date,
                crate::inference::traits::InferredType::Time => InferredType::Time,
                crate::inference::traits::InferredType::Uri => InferredType::Uri,
                crate::inference::traits::InferredType::Email => InferredType::Email,
                crate::inference::traits::InferredType::Unknown => InferredType::Unknown,
            };
            *self.votes.entry(inferred).or_insert(0) += 1;
            self.total_samples += 1;
        }
    }

    /// Get the majority type
    pub fn majority_type(&self) -> InferredType {
        self.votes
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(t, _)| t.clone())
            .unwrap_or(InferredType::Unknown)
    }

    /// Get confidence in the majority type (0.0 to 1.0)
    pub fn confidence(&self) -> f32 {
        if self.total_samples == 0 {
            return 0.0;
        }

        let majority = self.votes.values().max().copied().unwrap_or(0);
        majority as f32 / self.total_samples as f32
    }

    /// Check if there are any samples
    pub fn has_samples(&self) -> bool {
        self.total_samples > 0
    }
}

impl Default for TypeVotes {
    fn default() -> Self {
        Self::new()
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
        votes.add_samples(&vec![
            "true".to_string(),
            "false".to_string(),
            "TRUE".to_string(),
        ]);

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
        child_stats.min_occurs = Some(1);
        child_stats.max_occurs = Some(2);
        child_stats.parent_instances_with_child = Some(5);
        child_stats.total_parent_instances = Some(5);

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
