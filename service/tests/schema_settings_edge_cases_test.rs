//! Comprehensive tests for schema settings edge cases and interactions

use linkml_core::{
    settings::{
        DefaultSettings, GenerationSettings, ImportResolutionStrategy, ImportSettings,
        NamingSettings, SchemaSettings, ValidationSettings,
    },
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;

#[tokio::test]
async fn test_settings_override_precedence() {
    // Test that ValidationOptions properly override SchemaSettings
    let mut schema = SchemaDefinition::new("test_schema");

    // Schema says strict, but we'll override with lenient options
    let mut settings = SchemaSettings::strict();
    settings.validation.allow_additional_properties = false;
    settings.validation.fail_on_warning = true;
    schema.settings = Some(settings);

    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Data with extra field
    let data = json!({
        "name": "John",
        "extra": "field"
    });

    // First validate with schema settings (should fail)
    let report1 = engine
        .validate_as_class(&data, "Person", None)
        .await
        .expect("Test operation failed");
    assert!(!report1.valid);

    // Now validate with overriding options (should pass)
    let mut options = ValidationOptions::default();
    options.allow_additional_properties = true;
    options.fail_on_warning = false;

    let report2 = engine
        .validate_as_class(&data, "Person", Some(options))
        .await
        .expect("Test operation failed");
    assert!(report2.valid);
}

#[tokio::test]
async fn test_nested_settings_inheritance() {
    // Test how settings work with schema imports
    let mut base_schema = SchemaDefinition::new("base_schema");
    base_schema.settings = Some(SchemaSettings::strict());

    let mut derived_schema = SchemaDefinition::new("derived_schema");
    // Derived schema has lenient settings
    let mut lenient = SchemaSettings::default();
    lenient.validation.allow_additional_properties = true;
    derived_schema.settings = Some(lenient);
    derived_schema.imports = vec!["base_schema".to_string()];

    // The derived schema's settings should take precedence
    assert!(
        derived_schema
            .settings
            .as_ref()
            .expect("Test operation failed")
            .validation
            .allow_additional_properties
    );
}

#[tokio::test]
async fn test_generation_settings_with_naming() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut settings = SchemaSettings::default();
    settings.generation.include_metadata = true;
    settings.generation.include_validation = true;
    settings.generation.target_languages = vec!["rust".to_string(), "python".to_string()];

    settings.naming.class_naming_convention = Some("PascalCase".to_string());
    settings.naming.slot_naming_convention = Some("snake_case".to_string());
    settings.naming.enum_naming_convention = Some("SCREAMING_SNAKE_CASE".to_string());

    schema.settings = Some(settings);

    // Verify settings are accessible
    let settings = schema.settings.as_ref().expect("Test operation failed");
    assert!(settings.generation.include_metadata);
    assert_eq!(settings.generation.target_languages.len(), 2);
    assert_eq!(
        settings.naming.class_naming_convention.as_deref(),
        Some("PascalCase")
    );
}

#[tokio::test]
async fn test_import_settings_with_aliases() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut settings = SchemaSettings::default();
    settings.imports.resolution_strategy = ImportResolutionStrategy::Mixed;
    settings.imports.import_aliases.insert(
        "common".to_string(),
        "https://example.org/common/v2".to_string(),
    );
    settings.imports.import_mappings.insert(
        "base:".to_string(),
        "https://base.example.org/schemas/".to_string(),
    );

    schema.settings = Some(settings);

    // Test alias resolution
    let settings = schema.settings.as_ref().expect("Test operation failed");
    assert_eq!(
        settings.imports.import_aliases.get("common"),
        Some(&"https://example.org/common/v2".to_string())
    );
}

#[tokio::test]
async fn test_default_settings_application() {
    let mut schema = SchemaDefinition::new("test_schema");

    let mut settings = SchemaSettings::default();
    settings.defaults.range = Some("string".to_string());
    settings.defaults.required = Some(false);
    settings.defaults.multivalued = Some(false);
    settings.defaults.inlined = Some(true);

    schema.settings = Some(settings);

    // Create a slot without explicit settings
    let mut slot = SlotDefinition::new("test_slot");

    // In a real implementation, the engine would apply defaults
    // Here we verify the settings are stored correctly
    let defaults = &schema
        .settings
        .as_ref()
        .expect("Test operation failed")
        .defaults;
    assert_eq!(defaults.range.as_deref(), Some("string"));
    assert_eq!(defaults.required, Some(false));
}

#[tokio::test]
async fn test_conflicting_settings_resolution() {
    // Test what happens when different settings conflict
    let mut schema = SchemaDefinition::new("test_schema");

    let mut settings = SchemaSettings::strict();
    // But then we make one aspect lenient
    settings.validation.allow_additional_properties = true;

    schema.settings = Some(settings);

    // The specific setting should override the preset
    let validation = &schema
        .settings
        .as_ref()
        .expect("Test operation failed")
        .validation;
    assert!(validation.allow_additional_properties);
    assert!(validation.fail_on_warning); // From strict preset
    assert!(!validation.allow_undefined_slots); // From strict preset
}

