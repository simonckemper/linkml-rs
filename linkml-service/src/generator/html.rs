//! HTML documentation generation for `LinkML` schemas

use super::options::{GeneratorOptions, IndentStyle};
use super::traits::{CodeFormatter, GeneratedOutput, Generator, GeneratorResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::fmt::Write;

/// HTML documentation generator for `LinkML` schemas
pub struct HtmlGenerator {
    /// Generator name
    name: String,
}

impl HtmlGenerator {
    /// Create a new HTML generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "html".to_string(),
        }
    }

    /// Generate HTML page header
    fn generate_header(&self, title: &str, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        writeln!(&mut output, "<!DOCTYPE html>").unwrap();
        writeln!(&mut output, "<html lang=\"en\">").unwrap();
        writeln!(&mut output, "<head>").unwrap();
        writeln!(&mut output, "    <meta charset=\"UTF-8\">").unwrap();
        writeln!(
            &mut output,
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )
        .unwrap();
        writeln!(
            &mut output,
            "    <title>{} - LinkML Documentation</title>",
            self.escape_html(title)
        )
        .unwrap();

        // Add embedded CSS
        writeln!(&mut output, "    <style>").unwrap();
        writeln!(&mut output, "{}", self.get_css()).unwrap();
        writeln!(&mut output, "    </style>").unwrap();

        writeln!(&mut output, "</head>").unwrap();
        writeln!(&mut output, "<body>").unwrap();

        // Navigation
        writeln!(&mut output, "    <nav class=\"sidebar\">").unwrap();
        writeln!(&mut output, "        <h2>Contents</h2>").unwrap();
        writeln!(&mut output, "        <ul>").unwrap();
        writeln!(
            &mut output,
            "            <li><a href=\"#overview\">Overview</a></li>"
        )
        .unwrap();

        if !schema.classes.is_empty() {
            writeln!(&mut output, "            <li>").unwrap();
            writeln!(
                &mut output,
                "                <a href=\"#classes\">Classes</a>"
            )
            .unwrap();
            writeln!(&mut output, "                <ul>").unwrap();
            for class_name in schema.classes.keys() {
                writeln!(
                    &mut output,
                    "                    <li><a href=\"#class-{}\">{}</a></li>",
                    self.to_anchor(class_name),
                    self.escape_html(class_name)
                )
                .unwrap();
            }
            writeln!(&mut output, "                </ul>").unwrap();
            writeln!(&mut output, "            </li>").unwrap();
        }

        if !schema.slots.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#slots\">Slots</a></li>"
            )
            .unwrap();
        }

        if !schema.enums.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#enums\">Enumerations</a></li>"
            )
            .unwrap();
        }

        if !schema.types.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#types\">Types</a></li>"
            )
            .unwrap();
        }

        writeln!(&mut output, "        </ul>").unwrap();
        writeln!(&mut output, "    </nav>").unwrap();

        writeln!(&mut output, "    <main class=\"content\">").unwrap();

        output
    }

    /// Generate HTML page footer
    fn generate_footer(&self) -> String {
        let _ = self;
        let mut output = String::new();

        writeln!(&mut output, "    </main>").unwrap();
        writeln!(&mut output, "    <footer>").unwrap();
        writeln!(
            &mut output,
            "        <p>Generated by LinkML HTML Generator</p>"
        )
        .unwrap();
        writeln!(&mut output, "    </footer>").unwrap();
        writeln!(&mut output, "</body>").unwrap();
        writeln!(&mut output, "</html>").unwrap();

        output
    }

    /// Generate overview section
    fn generate_overview(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        writeln!(
            &mut output,
            "        <section id=\"overview\" class=\"section\">"
        )
        .unwrap();
        writeln!(
            &mut output,
            "            <h1>{}</h1>",
            self.escape_html(&schema.name)
        )
        .unwrap();

        if let Some(desc) = &schema.description {
            writeln!(
                &mut output,
                "            <p class=\"description\">{}</p>",
                self.escape_html(desc)
            )
            .unwrap();
        }

        // Schema metadata
        writeln!(&mut output, "            <div class=\"metadata\">").unwrap();
        writeln!(&mut output, "                <h3>Schema Information</h3>").unwrap();
        writeln!(&mut output, "                <table>").unwrap();

        if !schema.id.is_empty() {
            writeln!(
                &mut output,
                "                    <tr><th>ID:</th><td>{}</td></tr>",
                self.escape_html(&schema.id)
            )
            .unwrap();
        }

        if let Some(version) = &schema.version {
            writeln!(
                &mut output,
                "                    <tr><th>Version:</th><td>{}</td></tr>",
                self.escape_html(version)
            )
            .unwrap();
        }

        if !schema.imports.is_empty() {
            writeln!(
                &mut output,
                "                    <tr><th>Imports:</th><td>{}</td></tr>",
                schema
                    .imports
                    .iter()
                    .map(|i| self.escape_html(i))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
            .unwrap();
        }

        writeln!(&mut output, "                </table>").unwrap();
        writeln!(&mut output, "            </div>").unwrap();
        writeln!(&mut output, "        </section>").unwrap();

        output
    }

    /// Generate classes section
    fn generate_classes(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        if schema.classes.is_empty() {
            return output;
        }

        writeln!(
            &mut output,
            "        <section id=\"classes\" class=\"section\">"
        )
        .unwrap();
        writeln!(&mut output, "            <h2>Classes</h2>").unwrap();

        for (class_name, class) in &schema.classes {
            writeln!(
                &mut output,
                "            <div id=\"class-{}\" class=\"class\">",
                self.to_anchor(class_name)
            )
            .unwrap();
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(class_name)
            )
            .unwrap();

            if let Some(desc) = &class.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .unwrap();
            }

            // Class properties
            writeln!(&mut output, "                <div class=\"properties\">").unwrap();

            if let Some(parent) = &class.is_a {
                writeln!(&mut output, "                    <p><strong>Inherits from:</strong> <a href=\"#class-{}\">{}</a></p>", 
                    self.to_anchor(parent),
                    self.escape_html(parent)
                ).unwrap();
            }

            if !class.mixins.is_empty() {
                writeln!(
                    &mut output,
                    "                    <p><strong>Mixins:</strong> {}</p>",
                    class
                        .mixins
                        .iter()
                        .map(|m| format!(
                            "<a href=\"#class-{}\">{}</a>",
                            self.to_anchor(m),
                            self.escape_html(m)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .unwrap();
            }

            if class.abstract_ == Some(true) {
                writeln!(
                    &mut output,
                    "                    <p class=\"badge abstract\">Abstract</p>"
                )
                .unwrap();
            }

            // Class slots
            if !class.slots.is_empty() {
                writeln!(&mut output, "                    <h4>Slots</h4>").unwrap();
                writeln!(&mut output, "                    <table class=\"slots\">").unwrap();
                writeln!(&mut output, "                        <thead>").unwrap();
                writeln!(&mut output, "                            <tr>").unwrap();
                writeln!(&mut output, "                                <th>Name</th>").unwrap();
                writeln!(
                    &mut output,
                    "                                <th>Range</th>"
                )
                .unwrap();
                writeln!(
                    &mut output,
                    "                                <th>Required</th>"
                )
                .unwrap();
                writeln!(
                    &mut output,
                    "                                <th>Description</th>"
                )
                .unwrap();
                writeln!(&mut output, "                            </tr>").unwrap();
                writeln!(&mut output, "                        </thead>").unwrap();
                writeln!(&mut output, "                        <tbody>").unwrap();

                for slot_name in &class.slots {
                    if let Some(slot) = schema.slots.get(slot_name) {
                        writeln!(&mut output, "                            <tr>").unwrap();
                        writeln!(
                            &mut output,
                            "                                <td><a href=\"#slot-{}\">{}</a></td>",
                            self.to_anchor(slot_name),
                            self.escape_html(slot_name)
                        )
                        .unwrap();
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            self.escape_html(slot.range.as_deref().unwrap_or("string"))
                        )
                        .unwrap();
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            if slot.required == Some(true) {
                                "✓"
                            } else {
                                ""
                            }
                        )
                        .unwrap();
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            self.escape_html(slot.description.as_deref().unwrap_or(""))
                        )
                        .unwrap();
                        writeln!(&mut output, "                            </tr>").unwrap();
                    }
                }

                writeln!(&mut output, "                        </tbody>").unwrap();
                writeln!(&mut output, "                    </table>").unwrap();
            }

            writeln!(&mut output, "                </div>").unwrap();
            writeln!(&mut output, "            </div>").unwrap();
        }

        writeln!(&mut output, "        </section>").unwrap();

        output
    }

    /// Generate slots section
    fn generate_slots(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        if schema.slots.is_empty() {
            return output;
        }

        writeln!(
            &mut output,
            "        <section id=\"slots\" class=\"section\">"
        )
        .unwrap();
        writeln!(&mut output, "            <h2>Slots</h2>").unwrap();

        for (slot_name, slot) in &schema.slots {
            writeln!(
                &mut output,
                "            <div id=\"slot-{}\" class=\"slot\">",
                self.to_anchor(slot_name)
            )
            .unwrap();
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(slot_name)
            )
            .unwrap();

            if let Some(desc) = &slot.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .unwrap();
            }

            // Slot properties table
            writeln!(&mut output, "                <table class=\"properties\">").unwrap();

            if let Some(range) = &slot.range {
                writeln!(
                    &mut output,
                    "                    <tr><th>Range:</th><td>{}</td></tr>",
                    self.escape_html(range)
                )
                .unwrap();
            }

            if slot.required == Some(true) {
                writeln!(
                    &mut output,
                    "                    <tr><th>Required:</th><td>Yes</td></tr>"
                )
                .unwrap();
            }

            if slot.multivalued == Some(true) {
                writeln!(
                    &mut output,
                    "                    <tr><th>Multivalued:</th><td>Yes</td></tr>"
                )
                .unwrap();
            }

            if let Some(pattern) = &slot.pattern {
                writeln!(
                    &mut output,
                    "                    <tr><th>Pattern:</th><td><code>{}</code></td></tr>",
                    self.escape_html(pattern)
                )
                .unwrap();
            }

            if let Some(minimum) = &slot.minimum_value {
                writeln!(
                    &mut output,
                    "                    <tr><th>Minimum:</th><td>{}</td></tr>",
                    self.escape_html(&minimum.to_string())
                )
                .unwrap();
            }

            if let Some(maximum) = &slot.maximum_value {
                writeln!(
                    &mut output,
                    "                    <tr><th>Maximum:</th><td>{}</td></tr>",
                    self.escape_html(&maximum.to_string())
                )
                .unwrap();
            }

            writeln!(&mut output, "                </table>").unwrap();
            writeln!(&mut output, "            </div>").unwrap();
        }

        writeln!(&mut output, "        </section>").unwrap();

        output
    }

    /// Generate enums section
    fn generate_enums(&self, schema: &SchemaDefinition) -> String {
        let mut output = String::new();

        if schema.enums.is_empty() {
            return output;
        }

        writeln!(
            &mut output,
            "        <section id=\"enums\" class=\"section\">"
        )
        .unwrap();
        writeln!(&mut output, "            <h2>Enumerations</h2>").unwrap();

        for (enum_name, enum_def) in &schema.enums {
            writeln!(
                &mut output,
                "            <div id=\"enum-{}\" class=\"enum\">",
                self.to_anchor(enum_name)
            )
            .unwrap();
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(enum_name)
            )
            .unwrap();

            if let Some(desc) = &enum_def.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .unwrap();
            }

            // Permissible values
            writeln!(&mut output, "                <h4>Values</h4>").unwrap();
            writeln!(&mut output, "                <ul class=\"enum-values\">").unwrap();

            for value in &enum_def.permissible_values {
                match value {
                    PermissibleValue::Simple(text) => {
                        writeln!(
                            &mut output,
                            "                    <li><code>{}</code></li>",
                            self.escape_html(text)
                        )
                        .unwrap();
                    }
                    PermissibleValue::Complex {
                        text, description, ..
                    } => {
                        writeln!(&mut output, "                    <li>").unwrap();
                        writeln!(
                            &mut output,
                            "                        <code>{}</code>",
                            self.escape_html(text)
                        )
                        .unwrap();
                        if let Some(desc) = description {
                            writeln!(
                                &mut output,
                                "                        <span class=\"value-desc\"> - {}</span>",
                                self.escape_html(desc)
                            )
                            .unwrap();
                        }
                        writeln!(&mut output, "                    </li>").unwrap();
                    }
                }
            }

            writeln!(&mut output, "                </ul>").unwrap();
            writeln!(&mut output, "            </div>").unwrap();
        }

        writeln!(&mut output, "        </section>").unwrap();

        output
    }

    /// Get embedded CSS styles
    fn get_css(&self) -> &'static str {
        let _ = self;
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
            margin: 0;
            padding: 0;
            display: flex;
            min-height: 100vh;
            background: #f5f5f5;
        }
        
        .sidebar {
            width: 250px;
            background: #2c3e50;
            color: white;
            padding: 20px;
            position: fixed;
            height: 100vh;
            overflow-y: auto;
        }
        
        .sidebar h2 {
            margin-top: 0;
            font-size: 1.2rem;
        }
        
        .sidebar ul {
            list-style: none;
            padding-left: 0;
        }
        
        .sidebar ul ul {
            padding-left: 20px;
        }
        
        .sidebar a {
            color: #ecf0f1;
            text-decoration: none;
            display: block;
            padding: 5px 0;
        }
        
        .sidebar a:hover {
            color: #3498db;
        }
        
        .content {
            margin-left: 270px;
            flex: 1;
            padding: 20px 40px;
            background: white;
            min-height: 100vh;
        }
        
        .section {
            margin-bottom: 40px;
        }
        
        h1, h2, h3, h4 {
            color: #2c3e50;
        }
        
        h1 {
            border-bottom: 2px solid #3498db;
            padding-bottom: 10px;
        }
        
        h2 {
            border-bottom: 1px solid #ecf0f1;
            padding-bottom: 8px;
            margin-top: 30px;
        }
        
        .description {
            color: #7f8c8d;
            font-style: italic;
            margin: 10px 0;
        }
        
        table {
            border-collapse: collapse;
            width: 100%;
            margin: 15px 0;
        }
        
        th, td {
            text-align: left;
            padding: 10px;
            border: 1px solid #ecf0f1;
        }
        
        th {
            background: #ecf0f1;
            font-weight: 600;
        }
        
        tr:nth-child(even) {
            background: #f9f9f9;
        }
        
        code {
            background: #f4f4f4;
            padding: 2px 6px;
            border-radius: 3px;
            font-family: "Consolas", "Monaco", monospace;
        }
        
        .badge {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.85em;
            font-weight: 600;
        }
        
        .badge.abstract {
            background: #9b59b6;
            color: white;
        }
        
        .class, .slot, .enum {
            background: #f9f9f9;
            border-left: 4px solid #3498db;
            padding: 15px;
            margin: 20px 0;
        }
        
        .enum-values {
            list-style: none;
            padding-left: 20px;
        }
        
        .enum-values li {
            margin: 5px 0;
        }
        
        .value-desc {
            color: #7f8c8d;
            font-size: 0.9em;
        }
        
        footer {
            margin-left: 270px;
            padding: 20px 40px;
            background: #ecf0f1;
            text-align: center;
            color: #7f8c8d;
            font-size: 0.9em;
        }
        
        .metadata table {
            max-width: 600px;
        }
        
        .properties {
            margin-top: 15px;
        }
        
        .slots {
            margin-top: 10px;
        }
        "#
    }

    /// Convert text to HTML anchor
    fn to_anchor(&self, text: &str) -> String {
        let _ = self;
        text.to_lowercase()
            .replace([' ', '_'], "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }

    /// Escape HTML special characters
    fn escape_html(&self, text: &str) -> String {
        let _ = self;
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

impl Default for HtmlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for HtmlGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate HTML documentation from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".html", ".htm"]
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        // Validate schema
        self.validate_schema(schema).await?;

        let mut output = String::new();
        let title = if schema.name.is_empty() {
            "LinkML Schema"
        } else {
            &schema.name
        };

        // Generate HTML document
        output.push_str(&self.generate_header(title, schema));
        output.push_str(&self.generate_overview(schema));
        output.push_str(&self.generate_classes(schema));
        output.push_str(&self.generate_slots(schema));
        output.push_str(&self.generate_enums(schema));

        // Add types section if implemented
        // output.push_str(&self.generate_types(schema)?);

        output.push_str(&self.generate_footer());

        // Create output
        let filename = format!(
            "{}.html",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        );

        let mut metadata = HashMap::new();
        metadata.insert("generator".to_string(), self.name.clone());
        metadata.insert("schema_name".to_string(), schema.name.clone());

        Ok(vec![GeneratedOutput {
            content: output,
            filename,
            metadata,
        }])
    }
}

