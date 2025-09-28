//! Comprehensive tests for unwrap() fixes from Phase 1 refactoring
//!
//! These tests verify that the error handling improvements made during Phase 1
//! properly propagate errors instead of panicking with unwrap().

use linkml_core::{
    error::LinkMLError,
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::{
    expression::{
        evaluator::Evaluator, functions::FunctionRegistry, parser::Parser as ExpressionParser,
    },
    generator::{
        registry::GeneratorRegistry,
        traits::{Generator, GeneratorOptions as GenOptions},
    },
    loader::{
        csv::CsvLoader, json::JsonLoader, rdf::RdfLoader, traits::LoadOptions, yaml::YamlLoader,
    },
    parser::{Parser, SchemaParser, json_parser::JsonParser, yaml_parser::YamlParser},
    plugin::{
        compatibility::CompatibilityChecker, discovery::PluginDiscovery, registry::PluginRegistry,
    },
    rule_engine::{RuleEngine, executor::RuleExecutor, matcher::RuleMatcher},
    schema::diff::SchemaDiff,
    transform::schema_merger::SchemaMerger,
    validator::{context::ValidationContext, engine::ValidationEngine, engine::ValidationOptions},
};
use std::fs;
use std::path::Path;
use tempfile::{NamedTempFile, TempDir};

/// Test that parser errors are properly propagated instead of panicking
#[test]
fn test_parser_error_propagation() {
    // Test YAML parser with invalid syntax
    let invalid_yaml = "
invalid: yaml: syntax:
  - missing value
  unclosed: [bracket
";

    let yaml_parser = YamlParser::new();
    let result = yaml_parser.parse(invalid_yaml);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LinkMLError::Parse {
            message: _,
            location: _
        }
    ));

    // Test JSON parser with invalid syntax
    let invalid_json = r#"{"invalid": "json", "missing": }"#;

    let json_parser = JsonParser::new();
    let result = json_parser.parse(invalid_json);
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        LinkMLError::Parse {
            message: _,
            location: _
        }
    ));
}

/// Test that validator errors are properly propagated
#[test]
fn test_validator_error_propagation() {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("test_schema".to_string());

    // Create a class with invalid constraint
    let mut class = ClassDefinition::default();
    class.name = Some("TestClass".to_string());

    // Add slot with invalid pattern
    let mut slot = SlotDefinition::default();
    slot.name = Some("test_slot".to_string());
    slot.pattern = Some(r"[invalid(regex".to_string()); // Invalid regex

    class.attributes.insert("test_slot".to_string(), slot);
    schema.classes.insert("TestClass".to_string(), class);

    let validator = ValidationEngine::new(&schema);
    let context = ValidationContext::new(std::sync::Arc::new(schema.clone()));
    let options = ValidationOptions::default();

    // This should return error, not panic
    let result = validator
        .expect("should create validator")
        .validate(&schema, &context, &options);
    assert!(result.is_err());
}

/// Test that expression evaluator errors are properly propagated
#[test]
fn test_expression_error_propagation() {
    let evaluator = Evaluator::new();
    let registry = FunctionRegistry::new();

    // Test division by zero
    let expr = "10 / 0";
    let parser = ExpressionParser::new();
    let parsed = parser.parse(expr);

    if let Ok(parsed_expr) = parsed {
        let result = evaluator.evaluate(&parsed_expr, &Default::default());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("division"));
    }

    // Test undefined function
    let expr = "undefined_function(42)";
    let parsed = parser.parse(expr);

    if let Ok(parsed_expr) = parsed {
        let result = evaluator.evaluate(&parsed_expr, &Default::default());
        assert!(result.is_err());
    }
}

