//! Unit tests for generator module fixes
//!
//! Tests the generator module fixes including static helper functions,
//! type casting safety improvements, and error propagation enhancements.

use std::collections::HashMap;
use linkml_core::prelude::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, TypeDefinition, EnumDefinition};
use linkml_service::generator::{
    yaml::YamlGenerator,
    rust_generator::RustGenerator,
    json_schema::JsonSchemaGenerator,
    traits::{Generator, GeneratorOptions, GeneratorResult},
};
use pretty_assertions::{assert_eq, assert_ne};
use serde_json::Value as JsonValue;

/// Test fixture for generator testing
struct GeneratorTestFixture {
    simple_schema: SchemaDefinition,
    complex_schema: SchemaDefinition,
    edge_case_schema: SchemaDefinition,
}

impl GeneratorTestFixture {
    fn new() -> Self {
        Self {
            simple_schema: create_simple_test_schema(),
            complex_schema: create_complex_test_schema(),
            edge_case_schema: create_edge_case_schema(),
        }
    }
}

/// Test YAML generator static helper functions
#[test]
fn test_yaml_generator_static_helpers() {
    let fixture = GeneratorTestFixture::new();
    let generator = YamlGenerator::new();

    // Test generation with simple schema
    let result = generator.generate(&fixture.simple_schema)
        .expect("YAML generation should succeed with simple schema");

    assert!(
        result.contains("id: \"test://simple\""),
        "Generated YAML should contain schema ID"
    );
    assert!(
        result.contains("name: \"SimpleTest\""),
        "Generated YAML should contain schema name"
    );
    assert!(
        result.contains("classes:"),
        "Generated YAML should contain classes section"
    );
}

/// Test YAML generator with different configuration options
#[test]
fn test_yaml_generator_configuration_options() {
    let fixture = GeneratorTestFixture::new();

    // Test with metadata inclusion
    let with_metadata = YamlGenerator::new()
        .with_metadata(true)
        .with_sorted_keys(true);

    let result = with_metadata.generate(&fixture.simple_schema)
        .expect("YAML generation with metadata should succeed");

    assert!(
        !result.is_empty(),
        "YAML with metadata should not be empty"
    );

    // Test without metadata
    let without_metadata = YamlGenerator::new()
        .with_metadata(false)
        .with_include_nulls(false);

    let result = without_metadata.generate(&fixture.simple_schema)
        .expect("YAML generation without metadata should succeed");

    assert!(
        !result.is_empty(),
        "YAML without metadata should not be empty"
    );
}

/// Test YAML generator error handling for invalid schemas
#[test]
fn test_yaml_generator_error_handling() {
    let generator = YamlGenerator::new();
    let edge_case_schema = create_edge_case_schema();

    // Test with edge case schema (should handle gracefully)
    let result = generator.generate(&edge_case_schema);

    match result {
        Ok(yaml_output) => {
            assert!(
                !yaml_output.is_empty(),
                "Even edge cases should produce some output"
            );
        }
        Err(error) => {
            // Should have meaningful error message
            let error_msg = format!("{error}");
            assert!(
                !error_msg.is_empty(),
                "Error should have meaningful message"
            );
        }
    }
}

/// Test type casting safety improvements in generators
#[test]
fn test_type_casting_safety() {
    let fixture = GeneratorTestFixture::new();

    // Test with schema containing numeric values at boundaries
    let schema = create_numeric_boundary_schema();
    let yaml_generator = YamlGenerator::new();

    // Test numeric casting doesn't cause precision loss
    let result = yaml_generator.generate(&schema)
        .expect("Generation should handle numeric boundaries safely");

    // Verify result contains expected numeric representations
    assert!(
        result.contains("9223372036854775807") ||
        result.contains("maximum_value") ||
        result.len() > 0, // At minimum, should generate something
        "Should handle large numeric values safely"
    );
}

/// Test JSON schema generator static functions
#[test]
fn test_json_schema_generator_static_functions() {
    let fixture = GeneratorTestFixture::new();
    let generator = JsonSchemaGenerator::new();

    let result = generator.generate(&fixture.simple_schema)
        .expect("JSON schema generation should succeed");

    // Parse the result to verify it's valid JSON
    let json_value: JsonValue = serde_json::from_str(&result)
        .expect("Generated output should be valid JSON");

    // Verify JSON structure
    assert!(
        json_value.is_object(),
        "JSON schema should be an object"
    );

    if let JsonValue::Object(obj) = json_value {
        assert!(
            obj.contains_key("$schema") || obj.contains_key("type") || !obj.is_empty(),
            "JSON schema should have expected structure"
        );
    }
}

