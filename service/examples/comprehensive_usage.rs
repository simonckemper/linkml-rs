//! Comprehensive usage examples for LinkML service
//!
//! This example demonstrates all major features of the LinkML service including:
//! - Schema parsing and validation
//! - Data validation with advanced features
//! - Code generation for multiple languages
//! - Performance optimization techniques
//! - Real-world integration patterns

use linkml_core::types::SchemaDefinition;
use linkml_service::factory::create_linkml_service;
use linkml_service::generator::{
    python_dataclass::PythonDataclassGenerator, traits::Generator, typescript::TypeScriptGenerator,
};
use linkml_service::loader::json_v2::JsonV2Loader;
use linkml_service::parser::yaml_parser::YamlParser;
use linkml_service::validator::{ValidationContext, ValidationEngine};
use serde_json::json;
use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🚀 LinkML Service Comprehensive Usage Examples");
    println!("==============================================
");

    // Example 1: Basic Schema Parsing and Validation
    basic_schema_example().await?;

    // Example 2: Advanced Validation Features
    advanced_validation_example().await?;

    // Example 3: Code Generation
    code_generation_example().await?;

    // Example 4: Real-world Integration
    real_world_integration_example().await?;

    // Example 5: Performance Optimization
    performance_optimization_example().await?;

    println!("
✅ All examples completed successfully!");
    Ok(())
}

/// Example 1: Basic Schema Parsing and Validation
async fn basic_schema_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("📋 Example 1: Basic Schema Parsing and Validation");
    println!("--------------------------------------------------");

    // Define a simple schema
    let schema_yaml = r#"
id: https://example.org/person-schema
name: person_schema
description: A simple person schema

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
        required: true
      name:
        required: true
      email:
        pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
      age:
        range: integer
        minimum_value: 0
        maximum_value: 150

slots:
  id:
    range: string
    required: true
  name:
    range: string
    required: true
  email:
    range: string
  age:
    range: integer

types:
  string:
    base: str
  integer:
    base: int
"#;

    // Parse the schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    println!("✓ Schema parsed successfully");

    // Create validation engine
    let engine = ValidationEngine::new(Arc::new(schema));
    println!("✓ Validation engine created");

    // Test valid data
    let valid_person = json!({
        "id": "person:001",
        "name": "Alice Johnson",
        "email": "alice@example.com",
        "age": 30
    });

    let result = engine.validate_instance(&valid_person, "Person").await?;
    println!(
        "✓ Valid person validation: {}",
        if result.is_valid() {
            "PASSED"
        } else {
            "FAILED"
        }
    );

    // Test invalid data
    let invalid_person = json!({
        "id": "person:002",
        "name": "Bob Smith",
        "email": "invalid-email",  // Invalid email format
        "age": 200  // Age too high
    });

    let result = engine.validate_instance(&invalid_person, "Person").await?;
    println!(
        "✓ Invalid person validation: {} (expected to fail)",
        if result.is_valid() {
            "PASSED"
        } else {
            "FAILED"
        }
    );
    if !result.is_valid() {
        println!("  Validation errors:");
        for issue in result.issues.iter().take(3) {
            println!("    - {}", issue.message);
        }
    }

    println!();
    Ok(())
}

/// Example 2: Advanced Validation Features
async fn advanced_validation_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Example 2: Advanced Validation Features");
    println!("-------------------------------------------");

    let schema_yaml = r#"
id: https://example.org/advanced-schema
name: advanced_schema
description: Schema with advanced validation features

classes:
  Author:
    description: An author
    slots:
      - id
      - name
      - publications
    slot_usage:
      id:
        identifier: true
        required: true
      publications:
        range: Publication
        multivalued: true

  Publication:
    description: A publication
    slots:
      - id
      - title
      - authors
      - doi
    slot_usage:
      id:
        identifier: true
        required: true
      authors:
        range: Author
        multivalued: true
        required: true
      doi:
        pattern: "^10\\.\\d+/.+"

slots:
  id: {range: string, required: true}
  name: {range: string, required: true}
  title: {range: string, required: true}
  publications: {range: Publication, multivalued: true}
  authors: {range: Author, multivalued: true}
  doi: {range: string}

types:
  string: {base: str}
"#;

    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    // Test cross-reference validation
    let author = json!({
        "id": "author:001",
        "name": "Dr. Jane Smith",
        "publications": ["pub:001"]
    });

    let publication = json!({
        "id": "pub:001",
        "title": "Advanced LinkML Patterns",
        "authors": ["author:001"],
        "doi": "10.1234/example.2023"
    });

    // Create validation context with cross-references
    let mut context = ValidationContext::new(Arc::new(engine.schema().as_ref().clone());
    context.set_all_instances(vec![author.clone(), publication.clone()]);

    let author_result = engine.validate_instance(&author, "Author").await?;
    let pub_result = engine
        .validate_instance(&publication, "Publication")
        .await?;

    println!("✓ Cross-reference validation:");
    println!(
        "  Author: {}",
        if author_result.is_valid() {
            "PASSED"
        } else {
            "FAILED"
        }
    );
    println!(
        "  Publication: {}",
        if pub_result.is_valid() {
            "PASSED"
        } else {
            "FAILED"
        }
    );

    println!();
    Ok(())
}

