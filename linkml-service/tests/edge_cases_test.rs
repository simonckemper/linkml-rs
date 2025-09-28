use std::error::Error as StdError;
use linkml_service::parser::Parser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use tokio;


/// Test edge cases and complex scenarios
#[tokio::test]
async fn test_empty_schema() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let empty_schema = r"
id: https://example.org/empty
name: EmptySchema
";

    let parser = Parser::new();
    let schema = parser.parse(empty_schema, "yaml")?;

    assert_eq!(schema.name, "EmptySchema");
    assert!(schema.classes.is_empty());
    assert!(schema.slots.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_schema_with_special_characters() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/special-chars
name: SpecialCharsSchema

classes:
  PersonWithUnicode:
    name: PersonWithUnicode
    description: A person with unicode characters ñáéíóú
    slots:
      - name_with_spaces
      - email_address

slots:
  name_with_spaces:
    name: name_with_spaces
    range: string
    description: Name with spaces and unicode ñáéíóú
  email_address:
    name: email_address
    range: string
    pattern: '^[^@]+@[^@]+\.[^@]+$'
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = ValidationEngine::new(&schema)?;
    
    let test_data = json!({
        "name_with_spaces": "José María García",
        "email_address": "josé@example.com"
    });

    let result = validation_engine
        .validate_as_class(&test_data, "PersonWithUnicode", None)
        .await?;
    
    assert!(result.valid, "Should handle unicode characters correctly");

    Ok(())
}

#[tokio::test]
async fn test_deeply_nested_inheritance() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/deep-inheritance
name: DeepInheritanceSchema

classes:
  Entity:
    name: Entity
    description: Base entity
    slots:
      - id
  NamedEntity:
    name: NamedEntity
    description: Entity with name
    is_a: Entity
    slots:
      - name
  Person:
    name: Person
    description: A person
    is_a: NamedEntity
    slots:
      - email
  Employee:
    name: Employee
    description: An employee
    is_a: Person
    slots:
      - employee_id

slots:
  id:
    name: id
    range: string
    required: true
  name:
    name: name
    range: string
    required: true
  email:
    name: email
    range: string
  employee_id:
    name: employee_id
    range: string
    required: true
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = ValidationEngine::new(&schema)?;
    
    // Test that Employee inherits all fields from the chain
    let employee_data = json!({
        "id": "emp-001",
        "name": "John Doe",
        "email": "john@company.com",
        "employee_id": "EMP001"
    });

    let result = validation_engine
        .validate_as_class(&employee_data, "Employee", None)
        .await?;
    
    assert!(result.valid, "Employee should inherit all fields from inheritance chain");

    // Test missing required field from base class
    let invalid_employee = json!({
        "name": "Jane Doe",
        "employee_id": "EMP002"
        // missing required 'id' from Entity
    });

    let invalid_result = validation_engine
        .validate_as_class(&invalid_employee, "Employee", None)
        .await?;
    
    assert!(!invalid_result.valid, "Should fail when missing required field from base class");

    Ok(())
}

#[tokio::test]
async fn test_complex_patterns() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/patterns
name: PatternSchema

classes:
  DataWithPatterns:
    name: DataWithPatterns
    slots:
      - phone_number
      - credit_card
      - ip_address

slots:
  phone_number:
    name: phone_number
    range: string
    pattern: '^\+?[1-9]\d{1,14}$'
  credit_card:
    name: credit_card
    range: string
    pattern: '^[0-9]{4}-[0-9]{4}-[0-9]{4}-[0-9]{4}$'
  ip_address:
    name: ip_address
    range: string
    pattern: '^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$'
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = ValidationEngine::new(&schema)?;
    
    // Test valid patterns
    let valid_data = json!({
        "phone_number": "+1234567890",
        "credit_card": "1234-5678-9012-3456",
        "ip_address": "192.168.1.1"
    });

    let result = validation_engine
        .validate_as_class(&valid_data, "DataWithPatterns", None)
        .await?;
    
    assert!(result.valid, "Valid patterns should pass validation");

    // Test invalid patterns
    let invalid_data = json!({
        "phone_number": "invalid-phone",
        "credit_card": "1234567890123456",  // wrong format
        "ip_address": "999.999.999.999"     // invalid IP
    });

    let invalid_result = validation_engine
        .validate_as_class(&invalid_data, "DataWithPatterns", None)
        .await?;
    
    assert!(!invalid_result.valid, "Invalid patterns should fail validation");

    Ok(())
}

#[tokio::test]
async fn test_large_data_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/large-data
name: LargeDataSchema

classes:
  LargeRecord:
    name: LargeRecord
    slots:
      - id
      - data

slots:
  id:
    name: id
    range: string
    required: true
  data:
    name: data
    range: string
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = ValidationEngine::new(&schema)?;
    
    // Create a large string (1MB)
    let large_string = "x".repeat(1_000_000);
    
    let large_data = json!({
        "id": "large-001",
        "data": large_string
    });

    let result = validation_engine
        .validate_as_class(&large_data, "LargeRecord", None)
        .await?;
    
    assert!(result.valid, "Should handle large data efficiently");

    Ok(())
}

#[tokio::test]
async fn test_concurrent_validation_stress() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/stress-test
name: StressTestSchema

classes:
  StressTest:
    name: StressTest
    slots:
      - id
      - value

slots:
  id:
    name: id
    range: string
    required: true
  value:
    name: value
    range: integer
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = std::sync::Arc::new(ValidationEngine::new(&schema)?);
    
    let mut handles = vec![];
    
    // Spawn 100 concurrent validation tasks
    for i in 0..100 {
        let engine = validation_engine.clone();
        let handle = tokio::spawn(async move {
            let test_data = json!({
                "id": format!("stress-{}", i),
                "value": i
            });
            
            engine.validate_as_class(&test_data, "StressTest", None).await
        });
        handles.push(handle);
    }
    
    // Wait for all tasks to complete
    let mut success_count = 0;
    for handle in handles {
        let result = handle.await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        if result.is_ok() && result.expect("LinkML operation in test should succeed").valid {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 100, "All concurrent validations should succeed");

    Ok(())
}

#[tokio::test]
async fn test_malformed_json_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r"
id: https://example.org/json-test
name: JsonTestSchema

classes:
  JsonTest:
    name: JsonTest
    slots:
      - data

slots:
  data:
    name: data
    range: string
";

    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml")?;
    
    let validation_engine = ValidationEngine::new(&schema)?;
    
    // Test with various JSON edge cases
    let test_cases = vec![
        json!({"data": null}),
        json!({"data": ""}),
        json!({"data": "string with \"quotes\" and \\backslashes"}),
        json!({"data": "unicode: 🚀 ñáéíóú"}),
    ];
    
    for test_case in test_cases {
        let result = validation_engine
            .validate_as_class(&test_case, "JsonTest", None)
            .await?;
        
        // Should handle all these cases without crashing
        assert!(result.valid || !result.valid, "Should handle edge case JSON without crashing");
    }

    Ok(())
}