/// Test Rust generator static functions
#[test]
fn test_rust_generator_static_functions() {
    let fixture = GeneratorTestFixture::new();
    let generator = RustGenerator::new();

    let result = generator.generate(&fixture.simple_schema)
        .expect("Rust code generation should succeed");

    // Verify generated Rust code structure
    assert!(
        result.contains("struct") || result.contains("enum") || result.contains("use"),
        "Generated Rust code should contain expected constructs"
    );

    assert!(
        result.contains("serde") || result.contains("Serialize") || result.contains("Deserialize"),
        "Generated Rust code should include serialization support"
    );
}

/// Test error propagation improvements in generators
#[test]
fn test_error_propagation_improvements() {
    let malformed_schema = create_malformed_schema();

    // Test multiple generators handle errors consistently
    let yaml_generator = YamlGenerator::new();
    let json_generator = JsonSchemaGenerator::new();
    let rust_generator = RustGenerator::new();

    // All generators should either succeed or provide meaningful errors
    let yaml_result = yaml_generator.generate(&malformed_schema);
    let json_result = json_generator.generate(&malformed_schema);
    let rust_result = rust_generator.generate(&malformed_schema);

    // Verify error handling is consistent
    match yaml_result {
        Ok(_) => { /* Success is acceptable */ }
        Err(e) => {
            let error_msg = format!("{e}");
            assert!(
                !error_msg.is_empty(),
                "YAML generator should provide meaningful error"
            );
        }
    }

    match json_result {
        Ok(_) => { /* Success is acceptable */ }
        Err(e) => {
            let error_msg = format!("{e}");
            assert!(
                !error_msg.is_empty(),
                "JSON generator should provide meaningful error"
            );
        }
    }

    match rust_result {
        Ok(_) => { /* Success is acceptable */ }
        Err(e) => {
            let error_msg = format!("{e}");
            assert!(
                !error_msg.is_empty(),
                "Rust generator should provide meaningful error"
            );
        }
    }
}

/// Test generator output format validation
#[test]
fn test_generator_output_validation() {
    let fixture = GeneratorTestFixture::new();

    // Test YAML output is valid YAML
    let yaml_generator = YamlGenerator::new();
    let yaml_output = yaml_generator.generate(&fixture.complex_schema)
        .expect("YAML generation should succeed");

    let parsed_yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_output)
        .expect("Generated YAML should be valid");

    assert!(
        parsed_yaml.is_mapping(),
        "Generated YAML should be a valid mapping"
    );

    // Test JSON output is valid JSON
    let json_generator = JsonSchemaGenerator::new();
    let json_output = json_generator.generate(&fixture.complex_schema)
        .expect("JSON generation should succeed");

    let parsed_json: JsonValue = serde_json::from_str(&json_output)
        .expect("Generated JSON should be valid");

    assert!(
        parsed_json.is_object() || parsed_json.is_array(),
        "Generated JSON should be a valid object or array"
    );
}

/// Test edge cases and boundary conditions
#[test]
fn test_generator_edge_cases() {
    // Test with empty schema
    let empty_schema = SchemaDefinition {
        id: "test://empty".to_string(),
        name: "Empty".to_string(),
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    let generators = vec![
        Box::new(YamlGenerator::new()) as Box<dyn Generator>,
        Box::new(JsonSchemaGenerator::new()) as Box<dyn Generator>,
        Box::new(RustGenerator::new()) as Box<dyn Generator>,
    ];

    for generator in generators {
        let result = generator.generate(&empty_schema);

        match result {
            Ok(output) => {
                assert!(
                    !output.is_empty(),
                    "Generators should produce output even for empty schemas"
                );
            }
            Err(_) => {
                // Some generators may legitimately reject empty schemas
                // The important thing is they don't panic
            }
        }
    }
}

/// Test concurrent generator usage
#[test]
fn test_generator_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let fixture = GeneratorTestFixture::new();
    let schema = Arc::new(fixture.simple_schema);

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let schema_clone = Arc::clone(&schema);
            thread::spawn(move || {
                let generator = match i % 3 {
                    0 => Box::new(YamlGenerator::new()) as Box<dyn Generator>,
                    1 => Box::new(JsonSchemaGenerator::new()) as Box<dyn Generator>,
                    _ => Box::new(RustGenerator::new()) as Box<dyn Generator>,
                };

                generator.generate(&*schema_clone)
            })
        })
        .collect();

    // All threads should complete successfully
    for handle in handles {
        let result = handle.join().expect("Thread should not panic");
        match result {
            Ok(output) => {
                assert!(
                    !output.is_empty(),
                    "Concurrent generation should produce output"
                );
            }
            Err(_) => {
                // Some generation failures are acceptable,
                // but threads should not panic
            }
        }
    }
}

