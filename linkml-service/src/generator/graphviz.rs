//! Graphviz generator for `LinkML` schemas
//!
//! This module generates DOT format files that can be rendered by Graphviz
//! to visualize `LinkML` schemas as directed graphs. The generator supports
//! multiple diagram styles and customization options.

use bitflags::bitflags;
use linkml_core::prelude::*;
use std::collections::HashSet;
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorResult};

/// Graphviz diagram style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphvizStyle {
    /// Simple boxes and arrows
    Simple,
    /// UML-style class diagrams
    Uml,
    /// Entity-relationship style
    EntityRelationship,
    /// Hierarchical with inheritance focus
    Hierarchical,
}

/// Graphviz layout engine
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GraphvizLayout {
    /// Hierarchical layout (default)
    Dot,
    /// Spring model layout
    Neato,
    /// Spring model with overlap removal
    Fdp,
    /// Radial layout
    Twopi,
    /// Circular layout
    Circo,
}

bitflags! {
    /// Feature flags for Graphviz diagram generation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct GraphvizFeatures: u16 {
        /// Include slot details in the diagram
        const INCLUDE_SLOTS = 1 << 0;
        /// Include enumerations in the diagram
        const INCLUDE_ENUMS = 1 << 1;
        /// Include types in the diagram
        const INCLUDE_TYPES = 1 << 2;
        /// Show cardinality information
        const SHOW_CARDINALITY = 1 << 3;
        /// Show inheritance relationships
        const SHOW_INHERITANCE = 1 << 4;
        /// Show mixin relationships
        const SHOW_MIXINS = 1 << 5;
        /// Use color coding in the diagram
        const USE_COLORS = 1 << 6;

        /// Default feature set for typical usage
        const DEFAULT = Self::INCLUDE_SLOTS.bits() | Self::INCLUDE_ENUMS.bits()
                      | Self::SHOW_CARDINALITY.bits() | Self::SHOW_INHERITANCE.bits()
                      | Self::SHOW_MIXINS.bits() | Self::USE_COLORS.bits();
    }
}

/// Options for Graphviz generation
#[derive(Debug, Clone)]
pub struct GraphvizOptions {
    /// Diagram style
    pub style: GraphvizStyle,
    /// Layout engine
    pub layout: GraphvizLayout,
    /// Feature flags controlling what elements to include/show
    pub features: GraphvizFeatures,
    /// Rank direction (TB, BT, LR, RL)
    pub rankdir: String,
}

impl Default for GraphvizOptions {
    fn default() -> Self {
        Self {
            style: GraphvizStyle::Uml,
            layout: GraphvizLayout::Dot,
            features: GraphvizFeatures::DEFAULT,
            rankdir: "TB".to_string(),
        }
    }
}

/// Graphviz generator for schema visualization
pub struct GraphvizGenerator {
    /// Generation options
    options: GraphvizOptions,
}

