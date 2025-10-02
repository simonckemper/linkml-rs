//! XML introspector for schema inference
//!
//! This module analyzes XML documents using the Parse Service and collects
//! structure statistics for LinkML schema generation.

use crate::inference::builder::SchemaBuilder;
use crate::inference::traits::{DataIntrospector, InferenceError, InferenceResult, TypeInferencer};
use crate::inference::type_inference::create_type_inferencer;
use crate::inference::types::{DocumentStats, SchemaMetadata};
use async_trait::async_trait;
use linkml_core::types::SchemaDefinition;
use logger_core::{LoggerError, LoggerService};
use std::path::Path;
use std::sync::Arc;
use timestamp_core::{TimestampError, TimestampService};

/// XML introspector implementation
///
/// Analyzes XML documents by traversing their element tree and collecting
/// statistics about elements, attributes, and text content.
pub struct XmlIntrospector {
    /// Logger service for operation tracking
    logger: Arc<dyn LoggerService<Error = LoggerError>>,

    /// Timestamp service for metadata
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,

    /// Type inferencer for detecting types from samples
    type_inferencer: Arc<dyn TypeInferencer>,
}

impl XmlIntrospector {
    /// Create a new XML introspector
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

    /// Extract namespace prefix and local name from a qualified name
    ///
    /// # Arguments
    /// * `qname` - Qualified name (e.g., "ns:Element" or "Element")
    ///
    /// # Returns
    /// * `(Option<String>, String)` - (prefix, local_name)
    fn parse_qname(qname: &str) -> (Option<String>, String) {
        if let Some(colon_pos) = qname.find(':') {
            let prefix = qname[..colon_pos].to_string();
            let local_name = qname[colon_pos + 1..].to_string();
            (Some(prefix), local_name)
        } else {
            (None, qname.to_string())
        }
    }

    /// Detect format-specific patterns in XML structure
    ///
    /// # Arguments
    /// * `root_element` - Root element name
    /// * `namespaces` - Document namespaces
    ///
    /// # Returns
    /// * `Option<String>` - Detected format name
    fn detect_xml_format(
        root_element: &str,
        namespaces: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        // PAGE-XML detection
        if root_element == "PcGts"
            || namespaces
                .values()
                .any(|uri| uri.contains("primaresearch.org/PAGE"))
        {
            return Some("PAGE-XML".to_string());
        }

        // EAD (Encoded Archival Description) detection
        if root_element == "ead" || namespaces.values().any(|uri| uri.contains("loc.gov/ead")) {
            return Some("EAD".to_string());
        }

        // Dublin Core detection
        if root_element == "dc"
            || root_element == "metadata"
                && namespaces.values().any(|uri| uri.contains("purl.org/dc"))
        {
            return Some("Dublin Core".to_string());
        }

        None
    }

    /// Check if an element has mixed content (both text and child elements)
    ///
    /// # Arguments
    /// * `has_text` - Whether element has text content
    /// * `has_children` - Whether element has child elements
    ///
    /// # Returns
    /// * `bool` - True if mixed content detected
    fn is_mixed_content(has_text: bool, has_children: bool) -> bool {
        has_text && has_children
    }

