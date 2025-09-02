//! XML loader and dumper for LinkML
//!
//! This module provides functionality to load and dump LinkML data in XML format.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value;

/// XML loader for LinkML data
pub struct XmlLoader {
    /// Input file path
    file_path: Option<String>,
    /// Root element name
    root_element: String,
}

impl XmlLoader {
    /// Create a new XML loader
    pub fn new() -> Self {
        Self {
            file_path: None,
            root_element: "data".to_string(),
        }
    }

    /// Set the input file path
    pub fn with_file(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        self
    }

    /// Set the root element name
    pub fn with_root_element(mut self, root: &str) -> Self {
        self.root_element = root.to_string();
        self
    }
}

impl Default for XmlLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for XmlLoader {
    fn name(&self) -> &str {
        "xml"
    }

    fn description(&self) -> &str {
        "Load data from XML files"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["xml"]
    }

    async fn load_file(
        &self,
        path: &std::path::Path,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        let _content = std::fs::read_to_string(path).map_err(|e| LoaderError::Io(e))?;

        // Basic XML parsing implementation
        // This is a simplified implementation that can be enhanced with full XML schema support
        self.load_string(&_content, _schema, _options).await
    }

    async fn load_string(
        &self,
        _content: &str,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Basic XML parsing implementation using quick-xml
        use quick_xml::Reader;
        use quick_xml::events::Event;
        use std::collections::HashMap;

        let mut reader = Reader::from_str(_content);
        reader.trim_text(true);

        let mut instances = Vec::new();
        let mut current_instance: Option<DataInstance> = None;
        let mut current_element = String::new();
        let mut current_values = HashMap::new();

        loop {
            match reader.read_event() {
                Ok(Event::Start(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if _schema.classes.contains_key(&name) {
                        // Start of a new instance
                        current_instance = Some(DataInstance {
                            id: None,
                            class_name: name.clone(),
                            data: HashMap::new(),
                            metadata: HashMap::new(),
                        });
                        current_values.clear();
                    } else {
                        current_element = name;
                    }
                }
                Ok(Event::Text(e)) => {
                    if !current_element.is_empty() {
                        let text = e.unescape().unwrap_or_default().to_string();
                        if !text.trim().is_empty() {
                            current_values.insert(current_element.clone(), serde_json::Value::String(text));
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if let Some(ref mut instance) = current_instance {
                        if instance.class_name == name {
                            // End of instance
                            instance.data = current_values.clone();
                            instances.push(instance.clone());
                            current_instance = None;
                            current_values.clear();
                        }
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(LoaderError::InvalidFormat(format!("XML parsing error: {}", e))),
                _ => {}
            }
        }

        Ok(instances)
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
        Ok(())
    }
}

/// XML dumper for LinkML data
pub struct XmlDumper {
    /// Pretty print output
    pretty: bool,
    /// Root element name
    root_element: String,
    /// XML namespace
    namespace: Option<String>,
}

impl XmlDumper {
    /// Create a new XML dumper
    pub fn new(pretty: bool) -> Self {
        Self {
            pretty,
            root_element: "data".to_string(),
            namespace: None,
        }
    }

    /// Set the root element name
    pub fn with_root_element(mut self, root: &str) -> Self {
        self.root_element = root.to_string();
        self
    }

    /// Set the XML namespace
    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.namespace = Some(ns.to_string());
        self
    }
}

impl Default for XmlDumper {
    fn default() -> Self {
        Self::new(true)
    }
}

#[async_trait]
impl DataDumper for XmlDumper {
    fn name(&self) -> &str {
        "xml"
    }

    fn description(&self) -> &str {
        "Dump data to XML format"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["xml"]
    }

    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()> {
        let content = self.dump_string(instances, schema, options).await?;
        std::fs::write(path, content).map_err(|e| DumperError::Io(e))?;
        Ok(())
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        _schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String> {
        let mut xml = String::new();

        // XML declaration
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");

        // Root element
        if let Some(ns) = &self.namespace {
            xml.push_str(&format!("<{} xmlns=\"{}\">\n", self.root_element, ns));
        } else {
            xml.push_str(&format!("<{}>\n", self.root_element));
        }

        // Convert instances to XML
        for instance in instances {
            if self.pretty || options.pretty_print {
                xml.push_str("  ");
            }
            xml.push_str(&format!("<{}", instance.class_name));

            // Add simple attributes
            for (key, value) in &instance.data {
                if let Value::String(s) = value {
                    if !s.contains('\n') && s.len() < 50 {
                        xml.push_str(&format!(" {}=\"{}\"", key, escape_xml(s)));
                    }
                }
            }

            xml.push_str(">\n");

            // Add complex elements
            for (key, value) in &instance.data {
                match value {
                    Value::String(s) if s.contains('\n') || s.len() >= 50 => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        xml.push_str(&format!("<{}>{}</{}>\n", key, escape_xml(s), key));
                    }
                    Value::Number(n) => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        xml.push_str(&format!("<{}>{}</{}>\n", key, n, key));
                    }
                    Value::Bool(b) => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        xml.push_str(&format!("<{}>{}</{}>\n", key, b, key));
                    }
                    Value::Array(arr) => {
                        for item in arr {
                            if self.pretty || options.pretty_print {
                                xml.push_str("    ");
                            }
                            xml.push_str(&format!(
                                "<{}>{}</{}>\n",
                                key,
                                value_to_xml_string(item),
                                key
                            ));
                        }
                    }
                    _ => {}
                }
            }

            if self.pretty || options.pretty_print {
                xml.push_str("  ");
            }
            xml.push_str(&format!("</{}>\n", instance.class_name));
        }

        // Close root element
        xml.push_str(&format!("</{}>\n", self.root_element));

        Ok(xml)
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let result = self.dump_string(instances, schema, options).await?;
        Ok(result.into_bytes())
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> DumperResult<()> {
        Ok(())
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convert JSON value to XML string
fn value_to_xml_string(value: &Value) -> String {
    match value {
        Value::String(s) => escape_xml(s),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_xml_dumper() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let instances = vec![DataInstance {
            class_name: "Person".to_string(),
            data: std::collections::HashMap::from([
                ("id".to_string(), serde_json::json!("person1")),
                ("name".to_string(), serde_json::json!("Alice Smith")),
                ("age".to_string(), serde_json::json!(25)),
                ("active".to_string(), serde_json::json!(true)),
                (
                    "description".to_string(),
                    serde_json::json!("A person named Alice.\nShe is 25 years old."),
                ),
            ]),
            id: Some("person1".to_string()),
            metadata: std::collections::HashMap::new(),
        }];

        let schema = SchemaDefinition::default();
        let dumper = XmlDumper::new(true);
        let options = DumpOptions::default();
        let xml_str = dumper
            .dump_string(&instances, &schema, &options)
            .await?;
        assert!(xml_str.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(xml_str.contains("<data>"));
        // Name is an attribute, not a child element
        assert!(xml_str.contains("<Person id=\"person1\" name=\"Alice Smith\">"));
        assert!(xml_str.contains("<age>25</age>"));
        assert!(xml_str.contains("<active>true</active>"));
        assert!(xml_str.contains("<description>"));
        assert!(xml_str.contains("</Person>"));
        assert!(xml_str.contains("</data>"));
        Ok(())
    }
}
