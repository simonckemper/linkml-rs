//! Example demonstrating Rust code generation from LinkML schemas
//!
//! This example shows how to use the Rust code generator to create
//! idiomatic Rust code from LinkML schema definitions.

use linkml_core::types::{ClassDefinition, PermissibleValue, SchemaDefinition, SlotDefinition};
use linkml_service::generator::{Generator, GeneratorOptions, RustGenerator};
use serde_json::json;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema
    let mut schema = SchemaDefinition {
        id: "https://example.org/schemas/library".to_string(),
        name: "library_schema".to_string(),
        title: Some("Library Management Schema".to_string()),
        description: Some("Schema for managing library books and authors".to_string()),
        ..Default::default()
    };

    // Define an Author class
    let author = ClassDefinition {
        name: "Author".to_string(),
        description: Some("Represents a book author".to_string()),
        slots: vec![
            "id".to_string(),
            "name".to_string(),
            "email".to_string(),
            "birth_date".to_string(),
        ],
        ..Default::default()
    };

    // Define a Book class
    let book = ClassDefinition {
        name: "Book".to_string(),
        description: Some("Represents a book in the library".to_string()),
        slots: vec![
            "isbn".to_string(),
            "title".to_string(),
            "author_id".to_string(),
            "publication_date".to_string(),
            "pages".to_string(),
            "genres".to_string(),
            "status".to_string(),
        ],
        ..Default::default()
    };

    schema.classes.insert("Author".to_string(), author);
    schema.classes.insert("Book".to_string(), book);

    // Define slots
    schema.slots.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            description: Some("Unique identifier".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            pattern: Some(r"^[A-Z]{2}\d{6}$".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "name".to_string(),
        SlotDefinition {
            name: "name".to_string(),
            description: Some("Full name".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            description: Some("Email address".to_string()),
            range: Some("string".to_string()),
            pattern: Some(r"^[\w\.-]+@[\w\.-]+\.\w+$".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "birth_date".to_string(),
        SlotDefinition {
            name: "birth_date".to_string(),
            description: Some("Date of birth".to_string()),
            range: Some("date".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "isbn".to_string(),
        SlotDefinition {
            name: "isbn".to_string(),
            description: Some("ISBN-13 number".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            pattern: Some(r"^978-\d{1,5}-\d{1,7}-\d{1,7}-\d$".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "title".to_string(),
        SlotDefinition {
            name: "title".to_string(),
            description: Some("Book title".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "author_id".to_string(),
        SlotDefinition {
            name: "author_id".to_string(),
            description: Some("Reference to the author".to_string()),
            range: Some("string".to_string()),
            required: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "publication_date".to_string(),
        SlotDefinition {
            name: "publication_date".to_string(),
            description: Some("Date of publication".to_string()),
            range: Some("date".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "pages".to_string(),
        SlotDefinition {
            name: "pages".to_string(),
            description: Some("Number of pages".to_string()),
            range: Some("integer".to_string()),
            minimum_value: Some(json!(1)),
            maximum_value: Some(json!(10000)),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "genres".to_string(),
        SlotDefinition {
            name: "genres".to_string(),
            description: Some("Book genres".to_string()),
            range: Some("string".to_string()),
            multivalued: Some(true),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "status".to_string(),
        SlotDefinition {
            name: "status".to_string(),
            description: Some("Current status of the book".to_string()),
            permissible_values: vec![
                PermissibleValue::Simple("available".to_string()),
                PermissibleValue::Simple("checked-out".to_string()),
                PermissibleValue::Complex {
                    text: "lost".to_string(),
                    description: Some("Book is lost".to_string()),
                    meaning: None,
                },
                PermissibleValue::Complex {
                    text: "damaged".to_string(),
                    description: Some("Book is damaged".to_string()),
                    meaning: None,
                },
            ],
            ..Default::default()
        },
    );

    // Generate Rust code
    let generator = RustGenerator::new();
    let options = GeneratorOptions::new()
        .with_docs(true)
        .set_custom("generate_builder", "true");

    println!("Generating Rust code from LinkML schema...
");

    let outputs = generator.generate(&schema, &options).await?;

    // Display the generated code
    for output in outputs {
        println!("=== Generated file: {} ===
", output.filename);
        println!("{}", output.content);

        // In a real application, you would write this to a file:
        // std::fs::write(&output.filename, &output.content)?;
    }

    // Demonstrate what the generated code would look like in use
    println!("
=== Example usage of generated code ===
");
    println!(
        r#"
// Using the generated code:
use library_schema::{{Author, Book, Status}};

fn example() -> std::result::Result<(), Vec<ValidationError>> {{
    // Create an author
    let author = Author::new("AU123456", "Jane Doe");
    author.validate()?;

    // Create a book using the builder pattern
    let book = BookBuilder::default()
        .isbn("978-1-23456-789-0")
        .title("Rust Programming")
        .author_id("AU123456")
        .pages(350)
        .status(Status::Available)
        .build();

    book.validate()?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&book)?;
    println!("{{}}", json);

    Ok(())
}}
"#
    );

    Ok(())
}
