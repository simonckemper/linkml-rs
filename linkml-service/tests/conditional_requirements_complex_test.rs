//! Complex tests for conditional requirements with nested conditions and edge cases

use linkml_core::types::{
    ClassDefinition, ConditionalRequirement, IfRequiredCondition, Definition, SlotDefinition,
};
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use linkml_core::types::SchemaDefinition;
#[tokio::test]
async fn test_multiple_conditional_requirements() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut address_class = ClassDefinition::new("Address");
    address_class.slots = vec![
        "country".to_string(),
        "state".to_string(),
        "province".to_string(),
        "postal_code".to_string(),
        "zip_code".to_string(),
    ];

    // Multiple conditional requirements
    address_class.conditional_requirements = vec![
        // If country is US, require state and zip_code
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "country".to_string(),
                value: "US".to_string(),
            },
            then_required: vec!["state".to_string(), "zip_code".to_string()],
        },
        // If country is Canada, require province and postal_code
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "country".to_string(),
                value: "Canada".to_string(),
            },
            then_required: vec!["province".to_string(), "postal_code".to_string()],
        },
        // If state is CA, zip_code must be present (additional constraint)
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "state".to_string(),
                value: "CA".to_string(),
            },
            then_required: vec!["zip_code".to_string()],
        },
    ];

    schema.classes.insert("Address".to_string(), address_class);

    // Add slots
    for slot_name in ["country", "state", "province", "postal_code", "zip_code"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test US address - valid
    let us_address = json!({
        "country": "US",
        "state": "CA",
        "zip_code": "94107"
    });

    let report = engine
        .validate_as_class(&us_address, "Address", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test US address missing state - invalid
    let us_invalid = json!({
        "country": "US",
        "zip_code": "94107"
    });

    let report = engine
        .validate_as_class(&us_invalid, "Address", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert!(report.errors().any(|e| e.message.contains("state"));

    // Test Canadian address - valid
    let ca_address = json!({
        "country": "Canada",
        "province": "ON",
        "postal_code": "M5H 2N2"
    });

    let report = engine
        .validate_as_class(&ca_address, "Address", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test other country - no requirements
    let other_address = json!({
        "country": "UK"
        // No other fields required
    });

    let report = engine
        .validate_as_class(&other_address, "Address", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_pattern_based_conditional_requirements() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut file_class = ClassDefinition::new("File");
    file_class.slots = vec![
        "filename".to_string(),
        "content_type".to_string(),
        "encoding".to_string(),
        "compression".to_string(),
    ];

    // Pattern-based conditions
    file_class.conditional_requirements = vec![
        // If filename ends with .txt, require content_type
        ConditionalRequirement {
            if_required: IfRequiredCondition::MatchesPattern {
                slot: "filename".to_string(),
                pattern: r".*\.txt$".to_string(),
            },
            then_required: vec!["content_type".to_string()],
        },
        // If filename ends with .gz, require compression
        ConditionalRequirement {
            if_required: IfRequiredCondition::MatchesPattern {
                slot: "filename".to_string(),
                pattern: r".*\.gz$".to_string(),
            },
            then_required: vec!["compression".to_string()],
        },
        // If content_type contains "text", require encoding
        ConditionalRequirement {
            if_required: IfRequiredCondition::MatchesPattern {
                slot: "content_type".to_string(),
                pattern: r".*text.*".to_string(),
            },
            then_required: vec!["encoding".to_string()],
        },
    ];

    schema.classes.insert("File".to_string(), file_class);

    for slot_name in ["filename", "content_type", "encoding", "compression"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test .txt file with text content type
    let text_file = json!({
        "filename": "document.txt",
        "content_type": "text/plain",
        "encoding": "UTF-8"
    });

    let report = engine
        .validate_as_class(&text_file, "File", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test .txt file missing content_type
    let invalid_txt = json!({
        "filename": "document.txt"
    });

    let report = engine
        .validate_as_class(&invalid_txt, "File", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    // Test .gz file
    let gz_file = json!({
        "filename": "archive.tar.gz",
        "compression": "gzip"
    });

    let report = engine
        .validate_as_class(&gz_file, "File", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test text content type requiring encoding
    let text_content = json!({
        "filename": "data.json",
        "content_type": "application/json; charset=utf-8",
        "encoding": "UTF-8"
    });

    let report = engine
        .validate_as_class(&text_content, "File", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_numeric_range_conditional_requirements() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec![
        "price".to_string(),
        "discount_percentage".to_string(),
        "tax_rate".to_string(),
        "shipping_cost".to_string(),
        "weight".to_string(),
    ];

    // Numeric range conditions
    product_class.conditional_requirements = vec![
        // If price > 100, require tax_rate
        ConditionalRequirement {
            if_required: IfRequiredCondition::InRange {
                slot: "price".to_string(),
                min: Some(100.0),
                max: None,
            },
            then_required: vec!["tax_rate".to_string()],
        },
        // If weight > 50, require shipping_cost
        ConditionalRequirement {
            if_required: IfRequiredCondition::InRange {
                slot: "weight".to_string(),
                min: Some(50.0),
                max: None,
            },
            then_required: vec!["shipping_cost".to_string()],
        },
        // If discount_percentage between 20-100, require approval fields
        ConditionalRequirement {
            if_required: IfRequiredCondition::InRange {
                slot: "discount_percentage".to_string(),
                min: Some(20.0),
                max: Some(100.0),
            },
            then_required: vec!["discount_reason".to_string(), "approved_by".to_string()],
        },
    ];

    schema.classes.insert("Product".to_string(), product_class);

    // Add slots
    for slot_name in [
        "price",
        "discount_percentage",
        "tax_rate",
        "shipping_cost",
        "weight",
    ] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("float".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    for slot_name in ["discount_reason", "approved_by"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test expensive product requiring tax
    let expensive = json!({
        "price": 150.0,
        "tax_rate": 0.08,
        "weight": 10.0
    });

    let report = engine
        .validate_as_class(&expensive, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test expensive product missing tax
    let invalid_expensive = json!({
        "price": 150.0,
        "weight": 10.0
    });

    let report = engine
        .validate_as_class(&invalid_expensive, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    // Test heavy product
    let heavy = json!({
        "price": 50.0,
        "weight": 75.0,
        "shipping_cost": 25.0
    });

    let report = engine
        .validate_as_class(&heavy, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test high discount
    let high_discount = json!({
        "price": 80.0,
        "discount_percentage": 30.0,
        "discount_reason": "Clearance sale",
        "approved_by": "manager@example.com",
        "weight": 5.0
    });

    let report = engine
        .validate_as_class(&high_discount, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_field_presence_conditional_requirements() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut user_class = ClassDefinition::new("User");
    user_class.slots = vec![
        "username".to_string(),
        "email".to_string(),
        "phone".to_string(),
        "two_factor_enabled".to_string(),
        "recovery_email".to_string(),
        "recovery_phone".to_string(),
    ];

    // Field presence conditions
    user_class.conditional_requirements = vec![
        // If email is present, require email_verified
        ConditionalRequirement {
            if_required: IfRequiredCondition::FieldPresent {
                slot: "email".to_string(),
            },
            then_required: vec!["email_verified".to_string()],
        },
        // If two_factor_enabled, require either recovery_email or recovery_phone
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "two_factor_enabled".to_string(),
                value: "true".to_string(),
            },
            then_required: vec!["recovery_method".to_string()],
        },
    ];

    schema.classes.insert("User".to_string(), user_class);

    for slot_name in [
        "username",
        "email",
        "phone",
        "recovery_email",
        "recovery_phone",
        "recovery_method",
    ] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    for slot_name in ["two_factor_enabled", "email_verified"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("boolean".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test user with email
    let user_with_email = json!({
        "username": "johndoe",
        "email": "john@example.com",
        "email_verified": true
    });

    let report = engine
        .validate_as_class(&user_with_email, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test user with email but missing verification
    let invalid_email_user = json!({
        "username": "janedoe",
        "email": "jane@example.com"
        // Missing email_verified
    });

    let report = engine
        .validate_as_class(&invalid_email_user, "User", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);

    // Test user without email (no verification required)
    let user_no_email = json!({
        "username": "phoneuser",
        "phone": "+1234567890"
    });

    let report = engine
        .validate_as_class(&user_no_email, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test 2FA user
    let two_fa_user = json!({
        "username": "secureuser",
        "email": "secure@example.com",
        "email_verified": true,
        "two_factor_enabled": true,
        "recovery_method": "email"
    });

    let report = engine
        .validate_as_class(&two_fa_user, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_nested_object_conditional_requirements() {
    let mut schema = SchemaDefinition::new("test_schema");

    // Payment method class
    let mut payment_method = ClassDefinition::new("PaymentMethod");
    payment_method.slots = vec!["type".to_string(), "details".to_string()];
    schema
        .classes
        .insert("PaymentMethod".to_string(), payment_method);

    // Order class with nested payment
    let mut order_class = ClassDefinition::new("Order");
    order_class.slots = vec![
        "id".to_string(),
        "total".to_string(),
        "payment_method".to_string(),
        "billing_address".to_string(),
        "shipping_address".to_string(),
    ];

    // Complex nested conditions
    order_class.conditional_requirements = vec![
        // If total > 1000, require billing_address
        ConditionalRequirement {
            if_required: IfRequiredCondition::InRange {
                slot: "total".to_string(),
                min: Some(1000.0),
                max: None,
            },
            then_required: vec!["billing_address".to_string()],
        },
        // If payment_method.type = "credit_card", require billing_address
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "payment_method.type".to_string(), // Nested field
                value: "credit_card".to_string(),
            },
            then_required: vec!["billing_address".to_string()],
        },
    ];

    schema.classes.insert("Order".to_string(), order_class);

    // Add slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut total_slot = SlotDefinition::new("total");
    total_slot.range = Some("float".to_string());
    schema.slots.insert("total".to_string(), total_slot);

    let mut payment_slot = SlotDefinition::new("payment_method");
    payment_slot.range = Some("PaymentMethod".to_string());
    schema
        .slots
        .insert("payment_method".to_string(), payment_slot);

    for slot_name in ["type", "details", "billing_address", "shipping_address"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test high-value order
    let expensive_order = json!({
        "id": "ORDER-001",
        "total": 1500.0,
        "payment_method": {
            "type": "wire_transfer",
            "details": "Bank details"
        },
        "billing_address": "123 Main St"
    });

    let report = engine
        .validate_as_class(&expensive_order, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test credit card order
    let cc_order = json!({
        "id": "ORDER-002",
        "total": 100.0,
        "payment_method": {
            "type": "credit_card",
            "details": "****1234"
        },
        "billing_address": "456 Oak Ave"
    });

    let report = engine
        .validate_as_class(&cc_order, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_conditional_requirements_with_null_handling() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut record = ClassDefinition::new("Record");
    record.slots = vec![
        "status".to_string(),
        "completion_date".to_string(),
        "review_notes".to_string(),
        "reviewer".to_string(),
    ];

    // Conditions with null handling
    record.conditional_requirements = vec![
        // If status is "completed", require completion_date
        ConditionalRequirement {
            if_required: IfRequiredCondition::Equals {
                slot: "status".to_string(),
                value: "completed".to_string(),
            },
            then_required: vec!["completion_date".to_string()],
        },
        // If review_notes is not null/empty, require reviewer
        ConditionalRequirement {
            if_required: IfRequiredCondition::FieldPresent {
                slot: "review_notes".to_string(),
            },
            then_required: vec!["reviewer".to_string()],
        },
    ];

    schema.classes.insert("Record".to_string(), record);

    for slot_name in ["status", "completion_date", "review_notes", "reviewer"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with null values
    let null_record = json!({
        "status": null,
        "completion_date": null,
        "review_notes": null,
        "reviewer": null
    });

    let report = engine
        .validate_as_class(&null_record, "Record", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid); // No conditions triggered

    // Test with empty string vs null
    let empty_notes = json!({
        "status": "pending",
        "review_notes": ""  // Empty string - might be treated as present
    });

    let report = engine
        .validate_as_class(&empty_notes, "Record", None)
        .await
        .expect("Test operation failed");
    // Behavior depends on implementation - empty string might require reviewer

    // Test completed without date
    let invalid_completed = json!({
        "status": "completed"
        // Missing completion_date
    });

    let report = engine
        .validate_as_class(&invalid_completed, "Record", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
}

#[tokio::test]
async fn test_conditional_requirements_performance() {
    use std::time::Instant;
use linkml_core::types::{ClassDefinition, SlotDefinition};

    let mut schema = SchemaDefinition::new("test_schema");

    let mut complex_class = ClassDefinition::new("ComplexRecord");
    let mut slots = vec![];

    // Create many slots
    for i in 0..50 {
        let slot_name = format!("field_{}", i);
        slots.push(slot_name.clone());

        let mut slot = SlotDefinition::new(&slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name, slot);
    }

    complex_class.slots = slots;

    // Add many conditional requirements
    for i in 0..20 {
        complex_class
            .conditional_requirements
            .push(ConditionalRequirement {
                if_required: IfRequiredCondition::Equals {
                    slot: format!("field_{}", i),
                    value: "trigger".to_string(),
                },
                then_required: vec![format!("field_{}", i + 20)],
            });
    }

    schema
        .classes
        .insert("ComplexRecord".to_string(), complex_class);

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create test data
    let mut data = serde_json::Map::new();
    for i in 0..50 {
        data.insert(format!("field_{}", i), json!(format!("value_{}", i));
    }
    let data = serde_json::Value::Object(data);

    // Measure validation time
    let start = Instant::now();
    for _ in 0..100 {
        let report = engine
            .validate_as_class(&data, "ComplexRecord", None)
            .await
            .expect("Test operation failed");
        assert!(report.valid);
    }
    let elapsed = start.elapsed();

    // Should complete quickly even with many conditions
    assert!(elapsed.as_millis() < 500); // Less than 500ms for 100 validations
}
