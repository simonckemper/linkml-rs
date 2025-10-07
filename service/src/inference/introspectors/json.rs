//! JSON introspector for schema inference
//!
//! This module analyzes JSON documents by traversing their structure and
//! collecting statistics about objects, arrays, and primitive values for
//! LinkML schema generation.

use crate::inference::builder::SchemaBuilder;
use crate::inference::traits::{DataIntrospector, InferenceError, InferenceResult, TypeInferencer};
use crate::inference::type_inference::create_type_inferencer;
use crate::inference::types::{DocumentStats, SchemaMetadata};
use async_trait::async_trait;
use linkml_core::types::SchemaDefinition;
use logger_core::{LoggerError, LoggerService};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// JSON introspector implementation
///
/// Analyzes JSON documents by recursively traversing their structure and
/// collecting statistics about objects, arrays, and primitive values.
pub struct JsonIntrospector {
    /// Logger service for operation tracking
    logger: Arc<dyn LoggerService<Error = LoggerError>>,

    /// Timestamp service for metadata
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,

    /// Type inferencer for detecting types from samples
    type_inferencer: Arc<dyn TypeInferencer>,
}

impl JsonIntrospector {
    /// Create a new JSON introspector
    ///
    /// # Arguments
    /// * `logger` - Logger service instance
    /// * `timestamp` - Timestamp service instance
    pub fn new(
        logger: Arc<dyn LoggerService<Error = LoggerError>>,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            logger,
            timestamp,
            type_inferencer: create_type_inferencer(),
        }
    }

    /// Traverse JSON value recursively and collect statistics
    ///
    /// # Arguments
    /// * `key` - Current field name
    /// * `value` - JSON value to analyze
    /// * `stats` - Statistics accumulator
    /// * `current_depth` - Current nesting depth
    /// * `max_depth` - Maximum depth encountered
    /// * `parent_key` - Parent object name (if any)
    fn traverse_value(
        &self,
        key: &str,
        value: &JsonValue,
        stats: &mut DocumentStats,
        current_depth: usize,
        max_depth: &mut usize,
        parent_key: Option<&str>,
    ) {
        // Update max depth tracking
        if current_depth > *max_depth {
            *max_depth = current_depth;
        }

        match value {
            JsonValue::Object(map) => {
                // Record this object as an element
                stats.record_element(key);

                // Update depth for this element
                if let Some(element_stats) = stats.elements.get_mut(key) {
                    if current_depth > element_stats.max_depth {
                        element_stats.max_depth = current_depth;
                    }
                }

                // If we have a parent, record the parent-child relationship
                if let Some(parent) = parent_key {
                    stats.record_child(parent, key);
                }

                // Traverse all properties of this object
                for (prop_key, prop_value) in map {
                    match prop_value {
                        JsonValue::Object(_) => {
                            // Nested object - recurse with this object as parent
                            self.traverse_value(
                                prop_key,
                                prop_value,
                                stats,
                                current_depth + 1,
                                max_depth,
                                Some(key),
                            );
                        }
                        JsonValue::Array(_) => {
                            // Array property - analyze it
                            self.analyze_array(
                                key,
                                prop_key,
                                prop_value,
                                stats,
                                current_depth + 1,
                                max_depth,
                            );
                        }
                        _ => {
                            // Primitive value - record as attribute
                            let value_str = self.json_value_to_string(prop_value);
                            stats.record_attribute(key, prop_key, value_str);
                        }
                    }
                }
            }
            JsonValue::Array(arr) => {
                // Arrays at root level - analyze array contents
                self.analyze_array_elements(key, arr, stats, current_depth, max_depth, parent_key);
            }
            _ => {
                // Primitive value at top level
                if let Some(parent) = parent_key {
                    let value_str = self.json_value_to_string(value);
                    stats.record_attribute(parent, key, value_str);
                }
            }
        }
    }

    /// Analyze an array property
    ///
    /// Detects whether the array contains primitives or objects, and whether
    /// all elements are the same type.
    fn analyze_array(
        &self,
        parent_key: &str,
        array_key: &str,
        array_value: &JsonValue,
        stats: &mut DocumentStats,
        current_depth: usize,
        max_depth: &mut usize,
    ) {
        if let JsonValue::Array(arr) = array_value {
            self.analyze_array_elements(
                array_key,
                arr,
                stats,
                current_depth,
                max_depth,
                Some(parent_key),
            );
        }
    }

    /// Analyze array elements to determine their type and structure
    fn analyze_array_elements(
        &self,
        array_key: &str,
        arr: &[JsonValue],
        stats: &mut DocumentStats,
        current_depth: usize,
        max_depth: &mut usize,
        parent_key: Option<&str>,
    ) {
        if arr.is_empty() {
            // Record empty array as a multivalued field with unknown type
            if let Some(parent) = parent_key {
                stats.record_child(parent, array_key);
            }
            return;
        }

        // Determine array element type
        let mut element_types: HashMap<String, usize> = HashMap::new();
        let mut all_objects = true;
        let mut all_primitives = true;

        for item in arr {
            let type_name = match item {
                JsonValue::Null => "null",
                JsonValue::Bool(_) => "boolean",
                JsonValue::Number(_) => "number",
                JsonValue::String(_) => "string",
                JsonValue::Array(_) => {
                    all_primitives = false;
                    "array"
                }
                JsonValue::Object(_) => {
                    all_primitives = false;
                    "object"
                }
            };

            if !matches!(item, JsonValue::Object(_)) {
                all_objects = false;
            }

            *element_types.entry(type_name.to_string()).or_insert(0) += 1;
        }

        if all_objects {
            // Array of objects - treat each object as an instance of a class
            // Use singular form of array key as the element class name
            let element_class_name = singularize(array_key);

            for item in arr {
                if let JsonValue::Object(_) = item {
                    self.traverse_value(
                        &element_class_name,
                        item,
                        stats,
                        current_depth,
                        max_depth,
                        parent_key,
                    );
                }
            }

            // Record the array relationship
            if let Some(parent) = parent_key {
                stats.record_child(parent, &element_class_name);

                // Mark as multivalued by recording multiple occurrences
                if arr.len() > 1 {
                    for _ in 1..arr.len() {
                        stats.record_child(parent, &element_class_name);
                    }
                }
            }
        } else if all_primitives {
            // Array of primitives - collect samples for type inference
            let samples: Vec<String> = arr.iter().map(|v| self.json_value_to_string(v)).collect();

            if let Some(parent) = parent_key {
                // Record as a multivalued attribute
                for sample in samples {
                    stats.record_attribute(parent, array_key, sample);
                }
            }
        } else {
            // Mixed array - record as a field with mixed type
            if let Some(parent) = parent_key {
                stats.record_child(parent, array_key);
            }
        }
    }

    /// Convert JSON value to string for type inference
    fn json_value_to_string(&self, value: &JsonValue) -> String {
        match value {
            JsonValue::Null => String::new(),
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Number(n) => n.to_string(),
            JsonValue::String(s) => s.clone(),
            JsonValue::Array(_) => "[]".to_string(),
            JsonValue::Object(_) => "{}".to_string(),
        }
    }
}

