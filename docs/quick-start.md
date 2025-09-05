# Quick Start Guide

This guide will get you up and running with LinkML for Rust in 5 minutes.

## Installation

Add LinkML to your project:

```bash
cargo add linkml
```

Or add to `Cargo.toml`:

```toml
[dependencies]
linkml = "2.0"
```

## Your First Schema

Create a simple schema for a person:

```rust
use linkml_service::parser::Parser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Define a schema in YAML
    let schema_yaml = r"
id: https://example.org/person-schema
name: PersonSchema
description: A simple schema for people

classes:
  Person:
    name: Person
    description: A person with basic attributes
    slots:
      - id
      - name
      - email
      - age
      - occupation

slots:
  id:
    name: id
    identifier: true
    range: string
  name:
    name: name
    description: Full name of the person
    required: true
    range: string
  email:
    name: email
    description: Email address
    range: string
    pattern: '^[^@]+@[^@]+\.[^@]+$'
  age:
    name: age
    description: Age in years
    range: integer
  occupation:
    name: occupation
    description: Job title or profession
    range: string
";

    // Parse the schema
    let parser = Parser::new();
    let schema = parser.parse_str(schema_yaml, "yaml")?;

    println!("Schema loaded: {}", schema.name);

    // Create validation engine
    let validation_engine = ValidationEngine::new(&schema)?;

    // Create some test data
    let valid_person = json!({
        "id": "person-001",
        "name": "Alice Smith",
        "email": "alice@example.com",
        "age": 30,
        "occupation": "Software Engineer"
    });
    
    let invalid_person = json!({
        "id": "person-002",
        "email": "not-an-email",  // Invalid format
        "age": 30
        // Missing required 'name' field
    });

    // Validate the data
    println!("\nValidating valid person:");
    let result = validation_engine.validate_as_class(&valid_person, "Person", None).await?;
    println!("Valid: {}", result.valid);

    println!("\nValidating invalid person:");
    let result = validation_engine.validate_as_class(&invalid_person, "Person", None).await?;
    println!("Valid: {}", result.valid);

    if !result.valid {
        println!("Validation issues found:");
        for issue in &result.issues {
            println!("  - {}", issue.message);
        }
    }
    
    Ok(())
}
```

## Generate Code from Schema

LinkML can generate code in multiple languages:

```rust
use linkml::generator::{RustGenerator, GeneratorOptions};

// Generate Rust structs
let generator = RustGenerator::new();
let options = GeneratorOptions::default();
let rust_code = generator.generate(&schema, &options)?;

println!("Generated Rust code:\n{}", rust_code);
```

This generates:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    #[serde(rename = "id")]
    pub id: String,
    
    #[serde(rename = "name")]
    pub name: String,
    
    #[serde(rename = "email", skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    
    #[serde(rename = "age", skip_serializing_if = "Option::is_none")]
    pub age: Option<i32>,
    
    #[serde(rename = "occupation", skip_serializing_if = "Option::is_none")]
    pub occupation: Option<String>,
}
```

## Generate TypeQL for TypeDB

```rust
use linkml::generator::{TypeQLGenerator, TypeQLOptions};

let generator = TypeQLGenerator::new();
let mut options = TypeQLOptions::default();
options.generate_rules = true;

let typeql = generator.generate(&schema, &options)?;
println!("Generated TypeQL:\n{}", typeql);
```

Output:
```typeql
define

ex:Person sub entity,
    owns ex:id @key,
    owns ex:name,
    owns ex:email,
    owns ex:age,
    owns ex:occupation;

ex:id sub attribute, value string;
ex:name sub attribute, value string;
ex:email sub attribute, value string, regex "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$";
ex:age sub attribute, value long;
ex:occupation sub attribute, value string;
```

## Working with Complex Schemas

```rust
// Schema with inheritance and constraints
let complex_schema = r#"
classes:
  NamedThing:
    abstract: true
    attributes:
      id:
        identifier: true
      name:
        required: true
        
  Person:
    is_a: NamedThing
    attributes:
      email:
        range: string
        
  Organization:
    is_a: NamedThing
    attributes:
      employees:
        range: Person
        multivalued: true
        
  Document:
    attributes:
      title:
        required: true
      authors:
        range: Person
        multivalued: true
        minimum_cardinality: 1
    rules:
      - description: Must have DOI or ISBN
        exactly_one_of:
          - slot: doi
          - slot: isbn
"#;
```

## Streaming Large Datasets

For large datasets, use streaming validation:

```rust
use linkml::stream::StreamValidator;
use std::fs::File;
use std::io::{BufReader, BufRead};

let file = File::open("large_dataset.jsonl")?;
let reader = BufReader::new(file);

let validator = StreamValidator::new(schema);
let mut valid_count = 0;
let mut error_count = 0;

for line in reader.lines() {
    let line = line?;
    let data: serde_json::Value = serde_json::from_str(&line)?;
    
    match validator.validate_single(&data, "Person") {
        Ok(_) => valid_count += 1,
        Err(e) => {
            error_count += 1;
            eprintln!("Validation error: {}", e);
        }
    }
}

println!("Processed {} valid, {} invalid records", valid_count, error_count);
```

## Next Steps

1. **[Schema Authoring Guide](schema-authoring.md)** - Learn LinkML schema syntax
2. **[Validation Guide](validation-guide.md)** - Advanced validation features
3. **[Code Generation](code-generation.md)** - Generate code for 20+ languages
4. **[Examples](../linkml-service/examples/)** - More working examples

## Getting Help

- Run `cargo doc --open` for API documentation
- Check [examples](../linkml-service/examples/) for working code
- Report issues on [GitHub](https://github.com/simonckemper/linkml-rs/issues)
