//! PlantUML generator for LinkML schemas
//!
//! This module generates PlantUML diagrams from LinkML schemas. PlantUML is a
//! text-based UML diagramming tool that supports multiple diagram types.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{Generator, GeneratorError, GeneratorOptions, GeneratorResult, GeneratedOutput};
use async_trait::async_trait;

/// PlantUML diagram type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlantUmlDiagramType {
    /// Class diagram (default)
    Class,
    /// Object diagram showing instances
    Object,
    /// Entity-Relationship diagram
    EntityRelationship,
    /// State diagram
    State,
    /// Mind map
    MindMap,
    /// Component diagram
    Component,
}

/// PlantUML skin parameters
#[derive(Debug, Clone)]
pub struct PlantUmlSkin {
    /// Background color
    pub background_color: String,
    /// Class background color
    pub class_background_color: String,
    /// Class border color
    pub class_border_color: String,
    /// Font name
    pub font_name: String,
    /// Font size
    pub font_size: u8,
    /// Arrow color
    pub arrow_color: String,
}

impl Default for PlantUmlSkin {
    fn default() -> Self {
        Self {
            background_color: "white".to_string(),
            class_background_color: "#FFFFCC".to_string(),
            class_border_color: "#000000".to_string(),
            font_name: "Arial".to_string(),
            font_size: 12,
            arrow_color: "#000000".to_string(),
        }
    }
}

/// Options for PlantUML generation
#[derive(Debug, Clone)]
pub struct PlantUmlOptions {
    /// Diagram type
    pub diagram_type: PlantUmlDiagramType,
    /// Include private slots (prefixed with -)
    pub include_private: bool,
    /// Include methods/operations
    pub include_methods: bool,
    /// Show full details
    pub detailed: bool,
    /// Use packages for namespaces
    pub use_packages: bool,
    /// Skin parameters
    pub skin: PlantUmlSkin,
    /// Show cardinality on relationships
    pub show_cardinality: bool,
    /// Direction (top to bottom, left to right)
    pub direction: String,
}

impl Default for PlantUmlOptions {
    fn default() -> Self {
        Self {
            diagram_type: PlantUmlDiagramType::Class,
            include_private: true,
            include_methods: false,
            detailed: true,
            use_packages: false,
            skin: PlantUmlSkin::default(),
            show_cardinality: true,
            direction: "top to bottom".to_string(),
        }
    }
}

/// PlantUML generator
pub struct PlantUmlGenerator {
    /// Generation options
    options: PlantUmlOptions,
}

