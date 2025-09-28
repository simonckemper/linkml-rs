//! Integration tests for TypeDB code generation

use linkml_core::prelude::*;
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::generator::{Generator, GeneratorOptions, TypeQLGenerator};

#[tokio::test]
#[ignore] // Run with --ignored when TypeDB is available
async fn test_typeql_generation_with_complex_schema() {
    // Create a complex schema with inheritance and relationships
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.com/schemas/organization".to_string();
    schema.name = "organization_schema".to_string();
    schema.description = Some("Schema for organizational data".to_string());

    // Define slots
    let mut name_slot = SlotDefinition::default();
    name_slot.name = "name".to_string();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    name_slot.description = Some("Name of the entity".to_string());

    let mut email_slot = SlotDefinition::default();
    email_slot.name = "email".to_string();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string());

    let mut founded_date_slot = SlotDefinition::default();
    founded_date_slot.name = "founded_date".to_string();
    founded_date_slot.range = Some("date".to_string());

    let mut employees_slot = SlotDefinition::default();
    employees_slot.name = "employees".to_string();
    employees_slot.range = Some("Person".to_string());
    employees_slot.multivalued = Some(true);

    let mut employer_slot = SlotDefinition::default();
    employer_slot.name = "employer".to_string();
    employer_slot.range = Some("Organization".to_string());

    schema.slots.insert("name".to_string(), name_slot);
    schema.slots.insert("email".to_string(), email_slot);
    schema
        .slots
        .insert("founded_date".to_string(), founded_date_slot);
    schema.slots.insert("employees".to_string(), employees_slot);
    schema.slots.insert("employer".to_string(), employer_slot);

    // Define classes
    let mut organization_class = ClassDefinition::default();
    organization_class.name = "Organization".to_string();
    organization_class.description = Some("An organization or company".to_string());
    organization_class.slots = vec![
        "name".to_string(),
        "founded_date".to_string(),
        "employees".to_string(),
    ];

    let mut person_class = ClassDefinition::default();
    person_class.name = "Person".to_string();
    person_class.description = Some("A person".to_string());
    person_class.slots = vec![
        "name".to_string(),
        "email".to_string(),
        "employer".to_string(),
    ];

    // Add an abstract base class
    let mut named_entity_class = ClassDefinition::default();
    named_entity_class.name = "NamedEntity".to_string();
    named_entity_class.abstract_ = Some(true);
    named_entity_class.slots = vec!["name".to_string()];

    // Set inheritance
    organization_class.is_a = Some("NamedEntity".to_string());
    person_class.is_a = Some("NamedEntity".to_string());

    schema
        .classes
        .insert("NamedEntity".to_string(), named_entity_class);
    schema
        .classes
        .insert("Organization".to_string(), organization_class);
    schema.classes.insert("Person".to_string(), person_class);

    // Generate TypeQL
    let generator = TypeQLGenerator::new();

    let typeql = generator.generate(&schema).expect("Test operation failed");

    // Verify the generated TypeQL contains expected elements
    assert!(typeql.contains("define"));

    // Check attributes
    assert!(typeql.contains("name sub attribute, value string;"));
    assert!(typeql.contains("email sub attribute, value string;"));
    assert!(typeql.contains("founded-date sub attribute, value datetime;"));

    // Check entities
    assert!(typeql.contains("organization sub entity"));
    assert!(typeql.contains("person sub entity"));

    // Check ownership
    assert!(typeql.contains("owns name"));
    assert!(typeql.contains("owns email"));
    assert!(typeql.contains("owns founded-date"));

    // Check rules for required fields
    assert!(typeql.contains("rule"));
    assert!(typeql.contains("person-requires-name"));

    println!("Generated TypeQL:
{}", typeql);
}

#[tokio::test]
#[ignore] // Run with --ignored when TypeDB is available
async fn test_typeql_generation_with_relations() {
    let mut schema = SchemaDefinition::default();
    schema.id = "test".to_string();
    schema.name = "employment_schema".to_string();

    // Define slots
    let mut start_date_slot = SlotDefinition::default();
    start_date_slot.name = "start_date".to_string();
    start_date_slot.range = Some("date".to_string());
    start_date_slot.required = Some(true);

    let mut end_date_slot = SlotDefinition::default();
    end_date_slot.name = "end_date".to_string();
    end_date_slot.range = Some("date".to_string());

    let mut employee_slot = SlotDefinition::default();
    employee_slot.name = "employee".to_string();
    employee_slot.range = Some("Person".to_string());
    employee_slot.required = Some(true);

    let mut employer_slot = SlotDefinition::default();
    employer_slot.name = "employer".to_string();
    employer_slot.range = Some("Company".to_string());
    employer_slot.required = Some(true);

    schema
        .slots
        .insert("start_date".to_string(), start_date_slot);
    schema.slots.insert("end_date".to_string(), end_date_slot);
    schema.slots.insert("employee".to_string(), employee_slot);
    schema.slots.insert("employer".to_string(), employer_slot);

    // Define entity classes
    let mut person_class = ClassDefinition::default();
    person_class.name = "Person".to_string();

    let mut company_class = ClassDefinition::default();
    company_class.name = "Company".to_string();

    // Define relation class (has slots that reference other classes)
    let mut employment_class = ClassDefinition::default();
    employment_class.name = "Employment".to_string();
    employment_class.description = Some("Employment relationship".to_string());
    employment_class.slots = vec![
        "employee".to_string(),
        "employer".to_string(),
        "start_date".to_string(),
        "end_date".to_string(),
    ];

    schema.classes.insert("Person".to_string(), person_class);
    schema.classes.insert("Company".to_string(), company_class);
    schema
        .classes
        .insert("Employment".to_string(), employment_class);

    // Generate TypeQL
    let generator = TypeQLGenerator::new();
    let typeql = generator.generate(&schema).expect("Test operation failed");

    // Verify relation generation
    assert!(typeql.contains("employment sub relation"));
    assert!(typeql.contains("relates employee"));
    assert!(typeql.contains("relates employer"));
    assert!(typeql.contains("person plays employment:employee"));
    assert!(typeql.contains("company plays employment:employer"));

    println!("Generated TypeQL with relations:
{}", typeql);
}
