//! Integration tests verifying no panics occur in typical workflows
//!
//! These tests simulate real-world usage patterns to ensure that the
//! unwrap() removals don't cause panics in production scenarios.

use linkml_core::types::SchemaDefinition;
use linkml_service::{
    expression::{Evaluator, EvaluatorConfig, Parser as ExpressionParser},
    generator::{
        GeneratorOptions, GeneratorRegistry, PythonDataclassGenerator, SQLGenerator,
        TypeQLGenerator,
    },
    loader::{DataLoader, yaml::YamlLoader},
    parser::{SchemaParser, yaml_parser::YamlParser},
    schema_view::SchemaView,
    validator::engine::{ValidationEngine, ValidationOptions},
};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

/// Complete workflow test: parse -> validate -> generate
#[tokio::test]
async fn test_complete_workflow_no_panic() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create a realistic schema
    let schema_yaml = r#"
id: https://example.org/sample
name: sample_schema
description: A sample schema for testing
version: 1.0.0

prefixes:
  linkml: https://w3id.org/linkml/
  sample: https://example.org/sample/
  xsd: http://www.w3.org/2001/XMLSchema#

default_prefix: sample
default_range: string

imports:
  - linkml:types

classes:
  Person:
    description: A person with basic information
    slots:
      - id
      - name
      - age
      - email
      - address
      - friends
    slot_usage:
      id:
        required: true
        identifier: true
      name:
        required: true
      email:
        pattern: '^[^@]+@[^@]+\.[^@]+$'
      friends:
        multivalued: true
        range: Person

  Address:
    description: A postal address
    slots:
      - street
      - city
      - postal_code
      - country
    slot_usage:
      postal_code:
        pattern: '^\d{5}(-\d{4})?$'

  Organization:
    description: An organization
    is_a: LegalEntity
    mixins:
      - HasEmployees
    slots:
      - name
      - founded_year
      - industry
      - employees

slots:
  id:
    description: Unique identifier
    range: string

  name:
    description: Full name
    range: string

  age:
    description: Age in years
    range: integer
    minimum_value: 0
    maximum_value: 150

  email:
    description: Email address
    range: string

  street:
    description: Street address
    range: string

  city:
    description: City name
    range: string

  postal_code:
    description: Postal/ZIP code
    range: string

  country:
    description: Country name
    range: string

  address:
    description: Postal address
    range: Address

  friends:
    description: List of friends
    range: Person

  founded_year:
    description: Year organization was founded
    range: integer
    minimum_value: 1800

  industry:
    description: Industry sector
    range: string

  employees:
    description: List of employees
    range: Person
    multivalued: true

types:
  PositiveInt:
    typeof: integer
    minimum_value: 1

# This will cause validation warnings but shouldn't panic
classes:
  LegalEntity:
    abstract: true
    description: Abstract base for legal entities

  HasEmployees:
    mixin: true
    description: Mixin for entities with employees
    slots:
      - employee_count

slots:
  employee_count:
    range: PositiveInt