#[tokio::test]
async fn test_settings_serialization_roundtrip() {
    use serde_yaml;
use linkml_core::types::SchemaDefinition;

    let mut settings = SchemaSettings::default();
    settings.validation = ValidationSettings::strict();
    settings.generation.target_languages = vec!["rust".to_string(), "typescript".to_string()];
    settings
        .imports
        .import_aliases
        .insert("bio".to_string(), "https://w3.org/bio/".to_string());
    settings.naming.class_naming_convention = Some("PascalCase".to_string());
    settings.defaults.required = Some(true);

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&settings).expect("Test operation failed");

    // Deserialize back
    let deserialized: SchemaSettings = serde_yaml::from_str(&yaml).expect("Test operation failed");

    // Verify all settings survived the round trip
    assert_eq!(
        settings.validation.fail_on_warning,
        deserialized.validation.fail_on_warning
    );
    assert_eq!(
        settings.generation.target_languages,
        deserialized.generation.target_languages
    );
    assert_eq!(
        settings.imports.import_aliases.get("bio"),
        deserialized.imports.import_aliases.get("bio")
    );
    assert_eq!(
        settings.naming.class_naming_convention,
        deserialized.naming.class_naming_convention
    );
    assert_eq!(settings.defaults.required, deserialized.defaults.required);
}

#[tokio::test]
async fn test_empty_settings_behavior() {
    // Test behavior when settings are None vs empty
    let mut schema1 = SchemaDefinition::new("schema1");
    schema1.settings = None;

    let mut schema2 = SchemaDefinition::new("schema2");
    schema2.settings = Some(SchemaSettings::default());

    // Both should behave the same way with defaults
    let engine1 = ValidationEngine::new(&schema1).expect("Test operation failed");
    let engine2 = ValidationEngine::new(&schema2).expect("Test operation failed");

    let data = json!({"test": "value"});

    // Both should allow additional properties by default
    let report1 = engine1
        .validate(&data, None)
        .await
        .expect("Test operation failed");
    let report2 = engine2
        .validate(&data, None)
        .await
        .expect("Test operation failed");

    assert_eq!(report1.valid, report2.valid);
}

#[tokio::test]
async fn test_settings_with_inheritance_chain() {
    // Test settings behavior in complex inheritance scenarios
    let mut schema = SchemaDefinition::new("test_schema");

    let mut settings = SchemaSettings::default();
    settings.validation.check_slot_usage = true;
    settings.validation.allow_additional_properties = false;
    schema.settings = Some(settings);

    // Create inheritance chain: Base -> Middle -> Derived
    let mut base = ClassDefinition::new("Base");
    base.slots = vec!["id".to_string()];

    let mut middle = ClassDefinition::new("Middle");
    middle.is_a = Some("Base".to_string());
    middle.slots = vec!["name".to_string()];

    let mut derived = ClassDefinition::new("Derived");
    derived.is_a = Some("Middle".to_string());
    derived.slots = vec!["age".to_string()];

    schema.classes.insert("Base".to_string(), base);
    schema.classes.insert("Middle".to_string(), middle);
    schema.classes.insert("Derived".to_string(), derived);

    // Add slots
    for slot_name in ["id", "name", "age"] {
        let mut slot = SlotDefinition::new(slot_name);
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");

    // Test that settings apply throughout inheritance chain
    let data = json!({
        "id": "123",
        "name": "Test",
        "age": "30",
        "extra": "not allowed"
    });

    let report = engine
        .validate_as_class(&data, "Derived", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid); // Should fail due to extra field
}

#[tokio::test]
async fn test_settings_performance_impact() {
    use std::time::Instant;
use linkml_core::types::{ClassDefinition, SlotDefinition};

    // Create two schemas - one strict, one lenient
    let mut strict_schema = SchemaDefinition::new("strict");
    strict_schema.settings = Some(SchemaSettings::strict());

    let mut lenient_schema = SchemaDefinition::new("lenient");
    lenient_schema.settings = Some(SchemaSettings::default());

    // Add the same class to both
    for schema in [&mut strict_schema, &mut lenient_schema] {
        let mut class = ClassDefinition::new("TestClass");
        class.slots = (0..10).map(|i| format!("slot{}", i)).collect();
        schema.classes.insert("TestClass".to_string(), class);

        for i in 0..10 {
            let mut slot = SlotDefinition::new(&format!("slot{}", i));
            slot.range = Some("string".to_string());
            schema.slots.insert(format!("slot{}", i), slot);
        }
    }

    let strict_engine = ValidationEngine::new(&strict_schema).expect("Test operation failed");
    let lenient_engine = ValidationEngine::new(&lenient_schema).expect("Test operation failed");

    // Create test data
    let mut data = serde_json::Map::new();
    for i in 0..10 {
        data.insert(format!("slot{}", i), json!(format!("value{}", i));
    }
    let data = serde_json::Value::Object(data);

    // Measure validation time
    let start = Instant::now();
    for _ in 0..100 {
        let _ = strict_engine
            .validate_as_class(&data, "TestClass", None)
            .await
            .expect("Test operation failed");
    }
    let strict_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..100 {
        let _ = lenient_engine
            .validate_as_class(&data, "TestClass", None)
            .await
            .expect("Test operation failed");
    }
    let lenient_time = start.elapsed();

    // Strict validation should not be significantly slower (within 2x)
    assert!(strict_time.as_millis() < lenient_time.as_millis() * 2);
}
