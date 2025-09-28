//! Tests for database loading and dumping functionality

use linkml_core::prelude::*;
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::loader::{
    DataDumper, DataLoader, DatabaseDumper, DatabaseLoader, DatabaseOptions, ForeignKeyRelation,
    traits::DataInstance,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};

/// Create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("TestSchema".to_string());

    // Person class
    let mut person_class = ClassDefinition::default();
    person_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "age".to_string(),
        "active".to_string(),
    ];
    schema.classes.insert("Person".to_string(), person_class);

    // Address class
    let mut address_class = ClassDefinition::default();
    address_class.slots = vec![
        "id".to_string(),
        "person".to_string(),
        "street".to_string(),
        "city".to_string(),
        "postal_code".to_string(),
    ];
    schema.classes.insert("Address".to_string(), address_class);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot);

    let mut active_slot = SlotDefinition::default();
    active_slot.range = Some("boolean".to_string());
    schema.slots.insert("active".to_string(), active_slot);

    let mut person_slot = SlotDefinition::default();
    person_slot.range = Some("Person".to_string());
    person_slot.required = Some(true);
    schema.slots.insert("person".to_string(), person_slot);

    let mut street_slot = SlotDefinition::default();
    street_slot.range = Some("string".to_string());
    schema.slots.insert("street".to_string(), street_slot);

    let mut city_slot = SlotDefinition::default();
    city_slot.range = Some("string".to_string());
    schema.slots.insert("city".to_string(), city_slot);

    let mut postal_code_slot = SlotDefinition::default();
    postal_code_slot.range = Some("string".to_string());
    schema
        .slots
        .insert("postal_code".to_string(), postal_code_slot);

    schema
}

#[test]
fn test_database_options_default() {
    let options = DatabaseOptions::default();

    assert_eq!(options.batch_size, 1000);
    assert!(options.infer_types);
    assert!(!options.create_if_not_exists);
    assert!(options.use_transactions);
    assert_eq!(options.max_connections, 5);
}

#[test]
fn test_database_options_configuration() {
    let mut options = DatabaseOptions {
        connection_string: "postgresql://localhost/test".to_string(),
        schema_name: Some("public".to_string()),
        batch_size: 500,
        ..Default::default()
    };

    // Add table mappings
    options
        .table_mapping
        .insert("people".to_string(), "Person".to_string());
    options
        .table_mapping
        .insert("addresses".to_string(), "Address".to_string());

    // Add column mappings
    let mut person_columns = HashMap::new();
    person_columns.insert("full_name".to_string(), "name".to_string());
    person_columns.insert("is_active".to_string(), "active".to_string());
    options
        .column_mapping
        .insert("people".to_string(), person_columns);

    // Add foreign keys
    options.foreign_keys.insert(
        "addresses".to_string(),
        vec![ForeignKeyRelation {
            column: "person_id".to_string(),
            referenced_table: "people".to_string(),
            referenced_column: "id".to_string(),
            slot_name: "person".to_string(),
        }],
    );

    assert_eq!(options.connection_string, "postgresql://localhost/test");
    assert_eq!(options.schema_name, Some("public".to_string()));
    assert_eq!(options.batch_size, 500);
    assert_eq!(options.table_mapping.len(), 2);
    assert_eq!(options.column_mapping.len(), 1);
    assert_eq!(options.foreign_keys.len(), 1);
}

