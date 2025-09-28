//! Integration tests for unique key validation

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, SlotDefinition, UniqueKeyDefinition};
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;
/// Create a test schema with unique key constraints
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("test_unique_keys");

    // Create slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    id_slot.required = Some(true);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    email_slot.required = Some(true);

    let mut username_slot = SlotDefinition::new("username");
    username_slot.range = Some("string".to_string());
    username_slot.required = Some(true);

    let mut department_slot = SlotDefinition::new("department");
    department_slot.range = Some("string".to_string());
    department_slot.required = Some(false);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);

    schema.slots.insert("id".to_string(), id_slot);
    schema.slots.insert("email".to_string(), email_slot);
    schema.slots.insert("username".to_string(), username_slot);
    schema
        .slots
        .insert("department".to_string(), department_slot);
    schema.slots.insert("name".to_string(), name_slot);

    // Create User class with unique keys
    let mut user_class = ClassDefinition::new("User");
    user_class.slots = vec![
        "id".to_string(),
        "email".to_string(),
        "username".to_string(),
        "name".to_string(),
    ];

    // Email should be unique
    let mut email_unique = UniqueKeyDefinition::default();
    email_unique.unique_key_slots = vec!["email".to_string()];
    email_unique.consider_nulls_inequal = Some(true);
    user_class
        .unique_keys
        .insert("unique_email".to_string(), email_unique);

    // Username should be unique
    let mut username_unique = UniqueKeyDefinition::default();
    username_unique.unique_key_slots = vec!["username".to_string()];
    username_unique.consider_nulls_inequal = Some(true);
    user_class
        .unique_keys
        .insert("unique_username".to_string(), username_unique);

    schema.classes.insert("User".to_string(), user_class);

    // Create Employee class with composite unique key
    let mut employee_class = ClassDefinition::new("Employee");
    employee_class.slots = vec![
        "id".to_string(),
        "email".to_string(),
        "department".to_string(),
        "name".to_string(),
    ];

    // Composite key: email + department must be unique
    let mut composite_unique = UniqueKeyDefinition::default();
    composite_unique.unique_key_slots = vec!["email".to_string(), "department".to_string()];
    composite_unique.consider_nulls_inequal = Some(false); // Nulls are considered equal
    employee_class
        .unique_keys
        .insert("unique_email_dept".to_string(), composite_unique);

    schema
        .classes
        .insert("Employee".to_string(), employee_class);

    schema
}

#[tokio::test]
async fn test_identifier_slot_uniqueness() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instances = vec![
        json!({
            "id": "user1",
            "email": "alice@example.com",
            "username": "alice",
            "name": "Alice Smith"
        }),
        json!({
            "id": "user1", // Duplicate ID
            "email": "bob@example.com",
            "username": "bob",
            "name": "Bob Jones"
        }),
    ];

    let report = engine
        .validate_collection(&instances, "User", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1);

    let error = &errors[0];
    assert!(error.message.contains("Duplicate identifier"));
    assert!(error.message.contains("user1"));
}

#[tokio::test]
async fn test_single_field_uniqueness() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instances = vec![
        json!({
            "id": "user1",
            "email": "alice@example.com",
            "username": "alice",
            "name": "Alice Smith"
        }),
        json!({
            "id": "user2",
            "email": "alice@example.com", // Duplicate email
            "username": "alice2",
            "name": "Alice Jones"
        }),
        json!({
            "id": "user3",
            "email": "bob@example.com",
            "username": "alice", // Duplicate username
            "name": "Bob Smith"
        }),
    ];

    let report = engine
        .validate_collection(&instances, "User", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();

    assert_eq!(errors.len(), 2);

    // Check for email duplicate error
    let email_error = errors
        .iter()
        .find(|e| e.message.contains("unique_email"))
        .expect("Should have email uniqueness error");
    assert!(
        email_error.message.contains("email"),
        "Email error should mention the email slot"
    );
    // Check context contains the duplicate value
    if let Some(values) = email_error.context.get("duplicate_values") {
        assert!(values.to_string().contains("alice@example.com"));
    }

    // Check for username duplicate error
    let username_error = errors
        .iter()
        .find(|e| e.message.contains("unique_username"))
        .expect("Should have username uniqueness error");
    assert!(
        username_error.message.contains("username"),
        "Username error should mention the username slot"
    );
    // Check context contains the duplicate value
    if let Some(values) = username_error.context.get("duplicate_values") {
        assert!(values.to_string().contains("alice"));
    }
}

