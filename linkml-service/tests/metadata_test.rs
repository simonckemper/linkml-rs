//! Tests for metadata support

use linkml_core::{
    metadata::{Contributor, Example},
    types::{SchemaDefinition, SlotDefinition},
};
use linkml_service::parser::{SchemaParser, YamlParser};
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{SlotDefinition};

#[test]
fn test_schema_metadata() {
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
title: Test Schema
description: A schema for testing metadata
version: 1.0.0
license: CC-BY-4.0
status: release
keywords:
  - test
  - example
categories:
  - testing
see_also:
  - https://example.org/related-schema
contributors:
  - name: Jane Smith
    email: jane@example.com
    github: janesmith
    orcid: 0000-0000-0000-0001
    role: lead
  - name: John Doe
    email: john@example.com
    role: contributor
classes:
  Person:
    name: Person
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Test operation failed");

    // Check basic metadata
    assert_eq!(schema.title, Some("Test Schema".to_string());
    assert_eq!(schema.status, Some("release".to_string());
    assert_eq!(schema.keywords.len(), 2);
    assert_eq!(schema.categories.len(), 1);
    assert_eq!(schema.see_also.len(), 1);

    // Check contributors
    assert_eq!(schema.contributors.len(), 2);
    let jane = &schema.contributors[0];
    assert_eq!(jane.name, "Jane Smith");
    assert_eq!(jane.email, Some("jane@example.com".to_string()));
    assert_eq!(jane.github, Some("janesmith".to_string()));
    assert_eq!(jane.orcid, Some("0000-0000-0000-0001".to_string()));
    assert_eq!(jane.role, Some("lead".to_string()));
}

#[test]
fn test_class_metadata() {
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
classes:
  Dataset:
    name: Dataset
    description: A collection of data
    aliases:
      - DataCollection
      - DataSet
    see_also:
      - https://schema.org/Dataset
      - https://www.w3.org/TR/vocab-dcat/#Class:Dataset
    deprecated: Use DataPackage instead
    todos:
      - Add validation for size constraints
      - Implement data quality checks
    notes:
      - This class is being phased out
    comments:
      - Consider using DataPackage for new projects
    examples:
      - value: "genomic_dataset_001"
        description: "A genomic dataset identifier"
      - value: "survey_responses_2023"
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Test operation failed");

    let dataset_class = schema
        .classes
        .get("Dataset")
        .expect("Test operation failed");

    // Check metadata
    assert_eq!(dataset_class.aliases.len(), 2);
    assert_eq!(dataset_class.see_also.len(), 2);
    assert_eq!(
        dataset_class.deprecated,
        Some("Use DataPackage instead".to_string())
    );
    assert_eq!(dataset_class.todos.len(), 2);
    assert_eq!(dataset_class.notes.len(), 1);
    assert_eq!(dataset_class.comments.len(), 1);

    // Check examples
    assert_eq!(dataset_class.examples.len(), 2);
    assert_eq!(dataset_class.examples[0].value, "genomic_dataset_001");
    assert_eq!(
        dataset_class.examples[0].description,
        Some("A genomic dataset identifier".to_string())
    );
    assert_eq!(dataset_class.examples[1].value, "survey_responses_2023");
    assert!(dataset_class.examples[1].description.is_none());
}

#[test]
fn test_slot_metadata() {
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
classes:
  Person:
    name: Person
    slots:
      - email
slots:
  email:
    name: email
    description: Email address
    range: string
    aliases:
      - email_address
      - contact_email
    see_also:
      - https://schema.org/email
    examples:
      - value: user@example.com
        description: Standard email format
      - value: firstname.lastname@company.org
        description: Corporate email format
    deprecated: Use contact_info instead
    todos:
      - Add validation for email format
    notes:
      - Must be unique within organization
    comments:
      - Consider using a dedicated Email type
    rank: 10
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Test operation failed");

    let email_slot = schema.slots.get("email").expect("Test operation failed");

    // Check all metadata fields
    assert_eq!(email_slot.aliases.len(), 2);
    assert_eq!(email_slot.see_also.len(), 1);
    assert_eq!(email_slot.examples.len(), 2);
    assert_eq!(
        email_slot.deprecated,
        Some("Use contact_info instead".to_string())
    );
    assert_eq!(email_slot.todos.len(), 1);
    assert_eq!(email_slot.notes.len(), 1);
    assert_eq!(email_slot.comments.len(), 1);
    assert_eq!(email_slot.rank, Some(10));
}

#[test]
fn test_metadata_inheritance() {
    // Test that metadata is properly merged during slot inheritance
    let _schema = SchemaDefinition::new("test_schema");

    // Base slot with some metadata
    let mut base_slot = SlotDefinition::new("base");
    base_slot.aliases.push("original_name".to_string());
    base_slot
        .see_also
        .push("https://example.org/base".to_string());
    base_slot.notes.push("Base note".to_string());
    base_slot.rank = Some(1);

    // Override slot with additional metadata
    let mut override_slot = SlotDefinition::new("base");
    override_slot.aliases.push("new_name".to_string());
    override_slot
        .see_also
        .push("https://example.org/override".to_string());
    override_slot.notes.push("Override note".to_string());
    override_slot.deprecated = Some("Use new_slot instead".to_string());
    override_slot.rank = Some(2);

    // Merge the slots
    let merged = linkml_core::utils::merge_slot_definitions(&base_slot, &override_slot);

    // Check merged metadata
    assert_eq!(merged.aliases.len(), 2); // Both aliases preserved
    assert_eq!(merged.see_also.len(), 2); // Both references preserved
    assert_eq!(merged.notes.len(), 2); // Both notes preserved
    assert_eq!(merged.deprecated, Some("Use new_slot instead".to_string()));
    assert_eq!(merged.rank, Some(2)); // Override wins
}

#[test]
fn test_example_serialization() {
    let mut slot = SlotDefinition::new("test_slot");
    slot.examples.push(Example {
        value: "example1".to_string(),
        description: Some("First example".to_string()),
    });
    slot.examples.push(Example {
        value: "example2".to_string(),
        description: None,
    });

    let json = serde_json::to_string_pretty(&slot).expect("Test operation failed");
    assert!(json.contains(r#""value": "example1""#));
    assert!(json.contains(r#""description": "First example""#));

    // Deserialize back
    let parsed: SlotDefinition = serde_json::from_str(&json).expect("Test operation failed");
    assert_eq!(parsed.examples.len(), 2);
    assert_eq!(parsed.examples[0].value, "example1");
}

#[test]
fn test_contributor_serialization() {
    let mut schema = SchemaDefinition::new("test_schema");
    schema.contributors.push(Contributor {
        name: "Alice Johnson".to_string(),
        email: Some("alice@example.com".to_string()),
        github: Some("alicej".to_string()),
        orcid: None,
        role: Some("maintainer".to_string()),
    });

    let yaml = serde_yaml::to_string(&schema).expect("Test operation failed");
    assert!(yaml.contains("Alice Johnson"));
    assert!(yaml.contains("maintainer"));

    // Parse back
    let parsed: SchemaDefinition = serde_yaml::from_str(&yaml).expect("Test operation failed");
    assert_eq!(parsed.contributors.len(), 1);
    assert_eq!(parsed.contributors[0].name, "Alice Johnson");
}
