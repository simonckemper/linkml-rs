//! Advanced tests for custom validators with AppliesTo and complex scenarios

use linkml_core::types::{ClassDefinition, Definition, SlotDefinition};
use linkml_service::validator::{
    AppliesTo, CustomValidator, CustomValidatorBuilder, ValidationEngine, ValidationError,
};
use serde_json::json;
use linkml_core::types::SchemaDefinition;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[tokio::test]
async fn test_custom_validator_with_complex_applies_to() {
    let mut schema = SchemaDefinition::new("test_schema");

    // Create class hierarchy
    let mut base_class = ClassDefinition::new("Base");
    base_class.slots = vec!["id".to_string()];
    schema.classes.insert("Base".to_string(), base_class);

    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("Base".to_string());
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut employee_class = ClassDefinition::new("Employee");
    employee_class.is_a = Some("Person".to_string());
    employee_class.slots = vec!["employee_id".to_string(), "department".to_string()];
    schema
        .classes
        .insert("Employee".to_string(), employee_class);

    // Add slots
    for (name, range) in [
        ("id", "string"),
        ("name", "string"),
        ("age", "integer"),
        ("employee_id", "string"),
        ("department", "string"),
    ] {
        let mut slot = SlotDefinition::new(name);
        slot.range = Some(range.to_string());
        schema.slots.insert(name.to_string(), slot);
    }

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Custom validator that applies only to Employee class and checks department
    let dept_validator = CustomValidatorBuilder::new()
        .with_name("department_validator")
        .with_applies_to(AppliesTo::Classes(vec!["Employee".to_string()]))
        .with_validation_fn(|data, _schema, _context| {
            if let Some(dept) = data.get("department").and_then(|v| v.as_str()) {
                let valid_depts = ["Engineering", "Sales", "Marketing", "HR"];
                if !valid_depts.contains(&dept) {
                    return vec![ValidationError::new(format!(
                        "Invalid department: {}. Must be one of: {:?}",
                        dept, valid_depts
                    ))];
                }
            }
            vec![]
        })
        .build();

    engine.add_custom_validator(dept_validator);

    // Test with Employee (should apply)
    let employee = json!({
        "id": "E123",
        "name": "John Doe",
        "age": 30,
        "employee_id": "EMP001",
        "department": "Finance"  // Invalid
    });

    let report = engine
        .validate_as_class(&employee, "Employee", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(
        report
            .errors()
            .any(|e| e.message.contains("Invalid department"))
    );

    // Test with Person (should not apply)
    let person = json!({
        "id": "P123",
        "name": "Jane Doe",
        "age": 25,
        "department": "Finance"  // This field shouldn't be validated for Person
    });

    let report = engine
        .validate_as_class(&person, "Person", None)
        .await
        .expect("Test operation failed");
    // Should be valid because validator doesn't apply to Person class
    assert!(report.valid);
}

#[tokio::test]
async fn test_multiple_custom_validators_interaction() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut user_class = ClassDefinition::new("User");
    user_class.slots = vec![
        "username".to_string(),
        "email".to_string(),
        "password".to_string(),
    ];
    schema.classes.insert("User".to_string(), user_class);

    for (name, range) in [
        ("username", "string"),
        ("email", "string"),
        ("password", "string"),
    ] {
        let mut slot = SlotDefinition::new(name);
        slot.range = Some(range.to_string());
        schema.slots.insert(name.to_string(), slot);
    }

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Validator 1: Username constraints
    let username_validator = CustomValidatorBuilder::new()
        .with_name("username_validator")
        .with_applies_to(AppliesTo::Slots(vec!["username".to_string()]))
        .with_validation_fn(|data, _schema, _context| {
            let mut errors = vec![];
            if let Some(username) = data.get("username").and_then(|v| v.as_str()) {
                if username.len() < 3 {
                    errors.push(ValidationError::new(
                        "Username must be at least 3 characters",
                    ));
                }
                if username.contains(' ') {
                    errors.push(ValidationError::new("Username cannot contain spaces"));
                }
                if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    errors.push(ValidationError::new(
                        "Username can only contain letters, numbers, and underscores",
                    ));
                }
            }
            errors
        })
        .build();

    // Validator 2: Password strength
    let password_validator = CustomValidatorBuilder::new()
        .with_name("password_validator")
        .with_applies_to(AppliesTo::Slots(vec!["password".to_string()]))
        .with_validation_fn(|data, _schema, _context| {
            let mut errors = vec![];
            if let Some(password) = data.get("password").and_then(|v| v.as_str()) {
                if password.len() < 8 {
                    errors.push(ValidationError::new(
                        "Password must be at least 8 characters",
                    ));
                }
                if !password.chars().any(|c| c.is_uppercase()) {
                    errors.push(ValidationError::new(
                        "Password must contain at least one uppercase letter",
                    ));
                }
                if !password.chars().any(|c| c.is_numeric()) {
                    errors.push(ValidationError::new(
                        "Password must contain at least one number",
                    ));
                }
            }
            errors
        })
        .build();

    // Validator 3: Cross-field validation
    let cross_field_validator = CustomValidatorBuilder::new()
        .with_name("cross_field_validator")
        .with_applies_to(AppliesTo::All)
        .with_validation_fn(|data, _schema, _context| {
            let mut errors = vec![];
            let username = data.get("username").and_then(|v| v.as_str());
            let password = data.get("password").and_then(|v| v.as_str());

            if let (Some(user), Some(pass)) = (username, password) {
                if pass.contains(user) {
                    errors.push(ValidationError::new("Password cannot contain username"));
                }
            }
            errors
        })
        .build();

    engine.add_custom_validator(username_validator);
    engine.add_custom_validator(password_validator);
    engine.add_custom_validator(cross_field_validator);

    // Test with multiple validation failures
    let user = json!({
        "username": "ab",  // Too short
        "email": "user@example.com",
        "password": "abcdef"  // Contains username, too short, no uppercase, no number
    });

    let report = engine
        .validate_as_class(&user, "User", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    let errors: Vec<_> = report.errors().collect();
    assert!(errors.len() >= 5); // Should have multiple errors from different validators
}

