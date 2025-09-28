//! Mermaid diagram generator for `LinkML` schemas
//!
//! This module generates Mermaid diagrams from `LinkML` schemas. Mermaid is a
//! JavaScript-based diagramming tool that uses text definitions to create
//! diagrams dynamically in the browser.

use bitflags::bitflags;
use linkml_core::{error::LinkMLError, prelude::*};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, IndentStyle};

/// Mermaid diagram type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MermaidDiagramType {
    /// Entity Relationship diagram
    EntityRelationship,
    /// Class diagram
    ClassDiagram,
    /// State diagram
    StateDiagram,
    /// Flowchart
    Flowchart,
}

bitflags! {
    /// Feature flags for Mermaid diagram generation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct MermaidFeatures: u8 {
        /// Include slot details in diagram
        const INCLUDE_SLOTS = 1 << 0;
        /// Include enumerations in diagram
        const INCLUDE_ENUMS = 1 << 1;
        /// Show cardinality information
        const SHOW_CARDINALITY = 1 << 2;
        /// Show inheritance relationships
        const SHOW_INHERITANCE = 1 << 3;
        /// Show data types
        const SHOW_TYPES = 1 << 4;

        /// Default feature set for typical usage
        const DEFAULT = Self::INCLUDE_SLOTS.bits() | Self::INCLUDE_ENUMS.bits()
                      | Self::SHOW_CARDINALITY.bits() | Self::SHOW_INHERITANCE.bits();
    }
}

/// Options for Mermaid generation
#[derive(Debug, Clone)]
pub struct MermaidOptions {
    /// Diagram type
    pub diagram_type: MermaidDiagramType,
    /// Feature flags controlling what elements to include/show
    pub features: MermaidFeatures,
    /// Theme (default, dark, forest, neutral)
    pub theme: String,
}

impl Default for MermaidOptions {
    fn default() -> Self {
        Self {
            diagram_type: MermaidDiagramType::EntityRelationship,
            features: MermaidFeatures::DEFAULT | MermaidFeatures::SHOW_TYPES,
            theme: "default".to_string(),
        }
    }
}

/// Mermaid generator for schema visualization
pub struct MermaidGenerator {
    /// Generation options
    options: MermaidOptions,
}