#[tokio::test]
async fn test_composite_key_uniqueness() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instances = vec![
        json!({
            "id": "emp1",
            "email": "alice@example.com",
            "department": "Engineering",
            "name": "Alice Smith"
        }),
        json!({
            "id": "emp2",
            "email": "alice@example.com",
            "department": "Sales", // Different department, same email is OK
            "name": "Alice Jones"
        }),
        json!({
            "id": "emp3",
            "email": "alice@example.com",
            "department": "Engineering", // Duplicate email+department combination
            "name": "Alice Brown"
        }),
    ];

    let report = engine
        .validate_collection(&instances, "Employee", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1);

    let error = &errors[0];
    assert!(error.message.contains("unique_email_dept"));
    assert!(error.message.contains("email, department"));
}

#[tokio::test]
async fn test_null_handling_in_unique_keys() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with consider_nulls_inequal = false (nulls are not checked for uniqueness)
    // So two records with same email but null department should NOT trigger unique key error
    let instances = vec![
        json!({
            "id": "emp1",
            "email": "alice@example.com",
            "department": "Engineering",
            "name": "Alice Smith"
        }),
        json!({
            "id": "emp2",
            "email": "alice@example.com",
            "department": "Engineering", // Same email AND department - should be duplicate
            "name": "Alice Jones"
        }),
    ];

    let report = engine
        .validate_collection(&instances, "Employee", None)
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();

    // Should have one error for the composite key (email + department)
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("unique_email_dept"));
}

#[tokio::test]
async fn test_no_duplicates_passes_validation() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instances = vec![
        json!({
            "id": "user1",
            "email": "alice@example.com",
            "username": "alice",
            "name": "Alice Smith"
        }),
        json!({
            "id": "user2",
            "email": "bob@example.com",
            "username": "bob",
            "name": "Bob Jones"
        }),
        json!({
            "id": "user3",
            "email": "charlie@example.com",
            "username": "charlie",
            "name": "Charlie Brown"
        }),
    ];

    let report = engine
        .validate_collection(&instances, "User", None)
        .await
        .expect("Test operation failed");

    assert!(report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 0);
}

#[tokio::test]
async fn test_fail_fast_option() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    let instances = vec![
        json!({
            "id": "user1",
            "email": "alice@example.com",
            "username": "alice",
            "name": "Alice Smith"
        }),
        json!({
            "id": "user1", // First duplicate
            "email": "bob@example.com",
            "username": "bob",
            "name": "Bob Jones"
        }),
        json!({
            "id": "user3",
            "email": "alice@example.com", // Second duplicate (should not be reached with fail_fast)
            "username": "charlie",
            "name": "Charlie Brown"
        }),
    ];

    let mut options = ValidationOptions::default();
    options.fail_fast = Some(true);

    let report = engine
        .validate_collection(&instances, "User", Some(options))
        .await
        .expect("Test operation failed");

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1); // Only first error due to fail_fast
}

#[tokio::test]
async fn test_reset_between_validations() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // First validation
    let instances1 = vec![json!({
        "id": "user1",
        "email": "alice@example.com",
        "username": "alice",
        "name": "Alice Smith"
    })];

    let report1 = engine
        .validate_collection(&instances1, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report1.valid);

    // Second validation with same ID should pass (tracker was reset)
    let instances2 = vec![json!({
        "id": "user1",
        "email": "bob@example.com",
        "username": "bob",
        "name": "Bob Jones"
    })];

    let report2 = engine
        .validate_collection(&instances2, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report2.valid);
}

#[tokio::test]
async fn test_large_collection_performance() {
    let schema = create_test_schema();
    let mut engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create 1000 unique instances
    let mut instances = Vec::new();
    for i in 0..1000 {
        instances.push(json!({
            "id": format!("user{}", i),
            "email": format!("user{}@example.com", i),
            "username": format!("user{}", i),
            "name": format!("User {}", i)
        }));
    }

    // Add one duplicate at the end
    instances.push(json!({
        "id": "user999", // Duplicate ID
        "email": "duplicate@example.com",
        "username": "duplicate",
        "name": "Duplicate User"
    }));

    let start = std::time::Instant::now();
    let report = engine
        .validate_collection(&instances, "User", None)
        .await
        .expect("Test operation failed");
    let duration = start.elapsed();

    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1);

    // Performance assertion: should complete in reasonable time
    assert!(
        duration.as_millis() < 1000,
        "Validation took too long: {:?}",
        duration
    );
}
