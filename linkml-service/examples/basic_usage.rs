//! Basic usage example for LinkML service

use linkml_service::parser::Parser;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Example LinkML schema in YAML format
    let schema_yaml = r#"
id: https://example.org/person-schema
name: PersonSchema
description: A simple schema for modeling people
version: "1.0.0"
license: CC0

prefixes:
  ex: https://example.org/
  schema: http://schema.org/

default_prefix: ex

classes:
  Person:
    name: Person
    description: A person with basic attributes
    class_uri: schema:Person
    slots:
      - id
      - name
      - email
      - age
      - address

  Address:
    name: Address
    description: A postal address
    class_uri: schema:PostalAddress
    slots:
      - street
      - city
      - postal_code
      - country

slots:
  id:
    name: id
    description: Unique identifier for the person
    identifier: true
    range: string
    required: true

  name:
    name: name
    description: Full name of the person
    slot_uri: schema:name
    range: string
    required: true

  email:
    name: email
    description: Email address
    slot_uri: schema:email
    range: EmailAddress
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"

  age:
    name: age
    description: Age in years
    range: integer
    minimum_value: 0
    maximum_value: 150

  address:
    name: address
    description: Postal address
    range: Address
    inlined: true

  street:
    name: street
    description: Street address
    slot_uri: schema:streetAddress
    range: string

  city:
    name: city
    description: City name
    slot_uri: schema:addressLocality
    range: string

  postal_code:
    name: postal_code
    description: Postal code
    slot_uri: schema:postalCode
    range: string

  country:
    name: country
    description: Country name
    slot_uri: schema:addressCountry
    range: string

types:
  EmailAddress:
    name: EmailAddress
    description: A valid email address
    base_type: string
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"

enums:
  CountryCode:
    name: CountryCode
    description: ISO country codes
    permissible_values:
      - US
      - UK
      - DE
      - FR
      - JP
"#;

    // Parse the schema
    let parser = Parser::new();
    let schema = parser.parse_str(schema_yaml, "yaml")?;
    
    // Print schema information
    println!("Schema Information:");
    println!("==================");
    println!("ID: {}", schema.id);
    println!("Name: {}", schema.name);
    println!("Description: {}", schema.description.as_ref().unwrap_or(&"N/A".to_string()));
    println!("Version: {}", schema.version.as_ref().unwrap_or(&"N/A".to_string()));
    println!();
    
    // Print classes
    println!("Classes ({}):", schema.classes.len());
    for (name, class) in &schema.classes {
        println!("  - {}: {}", name, class.description.as_ref().unwrap_or(&"".to_string()));
        println!("    Slots: {:?}", class.slots);
    }
    println!();
    
    // Print slots
    println!("Slots ({}):", schema.slots.len());
    for (name, slot) in &schema.slots {
        let range = slot.range.as_ref().map(|s| s.as_str()).unwrap_or("string");
        let required = if slot.required.unwrap_or(false) { " (required)" } else { "" };
        let description = slot.description.as_ref().map(|s| s.as_str()).unwrap_or("");
        println!("  - {}: {} {}{}", name, range, description, required);
    }
    println!();
    
    // Print types
    if !schema.types.is_empty() {
        println!("Types ({}):", schema.types.len());
        for (name, type_def) in &schema.types {
            println!("  - {}: based on {}", name, 
                     type_def.base_type.as_ref().unwrap_or(&"string".to_string()));
        }
        println!();
    }
    
    // Print enums
    if !schema.enums.is_empty() {
        println!("Enums ({}):", schema.enums.len());
        for (name, enum_def) in &schema.enums {
            println!("  - {}: {} values", name, enum_def.permissible_values.len());
        }
    }
    
    Ok(())
}