    // NOTE: These methods are commented out for Phase 1.
    // They will be implemented in Phase 2 for full Parse Service integration.
    /*
    /// Analyze XML structure from ParsedDocument
    ///
    /// # Arguments
    /// * `document` - Parsed document from Parse Service
    ///
    /// # Returns
    /// Document statistics collected from analysis
    async fn analyze_parsed_document(
        &self,
        document: &ParsedDocument,
    ) -> InferenceResult<DocumentStats> {
        let doc_id = document.id.clone();
        let format = document.format.to_string();

        self.logger
            .log_info(&format!("Analyzing XML document: {}", doc_id))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        let mut stats = DocumentStats::new(doc_id, format);
        let mut current_depth = 0;
        let mut max_depth = 0;

        // Analyze document content
        match &document.content {
            DocumentContent::Structured(structured) => {
                self.analyze_structured_content(
                    structured,
                    &mut stats,
                    &mut current_depth,
                    &mut max_depth,
                )
                .await?;
            }
            DocumentContent::Text(_text) => {
                // Handle plain text XML (fallback)
                self.logger
                    .log_warn("Document content is plain text, not structured")
                    .await
                    .map_err(|e| InferenceError::LoggerError(e.to_string()))?;
            }
            DocumentContent::Binary(_) => {
                return Err(InferenceError::InvalidDataStructure(
                    "Binary content not supported for XML introspection".to_string(),
                ));
            }
        }

        // Update document metrics
        stats.document_metrics.max_nesting_depth = max_depth;
        stats.document_metrics.unique_element_names = stats.elements.len();
        stats.document_metrics.total_elements = stats
            .elements
            .values()
            .map(|e| e.occurrence_count)
            .sum();
        stats.document_metrics.total_attributes = stats
            .elements
            .values()
            .flat_map(|e| e.attributes.values())
            .map(|a| a.occurrence_count)
            .sum();

        // Set metadata
        let now = self.timestamp.now_utc().await
            .map_err(|e| InferenceError::ServiceError(format!("Failed to get timestamp: {}", e)))?;

        stats.metadata = SchemaMetadata {
            schema_id: Some(format!("{}_schema", document.id)),
            schema_name: Some(format!("{} Schema", document.metadata.title.clone().unwrap_or_else(|| "XML Document".to_string()))),
            version: Some("1.0.0".to_string()),
            generated_at: Some(now),
            generator: Some("rootreal-schema-inference/1.0".to_string()),
            source_files: vec![],
        };

        self.logger
            .log_info(&format!(
                "Analysis complete: {} elements, {} attributes, max depth {}",
                stats.document_metrics.unique_element_names,
                stats.document_metrics.total_attributes,
                max_depth
            ))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(stats)
    }

    /// Analyze structured content recursively
    async fn analyze_structured_content(
        &self,
        content: &StructuredContent,
        stats: &mut DocumentStats,
        current_depth: &mut usize,
        max_depth: &mut usize,
    ) -> InferenceResult<()> {
        *current_depth += 1;
        if *current_depth > *max_depth {
            *max_depth = *current_depth;
        }

        // Analyze each element
        for element in &content.elements {
            self.analyze_element(element, stats, current_depth, max_depth)
                .await?;
        }

        *current_depth -= 1;
        Ok(())
    }

    /// Analyze a single content element
    async fn analyze_element(
        &self,
        element: &ContentElement,
        stats: &mut DocumentStats,
        current_depth: &mut usize,
        max_depth: &mut usize,
    ) -> InferenceResult<()> {
        let element_name = &element.name;

        // Record element occurrence
        stats.record_element(element_name);

        // Record attributes
        for (attr_name, attr_value) in &element.attributes {
            stats.record_attribute(element_name, attr_name, attr_value.clone());
        }

        // Record text content
        if !element.text_content.is_empty() {
            stats.add_text_sample(element_name, element.text_content.clone());
        }

        // Update max depth for this element
        if let Some(element_stats) = stats.elements.get_mut(element_name) {
            if *current_depth > element_stats.max_depth {
                element_stats.max_depth = *current_depth;
            }
        }

        // Recursively analyze children
        for child in &element.children {
            // Record parent-child relationship
            stats.record_child(element_name, &child.name);

            // Analyze child element
            *current_depth += 1;
            if *current_depth > *max_depth {
                *max_depth = *current_depth;
            }

            self.analyze_element(child, stats, current_depth, max_depth)
                .await?;

            *current_depth -= 1;
        }

        Ok(())
    }
    */
}

#[async_trait]
impl DataIntrospector for XmlIntrospector {
    async fn analyze_file(&self, path: &Path) -> InferenceResult<DocumentStats> {
        self.logger
            .log_info(&format!("Starting XML file analysis: {:?}", path))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Read file to bytes
        let bytes = tokio::fs::read(path).await.map_err(InferenceError::Io)?;

        // Analyze bytes
        self.analyze_bytes(&bytes).await
    }