impl PlantUmlGenerator {
    /// Convert fmt::Error to GeneratorError
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::new(std::io::ErrorKind::Other, e))
    }
    
    /// Create a new PlantUML generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: PlantUmlOptions::default(),
        }
    }
    
    /// Create with custom options
    #[must_use]
    pub fn with_options(options: PlantUmlOptions) -> Self {
        Self { options }
    }
    
    /// Set the diagram type
    #[must_use]
    pub fn with_diagram_type(mut self, diagram_type: PlantUmlDiagramType) -> Self {
        self.options.diagram_type = diagram_type;
        self
    }
    
    /// Generate PlantUML diagram
    fn generate_plantuml(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.options.diagram_type {
            PlantUmlDiagramType::Class => self.generate_class_diagram(schema),
            PlantUmlDiagramType::Object => self.generate_object_diagram(schema),
            PlantUmlDiagramType::EntityRelationship => self.generate_er_diagram(schema),
            PlantUmlDiagramType::State => self.generate_state_diagram(schema),
            PlantUmlDiagramType::MindMap => self.generate_mindmap(schema),
            PlantUmlDiagramType::Component => self.generate_component_diagram(schema),
        }
    }
    
    /// Generate class diagram
    fn generate_class_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startuml").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML class diagram for {}", 
            schema.name.as_deref().unwrap_or("LinkML Schema")).map_err(Self::fmt_error_to_generator_error)?;
        
        // Add title
        if let Some(name) = &schema.name {
            writeln!(&mut output, "title {}", name).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Apply skin parameters
        self.apply_skin(&mut output);
        
        // Set direction
        match self.options.direction.as_str() {
            "left to right" => writeln!(&mut output, "left to right direction").map_err(Self::fmt_error_to_generator_error)?,
            "top to bottom" | _ => {}, // default
        }
        
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Use package if enabled
        if self.options.use_packages {
            if let Some(name) = &schema.name {
                writeln!(&mut output, "package {} {{", name).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        // Generate classes
        for (class_name, class_def) in &schema.classes {
            self.generate_class(&mut output, class_name, class_def, schema)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Generate enums
        for (enum_name, enum_def) in &schema.enums {
            self.generate_enum(&mut output, enum_name, enum_def)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if self.options.use_packages {
            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Generate relationships
        self.generate_relationships(&mut output, schema)?;
        
        writeln!(&mut output, "@enduml").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate a class
    fn generate_class(
        &self,
        output: &mut String,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition
    ) -> GeneratorResult<()> {
        // Class declaration
        if class_def.abstract_.unwrap_or(false) {
            writeln!(output, "abstract class {} {{", class_name).map_err(Self::fmt_error_to_generator_error)?;
        } else {
            writeln!(output, "class {} {{", class_name).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add description as note
        if self.options.detailed {
            if let Some(desc) = &class_def.description {
                writeln!(output, "  .. {} ..", desc).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        // Collect all slots
        let all_slots = self.collect_all_slots(class_name, class_def, schema);
        
        // Group slots by visibility
        let mut public_slots = Vec::new();
        let mut private_slots = Vec::new();
        
        for slot_name in &all_slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                if slot_def.required.unwrap_or(false) {
                    public_slots.push((slot_name, slot_def));
                } else {
                    private_slots.push((slot_name, slot_def));
                }
            }
        }
        
        // Generate public slots
        for (slot_name, slot_def) in &public_slots {
            self.generate_slot(output, slot_name, slot_def, "+", schema)?;
        }
        
        // Generate private slots
        if self.options.include_private {
            for (slot_name, slot_def) in &private_slots {
                self.generate_slot(output, slot_name, slot_def, "-", schema)?;
            }
        }
        
        // Add methods if enabled
        if self.options.include_methods {
            writeln!(output, "  --").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "  +validate()").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(output, "  +to_json()").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        
        // Add note with metadata
        if self.options.detailed && (class_def.see_also.is_some() || class_def.notes.is_some()) {
            writeln!(output, "note right of {}", class_name).map_err(Self::fmt_error_to_generator_error)?;
            
            if let Some(see_also) = &class_def.see_also {
                writeln!(output, "  See also: {}", see_also.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            if let Some(notes) = &class_def.notes {
                writeln!(output, "  Notes: {}", notes.join("; ")).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            writeln!(output, "end note").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        Ok(())
    }
    
    /// Generate a slot/attribute
    fn generate_slot(
        &self,
        output: &mut String,
        slot_name: &str,
        slot_def: &SlotDefinition,
        visibility: &str,
        schema: &SchemaDefinition
    ) -> GeneratorResult<()> {
        write!(output, "  {}{}", visibility, slot_name).map_err(Self::fmt_error_to_generator_error)?;
        
        // Add type
        if let Some(range) = &slot_def.range {
            write!(output, " : {}", range).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add multiplicity
        if self.options.show_cardinality {
            let cardinality = self.get_cardinality(slot_def);
            write!(output, " [{}]", cardinality).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Add constraints as stereotypes
        if self.options.detailed {
            let mut stereotypes = Vec::new();
            
            if slot_def.identifier == Some(true) {
                stereotypes.push("id");
            }
            
            if slot_def.key == Some(true) {
                stereotypes.push("key");
            }
            
            if slot_def.readonly == Some(true) {
                stereotypes.push("readonly");
            }
            
            if !stereotypes.is_empty() {
                write!(output, " <<{}>>", stereotypes.join(", ")).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Generate relationships
    fn generate_relationships(&self, output: &mut String, schema: &SchemaDefinition) -> GeneratorResult<()> {
        writeln!(output, "' Relationships").map_err(Self::fmt_error_to_generator_error)?;
        
        for (class_name, class_def) in &schema.classes {
            // Inheritance
            if let Some(parent) = &class_def.is_a {
                writeln!(output, "{} --|> {}", class_name, parent).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            // Mixins
            for mixin in &class_def.mixins {
                writeln!(output, "{} ..|> {} : <<mixin>>", class_name, mixin).map_err(Self::fmt_error_to_generator_error)?;
            }
            
            // Associations
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    if let Some(range) = &slot_def.range {
                        if schema.classes.contains_key(range) {
                            // This is an object reference
                            let arrow = if slot_def.multivalued.unwrap_or(false) {
                                "\"*\" -->"
                            } else {
                                "-->"
                            };
                            
                            writeln!(output, "{} {} {} : {}", 
                                class_name, arrow, range, slot_name).map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Generate enum
    fn generate_enum(&self, output: &mut String, enum_name: &str, enum_def: &EnumDefinition) -> GeneratorResult<()> {
        writeln!(output, "enum {} {{", enum_name).map_err(Self::fmt_error_to_generator_error)?;
        
        if self.options.detailed {
            if let Some(desc) = &enum_def.description {
                writeln!(output, "  .. {} ..", desc).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        for pv in &enum_def.permissible_values {
            let (value, desc) = match pv {
                PermissibleValue::Simple(s) => (s.clone(), None),
                PermissibleValue::Complex { text, description, .. } => (text.clone(), description.clone()),
            };
            
            write!(output, "  {}", value).map_err(Self::fmt_error_to_generator_error)?;
            
            if self.options.detailed {
                if let Some(description) = desc {
                    write!(output, " -- {}", description).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            
            writeln!(output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(())
    }
    
    /// Generate object diagram
    fn generate_object_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startuml").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML object diagram").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "title Example Instances").map_err(Self::fmt_error_to_generator_error)?;
        
        self.apply_skin(&mut output);
        
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate example objects
        let mut instance_count = 0;
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            instance_count += 1;
            let instance_name = format!("{}_{}", class_name.to_lowercase(), instance_count);
            
            writeln!(&mut output, "object {} {{", instance_name).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "  = {} =", class_name).map_err(Self::fmt_error_to_generator_error)?;
            
            // Add example slot values
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots.iter().take(3).cloned().collect::<Vec<_>>() {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let example_value = self.get_example_value(&slot_def.range);
                    writeln!(&mut output, "  {} = {}", slot_name, example_value).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            
            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(&mut output, "@enduml").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate ER diagram
    fn generate_er_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startuml").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML Entity-Relationship diagram").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "!define ENTITY entity").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "!define TABLE(x) entity x <<table>>").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        
        // Generate entities
        for (class_name, class_def) in &schema.classes {
            if class_def.abstract_.unwrap_or(false) {
                continue;
            }
            
            writeln!(&mut output, "TABLE({}) {{", class_name).map_err(Self::fmt_error_to_generator_error)?;
            
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    let key_marker = if slot_def.identifier == Some(true) {
                        "*"
                    } else if slot_def.required == Some(true) {
                        "+"
                    } else {
                        ""
                    };
                    
                    let type_str = slot_def.range.as_deref().unwrap_or("string");
                    writeln!(&mut output, "  {}{} : {}", key_marker, slot_name, type_str).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
            
            writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Generate relationships
        for (class_name, class_def) in &schema.classes {
            let all_slots = self.collect_all_slots(class_name, class_def, schema);
            
            for slot_name in &all_slots {
                if let Some(slot_def) = schema.slots.get(slot_name) {
                    if let Some(range) = &slot_def.range {
                        if schema.classes.contains_key(range) {
                            let rel = if slot_def.multivalued.unwrap_or(false) {
                                "}o--||"
                            } else {
                                "||--||"
                            };
                            
                            writeln!(&mut output, "{} {} {} : has", 
                                class_name, rel, range).map_err(Self::fmt_error_to_generator_error)?;
                        }
                    }
                }
            }
        }
        
        writeln!(&mut output, "@enduml").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate state diagram
    fn generate_state_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startuml").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML state diagram").map_err(Self::fmt_error_to_generator_error)?;
        
        // Find enums that represent states
        for (enum_name, enum_def) in &schema.enums {
            if enum_name.to_lowercase().contains("state") || enum_name.to_lowercase().contains("status") {
                writeln!(&mut output, "title {} State Diagram", enum_name).map_err(Self::fmt_error_to_generator_error)?;
                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
                
                let states: Vec<String> = enum_def.permissible_values.iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();
                
                // Initial state
                writeln!(&mut output, "[*] --> {}", states[0]).map_err(Self::fmt_error_to_generator_error)?;
                
                // State transitions (simplified)
                for i in 0..states.len() {
                    if i < states.len() - 1 {
                        writeln!(&mut output, "{} --> {}", states[i], states[i + 1]).map_err(Self::fmt_error_to_generator_error)?;
                    }
                    
                    // Add state details
                    writeln!(&mut output, "{} : {}", states[i], states[i]).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                // Final state
                if !states.is_empty() {
                    writeln!(&mut output, "{} --> [*]", states.last().expect("checked states is not empty")).map_err(Self::fmt_error_to_generator_error)?;
                }
                
                break; // Only do first state enum
            }
        }
        
        writeln!(&mut output, "@enduml").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate mind map
    fn generate_mindmap(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startmindmap").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML mind map for schema structure").map_err(Self::fmt_error_to_generator_error)?;
        
        let schema_name = schema.name.as_deref().unwrap_or("Schema");
        writeln!(&mut output, "* {}", schema_name).map_err(Self::fmt_error_to_generator_error)?;
        
        // Classes branch
        if !schema.classes.is_empty() {
            writeln!(&mut output, "** Classes").map_err(Self::fmt_error_to_generator_error)?;
            for (class_name, class_def) in &schema.classes {
                let abstract_marker = if class_def.abstract_.unwrap_or(false) { " <<abstract>>" } else { "" };
                writeln!(&mut output, "*** {}{}", class_name, abstract_marker).map_err(Self::fmt_error_to_generator_error)?;
                
                // Show a few slots
                let all_slots = self.collect_all_slots(class_name, class_def, schema);
                for slot in all_slots.iter().take(3) {
                    writeln!(&mut output, "**** {}", slot).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }
        
        // Enums branch
        if !schema.enums.is_empty() {
            writeln!(&mut output, "** Enumerations").map_err(Self::fmt_error_to_generator_error)?;
            for (enum_name, enum_def) in &schema.enums {
                writeln!(&mut output, "*** {}", enum_name).map_err(Self::fmt_error_to_generator_error)?;
                for pv in enum_def.permissible_values.iter().take(3) {
                    let value = match pv {
                        PermissibleValue::Simple(s) => s,
                        PermissibleValue::Complex { text, .. } => text,
                    };
                    writeln!(&mut output, "**** {}", value).map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }
        
        // Types branch
        if !schema.types.is_empty() {
            writeln!(&mut output, "** Types").map_err(Self::fmt_error_to_generator_error)?;
            for (type_name, _) in schema.types.iter().take(5) {
                writeln!(&mut output, "*** {}", type_name).map_err(Self::fmt_error_to_generator_error)?;
            }
        }
        
        writeln!(&mut output, "@endmindmap").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Generate component diagram
    fn generate_component_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        
        writeln!(&mut output, "@startuml").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "'PlantUML component diagram").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output, "title Schema Components").map_err(Self::fmt_error_to_generator_error)?;
        
        // Main schema component
        let schema_name = schema.name.as_deref().unwrap_or("Schema");
        writeln!(&mut output, "package \"{}\" {{", schema_name).map_err(Self::fmt_error_to_generator_error)?;
        
        // Classes component
        if !schema.classes.is_empty() {
            writeln!(&mut output, "  component [Classes] as classes").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Enums component
        if !schema.enums.is_empty() {
            writeln!(&mut output, "  component [Enumerations] as enums").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Types component
        if !schema.types.is_empty() {
            writeln!(&mut output, "  component [Types] as types").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        // Slots component
        if !schema.slots.is_empty() {
            writeln!(&mut output, "  component [Slots] as slots").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(&mut output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        
        // Show dependencies
        if !schema.classes.is_empty() && !schema.slots.is_empty() {
            writeln!(&mut output, "classes --> slots : uses").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        if !schema.classes.is_empty() && !schema.types.is_empty() {
            writeln!(&mut output, "classes --> types : references").map_err(Self::fmt_error_to_generator_error)?;
        }
        
        writeln!(&mut output, "@enduml").map_err(Self::fmt_error_to_generator_error)?;
        
        Ok(output)
    }
    
    /// Apply skin parameters
    fn apply_skin(&self, output: &mut String) {
        writeln!(output, "skinparam backgroundColor {}", self.options.skin.background_color).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "skinparam class {{").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  BackgroundColor {}", self.options.skin.class_background_color).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  BorderColor {}", self.options.skin.class_border_color).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  FontName {}", self.options.skin.font_name).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "  FontSize {}", self.options.skin.font_size).map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "}}").map_err(Self::fmt_error_to_generator_error)?;
        writeln!(output, "skinparam ArrowColor {}", self.options.skin.arrow_color).map_err(Self::fmt_error_to_generator_error)?;
    }
    
    /// Get cardinality string
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
    
    /// Get example value for a type
    fn get_example_value(&self, range: &Option<String>) -> &'static str {
        match range.as_deref() {
            Some("string") => "\"Example\"",
            Some("integer") => "42",
            Some("float") | Some("double") => "3.14",
            Some("boolean") => "true",
            Some("date") => "2024-01-01",
            _ => "value",
        }
    }
    
    /// Collect all slots including inherited ones
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
}

impl Default for PlantUmlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Generator for PlantUmlGenerator {
    fn name(&self) -> &str {
        match self.options.diagram_type {
            PlantUmlDiagramType::Class => "plantuml",
            PlantUmlDiagramType::Object => "plantuml-object",
            PlantUmlDiagramType::EntityRelationship => "plantuml-er",
            PlantUmlDiagramType::State => "plantuml-state",
            PlantUmlDiagramType::MindMap => "plantuml-mindmap",
            PlantUmlDiagramType::Component => "plantuml-component",
        }
    }
    
    fn description(&self) -> &str {
        "Generates PlantUML diagrams from LinkML schemas"
    }
    
    fn file_extensions(&self) -> Vec<&str> {
        vec![".puml", ".plantuml", ".pu"]
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let content = self.generate_plantuml(schema)?;
        
        let filename = format!("{}.puml", 
            schema.name.as_deref().unwrap_or("schema"));
        
        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "plantuml".to_string());
        metadata.insert("diagram_type".to_string(), format!("{:?}", self.options.diagram_type).to_lowercase());
        
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
        
        // Base class
        let mut base_class = ClassDefinition::default();
        base_class.abstract_ = Some(true);
        base_class.slots = vec!["id".to_string()];
        schema.classes.insert("Base".to_string(), base_class);
        
        // Derived class
        let mut person_class = ClassDefinition::default();
        person_class.is_a = Some("Base".to_string());
        person_class.slots = vec!["name".to_string(), "age".to_string()];
        schema.classes.insert("Person".to_string(), person_class);
        
        // Define slots
        let mut id_slot = SlotDefinition::default();
        id_slot.identifier = Some(true);
        id_slot.range = Some("string".to_string());
        schema.slots.insert("id".to_string(), id_slot);
        
        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);
        
        let mut age_slot = SlotDefinition::default();
        age_slot.range = Some("integer".to_string());
        schema.slots.insert("age".to_string(), age_slot);
        
        // Add enum
        let mut status_enum = EnumDefinition::default();
        status_enum.permissible_values = vec![
            PermissibleValue::Simple("ACTIVE".to_string()),
            PermissibleValue::Simple("INACTIVE".to_string()),
        ];
        schema.enums.insert("Status".to_string(), status_enum);
        
        schema
    }
    
    #[tokio::test]
    async fn test_class_diagram_generation() {
        let schema = create_test_schema();
        let generator = PlantUmlGenerator::new();
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.map_err(Self::fmt_error_to_generator_error)?;
        assert_eq!(result.len(), 1);
        
        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.puml");
        
        // Check content
        assert!(output.content.contains("@startuml"));
        assert!(output.content.contains("@enduml"));
        assert!(output.content.contains("abstract class Base"));
        assert!(output.content.contains("class Person"));
        assert!(output.content.contains("Person --|> Base"));
    }
    
    #[tokio::test]
    async fn test_er_diagram_generation() {
        let schema = create_test_schema();
        let generator = PlantUmlGenerator::new()
            .with_diagram_type(PlantUmlDiagramType::EntityRelationship);
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.map_err(Self::fmt_error_to_generator_error)?;
        let output = &result[0];
        
        assert!(output.content.contains("!define ENTITY"));
        assert!(output.content.contains("TABLE(Person)"));
    }
    
    #[tokio::test]
    async fn test_mindmap_generation() {
        let schema = create_test_schema();
        let generator = PlantUmlGenerator::new()
            .with_diagram_type(PlantUmlDiagramType::MindMap);
        let options = GeneratorOptions::default();
        
        let result = generator.generate(&schema, &options).await.map_err(Self::fmt_error_to_generator_error)?;
        let output = &result[0];
        
        assert!(output.content.contains("@startmindmap"));
        assert!(output.content.contains("@endmindmap"));
        assert!(output.content.contains("** Classes"));
    }
}