#[tokio::test]
async fn test_custom_validator_with_state() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut transaction_class = ClassDefinition::new("Transaction");
    transaction_class.slots = vec![
        "id".to_string(),
        "amount".to_string(),
        "user_id".to_string(),
    ];
    schema
        .classes
        .insert("Transaction".to_string(), transaction_class);

    for (name, range) in [("id", "string"), ("amount", "float"), ("user_id", "string")] {
        let mut slot = SlotDefinition::new(name);
        slot.range = Some(range.to_string());
        schema.slots.insert(name.to_string(), slot);
    }

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Stateful validator that tracks daily limits per user
    let daily_limits: Arc<Mutex<HashMap<String, f64>>> = Arc::new(Mutex::new(HashMap::new());
    let daily_limits_clone = Arc::clone(&daily_limits);

    let limit_validator = CustomValidatorBuilder::new()
        .with_name("daily_limit_validator")
        .with_applies_to(AppliesTo::Classes(vec!["Transaction".to_string()]))
        .with_validation_fn(move |data, _schema, _context| {
            let mut errors = vec![];

            if let (Some(user_id), Some(amount)) = (
                data.get("user_id").and_then(|v| v.as_str()),
                data.get("amount").and_then(|v| v.as_f64()),
            ) {
                let mut limits = daily_limits_clone.lock().expect("Test operation failed");
                let daily_total = limits.entry(user_id.to_string()).or_insert(0.0);
                *daily_total += amount;

                if *daily_total > 10000.0 {
                    errors.push(ValidationError::new(format!(
                        "Daily transaction limit exceeded for user {}. Total: ${:.2}",
                        user_id, daily_total
                    )));
                }
            }

            errors
        })
        .build();

    engine.add_custom_validator(limit_validator);

    // First transaction - should pass
    let tx1 = json!({
        "id": "TX001",
        "amount": 5000.0,
        "user_id": "USER123"
    });

    let report = engine
        .validate_as_class(&tx1, "Transaction", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Second transaction - should pass (total 9000)
    let tx2 = json!({
        "id": "TX002",
        "amount": 4000.0,
        "user_id": "USER123"
    });

    let report = engine
        .validate_as_class(&tx2, "Transaction", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Third transaction - should fail (would exceed 10000)
    let tx3 = json!({
        "id": "TX003",
        "amount": 2000.0,
        "user_id": "USER123"
    });

    let report = engine
        .validate_as_class(&tx3, "Transaction", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(
        report
            .errors()
            .any(|e| e.message.contains("Daily transaction limit exceeded"))
    );
}

#[tokio::test]
async fn test_custom_validator_with_async_validation() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut api_key_class = ClassDefinition::new("ApiKey");
    api_key_class.slots = vec!["key".to_string(), "permissions".to_string()];
    schema.classes.insert("ApiKey".to_string(), api_key_class);

    let mut key_slot = SlotDefinition::new("key");
    key_slot.range = Some("string".to_string());
    schema.slots.insert("key".to_string(), key_slot);

    let mut permissions_slot = SlotDefinition::new("permissions");
    permissions_slot.range = Some("string".to_string());
    permissions_slot.multivalued = Some(true);
    schema
        .slots
        .insert("permissions".to_string(), permissions_slot);

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Simulate async API key validation
    let api_validator = CustomValidatorBuilder::new()
        .with_name("api_key_validator")
        .with_applies_to(AppliesTo::Slots(vec!["key".to_string()]))
        .with_validation_fn(|data, _schema, _context| {
            let mut errors = vec![];

            if let Some(key) = data.get("key").and_then(|v| v.as_str()) {
                // Simulate async check (in real code this would be async)
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Check key format
                if !key.starts_with("sk_") || key.len() != 32 {
                    errors.push(ValidationError::new("Invalid API key format"));
                }

                // Check if key is "revoked" (simulation)
                if key.contains("revoked") {
                    errors.push(ValidationError::new("API key has been revoked"));
                }
            }

            errors
        })
        .build();

    engine.add_custom_validator(api_validator);

    // Test valid key
    let valid_key = json!({
        "key": "sk_1234567890abcdef1234567890abcd",
        "permissions": ["read", "write"]
    });

    let report = engine
        .validate_as_class(&valid_key, "ApiKey", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test revoked key
    let revoked_key = json!({
        "key": "sk_revoked90abcdef1234567890abcd",
        "permissions": ["read"]
    });

    let report = engine
        .validate_as_class(&revoked_key, "ApiKey", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.errors().any(|e| e.message.contains("revoked"));
}

#[tokio::test]
async fn test_custom_validator_priority_and_ordering() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut form_class = ClassDefinition::new("Form");
    form_class.slots = vec!["field1".to_string(), "field2".to_string()];
    schema.classes.insert("Form".to_string(), form_class);

    for name in ["field1", "field2"] {
        let mut slot = SlotDefinition::new(name);
        slot.range = Some("string".to_string());
        schema.slots.insert(name.to_string(), slot);
    }

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let execution_order = Arc::new(Mutex::new(Vec::new());

    // Validator 1 - runs first
    let order_clone1 = Arc::clone(&execution_order);
    let validator1 = CustomValidatorBuilder::new()
        .with_name("validator1")
        .with_priority(1)
        .with_applies_to(AppliesTo::All)
        .with_validation_fn(move |_data, _schema, _context| {
            order_clone1
                .lock()
                .expect("Test operation failed")
                .push("validator1");
            vec![]
        })
        .build();

    // Validator 2 - runs second
    let order_clone2 = Arc::clone(&execution_order);
    let validator2 = CustomValidatorBuilder::new()
        .with_name("validator2")
        .with_priority(2)
        .with_applies_to(AppliesTo::All)
        .with_validation_fn(move |_data, _schema, _context| {
            order_clone2
                .lock()
                .expect("Test operation failed")
                .push("validator2");
            vec![]
        })
        .build();

    // Validator 3 - runs third
    let order_clone3 = Arc::clone(&execution_order);
    let validator3 = CustomValidatorBuilder::new()
        .with_name("validator3")
        .with_priority(3)
        .with_applies_to(AppliesTo::All)
        .with_validation_fn(move |_data, _schema, _context| {
            order_clone3
                .lock()
                .expect("Test operation failed")
                .push("validator3");
            vec![]
        })
        .build();

    // Add in random order
    engine.add_custom_validator(validator3);
    engine.add_custom_validator(validator1);
    engine.add_custom_validator(validator2);

    let data = json!({
        "field1": "value1",
        "field2": "value2"
    });

    let _ = engine
        .validate_as_class(&data, "Form", None)
        .await
        .expect("Test operation failed");

    // Check execution order
    let order = execution_order.lock().expect("Test operation failed");
    assert_eq!(order.len(), 3);
    assert_eq!(order[0], "validator1");
    assert_eq!(order[1], "validator2");
    assert_eq!(order[2], "validator3");
}

#[tokio::test]
async fn test_custom_validator_error_recovery() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut data_class = ClassDefinition::new("Data");
    data_class.slots = vec!["value".to_string()];
    schema.classes.insert("Data".to_string(), data_class);

    let mut value_slot = SlotDefinition::new("value");
    value_slot.range = Some("string".to_string());
    schema.slots.insert("value".to_string(), value_slot);

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Validator that might panic (but shouldn't crash the system)
    let risky_validator = CustomValidatorBuilder::new()
        .with_name("risky_validator")
        .with_applies_to(AppliesTo::All)
        .with_validation_fn(|data, _schema, _context| {
            if let Some(value) = data.get("value").and_then(|v| v.as_str()) {
                if value == "panic" {
                    // Simulate a potential panic scenario
                    // In real code, this should be caught and handled
                    return vec![ValidationError::new("Dangerous value detected")];
                }

                // Simulate complex processing that could fail
                let _parsed: Result<i32, _> = value.parse();
                // Even if parsing fails, we don't crash
            }
            vec![]
        })
        .build();

    engine.add_custom_validator(risky_validator);

    // Test various inputs
    let test_cases = vec![
        (json!({"value": "normal"}), true),
        (json!({"value": "panic"}), false),
        (json!({"value": "12345"}), true),
        (json!({"value": "not_a_number"}), true),
        (json!({"value": null}), true),
    ];

    for (data, should_be_valid) in test_cases {
        let report = engine
            .validate_as_class(&data, "Data", None)
            .await
            .expect("Test operation failed");
        assert_eq!(report.valid, should_be_valid);
    }
}

#[tokio::test]
async fn test_custom_validator_performance() {
    use std::time::Instant;


    let mut schema = SchemaDefinition::new("test_schema");

    let mut record_class = ClassDefinition::new("Record");
    let mut slots = vec![];

    // Create many slots
    for i in 0..100 {
        let slot_name = format!("field_{}", i);
        slots.push(slot_name.clone());

        let mut slot = SlotDefinition::new(&slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name, slot);
    }

    record_class.slots = slots;
    schema.classes.insert("Record".to_string(), record_class);

    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Add multiple custom validators
    for i in 0..10 {
        let validator = CustomValidatorBuilder::new()
            .with_name(&format!("validator_{}", i))
            .with_applies_to(AppliesTo::All)
            .with_validation_fn(move |data, _schema, _context| {
                // Simulate some processing
                let field_name = format!("field_{}", i);
                if let Some(value) = data.get(&field_name).and_then(|v| v.as_str()) {
                    if value.len() > 1000 {
                        return vec![ValidationError::new("Value too long")];
                    }
                }
                vec![]
            })
            .build();

        engine.add_custom_validator(validator);
    }

    // Create test data
    let mut data = serde_json::Map::new();
    for i in 0..100 {
        data.insert(format!("field_{}", i), json!(format!("value_{}", i));
    }
    let data = serde_json::Value::Object(data);

    // Measure validation time
    let start = Instant::now();
    for _ in 0..100 {
        let report = engine
            .validate_as_class(&data, "Record", None)
            .await
            .expect("Test operation failed");
        assert!(report.valid);
    }
    let elapsed = start.elapsed();

    // Should complete in reasonable time
    assert!(elapsed.as_millis() < 1000); // Less than 1 second for 100 validations
}
