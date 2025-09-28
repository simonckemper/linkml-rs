//! Tests for unwrap() removal - error handling verification
//!
//! This test file verifies that the removal of unwrap() calls in Phase 1
//! results in proper error handling instead of panics.

use linkml_service::{
    parser::{
        yaml_parser::YamlParser,
        json_parser::JsonParser,
        Parser as SchemaParser,
    },
    validator::{
        engine::{ValidationEngine, ValidationOptions},
        context::ValidationContext,
    },
    generator::{
        typeql_generator::TypeQLGenerator,
        python_dataclass::PythonDataclassGenerator,
        sql::SQLGenerator,
        traits::{Generator, GeneratorOptions},
        registry::GeneratorRegistry,
    },
    expression::{
        ExpressionEngine,
        evaluator::Evaluator,
        functions::FunctionRegistry,
        parser::Parser as ExpressionParser,
    },
    loader::{
        yaml::YamlLoader,
        json::JsonLoader,
        csv::CsvLoader,
    },
    schema_view::SchemaView,
};
use linkml_core::{
    error::LinkMLError,
    types::{SchemaDefinition, ClassDefinition, SlotDefinition},
};
use std::path::Path;
use tempfile::TempDir;
use std::fs;

#[test]
fn test_parser_handles_invalid_yaml() {
    let invalid_yaml = "
classes:
  Person:
    slots: [name
      - age  # Missing closing bracket above
";

    let parser = YamlParser::new();
    let result = parser.parse(invalid_yaml);

    // Should return error, not panic
    assert!(result.is_err());
    match result {
        Err(LinkMLError::Parse(msg)) => {
            assert!(msg.contains("yaml") || msg.contains("parse"));
        }
        _ => panic!("Expected ParseError"),
    }
}

#[test]
fn test_parser_handles_invalid_json() {
    let invalid_json = r#"{
    "classes": {
        "Person": {
            "slots": ["name", "age"  // Missing closing bracket
        }
    }
}"#;

    let parser = JsonParser::new();
    let result = parser.parse(invalid_json);

    // Should return error, not panic
    assert!(result.is_err());
}

#[test]
fn test_validator_handles_invalid_pattern() {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("test".to_string());

    let mut slot = SlotDefinition::default();
    slot.name = Some("email".to_string());
    // Invalid regex pattern
    slot.pattern = Some(r"[unclosed(bracket".to_string());

    let mut class = ClassDefinition::default();
    class.name = Some("Person".to_string());
    class.attributes.insert("email".to_string(), slot);

    schema.classes.insert("Person".to_string(), class);

    let engine = ValidationEngine::new();
    // ValidationContext::new();
    let context = ValidationContext::new();
    let options = ValidationOptions::default();

    // Should handle regex compilation error gracefully
    let result = engine.validate(&schema, &context, &options);
    // May succeed with warnings or fail - both are OK as long as no panic
    match result {
        Ok(report) => {
            // Check if there are warnings about the pattern
            assert!(!report.warnings.is_empty() || !report.errors.is_empty());
        }
        Err(_) => {
            // Error is also acceptable
        }
    }
}

#[test]
fn test_expression_handles_division_by_zero() {
    let engine = ExpressionEngine::new();
    let context = std::collections::HashMap::new();

    // Division by zero
    let result = engine.evaluate("10 / 0", &context);
    assert!(result.is_err());

    // Invalid syntax
    let result = engine.evaluate("1 + + 2", &context);
    assert!(result.is_err());

    // Undefined variable
    let result = engine.evaluate("undefined_var * 5", &context);
    assert!(result.is_err());
}

#[test]
fn test_generator_handles_special_characters() {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("test".to_string());

    // Class with special characters that could break code generation
    let mut class = ClassDefinition::default();
    class.name = Some("Person-With-Dashes".to_string());
    class.description = Some("Class with \"quotes\" and 'apostrophes'".to_string());

    let mut slot = SlotDefinition::default();
    slot.name = Some("email@address".to_string()); // Invalid identifier
    slot.range = Some("string".to_string());

    class.attributes.insert(slot.name.clone().expect("Test operation failed"), slot);
    schema.classes.insert(class.name.clone().expect("Test operation failed"), class);

    // Test different generators
    let generators: Vec<Box<dyn Generator>> = vec![
        Box::new(TypeQLGenerator::new()),
        Box::new(PythonDataclassGenerator::new()),
        Box::new(SQLGenerator::new()),
    ];

    let options = GeneratorOptions::default();

    for generator in generators {
        let result = generator.generate(&schema, &options);
        // Should either sanitize names or return error - not panic
        match result {
            Ok(output) => {
                // Check that problematic characters are handled
                assert!(!output.contains("email@address"));
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }
}

#[test]
fn test_loader_handles_missing_files() {
    let yaml_loader = YamlLoader::new();
    let json_loader = JsonLoader::new();
    let csv_loader = CsvLoader::new();

    // Non-existent files
    let result = yaml_loader.load(Path::new("/non/existent/file.yaml"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), LinkMLError::IO(_));

    let result = json_loader.load(Path::new("/non/existent/file.json"));
    assert!(result.is_err());

    let result = csv_loader.load(Path::new("/non/existent/file.csv"));
    assert!(result.is_err());
}

#[test]
fn test_schema_view_handles_missing_elements() {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("test".to_string());

    // Add class that references non-existent parent
    let mut class = ClassDefinition::default();
    class.name = Some("Child".to_string());
    class.is_a = Some("NonExistentParent".to_string());

    schema.classes.insert("Child".to_string(), class);

    let view = SchemaView::new(schema);

    // Should handle missing parent gracefully
    let parents = view.class_parents("Child");
    assert!(parents.is_ok());

    // Should handle non-existent class
    let result = view.get_class("NonExistentClass");
    assert!(result.is_none());
}

#[test]
fn test_generator_registry_duplicate_registration() {
    let mut registry = GeneratorRegistry::new();

    // First registration should succeed
    let result = registry.register("test", Box::new(TypeQLGenerator::new());
    assert!(result.is_ok());

    // Second registration with same name should fail gracefully
    let result = registry.register("test", Box::new(SQLGenerator::new());
    assert!(result.is_err());
}

#[test]
fn test_complete_workflow_with_errors() {
    let temp_dir = TempDir::new().expect("create temp dir");

    // Create schema with various issues
    let schema_yaml = r#"
id: test_schema
name: TestSchema

classes:
  Person:
    is_a: NonExistentBase  # Missing base class
    slots:
      - name
      - email
      - friends

  InvalidClass:
    # Missing name
    slots:
      - field1

slots:
  name:
    range: string
    required: true

  email:
    range: string
    pattern: "[invalid(regex"  # Invalid regex

  friends:
    range: Person
    multivalued: true

  field1:
    range: NonExistentType  # Invalid type
"#;

    let schema_path = temp_dir.path().join("schema.yaml");
    fs::write(&schema_path, schema_yaml).expect("write schema");

    // Parse schema
    let parser = YamlParser::new();
    let schema = match parser.parse_file(&schema_path) {
        Ok(s) => s,
        Err(e) => {
            // Parse error is acceptable
            println!("Parse error: {}", e);
            return;
        }
    };

    // Validate schema
    let engine = ValidationEngine::new();
    // ValidationContext::new();
    let context = ValidationContext::new();
    let options = ValidationOptions {
        strict: false,
        ..Default::default()
    };

    match engine.validate(&schema, &context, &options) {
        Ok(report) => {
            // Should have warnings/errors but not panic
            assert!(!report.warnings.is_empty() || !report.errors.is_empty());
        }
        Err(e) => {
            println!("Validation error: {}", e);
        }
    }

    // Try to generate code
    let generator = PythonDataclassGenerator::new();
    let gen_options = GeneratorOptions::default();

    match generator.generate(&schema, &gen_options) {
        Ok(output) => {
            println!("Generated {} bytes", output.len());
        }
        Err(e) => {
            println!("Generation error: {}", e);
        }
    }

    // No panics should occur throughout this workflow
}

/// Test that concurrent operations don't panic
#[tokio::test]
async fn test_concurrent_operations_no_panic() {
    use tokio::task::JoinSet;
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};
use linkml_core::error::LinkMLError;

    let mut schema = SchemaDefinition::default();
    schema.name = Some("concurrent_test".to_string());

    let mut class = ClassDefinition::default();
    class.name = Some("TestClass".to_string());
    schema.classes.insert("TestClass".to_string(), class);

    let mut tasks = JoinSet::new();

    // Spawn multiple concurrent tasks
    for i in 0..5 {
        let schema_clone = schema.clone();

        tasks.spawn(async move {
            // Each task does different operations
            match i % 3 {
                0 => {
                    // Validation
                    let engine = ValidationEngine::new();
                    // ValidationContext::new();
                    let context = ValidationContext::new();
                    let options = ValidationOptions::default();
                    let _ = engine.validate(&schema_clone, &context, &options);
                }
                1 => {
                    // Generation
                    let generator = TypeQLGenerator::new();
                    let _ = generator.generate(&schema_clone, &GeneratorOptions::default());
                }
                _ => {
                    // Schema view operations
                    let view = SchemaView::new(schema_clone);
                    let _ = view.all_classes();
                    let _ = view.all_slots();
                }
            }
        });
    }

    // All tasks should complete without panic
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok(), "Task panicked");
    }
}
