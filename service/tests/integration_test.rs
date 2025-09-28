use linkml_service::generator::Generator;
use linkml_service::parser::Parser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::error::Error as StdError;

/// Integration test to ensure core LinkML functionality works after compilation fixes
#[tokio::test]
async fn test_basic_schema_operations() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a simple schema using YAML
    let schema_yaml = r"
id: https://example.org/test
name: TestSchema
description: Test schema for integration testing

classes:
  Person:
    name: Person
    description: A person
    slots:
      - name
      - age

slots:
  name:
    name: name
    range: string
    required: true
  age:
    name: age
    range: integer
";

    // Parse the schema
    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;

    // Test basic schema structure
    assert_eq!(schema.name, "TestSchema");
    assert!(schema.classes.contains_key("Person"));
    assert!(schema.slots.contains_key("name"));
    assert!(schema.slots.contains_key("age"));

    // Test validation engine creation
    let validation_engine = ValidationEngine::new(&schema)?;

    // Test valid instance
    let valid_instance = json!({
        "name": "John Doe",
        "age": 30
    });

    let validation_result = validation_engine
        .validate_as_class(&valid_instance, "Person", None)
        .await?;

    assert!(
        validation_result.valid,
        "Valid instance should pass validation"
    );

    // Test invalid instance (missing required field)
    let invalid_instance = json!({
        "age": 30
        // missing required name field
    });

    let invalid_result = validation_engine
        .validate_as_class(&invalid_instance, "Person", None)
        .await?;

    assert!(
        !invalid_result.valid,
        "Invalid instance should fail validation"
    );
    assert!(
        !invalid_result.issues.is_empty(),
        "Should have validation issues"
    );

    Ok(())
}

/// Test schema loading and parsing functionality
#[tokio::test]
async fn test_schema_loading() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let yaml_schema = r"
id: https://example.org/test
name: TestSchema
description: Test schema for loading

classes:
  Person:
    name: Person
    description: A person
    slots:
      - name
      - email

slots:
  name:
    name: name
    range: string
    required: true
  email:
    name: email
    range: string
    pattern: '^[^@]+@[^@]+\.[^@]+$'
";

    let parser = Parser::new();
    let schema = parser.parse(yaml_schema, "yaml")?;

    assert_eq!(schema.name, "TestSchema");
    assert!(schema.classes.contains_key("Person"));
    assert!(schema.slots.contains_key("name"));
    assert!(schema.slots.contains_key("email"));

    // Test that email slot has pattern
    let email_slot = schema.slots.get("email").expect("test access failed");
    assert!(email_slot.pattern.is_some());

    Ok(())
}

/// Test generator functionality
#[tokio::test]
async fn test_basic_generation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/generator-test
name: GeneratorTest

classes:
  TestClass:
    name: TestClass
    description: A test class
    slots:
      - name

slots:
  name:
    name: name
    range: string
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;

    // Test JSON Schema generation
    use linkml_service::generator::json_schema::JsonSchemaGenerator;

    let generator = JsonSchemaGenerator::new();
    let json_schema_result = generator.generate(&schema);

    assert!(
        json_schema_result.is_ok(),
        "JSON Schema generation should succeed"
    );

    let json_schema = json_schema_result.expect("LinkML operation in test should succeed");
    assert!(
        json_schema.contains("TestClass"),
        "Generated schema should contain TestClass"
    );

    Ok(())
}

/// Test validation with inheritance
#[tokio::test]
async fn test_inheritance_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/inheritance-test
name: InheritanceTest

classes:
  Entity:
    name: Entity
    description: Base entity
    slots:
      - id
  Person:
    name: Person
    description: A person
    is_a: Entity
    slots:
      - name

slots:
  id:
    name: id
    range: string
    required: true
  name:
    name: name
    range: string
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;

    let validation_engine = ValidationEngine::new(&schema)?;

    // Test that Person inherits id slot from Entity
    let instance = json!({
        "id": "person-1",
        "name": "Alice"
    });

    let validation = validation_engine
        .validate_as_class(&instance, "Person", None)
        .await?;

    assert!(
        validation.valid,
        "Person should inherit id slot from Entity"
    );

    Ok(())
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let parser = Parser::new();

    // Test invalid YAML
    let invalid_yaml = "invalid: yaml: content: [unclosed";
    let result = parser.parse(invalid_yaml, "yaml");
    assert!(result.is_err(), "Invalid YAML should return error");

    // Test invalid schema structure (this should parse but might have validation issues)
    let invalid_schema_yaml = r"
name: InvalidSchema
classes:
  TestClass:
    slots:
      - nonexistent_slot  # This slot is not defined
";

    let result = parser.parse(invalid_schema_yaml, "yaml");
    // This might succeed at parsing but would fail at validation
    if let Ok(schema) = result {
        // Try to create validation engine - this might catch the error
        let validation_engine_result = ValidationEngine::new(&schema);
        // Should either fail here or when validating instances
        if let Ok(engine) = validation_engine_result {
            let test_instance = json!({"name": "test"});
            let validation = engine
                .validate_as_class(&test_instance, "TestClass", None)
                .await;
            // Should have issues due to undefined slot
            assert!(
                validation.is_err()
                    || !validation
                        .expect("LinkML operation in test should succeed")
                        .valid
            );
        }
    }

    Ok(())
}

/// Test concurrent operations
#[tokio::test]
async fn test_concurrent_operations() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut handles = vec![];

    // Spawn multiple concurrent parsing tasks
    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let schema_yaml = format!(
                r"
id: https://example.org/concurrent-test-{}
    Ok(())
}
name: ConcurrentTest{}

classes:
  TestClass{}:
    name: TestClass{}
    description: A test class
    slots:
      - name

slots:
  name:
    name: name
    range: string
",
                i, i, i, i
            );

            let parser = Parser::new();
            parser.parse(&schema_yaml, "yaml")
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        let result = handle
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        assert!(result.is_ok(), "Concurrent parsing should succeed");
    }

    Ok(())
}