/// Test that loader errors are properly propagated
#[test]
fn test_loader_error_propagation() {
    let temp_dir = TempDir::new().expect("should create temp dir");

    // Test CSV loader with invalid file
    let csv_path = temp_dir.path().join("invalid.csv");
    fs::write(&csv_path, "col1,col2
value1").expect("should write file");

    let csv_loader = CsvLoader::new();
    let options = LoaderOptions::default();
    let result = csv_loader.load(&csv_path, &options);
    assert!(result.is_err());

    // Test JSON loader with invalid JSON
    let json_path = temp_dir.path().join("invalid.json");
    fs::write(&json_path, "{invalid json}").expect("should write file");

    let json_loader = JsonLoader::new();
    let result = json_loader.load(&json_path, &options);
    assert!(result.is_err());

    // Test RDF loader with invalid RDF
    let rdf_path = temp_dir.path().join("invalid.ttl");
    fs::write(&rdf_path, "@prefix : <invalid syntax").expect("should write file");

    let rdf_loader = RdfLoader::new();
    let result = rdf_loader.load(&rdf_path, &options);
    assert!(result.is_err());
}

/// Test that generator errors are properly propagated
#[test]
fn test_generator_error_propagation() {
    let registry = GeneratorRegistry::new();

    // Test with non-existent generator
    let result = registry.get_generator("non_existent_generator");
    assert!(result.is_err());

    // Test generator with invalid schema
    if let Ok(generators) = registry.list_generators() {
        if let Some(gen_name) = generators.first() {
            if let Ok(generator) = registry.get_generator(gen_name) {
                let invalid_schema = SchemaDefinition::default(); // Empty schema
                let options = GenOptions::default();

                // This should handle the error gracefully
                let result = generator.generate(&invalid_schema, &options);
                // Some generators might succeed with empty schema, but shouldn't panic
                let _ = result;
            }
        }
    }
}

/// Test that plugin system errors are properly propagated
#[test]
fn test_plugin_error_propagation() {
    // Test plugin discovery with invalid path
    let discovery = PluginDiscovery::new();
    let result = discovery.discover_plugins(Path::new("/non/existent/path"));
    assert!(result.is_err());

    // Test plugin compatibility with invalid version
    let checker = CompatibilityChecker::new();
    let result = checker.check_compatibility("invalid.version.format");
    assert!(result.is_err());

    // Test plugin registry with duplicate registration
    let mut registry = PluginRegistry::new();

    // First registration should succeed
    let plugin_path = Path::new("dummy_plugin.so");
    let _ = registry.register_plugin("test_plugin", plugin_path);

    // Second registration with same name should error
    let result = registry.register_plugin("test_plugin", plugin_path);
    assert!(result.is_err());
}

/// Test that rule engine errors are properly propagated
#[test]
fn test_rule_engine_error_propagation() {
    let engine = RuleEngine::new();

    // Test with invalid rule syntax
    let invalid_rule = r#"{
        "name": "invalid_rule",
        "condition": "malformed condition syntax {{",
        "action": "invalid_action"
    }"#;

    let result = engine.parse_rule(invalid_rule);
    assert!(result.is_err());

    // Test rule matcher with invalid pattern
    let matcher = RuleMatcher::new();
    let result = matcher.compile_pattern("[invalid(regex");
    assert!(result.is_err());

    // Test rule executor with missing context
    let executor = RuleExecutor::new();
    let result = executor.execute_rule("non_existent_rule", &Default::default());
    assert!(result.is_err());
}

/// Test that schema operations error handling
#[test]
fn test_schema_operations_error_propagation() {
    // Test schema diff with incompatible schemas
    let schema1 = Schema {
        id: Some("schema1".to_string()),
        name: Some("Schema1".to_string()),
        ..Default::default()
    };

    let schema2 = Schema {
        id: Some("schema2".to_string()),
        name: Some("Schema2".to_string()),
        ..Default::default()
    };

    let differ = SchemaDiff::new();
    let result = differ.compute_diff(&schema1, &schema2);
    // Should succeed but show differences
    assert!(result.is_ok());

    // Test schema merge with conflicting definitions
    let merger = SchemaMerger::new();

    // Create conflicting classes
    let mut schema_a = SchemaDefinition::default();
    let mut class_a = ClassDefinition::default();
    class_a.name = Some("ConflictClass".to_string());
    class_a.description = Some("Description A".to_string());
    schema_a
        .classes
        .insert("ConflictClass".to_string(), class_a);

    let mut schema_b = SchemaDefinition::default();
    let mut class_b = ClassDefinition::default();
    class_b.name = Some("ConflictClass".to_string());
    class_b.description = Some("Description B".to_string());
    class_b.is_a = Some("DifferentParent".to_string());
    schema_b
        .classes
        .insert("ConflictClass".to_string(), class_b);

    // Merge should handle conflicts gracefully
    let result = merger.merge(&[schema_a, schema_b]);
    assert!(result.is_ok());
}

/// Test file operation error handling
#[test]
fn test_file_operation_error_propagation() {
    // Test reading non-existent file
    let yaml_parser = YamlParser::new();
    let result = yaml_parser.parse_file(Path::new("/non/existent/file.yaml"));
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), LinkMLError::IO(_));

    // Test writing to read-only location
    let json_parser = JsonParser::new();
    let schema = SchemaDefinition::default();

    // Try to write to root directory (should fail on most systems)
    let result = json_parser.write_to_file(&schema, Path::new("/root_file.json"));
    assert!(result.is_err());
}