/// Simple singularization function for array element names
///
/// This is a basic heuristic that handles common cases. For production use,
/// consider using a full inflection library.
fn singularize(plural: &str) -> String {
    if plural.ends_with("ies") {
        format!("{}y", &plural[..plural.len() - 3])
    } else if plural.ends_with("sses") || plural.ends_with("shes") || plural.ends_with("ches") {
        plural[..plural.len() - 2].to_string()
    } else if plural.ends_with('s') && plural.len() > 1 {
        plural[..plural.len() - 1].to_string()
    } else {
        format!("{}_item", plural)
    }
}

#[async_trait]
impl DataIntrospector for JsonIntrospector {
    async fn analyze_file(&self, path: &Path) -> InferenceResult<DocumentStats> {
        self.logger
            .log_info(&format!("Starting JSON file analysis: {:?}", path))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Read file to bytes
        let bytes = tokio::fs::read(path).await.map_err(InferenceError::Io)?;

        // Analyze bytes
        self.analyze_bytes(&bytes).await
    }

    async fn analyze_bytes(&self, data: &[u8]) -> InferenceResult<DocumentStats> {
        let doc_id = format!("json_doc_{}", uuid::Uuid::new_v4());
        let format = "json".to_string();

        self.logger
            .log_info(&format!("Analyzing JSON bytes: {} bytes", data.len()))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Parse JSON
        let json_value: JsonValue = serde_json::from_slice(data)
            .map_err(|e| InferenceError::ParseServiceError(format!("JSON parsing error: {}", e)))?;

        let mut stats = DocumentStats::new(doc_id, format);
        let mut max_depth = 0;

        // Start traversal from root
        match &json_value {
            JsonValue::Object(_) => {
                self.traverse_value("root", &json_value, &mut stats, 1, &mut max_depth, None);
            }
            JsonValue::Array(arr) => {
                // Root-level array
                self.analyze_array_elements("items", arr, &mut stats, 1, &mut max_depth, None);
            }
            _ => {
                return Err(InferenceError::InvalidDataStructure(
                    "JSON must be an object or array at root level".to_string(),
                ));
            }
        }

        // Update metrics
        stats.document_metrics.max_nesting_depth = max_depth;
        stats.document_metrics.unique_element_names = stats.elements.len();
        stats.document_metrics.total_elements =
            stats.elements.values().map(|e| e.occurrence_count).sum();
        stats.document_metrics.total_attributes = stats
            .elements
            .values()
            .flat_map(|e| e.attributes.values())
            .map(|a| a.occurrence_count)
            .sum();
        stats.document_metrics.document_size_bytes = data.len();

        // Set metadata
        let now =
            self.timestamp.now_utc().await.map_err(|e| {
                InferenceError::ServiceError(format!("Failed to get timestamp: {}", e))
            })?;

        stats.metadata = SchemaMetadata {
            schema_id: Some("json_schema".to_string()),
            schema_name: Some("JSON Schema".to_string()),
            version: Some("1.0.0".to_string()),
            generated_at: Some(now),
            generator: Some("rootreal-schema-inference/1.0".to_string()),
            source_files: vec![],
        };

        self.logger
            .log_info(&format!(
                "JSON analysis complete: {} elements, {} unique element types, max depth: {}",
                stats.document_metrics.total_elements,
                stats.document_metrics.unique_element_names,
                max_depth
            ))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(stats)
    }

