//! Tests for unique key validation with concurrent access and edge cases

use indexmap::IndexMap;
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, SlotDefinition, UniqueKeyDefinition};
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::sync::Arc;
use tokio::task;
#[tokio::test]
async fn test_concurrent_unique_validation() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut user_class = ClassDefinition::new("User");
    user_class.slots = vec!["id".to_string(), "email".to_string()];

    // Add unique key for email
    let mut unique_keys = IndexMap::new();
    unique_keys.insert(
        "email_key".to_string(),
        UniqueKeyDefinition {
            description: Some("Email uniqueness".to_string()),
            unique_key_slots: vec!["email".to_string()],
            consider_nulls_inequal: Some(false),
        },
    );
    user_class.unique_keys = unique_keys;

    schema.classes.insert("User".to_string(), user_class);

    let mut id_slot = SlotDefinition::new("id");
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let engine = Arc::new(ValidationEngine::new(&schema).expect("Test operation failed"));

    // Create multiple concurrent validation tasks
    let mut handles = vec![];

    for i in 0..10 {
        let engine_clone = Arc::clone(&engine);
        let handle = task::spawn(async move {
            // Each task validates multiple users
            let mut results = vec![];

            for j in 0..5 {
                let user = json!({
                    "id": format!("user_{}_{}", i, j),
                    "email": format!("user{}@example.com", i * 5 + j)
                });

                let report = engine_clone
                    .validate_as_class(&user, "User", None)
                    .await
                    .expect("Test operation failed");
                results.push(report.valid);
            }

            results
        });

        handles.push(handle);
    }

    // Wait for all tasks
    let mut all_results = vec![];
    for handle in handles {
        let results = handle.await.expect("Test operation failed");
        all_results.extend(results);
    }

    // All validations should pass (no duplicates)
    assert!(all_results.iter().all(|&valid| valid));

    // Now test duplicate detection across concurrent validations
    let engine2 = Arc::new(ValidationEngine::new(&schema).expect("Test operation failed"));

    // First, validate some users to populate the tracker
    let user1 = json!({
        "id": "1",
        "email": "taken@example.com"
    });

    engine2
        .validate_as_class(&user1, "User", None)
        .await
        .expect("Test operation failed");

    // Now spawn concurrent tasks that try to use the same email
    let mut duplicate_handles = vec![];

    for i in 0..5 {
        let engine_clone = Arc::clone(&engine2);
        let handle = task::spawn(async move {
            let user = json!({
                "id": format!("dup_{}", i),
                "email": "taken@example.com"  // Same email!
            });

            let report = engine_clone
                .validate_as_class(&user, "User", None)
                .await
                .expect("Test operation failed");
            report.valid
        });

        duplicate_handles.push(handle);
    }

    // Collect results
    let duplicate_results: Vec<bool> = futures::future::join_all(duplicate_handles)
        .await
        .into_iter()
        .map(|r| r.expect("Test operation failed"))
        .collect();

    // All should fail due to duplicate email
    assert!(duplicate_results.iter().all(|&valid| !valid));
}

