//! Real XML loader implementation for `LinkML`
//!
//! This module provides comprehensive XML loading functionality
//! for `LinkML` schemas and data instances.

use async_trait::async_trait;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};
use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::loader::{DataInstance, DataLoader, LoadOptions, LoaderError, LoaderResult};
use linkml_core::prelude::*;

/// `XML` loader implementation
pub struct XmlLoader {
    /// Parser configuration
    config: XmlConfig,
}

/// `XML` parser configuration
#[derive(Debug, Clone)]
pub struct XmlConfig {
    /// Trim whitespace from text nodes
    pub trim_text: bool,
    /// Convert attributes to properties
    pub attributes_as_properties: bool,
    /// Namespace handling
    pub handle_namespaces: bool,
    /// Preserve element order
    pub preserve_order: bool,
}

impl Default for XmlConfig {
    fn default() -> Self {
        Self {
            trim_text: true,
            attributes_as_properties: true,
            handle_namespaces: true,
            preserve_order: false,
        }
    }
}

impl Default for XmlLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlLoader {
    /// Create a new `XML` loader with default configuration
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: XmlConfig::default(),
        }
    }

    /// Create `XML` loader with custom configuration
    #[must_use]
    pub fn with_config(config: XmlConfig) -> Self {
        Self { config }
    }

    /// Parse `XML` content into `JSON`-like structure
    fn parse_xml(&self, xml_content: &str) -> LoaderResult<Value> {
        let mut reader = Reader::from_str(xml_content);
        reader.trim_text(self.config.trim_text);

        let mut buf = Vec::new();
        let mut stack: Vec<XmlElement> = Vec::new();
        let mut root: Option<Value> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(e)) => {
                    let element = self.parse_element(&e)?;
                    stack.push(element);
                }
                Ok(Event::End(_)) => {
                    if let Some(completed) = stack.pop() {
                        let value = self.element_to_value(completed);

                        if let Some(parent) = stack.last_mut() {
                            parent.add_child(value);
                        } else {
                            root = Some(value);
                        }
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e
                        .unescape()
                        .map_err(|err| {
                            LoaderError::Parse(format!("XML text unescape error: {err}"))
                        })?
                        .to_string();

                    if !text.trim().is_empty()
                        && let Some(current) = stack.last_mut()
                    {
                        current.text = Some(text);
                    }
                }
                Ok(Event::Empty(e)) => {
                    let element = self.parse_element(&e)?;
                    let value = self.element_to_value(element);

                    if let Some(parent) = stack.last_mut() {
                        parent.add_child(value);
                    } else {
                        root = Some(value);
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(LoaderError::Parse(format!("XML parse error: {e}"))),
                _ => {} // Ignore other events (comments, processing instructions, etc.)
            }

            buf.clear();
        }

        root.ok_or_else(|| LoaderError::Parse("No root element found in XML".to_string()))
    }

    /// Parse `XML` element start tag
    fn parse_element(&self, e: &BytesStart) -> LoaderResult<XmlElement> {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
        let mut attributes = HashMap::new();

        for attr_result in e.attributes() {
            let attr = attr_result
                .map_err(|err| LoaderError::Parse(format!("XML attribute error: {err}")))?;

            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = attr
                .unescape_value()
                .map_err(|err| LoaderError::Parse(format!("XML attribute value error: {err}")))?
                .to_string();

            attributes.insert(key, value);
        }

        Ok(XmlElement {
            name,
            attributes,
            children: Vec::new(),
            text: None,
        })
    }

    /// Convert `XML` element to `JSON` value
    fn element_to_value(&self, element: XmlElement) -> Value {
        let mut obj = Map::new();

        // Add element name as type hint
        obj.insert("_type".to_string(), Value::String(element.name.clone()));

        // Add attributes
        if self.config.attributes_as_properties && !element.attributes.is_empty() {
            for (key, value) in element.attributes {
                obj.insert(format!("@{key}"), Value::String(value));
            }
        }

        // Add text content if present
        if let Some(text) = element.text {
            if element.children.is_empty() {
                // Leaf node with text - return as simple value
                return Value::String(text);
            }
            // Mixed content - add as special property
            obj.insert("_text".to_string(), Value::String(text));
        }

        // Add children
        for child in element.children {
            if let Value::Object(child_obj) = child {
                if let Some(Value::String(child_type)) = child_obj.get("_type") {
                    let child_name = child_type.clone();

                    // Group children by element name
                    let entry = obj
                        .entry(child_name.clone())
                        .or_insert_with(|| Value::Array(Vec::new()));
                    if let Value::Array(arr) = entry {
                        // Clean up child object (remove _type)
                        let mut clean_child = child_obj.clone();
                        clean_child.remove("_type");

                        if clean_child.len() == 1 && clean_child.contains_key("_text") {
                            // Simple text element
                            if let Some(text_val) = clean_child.get("_text") {
                                arr.push(text_val.clone());
                            }
                        } else {
                            arr.push(Value::Object(clean_child));
                        }
                    }
                }
            } else {
                // Simple value child
                let children_entry = obj
                    .entry("children".to_string())
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(arr) = children_entry {
                    arr.push(child);
                }
            }
        }

        Value::Object(obj)
    }

    /// Convert `JSON` value to `LinkML` data instance
    fn value_to_instance(&self, value: Value, target_class: &str) -> DataInstance {
        // Convert Value to HashMap<String, JsonValue>
        let data = if let Value::Object(obj) = value {
            // Convert serde_json::Map to HashMap
            obj.into_iter().collect()
        } else {
            let mut map = HashMap::new();
            map.insert("value".to_string(), value);
            map
        };

        DataInstance {
            class_name: target_class.to_string(),
            data,
            id: None,
            metadata: HashMap::new(),
        }
    }
}