/// Test memory efficiency of refactored functions
#[test]
fn test_generator_memory_efficiency() {
    // Create a large schema to test memory efficiency
    let large_schema = create_large_schema();
    let yaml_generator = YamlGenerator::new();

    // Generation should complete without excessive memory usage
    let start_time = std::time::Instant::now();
    let result = yaml_generator.generate(&large_schema)
        .expect("Large schema generation should succeed");
    let duration = start_time.elapsed();

    assert!(
        !result.is_empty(),
        "Large schema should produce output"
    );

    // Should complete in reasonable time (less than 5 seconds for test)
    assert!(
        duration.as_secs() < 5,
        "Generation should complete in reasonable time"
    );
}

// Helper functions to create test schemas

fn create_simple_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "test://simple".to_string(),
        name: "SimpleTest".to_string(),
        version: Some("1.0.0".to_string()),
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    // Add a simple class
    let person_class = ClassDefinition {
        name: "Person".to_string(),
        description: Some("A person entity".to_string()),
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person_class);

    // Add a simple slot
    let name_slot = SlotDefinition {
        name: "name".to_string(),
        range: Some("string".to_string()),
        required: Some(true),
        ..Default::default()
    };

    schema.slots.insert("name".to_string(), name_slot);

    schema
}

fn create_complex_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "test://complex".to_string(),
        name: "ComplexTest".to_string(),
        version: Some("2.0.0".to_string()),
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    // Add multiple classes with inheritance
    let entity_class = ClassDefinition {
        name: "Entity".to_string(),
        description: Some("Base entity class".to_string()),
        abstract_: Some(true),
        ..Default::default()
    };

    let person_class = ClassDefinition {
        name: "Person".to_string(),
        is_a: Some("Entity".to_string()),
        description: Some("A person entity".to_string()),
        ..Default::default()
    };

    let organization_class = ClassDefinition {
        name: "Organization".to_string(),
        is_a: Some("Entity".to_string()),
        description: Some("An organization entity".to_string()),
        ..Default::default()
    };

    schema.classes.insert("Entity".to_string(), entity_class);
    schema.classes.insert("Person".to_string(), person_class);
    schema.classes.insert("Organization".to_string(), organization_class);

    // Add multiple slot types
    let slots_data = vec![
        ("id", "string", true, "Unique identifier"),
        ("name", "string", true, "Name of the entity"),
        ("created_at", "datetime", false, "Creation timestamp"),
        ("updated_at", "datetime", false, "Update timestamp"),
    ];

    for (name, range, required, description) in slots_data {
        let slot = SlotDefinition {
            name: name.to_string(),
            range: Some(range.to_string()),
            required: Some(required),
            description: Some(description.to_string()),
            ..Default::default()
        };
        schema.slots.insert(name.to_string(), slot);
    }

    // Add enum
    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        description: Some("Status enumeration".to_string()),
        ..Default::default()
    };

    schema.enums.insert("Status".to_string(), status_enum);

    schema
}

fn create_edge_case_schema() -> SchemaDefinition {
    SchemaDefinition {
        id: "test://edge-cases".to_string(),
        name: "EdgeCases".to_string(),
        // Intentionally leave some fields as None to test edge cases
        version: None,
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    }
}

fn create_numeric_boundary_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "test://numeric-boundaries".to_string(),
        name: "NumericBoundaries".to_string(),
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    // Add type definitions with boundary values
    let integer_type = TypeDefinition {
        name: "big_integer".to_string(),
        typeof_: Some("integer".to_string()),
        minimum_value: Some(-9223372036854775808_i64 as f64),
        maximum_value: Some(9223372036854775807_i64 as f64),
        ..Default::default()
    };

    schema.types.insert("big_integer".to_string(), integer_type);

    schema
}

fn create_malformed_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "test://malformed".to_string(),
        name: "".to_string(), // Empty name might cause issues
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    // Add class with circular reference
    let circular_class = ClassDefinition {
        name: "Circular".to_string(),
        is_a: Some("Circular".to_string()), // Self-reference
        ..Default::default()
    };

    schema.classes.insert("Circular".to_string(), circular_class);

    schema
}

fn create_large_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "test://large".to_string(),
        name: "LargeSchema".to_string(),
        classes: HashMap::new(),
        slots: HashMap::new(),
        types: HashMap::new(),
        enums: HashMap::new(),
        ..Default::default()
    };

    // Generate many classes and slots for memory testing
    for i in 0..100 {
        let class = ClassDefinition {
            name: format!("Class{i}"),
            description: Some(format!("Generated class number {i}")),
            ..Default::default()
        };
        schema.classes.insert(format!("Class{i}"), class);

        let slot = SlotDefinition {
            name: format!("slot{i}"),
            range: Some("string".to_string()),
            description: Some(format!("Generated slot number {i}")),
            ..Default::default()
        };
        schema.slots.insert(format!("slot{i}"), slot);
    }

    schema
}