    async fn analyze_bytes(&self, data: &[u8]) -> InferenceResult<DocumentStats> {
        // For now, create a simple XML representation
        // In production, this would use the Parse Service
        let doc_id = format!("xml_doc_{}", uuid::Uuid::new_v4());
        let format = "xml".to_string();

        self.logger
            .log_info(&format!("Analyzing XML bytes: {} bytes", data.len()))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        // Parse XML using quick-xml for basic introspection
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use std::collections::HashMap;

        let mut reader = Reader::from_reader(data);
        reader.trim_text(true);

        let mut stats = DocumentStats::new(doc_id, format);
        let mut element_stack: Vec<String> = Vec::new();
        let mut current_depth = 0;
        let mut max_depth = 0;
        let mut root_element: Option<String> = None;

        // Track element text and children for mixed content detection
        let mut element_has_text: HashMap<String, bool> = HashMap::new();
        let mut element_has_children: HashMap<String, bool> = HashMap::new();

        // Track child occurrences per parent instance
        let mut parent_children_count: HashMap<String, HashMap<String, usize>> = HashMap::new();

        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                    let raw_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let (prefix, local_name) = Self::parse_qname(&raw_name);

                    // Store root element for format detection
                    if root_element.is_none() {
                        root_element = Some(local_name.clone());
                    }

                    current_depth += 1;
                    if current_depth > max_depth {
                        max_depth = current_depth;
                    }

                    // Record element
                    stats.record_element(&local_name);

                    // Set namespace information
                    if let Some(element_stats) = stats.elements.get_mut(&local_name) {
                        if current_depth > element_stats.max_depth {
                            element_stats.max_depth = current_depth;
                        }

                        // Extract namespace URI from attributes if present
                        let namespace_uri = if let Some(ref pfx) = prefix {
                            stats.namespaces.get(pfx).cloned()
                        } else {
                            stats.namespaces.get("").cloned()
                        };

                        if let Some(ns_uri) = namespace_uri {
                            element_stats.set_namespace(ns_uri);
                        }
                    }

                    // Record parent-child relationship with occurrence tracking
                    if let Some(parent) = element_stack.last() {
                        stats.record_child(parent, &local_name);

                        // Track child count for this parent instance
                        parent_children_count
                            .entry(parent.clone())
                            .or_insert_with(HashMap::new)
                            .entry(local_name.clone())
                            .and_modify(|count| *count += 1)
                            .or_insert(1);

                        // Mark parent as having children
                        element_has_children.insert(parent.clone(), true);
                    }

                    // Process attributes (including namespace declarations)
                    for attr in e.attributes() {
                        if let Ok(attr) = attr {
                            let attr_name = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                            let attr_value = String::from_utf8_lossy(&attr.value).to_string();

                            // Handle namespace declarations
                            if attr_name == "xmlns" {
                                stats.record_namespace(String::new(), attr_value.clone());
                            } else if let Some(prefix) = attr_name.strip_prefix("xmlns:") {
                                stats.record_namespace(prefix.to_string(), attr_value.clone());
                            }

                            stats.record_attribute(&local_name, &attr_name, attr_value);
                        }
                    }

                    element_stack.push(local_name);
                }
                Ok(Event::End(_)) => {
                    if let Some(popped_element) = element_stack.pop() {
                        // Check for mixed content
                        let has_text = element_has_text
                            .get(&popped_element)
                            .copied()
                            .unwrap_or(false);
                        let has_children = element_has_children
                            .get(&popped_element)
                            .copied()
                            .unwrap_or(false);

                        if Self::is_mixed_content(has_text, has_children) {
                            if let Some(element_stats) = stats.elements.get_mut(&popped_element) {
                                element_stats.mark_mixed_content();
                            }
                        }

                        // Update child occurrence statistics for completed parent
                        if let Some(children_counts) = parent_children_count.remove(&popped_element)
                        {
                            if let Some(element_stats) = stats.elements.get_mut(&popped_element) {
                                for (child_name, count) in children_counts {
                                    if let Some(child_stats) =
                                        element_stats.children.get_mut(&child_name)
                                    {
                                        child_stats.update_occurs(count, count);
                                    }
                                }
                            }
                        }
                    }

                    if current_depth > 0 {
                        current_depth -= 1;
                    }
                }
                Ok(Event::Text(e)) => {
                    if let Some(current_element) = element_stack.last() {
                        let text = e.unescape().map_err(|e| {
                            InferenceError::ParseServiceError(format!(
                                "XML text unescape error: {}",
                                e
                            ))
                        })?;
                        let text_str = text.to_string().trim().to_string();
                        if !text_str.is_empty() {
                            stats.add_text_sample(current_element, text_str);
                            element_has_text.insert(current_element.clone(), true);
                        }
                    }
                }
                Ok(Event::CData(e)) => {
                    if let Some(current_element) = element_stack.last() {
                        let text_str = String::from_utf8_lossy(e.as_ref()).trim().to_string();
                        if !text_str.is_empty() {
                            stats.add_text_sample(current_element, text_str);
                            element_has_text.insert(current_element.clone(), true);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(InferenceError::ParseServiceError(format!(
                        "XML parsing error: {}",
                        e
                    )));
                }
                _ => {}
            }
            buf.clear();
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

        // Detect specific XML format if possible
        let detected_format = root_element
            .as_ref()
            .and_then(|root| Self::detect_xml_format(root, &stats.namespaces));

        let format_name = detected_format.unwrap_or_else(|| "XML".to_string());

        // Set metadata
        let now =
            self.timestamp.now_utc().await.map_err(|e| {
                InferenceError::ServiceError(format!("Failed to get timestamp: {}", e))
            })?;

        stats.metadata = SchemaMetadata {
            schema_id: Some("xml_schema".to_string()),
            schema_name: Some(format!("{} Schema", format_name)),
            version: Some("1.0.0".to_string()),
            generated_at: Some(now),
            generator: Some("rootreal-schema-inference/1.0".to_string()),
            source_files: vec![],
        };

        self.logger
            .log_info(&format!(
                "{} analysis complete: {} elements, {} unique element types, {} namespaces",
                format_name,
                stats.document_metrics.total_elements,
                stats.document_metrics.unique_element_names,
                stats.namespaces.len()
            ))
            .await
            .map_err(|e| InferenceError::LoggerError(e.to_string()))?;

        Ok(stats)
    }

