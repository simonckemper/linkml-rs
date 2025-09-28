//! Tests for schema settings integration

use linkml_core::{
    settings::{SchemaSettings, ValidationSettings},
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;

#[tokio::test]
async fn test_strict_validation_settings() {
    // Create a schema with strict settings
    let mut schema = SchemaDefinition::new("test_schema");
    schema.settings = Some(SchemaSettings::strict());

    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot);

    // Create validation engine
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with additional properties (should fail in strict mode)
    let data_with_extra = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john@example.com"  // Extra field
    });

    let report = engine
        .validate_as_class(&data_with_extra, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("Unknown slot"));
}

#[tokio::test]
async fn test_default_settings_allow_additional() {
    // Create a schema without settings (default behavior)
    let mut schema = SchemaDefinition::new("test_schema");

    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    // Create validation engine
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test with additional properties (should only warn by default)
    let data_with_extra = json!({
        "name": "Jane Doe",
        "extra_field": "value"
    });

    let report = engine
        .validate_as_class(&data_with_extra, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid); // Should be valid
    let warnings: Vec<_> = report.warnings().collect();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("Unknown slot"));
}

#[tokio::test]
async fn test_options_override_settings() {
    // Create a schema with strict settings
    let mut schema = SchemaDefinition::new("test_schema");
    schema.settings = Some(SchemaSettings {
        validation: Some(ValidationSettings {
            fail_fast: Some(true),
            check_permissibles: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    });

    let mut status_class = ClassDefinition::new("Status");
    status_class.slots = vec!["code".to_string(), "message".to_string()];
    schema.classes.insert("Status".to_string(), status_class);

    let mut code_slot = SlotDefinition::new("code");
    code_slot.range = Some("string".to_string());
    code_slot.required = Some(true);
    schema.slots.insert("code".to_string(), code_slot);

    let mut message_slot = SlotDefinition::new("message");
    message_slot.range = Some("string".to_string());
    message_slot.required = Some(true);
    schema.slots.insert("message".to_string(), message_slot);

    // Create validation engine
    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Create data with multiple errors
    let invalid_data = json!({
        // Missing both required fields
    });

    // Test with schema settings (fail_fast = true)
    let report = engine
        .validate_as_class(&invalid_data, "Status", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    // With fail_fast, we should only see one error
    assert_eq!(errors.len(), 1);

    // Test with explicit options that override settings
    let options = ValidationOptions {
        fail_fast: Some(false), // Override the schema setting
        ..Default::default()
    };

    let report = engine
        .validate_as_class(&invalid_data, "Status", Some(options))
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    let errors: Vec<_> = report.errors().collect();
    println!(
        "Errors without fail_fast: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>()
    );
    // Without fail_fast, we should see both errors
    assert_eq!(errors.len(), 2);
}

#[tokio::test]
async fn test_settings_from_yaml() {
    // Test that settings can be parsed from YAML
    let yaml_content = r#"
id: https://example.org/schema
name: test_schema
settings:
  validation:
    strict: true
    check_permissibles: true
    fail_fast: false
    allow_additional_properties: false
classes:
  Person:
    name: Person
    slots:
      - name
slots:
  name:
    name: name
    range: string
"#;

    use linkml_core::types::SchemaDefinition;
    use linkml_core::types::{ClassDefinition, SlotDefinition};
    use linkml_service::parser::{SchemaParser, YamlParser};
    let parser = YamlParser::new();
    let schema = parser.parse(yaml_content).expect("Test operation failed");

    assert!(schema.settings.is_some());
    let settings = schema.settings.as_ref().expect("Test operation failed");
    assert!(settings.validation.is_some());
    let validation = settings.validation.as_ref().expect("Test operation failed");
    assert_eq!(validation.strict, Some(true));
    assert_eq!(validation.check_permissibles, Some(true));
    assert_eq!(validation.fail_fast, Some(false));
    assert_eq!(validation.allow_additional_properties, Some(false));
}

#[test]
fn test_settings_merge() {
    let base_settings = SchemaSettings {
        validation: Some(ValidationSettings {
            strict: Some(true),
            fail_fast: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let override_settings = SchemaSettings {
        validation: Some(ValidationSettings {
            fail_fast: Some(false),
            check_permissibles: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };

    let merged = base_settings.merge(override_settings);
    let validation = merged.validation.expect("Test operation failed");
    assert_eq!(validation.fail_fast, Some(false)); // Overridden
    assert_eq!(validation.check_permissibles, Some(true)); // From override
}