#[tokio::test]
async fn test_composite_unique_keys_edge_cases() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut order_class = ClassDefinition::new("Order");
    order_class.slots = vec![
        "customer_id".to_string(),
        "order_date".to_string(),
        "product_id".to_string(),
    ];

    // Composite unique key
    let mut composite_key = UniqueKeyDefinition::default();
    composite_key.unique_key_slots = vec![
        "customer_id".to_string(),
        "order_date".to_string(),
        "product_id".to_string(),
    ];
    composite_key.consider_nulls_inequal = Some(true);
    order_class
        .unique_keys
        .insert("customer_date_product".to_string(), composite_key);

    schema.classes.insert("Order".to_string(), order_class);

    for slot_name in ["customer_id", "order_date", "product_id"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test various edge cases

    // Case 1: All fields identical - should fail
    let order1 = json!({
        "customer_id": "C123",
        "order_date": "2024-01-15",
        "product_id": "P456"
    });

    let report1 = engine
        .validate_as_class(&order1, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report1.valid);

    let order1_dup = json!({
        "customer_id": "C123",
        "order_date": "2024-01-15",
        "product_id": "P456"
    });

    let report1_dup = engine
        .validate_as_class(&order1_dup, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(!report1_dup.valid);

    // Case 2: One field different - should pass
    let order2 = json!({
        "customer_id": "C123",
        "order_date": "2024-01-16",  // Different date
        "product_id": "P456"
    });

    let report2 = engine
        .validate_as_class(&order2, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report2.valid);

    // Case 3: Null handling with consider_nulls_inequal = true
    let order3 = json!({
        "customer_id": "C123",
        "order_date": null,
        "product_id": "P456"
    });

    let report3 = engine
        .validate_as_class(&order3, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report3.valid);

    // Another order with same values but null date - should pass (nulls are inequal)
    let order4 = json!({
        "customer_id": "C123",
        "order_date": null,
        "product_id": "P456"
    });

    let report4 = engine
        .validate_as_class(&order4, "Order", None)
        .await
        .expect("Test operation failed");
    assert!(report4.valid); // Passes because nulls are considered inequal
}

#[tokio::test]
async fn test_unique_validation_with_updates() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec!["id".to_string(), "sku".to_string(), "name".to_string()];

    // SKU should be unique
    let mut sku_key = UniqueKeyDefinition::default();
    sku_key.unique_key_slots = vec!["sku".to_string()];
    sku_key.consider_nulls_inequal = Some(false);
    product_class
        .unique_keys
        .insert("sku_key".to_string(), sku_key);

    schema.classes.insert("Product".to_string(), product_class);

    for (slot_name, range) in [("id", "string"), ("sku", "string"), ("name", "string")] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some(range.to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Simulate updating a record (would need to handle in real implementation)
    let product1 = json!({
        "id": "1",
        "sku": "ABC-123",
        "name": "Product 1"
    });

    let report1 = engine
        .validate_as_class(&product1, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report1.valid);

    // "Update" the same product (same ID, same SKU) - should ideally pass
    // In a real system, we'd need to track IDs to allow updates
    let product1_updated = json!({
        "id": "1",
        "sku": "ABC-123",
        "name": "Product 1 Updated"
    });

    // This will fail in current implementation - documents the limitation
    let report_update = engine
        .validate_as_class(&product1_updated, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(!report_update.valid); // Current behavior - treats as duplicate

    // Different product with different SKU - should pass
    let product2 = json!({
        "id": "2",
        "sku": "XYZ-789",
        "name": "Product 2"
    });

    let report2 = engine
        .validate_as_class(&product2, "Product", None)
        .await
        .expect("Test operation failed");
    assert!(report2.valid);
}

#[tokio::test]
async fn test_unique_keys_with_inheritance() {
    let mut schema = SchemaDefinition::new("test_schema");

    // Base class with ID
    let mut entity = ClassDefinition::new("Entity");
    entity.slots = vec!["id".to_string()];
    let mut id_key = UniqueKeyDefinition::default();
    id_key.unique_key_slots = vec!["id".to_string()];
    id_key.consider_nulls_inequal = Some(false);
    entity.unique_keys.insert("id_key".to_string(), id_key);
    schema.classes.insert("Entity".to_string(), entity);

    // Derived class adds email uniqueness
    let mut user = ClassDefinition::new("User");
    user.is_a = Some("Entity".to_string());
    user.slots = vec!["email".to_string()];
    let mut email_key = UniqueKeyDefinition::default();
    email_key.unique_key_slots = vec!["email".to_string()];
    email_key.consider_nulls_inequal = Some(false);
    user.unique_keys.insert("email_key".to_string(), email_key);
    schema.classes.insert("User".to_string(), user);

    for (slot_name, range) in [("id", "string"), ("email", "string")] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some(range.to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Both ID and email should be unique for User
    let user1 = json!({
        "id": "U1",
        "email": "user1@example.com"
    });

    let report1 = engine
        .validate_as_class(&user1, "User", None)
        .await
        .expect("Test operation failed");
    assert!(report1.valid);

    // Duplicate ID - should fail
    let user2 = json!({
        "id": "U1",  // Duplicate!
        "email": "user2@example.com"
    });

    let report2 = engine
        .validate_as_class(&user2, "User", None)
        .await
        .expect("Test operation failed");
    assert!(!report2.valid);

    // Duplicate email - should fail
    let user3 = json!({
        "id": "U3",
        "email": "user1@example.com"  // Duplicate!
    });

    let report3 = engine
        .validate_as_class(&user3, "User", None)
        .await
        .expect("Test operation failed");
    assert!(!report3.valid);
}

#[tokio::test]
async fn test_unique_validation_memory_efficiency() {
    use std::time::Instant;

    let mut schema = SchemaDefinition::new("test_schema");

    let mut record = ClassDefinition::new("Record");
    record.slots = vec!["id".to_string(), "code".to_string()];
    let mut code_key = UniqueKeyDefinition::default();
    code_key.unique_key_slots = vec!["code".to_string()];
    code_key.consider_nulls_inequal = Some(false);
    record.unique_keys.insert("code_key".to_string(), code_key);
    schema.classes.insert("Record".to_string(), record);

    for (slot_name, range) in [("id", "integer"), ("code", "string")] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some(range.to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with many unique values
    let start = Instant::now();
    let mut validation_times = vec![];

    for i in 0..1000 {
        let record = json!({
            "id": i,
            "code": format!("CODE-{:06}", i)
        });

        let validation_start = Instant::now();
        let report = engine
            .validate_as_class(&record, "Record", None)
            .await
            .expect("Test operation failed");
        validation_times.push(validation_start.elapsed());

        assert!(report.valid);
    }

    let total_time = start.elapsed();

    // Performance should remain consistent
    let avg_time =
        validation_times.iter().sum::<std::time::Duration>() / validation_times.len() as u32;
    let first_100_avg = validation_times[..100].iter().sum::<std::time::Duration>() / 100;
    let last_100_avg = validation_times[900..].iter().sum::<std::time::Duration>() / 100;

    // Last validations shouldn't be significantly slower than first
    assert!(last_100_avg.as_micros() < first_100_avg.as_micros() * 2);

    println!("Total time for 1000 validations: {:?}", total_time);
    println!("Average validation time: {:?}", avg_time);
}

#[tokio::test]
async fn test_unique_keys_with_special_characters() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut document = ClassDefinition::new("Document");
    document.slots = vec!["path".to_string()];
    let mut path_key = UniqueKeyDefinition::default();
    path_key.unique_key_slots = vec!["path".to_string()];
    path_key.consider_nulls_inequal = Some(false);
    document
        .unique_keys
        .insert("path_key".to_string(), path_key);
    schema.classes.insert("Document".to_string(), document);

    let mut path_slot = SlotDefinition::new("path");
    path_slot.range = Some("string".to_string());
    schema.slots.insert("path".to_string(), path_slot);

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with various special characters
    let test_paths = vec![
        "/home/user/file.txt",
        "C:\\Users\\Documents\\file.txt",
        "file with spaces.txt",
        "file[brackets].txt",
        "file{braces}.txt",
        "file(parens).txt",
        "file@symbol.txt",
        "file#hash.txt",
        "file$dollar.txt",
        "file%percent.txt",
        "file&ampersand.txt",
        "file*asterisk.txt",
        "file+plus.txt",
        "file=equals.txt",
        "émojis-🚀-✨.txt",
        "unicode-测试-файл.txt",
    ];

    for path in test_paths {
        let doc = json!({
            "path": path
        });

        let report = engine
            .validate_as_class(&doc, "Document", None)
            .await
            .expect("Test operation failed");
        assert!(report.valid, "Failed for path: {}", path);

        // Duplicate should fail
        let doc_dup = json!({
            "path": path
        });

        let report_dup = engine
            .validate_as_class(&doc_dup, "Document", None)
            .await
            .expect("Test operation failed");
        assert!(
            !report_dup.valid,
            "Duplicate not detected for path: {}",
            path
        );
    }
}

#[tokio::test]
async fn test_unique_validation_with_empty_values() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut config = ClassDefinition::new("Config");
    config.slots = vec!["key".to_string(), "value".to_string()];
    let mut key_unique = UniqueKeyDefinition::default();
    key_unique.unique_key_slots = vec!["key".to_string()];
    key_unique.consider_nulls_inequal = Some(false);
    config
        .unique_keys
        .insert("key_unique".to_string(), key_unique);
    schema.classes.insert("Config".to_string(), config);

    for slot_name in ["key", "value"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test empty string as unique value
    let config1 = json!({
        "key": "",
        "value": "empty key"
    });

    let report1 = engine
        .validate_as_class(&config1, "Config", None)
        .await
        .expect("Test operation failed");
    assert!(report1.valid);

    // Another empty string - should fail
    let config2 = json!({
        "key": "",
        "value": "another empty key"
    });

    let report2 = engine
        .validate_as_class(&config2, "Config", None)
        .await
        .expect("Test operation failed");
    assert!(!report2.valid);

    // Whitespace variations
    let config3 = json!({
        "key": " ",  // Single space
        "value": "space key"
    });

    let report3 = engine
        .validate_as_class(&config3, "Config", None)
        .await
        .expect("Test operation failed");
    assert!(report3.valid);

    // Different whitespace - should be treated as different
    let config4 = json!({
        "key": "  ",  // Two spaces
        "value": "two space key"
    });

    let report4 = engine
        .validate_as_class(&config4, "Config", None)
        .await
        .expect("Test operation failed");
    assert!(report4.valid); // Different from single space
}