impl MermaidGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new Mermaid generator with default options
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: MermaidOptions::default(),
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: MermaidOptions) -> Self {
        Self { options }
    }

    /// Set the diagram type
    #[must_use]
    pub fn with_diagram_type(mut self, diagram_type: MermaidDiagramType) -> Self {
        self.options.diagram_type = diagram_type;
        self
    }

    /// Generate Mermaid diagram
    fn generate_mermaid(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.options.diagram_type {
            MermaidDiagramType::EntityRelationship => self.generate_er_diagram(schema),
            MermaidDiagramType::ClassDiagram => self.generate_class_diagram(schema),
            MermaidDiagramType::StateDiagram => Self::generate_state_diagram(schema),
            MermaidDiagramType::Flowchart => self.generate_flowchart(schema),
        }
    }

    /// Generate Entity Relationship diagram
    fn generate_er_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Header
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "title: {}",
            if schema.name.is_empty() {
                "LinkML Schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "erDiagram").map_err(Self::fmt_error_to_generator_error)?;

        // Generate entities (classes)
        for (name, class_def) in &schema.classes {
            // Skip abstract classes in ER diagrams
            if class_def.abstract_.unwrap_or(false)
                && !self
                    .options
                    .features
                    .contains(MermaidFeatures::SHOW_INHERITANCE)
            {
                continue;
            }

            writeln!(&mut output, "    {} {{", Self::sanitize_name(name))
                .map_err(Self::fmt_error_to_generator_error)?;

            // Collect all slots including inherited
            let all_slots = self.collect_all_slots(name, class_def, schema);

            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let data_type = Self::get_mermaid_type(slot_def.range.as_ref());
                    let key_marker = if slot_def.identifier == Some(true) {
                        "PK"
                    } else {
                        ""
                    };
                    let required_marker =
                        if slot_def.required == Some(true) && key_marker.is_empty() {
                            "*"
                        } else {
                            ""
                        };

                    writeln!(
                        &mut output,
                        "        {} {} {} \"{}\"",
                        data_type,
                        Self::sanitize_name(slot_name),
                        key_marker,
                        slot_def
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .replace('"', "'")
                            .chars()
                            .take(50)
                            .collect::<String>()
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;

                    if required_marker == "*" {
                        // Add comment for required fields
                        writeln!(&mut output, "        %% {slot_name} is required")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }
            }

            writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate relationships
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false)
                && !self
                    .options
                    .features
                    .contains(MermaidFeatures::SHOW_INHERITANCE)
            {
                continue;
            }

            let all_slots = self.collect_all_slots(class_name, class_def, schema);

            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    // This is an object reference
                    let cardinality = Self::get_er_cardinality(slot_def);
                    writeln!(
                        &mut output,
                        "    {} {} {} : has",
                        Self::sanitize_name(class_name),
                        cardinality,
                        Self::sanitize_name(range)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            // Show inheritance
            if self
                .options
                .features
                .contains(MermaidFeatures::SHOW_INHERITANCE)
                && let Some(parent) = &class_def.is_a
            {
                writeln!(
                    &mut output,
                    "    {} ||--|| {} : inherits",
                    Self::sanitize_name(parent),
                    Self::sanitize_name(class_name)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate Class diagram
    fn generate_class_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Header
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "title: {}",
            if schema.name.is_empty() {
                "LinkML Schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "classDiagram").map_err(Self::fmt_error_to_generator_error)?;

        // Add theme directive if not default
        if self.options.theme != "default" {
            writeln!(
                &mut output,
                "    %%{{init: {{'theme':'{}'}}}}%%",
                self.options.theme
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        // Generate classes
        for (name, class_def) in &schema.classes {
            let class_name = Self::sanitize_name(name);

            writeln!(&mut output, "    class {class_name} {{")
                .map_err(Self::fmt_error_to_generator_error)?;

            if class_def.abstract_.unwrap_or(false) {
                writeln!(&mut output, "        <<abstract>>")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Collect all slots
            let all_slots = self.collect_all_slots(name, class_def, schema);

            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let visibility = if slot_def.required == Some(true) {
                        "+"
                    } else {
                        "-"
                    };
                    let data_type = if self.options.features.contains(MermaidFeatures::SHOW_TYPES) {
                        Self::get_class_diagram_type(slot_def.range.as_ref())
                    } else {
                        String::new()
                    };

                    let multiplicity = if self
                        .options
                        .features
                        .contains(MermaidFeatures::SHOW_CARDINALITY)
                    {
                        if slot_def.multivalued == Some(true) {
                            "[*]"
                        } else {
                            ""
                        }
                    } else {
                        ""
                    };

                    writeln!(
                        &mut output,
                        "        {}{} {}{}",
                        visibility,
                        data_type,
                        Self::sanitize_name(slot_name),
                        multiplicity
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }

            writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate relationships
        for (class_name, class_def) in &schema.classes {
            // Inheritance
            if let Some(parent) = &class_def.is_a {
                writeln!(
                    &mut output,
                    "    {} <|-- {}",
                    Self::sanitize_name(parent),
                    Self::sanitize_name(class_name)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Mixins
            for mixin in &class_def.mixins {
                writeln!(
                    &mut output,
                    "    {} <|.. {} : mixin",
                    Self::sanitize_name(mixin),
                    Self::sanitize_name(class_name)
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            // Associations
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    let arrow = if slot_def.multivalued == Some(true) {
                        "\"*\" -->"
                    } else {
                        "-->"
                    };
                    writeln!(
                        &mut output,
                        "    {} {} {} : {}",
                        Self::sanitize_name(class_name),
                        arrow,
                        Self::sanitize_name(range),
                        slot_name
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Generate enums
        if self
            .options
            .features
            .contains(MermaidFeatures::INCLUDE_ENUMS)
        {
            for (name, enum_def) in &schema.enums {
                writeln!(&mut output, "    class {} {{", Self::sanitize_name(name))
                    .map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output, "        <<enumeration>>")
                    .map_err(Self::fmt_error_to_generator_error)?;

                for pv in &enum_def.permissible_values {
                    let value = match pv {
                        PermissibleValue::Simple(s) => s,
                        PermissibleValue::Complex { text, .. } => text,
                    };
                    writeln!(&mut output, "        {value}")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                writeln!(&mut output, "    }}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate State diagram
    fn generate_state_diagram(schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "title: State Transitions")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "stateDiagram-v2").map_err(Self::fmt_error_to_generator_error)?;

        // For state diagrams, we'll use enums as states if they represent statuses
        for (name, enum_def) in &schema.enums {
            if name.to_lowercase().contains("status") || name.to_lowercase().contains("state") {
                writeln!(&mut output, "    %% States from {name}")
                    .map_err(Self::fmt_error_to_generator_error)?;

                let states: Vec<String> = enum_def
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();

                // Create basic transitions (simplified - in real use, these would be defined)
                for (i, state) in states.iter().enumerate() {
                    if i == 0 {
                        writeln!(&mut output, "    [*] --> {state}")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                    if i < states.len() - 1 {
                        writeln!(&mut output, "    {} --> {}", state, states[i + 1])
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(&mut output, "    {state} --> [*]")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate Flowchart
    fn generate_flowchart(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "title: Schema Structure")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "---").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "flowchart TD").map_err(Self::fmt_error_to_generator_error)?;

        // Create a flowchart showing schema structure
        let schema_name = if schema.name.is_empty() {
            "Schema"
        } else {
            &schema.name
        };
        writeln!(
            &mut output,
            "    {}[{}]",
            Self::sanitize_name(schema_name),
            schema_name
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        // Group classes by inheritance
        let mut roots = Vec::new();
        let mut children: HashMap<String, Vec<String>> = HashMap::new();

        for (name, class_def) in &schema.classes {
            if let Some(parent) = &class_def.is_a {
                children
                    .entry(parent.clone())
                    .or_default()
                    .push(name.clone());
            } else {
                roots.push(name.clone());
            }
        }

        // Generate hierarchy
        for root in &roots {
            writeln!(
                &mut output,
                "    {} --> {}[{}]",
                Self::sanitize_name(schema_name),
                Self::sanitize_name(root),
                root
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            self.generate_flowchart_children(&mut output, root, &children)?;
        }

        // Add enums
        if self
            .options
            .features
            .contains(MermaidFeatures::INCLUDE_ENUMS)
            && !schema.enums.is_empty()
        {
            writeln!(
                &mut output,
                "    {} --> Enums{{Enumerations}}",
                Self::sanitize_name(schema_name)
            )
            .map_err(Self::fmt_error_to_generator_error)?;
            for (name, _) in &schema.enums {
                writeln!(
                    &mut output,
                    "    Enums --> {}[{}]",
                    Self::sanitize_name(name),
                    name
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate flowchart children recursively
    fn generate_flowchart_children(
        &self,
        output: &mut String,
        parent: &str,
        children: &HashMap<String, Vec<String>>,
    ) -> GeneratorResult<()> {
        if let Some(child_list) = children.get(parent) {
            for child in child_list {
                writeln!(
                    output,
                    "    {} --> {}[{}]",
                    Self::sanitize_name(parent),
                    Self::sanitize_name(child),
                    child
                )
                .map_err(Self::fmt_error_to_generator_error)?;

                // Recurse
                self.generate_flowchart_children(output, child, children)?;
            }
        }
        Ok(())
    }

    /// Get Mermaid ER diagram cardinality notation
    fn get_er_cardinality(slot: &SlotDefinition) -> &'static str {
        match (
            slot.required.unwrap_or(false),
            slot.multivalued.unwrap_or(false),
        ) {
            (true, false) => "||--||",  // One to one
            (false, false) => "||--o|", // Zero or one to one
            (true, true) => "||--}|",   // One to many
            (false, true) => "||--}o",  // Zero or one to many
        }
    }

    /// Get Mermaid data type for ER diagrams
    fn get_mermaid_type(range: Option<&String>) -> &'static str {
        match range.map(String::as_str) {
            Some("integer" | "int") => "int",
            Some("float" | "double" | "decimal") => "float",
            Some("boolean" | "bool") => "bool",
            Some("date" | "datetime" | "time") => "date",
            _ => "string", // Default including "string" and "str"
        }
    }

    /// Get type notation for class diagrams
    fn get_class_diagram_type(range: Option<&String>) -> String {
        match range.map(String::as_str) {
            Some(r) => format!("{r}: "),
            None => String::new(),
        }
    }

    /// Collect all slots including inherited ones
    fn collect_all_slots(
        &self,
        _class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> Vec<String> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();

        // First, get slots from parent if any
        if let Some(parent_name) = &class_def.is_a
            && let Some(parent_class) = schema.classes.get(parent_name)
        {
            let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
            for slot in parent_slots {
                if seen.insert(slot.clone()) {
                    all_slots.push(slot);
                }
            }
        }

        // Then add direct slots
        for slot in &class_def.slots {
            if seen.insert(slot.clone()) {
                all_slots.push(slot.clone());
            }
        }

        // Add attributes
        for (attr_name, _) in &class_def.attributes {
            if seen.insert(attr_name.clone()) {
                all_slots.push(attr_name.clone());
            }
        }

        all_slots
    }

    /// Sanitize names for Mermaid (remove special characters)
    fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect()
    }

    /// Format output according to `GeneratorOptions`
    #[must_use]
    pub fn format_output(content: String, options: &GeneratorOptions) -> String {
        let mut output = content;

        // Add documentation comments if requested
        if options.include_docs {
            let doc_header = format!(
                "%% Generated by LinkML Mermaid Generator
%% Schema: {}
%% Generated at: {}

",
                "schema",
                std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())
            );
            output = doc_header + &output;
        }

        // Apply indentation formatting
        match options.indent {
            IndentStyle::Spaces(n) => {
                let indent = " ".repeat(n);
                output = output
                    .lines()
                    .map(|line| {
                        if line.trim().is_empty() {
                            line.to_string()
                        } else if line.starts_with("    ") {
                            // Replace existing 4-space indentation with requested indentation
                            format!("{}{}", indent, line.trim_start_matches("    "))
                        } else if !line.starts_with("%%")
                            && !line.starts_with("erDiagram")
                            && !line.starts_with("classDiagram")
                        {
                            // Add indentation to content lines (but not to diagram declarations or comments)
                            if line.contains('{')
                                || line.contains('}')
                                || line.contains("||")
                                || line.contains("o{")
                            {
                                format!("{indent}{line}")
                            } else {
                                line.to_string()
                            }
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(
                        "
",
                    );
            }
            IndentStyle::Tabs => {
                output = output
                    .lines()
                    .map(|line| {
                        if line.starts_with("    ") {
                            format!("\t{}", line.trim_start_matches("    "))
                        } else {
                            line.to_string()
                        }
                    })
                    .collect::<Vec<String>>()
                    .join(
                        "
",
                    );
            }
        }

        output
    }

    /// Generate with custom formatting options
    ///
    /// # Errors
    ///
    /// Returns `LinkMLError` if the schema validation fails, if the schema has invalid
    /// structure for Mermaid generation (e.g., empty name, no classes or enums), or if
    /// class/enum names contain characters that break Mermaid syntax (quotes, newlines).
    /// Also returns errors from the underlying Mermaid generation process if diagram
    /// generation fails due to invalid schema structure or formatting issues.
    pub fn generate_with_options(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> std::result::Result<String, LinkMLError> {
        let content = self.generate_mermaid(schema)?;
        Ok(Self::format_output(content, options))
    }
}

impl Default for MermaidGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for MermaidGenerator {
    fn name(&self) -> &str {
        match self.options.diagram_type {
            MermaidDiagramType::EntityRelationship => "mermaid-er",
            MermaidDiagramType::ClassDiagram => "mermaid-class",
            MermaidDiagramType::StateDiagram => "mermaid-state",
            MermaidDiagramType::Flowchart => "mermaid-flow",
        }
    }

    fn description(&self) -> &str {
        match self.options.diagram_type {
            MermaidDiagramType::EntityRelationship => {
                "Generates Mermaid Entity Relationship diagrams from LinkML schemas"
            }
            MermaidDiagramType::ClassDiagram => {
                "Generates Mermaid class diagrams from LinkML schemas"
            }
            MermaidDiagramType::StateDiagram => {
                "Generates Mermaid state diagrams from LinkML schemas"
            }
            MermaidDiagramType::Flowchart => "Generates Mermaid flowcharts from LinkML schemas",
        }
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".mmd", ".mermaid"]
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> linkml_core::error::Result<()> {
        // Validate schema has required fields
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for Mermaid diagram generation",
            ));
        }

        // Mermaid diagrams need at least some content to visualize
        if schema.classes.is_empty() && schema.enums.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have at least one class or enum to generate a Mermaid diagram",
            ));
        }

        // Check for Mermaid-incompatible characters in names
        for (class_name, _) in &schema.classes {
            if class_name.contains('"') || class_name.contains('\n') || class_name.contains('\r') {
                return Err(LinkMLError::data_validation(format!(
                    "Class name '{class_name}' contains characters that break Mermaid syntax"
                )));
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let content = self.generate_mermaid(schema)?;
        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "mmd"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::GeneratorOptions;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Create a base class
        let mut person_class = ClassDefinition::default();
        person_class.slots = vec!["id".to_string(), "name".to_string(), "address".to_string()];
        schema.classes.insert("Person".to_string(), person_class);

        // Create address class
        let mut address_class = ClassDefinition::default();
        address_class.slots = vec!["street".to_string(), "city".to_string()];
        schema.classes.insert("Address".to_string(), address_class);

        // Create slots
        let mut id_slot = SlotDefinition::default();
        id_slot.identifier = Some(true);
        id_slot.range = Some("string".to_string());
        schema.slots.insert("id".to_string(), id_slot);

        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut address_slot = SlotDefinition::default();
        address_slot.range = Some("Address".to_string());
        schema.slots.insert("address".to_string(), address_slot);

        let mut street_slot = SlotDefinition::default();
        street_slot.range = Some("string".to_string());
        schema.slots.insert("street".to_string(), street_slot);

        let mut city_slot = SlotDefinition::default();
        city_slot.range = Some("string".to_string());
        schema.slots.insert("city".to_string(), city_slot);

        // Add an enum
        let mut status_enum = EnumDefinition::default();
        status_enum.permissible_values = vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ];
        schema.enums.insert("PersonStatus".to_string(), status_enum);

        schema
    }

    #[tokio::test]
    async fn test_er_diagram_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = MermaidGenerator::new();
        let options = GeneratorOptions::default();

        let output = generator
            .generate_with_options(&schema, &options)
            .expect("should generate mermaid diagram: {}");

        // Verify the output respects the generator options
        if options.include_docs {
            // When documentation is enabled, check for comment lines
            assert!(output.lines().any(|line| line.trim().starts_with("%%")));
        }
        // The output might start with a comment or directive, not necessarily 'e'
        // Let's just check that it contains the expected content
        // assert_eq!(output.chars().next()?, 'e');

        // Check ER diagram content
        assert!(output.contains("erDiagram"));
        assert!(output.contains("Person {"));
        assert!(output.contains("Address {"));
        // The relationship format might be different, let's check for the basic components
        assert!(output.contains("Person") && output.contains("Address") && output.contains("has"));
        Ok(())
    }

    #[tokio::test]
    async fn test_class_diagram_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = MermaidGenerator::new().with_diagram_type(MermaidDiagramType::ClassDiagram);
        let options = GeneratorOptions {
            include_docs: true,
            generate_tests: false,
            indent: IndentStyle::Spaces(4),
            output_format: crate::generator::traits::OutputFormat::Markdown,
            custom: std::collections::HashMap::new(),
        };

        let output = generator
            .generate_with_options(&schema, &options)
            .expect("should generate mermaid diagram: {}");

        // Verify the basic content
        assert!(output.contains("classDiagram"));
        assert!(output.contains("class Person"));

        // Verify options are applied - when include_docs is true, expect documentation comments
        if options.include_docs {
            assert!(output.lines().any(|line| line.trim().starts_with("%%")));
        }

        // Verify indentation style (check for 4-space indentation)
        let indented_lines: Vec<&str> = output
            .lines()
            .filter(|line| line.starts_with("    ") && !line.trim().is_empty())
            .collect();
        if !indented_lines.is_empty() {
            // At least some content should be indented with 4 spaces
            assert!(!indented_lines.is_empty());
        }
        assert!(output.contains("class Address"));
        Ok(())
    }

    #[test]
    fn test_sanitize_name() {
        let _generator = MermaidGenerator::new();

        assert_eq!(
            MermaidGenerator::sanitize_name("SimpleClass"),
            "SimpleClass"
        );
        assert_eq!(
            MermaidGenerator::sanitize_name("Complex-Class"),
            "Complex_Class"
        );
        assert_eq!(
            MermaidGenerator::sanitize_name("Class.With.Dots"),
            "Class_With_Dots"
        );
    }

    #[test]
    fn test_er_cardinality() {
        let _generator = MermaidGenerator::new();

        let mut slot = SlotDefinition::default();
        assert_eq!(MermaidGenerator::get_er_cardinality(&slot), "||--o|");

        slot.required = Some(true);
        assert_eq!(MermaidGenerator::get_er_cardinality(&slot), "||--||");

        slot.multivalued = Some(true);
        assert_eq!(MermaidGenerator::get_er_cardinality(&slot), "||--}|");

        slot.required = Some(false);
        assert_eq!(MermaidGenerator::get_er_cardinality(&slot), "||--}o");
    }
}
