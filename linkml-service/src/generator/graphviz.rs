//! Graphviz generator for LinkML schemas
//!
//! This module generates DOT format files that can be rendered by Graphviz
//! to visualize LinkML schemas as directed graphs. The generator supports
//! multiple diagram styles and customization options.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

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

/// Options for Graphviz generation
#[derive(Debug, Clone)]
pub struct GraphvizOptions {
    /// Diagram style
    pub style: GraphvizStyle,
    /// Layout engine
    pub layout: GraphvizLayout,
    /// Include slot details
    pub include_slots: bool,
    /// Include enumerations
    pub include_enums: bool,
    /// Include types
    pub include_types: bool,
    /// Show cardinality
    pub show_cardinality: bool,
    /// Show inheritance
    pub show_inheritance: bool,
    /// Show mixins
    pub show_mixins: bool,
    /// Use color coding
    pub use_colors: bool,
    /// Rank direction (TB, BT, LR, RL)
    pub rankdir: String,
}

impl Default for GraphvizOptions {
    fn default() -> Self {
        Self {
            style: GraphvizStyle::Uml,
            layout: GraphvizLayout::Dot,
            include_slots: true,
            include_enums: true,
            include_types: false,
            show_cardinality: true,
            show_inheritance: true,
            show_mixins: true,
            use_colors: true,
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
        writeln!(&mut output, "// Graphviz diagram generated from LinkML schema: {}", 
            schema.name.as_deref().unwrap_or("unnamed")).unwrap();
        writeln!(&mut output, "digraph LinkMLSchema {{").unwrap();
        
        // Graph attributes
        self.write_graph_attributes(&mut output);
        
        // Node attributes
        self.write_node_attributes(&mut output);
        
        // Edge attributes
        self.write_edge_attributes(&mut output);
        
        writeln!(&mut output).unwrap();
        
        // Generate nodes for classes
        for (name, class_def) in &schema.classes {
            self.generate_class_node(&mut output, name, class_def, schema)?;
        }
        
        // Generate nodes for enums if included
        if self.options.include_enums {
            for (name, enum_def) in &schema.enums {
                self.generate_enum_node(&mut output, name, enum_def)?;
            }
        }
        
        // Generate nodes for types if included
        if self.options.include_types {
            for (name, type_def) in &schema.types {
                self.generate_type_node(&mut output, name, type_def)?;
            }
        }
        
        writeln!(&mut output).unwrap();
        
        // Generate edges
        self.generate_edges(&mut output, schema)?;
        
        // Footer
        writeln!(&mut output, "}}").unwrap();
        
        Ok(output)
    }
    
    /// Write graph-level attributes
    fn write_graph_attributes(&self, output: &mut String) {
        writeln!(output, "    // Graph attributes").unwrap();
        writeln!(output, "    rankdir={};", self.options.rankdir).unwrap();
        writeln!(output, "    charset=\"UTF-8\";").unwrap();
        
        match self.options.layout {
            GraphvizLayout::Dot => writeln!(output, "    splines=ortho;").unwrap(),
            GraphvizLayout::Neato => writeln!(output, "    overlap=false;").unwrap(),
            GraphvizLayout::Fdp => writeln!(output, "    overlap=false; sep=\"+20\";").unwrap(),
            _ => {}
        }
        
        if self.options.style == GraphvizStyle::Uml {
            writeln!(output, "    ranksep=1.0;").unwrap();
            writeln!(output, "    nodesep=0.5;").unwrap();
        }
    }
    
    /// Write default node attributes
    fn write_node_attributes(&self, output: &mut String) {
        writeln!(output, "    \n    // Node defaults").unwrap();
        
        match self.options.style {
            GraphvizStyle::Simple => {
                writeln!(output, "    node [shape=box, style=rounded, fontname=\"Arial\"];").unwrap();
            }
            GraphvizStyle::Uml => {
                writeln!(output, "    node [shape=record, fontname=\"Arial\", fontsize=10];").unwrap();
            }
            GraphvizStyle::EntityRelationship => {
                writeln!(output, "    node [shape=box, fontname=\"Arial\"];").unwrap();
            }
            GraphvizStyle::Hierarchical => {
                writeln!(output, "    node [shape=box, style=\"rounded,filled\", fillcolor=lightblue, fontname=\"Arial\"];").unwrap();
            }
        }
    }
    
    /// Write default edge attributes
    fn write_edge_attributes(&self, output: &mut String) {
        writeln!(output, "    \n    // Edge defaults").unwrap();
        writeln!(output, "    edge [fontname=\"Arial\", fontsize=9];").unwrap();
    }
    
    /// Generate a class node
    fn generate_class_node(&self, output: &mut String, name: &str, class_def: &ClassDefinition, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let node_id = self.sanitize_id(name);
        
        match self.options.style {
            GraphvizStyle::Simple => {
                write!(output, "    {} [label=\"{}\"]", node_id, name).unwrap();
                
                if self.options.use_colors {
                    if class_def.abstract_.unwrap_or(false) {
                        write!(output, " [style=\"dashed\"]").unwrap();
                    }
                }
                
                writeln!(output, ";").unwrap();
            }
            GraphvizStyle::Uml => {
                write!(output, "    {} [label=\"{{", node_id).unwrap();
                
                // Class name compartment
                if class_def.abstract_.unwrap_or(false) {
                    write!(output, "\\<\\<abstract\\>\\>\\n{}", name).unwrap();
                } else {
                    write!(output, "{}", name).unwrap();
                }
                
                if self.options.include_slots {
                    // Slots compartment
                    write!(output, "|").unwrap();
                    
                    let all_slots = self.collect_all_slots(name, class_def, schema);
                    let mut first = true;
                    
                    for slot_name in &all_slots {
                        if let Some(slot_def) = schema.slots.get(slot_name) {
                            if !first {
                                write!(output, "\\l").unwrap(); // left-aligned newline
                            }
                            first = false;
                            
                            // Slot with type and cardinality
                            write!(output, "{}: {}", slot_name, 
                                slot_def.range.as_deref().unwrap_or("string")).unwrap();
                            
                            if self.options.show_cardinality {
                                let cardinality = self.get_cardinality(slot_def);
                                write!(output, " [{}]", cardinality).unwrap();
                            }
                        }
                    }
                    
                    if !all_slots.is_empty() {
                        write!(output, "\\l").unwrap();
                    }
                }
                
                write!(output, "}}\"").unwrap();
                
                if self.options.use_colors && class_def.abstract_.unwrap_or(false) {
                    write!(output, ", fillcolor=lightgray, style=filled").unwrap();
                }
                
                writeln!(output, "];").unwrap();
            }
            _ => {
                // EntityRelationship and Hierarchical styles
                write!(output, "    {} [label=\"{}\"", node_id, name).unwrap();
                
                if self.options.use_colors {
                    if class_def.abstract_.unwrap_or(false) {
                        write!(output, ", style=\"dashed,filled\", fillcolor=lightgray").unwrap();
                    }
                }
                
                writeln!(output, "];").unwrap();
            }
        }
        
        Ok(String::new())
    }
    
    /// Generate an enum node
    fn generate_enum_node(&self, output: &mut String, name: &str, enum_def: &EnumDefinition) -> GeneratorResult<String> {
        let node_id = self.sanitize_id(name);
        
        match self.options.style {
            GraphvizStyle::Uml => {
                write!(output, "    {} [label=\"{{\\<\\<enumeration\\>\\>\\n{}|", node_id, name).unwrap();
                
                // List enum values
                let mut first = true;
                for pv in &enum_def.permissible_values {
                    if !first {
                        write!(output, "\\l").unwrap();
                    }
                    first = false;
                    
                    let value = match pv {
                        PermissibleValue::Simple(s) => s,
                        PermissibleValue::Complex { text, .. } => text,
                    };
                    write!(output, "{}", value).unwrap();
                }
                
                if !enum_def.permissible_values.is_empty() {
                    write!(output, "\\l").unwrap();
                }
                
                write!(output, "}}\"").unwrap();
                
                if self.options.use_colors {
                    write!(output, ", fillcolor=lightyellow, style=filled").unwrap();
                }
                
                writeln!(output, "];").unwrap();
            }
            _ => {
                write!(output, "    {} [label=\"{} (enum)\"", node_id, name).unwrap();
                
                if self.options.use_colors {
                    write!(output, ", shape=diamond, fillcolor=lightyellow, style=filled").unwrap();
                }
                
                writeln!(output, "];").unwrap();
            }
        }
        
        Ok(String::new())
    }
    
    /// Generate a type node
    fn generate_type_node(&self, output: &mut String, name: &str, type_def: &TypeDefinition) -> GeneratorResult<String> {
        let node_id = self.sanitize_id(name);
        
        write!(output, "    {} [label=\"{}", node_id, name).unwrap();
        
        if let Some(base) = &type_def.typeof {
            write!(output, "\\n({})", base).unwrap();
        }
        
        write!(output, "\"").unwrap();
        
        if self.options.use_colors {
            write!(output, ", shape=ellipse, fillcolor=lightgreen, style=filled").unwrap();
        }
        
        writeln!(output, "];").unwrap();
        
        Ok(String::new())
    }
    
    /// Generate edges for relationships
    fn generate_edges(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "    // Relationships").unwrap();
        
        // Inheritance edges
        if self.options.show_inheritance {
            for (name, class_def) in &schema.classes {
                if let Some(parent) = &class_def.is_a {
                    writeln!(output, "    {} -> {} [arrowhead=empty, style=solid];", 
                        self.sanitize_id(parent), self.sanitize_id(name)).unwrap();
                }
            }
        }
        
        // Mixin edges
        if self.options.show_mixins {
            for (name, class_def) in &schema.classes {
                for mixin in &class_def.mixins {
                    writeln!(output, "    {} -> {} [arrowhead=empty, style=dashed, label=\"mixin\"];", 
                        self.sanitize_id(mixin), self.sanitize_id(name)).unwrap();
                }
            }
        }
        
        // Composition/aggregation edges (object-valued slots)
        for (class_name, class_def) in &schema.classes {
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    if let Some(range) = &slot_def.range {
                        if schema.classes.contains_key(range) {
                            // This is an object reference
                            let label = if self.options.show_cardinality {
                                format!("{} [{}]", slot_name, self.get_cardinality(slot_def))
                            } else {
                                slot_name.clone()
                            };
                            
                            writeln!(output, "    {} -> {} [arrowhead=open, label=\"{}\"];", 
                                self.sanitize_id(class_name), 
                                self.sanitize_id(range),
                                label).unwrap();
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Collect all slots for a class including inherited ones
    fn collect_all_slots(&self, _class_name: &str, class_def: &ClassDefinition, schema: &SchemaDefinition) -> Vec<String> {
        let mut all_slots = Vec::new();
        let mut seen = HashSet::new();
        
        // First, get slots from parent if any
        if let Some(parent_name) = &class_def.is_a {
            if let Some(parent_class) = schema.classes.get(parent_name) {
                let parent_slots = self.collect_all_slots(parent_name, parent_class, schema);
                for slot in parent_slots {
                    if seen.insert(slot.clone()) {
                        all_slots.push(slot);
                    }
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
    fn get_cardinality(&self, slot: &SlotDefinition) -> String {
        let min = if slot.required.unwrap_or(false) { 1 } else { 0 };
        let max = if slot.multivalued.unwrap_or(false) { "*" } else { "1" };
        
        if min == 0 && max == "1" {
            "0..1".to_string()
        } else if min == 1 && max == "1" {
            "1".to_string()
        } else if min == 0 && max == "*" {
            "*".to_string()
        } else if min == 1 && max == "*" {
            "1..*".to_string()
        } else {
            format!("{}..{}", min, max)
        }
    }
    
    /// Sanitize identifier for DOT format
    fn sanitize_id(&self, name: &str) -> String {
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

#[async_trait]
impl Generator for GraphvizGenerator {
    fn name(&self) -> &str {
        "graphviz"
    }
    
    fn description(&self) -> &str {
        "Generates Graphviz DOT format diagrams from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".dot", ".gv"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let content = self.generate_dot(schema)?;
        
        let filename = format!("{}.dot", 
            schema.name.as_deref().unwrap_or("schema"));
        
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "dot".to_string());
        metadata.insert("layout".to_string(), format!("{:?}", self.options.layout).to_lowercase());
        metadata.insert("style".to_string(), format!("{:?}", self.options.style).to_lowercase());
        
        Ok(vec![GeneratedOutput {
            filename,
            content,
            metadata,
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition::default();
        schema.name = Some("TestSchema".to_string());
        
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
    
    #[tokio::test]
    async fn test_graphviz_generation() {
        let schema = create_test_schema();
        let generator = GraphvizGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.unwrap();
        assert_eq!(result.len(), 1);
        
        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.dot");
        
        // Check content includes basic structure
        assert!(output.content.contains("digraph LinkMLSchema"));
        assert!(output.content.contains("Animal"));
        assert!(output.content.contains("Dog"));
        assert!(output.content.contains("->"));
    }
    
    #[test]
    fn test_cardinality() {
        let generator = GraphvizGenerator::new();
        
        let mut slot = SlotDefinition::default();
        assert_eq!(generator.get_cardinality(&slot), "0..1");
        
        slot.required = Some(true);
        assert_eq!(generator.get_cardinality(&slot), "1");
        
        slot.required = Some(false);
        slot.multivalued = Some(true);
        assert_eq!(generator.get_cardinality(&slot), "*");
        
        slot.required = Some(true);
        slot.multivalued = Some(true);
        assert_eq!(generator.get_cardinality(&slot), "1..*");
    }
    
    #[test]
    fn test_sanitize_id() {
        let generator = GraphvizGenerator::new();
        
        assert_eq!(generator.sanitize_id("SimpleClass"), "SimpleClass");
        assert_eq!(generator.sanitize_id("Complex-Class"), "Complex_Class");
        assert_eq!(generator.sanitize_id("Class.With.Dots"), "Class_With_Dots");
    }
}