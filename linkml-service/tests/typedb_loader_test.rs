//! Tests for TypeDB loader integration

use async_trait::async_trait;
use linkml_core::prelude::*;
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::loader::{
    DataDumper, DataLoader, TypeDBIntegrationDumper, TypeDBIntegrationLoader,
    TypeDBIntegrationOptions, TypeDBQueryExecutor, traits::DataInstance,
};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error as StdError;

/// Mock TypeDB executor for testing
struct MockTypeDBExecutor {
    /// Expected queries and their responses
    responses: HashMap<String, String>,
}

impl MockTypeDBExecutor {
    fn new() -> Self {
        let mut responses = HashMap::new();

        // Mock entity types response
        responses.insert(
            "match $x sub entity; get $x;".to_string(),
            r#"[
                {"x": {"label": "entity", "abstract": false}},
                {"x": {"label": "person", "abstract": false}},
                {"x": {"label": "organization", "abstract": false}}
            ]"#
            .to_string(),
        );

        // Mock relation types response
        responses.insert(
            "match $x sub relation; get $x;".to_string(),
            r#"[
                {"x": {"label": "relation", "abstract": false}},
                {"x": {"label": "employment", "abstract": false}},
                {"x": {"label": "friendship", "abstract": false}}
            ]"#
            .to_string(),
        );

        // Mock person attributes
        responses.insert(
            "match $type type person; $type owns $attr; get $attr;".to_string(),
            r#"[
                {"attr": {"label": "full_name", "value_type": "string"}},
                {"attr": {"label": "age", "value_type": "long"}},
                {"attr": {"label": "email", "value_type": "string"}}
            ]"#
            .to_string(),
        );

        // Mock organization attributes
        responses.insert(
            "match $type type organization; $type owns $attr; get $attr;".to_string(),
            r#"[
                {"attr": {"label": "name", "value_type": "string"}},
                {"attr": {"label": "founded_year", "value_type": "long"}}
            ]"#
            .to_string(),
        );

        // Mock employment attributes
        responses.insert(
            "match $type type employment; $type owns $attr; get $attr;".to_string(),
            r#"[
                {"attr": {"label": "start_date", "value_type": "datetime"}},
                {"attr": {"label": "role", "value_type": "string"}}
            ]"#
            .to_string(),
        );

        // Mock employment roles
        responses.insert(
            "match $rel type employment; $rel relates $role; get $role;".to_string(),
            r#"[
                {"role": {"label": "employee"}},
                {"role": {"label": "employer"}}
            ]"#
            .to_string(),
        );

        // Mock person instances
        responses.insert(
            "match $x isa person; $x has full_name $attr_full_name; $x has age $attr_age; $x has email $attr_email; get $x, $attr_full_name, $attr_age, $attr_email;".to_string(),
            r#"[
                {
                    "x": {"iid": "person-001"},
                    "attr_full_name": {"value": "Alice Johnson"},
                    "attr_age": {"value": 30},
                    "attr_email": {"value": "alice@example.com"}
                },
                {
                    "x": {"iid": "person-002"},
                    "attr_full_name": {"value": "Bob Smith"},
                    "attr_age": {"value": 25},
                    "attr_email": {"value": "bob@example.com"}
                }
            ]"#.to_string()
        );

        Self { responses }
    }
}

#[async_trait]
impl TypeDBQueryExecutor for MockTypeDBExecutor {
    async fn execute_query(
        &self,
        query: &str,
        _database: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.responses
            .get(query)
            .cloned()
            .ok_or_else(|| format!("No mock response for query: {}", query).into())
    }

