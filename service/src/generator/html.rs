//! HTML documentation generation for `LinkML` schemas

use super::options::IndentStyle;
use super::traits::{CodeFormatter, Generator, GeneratorResult};
use linkml_core::prelude::*;
use std::fmt::Write;

/// HTML documentation generator for `LinkML` schemas
pub struct HtmlGenerator {
    /// Generator name
    name: String,
    /// Generator options
    options: super::traits::GeneratorOptions,
}

impl HtmlGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> super::traits::GeneratorError {
        super::traits::GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new HTML generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "html".to_string(),
            options: super::traits::GeneratorOptions::default(),
        }
    }
    /// Create a new HTML generator with options
    #[must_use]
    pub fn with_options(options: super::traits::GeneratorOptions) -> Self {
        Self {
            name: "html".to_string(),
            options,
        }
    }

    /// Generate HTML page header
    fn generate_header(&self, title: &str, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "<!DOCTYPE html>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "<html lang=\"en\">").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "<head>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    <meta charset=\"UTF-8\">")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "    <title>{} - LinkML Documentation</title>",
            self.escape_html(title)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Add embedded CSS
        writeln!(&mut output, "    <style>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "{}", Self::get_css()).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    </style>").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "</head>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "<body>").map_err(Self::fmt_error_to_generator_error)?;

        // Navigation
        writeln!(&mut output, "    <nav class=\"sidebar\">")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        <h2>Contents</h2>")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        <ul>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "            <li><a href=\"#overview\">Overview</a></li>"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        if !schema.classes.is_empty() {
            writeln!(&mut output, "            <li>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "                <a href=\"#classes\">Classes</a>"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "                <ul>")
                .map_err(Self::fmt_error_to_generator_error)?;
            for class_name in schema.classes.keys() {
                writeln!(
                    &mut output,
                    "                    <li><a href=\"#class-{}\">{}</a></li>",
                    Self::to_anchor(class_name),
                    self.escape_html(class_name)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
            writeln!(&mut output, "                </ul>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            </li>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        if !schema.slots.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#slots\">Slots</a></li>"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        if !schema.enums.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#enums\">Enumerations</a></li>"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        if !schema.types.is_empty() {
            writeln!(
                &mut output,
                "            <li><a href=\"#types\">Types</a></li>"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "        </ul>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    </nav>").map_err(Self::fmt_error_to_generator_error)?;

        writeln!(&mut output, "    <main class=\"content\">")
            .map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate HTML page footer
    fn generate_footer() -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "    </main>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    <footer>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "        <p>Generated by LinkML HTML Generator</p>"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "    </footer>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "</body>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "</html>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate overview section
    fn generate_overview(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "        <section id=\"overview\" class=\"section\">"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "            <h1>{}</h1>",
            self.escape_html(&schema.name)
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Only include description if documentation is enabled
        if self.options.include_docs
            && let Some(desc) = &schema.description
        {
            writeln!(
                &mut output,
                "            <p class=\"description\">{}</p>",
                self.escape_html(desc)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Schema metadata
        writeln!(&mut output, "            <div class=\"metadata\">")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "                <h3>Schema Information</h3>")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "                <table>")
            .map_err(Self::fmt_error_to_generator_error)?;

        if !schema.id.is_empty() {
            writeln!(
                &mut output,
                "                    <tr><th>ID:</th><td>{}</td></tr>",
                self.escape_html(&schema.id)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        if let Some(version) = &schema.version {
            writeln!(
                &mut output,
                "                    <tr><th>Version:</th><td>{}</td></tr>",
                self.escape_html(version)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
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
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "                </table>")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "            </div>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "        </section>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate classes section
    fn generate_classes(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        if schema.classes.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "        <section id=\"classes\" class=\"section\">"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "            <h2>Classes</h2>")
            .map_err(Self::fmt_error_to_generator_error)?;

        for (class_name, class) in &schema.classes {
            writeln!(
                &mut output,
                "            <div id=\"class-{}\" class=\"class\">",
                Self::to_anchor(class_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(class_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(desc) = &class.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Class properties
            writeln!(&mut output, "                <div class=\"properties\">")
                .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(parent) = &class.is_a {
                writeln!(&mut output, "                    <p><strong>Inherits from:</strong> <a href=\"#class-{}\">{}</a></p>",
                    Self::to_anchor(parent),
                    self.escape_html(parent)
                ).map_err(Self::fmt_error_to_generator_error)?;
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
                            Self::to_anchor(m),
                            self.escape_html(m)
                        ))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if class.abstract_ == Some(true) {
                writeln!(
                    &mut output,
                    "                    <p class=\"badge abstract\">Abstract</p>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Class slots
            if !class.slots.is_empty() {
                writeln!(&mut output, "                    <h4>Slots</h4>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                    <table class=\"slots\">")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                        <thead>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                            <tr>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                                <th>Name</th>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "                                <th>Range</th>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "                                <th>Required</th>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(
                    &mut output,
                    "                                <th>Description</th>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                            </tr>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                        </thead>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                        <tbody>")
                    .map_err(Self::fmt_error_to_generator_error)?;

                for slot_name in &class.slots {
                    if let Some(slot) = schema.slots.get(slot_name) {
                        writeln!(&mut output, "                            <tr>")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            &mut output,
                            "                                <td><a href=\"#slot-{}\">{}</a></td>",
                            Self::to_anchor(slot_name),
                            self.escape_html(slot_name)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            self.escape_html(slot.range.as_deref().unwrap_or("string"))
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            if slot.required == Some(true) {
                                "✓"
                            } else {
                                ""
                            }
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            &mut output,
                            "                                <td>{}</td>",
                            self.escape_html(slot.description.as_deref().unwrap_or(""))
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(&mut output, "                            </tr>")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(&mut output, "                        </tbody>")
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "                    </table>")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(&mut output, "                </div>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            </div>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "        </section>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate slots section
    fn generate_slots(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        if schema.slots.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "        <section id=\"slots\" class=\"section\">"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "            <h2>Slots</h2>")
            .map_err(Self::fmt_error_to_generator_error)?;

        for (slot_name, slot) in &schema.slots {
            writeln!(
                &mut output,
                "            <div id=\"slot-{}\" class=\"slot\">",
                Self::to_anchor(slot_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(slot_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(desc) = &slot.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Slot properties table
            writeln!(&mut output, "                <table class=\"properties\">")
                .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(range) = &slot.range {
                writeln!(
                    &mut output,
                    "                    <tr><th>Range:</th><td>{}</td></tr>",
                    self.escape_html(range)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if slot.required == Some(true) {
                writeln!(
                    &mut output,
                    "                    <tr><th>Required:</th><td>Yes</td></tr>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if slot.multivalued == Some(true) {
                writeln!(
                    &mut output,
                    "                    <tr><th>Multivalued:</th><td>Yes</td></tr>"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if let Some(pattern) = &slot.pattern {
                writeln!(
                    &mut output,
                    "                    <tr><th>Pattern:</th><td><code>{}</code></td></tr>",
                    self.escape_html(pattern)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if let Some(minimum) = &slot.minimum_value {
                writeln!(
                    &mut output,
                    "                    <tr><th>Minimum:</th><td>{}</td></tr>",
                    self.escape_html(&minimum.to_string())
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            if let Some(maximum) = &slot.maximum_value {
                writeln!(
                    &mut output,
                    "                    <tr><th>Maximum:</th><td>{}</td></tr>",
                    self.escape_html(&maximum.to_string())
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(&mut output, "                </table>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            </div>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "        </section>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Generate enums section
    fn generate_enums(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        if schema.enums.is_empty() {
            return Ok(output);
        }

        writeln!(
            &mut output,
            "        <section id=\"enums\" class=\"section\">"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "            <h2>Enumerations</h2>")
            .map_err(Self::fmt_error_to_generator_error)?;

        for (enum_name, enum_def) in &schema.enums {
            writeln!(
                &mut output,
                "            <div id=\"enum-{}\" class=\"enum\">",
                Self::to_anchor(enum_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(
                &mut output,
                "                <h3>{}</h3>",
                self.escape_html(enum_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            if let Some(desc) = &enum_def.description {
                writeln!(
                    &mut output,
                    "                <p class=\"description\">{}</p>",
                    self.escape_html(desc)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Permissible values
            writeln!(&mut output, "                <h4>Values</h4>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "                <ul class=\"enum-values\">")
                .map_err(Self::fmt_error_to_generator_error)?;

            for value in &enum_def.permissible_values {
                match value {
                    PermissibleValue::Simple(text) => {
                        writeln!(
                            &mut output,
                            "                    <li><code>{}</code></li>",
                            self.escape_html(text)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    PermissibleValue::Complex {
                        text, description, ..
                    } => {
                        writeln!(&mut output, "                    <li>")
                            .map_err(Self::fmt_error_to_generator_error)?;
                        writeln!(
                            &mut output,
                            "                        <code>{}</code>",
                            self.escape_html(text)
                        )
                        .map_err(Self::fmt_error_to_generator_error)?;
                        if let Some(desc) = description {
                            writeln!(
                                &mut output,
                                "                        <span class=\"value-desc\"> - {}</span>",
                                self.escape_html(desc)
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;
                        }
                        writeln!(&mut output, "                    </li>")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }

            writeln!(&mut output, "                </ul>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "            </div>")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output, "        </section>").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Get embedded CSS styles
    fn get_css() -> &'static str {
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
    fn to_anchor(text: &str) -> String {
        text.to_lowercase()
            .replace([' ', '_'], "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }

    /// Escape HTML special characters based on options
    fn escape_html(&self, text: &str) -> String {
        // Check if strict escaping is enabled via custom options
        let strict_mode = self
            .options
            .custom
            .get("strict_escaping")
            .is_none_or(|v| v == "true"); // Default to strict for security

        if strict_mode {
            // Full HTML entity escaping for maximum security
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#39;")
                .replace('/', "&#x2F;") // Also escape forward slash in strict mode
        } else {
            // Basic escaping only
            text.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        }
    }
}

impl Default for HtmlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for HtmlGenerator {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &'static str {
        "Generate HTML documentation from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Basic validation for HTML generation
        if schema.name.is_empty() {
            return Err(LinkMLError::SchemaValidationError {
                message: "Schema must have a name for HTML documentation".to_string(),
                element: Some("schema.name".to_string()),
            });
        }

        // Check for XSS-prone content in names
        for (class_name, _class_def) in &schema.classes {
            if class_name.contains('<')
                || class_name.contains('>')
                || class_name.contains("script")
                || class_name.contains("javascript:")
            {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Class name '{class_name}' contains potentially unsafe HTML characters"
                    ),
                    element: Some(format!("class.{class_name}")),
                });
            }
        }

        // Validate that we have at least some content to document
        if schema.classes.is_empty()
            && schema.slots.is_empty()
            && schema.types.is_empty()
            && schema.enums.is_empty()
        {
            return Err(LinkMLError::SchemaValidationError {
                message: "Schema must have at least one class, slot, type, or enum to generate documentation".to_string(),
                element: Some("schema".to_string())});
        }

        Ok(())
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".html", ".htm"]
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Validate schema
        self.validate_schema(schema)?;

        let mut output = String::new();
        let title = if schema.name.is_empty() {
            "LinkML Schema"
        } else {
            &schema.name
        };

        // Generate HTML document
        output.push_str(&self.generate_header(title, schema)?);
        output.push_str(&self.generate_overview(schema)?);
        output.push_str(&self.generate_classes(schema)?);
        output.push_str(&self.generate_slots(schema)?);
        output.push_str(&self.generate_enums(schema)?);

        // Add types section if implemented
        // output.push_str(&self.generate_types(schema)?);

        output.push_str(&Self::generate_footer()?);

        // Return the generated HTML content
        Ok(output)
    }

    fn get_file_extension(&self) -> &'static str {
        "html"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

impl CodeFormatter for HtmlGenerator {
    fn name(&self) -> &'static str {
        "html"
    }

    fn description(&self) -> &'static str {
        "Code formatter for html output with proper indentation and syntax"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec!["html", "htm"]
    }

    fn format_code(&self, code: &str) -> GeneratorResult<String> {
        // Basic formatting - just ensure consistent indentation
        let mut formatted = String::new();
        let indent = "    ";
        let mut indent_level: usize = 0;

        for line in code.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Decrease indent for closing braces
            if trimmed.starts_with('}') || trimmed.starts_with(']') || trimmed.starts_with(')') {
                indent_level = indent_level.saturating_sub(1);
            }

            // Add proper indentation
            formatted.push_str(&indent.repeat(indent_level));
            formatted.push_str(trimmed);
            formatted.push('\n');

            // Increase indent after opening braces
            if trimmed.ends_with('{') || trimmed.ends_with('[') || trimmed.ends_with('(') {
                indent_level += 1;
            }
        }

        Ok(formatted)
    }
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
    use linkml_core::types::{ClassDefinition, SchemaDefinition};

    #[tokio::test]
    async fn test_html_generation() -> anyhow::Result<()> {
        let generator = HtmlGenerator::new();

        let mut schema = SchemaDefinition {
            id: "test".to_string(),
            name: "Test Schema".to_string(),
            description: Some("A test schema for HTML generation".to_string()),
            ..Default::default()
        };

        // Add a class
        let class = ClassDefinition {
            name: "Person".to_string(),
            description: Some("Represents a person".to_string()),
            ..Default::default()
        };

        schema.classes.insert("Person".to_string(), class);

        let html = generator
            .generate(&schema)
            .expect("should generate HTML output: {}");

        // Check basic structure
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<title>Test Schema - LinkML Documentation</title>"));
        assert!(html.contains("Test Schema"));
        assert!(html.contains("A test schema for HTML generation"));
        assert!(html.contains("Person"));
        assert!(html.contains("Represents a person"));
        Ok(())
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
        assert_eq!(HtmlGenerator::to_anchor("Person Name"), "person-name");
        assert_eq!(HtmlGenerator::to_anchor("test_class"), "test-class");
        assert_eq!(HtmlGenerator::to_anchor("Test123!@#"), "test123");
    }
}