/// Example 3: Code Generation
async fn code_generation_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🏗️  Example 3: Code Generation");
    println!("------------------------------");

    let schema_yaml = r#"
id: https://example.org/codegen-schema
name: codegen_schema
description: Schema for code generation examples

classes:
  Product:
    description: A product in an e-commerce system
    slots:
      - id
      - name
      - price
      - category
      - in_stock
    slot_usage:
      id:
        identifier: true
        required: true
      price:
        range: float
        minimum_value: 0.0
      category:
        range: ProductCategory

slots:
  id: {range: string, required: true}
  name: {range: string, required: true}
  price: {range: float}
  category: {range: ProductCategory}
  in_stock: {range: boolean}

enums:
  ProductCategory:
    permissible_values:
      electronics: {description: "Electronic devices"}
      clothing: {description: "Clothing and accessories"}
      books: {description: "Books and publications"}
      home: {description: "Home and garden"}

types:
  string: {base: str}
  float: {base: float}
  boolean: {base: bool}
"#;

    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;

    // Generate Python dataclasses
    let python_generator = PythonDataclassGenerator::new();
    let python_code = python_generator.generate(&schema)?;
    println!(
        "✓ Python dataclass generated ({} lines)",
        python_code.lines().count()
    );
    println!("  Preview:");
    for line in python_code.lines().take(5) {
        println!("    {}", line);
    }
    println!("    ...");

    // Generate TypeScript interfaces
    let typescript_generator = TypeScriptGenerator::new();
    let typescript_code = typescript_generator.generate(&schema)?;
    println!(
        "✓ TypeScript interfaces generated ({} lines)",
        typescript_code.lines().count()
    );
    println!("  Preview:");
    for line in typescript_code.lines().take(5) {
        println!("    {}", line);
    }
    println!("    ...");

    println!();
    Ok(())
}

/// Example 4: Real-world Integration
async fn performance_optimization_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("🌍 Example 4: Real-world Integration");
    println!("------------------------------------");

    // Simulate loading data from multiple sources
    let json_data = r#"[
        {
            "id": "user:001",
            "name": "Alice Johnson",
            "email": "alice@company.com",
            "department": "Engineering"
        },
        {
            "id": "user:002",
            "name": "Bob Smith",
            "email": "bob@company.com",
            "department": "Marketing"
        }
    ]"#;

    let schema_yaml = r#"
id: https://company.com/user-schema
name: user_schema
classes:
  User:
    slots: [id, name, email, department]
    slot_usage:
      id: {identifier: true, required: true}
      name: {required: true}
      email: {pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"}
      department: {range: Department}
slots:
  id: {range: string}
  name: {range: string}
  email: {range: string}
  department: {range: Department}
enums:
  Department:
    permissible_values:
      Engineering: {}
      Marketing: {}
      Sales: {}
      HR: {}
types:
  string: {base: str}
"#;

    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    // Parse and validate JSON data
    let users: Vec<serde_json::Value> = serde_json::from_str(json_data)?;
    let mut valid_count = 0;
    let mut error_count = 0;

    for user in &users {
        let result = engine.validate_instance(user, "User").await?;
        if result.is_valid() {
            valid_count += 1;
        } else {
            error_count += 1;
        }
    }

    println!("✓ Processed {} users", users.len());
    println!("  Valid: {}, Errors: {}", valid_count, error_count);

    println!();
    Ok(())
}

/// Example 5: Performance Optimization
async fn typedb_integration_example() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("⚡ Example 5: Performance Optimization");
    println!("--------------------------------------");

    let schema_yaml = r#"
id: https://example.org/perf-schema
name: perf_schema
classes:
  Record:
    slots: [id, value, timestamp]
    slot_usage:
      id: {identifier: true, required: true}
slots:
  id: {range: string}
  value: {range: float}
  timestamp: {range: string}
types:
  string: {base: str}
  float: {base: float}
"#;

    let parser = YamlParser::new();
    let schema = Arc::new(parser.parse_str(schema_yaml)?);

    // Create reusable validation engine
    let engine = ValidationEngine::new(Arc::clone(&schema));

    // Generate test data
    let start_time = std::time::Instant::now();
    let mut records = Vec::new();
    for i in 0..1000 {
        records.push(json!({
            "id": format!("record:{:04}", i),
            "value": (i as f64) * 0.1,
            "timestamp": "2023-01-01T12:00:00Z"
        }));
    }
    let generation_time = start_time.elapsed();

    // Validate all records
    let validation_start = std::time::Instant::now();
    let mut results = Vec::new();
    for record in &records {
        let result = engine.validate_instance(record, "Record").await?;
        results.push(result);
    }
    let validation_time = validation_start.elapsed();

    let records_per_second = records.len() as f64 / validation_time.as_secs_f64();

    println!("✓ Performance metrics:");
    println!(
        "  Generated {} records in {:?}",
        records.len(),
        generation_time
    );
    println!(
        "  Validated {} records in {:?}",
        records.len(),
        validation_time
    );
    println!("  Throughput: {:.2} records/second", records_per_second);

    let valid_count = results.iter().filter(|r| r.is_valid()).count();
    println!(
        "  Success rate: {}/{} ({:.1}%)",
        valid_count,
        records.len(),
        (valid_count as f64 / records.len() as f64) * 100.0
    );

    println!();
    Ok(())
}
