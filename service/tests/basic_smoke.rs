#![allow(missing_docs)]

use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_service::validator::ValidationEngine;
use serde_json::json;

#[tokio::test]
async fn validates_simple_instance() -> linkml_core::error::Result<()> {
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.org/smoke".to_string();
    schema.name = "SmokeTest".to_string();

    let person = ClassDefinition {
        name: "Person".to_string(),
        slots: vec!["id".to_string(), "name".to_string()],
        ..Default::default()
    };
    schema.classes.insert("Person".to_string(), person);

    let id_slot = SlotDefinition {
        name: "id".to_string(),
        identifier: Some(true),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("id".to_string(), id_slot);

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        required: Some(true),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("name".to_string(), name_slot);

    let engine = ValidationEngine::new(&schema)?;

    let valid = json!({
        "@type": "Person",
        "id": "person-1",
        "name": "Smoke Test"
    });

    let report = engine.validate(&valid, None).await?;
    assert!(report.valid);

    let invalid = json!({
        "@type": "Person",
        "id": "person-2"
    });
    let report = engine.validate(&invalid, None).await?;
    assert!(!report.valid);
    assert!(!report.issues.is_empty());

    Ok(())
}
