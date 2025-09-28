//! XML loader and dumper for `LinkML`
//!
//! This module provides functionality to load and dump `LinkML` data in XML format.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperError, DumperResult, LoadOptions,
    LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value;
use std::fmt::Write;

/// `XML` loader for `LinkML` data
pub struct XmlLoader {
    /// Input file path
    file_path: Option<String>,
    /// Root element name
    root_element: String,
}

impl XmlLoader {
    /// Create a new `XML` loader
    #[must_use]
    pub fn new() -> Self {
        Self {
            file_path: None,
            root_element: "data".to_string(),
        }
    }

    /// Set the input file path
    #[must_use]
    pub fn with_file(mut self, path: &str) -> Self {
        self.file_path = Some(path.to_string());
        self
    }

    /// Set the root element name
    #[must_use]
    pub fn with_root_element(mut self, root: &str) -> Self {
        self.root_element = root.to_string();
        self
    }

    fn check_circular_references(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        use std::collections::HashSet;

        for (class_name, _class_def) in &schema.classes {
            let mut visited = HashSet::new();
            let mut stack = vec![class_name.clone()];

            while let Some(current_class) = stack.pop() {
                if visited.contains(&current_class) {
                    continue;
                }
                visited.insert(current_class.clone());

                if let Some(current_def) = schema.classes.get(&current_class) {
                    // Check parent classes
                    if let Some(parent) = &current_def.is_a {
                        if parent == class_name {
                            return Err(LoaderError::SchemaValidation(format!(
                                "Circular inheritance detected: class '{class_name}' inherits from itself"
                            )));
                        }
                        if !visited.contains(parent) {
                            stack.push(parent.clone());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for XmlLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DataLoader for XmlLoader {
    fn name(&self) -> &'static str {
        "xml"
    }

    fn description(&self) -> &'static str {
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
        let _content = std::fs::read_to_string(path).map_err(LoaderError::Io)?;

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
                            current_values
                                .insert(current_element.clone(), serde_json::Value::String(text));
                        }
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    if let Some(ref mut instance) = current_instance
                        && instance.class_name == name
                    {
                        // End of instance
                        instance.data.clone_from(&current_values);
                        instances.push(instance.clone());
                        current_instance = None;
                        current_values.clear();
                    }
                    current_element.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(LoaderError::InvalidFormat(format!(
                        "XML parsing error: {e}"
                    )));
                }
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

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate schema for XML loading compatibility

        // Check if schema has required basic elements
        if schema.name.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema name is required for XML loading".to_string(),
            ));
        }

        // Validate that classes have appropriate structures for XML
        for (class_name, class_def) in &schema.classes {
            // Check for XML-incompatible characters in class names
            if class_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                return Err(LoaderError::SchemaValidation(format!(
                    "Class name '{class_name}' contains XML-incompatible characters"
                )));
            }

            // Validate slots for XML compatibility
            for slot_name in &class_def.slots {
                if slot_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Slot name '{slot_name}' in class '{class_name}' contains XML-incompatible characters"
                    )));
                }
            }
        }

        // Validate enums for XML compatibility
        for (enum_name, enum_def) in &schema.enums {
            if enum_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                return Err(LoaderError::SchemaValidation(format!(
                    "Enum name '{enum_name}' contains XML-incompatible characters"
                )));
            }

            // Check enum values
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };
                if pv_text
                    .contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.')
                {
                    return Err(LoaderError::SchemaValidation(format!(
                        "Enum value '{pv_text}' in enum '{enum_name}' contains XML-incompatible characters"
                    )));
                }
            }
        }

        // Check for circular references that could cause issues in XML
        // This is a basic check - could be enhanced
        self.check_circular_references(schema)?;

        Ok(())
    }
}

/// `XML` dumper for `LinkML` data
pub struct XmlDumper {
    /// Pretty print output
    pretty: bool,
    /// Root element name
    root_element: String,
    /// `XML` namespace
    namespace: Option<String>,
}

impl XmlDumper {
    /// Create a new `XML` dumper
    #[must_use]
    pub fn new(pretty: bool) -> Self {
        Self {
            pretty,
            root_element: "data".to_string(),
            namespace: None,
        }
    }

    /// Set the root element name
    #[must_use]
    pub fn with_root_element(mut self, root: &str) -> Self {
        self.root_element = root.to_string();
        self
    }