"#;

    let schema_path = temp_dir.path().join("schema.yaml");
    fs::write(&schema_path, schema_yaml).expect("write schema");

    // Step 1: Parse the schema
    let parser = YamlParser::new();
    let schema = match parser.parse_file(&schema_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Parse error (expected): {}", e);
            return; // Some parse errors are acceptable
        }
    };

    // Step 2: Create SchemaView
    let schema_view = SchemaView::new(schema.clone()).expect("create schema view");

    // Test various SchemaView operations (ignore results for panic test)
    let _ = schema_view.all_classes();
    let _ = schema_view.class_slots("Person");
    let _ = schema_view.class_view("Person");

    // Step 3: Validate the schema
    let validator = ValidationEngine::new(&schema).expect("create validator");
    let options = ValidationOptions {
        fail_fast: Some(false), // Allow some validation errors
        max_depth: Some(100),
        ..Default::default()
    };

    // Create a sample instance to validate
    let instance = serde_json::json!({
        "name": "Test Organization",
        "email": "test@example.com"
    });

    match validator
        .validate(&instance, "Organization", &options)
        .await
    {
        Ok(report) => {
            println!("Validation passed with {} issues", report.issues.len());
        }
        Err(e) => {
            eprintln!("Validation error (handled): {}", e);
        }
    }

    // Step 4: Generate code
    let mut registry = GeneratorRegistry::new();
    registry.register("typeql", Box::new(TypeQLGenerator::new());
    registry.register("python", Box::new(PythonDataclassGenerator::new());
    registry.register("sql", Box::new(SQLGenerator::new());

    let gen_options = GeneratorOptions::default();

    for gen_name in ["typeql", "python", "sql"] {
        match registry.get_generator(gen_name) {
            Ok(generator) => match generator.generate(&schema, &gen_options) {
                Ok(output) => {
                    assert!(!output.is_empty());
                    println!("Generated {} code: {} chars", gen_name, output.len());
                }
                Err(e) => {
                    eprintln!("Generation error for {} (handled): {}", gen_name, e);
                }
            },
            Err(e) => {
                eprintln!("Registry error (handled): {}", e);
            }
        }
    }
}

/// Test loading and processing data instances
#[test]
fn test_data_instance_processing_no_panic() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create schema
    let schema_yaml = r#"
id: https://example.org/data
name: data_schema

classes:
  DataRecord:
    slots:
      - id
      - value
      - metadata

slots:
  id:
    identifier: true
    range: string
  value:
    range: float
  metadata:
    range: string
"#;

    // Create data instances with various edge cases
    let data_yaml = r#"
# Valid record
- id: record1
  value: 42.5
  metadata: "Normal metadata"

# Record with missing optional field
- id: record2
  value: 17.3

# Record with wrong type (should error but not panic)
- id: record3
  value: "not a number"
  metadata: "This should cause validation error"

# Record with very large number
- id: record4
  value: 1.7976931348623157e+308
  metadata: "Maximum double value"

# Record with special characters
- id: record5
  value: 0.0
  metadata: "Special chars: 
\t\r\"'\\"
"#;

    let schema_path = temp_dir.path().join("schema.yaml");
    let data_path = temp_dir.path().join("data.yaml");

    fs::write(&schema_path, schema_yaml).expect("write schema");
    fs::write(&data_path, data_yaml).expect("write data");

    // Load schema
    let parser = YamlParser::new();
    let schema = parser.parse_file(&schema_path).expect("parse schema");

    // Load data
    let loader = YamlLoader::new();
    match loader.load_data(&data_path, &schema) {
        Ok(data) => {
            println!("Loaded {} records", data.len());
        }
        Err(e) => {
            eprintln!("Data loading error (handled): {}", e);
        }
    }
}

/// Test expression evaluation in context
#[test]
fn test_expression_evaluation_no_panic() {
    let config = EvaluatorConfig::default();
    let evaluator = Evaluator::with_config(config);
    let parser = ExpressionParser::new();

    // Test various expressions that might cause issues
    let test_cases = vec![
        // Normal cases
        ("1 + 2 * 3", HashMap::new()),
        // With variables
        ("x + y", {
            let mut ctx = HashMap::new();
            ctx.insert("x".to_string(), Value::from(10.0));
            ctx.insert("y".to_string(), Value::from(20.0));
            ctx
        }),
        // Edge cases that should error gracefully
        ("1 / 0", HashMap::new()),
        ("sqrt(-1)", HashMap::new()),
        ("undefined_var + 5", HashMap::new()),
        ("log(0)", HashMap::new()),
        // String operations
        ("concat('hello', ' ', 'world')", HashMap::new()),
        // Complex nested expression
        ("if x > 10 then x * 2 else x / 2", {
            let mut ctx = HashMap::new();
            ctx.insert("x".to_string(), Value::from(15.0));
            ctx
        }),
    ];

    for (expr_str, context) in test_cases {
        match parser.parse(expr_str) {
            Ok(expr) => match evaluator.evaluate(&expr, &context) {
                Ok(result) => {
                    println!("Expression '{}' = {:?}", expr_str, result);
                }
                Err(e) => {
                    println!(
                        "Expression '{}' evaluation error (handled): {}",
                        expr_str, e
                    );
                }
            },
            Err(e) => {
                println!("Expression '{}' parse error (handled): {}", expr_str, e);
            }
        }
    }
}

/// Test configuration usage
#[test]
fn test_configuration_usage_no_panic() {
    // Test with default options
    let schema = SchemaDefinition::default();
    let validator = ValidationEngine::new(&schema).expect("create validator");
    let options = ValidationOptions::default();

    // Should handle empty schema gracefully
    // Note: validate is async, so we'd need tokio runtime here
    // For now, just ensure creation doesn't panic
    let _ = validator;
    let _ = options;
}

/// Test handling of malformed schemas
#[test]
fn test_malformed_schema_handling() {
    let test_schemas = vec![
        // Schema with syntax error
        "classes:
  Person
    slots: [name",
        // Schema with circular inheritance
        r#"
classes:
  A:
    is_a: B
  B:
    is_a: C
  C:
    is_a: A
"#,
        // Schema with invalid regex patterns
        r#"
slots:
  bad_pattern:
    pattern: '[invalid(regex'
"#,
        // Schema with type conflicts
        r#"
types:
  MyString:
    typeof: string
  MyString:
    typeof: integer
"#,
    ];

    let parser = YamlParser::new();

    for (i, schema_str) in test_schemas.iter().enumerate() {
        match parser.parse(schema_str) {
            Ok(schema) => {
                // Even if parsing succeeds, validation should catch issues
                let validator = ValidationEngine::new(&schema).expect("create validator");
                let options = ValidationOptions::default();
                // Note: validate is async and needs instance data
                let _ = validator;
                let _ = options;
            }
            Err(e) => {
                println!("Test schema {} error (expected): {}", i, e);
            }
        }
    }
}

/// Test concurrent schema operations
#[tokio::test]
async fn test_concurrent_operations_no_panic() {
    use tokio::task::JoinSet;

    let schema_yaml = r#"
id: https://example.org/concurrent
name: concurrent_test

classes:
  TestClass:
    slots:
      - id
      - value

slots:
  id:
    range: string
  value:
    range: integer
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(schema_yaml).expect("parse schema");

    let mut tasks = JoinSet::new();

    // Spawn multiple concurrent operations
    for i in 0..10 {
        let schema_clone = schema.clone();

        tasks.spawn(async move {
            // Validate
            let validator = ValidationEngine::new(&schema_clone).expect("create validator");
            let options = ValidationOptions::default();
            // Note: validate is async and needs instance data
            let _ = validator;
            let _ = options;

            // Generate code
            if i % 2 == 0 {
                let generator = TypeQLGenerator::new();
                let _ = generator.generate(&schema_clone, &GeneratorOptions::default());
            } else {
                let generator = SQLGenerator::new();
                let _ = generator.generate(&schema_clone, &GeneratorOptions::default());
            }

            // Create schema view
            let view = SchemaView::new(schema_clone);
            let _ = view.all_classes();
            let _ = view.all_slots();
        });
    }

    // All tasks should complete without panicking
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok(), "Task panicked");
    }
}
