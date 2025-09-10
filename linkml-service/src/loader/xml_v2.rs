//! XML loader using RootReal Parse Service
//!
//! This module provides XML loading functionality that integrates with
//! RootReal's Parse Service, avoiding direct file system access.

use super::traits::{DataLoader, LoaderError, LoaderResult, DataInstance};
use linkml_core::prelude::*;
use async_trait::async_trait;
use serde_json::{Value, Map};
use std::sync::Arc;
use parse_core::{ParseService, ParseFormat, XmlFormat};

/// `XML` loader that uses Parse Service
pub struct XmlLoaderV2<P: ParseService> {
    /// Parse service instance
    parse_service: Arc<P>,
    /// Content to parse (instead of file path)
    content: Option<String>,
    /// Root element name
    root_element: String}

impl<P: ParseService> XmlLoaderV2<P> {
    /// Create a new `XML` loader with Parse Service
    pub fn new(parse_service: Arc<P>) -> Self {
        Self {
            parse_service,
            content: None,
            root_element: "data".to_string()}
    }

    /// Set the content to parse
    pub fn with_content(mut self, content: String) -> Self {
        self.content = Some(content);
        self
    }

    /// Set the root element name
    pub fn with_root_element(mut self, root: &str) -> Self {
        self.root_element = root.to_string();
        self
    }
}

#[async_trait]
impl<P: ParseService + Send + Sync> DataLoader for XmlLoaderV2<P> {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let content = self.content.as_ref()
            .ok_or_else(|| LoaderError::Configuration("No content provided".to_string()))?;

        // Use Parse Service to parse XML
        let parsed_doc = self.parse_service
            .parse_document(content, ParseFormat::Xml(XmlFormat::Generic))
            .await
            .map_err(|e| LoaderError::Parser(format!("Parse service error: {e}")))?;

        // Convert parsed document to LinkML data instances
        let instances = convert_parsed_to_instances(&parsed_doc, schema)?;

        Ok(instances)
    }
}

/// Convert parsed document to `LinkML` data instances
fn convert_parsed_to_instances(
    parsed_doc: &parse_core::ParsedDocument,
    schema: &SchemaDefinition,
) -> LoaderResult<Vec<DataInstance>> {
    let mut instances = Vec::new();

    // Extract structured content from parsed document
    if let parse_core::DocumentContent::Structured(structured) = &parsed_doc.content {
        // Convert structured content to data instances
        for element in &structured.elements {
            if let Some(instance) = convert_element_to_instance(element, schema)? {
                instances.push(instance);
            }
        }
    } else {
        return Err(LoaderError::Parser("Expected structured XML content".to_string());
    }

    Ok(instances)
}

/// Convert a content element to a data instance
fn convert_element_to_instance(
    element: &parse_core::ContentElement,
    schema: &SchemaDefinition,
) -> LoaderResult<Option<DataInstance>> {
    // Check if element name matches a class in the schema
    if !schema.classes.contains_key(&element.name) {
        // Skip elements that don't correspond to schema classes
        return Ok(None);
    }

    let mut data = Map::new();

    // Add attributes as properties
    for (key, value) in &element.attributes {
        data.insert(key.clone(), Value::String(value.clone());
    }

    // Process element content based on type
    match &element.content {
        parse_core::ElementContent::Text(text) => {
            // If element has text content, add it as "value" property
            data.insert("value".to_string(), Value::String(text.clone());
        }
        parse_core::ElementContent::Mixed(mixed) => {
            // Process mixed content (text and child elements)
            process_mixed_content(mixed, &mut data, schema)?;
        }
        parse_core::ElementContent::Elements(children) => {
            // Process child elements
            for child in children {
                process_child_element(child, &mut data, schema)?;
            }
        }
    }

    Ok(Some(DataInstance {
        class_name: element.name.clone(),
        data: Value::Object(data)}))
}

/// Process mixed content (text and elements)
fn process_mixed_content(
    mixed: &parse_core::MixedContent,
    data: &mut Map<String, Value>,
    schema: &SchemaDefinition,
) -> LoaderResult<()> {
    let mut text_parts = Vec::new();
    let mut elements_by_name: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();

    for item in &mixed.items {
        match item {
            parse_core::MixedContentItem::Text(text) => {
                text_parts.push(text.clone());
            }
            parse_core::MixedContentItem::Element(element) => {
                // Convert element to value
                if let Some(value) = element_to_value(element, schema)? {
                    elements_by_name
                        .entry(element.name.clone())
                        .or_default()
                        .push(value);
                }
            }
        }
    }

    // Add concatenated text if any
    if !text_parts.is_empty() {
        let combined_text = text_parts.join(" ").trim().to_string();
        if !combined_text.is_empty() {
            data.insert("text".to_string(), Value::String(combined_text));
        }
    }

    // Add child elements
    for (name, mut values) in elements_by_name {
        if values.len() == 1 {
            // Safe to use expect here because we just checked length is 1
            data.insert(name, values.into_iter().next().expect("values should have exactly one element: {}"));
        } else {
            data.insert(name, Value::Array(values));
        }
    }

    Ok(())
}

/// Process a child element
fn process_child_element(
    element: &parse_core::ContentElement,
    data: &mut Map<String, Value>,
    schema: &SchemaDefinition,
) -> LoaderResult<()> {
    if let Some(value) = element_to_value(element, schema)? {
        // Check if property already exists (for multivalued)
        if let Some(existing) = data.get_mut(&element.name) {
            // Convert to array if not already
            match existing {
                Value::Array(arr) => {
                    arr.push(value);
                }
                _ => {
                    let old_value = existing.clone();
                    *existing = Value::Array(vec![old_value, value]);
                }
            }
        } else {
            data.insert(element.name.clone(), value);
        }
    }

    Ok(())
}

/// Convert element to `JSON` value
fn element_to_value(
    element: &parse_core::ContentElement,
    schema: &SchemaDefinition,
) -> LoaderResult<Option<Value>> {
    // If element represents a class, convert to object
    if schema.classes.contains_key(&element.name) {
        if let Some(instance) = convert_element_to_instance(element, schema)? {
            return Ok(Some(instance.data));
        }
    }

    // Otherwise, extract simple value
    match &element.content {
        parse_core::ElementContent::Text(text) => {
            Ok(Some(Value::String(text.clone())))
        }
        parse_core::ElementContent::Mixed(mixed) => {
            // For mixed content, extract text
            let text: String = mixed.items.iter()
                .filter_map(|item| match item {
                    parse_core::MixedContentItem::Text(t) => Some(t.as_str()),
                    _ => None})
                .collect::<Vec<_>>()
                .join(" ");
            Ok(Some(Value::String(text.trim().to_string())))
        }
        parse_core::ElementContent::Elements(_) => {
            // Complex element - convert to object
            let mut obj = Map::new();
            if let Some(instance) = convert_element_to_instance(element, schema)? {
                if let Value::Object(map) = instance.data {
                    obj = map;
                }
            }
            Ok(Some(Value::Object(obj)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests would use mock Parse Service following RootReal patterns
}