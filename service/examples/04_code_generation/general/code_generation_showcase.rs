//! Code Generation Showcase
//!
//! This example demonstrates all the code generation capabilities of LinkML:
//! - TypeQL for TypeDB
//! - SQL DDL for various databases
//! - GraphQL schemas
//! - JSON Schema
//! - Rust structs
//! - Python dataclasses
//! - And more!

mod common;
use common::create_example_service;

use linkml_service::generator::{GeneratorOptions, sql_generator::SqlDialect};
use linkml_service::prelude::*;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Code Generation Showcase");
    println!("==============================
");

    // Example schema for code generation
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

    // Parse schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;

    // Create service
    let service = create_example_linkml_service().await?;

    // 1. TypeQL Generation
    println!("1. TypeQL (TypeDB) Generation");
    println!("-----------------------------");

    let typeql = service
        .generate_code(&schema, "typeql", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&typeql[0].content, 500));

    // 2. SQL DDL Generation
    println!("2. SQL DDL Generation");
    println!("--------------------");

    // PostgreSQL
    let mut sql_options = GeneratorOptions::default();
    sql_options.set_custom("dialect", "postgresql");
    let postgresql = service.generate_code(&schema, "sql", &sql_options).await?;
    println!("PostgreSQL:
{}
", truncate(&postgresql[0].content, 400));

    // MySQL
    sql_options.set_custom("dialect", "mysql");
    let mysql = service.generate_code(&schema, "sql", &sql_options).await?;
    println!("MySQL:
{}
", truncate(&mysql[0].content, 400));

    // 3. GraphQL Schema
    println!("3. GraphQL Schema Generation");
    println!("---------------------------");

    let graphql = service
        .generate_code(&schema, "graphql", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&graphql[0].content, 500));

    // 4. JSON Schema
    println!("4. JSON Schema Generation");
    println!("------------------------");

    let json_schema = service
        .generate_code(&schema, "jsonschema", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&json_schema[0].content, 500));

    // 5. Rust Code Generation
    println!("5. Rust Struct Generation");
    println!("------------------------");

    let rust_code = service
        .generate_code(&schema, "rust", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&rust_code[0].content, 600));

    // 6. Python Dataclasses
    println!("6. Python Dataclass Generation");
    println!("-----------------------------");

    let python_code = service
        .generate_code(&schema, "python", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&python_code[0].content, 500));

    // 7. OWL/RDF
    println!("7. OWL/RDF Generation");
    println!("--------------------");

    let owl = service
        .generate_code(&schema, "owl", &GeneratorOptions::default())
        .await?;
    println!("{}
", truncate(&owl[0].content, 400));

    // 8. Performance comparison
    println!("Performance Comparison");
    println!("---------------------");

    use std::time::Instant;

    let generators = vec![
        ("TypeQL", "typeql"),
        ("SQL", "sql"),
        ("GraphQL", "graphql"),
        ("JSON Schema", "jsonschema"),
        ("Rust", "rust"),
        ("Python", "python"),
    ];

    for (name, gen_type) in generators {
        let start = Instant::now();
        let _ = service
            .generate_code(&schema, gen_type, &GeneratorOptions::default())
            .await?;
        let duration = start.elapsed();
        println!("{}: {:?}", name, duration);
    }

    // 9. Custom generation options
    println!("

Custom Generation Options");
    println!("------------------------");

    let mut custom_options = GeneratorOptions::default();
    custom_options.include_docs = true;
    custom_options.target_version = Some("3.0".to_string());
    custom_options.set_custom("namespace", "com.example.library");
    custom_options.set_custom("package_name", "library_system");

    println!("Available options:");
    println!("- include_docs: Include documentation comments");
    println!("- target_version: Target language/framework version");
    println!("- namespace: Custom namespace/package");
    println!("- dialect: SQL dialect (postgresql, mysql, sqlite)");
    println!("- naming_convention: snake_case, camelCase, PascalCase");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!(
            "{}...
[truncated {} more characters]",
            &s[..max_len],
            s.len() - max_len
        )
    }
}

async fn create_example_linkml_service() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // In a real application, this would initialize with all dependencies
    create_example_service().await?;
    Ok(())
}