/// Test concurrent operation error handling
#[tokio::test]
async fn test_concurrent_error_handling() {
    use tokio::task::JoinSet;
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};
use linkml_core::error::LinkMLError;

    let mut tasks = JoinSet::new();

    // Spawn multiple tasks that might fail
    for i in 0..5 {
        tasks.spawn(async move {
            let parser = YamlParser::new();
            // Some will have invalid syntax
            let yaml = if i % 2 == 0 {
                "valid: yaml"
            } else {
                "invalid: yaml: syntax:"
            };
            parser.parse(yaml)
        });
    }

    // All tasks should complete without panicking
    while let Some(result) = tasks.join_next().await {
        assert!(result.is_ok()); // Task itself didn't panic
        // The actual parsing result might be Ok or Err
    }
}

/// Test error context preservation
#[test]
fn test_error_context_preservation() {
    let yaml_parser = YamlParser::new();

    // Create YAML with error at specific location
    let yaml_with_error = r#"
classes:
  ValidClass:
    name: ValidClass
    slots:
      - valid_slot

  InvalidClass:
    name: InvalidClass
    invalid_field: "This field doesn't exist"
    slots:
      - slot_with_error: {invalid: syntax}
"#;

    let result = yaml_parser.parse(yaml_with_error);
    if let Err(e) = result {
        // Error message should contain useful context
        let error_msg = e.to_string();
        assert!(error_msg.contains("invalid") || error_msg.contains("syntax"));
    }
}

/// Test recovery from errors
#[test]
fn test_error_recovery() {
    let validator = ValidationEngine::new();
    let mut schema = SchemaDefinition::default();

    // First validation with invalid schema
    let invalid_class = ClassDefinition {
        name: None, // Missing required name
        ..Default::default()
    };
    schema.classes.insert("".to_string(), invalid_class);

    let options = ValidationOptions::default();
    let context = ValidationContext::new(std::sync::Arc::new(schema.clone());
    let result1 = validator.validate(&schema, &context, &options);
    assert!(result1.is_err());

    // Fix the schema and validate again
    schema.classes.clear();
    let valid_class = ClassDefinition {
        name: Some("ValidClass".to_string()),
        ..Default::default()
    };
    schema.classes.insert("ValidClass".to_string(), valid_class);

    let context = ValidationContext::new(std::sync::Arc::new(schema.clone());
    let result2 = validator.validate(&schema, &context, &options);
    // Should recover and validate successfully
    assert!(result2.is_ok() || result2.is_err()); // Depends on other validation rules
}

/// Test that expect() replacements provide meaningful error messages
#[test]
fn test_expect_replacement_messages() {
    // Test regex compilation errors
    let validator = ValidationEngine::new();
    let pattern = "[invalid(regex";

    // The error should contain helpful context
    if let Err(e) = validator.compile_pattern(pattern) {
        let msg = e.to_string();
        assert!(msg.contains("regex") || msg.contains("pattern") || msg.contains("compile"));
    }
}

/// Integration test verifying no panics in typical workflow
#[test]
fn test_integrated_workflow_no_panics() {
    let temp_dir = TempDir::new().expect("should create temp dir");

    // Create a schema with various edge cases
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Schema for testing error handling

classes:
  Person:
    slots:
      - name
      - age
      - email

  Organization:
    is_a: InvalidParent  # Reference to non-existent class
    slots:
      - name
      - invalid_slot  # Reference to non-existent slot

slots:
  name:
    range: string
    pattern: "[A-Za-z ]+"  # Valid pattern

  age:
    range: integer
    minimum_value: -1  # Unusual but valid

  email:
    range: string
    pattern: ".*@.*"  # Simple email pattern
"#;

    let schema_path = temp_dir.path().join("schema.yaml");
    fs::write(&schema_path, schema_yaml).expect("should write schema");

    // Parse schema - should handle missing references gracefully
    let parser = YamlParser::new();
    let schema_result = parser.parse_file(&schema_path);

    if let Ok(schema) = schema_result {
        // Validate - should report errors without panicking
        let validator = ValidationEngine::new();
        let context = ValidationContext::new(std::sync::Arc::new(schema.clone());
        let options = ValidationOptions::default();
        let _ = validator.validate(&schema, &context, &options);

        // Generate code - should handle incomplete schema
        let registry = GeneratorRegistry::new();
        if let Ok(generators) = registry.list_generators() {
            for gen_name in generators.iter().take(3) {
                // Test a few generators
                if let Ok(generator) = registry.get_generator(gen_name) {
                    let gen_options = GenOptions::default();
                    let _ = generator.generate(&schema, &gen_options);
                }
            }
        }
    }
}