    fn format_name(&self) -> &str {
        "xml"
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
                "Auto-generated schema from XML introspection ({})",
                stats.format
            ))
            .with_version("1.0.0")
            .with_default_range("string");

        // Create classes for each element
        for (element_name, element_stats) in &stats.elements {
            let mut class_builder = builder.add_class(element_name);

            class_builder = class_builder.with_description(format!(
                "Element '{}' appears {} times, max depth: {}",
                element_name, element_stats.occurrence_count, element_stats.max_depth
            ));

            // Add slots for attributes
            for (attr_name, attr_stats) in &element_stats.attributes {
                let inferred_type = self
                    .type_inferencer
                    .infer_from_samples(&attr_stats.value_samples);
                let required = attr_stats.occurrence_count == element_stats.occurrence_count;

                class_builder =
                    class_builder.add_slot_with_type(attr_name, &inferred_type, required, false);
            }

            // Add slot for text content if present
            if !element_stats.text_samples.is_empty() {
                let inferred_type = self
                    .type_inferencer
                    .infer_from_samples(&element_stats.text_samples);
                class_builder =
                    class_builder.add_slot_with_type("text_content", &inferred_type, false, false);
            }

            // Add slots for child elements with improved cardinality detection
            for (child_name, child_stats) in &element_stats.children {
                let required = child_stats.is_required();
                let multivalued = child_stats.is_multivalued();

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
    use logger_service::create_logger_service;
    use timestamp_service::create_timestamp_service;

    fn create_test_services() -> (
        Arc<dyn LoggerService<Error = LoggerError>>,
        Arc<dyn TimestampService<Error = TimestampError>>,
    ) {
        let logger =
            create_logger_service().unwrap_or_else(|e| panic!("Failed to create logger: {}", e));
        let timestamp = create_timestamp_service();
        (logger, timestamp)
    }

    #[tokio::test]
    async fn test_xml_introspector_format_name() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);
        assert_eq!(introspector.format_name(), "xml");
    }

    #[tokio::test]
    async fn test_analyze_simple_xml() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <root>
                <person age="25">
                    <name>John Doe</name>
                    <email>john@example.com</email>
                </person>
                <person age="30">
                    <name>Jane Smith</name>
                    <email>jane@example.com</email>
                </person>
            </root>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        assert_eq!(stats.elements.len(), 4); // root, person, name, email
        assert!(stats.elements.contains_key("root"));
        assert!(stats.elements.contains_key("person"));
        assert!(stats.elements.contains_key("name"));
        assert!(stats.elements.contains_key("email"));

        let person = stats.elements.get("person").unwrap();
        assert_eq!(person.occurrence_count, 2);
        assert!(person.attributes.contains_key("age"));
        assert_eq!(person.children.len(), 2); // name, email
    }

    #[tokio::test]
    async fn test_generate_schema_from_simple_xml() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <root>
                <person age="25">
                    <name>John Doe</name>
                </person>
            </root>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();
        let schema = introspector
            .generate_schema(&stats, "test_schema")
            .await
            .unwrap();

        assert_eq!(schema.id, "test_schema");
        assert!(schema.classes.contains_key("root"));
        assert!(schema.classes.contains_key("person"));
        assert!(schema.classes.contains_key("name"));

        let person_class = schema.classes.get("person").unwrap();
        assert!(person_class.slots.contains(&"age".to_string()));
    }

    #[tokio::test]
    async fn test_analyze_nested_xml() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <root>
                <level1>
                    <level2>
                        <level3>Deep content</level3>
                    </level2>
                </level1>
            </root>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        assert_eq!(stats.document_metrics.max_nesting_depth, 4); // root -> level1 -> level2 -> level3
        assert!(stats.elements.contains_key("level3"));

        let level3 = stats.elements.get("level3").unwrap();
        assert_eq!(level3.text_samples.len(), 1);
        assert_eq!(level3.text_samples[0], "Deep content");
    }

    #[tokio::test]
    async fn test_analyze_xml_with_attributes() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <book isbn="123-456" year="2025">
                <title lang="en">Test Book</title>
            </book>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let book = stats.elements.get("book").unwrap();
        assert_eq!(book.attributes.len(), 2);
        assert!(book.attributes.contains_key("isbn"));
        assert!(book.attributes.contains_key("year"));

        let isbn = book.attributes.get("isbn").unwrap();
        assert_eq!(isbn.value_samples[0], "123-456");
    }

    #[tokio::test]
    async fn test_namespace_detection() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <root xmlns="http://example.com/default" xmlns:custom="http://example.com/custom">
                <custom:element>Content</custom:element>
            </root>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        // Check namespace declarations were captured
        assert_eq!(stats.namespaces.len(), 2);
        assert_eq!(
            stats.namespaces.get(""),
            Some(&"http://example.com/default".to_string())
        );
        assert_eq!(
            stats.namespaces.get("custom"),
            Some(&"http://example.com/custom".to_string())
        );

        // Check element has namespace information
        let element = stats.elements.get("element").unwrap();
        assert_eq!(element.namespace_prefix, Some("custom".to_string()));
        assert_eq!(
            element.namespace_uri,
            Some("http://example.com/custom".to_string())
        );
    }

    #[tokio::test]
    async fn test_page_xml_format_detection() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <PcGts xmlns="http://schema.primaresearch.org/PAGE/gts/pagecontent/2013-07-15">
                <Metadata>
                    <Creator>Test</Creator>
                </Metadata>
                <Page imageFilename="test.jpg" imageWidth="1000" imageHeight="1000">
                    <TextRegion id="r1">
                        <Coords points="0,0 100,0 100,100 0,100"/>
                        <TextLine id="l1">
                            <Baseline points="0,50 100,50"/>
                            <TextEquiv>
                                <PlainText>Test text</PlainText>
                            </TextEquiv>
                        </TextLine>
                    </TextRegion>
                </Page>
            </PcGts>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        // Verify format was detected
        assert!(
            stats
                .metadata
                .schema_name
                .is_some_and(|name| name.contains("PAGE-XML"))
        );

        // Verify structure was analyzed
        assert!(stats.elements.contains_key("PcGts"));
        assert!(stats.elements.contains_key("TextRegion"));
        assert!(stats.elements.contains_key("TextLine"));
        assert!(stats.elements.contains_key("Baseline"));
        assert!(stats.elements.contains_key("PlainText"));
    }

    #[tokio::test]
    async fn test_ead_format_detection() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <ead xmlns="urn:isbn:1-931666-22-9">
                <eadheader>
                    <eadid>test-001</eadid>
                    <filedesc>
                        <titlestmt>
                            <titleproper>Test Collection</titleproper>
                        </titlestmt>
                    </filedesc>
                </eadheader>
                <archdesc level="collection">
                    <did>
                        <unittitle>Test Finding Aid</unittitle>
                    </did>
                </archdesc>
            </ead>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        // Verify format was detected
        assert!(
            stats
                .metadata
                .schema_name
                .is_some_and(|name| name.contains("EAD"))
        );

        // Verify EAD structure
        assert!(stats.elements.contains_key("ead"));
        assert!(stats.elements.contains_key("archdesc"));
        assert!(stats.elements.contains_key("did"));
    }

    #[tokio::test]
    async fn test_dublin_core_format_detection() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
                <dc:title>Test Document</dc:title>
                <dc:creator>Test Author</dc:creator>
                <dc:date>2025-10-02</dc:date>
                <dc:description>Test description</dc:description>
            </metadata>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        // Verify format was detected
        assert!(
            stats
                .metadata
                .schema_name
                .is_some_and(|name| name.contains("Dublin Core"))
        );

        // Verify Dublin Core elements
        assert!(stats.elements.contains_key("title"));
        assert!(stats.elements.contains_key("creator"));
        assert!(stats.elements.contains_key("date"));
        assert!(stats.elements.contains_key("description"));
    }

    #[tokio::test]
    async fn test_mixed_content_detection() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <paragraph>
                This is some text with <emphasis>emphasized content</emphasis> mixed in.
            </paragraph>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        // Verify mixed content was detected
        let paragraph = stats.elements.get("paragraph").unwrap();
        assert!(
            paragraph.has_mixed_content,
            "Paragraph should have mixed content"
        );
        assert!(
            !paragraph.text_samples.is_empty(),
            "Should have text samples"
        );
        assert!(!paragraph.children.is_empty(), "Should have child elements");
    }

    #[tokio::test]
    async fn test_cdata_handling() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <script>
                <![CDATA[
                    function test() {
                        return x < y && a > b;
                    }
                ]]>
            </script>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let script = stats.elements.get("script").unwrap();
        assert!(
            !script.text_samples.is_empty(),
            "CDATA content should be captured"
        );
        assert!(
            script.text_samples[0].contains("function test"),
            "CDATA content should be preserved"
        );
    }

    #[tokio::test]
    async fn test_cardinality_detection_required() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <library>
                <book>
                    <title>Book 1</title>
                    <author>Author 1</author>
                </book>
                <book>
                    <title>Book 2</title>
                    <author>Author 2</author>
                </book>
            </library>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let book = stats.elements.get("book").unwrap();
        let title_child = book.children.get("title").unwrap();
        let author_child = book.children.get("author").unwrap();

        // Both title and author appear in all book instances, so they're required
        assert!(title_child.is_required(), "Title should be required");
        assert!(author_child.is_required(), "Author should be required");
    }

    #[tokio::test]
    async fn test_cardinality_detection_optional() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <library>
                <book>
                    <title>Book 1</title>
                    <isbn>123-456</isbn>
                </book>
                <book>
                    <title>Book 2</title>
                </book>
            </library>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let book = stats.elements.get("book").unwrap();
        let title_child = book.children.get("title").unwrap();
        let isbn_child = book.children.get("isbn").unwrap();

        assert!(title_child.is_required(), "Title should be required");
        assert!(!isbn_child.is_required(), "ISBN should be optional");
    }

    #[tokio::test]
    async fn test_cardinality_detection_multivalued() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <library>
                <book>
                    <title>Book 1</title>
                    <author>Author 1</author>
                    <author>Author 2</author>
                    <author>Author 3</author>
                </book>
            </library>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let book = stats.elements.get("book").unwrap();
        let author_child = book.children.get("author").unwrap();

        assert!(
            author_child.is_multivalued(),
            "Author should be multivalued"
        );
        assert_eq!(author_child.max_occurs, 3, "Max occurs should be 3");
        assert_eq!(author_child.min_occurs, 3, "Min occurs should be 3");
    }

    #[tokio::test]
    async fn test_min_max_occurs_tracking() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <catalog>
                <product>
                    <tag>electronics</tag>
                </product>
                <product>
                    <tag>computers</tag>
                    <tag>laptops</tag>
                </product>
                <product>
                    <tag>phones</tag>
                    <tag>smartphones</tag>
                    <tag>android</tag>
                </product>
            </catalog>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        let product = stats.elements.get("product").unwrap();
        let tag_child = product.children.get("tag").unwrap();

        assert_eq!(tag_child.min_occurs, 1, "Min occurs should be 1");
        assert_eq!(tag_child.max_occurs, 3, "Max occurs should be 3");
        assert!(
            tag_child.is_required(),
            "Tag should be required (appears in all products)"
        );
        assert!(tag_child.is_multivalued(), "Tag should be multivalued");
    }

    #[tokio::test]
    async fn test_complex_namespace_scenario() {
        let (logger, timestamp) = create_test_services();
        let introspector = XmlIntrospector::new(logger, timestamp);

        let xml = br#"
            <root xmlns="http://default.com" xmlns:ns1="http://ns1.com" xmlns:ns2="http://ns2.com">
                <ns1:item>Item 1</ns1:item>
                <ns2:item>Item 2</ns2:item>
                <item>Default NS Item</item>
            </root>
        "#;

        let stats = introspector.analyze_bytes(xml).await.unwrap();

        assert_eq!(stats.namespaces.len(), 3);
        assert!(stats.elements.contains_key("item"));

        // All items should be tracked under the same local name
        let item = stats.elements.get("item").unwrap();
        assert_eq!(item.occurrence_count, 3);
    }
}