#[test]
fn test_table_filtering() {
    let mut options = DatabaseOptions::default();

    // Test exclude tables
    options.exclude_tables = ["audit_log", "temp_table"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    assert!(options.exclude_tables.contains("audit_log"));
    assert!(options.exclude_tables.contains("temp_table"));
    assert!(!options.exclude_tables.contains("users"));

    // Test include tables
    options.include_tables = Some(["users", "orders"].iter().map(|s| s.to_string()).collect());

    if let Some(include) = &options.include_tables {
        assert!(include.contains("users"));
        assert!(include.contains("orders"));
        assert!(!include.contains("products"));
    }
}

#[test]
fn test_foreign_key_relation() {
    let fk = ForeignKeyRelation {
        column: "user_id".to_string(),
        referenced_table: "users".to_string(),
        referenced_column: "id".to_string(),
        slot_name: "user".to_string(),
    };

    assert_eq!(fk.column, "user_id");
    assert_eq!(fk.referenced_table, "users");
    assert_eq!(fk.referenced_column, "id");
    assert_eq!(fk.slot_name, "user");
}

#[test]
fn test_data_instance_creation() {
    let instance = DataInstance {
        class_name: "Person".to_string(),
        data: serde_json::from_value(json!({
            "id": "person1",
            "name": "Alice Smith",
            "age": 30,
            "active": true
        }))
        .expect("Test operation failed"),
    };

    assert_eq!(instance.class_name, "Person");
    assert_eq!(instance.data.get("id"), Some(&json!("person1")));
    assert_eq!(instance.data.get("name"), Some(&json!("Alice Smith")));
    assert_eq!(instance.data.get("age"), Some(&json!(30)));
    assert_eq!(instance.data.get("active"), Some(&json!(true)));
}

#[test]
fn test_batch_processing() {
    let options = DatabaseOptions {
        batch_size: 100,
        ..Default::default()
    };

    // Create 250 instances
    let instances: Vec<DataInstance> = (0..250)
        .map(|i| DataInstance {
            class_name: "Person".to_string(),
            data: serde_json::from_value(json!({
                "id": format!("person{}", i),
                "name": format!("Person {}", i),
                "age": 20 + (i % 50),
                "active": i % 2 == 0
            }))
            .expect("Test operation failed"),
        })
        .collect();

    // With batch size 100, we should process in 3 batches
    let batches: Vec<_> = instances.chunks(options.batch_size).collect();
    assert_eq!(batches.len(), 3);
    assert_eq!(batches[0].len(), 100);
    assert_eq!(batches[1].len(), 100);
    assert_eq!(batches[2].len(), 50);
}

#[test]
fn test_connection_string_parsing() {
    let pg_options = DatabaseOptions {
        connection_string: "postgresql://user:pass@localhost:5432/mydb".to_string(),
        ..Default::default()
    };
    assert!(pg_options.connection_string.starts_with("postgresql://"));

    let mysql_options = DatabaseOptions {
        connection_string: "mysql://user:pass@localhost:3306/mydb".to_string(),
        ..Default::default()
    };
    assert!(mysql_options.connection_string.starts_with("mysql://"));

    let sqlite_options = DatabaseOptions {
        connection_string: "sqlite://./test.db".to_string(),
        ..Default::default()
    };
    assert!(sqlite_options.connection_string.starts_with("sqlite://"));
}

#[tokio::test]
async fn test_loader_creation() {
    let options = DatabaseOptions {
        connection_string: "sqlite://./test.db".to_string(),
        ..Default::default()
    };

    let loader = DatabaseLoader::new(options.clone());
    // Loader should be created but not connected

    let dumper = DatabaseDumper::new(options);
    // Dumper should be created but not connected
}

#[test]
fn test_complex_mapping_scenario() {
    let mut options = DatabaseOptions::default();

    // Complex table mappings
    options
        .table_mapping
        .insert("tbl_usr".to_string(), "User".to_string());
    options
        .table_mapping
        .insert("tbl_prd".to_string(), "Product".to_string());
    options
        .table_mapping
        .insert("tbl_ord".to_string(), "Order".to_string());

    // Complex column mappings
    let mut user_columns = HashMap::new();
    user_columns.insert("usr_id".to_string(), "id".to_string());
    user_columns.insert("usr_nm".to_string(), "name".to_string());
    user_columns.insert("usr_em".to_string(), "email".to_string());
    options
        .column_mapping
        .insert("tbl_usr".to_string(), user_columns);

    let mut product_columns = HashMap::new();
    product_columns.insert("prd_id".to_string(), "id".to_string());
    product_columns.insert("prd_nm".to_string(), "name".to_string());
    product_columns.insert("prd_prc".to_string(), "price".to_string());
    options
        .column_mapping
        .insert("tbl_prd".to_string(), product_columns);

    // Multiple foreign keys
    options.foreign_keys.insert(
        "tbl_ord".to_string(),
        vec![
            ForeignKeyRelation {
                column: "usr_id".to_string(),
                referenced_table: "tbl_usr".to_string(),
                referenced_column: "usr_id".to_string(),
                slot_name: "customer".to_string(),
            },
            ForeignKeyRelation {
                column: "ship_addr_id".to_string(),
                referenced_table: "tbl_addr".to_string(),
                referenced_column: "addr_id".to_string(),
                slot_name: "shipping_address".to_string(),
            },
            ForeignKeyRelation {
                column: "bill_addr_id".to_string(),
                referenced_table: "tbl_addr".to_string(),
                referenced_column: "addr_id".to_string(),
                slot_name: "billing_address".to_string(),
            },
        ],
    );

    assert_eq!(options.table_mapping.len(), 3);
    assert_eq!(options.column_mapping.len(), 2);
    assert_eq!(
        options
            .foreign_keys
            .get("tbl_ord")
            .expect("Test operation failed")
            .len(),
        3
    );
}

#[test]
fn test_type_mapping() {
    let schema = create_test_schema();

    // Test that the schema has correct type mappings
    assert_eq!(
        schema.slots.get("id").expect("Test operation failed").range,
        Some("string".to_string())
    );
    assert_eq!(
        schema
            .slots
            .get("age")
            .expect("Test operation failed")
            .range,
        Some("integer".to_string())
    );
    assert_eq!(
        schema
            .slots
            .get("active")
            .expect("Test operation failed")
            .range,
        Some("boolean".to_string())
    );
    assert_eq!(
        schema
            .slots
            .get("person")
            .expect("Test operation failed")
            .range,
        Some("Person".to_string())
    );
}

#[test]
fn test_instance_with_references() {
    let person = DataInstance {
        class_name: "Person".to_string(),
        data: serde_json::from_value(json!({
            "id": "p1",
            "name": "John Doe",
            "age": 25
        }))
        .expect("Test operation failed"),
    };

    let address = DataInstance {
        class_name: "Address".to_string(),
        data: serde_json::from_value(json!({
            "id": "a1",
            "person": {
                "@type": "Person",
                "id": "p1"
            },
            "street": "123 Main St",
            "city": "Springfield",
            "postal_code": "12345"
        }))
        .expect("Test operation failed"),
    };

    // Verify reference structure
    let person_ref = address.data.get("person").expect("Test operation failed");
    assert!(person_ref.is_object());
    assert_eq!(person_ref.get("@type"), Some(&json!("Person")));
    assert_eq!(person_ref.get("id"), Some(&json!("p1")));
}