    /// Set the `XML` namespace
    #[must_use]
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
    fn name(&self) -> &'static str {
        "xml"
    }

    fn description(&self) -> &'static str {
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
        std::fs::write(path, content).map_err(DumperError::Io)?;
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
        xml.push_str(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
",
        );

        // Root element
        if let Some(ns) = &self.namespace {
            writeln!(xml, "<{} xmlns=\"{}\">", self.root_element, ns)
                .expect("writeln! to String should never fail");
        } else {
            writeln!(xml, "<{}>", self.root_element).expect("writeln! to String should never fail");
        }

        // Convert instances to XML
        for instance in instances {
            if self.pretty || options.pretty_print {
                xml.push_str("  ");
            }
            write!(xml, "<{}", instance.class_name).expect("write! to String should never fail");

            // Add simple attributes
            for (key, value) in &instance.data {
                if let Value::String(s) = value
                    && !s.contains('\n')
                    && s.len() < 50
                {
                    use std::fmt::Write;
                    write!(xml, " {}=\"{}\"", key, escape_xml(s))
                        .expect("write to string cannot fail");
                }
            }

            xml.push_str(
                ">
",
            );

            // Add complex elements
            for (key, value) in &instance.data {
                match value {
                    Value::String(s) if s.contains('\n') || s.len() >= 50 => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        write!(
                            xml,
                            "<{}>{}</{}>
",
                            key,
                            escape_xml(s),
                            key
                        )
                        .expect("Writing to string cannot fail");
                    }
                    Value::Number(n) => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        writeln!(xml, "<{key}>{n}</{key}>")
                            .expect("writeln! to String should never fail");
                    }
                    Value::Bool(b) => {
                        if self.pretty || options.pretty_print {
                            xml.push_str("    ");
                        }
                        writeln!(xml, "<{key}>{b}</{key}>")
                            .expect("writeln! to String should never fail");
                    }
                    Value::Array(arr) => {
                        for item in arr {
                            if self.pretty || options.pretty_print {
                                xml.push_str("    ");
                            }
                            write!(
                                xml,
                                "<{}>{}</{}>
",
                                key,
                                value_to_xml_string(item),
                                key
                            )
                            .expect("Writing to string cannot fail");
                        }
                    }
                    _ => {}
                }
            }

            if self.pretty || options.pretty_print {
                xml.push_str("  ");
            }
            writeln!(xml, "</{}>", instance.class_name)
                .expect("writeln! to String should never fail");
        }

        // Close root element
        writeln!(xml, "</{}>", self.root_element).expect("writeln! to String should never fail");

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

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate schema for XML dumping compatibility

        // Check if schema has required basic elements for XML output
        if schema.name.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Schema name is required for XML dumping".to_string(),
            ));
        }

        // Validate that classes can be represented as XML elements
        for (class_name, class_def) in &schema.classes {
            // Check for XML-incompatible characters in class names
            if class_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                return Err(DumperError::SchemaValidation(format!(
                    "Class name '{class_name}' contains XML-incompatible characters"
                )));
            }

            // Check if class name conflicts with XML reserved names
            if class_name.to_lowercase().starts_with("xml") {
                return Err(DumperError::SchemaValidation(format!(
                    "Class name '{class_name}' starts with 'xml' which is reserved in XML"
                )));
            }

            // Validate slots for XML attribute/element compatibility
            for slot_name in &class_def.slots {
                if slot_name
                    .contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-' && c != '.')
                {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot name '{slot_name}' in class '{class_name}' contains XML-incompatible characters"
                    )));
                }

                // Check for XML namespace conflicts
                if slot_name.contains(':') {
                    return Err(DumperError::SchemaValidation(format!(
                        "Slot name '{slot_name}' contains ':' which conflicts with XML namespaces"
                    )));
                }
            }
        }

        // Validate enums for XML compatibility
        for (enum_name, enum_def) in &schema.enums {
            if enum_name.contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-') {
                return Err(DumperError::SchemaValidation(format!(
                    "Enum name '{enum_name}' contains XML-incompatible characters"
                )));
            }

            // Check enum values for XML compatibility
            for pv in &enum_def.permissible_values {
                let pv_text = match pv {
                    linkml_core::types::PermissibleValue::Simple(text)
                    | linkml_core::types::PermissibleValue::Complex { text, .. } => text,
                };
                if pv_text.is_empty() {
                    return Err(DumperError::SchemaValidation(format!(
                        "Empty enum value in enum '{enum_name}'"
                    )));
                }

                // Check for XML-unsafe characters in enum values
                if pv_text.contains(['<', '>', '&', '"', '\'']) {
                    return Err(DumperError::SchemaValidation(format!(
                        "Enum value '{pv_text}' contains XML-unsafe characters"
                    )));
                }
            }
        }

        // Check for potential namespace conflicts if namespace is set
        if let Some(namespace) = &self.namespace {
            if namespace.is_empty() {
                return Err(DumperError::SchemaValidation(
                    "XML namespace cannot be empty if specified".to_string(),
                ));
            }

            // Validate namespace URI format (basic check)
            if !namespace.starts_with("http://")
                && !namespace.starts_with("https://")
                && !namespace.starts_with("urn:")
            {
                return Err(DumperError::SchemaValidation(format!(
                    "Invalid namespace URI format: {namespace}"
                )));
            }
        }

        // Validate root element name
        if self.root_element.is_empty() {
            return Err(DumperError::SchemaValidation(
                "Root element name cannot be empty".to_string(),
            ));
        }

        if self
            .root_element
            .contains(|c: char| !c.is_alphanumeric() && c != '_' && c != '-')
        {
            return Err(DumperError::SchemaValidation(format!(
                "Root element name '{}' contains XML-incompatible characters",
                self.root_element
            )));
        }

        Ok(())
    }
}

/// Escape `XML` special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convert `JSON` value to XML string
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
                    serde_json::json!(
                        "A person named Alice.
She is 25 years old."
                    ),
                ),
            ]),
            id: Some("person1".to_string()),
            metadata: std::collections::HashMap::new(),
        }];

        let schema = SchemaDefinition::default();
        let dumper = XmlDumper::new(true);
        let options = DumpOptions::default();
        let xml_str = dumper.dump_string(&instances, &schema, &options).await?;
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