impl CodeFormatter for HtmlGenerator {
    fn format_doc(&self, doc: &str, _indent: &IndentStyle, _level: usize) -> String {
        self.escape_html(doc)
    }

    fn format_list<T: AsRef<str>>(
        &self,
        items: &[T],
        _indent: &IndentStyle,
        _level: usize,
        separator: &str,
    ) -> String {
        items
            .iter()
            .map(|item| self.escape_html(item.as_ref()))
            .collect::<Vec<_>>()
            .join(separator)
    }

    fn escape_string(&self, s: &str) -> String {
        self.escape_html(s)
    }

    fn convert_identifier(&self, id: &str) -> String {
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_html_generation() {
        let generator = HtmlGenerator::new();

        let mut schema = SchemaDefinition::default();
        schema.id = "test".to_string();
        schema.name = "Test Schema".to_string();
        schema.description = Some("A test schema for HTML generation".to_string());

        // Add a class
        let mut class = ClassDefinition::default();
        class.name = "Person".to_string();
        class.description = Some("Represents a person".to_string());

        schema.classes.insert("Person".to_string(), class);

        let options = GeneratorOptions::new();
        let outputs = generator.generate(&schema, &options).await.unwrap();

        assert_eq!(outputs.len(), 1);
        let html = &outputs[0].content;

        // Check basic structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Test Schema - LinkML Documentation</title>"));
        assert!(html.contains("Test Schema"));
        assert!(html.contains("A test schema for HTML generation"));
        assert!(html.contains("Person"));
        assert!(html.contains("Represents a person"));
    }

    #[test]
    fn test_html_escaping() {
        let generator = HtmlGenerator::new();

        assert_eq!(
            generator.escape_html("Test <script>alert('XSS')</script>"),
            "Test &lt;script&gt;alert(&#39;XSS&#39;)&lt;/script&gt;"
        );

        assert_eq!(
            generator.escape_html("A & B < C > D"),
            "A &amp; B &lt; C &gt; D"
        );
    }

    #[test]
    fn test_anchor_conversion() {
        let generator = HtmlGenerator::new();

        assert_eq!(generator.to_anchor("Person Name"), "person-name");
        assert_eq!(generator.to_anchor("test_class"), "test-class");
        assert_eq!(generator.to_anchor("Test123!@#"), "test123");
    }
}
