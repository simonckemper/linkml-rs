//! Schema Builder demonstration example
//!
//! This example demonstrates how to use the SchemaBuilder to programmatically
//! construct LinkML schemas from scratch.

use linkml_service::inference::builder::SchemaBuilder;
use timestamp_service::create_timestamp_service;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== LinkML Schema Builder Demo ===\n");

    // Create timestamp service
    let timestamp_service = create_timestamp_service()?;

    // Example 1: Simple Person Schema
    println!("Example 1: Building a Person schema...");
    let mut builder = SchemaBuilder::new("person_schema", "PersonSchema", timestamp_service.clone());

    builder
        .add_class("Person")
        .with_description("A person entity")
        .add_slot("name", "string", true, false)
        .add_slot("age", "integer", false, false)
        .add_slot("emails", "string", false, true)
        .finish();

    let schema = builder
        .with_description("Schema for person data")
        .with_version("1.0.0")
        .with_generation_metadata()?
        .build();

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&schema)?;
    println!("Generated Person Schema:\n{}\n", yaml);

    // Example 2: Organization Schema with relationships
    println!("Example 2: Building an Organization schema with relationships...");
    let mut builder = SchemaBuilder::new(
        "org_schema",
        "OrganizationSchema",
        timestamp_service.clone(),
    );

    // Add Person class
    builder
        .add_class("Person")
        .with_description("A person in the organization")
        .add_slot("id", "string", true, false)
        .add_slot("name", "string", true, false)
        .add_slot("role", "string", false, false)
        .finish();

    // Add Organization class
    builder
        .add_class("Organization")
        .with_description("An organization entity")
        .add_slot("org_name", "string", true, false)
        .add_slot("founded", "date", false, false)
        .add_slot("employees", "Person", false, true)
        .finish();

    let schema = builder
        .with_description("Schema for organization data")
        .with_version("1.0.0")
        .with_default_range("string")
        .with_generation_metadata()?
        .build();

    let yaml = serde_yaml::to_string(&schema)?;
    println!("Generated Organization Schema:\n{}\n", yaml);

    // Example 3: Schema with inheritance
    println!("Example 3: Building a schema with class inheritance...");
    let mut builder = SchemaBuilder::new(
        "inheritance_schema",
        "InheritanceSchema",
        timestamp_service.clone(),
    );

    // Abstract base class
    builder
        .add_class("Entity")
        .with_description("Abstract base entity")
        .abstract_class()
        .add_slot("id", "string", true, false)
        .add_slot("created_at", "datetime", true, false)
        .finish();

    // Derived class
    builder
        .add_class("Document")
        .with_description("A document entity")
        .with_parent("Entity")
        .add_slot("title", "string", true, false)
        .add_slot("content", "string", false, false)
        .finish();

    let schema = builder
        .with_description("Schema demonstrating inheritance")
        .with_version("1.0.0")
        .with_generation_metadata()?
        .build();

    let yaml = serde_yaml::to_string(&schema)?;
    println!("Generated Inheritance Schema:\n{}\n", yaml);

    println!("=== Demo Complete ===");
    println!("\nThe SchemaBuilder provides a fluent API for:");
    println!("  - Creating schemas with metadata (id, name, version, description)");
    println!("  - Adding classes with attributes");
    println!("  - Defining slot constraints (required, multivalued, types)");
    println!("  - Setting up inheritance hierarchies");
    println!("  - Generating valid LinkML YAML output");

    Ok(())
}