    async fn execute_define(
        &self,
        _query: &str,
        _database: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn execute_insert(
        &self,
        _query: &str,
        _database: &str,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

/// Create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("TestSchema".to_string());

    // Person class
    let mut person_class = ClassDefinition::default();
    person_class.slots = vec![
        "id".to_string(),
        "full_name".to_string(),
        "age".to_string(),
        "email".to_string(),
    ];
    schema.classes.insert("Person".to_string(), person_class);

    // Organization class
    let mut org_class = ClassDefinition::default();
    org_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "founded_year".to_string(),
    ];
    schema.classes.insert("Organization".to_string(), org_class);

    // Employment relation
    let mut employment_class = ClassDefinition::default();
    employment_class.slots = vec![
        "employee".to_string(),
        "employer".to_string(),
        "start_date".to_string(),
        "role".to_string(),
    ];
    schema
        .classes
        .insert("Employment".to_string(), employment_class);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    schema
        .slots
        .insert("full_name".to_string(), name_slot.clone());
    schema.slots.insert("name".to_string(), name_slot.clone());
    schema.slots.insert("role".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot.clone());
    schema.slots.insert("founded_year".to_string(), age_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut date_slot = SlotDefinition::default();
    date_slot.range = Some("datetime".to_string());
    schema.slots.insert("start_date".to_string(), date_slot);

    let mut employee_slot = SlotDefinition::default();
    employee_slot.range = Some("Person".to_string());
    schema.slots.insert("employee".to_string(), employee_slot);

    let mut employer_slot = SlotDefinition::default();
    employer_slot.range = Some("Organization".to_string());
    schema.slots.insert("employer".to_string(), employer_slot);

    schema
}

#[tokio::test]
async fn test_typedb_loader_with_mock() {
    let options = TypeDBIntegrationOptions {
        database_name: "test".to_string(),
        ..Default::default()
    };

    let executor = MockTypeDBExecutor::new();
    let mut loader = TypeDBIntegrationLoader::new(options, executor);
    let schema = create_test_schema();

    let instances = loader.load(&schema).await.expect("Test operation failed");

    // Should have loaded 2 person instances
    assert_eq!(instances.len(), 2);

    // Check first person
    let person1 = &instances[0];
    assert_eq!(person1.class_name, "Person");
    assert_eq!(person1.data.get("full_name"), Some(&json!("Alice Johnson")));
    assert_eq!(person1.data.get("age"), Some(&json!(30)));
    assert_eq!(person1.data.get("email"), Some(&json!("alice@example.com")));

    // Check second person
    let person2 = &instances[1];
    assert_eq!(person2.class_name, "Person");
    assert_eq!(person2.data.get("full_name"), Some(&json!("Bob Smith")));
    assert_eq!(person2.data.get("age"), Some(&json!(25)));
}

#[tokio::test]
async fn test_typedb_dumper_with_mock() {
    let options = TypeDBIntegrationOptions {
        database_name: "test".to_string(),
        ..Default::default()
    };

    let executor = MockTypeDBExecutor::new();
    let mut dumper = TypeDBIntegrationDumper::new(options, executor);
    let schema = create_test_schema();

    let instances = vec![DataInstance {
        class_name: "Person".to_string(),
        data: serde_json::from_value(json!({
            "id": "p1",
            "full_name": "Test Person",
            "age": 35,
            "email": "test@example.com"
        }))
        .expect("Test operation failed"),
    }];

    let result = dumper
        .dump(&instances, &schema)
        .await
        .expect("Test operation failed");
    let summary = String::from_utf8(result).expect("Test operation failed");
    assert!(summary.contains("Successfully dumped 1 instances"));
}

#[test]
fn test_options_with_mappings() {
    let mut options = TypeDBIntegrationOptions::default();

    // Add type mappings
    options
        .type_mapping
        .insert("person".to_string(), "Person".to_string());
    options
        .type_mapping
        .insert("company".to_string(), "Organization".to_string());

    // Add attribute mappings
    let mut person_attrs = HashMap::new();
    person_attrs.insert("full-name".to_string(), "full_name".to_string());
    person_attrs.insert("birth-date".to_string(), "birth_date".to_string());
    options
        .attribute_mapping
        .insert("person".to_string(), person_attrs);

    assert_eq!(options.type_mapping.len(), 2);
    assert_eq!(options.attribute_mapping.len(), 1);
    assert_eq!(
        options
            .attribute_mapping
            .get("person")
            .expect("Test operation failed")
            .get("full-name"),
        Some(&"full_name".to_string())
    );
}

#[test]
fn test_configuration() {
    let options = TypeDBIntegrationOptions {
        database_name: "knowledge-base".to_string(),
        batch_size: 500,
        infer_types: false,
        include_inferred: true,
        query_timeout_ms: 60000,
        ..Default::default()
    };

    assert_eq!(options.database_name, "knowledge-base");
    assert_eq!(options.batch_size, 500);
    assert!(!options.infer_types);
    assert!(options.include_inferred);
    assert_eq!(options.query_timeout_ms, 60000);
}