impl GraphvizGenerator {
    /// Create a new Graphviz generator with default options
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: GraphvizOptions::default(),
        }
    }

    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: GraphvizOptions) -> Self {
        Self { options }
    }

    /// Set the diagram style
    #[must_use]
    pub fn with_style(mut self, style: GraphvizStyle) -> Self {
        self.options.style = style;
        self
    }

    /// Set the layout engine
    #[must_use]
    pub fn with_layout(mut self, layout: GraphvizLayout) -> Self {
        self.options.layout = layout;
        self
    }

    /// Generate DOT format output
    fn generate_dot(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        // Header
        writeln!(
            &mut output,
            "// Graphviz diagram generated from LinkML schema: {}",
            if schema.name.is_empty() {
                "unnamed"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "digraph LinkMLSchema {{")
            .map_err(Self::fmt_error_to_generator_error)?;

        // Graph attributes
        self.write_graph_attributes(&mut output)?;

        // Node attributes
        self.write_node_attributes(&mut output)?;

        // Edge attributes
        Self::write_edge_attributes(&mut output)?;

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate nodes for classes
        for (name, class_def) in &schema.classes {
            self.generate_class_node(&mut output, name, class_def, schema)?;
        }

        // Generate nodes for enums if included
        if self
            .options
            .features
            .contains(GraphvizFeatures::INCLUDE_ENUMS)
        {
            for (name, enum_def) in &schema.enums {
                self.generate_enum_node(&mut output, name, enum_def)?;
            }
        }

        // Generate nodes for types if included
        if self
            .options
            .features
            .contains(GraphvizFeatures::INCLUDE_TYPES)
        {
            for (name, type_def) in &schema.types {
                self.generate_type_node(&mut output, name, type_def)?;
            }
        }

        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate edges
        self.generate_edges(&mut output, schema)?;

        // Footer
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;

        Ok(output)
    }

    /// Write graph-level attributes
    fn write_graph_attributes(&self, output: &mut String) -> GeneratorResult<()> {
        writeln!(output, "    // Graph attributes").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    rankdir={};", self.options.rankdir)
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    charset=\"UTF-8\";").map_err(Self::fmt_error_to_generator_error)?;

        match self.options.layout {
            GraphvizLayout::Dot => writeln!(output, "    splines=ortho;")
                .map_err(Self::fmt_error_to_generator_error)?,
            GraphvizLayout::Neato => writeln!(output, "    overlap=false;")
                .map_err(Self::fmt_error_to_generator_error)?,
            GraphvizLayout::Fdp => writeln!(output, "    overlap=false; sep=\"+20\";")
                .map_err(Self::fmt_error_to_generator_error)?,
            _ => {}
        }

        if self.options.style == GraphvizStyle::Uml {
            writeln!(output, "    ranksep=1.0;").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "    nodesep=0.5;").map_err(Self::fmt_error_to_generator_error)?;
        }
        Ok(())
    }

    /// Write default node attributes
    fn write_node_attributes(&self, output: &mut String) -> GeneratorResult<()> {
        writeln!(
            output,
            "    
    // Node defaults"
        )
        .map_err(Self::fmt_error_to_generator_error)?;

        match self.options.style {
            GraphvizStyle::Simple => {
                writeln!(
                    output,
                    "    node [shape=box, style=rounded, fontname=\"Arial\"];"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
            GraphvizStyle::Uml => {
                writeln!(
                    output,
                    "    node [shape=record, fontname=\"Arial\", fontsize=10];"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }
            GraphvizStyle::EntityRelationship => {
                writeln!(output, "    node [shape=box, fontname=\"Arial\"];")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
            GraphvizStyle::Hierarchical => {
                writeln!(output, "    node [shape=box, style=\"rounded,filled\", fillcolor=lightblue, fontname=\"Arial\"];").map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        Ok(())
    }

    /// Write default edge attributes
    fn write_edge_attributes(output: &mut String) -> GeneratorResult<()> {
        writeln!(
            output,
            "    
    // Edge defaults"
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "    edge [fontname=\"Arial\", fontsize=9];")
            .map_err(Self::fmt_error_to_generator_error)?;
        Ok(())
    }

    /// Generate a class node
    fn generate_class_node(
        &self,
        output: &mut String,
        name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<String> {
        let node_id = Self::sanitize_id(name);

        match self.options.style {
            GraphvizStyle::Simple => {
                write!(output, "    {node_id} [label=\"{name}\"]")
                    .map_err(Self::fmt_error_to_generator_error)?;

                if self.options.features.contains(GraphvizFeatures::USE_COLORS)
                    && class_def.abstract_.unwrap_or(false)
                {
                    write!(output, " [style=\"dashed\"]")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                writeln!(output, ";").map_err(Self::fmt_error_to_generator_error)?;
            }
            GraphvizStyle::Uml => {
                write!(output, "    {node_id} [label=\"{{")
                    .map_err(Self::fmt_error_to_generator_error)?;

                // Class name compartment
                if class_def.abstract_.unwrap_or(false) {
                    write!(
                        output,
                        "\\<\\<abstract\\>\\>\
{name}"
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                } else {
                    write!(output, "{name}").map_err(Self::fmt_error_to_generator_error)?;
                }

                if self
                    .options
                    .features
                    .contains(GraphvizFeatures::INCLUDE_SLOTS)
                {
                    // Slots compartment
                    write!(output, "|").map_err(Self::fmt_error_to_generator_error)?;

                    let all_slots = self.collect_all_slots(name, class_def, schema);
                    let mut first = true;

                    for slot_name in &all_slots {
                        if let Some(slot_def) = schema.slots.get(slot_name) {
                            if !first {
                                write!(output, "\\l")
                                    .map_err(Self::fmt_error_to_generator_error)?; // left-aligned newline
                            }
                            first = false;

                            // Slot with type and cardinality
                            write!(
                                output,
                                "{}: {}",
                                slot_name,
                                slot_def.range.as_deref().unwrap_or("string")
                            )
                            .map_err(Self::fmt_error_to_generator_error)?;

                            if self
                                .options
                                .features
                                .contains(GraphvizFeatures::SHOW_CARDINALITY)
                            {
                                let cardinality = Self::get_cardinality(slot_def);
                                write!(output, " [{cardinality}]")
                                    .map_err(Self::fmt_error_to_generator_error)?;
                            }
                        }
                    }

                    if !all_slots.is_empty() {
                        write!(output, "\\l").map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                write!(output, "}}\"").map_err(Self::fmt_error_to_generator_error)?;

                if self.options.features.contains(GraphvizFeatures::USE_COLORS)
                    && class_def.abstract_.unwrap_or(false)
                {
                    write!(output, ", fillcolor=lightgray, style=filled")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                writeln!(output, "];").map_err(Self::fmt_error_to_generator_error)?;
            }
            _ => {
                // EntityRelationship and Hierarchical styles
                write!(output, "    {node_id} [label=\"{name}\"")
                    .map_err(Self::fmt_error_to_generator_error)?;

                if self.options.features.contains(GraphvizFeatures::USE_COLORS)
                    && class_def.abstract_.unwrap_or(false)
                {
                    write!(output, ", style=\"dashed,filled\", fillcolor=lightgray")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }

                writeln!(output, "];").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(String::new())
    }

    /// Generate an enum node
    fn generate_enum_node(
        &self,
        output: &mut String,
        name: &str,
        enum_def: &EnumDefinition,
    ) -> GeneratorResult<String> {
        let node_id = Self::sanitize_id(name);

        if self.options.style == GraphvizStyle::Uml {
            write!(
                output,
                "    {node_id} [label=\"{{\\<\\<enumeration\\>\\>\
{name}|"
            )
            .map_err(Self::fmt_error_to_generator_error)?;

            // List enum values
            let mut first = true;
            for pv in &enum_def.permissible_values {
                if !first {
                    write!(output, "\\l").map_err(Self::fmt_error_to_generator_error)?;
                }
                first = false;

                let value = match pv {
                    PermissibleValue::Simple(s) => s,
                    PermissibleValue::Complex { text, .. } => text,
                };
                write!(output, "{value}").map_err(Self::fmt_error_to_generator_error)?;
            }

            if !enum_def.permissible_values.is_empty() {
                write!(output, "\\l").map_err(Self::fmt_error_to_generator_error)?;
            }

            write!(output, "}}\"").map_err(Self::fmt_error_to_generator_error)?;

            if self.options.features.contains(GraphvizFeatures::USE_COLORS) {
                write!(output, ", fillcolor=lightyellow, style=filled")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(output, "];").map_err(Self::fmt_error_to_generator_error)?;
        } else {
            write!(output, "    {node_id} [label=\"{name} (enum)\"")
                .map_err(Self::fmt_error_to_generator_error)?;

            if self.options.features.contains(GraphvizFeatures::USE_COLORS) {
                write!(
                    output,
                    ", shape=diamond, fillcolor=lightyellow, style=filled"
                )
                .map_err(Self::fmt_error_to_generator_error)?;
            }

            writeln!(output, "];").map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(String::new())
    }

    /// Generate a type node
    fn generate_type_node(
        &self,
        output: &mut String,
        name: &str,
        type_def: &TypeDefinition,
    ) -> GeneratorResult<String> {
        let node_id = Self::sanitize_id(name);

        write!(output, "    {node_id} [label=\"{name}")
            .map_err(Self::fmt_error_to_generator_error)?;

        if let Some(base) = &type_def.base_type {
            write!(
                output,
                "\
({base})"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        write!(output, "\"").map_err(Self::fmt_error_to_generator_error)?;

        if self.options.features.contains(GraphvizFeatures::USE_COLORS) {
            write!(
                output,
                ", shape=ellipse, fillcolor=lightgreen, style=filled"
            )
            .map_err(Self::fmt_error_to_generator_error)?;
        }

        writeln!(output, "];").map_err(Self::fmt_error_to_generator_error)?;

        Ok(String::new())
    }

    /// Generate edges for relationships
    fn generate_edges(
        &self,
        output: &mut String,
        schema: &SchemaDefinition,
    ) -> GeneratorResult<()> {
        writeln!(output, "    // Relationships").map_err(Self::fmt_error_to_generator_error)?;

        // Inheritance edges
        if self
            .options
            .features
            .contains(GraphvizFeatures::SHOW_INHERITANCE)
        {
            for (name, class_def) in &schema.classes {
                if let Some(parent) = &class_def.is_a {
                    writeln!(
                        output,
                        "    {} -> {} [arrowhead=empty, style=solid];",
                        Self::sanitize_id(parent),
                        Self::sanitize_id(name)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Mixin edges
        if self
            .options
            .features
            .contains(GraphvizFeatures::SHOW_MIXINS)
        {
            for (name, class_def) in &schema.classes {
                for mixin in &class_def.mixins {
                    writeln!(
                        output,
                        "    {} -> {} [arrowhead=empty, style=dashed, label=\"mixin\"];",
                        Self::sanitize_id(mixin),
                        Self::sanitize_id(name)
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // Composition/aggregation edges (object-valued slots)
        for (class_name, class_def) in &schema.classes {
            let all_slots = self.collect_all_slots(class_name, class_def, schema);

            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name)
                    && let Some(range) = &slot_def.range
                    && schema.classes.contains_key(range)
                {
                    // This is an object reference
                    let label = if self
                        .options
                        .features
                        .contains(GraphvizFeatures::SHOW_CARDINALITY)
                    {
                        format!("{} [{}]", slot_name, Self::get_cardinality(slot_def))
                    } else {
                        slot_name.clone()
                    };

                    writeln!(
                        output,
                        "    {} -> {} [arrowhead=open, label=\"{}\"];",
                        Self::sanitize_id(class_name),
                        Self::sanitize_id(range),
                        label
                    )
                    .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        Ok(())
    }

    /// Collect all slots for a class including inherited ones
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

    /// Get cardinality notation for a slot
    fn get_cardinality(slot: &SlotDefinition) -> String {
        let min = i32::from(slot.required.unwrap_or(false));
        let max = if slot.multivalued.unwrap_or(false) {
            "*"
        } else {
            "1"
        };

        if min == 0 && max == "1" {
            "0..1".to_string()
        } else if min == 1 && max == "1" {
            "1".to_string()
        } else if min == 0 && max == "*" {
            "*".to_string()
        } else if min == 1 && max == "*" {
            "1..*".to_string()
        } else {
            format!("{min}..{max}")
        }
    }

    /// Sanitize identifier for DOT format
    fn sanitize_id(name: &str) -> String {
        // Replace non-alphanumeric characters with underscores
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }
}

impl Default for GraphvizGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl Generator for GraphvizGenerator {
    fn name(&self) -> &'static str {
        "graphviz"
    }

    fn description(&self) -> &'static str {
        "Generates Graphviz DOT format diagrams from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> std::result::Result<(), LinkMLError> {
        // Validate schema has required fields
        if schema.name.is_empty() {
            return Err(LinkMLError::SchemaValidationError {
                message: "Schema must have a name for Graphviz generation".to_string(),
                element: Some("schema.name".to_string()),
            });
        }

        // Validate DOT identifier requirements
        for (class_name, _class_def) in &schema.classes {
            // DOT identifiers should not contain special characters that break graphs
            if class_name.contains('"')
                || class_name.contains('{')
                || class_name.contains('}')
                || class_name.contains('[')
                || class_name.contains(']')
                || class_name.contains('<')
                || class_name.contains('>')
            {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Class name '{class_name}' contains characters that need escaping in DOT format"
                    ),
                    element: Some(format!("class.{class_name}")),
                });
            }
        }

        // Validate slot names
        for (slot_name, _slot_def) in &schema.slots {
            if slot_name.contains('"') || slot_name.contains('{') || slot_name.contains('}') {
                return Err(LinkMLError::SchemaValidationError {
                    message: format!(
                        "Slot name '{slot_name}' contains characters that need escaping in DOT format"
                    ),
                    element: Some(format!("slot.{slot_name}")),
                });
            }
        }

        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        let content = self
            .generate_dot(schema)
            .map_err(|e| LinkMLError::other(e.to_string()))?;
        Ok(content)
    }

    fn get_file_extension(&self) -> &'static str {
        "dot"
    }

    fn get_default_filename(&self) -> &'static str {
        "schema"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = "TestSchema".to_string();

        // Create a base class
        let mut animal_class = ClassDefinition::default();
        animal_class.abstract_ = Some(true);
        animal_class.slots = vec!["name".to_string()];
        schema.classes.insert("Animal".to_string(), animal_class);

        // Create a derived class
        let mut dog_class = ClassDefinition::default();
        dog_class.is_a = Some("Animal".to_string());
        dog_class.slots = vec!["breed".to_string()];
        schema.classes.insert("Dog".to_string(), dog_class);

        // Create slots
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        let mut breed_slot = SlotDefinition::default();
        breed_slot.range = Some("string".to_string());
        schema.slots.insert("breed".to_string(), breed_slot);

        schema
    }

    #[test]
    fn test_graphviz_generation() -> anyhow::Result<()> {
        let schema = create_test_schema();
        let generator = GraphvizGenerator::new();

        let result = generator
            .generate(&schema)
            .expect("should generate Graphviz output: {}");

        // Check content includes basic structure
        assert!(result.contains("digraph LinkMLSchema"));
        assert!(result.contains("Animal"));
        assert!(result.contains("Dog"));
        assert!(result.contains("->"));
        Ok(())
    }

    #[test]
    fn test_cardinality() {
        let mut slot = SlotDefinition::default();
        assert_eq!(GraphvizGenerator::get_cardinality(&slot), "0..1");

        slot.required = Some(true);
        assert_eq!(GraphvizGenerator::get_cardinality(&slot), "1");

        slot.required = Some(false);
        slot.multivalued = Some(true);
        assert_eq!(GraphvizGenerator::get_cardinality(&slot), "*");

        slot.required = Some(true);
        slot.multivalued = Some(true);
        assert_eq!(GraphvizGenerator::get_cardinality(&slot), "1..*");
    }

    #[test]
    fn test_sanitize_id() {
        assert_eq!(GraphvizGenerator::sanitize_id("SimpleClass"), "SimpleClass");
        assert_eq!(
            GraphvizGenerator::sanitize_id("Complex-Class"),
            "Complex_Class"
        );
        assert_eq!(
            GraphvizGenerator::sanitize_id("Class.With.Dots"),
            "Class_With_Dots"
        );
    }
}
