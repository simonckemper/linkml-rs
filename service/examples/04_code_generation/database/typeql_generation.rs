//! TypeQL Generation Example
//!
//! This example demonstrates how to use the LinkML TypeQL generator to convert
//! LinkML schemas into TypeDB schemas with exceptional performance.

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator,
    GeneratorOptions,
    // typeql_generator_enhanced::{EnhancedTypeQLGenerator, create_enhanced_typeql_generator},
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML TypeQL Generation Example");
    println!("================================
");

    // Example 1: Simple entity schema
    let simple_schema = r#"
id: https://example.com/simple
name: SimpleSchema
description: A simple schema for TypeQL generation

classes:
  Person:
    description: A person entity
    slots:
      - id
      - name
      - email
      - age
    slot_usage:
      id:
        identifier: true
      email:
        pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"
      age:
        minimum_value: 0
        maximum_value: 150

slots:
  id:
    description: Unique identifier
    range: string
    required: true
  name:
    description: Person's full name
    range: string
    required: true
  email:
    description: Email address
    range: string
  age:
    description: Age in years
    range: integer
"#;

    let parser = YamlParser::new();
    let schema = parser.parse_str(simple_schema)?;

    // Generate TypeQL using the factory function
    let generator = create_enhanced_typeql_generator();
    let options = GeneratorOptions::default();

    println!("Generating TypeQL for simple schema...");
    let outputs = generator.generate(&schema, &options).await?;

    println!("Generated TypeQL:
");
    println!("{}", outputs[0].content);

    // Example 2: Complex relations
    let relation_schema = r#"
id: https://example.com/relations
name: RelationSchema
description: Schema demonstrating complex relations

classes:
  Person:
    slots:
      - id
      - name

  Organization:
    slots:
      - id
      - name

  Employment:
    description: Employment relationship
    slots:
      - employee
      - employer
      - start_date
      - end_date
      - position
    slot_usage:
      employee:
        range: Person
        required: true
      employer:
        range: Organization
        required: true

  Meeting:
    description: Multi-way relation
    slots:
      - organizer
      - participants
      - location
      - date
    slot_usage:
      organizer:
        range: Person
        required: true
      participants:
        range: Person
        multivalued: true
        minimum_cardinality: 2

slots:
  id:
    identifier: true
    range: string
  name:
    range: string
    required: true
  employee:
    range: Person
  employer:
    range: Organization
  start_date:
    range: date
  end_date:
    range: date
  position:
    range: string
  organizer:
    range: Person
  participants:
    range: Person
  location:
    range: string
  date:
    range: datetime
"#;

    let relation_schema = parser.parse_str(relation_schema)?;

    println!("

Generating TypeQL for relation schema...");
    let outputs = generator.generate(&relation_schema, &options).await?;

    println!("Generated TypeQL with relations:
");
    println!("{}", outputs[0].content);

    // Example 3: Schema with rules and constraints
    let rule_schema = r#"
id: https://example.com/rules
name: RuleSchema
description: Schema with validation rules

classes:
  Document:
    slots:
      - id
      - status
      - approved_by
      - approval_date
    rules:
      - title: approval_required
        description: Documents must be approved when status is published
    conditional_requirements:
      - if_field: status
        equals_string: published
        required_fields: [approved_by, approval_date]

slots:
  id:
    identifier: true
    range: string
  status:
    range: DocumentStatus
  approved_by:
    range: string
  approval_date:
    range: date

enums:
  DocumentStatus:
    permissible_values:
      draft:
        description: Document is being edited
      review:
        description: Document is under review
      published:
        description: Document is published
"#;

    let rule_schema = parser.parse_str(rule_schema)?;

    println!("

Generating TypeQL with rules...");
    let outputs = generator.generate(&rule_schema, &options).await?;

    println!("Generated TypeQL with validation rules:
");
    println!("{}", outputs[0].content);

    // Performance demonstration
    println!("

Performance Demonstration");
    println!("------------------------");

    let mut large_schema = SchemaDefinition::default();
    large_schema.name = "LargeSchema".to_string();

    // Create 100 classes
    for i in 0..100 {
        let mut class = ClassDefinition::default();
        class.description = Some(format!("Class number {}", i));
        class.slots = vec!["id".to_string(), "name".to_string()];

        if i > 0 && i % 10 == 0 {
            class.is_a = Some(format!("Class{}", i - 1));
        }

        large_schema.classes.insert(format!("Class{}", i), class);
    }

    // Add slots
    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    large_schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    large_schema.slots.insert("name".to_string(), name_slot);

    use std::time::Instant;
    let start = Instant::now();
    let outputs = generator.generate(&large_schema, &options).await?;
    let duration = start.elapsed();

    println!("Generated TypeQL for 100 classes in {:?}", duration);
    println!("Output size: {} bytes", outputs[0].content.len());
    println!(
        "Performance: {:.2} classes/ms",
        100.0 / duration.as_millis() as f64
    );

    // Migration example
    println!("

Schema Migration Example");
    println!("-----------------------");

    let mut options_with_migration = GeneratorOptions::default();
    options_with_migration.set_custom("generate_migration", "true");

    let outputs = generator.generate(&schema, &options_with_migration).await?;

    if outputs.len() > 1 {
        println!("Migration script generated:");
        println!("{}", outputs[1].content);
    }

    Ok(())
}
