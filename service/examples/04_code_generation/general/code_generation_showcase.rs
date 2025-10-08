//! Code Generation Showcase
//!
//! Demonstrates how to generate multiple artifacts from a LinkML schema using the
//! built-in generator registry. The example keeps the schema inline to make the
//! workflow copy/paste friendly.

use anyhow::Context;
use linkml_core::prelude::SchemaDefinition;
use linkml_service::generator::registry::GeneratorRegistry;
use serde_yaml;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Code Generation Showcase");
    println!("================================\n");

    let schema = load_schema()?;
    let registry = GeneratorRegistry::with_defaults().await;

    let highlights = [
        ("Rust", "rust"),
        ("Python (Pydantic)", "pydantic"),
        ("JSON Schema", "jsonschema"),
        ("GraphQL", "graphql"),
        ("SQL (generic)", "sql"),
        ("TypeQL", "typeql"),
    ];

    for (label, generator_name) in highlights {
        println!("{label}");
        println!("{}", "-".repeat(label.len()));

        match registry.get(generator_name).await {
            Some(generator) => {
                generator.validate_schema(&schema).with_context(|| {
                    format!("validating schema for generator '{generator_name}'")
                })?;

                let output = generator
                    .generate(&schema)
                    .with_context(|| format!("running generator '{generator_name}'"))?;

                println!("• name: {}", generator.name());
                println!("• extension: .{}", generator.get_file_extension());
                println!("• preview:\n{}\n", truncate(&output, 400));
            }
            None => {
                println!("(generator '{generator_name}' not available in this build)\n");
            }
        }
    }

    Ok(())
}

fn load_schema() -> Result<SchemaDefinition, Box<dyn std::error::Error>> {
    let schema_yaml = r#"
id: https://example.com/library
name: LibrarySystem
description: Library management system schema

prefixes:
  library: https://example.com/library/
  schema: http://schema.org/

default_prefix: library

classes:
  Book:
    description: A book in the library
    slots:
      - isbn
      - title
      - authors
      - publication_year
      - genre
      - available
      - borrower
    slot_usage:
      isbn:
        identifier: true
      authors:
        required: true
        minimum_cardinality: 1

  Person:
    description: A person (author or borrower)
    slots:
      - id
      - name
      - email
      - borrowed_books
    slot_usage:
      id:
        identifier: true
      borrowed_books:
        multivalued: true

  Loan:
    description: Book loan record
    slots:
      - loan_id
      - book
      - borrower
      - loan_date
      - due_date
      - return_date
    slot_usage:
      loan_id:
        identifier: true
      book:
        required: true
      borrower:
        required: true

slots:
  isbn:
    description: International Standard Book Number
    range: string
    pattern: "^(978|979)-\\d{1,5}-\\d{1,7}-\\d{1,7}-\\d$"

  title:
    description: Book title
    range: string
    required: true

  authors:
    description: Book authors
    range: Person
    multivalued: true

  publication_year:
    description: Year of publication
    range: integer
    minimum_value: 1000
    maximum_value: 2100

  genre:
    description: Literary genre
    range: Genre

  available:
    description: Whether the book is available for loan
    range: boolean

  borrower:
    description: Current borrower
    range: Person

  id:
    description: Person identifier
    range: string

  name:
    description: Person's full name
    range: string
    required: true

  email:
    description: Email address
    range: string
    pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"

  borrowed_books:
    description: Books currently borrowed
    range: Book

  loan_id:
    description: Unique loan identifier
    range: string

  book:
    description: The borrowed book
    range: Book

  loan_date:
    description: Date of loan
    range: date

  due_date:
    description: Due date for return
    range: date

  return_date:
    description: Actual return date
    range: date

enums:
  Genre:
    permissible_values:
      fiction:
        description: Fictional works
      non_fiction:
        description: Non-fictional works
      science_fiction:
        description: Science fiction
      mystery:
        description: Mystery and thriller
      biography:
        description: Biographical works
      technical:
        description: Technical and educational
"#;

    Ok(serde_yaml::from_str(schema_yaml)?)
}

fn truncate(content: &str, max_len: usize) -> String {
    if content.len() <= max_len {
        return content.to_string();
    }

    format!(
        "{}…\n[truncated {} characters]",
        &content[..max_len],
        content.len() - max_len
    )
}
