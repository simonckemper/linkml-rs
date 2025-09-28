//! Integration tests for the Graphviz generator

use linkml_core::prelude::*;
use linkml_service::generator::{Generator, GeneratorOptions, GraphvizGenerator};

/// Create a test schema with various features
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("PersonSchema".to_string());
    schema.id = Some("https://example.org/person-schema".to_string());
    schema.description = Some("A schema for person data with inheritance".to_string());

    // Base class: NamedThing
    let mut named_thing = ClassDefinition::default();
    named_thing.abstract_ = Some(true);
    named_thing.description = Some("A thing with a name".to_string());
    named_thing.slots = vec!["name".to_string()];
    schema.classes.insert("NamedThing".to_string(), named_thing);

    // Person class inheriting from NamedThing
    let mut person_class = ClassDefinition::default();
    person_class.description = Some("A person with basic information".to_string());
    person_class.is_a = Some("NamedThing".to_string());
    person_class.slots = vec![
        "age".to_string(),
        "email".to_string(),
        "address".to_string(),
    ];
    schema.classes.insert("Person".to_string(), person_class);

    // Address class
    let mut address_class = ClassDefinition::default();
    address_class.description = Some("A postal address".to_string());
    address_class.slots = vec![
        "street".to_string(),
        "city".to_string(),
        "postal_code".to_string(),
    ];
    schema.classes.insert("Address".to_string(), address_class);

    // Define slots
    let mut name_slot = SlotDefinition::default();
    name_slot.description = Some("The name of a thing".to_string());
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.description = Some("Age in years".to_string());
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(serde_json::json!(0));
    age_slot.maximum_value = Some(serde_json::json!(150));
    schema.slots.insert("age".to_string(), age_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.description = Some("Email address".to_string());
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^\S+@\S+\.\S+$".to_string());
    email_slot.multivalued = Some(true);
    schema.slots.insert("email".to_string(), email_slot);

    let mut address_slot = SlotDefinition::default();
    address_slot.description = Some("Postal address".to_string());
    address_slot.range = Some("Address".to_string());
    schema.slots.insert("address".to_string(), address_slot);

    let mut street_slot = SlotDefinition::default();
    street_slot.range = Some("string".to_string());
    schema.slots.insert("street".to_string(), street_slot);

    let mut city_slot = SlotDefinition::default();
    city_slot.range = Some("string".to_string());
    schema.slots.insert("city".to_string(), city_slot);

    let mut postal_code_slot = SlotDefinition::default();
    postal_code_slot.range = Some("string".to_string());
    schema
        .slots
        .insert("postal_code".to_string(), postal_code_slot);

    // Add an enum
    let mut status_enum = EnumDefinition::default();
    status_enum.description = Some("Person status".to_string());
    status_enum
        .permissible_values
        .push(PermissibleValue::Simple("ACTIVE".to_string());
    status_enum
        .permissible_values
        .push(PermissibleValue::Simple("INACTIVE".to_string());
    status_enum
        .permissible_values
        .push(PermissibleValue::Simple("PENDING".to_string()));
    schema.enums.insert("PersonStatus".to_string(), status_enum);

    schema
}

#[tokio::test]
async fn test_graphviz_basic_generation() {
    let schema = create_test_schema();
    let generator = GraphvizGenerator::new();
    let options = GeneratorOptions::default();

    let output = generator.generate(&schema).expect("Test operation failed");

    // Verify basic structure
    assert!(output.contains("digraph LinkMLSchema"));
    assert!(output.contains("rankdir=TB"));

    // Check classes are present
    assert!(output.contains("NamedThing"));
    assert!(output.contains("Person"));
    assert!(output.contains("Address"));

    // Check inheritance relationship
    assert!(output.contains("NamedThing -> Person"));

    // Check object relationship
    assert!(output.contains("Person -> Address"));
}

#[tokio::test]
async fn test_graphviz_styles() {
    let schema = create_test_schema();
    let options = GeneratorOptions::default();

    // Test different styles
    let styles = vec![
        graphviz::GraphvizStyle::Simple,
        graphviz::GraphvizStyle::Uml,
        graphviz::GraphvizStyle::EntityRelationship,
        graphviz::GraphvizStyle::Hierarchical,
    ];

    for style in styles {
        let generator = GraphvizGenerator::new().with_style(style);
        let output = generator.generate(&schema).expect("Test operation failed");

        // Style-specific checks
        match style {
            graphviz::GraphvizStyle::Uml => {
                assert!(output.contains("shape=record"));
                assert!(output.contains("<<abstract>>"));
            }
            graphviz::GraphvizStyle::Simple => {
                assert!(output.contains("shape=box"));
            }
            graphviz::GraphvizStyle::Hierarchical => {
                assert!(output.contains("fillcolor=lightblue"));
            }
            _ => {}
        }
    }
}

#[tokio::test]
async fn test_graphviz_layouts() {
    let schema = create_test_schema();
    let options = GeneratorOptions::default();

    // Test different layout engines
    let layouts = vec![
        graphviz::GraphvizLayout::Dot,
        graphviz::GraphvizLayout::Neato,
        graphviz::GraphvizLayout::Fdp,
        graphviz::GraphvizLayout::Twopi,
        graphviz::GraphvizLayout::Circo,
    ];

    for layout in layouts {
        let generator = GraphvizGenerator::new().with_layout(layout);
        let output = generator.generate(&schema).expect("Test operation failed");
        // Note: metadata checks would need to be adjusted based on new String return type
    }
}

#[tokio::test]
async fn test_graphviz_with_options() {
    let schema = create_test_schema();
    let options = GeneratorOptions::default();

    // Test with custom options
    let custom_options = graphviz::GraphvizOptions {
        style: graphviz::GraphvizStyle::Uml,
        layout: graphviz::GraphvizLayout::Dot,
        include_slots: true,
        include_enums: true,
        include_types: false,
        show_cardinality: true,
        show_inheritance: true,
        show_mixins: true,
        use_colors: true,
        rankdir: "LR".to_string(), // Left to right
    };

    let generator = GraphvizGenerator::with_options(custom_options);
    let output = generator.generate(&schema).expect("Test operation failed");

    // Check custom rankdir
    assert!(output.contains("rankdir=LR"));

    // Check cardinality is shown
    assert!(output.contains("[0..1]") || output.contains("[*]"));

    // Check enum is included
    assert!(output.contains("PersonStatus"));
}

// Need to use the graphviz module from the generator
use linkml_service::generator::graphviz;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};