    fn format_name(&self) -> &'static str {
        "json"
    }

    async fn generate_schema(
        &self,
        stats: &DocumentStats,
        schema_id: &str,
    ) -> InferenceResult<SchemaDefinition> {
        self.logger
            .log_info(&format!("Generating LinkML schema: {}", schema_id))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        let schema_name = stats
            .metadata
            .schema_name
            .clone()
            .unwrap_or_else(|| format!("{} Schema", schema_id));

        let mut builder = SchemaBuilder::new(schema_id, &schema_name)
            .with_timestamp_service(Arc::clone(&self.timestamp));

        builder = builder
            .with_description(format!(
                "Auto-generated schema from JSON introspection ({})",
                stats.format
            ))
            .with_version("1.0.0")
            .with_default_range("string");

        // Create classes for each element (object type)
        for (element_name, element_stats) in &stats.elements {
            let mut class_builder = builder.add_class(element_name);

            class_builder = class_builder.with_description(format!(
                "JSON object '{}' appears {} times, max depth: {}",
                element_name, element_stats.occurrence_count, element_stats.max_depth
            ));

            // Add slots for attributes (primitive properties)
            for (attr_name, attr_stats) in &element_stats.attributes {
                let inferred_type = self
                    .type_inferencer
                    .infer_from_samples(&attr_stats.value_samples);
                let required = attr_stats.occurrence_count == element_stats.occurrence_count;
                let multivalued = attr_stats.occurrence_count > element_stats.occurrence_count;

                class_builder = class_builder.add_slot_with_type(
                    attr_name,
                    &inferred_type,
                    required,
                    multivalued,
                );
            }

            // Add slots for child objects and arrays
            for (child_name, child_stats) in &element_stats.children {
                let required = child_stats.occurrence_count >= element_stats.occurrence_count;
                let multivalued = child_stats.occurrence_count > element_stats.occurrence_count;

                class_builder =
                    class_builder.add_slot_with_type(child_name, child_name, required, multivalued);
            }

            builder = class_builder.finish();
        }

        let schema = builder.build();

        self.logger
            .log_info(&format!(
                "Schema generation complete: {} classes, {} slots",
                schema.classes.len(),
                schema.slots.len()
            ))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use timestamp_service::create_timestamp_service;

    fn create_test_services() -> (
        Arc<dyn LoggerService<Error = LoggerError>>,
        Arc<dyn TimestampService<Error = TimestampError>>,
    ) {
        let logger =
            create_logger_service().unwrap_or_else(|e| panic!("Failed to create logger: {}", e));
        let timestamp = wire_timestamp().into_inner();
        (logger, timestamp)
    }

    #[tokio::test]
    async fn test_json_introspector_format_name() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);
        assert_eq!(introspector.format_name(), "json");
    }

    #[tokio::test]
    async fn test_analyze_simple_json_object() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "name": "John Doe",
            "age": 30,
            "email": "john@example.com"
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert_eq!(stats.elements.len(), 1); // root object
        assert!(stats.elements.contains_key("root"));

        let root = stats.elements.get("root").unwrap();
        assert_eq!(root.attributes.len(), 3); // name, age, email
        assert!(root.attributes.contains_key("name"));
        assert!(root.attributes.contains_key("age"));
        assert!(root.attributes.contains_key("email"));
    }

    #[tokio::test]
    async fn test_analyze_nested_json() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "person": {
                "name": "John Doe",
                "age": 30,
                "address": {
                    "street": "123 Main St",
                    "city": "NYC"
                }
            }
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert!(stats.elements.contains_key("person"));
        assert!(stats.elements.contains_key("address"));
        assert!(stats.document_metrics.max_nesting_depth >= 3);

        let person = stats.elements.get("person").unwrap();
        assert!(person.children.contains_key("address"));
    }

    #[tokio::test]
    async fn test_analyze_json_with_array_of_objects() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "people": [
                {
                    "name": "John Doe",
                    "age": 30
                },
                {
                    "name": "Jane Smith",
                    "age": 25
                }
            ]
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        // Should have root and person (singular of people)
        assert!(stats.elements.contains_key("root"));
        assert!(stats.elements.contains_key("person"));

        let person = stats.elements.get("person").unwrap();
        assert_eq!(person.occurrence_count, 2);
        assert!(person.attributes.contains_key("name"));
        assert!(person.attributes.contains_key("age"));

        let root = stats.elements.get("root").unwrap();
        assert!(root.children.contains_key("person"));
    }

    #[tokio::test]
    async fn test_analyze_json_with_array_of_primitives() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "tags": ["rust", "json", "schema"]
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        let root = stats.elements.get("root").unwrap();
        assert!(root.attributes.contains_key("tags"));

        let tags = root.attributes.get("tags").unwrap();
        assert_eq!(tags.occurrence_count, 3); // Three array elements
    }

    #[tokio::test]
    async fn test_analyze_deeply_nested_json() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "level1": {
                "level2": {
                    "level3": {
                        "level4": {
                            "value": "deep"
                        }
                    }
                }
            }
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert!(stats.document_metrics.max_nesting_depth >= 5);
        assert!(stats.elements.contains_key("level4"));
    }

    #[tokio::test]
    async fn test_analyze_json_with_null_values() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "name": "John",
            "middle_name": null,
            "age": 30
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        let root = stats.elements.get("root").unwrap();
        assert!(root.attributes.contains_key("name"));
        assert!(root.attributes.contains_key("middle_name"));
        assert!(root.attributes.contains_key("age"));
    }

    #[tokio::test]
    async fn test_analyze_json_with_boolean_values() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "active": true,
            "verified": false
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        let root = stats.elements.get("root").unwrap();
        assert!(root.attributes.contains_key("active"));
        assert!(root.attributes.contains_key("verified"));
    }

    #[tokio::test]
    async fn test_analyze_json_with_number_values() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "count": 42,
            "price": 19.99,
            "negative": -5
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        let root = stats.elements.get("root").unwrap();
        assert!(root.attributes.contains_key("count"));
        assert!(root.attributes.contains_key("price"));
        assert!(root.attributes.contains_key("negative"));
    }

    #[tokio::test]
    async fn test_analyze_root_level_array() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"[
            {"name": "John", "age": 30},
            {"name": "Jane", "age": 25}
        ]"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert!(stats.elements.contains_key("item"));
        let item = stats.elements.get("item").unwrap();
        assert_eq!(item.occurrence_count, 2);
    }

    #[tokio::test]
    async fn test_analyze_empty_object() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{}"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert!(stats.elements.contains_key("root"));
        let root = stats.elements.get("root").unwrap();
        assert_eq!(root.attributes.len(), 0);
    }

    #[tokio::test]
    async fn test_generate_schema_from_json() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "person": {
                "name": "John Doe",
                "age": 30
            }
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();
        let schema = introspector
            .generate_schema(&stats, "test_schema")
            .await
            .unwrap();

        assert_eq!(schema.id, "test_schema");
        assert!(schema.classes.contains_key("person"));

        let person_class = schema.classes.get("person").unwrap();
        assert!(person_class.slots.contains(&"name".to_string()));
        assert!(person_class.slots.contains(&"age".to_string()));
    }

    #[tokio::test]
    async fn test_singularize() {
        assert_eq!(singularize("people"), "person");
        assert_eq!(singularize("entries"), "entri");
        assert_eq!(singularize("addresses"), "addresse");
        assert_eq!(singularize("boxes"), "boxe");
        assert_eq!(singularize("items"), "item");
        assert_eq!(singularize("data"), "data_item");
    }

    #[tokio::test]
    async fn test_mixed_type_array() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "mixed": [1, "string", true, {"nested": "object"}]
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        let root = stats.elements.get("root").unwrap();
        assert!(root.children.contains_key("mixed"));
    }

    #[tokio::test]
    async fn test_json_with_multiple_objects_same_level() {
        let (logger, timestamp) = create_test_services();
        let introspector = JsonIntrospector::new(logger, timestamp);

        let json = r#"{
            "user": {
                "name": "John",
                "email": "john@example.com"
            },
            "settings": {
                "theme": "dark",
                "notifications": true
            }
        }"#;

        let stats = introspector.analyze_bytes(json.as_bytes()).await.unwrap();

        assert!(stats.elements.contains_key("user"));
        assert!(stats.elements.contains_key("settings"));

        let root = stats.elements.get("root").unwrap();
        assert!(root.children.contains_key("user"));
        assert!(root.children.contains_key("settings"));
    }
}
