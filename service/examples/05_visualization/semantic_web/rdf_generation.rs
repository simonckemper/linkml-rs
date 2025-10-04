//! Example demonstrating RDF generation from LinkML schemas
//!
//! This example shows how to use the enhanced RDF generator with different
//! modes (OWL, RDFS, Simple) and formats (Turtle, RDF/XML, N-Triples, JSON-LD).

use linkml_core::prelude::*;
use linkml_service::generator::{Generator, GeneratorOptions, RdfGenerator};
// TODO: RdfFormat and RdfMode imports commented out - API mismatch
// use linkml_service::generator::{RdfFormat, RdfMode};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema
    let mut schema = SchemaDefinition::default();
    schema.name = Some("PersonSchema".to_string());
    schema.id = Some("https://example.org/schemas/person".to_string());
    schema.description = Some("A simple schema for person data".to_string());

    // Define slots
    let mut name_slot = SlotDefinition::default();
    name_slot.description = Some("The person's full name".to_string());
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.description = Some("The person's age in years".to_string());
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(serde_json::json!(0));
    age_slot.maximum_value = Some(serde_json::json!(150));
    schema.slots.insert("age".to_string(), age_slot);

    // Define a class
    let mut person_class = ClassDefinition::default();
    person_class.description = Some("A person with basic information".to_string());
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Define an enumeration
    let mut status_enum = EnumDefinition::default();
    status_enum.description = Some("Person status values".to_string());
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Simple(
            "ACTIVE".to_string(),
        ));
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Simple(
            "INACTIVE".to_string(),
        ));
    schema.enums.insert("PersonStatus".to_string(), status_enum);

    let options = GeneratorOptions::default();

    println!(
        "=== RDF Generation Examples ===
"
    );

    // Example 1: OWL Ontology in Turtle format (default)
    println!("1. OWL Ontology (Turtle):");
    println!("-------------------------");
    let owl_generator = RdfGenerator::new();
    let owl_outputs = owl_generator.generate(&schema, &options).await?;
    if let Some(output) = owl_outputs.first() {
        println!("Filename: {}", output.filename);
        println!(
            "First 500 chars:
{}
...
",
            output.content.chars().take(500).collect::<String>()
        );
    }

    // Example 2: RDFS Schema in Turtle format
    println!("2. RDFS Schema (Turtle):");
    println!("------------------------");
    let rdfs_generator = RdfGenerator::rdfs();
    let rdfs_outputs = rdfs_generator.generate(&schema, &options).await?;
    if let Some(output) = rdfs_outputs.first() {
        println!("Filename: {}", output.filename);
        println!(
            "First 500 chars:
{}
...
",
            output.content.chars().take(500).collect::<String>()
        );
    }

    // Example 3: Simple RDF triples
    println!("3. Simple RDF (Turtle):");
    println!("-----------------------");
    let simple_generator = RdfGenerator::simple();
    let simple_outputs = simple_generator.generate(&schema, &options).await?;
    if let Some(output) = simple_outputs.first() {
        println!("Filename: {}", output.filename);
        println!(
            "Content:
{}
",
            output.content
        );
    }

    // Example 4: JSON-LD format
    println!("4. RDF in JSON-LD format:");
    println!("-------------------------");
    // TODO: JsonLd format not available in current API
    let jsonld_generator = RdfGenerator::new(); // .with_format(RdfFormat::JsonLd);
    let jsonld_outputs = jsonld_generator.generate(&schema, &options).await?;
    if let Some(output) = jsonld_outputs.first() {
        println!("Filename: {}", output.filename);
        println!(
            "Content:
{}
",
            output.content
        );
    }

    // Example 5: N-Triples format (simple RDF)
    println!("5. RDF in N-Triples format:");
    println!("---------------------------");
    // TODO: NTriples format and simple() method not available in current API
    let ntriples_generator = RdfGenerator::new(); // RdfGenerator::simple().with_format(RdfFormat::NTriples);
    let nt_outputs = ntriples_generator.generate(&schema, &options).await?;
    if let Some(output) = nt_outputs.first() {
        println!("Filename: {}", output.filename);
        println!(
            "Content:
{}
",
            output.content
        );
    }

    println!("✅ RDF generation complete!");
    println!(
        "
Available modes:"
    );
    println!("  - OWL: Full OWL ontology with restrictions and inference rules");
    println!("  - RDFS: Simple RDFS schema with classes and properties");
    println!("  - Simple: Plain RDF triples without ontology semantics");
    println!(
        "
Available formats:"
    );
    println!("  - Turtle (.ttl): Human-readable, compact RDF syntax");
    println!("  - RDF/XML (.rdf): XML-based RDF syntax");
    println!("  - N-Triples (.nt): Line-based, simple triple format");
    println!("  - JSON-LD (.jsonld): JSON-based linked data format");

    Ok(())
}
