//! Integration tests for conditional requirements validation

use linkml_core::types::{
    ClassDefinition, ConditionalRequirement, Definition, SlotCondition, SlotDefinition,
};
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, SlotDefinition};
#[tokio::test]
async fn test_conditional_requirements_basic() {
    // Create schema with conditional requirements
    let mut schema = SchemaDefinition {
        id: "test-schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add slots
    schema.slots.insert(
        "status".to_string(),
        SlotDefinition {
            name: "status".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "phone".to_string(),
        SlotDefinition {
            name: "phone".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    // Create class with conditional requirement: if status="active", then email and phone are required
    let mut if_required = indexmap::IndexMap::new();
    if_required.insert(
        "status".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                equals_string: Some("active".to_string()),
                ..Default::default()
            }),
            then_required: Some(vec!["email".to_string(), "phone".to_string()]),
        },
    );

    let person_class = ClassDefinition {
        name: "Person".to_string(),
        slots: vec![
            "status".to_string(),
            "email".to_string(),
            "phone".to_string(),
        ],
        if_required: Some(if_required),
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person_class);

    // Create validation engine
    let engine = ValidationEngine::new(&schema).expect("Failed to create engine");
    let options = ValidationOptions::default();

    // Test case 1: Active status without required fields
    let data1 = json!({
        "status": "active"
    });

    let report1 = engine
        .validate_as_class(&data1, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(
        !report1.valid,
        "Should be invalid when conditional requirements not met"
    );
    let errors1: Vec<_> = report1.errors().collect();
    assert_eq!(
        errors1.len(),
        2,
        "Should have 2 errors for missing email and phone"
    );
    assert!(errors1.iter().any(|e| e.message.contains("email")));
    assert!(errors1.iter().any(|e| e.message.contains("phone")));

    // Test case 2: Active status with required fields
    let data2 = json!({
        "status": "active",
        "email": "test@example.com",
        "phone": "555-1234"
    });

    let report2 = engine
        .validate_as_class(&data2, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(
        report2.valid,
        "Should be valid when conditional requirements are met"
    );
    assert_eq!(report2.errors().count(), 0);

    // Test case 3: Inactive status without conditional fields (should be valid)
    let data3 = json!({
        "status": "inactive"
    });

    let report3 = engine
        .validate_as_class(&data3, "Person", Some(options))
        .await
        .expect("Validation failed");

    assert!(report3.valid, "Should be valid when condition not met");
    assert_eq!(report3.errors().count(), 0);
}

#[tokio::test]
async fn test_conditional_requirements_numeric_range() {
    let mut schema = SchemaDefinition {
        id: "test-schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add slots
    schema.slots.insert(
        "age".to_string(),
        SlotDefinition {
            name: "age".to_string(),
            range: Some("integer".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "guardian_name".to_string(),
        SlotDefinition {
            name: "guardian_name".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "driver_license".to_string(),
        SlotDefinition {
            name: "driver_license".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    // Create class with two conditional requirements:
    // 1. If age < 18, then guardian_name is required
    // 2. If age >= 16, then driver_license is required
    let mut if_required = indexmap::IndexMap::new();

    if_required.insert(
        "age".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                maximum_value: Some(json!(17)), // < 18
                ..Default::default()
            }),
            then_required: Some(vec!["guardian_name".to_string()]),
        },
    );

    let person_class = ClassDefinition {
        name: "Person".to_string(),
        slots: vec![
            "age".to_string(),
            "guardian_name".to_string(),
            "driver_license".to_string(),
        ],
        if_required: Some(if_required),
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person_class);

    let engine = ValidationEngine::new(&schema).expect("Failed to create engine");
    let options = ValidationOptions::default();

    // Test case 1: Minor without guardian
    let data1 = json!({
        "age": 15
    });

    let report1 = engine
        .validate_as_class(&data1, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(!report1.valid, "Minor should require guardian");
    assert!(
        report1
            .errors()
            .any(|e| e.message.contains("guardian_name"))
    );

    // Test case 2: Minor with guardian
    let data2 = json!({
        "age": 15,
        "guardian_name": "Parent Name"
    });

    let report2 = engine
        .validate_as_class(&data2, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(report2.valid, "Minor with guardian should be valid");

    // Test case 3: Adult without guardian (should be valid)
    let data3 = json!({
        "age": 25
    });

    let report3 = engine
        .validate_as_class(&data3, "Person", Some(options))
        .await
        .expect("Validation failed");

    assert!(report3.valid, "Adult without guardian should be valid");
}

#[tokio::test]
async fn test_conditional_requirements_pattern() {
    let mut schema = SchemaDefinition {
        id: "test-schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add slots
    schema.slots.insert(
        "email".to_string(),
        SlotDefinition {
            name: "email".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "employee_id".to_string(),
        SlotDefinition {
            name: "employee_id".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "department".to_string(),
        SlotDefinition {
            name: "department".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    // If email matches company domain, then employee_id and department are required
    let mut if_required = indexmap::IndexMap::new();
    if_required.insert(
        "email".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                pattern: Some(r".*@company\.com$".to_string()),
                ..Default::default()
            }),
            then_required: Some(vec!["employee_id".to_string(), "department".to_string()]),
        },
    );

    let person_class = ClassDefinition {
        name: "Person".to_string(),
        slots: vec![
            "email".to_string(),
            "employee_id".to_string(),
            "department".to_string(),
        ],
        if_required: Some(if_required),
        ..Default::default()
    };

    schema.classes.insert("Person".to_string(), person_class);

    let engine = ValidationEngine::new(&schema).expect("Failed to create engine");
    let options = ValidationOptions::default();

    // Test case 1: Company email without employee info
    let data1 = json!({
        "email": "john@company.com"
    });

    let report1 = engine
        .validate_as_class(&data1, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(!report1.valid, "Company email should require employee info");
    assert_eq!(report1.errors().count(), 2);

    // Test case 2: External email without employee info (should be valid)
    let data2 = json!({
        "email": "john@gmail.com"
    });

    let report2 = engine
        .validate_as_class(&data2, "Person", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(
        report2.valid,
        "External email should not require employee info"
    );
}

#[tokio::test]
async fn test_conditional_requirements_field_presence() {
    let mut schema = SchemaDefinition {
        id: "test-schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add slots
    schema.slots.insert(
        "shipping_address".to_string(),
        SlotDefinition {
            name: "shipping_address".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "shipping_method".to_string(),
        SlotDefinition {
            name: "shipping_method".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    schema.slots.insert(
        "tracking_email".to_string(),
        SlotDefinition {
            name: "tracking_email".to_string(),
            range: Some("string".to_string()),
            ..Default::default()
        },
    );

    // If shipping_address is provided (present), then shipping_method and tracking_email are required
    let mut if_required = indexmap::IndexMap::new();
    if_required.insert(
        "shipping_address".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                required: Some(true), // Field is present/provided
                ..Default::default()
            }),
            then_required: Some(vec![
                "shipping_method".to_string(),
                "tracking_email".to_string(),
            ]),
        },
    );

    let order_class = ClassDefinition {
        name: "Order".to_string(),
        slots: vec![
            "shipping_address".to_string(),
            "shipping_method".to_string(),
            "tracking_email".to_string(),
        ],
        if_required: Some(if_required),
        ..Default::default()
    };

    schema.classes.insert("Order".to_string(), order_class);

    let engine = ValidationEngine::new(&schema).expect("Failed to create engine");
    let options = ValidationOptions::default();

    // Test case 1: Shipping address provided without method/email
    let data1 = json!({
        "shipping_address": "123 Main St"
    });

    let report1 = engine
        .validate_as_class(&data1, "Order", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(!report1.valid, "Shipping address requires method and email");
    assert_eq!(report1.errors().count(), 2);

    // Test case 2: No shipping address (digital order)
    let data2 = json!({});

    let report2 = engine
        .validate_as_class(&data2, "Order", Some(options.clone()))
        .await
        .expect("Validation failed");

    assert!(
        report2.valid,
        "Digital order without shipping should be valid"
    );

    // Test case 3: Complete shipping info
    let data3 = json!({
        "shipping_address": "123 Main St",
        "shipping_method": "express",
        "tracking_email": "customer@example.com"
    });

    let report3 = engine
        .validate_as_class(&data3, "Order", Some(options))
        .await
        .expect("Validation failed");

    assert!(report3.valid, "Complete shipping info should be valid");
}

#[tokio::test]
async fn test_multiple_conditional_requirements() {
    let mut schema = SchemaDefinition {
        id: "test-schema".to_string(),
        name: "test_schema".to_string(),
        ..Default::default()
    };

    // Add all necessary slots
    for slot_name in [
        "country",
        "state",
        "zip_code",
        "is_business",
        "tax_id",
        "business_name",
    ] {
        schema.slots.insert(
            slot_name.to_string(),
            SlotDefinition {
                name: slot_name.to_string(),
                range: Some("string".to_string()),
                ..Default::default()
            },
        );
    }

    // Multiple conditional requirements:
    // 1. If country="US", then state and zip_code are required
    // 2. If is_business="true", then tax_id and business_name are required
    let mut if_required = indexmap::IndexMap::new();

    if_required.insert(
        "country".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                equals_string: Some("US".to_string()),
                ..Default::default()
            }),
            then_required: Some(vec!["state".to_string(), "zip_code".to_string()]),
        },
    );

    if_required.insert(
        "is_business".to_string(),
        ConditionalRequirement {
            condition: Some(SlotCondition {
                equals_string: Some("true".to_string()),
                ..Default::default()
            }),
            then_required: Some(vec!["tax_id".to_string(), "business_name".to_string()]),
        },
    );

    let address_class = ClassDefinition {
        name: "Address".to_string(),
        slots: vec![
            "country".to_string(),
            "state".to_string(),
            "zip_code".to_string(),
            "is_business".to_string(),
            "tax_id".to_string(),
            "business_name".to_string(),
        ],
        if_required: Some(if_required),
        ..Default::default()
    };

    schema.classes.insert("Address".to_string(), address_class);

    let engine = ValidationEngine::new(&schema).expect("Failed to create engine");
    let options = ValidationOptions::default();

    // Test case: US business address missing all conditional fields
    let data = json!({
        "country": "US",
        "is_business": "true"
    });

    let report = engine
        .validate_as_class(&data, "Address", Some(options))
        .await
        .expect("Validation failed");

    assert!(
        !report.valid,
        "Should have multiple conditional requirement violations"
    );
    assert_eq!(report.errors().count(), 4, "Should have 4 errors total");

    // Verify all expected fields are reported
    let error_messages: Vec<String> = report.errors().map(|e| e.message.clone()).collect();

    assert!(error_messages.iter().any(|m| m.contains("state"));
    assert!(error_messages.iter().any(|m| m.contains("zip_code"));
    assert!(error_messages.iter().any(|m| m.contains("tax_id"));
    assert!(error_messages.iter().any(|m| m.contains("business_name")));
}
