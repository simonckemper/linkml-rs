//! yUML generator for `LinkML` schemas
//!
//! This module generates yUML diagrams from `LinkML` schemas. yUML is a simple
//! online tool for creating UML diagrams using a text-based syntax.

use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::traits::{
    AsyncGenerator, GeneratedOutput, Generator, GeneratorError, GeneratorOptions, GeneratorResult,
};
use async_trait::async_trait;
use linkml_core::error::LinkMLError;

/// yUML diagram type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YumlDiagramType {
    /// Class diagram (default)
    Class,
    /// Use case diagram
    UseCase,
    /// Activity diagram
    Activity,
}

/// Options for yUML generation
#[derive(Debug, Clone)]
pub struct YumlOptions {
    /// Diagram type
    pub diagram_type: YumlDiagramType,
    /// Include slots in class diagrams
    pub include_slots: bool,
    /// Show inheritance relationships
    pub show_inheritance: bool,
    /// Show associations
    pub show_associations: bool,
    /// Diagram style (plain, scruffy, nofunky)
    pub style: String,
    /// Direction (LR, TB, RL, BT)
    pub direction: String,
}

impl Default for YumlOptions {
    fn default() -> Self {
        Self {
            diagram_type: YumlDiagramType::Class,
            include_slots: true,
            show_inheritance: true,
            show_associations: true,
            style: "plain".to_string(),
            direction: "TB".to_string(),
        }
    }
}

/// yUML generator for simple UML diagrams
pub struct YumlGenerator {
    /// Generation options
    options: YumlOptions,
}

impl YumlGenerator {
    /// Convert `fmt::Error` to `GeneratorError`
    fn fmt_error_to_generator_error(e: std::fmt::Error) -> GeneratorError {
        GeneratorError::Io(std::io::Error::other(e))
    }

    /// Create a new yUML generator
    #[must_use]
    pub fn new() -> Self {
        Self {
            options: YumlOptions::default(),
        }
    }

    /// Create with custom options
    #[must_use]
    pub fn with_options(options: YumlOptions) -> Self {
        Self { options }
    }

    /// Set the diagram type
    #[must_use]
    pub fn with_diagram_type(mut self, diagram_type: YumlDiagramType) -> Self {
        self.options.diagram_type = diagram_type;
        self
    }

    /// Generate yUML diagram
    fn generate_yuml(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        match self.options.diagram_type {
            YumlDiagramType::Class => self.generate_class_diagram(schema),
            YumlDiagramType::UseCase => self.generate_use_case_diagram(schema),
            YumlDiagramType::Activity => self.generate_activity_diagram(schema),
        }
    }

    /// Generate class diagram
    fn generate_class_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();
        let mut relationships = Vec::new();