/// Internal `XML` element representation
struct XmlElement {
    name: String,
    attributes: HashMap<String, String>,
    children: Vec<Value>,
    text: Option<String>,
}

impl XmlElement {
    fn add_child(&mut self, child: Value) {
        self.children.push(child);
    }
}

#[async_trait]
impl DataLoader for XmlLoader {
    fn name(&self) -> &'static str {
        "XmlLoader"
    }

    fn description(&self) -> &'static str {
        "Loads data from XML files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["xml"]
    }

    async fn load_file(
        &self,
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content = std::fs::read_to_string(path).map_err(LoaderError::Io)?;

        self.load_string(&content, schema, options).await
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Parse XML to JSON structure
        let value = self.parse_xml(content)?;

        // Determine target class
        let target_class = options
            .target_class
            .as_deref()
            .or_else(|| {
                schema
                    .classes
                    .keys()
                    .next()
                    .map(std::string::String::as_str)
            })
            .unwrap_or("Entity");

        // Convert to data instance
        let instance = self.value_to_instance(value, target_class);

        Ok(vec![instance])
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let content =
            String::from_utf8(data.to_vec()).map_err(|e| LoaderError::Parse(e.to_string()))?;
        self.load_string(&content, schema, options).await
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> LoaderResult<()> {
        // XML can represent any schema structure
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::error::Result;

    #[tokio::test]
    async fn test_xml_parsing() -> Result<()> {
        let xml = r#"
            <person id="123">
                <name>John Doe</name>
                <age>30</age>
                <email>john@example.com</email>
            </person>
        "#;

        let loader = XmlLoader::new();
        let value = loader.parse_xml(xml)?;

        assert!(value.is_object());
        if let Value::Object(obj) = value {
            assert_eq!(obj.get("_type"), Some(&Value::String("person".to_string())));
            assert_eq!(obj.get("@id"), Some(&Value::String("123".to_string())));
        }
        Ok(())
    }

    #[tokio::test]
    async fn test_nested_xml() -> Result<()> {
        let xml = r"
            <organization>
                <name>ACME Corp</name>
                <employees>
                    <employee>
                        <name>Alice</name>
                        <role>Developer</role>
                    </employee>
                    <employee>
                        <name>Bob</name>
                        <role>Manager</role>
                    </employee>
                </employees>
            </organization>
        ";

        let loader = XmlLoader::new();
        let value = loader.parse_xml(xml)?;

        assert!(value.is_object());
        Ok(())
    }
}
