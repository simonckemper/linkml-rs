//! Example demonstrating Excel generation from LinkML schemas
//!
//! This example shows how to use the Excel generator to create
//! multi-sheet workbooks from LinkML schemas.

use linkml_core::prelude::*;
use linkml_service::generator::{ExcelGenerator, Generator, GeneratorOptions};
use std::collections::HashMap;
use std::fs;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema
    let mut schema = SchemaDefinition::default();
    schema.name = Some("PersonSchema".to_string());
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

    let mut email_slot = SlotDefinition::default();
    email_slot.description = Some("The person's email address".to_string());
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut status_slot = SlotDefinition::default();
    status_slot.description = Some("The person's current status".to_string());
    status_slot.range = Some("PersonStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    // Define an enumeration
    let mut status_enum = EnumDefinition::default();
    status_enum.description = Some("Possible status values for a person".to_string());
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Complex {
            text: "ACTIVE".to_string(),
            description: Some("Person is currently active".to_string()),
            meaning: None,
            aliases: vec![],
            flags: HashMap::new(),
            extensions: HashMap::new(),
        });
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Complex {
            text: "INACTIVE".to_string(),
            description: Some("Person is currently inactive".to_string()),
            meaning: None,
            aliases: vec![],
            flags: HashMap::new(),
            extensions: HashMap::new(),
        });
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Simple(
            "PENDING".to_string(),
        ));
    schema.enums.insert("PersonStatus".to_string(), status_enum);

    // Define a class
    let mut person_class = ClassDefinition::default();
    person_class.description = Some("A person with basic information".to_string());
    person_class.slots = vec![
        "name".to_string(),
        "age".to_string(),
        "email".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("Person".to_string(), person_class);

    // Create the Excel generator
    let generator = ExcelGenerator::new()
        .with_summary(true)
        .with_validation(true)
        .with_frozen_headers(true)
        .with_filters(true);

    println!("Generating Excel workbook...");

    // Generate the Excel file
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema)?;

    // The output is base64 encoded, so we need to decode it
    use base64::Engine;
    let decoded = base64::engine::general_purpose::STANDARD.decode(&output)?;

    // Save to file
    let filename = "person_schema.xlsx";
    fs::write(filename, decoded)?;

    println!("✅ Excel workbook generated: {}", filename);
        println!("   - Summary sheet with schema statistics");
        println!("   - Person sheet with headers, types, and descriptions");
        println!("   - Enumerations sheet with PersonStatus values");
        println!("   - Validation Info sheet with field constraints");
        println!("
Note: Due to rust_xlsxwriter v0.64 limitations:");
        println!("   - Cell comments are shown as a description row");
        println!("   - Data validation rules are documented in a separate sheet");
    }

    Ok(())
}