        // Comment with diagram info
        writeln!(
            &mut output,
            "// yUML class diagram for {}",
            if schema.name.is_empty() {
                "LinkML Schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "// Paste at: https://yuml.me/diagram/{}/{}/class",
            self.options.style, self.options.direction
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate class definitions
        for (name, class_def) in &schema.classes {
            let mut class_str = String::new();
            write!(&mut class_str, "[{name}").map_err(Self::fmt_error_to_generator_error)?;

            if self.options.include_slots {
                // Collect all slots
                let all_slots = self.collect_all_slots(name, class_def, schema);

                if !all_slots.is_empty() {
                    write!(&mut class_str, "|").map_err(Self::fmt_error_to_generator_error)?;

                    for (i, slot_name) in all_slots.iter().enumerate() {
                        if i > 0 {
                            write!(&mut class_str, ";")
                                .map_err(Self::fmt_error_to_generator_error)?;
                        }

                        if let Some(slot_def) = schema.slots.get(slot_name) {
                            // Add visibility marker
                            if slot_def.required == Some(true) {
                                write!(&mut class_str, "+")
                                    .map_err(Self::fmt_error_to_generator_error)?;
                            } else {
                                write!(&mut class_str, "-")
                                    .map_err(Self::fmt_error_to_generator_error)?;
                            }

                            write!(&mut class_str, "{slot_name}")
                                .map_err(Self::fmt_error_to_generator_error)?;

                            // Add type if available
                            if let Some(range) = &slot_def.range {
                                write!(&mut class_str, ":{range}")
                                    .map_err(Self::fmt_error_to_generator_error)?;
                            }

                            // Add multiplicity
                            if slot_def.multivalued == Some(true) {
                                write!(&mut class_str, " *")
                                    .map_err(Self::fmt_error_to_generator_error)?;
                            }
                        }
                    }
                }
            }

            // Add stereotype for abstract classes
            if class_def.abstract_ == Some(true) {
                write!(&mut class_str, "|<<abstract>>")
                    .map_err(Self::fmt_error_to_generator_error)?;
            }

            write!(&mut class_str, "]").map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "{class_str}").map_err(Self::fmt_error_to_generator_error)?;

            // Collect inheritance relationships
            if self.options.show_inheritance {
                if let Some(parent) = &class_def.is_a {
                    relationships.push(format!("[{parent}]^-[{name}]"));
                }

                // Mixins (interface realization)
                for mixin in &class_def.mixins {
                    relationships.push(format!("[{mixin}]^-.-[{name}]"));
                }
            }

            // Collect associations
            if self.options.show_associations {
                let all_slots = self.collect_all_slots(name, class_def, schema);

                for slot_name in &all_slots {
                    if let Some(slot_def) = schema.slots.get(slot_name)
                        && let Some(range) = &slot_def.range
                        && schema.classes.contains_key(range)
                    {
                        // This is an object reference
                        let arrow = if slot_def.multivalued == Some(true) {
                            format!("[{name}]-{slot_name}*>[{range}]")
                        } else {
                            format!("[{name}]-{slot_name}>[{range}]")
                        };
                        relationships.push(arrow);
                    }
                }
            }
        }

        // Add relationships
        if !relationships.is_empty() {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            for rel in relationships {
                writeln!(&mut output, "{rel}").map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        // Add notes about enums if any
        if !schema.enums.is_empty() {
            writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "// Enumerations:")
                .map_err(Self::fmt_error_to_generator_error)?;
            for (name, enum_def) in &schema.enums {
                let values: Vec<String> = enum_def
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();
                writeln!(&mut output, "// {} enum: {}", name, values.join(", "))
                    .map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        Ok(output)
    }

    /// Generate use case diagram
    fn generate_use_case_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(
            &mut output,
            "// yUML use case diagram for {}",
            if schema.name.is_empty() {
                "LinkML Schema"
            } else {
                &schema.name
            }
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "// Paste at: https://yuml.me/diagram/{}/{}/usecase",
            self.options.style, self.options.direction
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Create actors from classes with certain patterns
        let mut actors = Vec::new();
        let mut use_cases = Vec::new();

        for (name, class_def) in &schema.classes {
            // Heuristic: classes ending with User, Actor, System are actors
            if name.ends_with("User")
                || name.ends_with("Actor")
                || name.ends_with("Member")
                || name.ends_with("Customer")
            {
                actors.push(name.clone());
            } else if !class_def.abstract_.unwrap_or(false) {
                // Non-abstract classes can be use cases
                let use_case_name = format!("Manage {name}");
                use_cases.push(use_case_name.clone());

                // Create relationships
                for actor in &actors {
                    writeln!(&mut output, "[{actor}]-({use_case_name})")
                        .map_err(Self::fmt_error_to_generator_error)?;
                }
            }
        }

        // If no actors found, create a generic one
        if actors.is_empty() && !use_cases.is_empty() {
            writeln!(&mut output, "[User]-(Manage Schema)")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
    }

    /// Generate activity diagram
    fn generate_activity_diagram(&self, schema: &SchemaDefinition) -> GeneratorResult<String> {
        let mut output = String::new();

        writeln!(&mut output, "// yUML activity diagram")
            .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(
            &mut output,
            "// Paste at: https://yuml.me/diagram/{}/{}/activity",
            self.options.style, self.options.direction
        )
        .map_err(Self::fmt_error_to_generator_error)?;
        writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;

        // Generate a simple workflow based on enums (if they represent states)
        let mut has_workflow = false;

        for (name, enum_def) in &schema.enums {
            if name.to_lowercase().contains("status") || name.to_lowercase().contains("state") {
                has_workflow = true;
                writeln!(&mut output, "// Workflow for {name}")
                    .map_err(Self::fmt_error_to_generator_error)?;

                let states: Vec<String> = enum_def
                    .permissible_values
                    .iter()
                    .map(|pv| match pv {
                        PermissibleValue::Simple(s) => s.clone(),
                        PermissibleValue::Complex { text, .. } => text.clone(),
                    })
                    .collect();

                // Create a simple linear workflow
                writeln!(&mut output, "(start)").map_err(Self::fmt_error_to_generator_error)?;

                for (i, state) in states.iter().enumerate() {
                    if i == 0 {
                        writeln!(&mut output, "(start)->|begin|<{state}>;")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }

                    if i < states.len() - 1 {
                        writeln!(&mut output, "<{}>->|next|<{}>;", state, states[i + 1])
                            .map_err(Self::fmt_error_to_generator_error)?;
                    } else {
                        writeln!(&mut output, "<{state}>->|complete|(end)")
                            .map_err(Self::fmt_error_to_generator_error)?;
                    }
                }

                writeln!(&mut output).map_err(Self::fmt_error_to_generator_error)?;
            }
        }

        if !has_workflow {
            // Create a generic workflow
            writeln!(&mut output, "(start)->|create|<Draft>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "<Draft>->|review|<Review>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "<Review>->|approve|<Approved>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "<Review>->|reject|<Draft>")
                .map_err(Self::fmt_error_to_generator_error)?;
            writeln!(&mut output, "<Approved>->|publish|(end)")
                .map_err(Self::fmt_error_to_generator_error)?;
        }

        Ok(output)
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
}

impl Default for YumlGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AsyncGenerator for YumlGenerator {
    fn name(&self) -> &str {
        match self.options.diagram_type {
            YumlDiagramType::Class => "yuml",
            YumlDiagramType::UseCase => "yuml-usecase",
            YumlDiagramType::Activity => "yuml-activity",
        }
    }

    fn description(&self) -> &'static str {
        "Generates yUML diagrams from LinkML schemas"
    }

    fn file_extensions(&self) -> Vec<&str> {
        vec![".yuml", ".txt"]
    }

    async fn validate_schema(&self, schema: &SchemaDefinition) -> GeneratorResult<()> {
        // Validate schema has required fields
        if schema.name.is_empty() {
            return Err(GeneratorError::SchemaValidation(
                "Schema must have a name for yUML generation".to_string(),
            ));
        }

        // yUML diagrams need at least some content to visualize
        if schema.classes.is_empty() && schema.enums.is_empty() {
            return Err(GeneratorError::SchemaValidation(
                "Schema must have at least one class or enum to generate a yUML diagram"
                    .to_string(),
            ));
        }

        Ok(())
    }

    async fn generate(
        &self,
        schema: &SchemaDefinition,
        _options: &GeneratorOptions,
    ) -> GeneratorResult<Vec<GeneratedOutput>> {
        let content = self.generate_yuml(schema)?;

        let filename = format!(
            "{}.yuml",
            if schema.name.is_empty() {
                "schema"
            } else {
                &schema.name
            }
        );

        let mut metadata = HashMap::new();
        metadata.insert("format".to_string(), "yuml".to_string());
        metadata.insert(
            "diagram_type".to_string(),
            format!("{:?}", self.options.diagram_type).to_lowercase(),
        );
        metadata.insert("style".to_string(), self.options.style.clone());

        Ok(vec![GeneratedOutput {
            content,
            filename,
            metadata,
        }])
    }
}

// Implement the synchronous Generator trait for backward compatibility
impl Generator for YumlGenerator {
    fn name(&self) -> &str {
        match self.options.diagram_type {
            YumlDiagramType::Class => "yuml",
            YumlDiagramType::UseCase => "yuml-usecase",
            YumlDiagramType::Activity => "yuml-activity",
        }
    }

    fn description(&self) -> &'static str {
        "Generates yUML diagrams from LinkML schemas"
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> Result<()> {
        // Validate schema has a name
        if schema.name.is_empty() {
            return Err(LinkMLError::data_validation(
                "Schema must have a name for yuml generation",
            ));
        }
        Ok(())
    }

    fn generate(&self, schema: &SchemaDefinition) -> std::result::Result<String, LinkMLError> {
        // Use tokio to run the async version
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| LinkMLError::service(format!("Failed to create runtime: {e}")))?;

        let options = GeneratorOptions::new();
        let outputs = runtime
            .block_on(AsyncGenerator::generate(self, schema, &options))
            .map_err(|e| LinkMLError::service(e.to_string()))?;

        // Concatenate all outputs into a single string
        Ok(outputs
            .into_iter()
            .map(|output| output.content)
            .collect::<Vec<_>>()
            .join(
                "
",
            ))
    }

    fn get_file_extension(&self) -> &'static str {
        "yuml"
    }

    fn get_default_filename(&self) -> &'static str {
        "generated.yuml"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    fn create_test_schema() -> SchemaDefinition {
        let mut schema = SchemaDefinition {
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        let person_class = ClassDefinition {
            slots: vec!["name".to_string(), "age".to_string()],
            ..Default::default()
        };
        schema.classes.insert("Person".to_string(), person_class);

        let employee_class = ClassDefinition {
            is_a: Some("Person".to_string()),
            slots: vec!["employee_id".to_string()],
            ..Default::default()
        };
        schema
            .classes
            .insert("Employee".to_string(), employee_class);

        let name_slot = SlotDefinition {
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        };
        schema.slots.insert("name".to_string(), name_slot);

        let age_slot = SlotDefinition {
            range: Some("integer".to_string()),
            ..Default::default()
        };
        schema.slots.insert("age".to_string(), age_slot);

        let id_slot = SlotDefinition {
            range: Some("string".to_string()),
            ..Default::default()
        };
        schema.slots.insert("employee_id".to_string(), id_slot);

        schema
    }

    #[tokio::test]
    async fn test_yuml_class_generation() {
        let schema = create_test_schema();
        let generator = YumlGenerator::new();
        let options = GeneratorOptions::default();

        let result = AsyncGenerator::generate(&generator, &schema, &options)
            .await
            .expect("should generate yuml: {}");
        assert_eq!(result.len(), 1);

        let output = &result[0];
        assert_eq!(output.filename, "TestSchema.yuml");

        // Check content
        assert!(output.content.contains("[Person|"));
        assert!(output.content.contains("[Employee|"));
        assert!(output.content.contains("[Person]^-[Employee]"));
    }

    #[tokio::test]
    async fn test_yuml_styles() {
        let schema = create_test_schema();
        let options = GeneratorOptions::default();

        for style in &["plain", "scruffy", "nofunky"] {
            let yuml_options = YumlOptions {
                style: style.to_string(),
                ..Default::default()
            };

            let generator = YumlGenerator::with_options(yuml_options);
            let result = AsyncGenerator::generate(&generator, &schema, &options)
                .await
                .expect("should generate yuml: {}");

            let output = &result[0];
            assert!(output.content.contains(&format!("diagram/{style}/")));
        }
    }
}
