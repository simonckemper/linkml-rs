//! Comprehensive test suite for TypeQL generation

use linkml_core::Value;
use linkml_core::prelude::*;
use linkml_service::expression::Expression;
use linkml_service::generator::{
    Generator, GeneratorOptions, typeql_generator_enhanced::EnhancedTypeQLGenerator,
};

/// Create a comprehensive test schema with all features
fn create_comprehensive_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "ComprehensiveTestSchema".to_string();
    schema.id = "https://example.org/schemas/comprehensive".to_string();
    schema.version = Some("1.0.0".to_string());
    schema.description = Some("A comprehensive test schema for TypeQL generation".to_string());

    // Define types
    let mut positive_int = TypeDefinition::default();
    positive_int.base_type = Some("integer".to_string());
    positive_int.minimum_value = Some(Value::Number(serde_json::Number::from(0));
    schema.types.insert("PositiveInt".to_string(), positive_int);

    // Define enums
    let mut status_enum = EnumDefinition::default();
    status_enum.description = Some("Status values".to_string());

    let active = PermissibleValue {
        text: "ACTIVE".to_string(),
        description: Some("Active status".to_string()),
        meaning: None,
    };
    status_enum
        .permissible_values
        .insert("ACTIVE".to_string(), active);

    let inactive = PermissibleValue {
        text: "INACTIVE".to_string(),
        description: Some("Inactive status".to_string()),
        meaning: None,
    };
    status_enum
        .permissible_values
        .insert("INACTIVE".to_string(), inactive);

    schema.enums.insert("StatusEnum".to_string(), status_enum);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    id_slot.required = Some(true);
    id_slot.pattern = Some(r"^[A-Z]{3}-\d{6}$".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    name_slot.pattern = Some(r"^[A-Za-z\s]+$".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^\w+@\w+\.\w+$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.range = Some("PositiveInt".to_string());
    age_slot.minimum_value = Some(Value::Number(serde_json::Number::from(0));
    age_slot.maximum_value = Some(Value::Number(serde_json::Number::from(150));
    schema.slots.insert("age".to_string(), age_slot);

    let mut created_at_slot = SlotDefinition::default();
    created_at_slot.range = Some("datetime".to_string());
    schema
        .slots
        .insert("created_at".to_string(), created_at_slot);

    let mut status_slot = SlotDefinition::default();
    status_slot.range = Some("StatusEnum".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    let mut tags_slot = SlotDefinition::default();
    tags_slot.range = Some("string".to_string());
    tags_slot.multivalued = Some(true);
    schema.slots.insert("tags".to_string(), tags_slot);

    // Define abstract base class
    let mut named_thing = ClassDefinition::default();
    named_thing.abstract_ = Some(true);
    named_thing.description = Some("Abstract base for named entities".to_string());
    named_thing.slots.push("name".to_string());
    schema.classes.insert("NamedThing".to_string(), named_thing);

    // Define Person class with inheritance
    let mut person = ClassDefinition::default();
    person.is_a = Some("NamedThing".to_string());
    person.description = Some("A person entity".to_string());
    person.slots.extend(vec![
        "id".to_string(),
        "email".to_string(),
        "age".to_string(),
        "created_at".to_string(),
    ]);

    // Add unique key
    let mut person_unique = UniqueKeyDefinition::default();
    person_unique.unique_key_slots.push("email".to_string());
    person
        .unique_keys
        .insert("email_key".to_string(), person_unique);

    // Removed conditional requirements as they don't exist in the current API

    // Add rule
    let mut adult_rule = Rule::default();
    adult_rule.description = Some("Validate adult age".to_string());
    person.rules.push(adult_rule);

    schema.classes.insert("Person".to_string(), person);

    // Define Organization class
    let mut org = ClassDefinition::default();
    org.is_a = Some("NamedThing".to_string());
    org.description = Some("An organization entity".to_string());
    org.slots.extend(vec![
        "id".to_string(),
        "status".to_string(),
        "created_at".to_string(),
        "tags".to_string(),
    ]);
    schema.classes.insert("Organization".to_string(), org);

    // Define Employment relation
    let mut employment = ClassDefinition::default();
    employment.description = Some("Employment relationship".to_string());
    employment.slots.extend(vec![
        "employee".to_string(),
        "employer".to_string(),
        "start_date".to_string(),
        "end_date".to_string(),
        "position".to_string(),
    ]);

    // Define relation slots
    let mut employee_slot = SlotDefinition::default();
    employee_slot.range = Some("Person".to_string());
    employee_slot.required = Some(true);
    schema.slots.insert("employee".to_string(), employee_slot);

    let mut employer_slot = SlotDefinition::default();
    employer_slot.range = Some("Organization".to_string());
    employer_slot.required = Some(true);
    schema.slots.insert("employer".to_string(), employer_slot);

    let mut start_date = SlotDefinition::default();
    start_date.range = Some("date".to_string());
    start_date.required = Some(true);
    schema.slots.insert("start_date".to_string(), start_date);

    let mut end_date = SlotDefinition::default();
    end_date.range = Some("date".to_string());
    schema.slots.insert("end_date".to_string(), end_date);

    let mut position = SlotDefinition::default();
    position.range = Some("string".to_string());
    schema.slots.insert("position".to_string(), position);

    schema.classes.insert("Employment".to_string(), employment);

    // Define a complex multi-way relation
    let mut meeting = ClassDefinition::default();
    meeting.description = Some("A meeting between multiple participants".to_string());
    meeting.slots.extend(vec![
        "organizer".to_string(),
        "participants".to_string(),
        "location".to_string(),
        "scheduled_time".to_string(),
        "duration_minutes".to_string(),
    ]);

    let mut organizer_slot = SlotDefinition::default();
    organizer_slot.range = Some("Person".to_string());
    organizer_slot.required = Some(true);
    schema.slots.insert("organizer".to_string(), organizer_slot);

    let mut participants_slot = SlotDefinition::default();
    participants_slot.range = Some("Person".to_string());
    participants_slot.multivalued = Some(true);
    participants_slot.required = Some(true);
    schema
        .slots
        .insert("participants".to_string(), participants_slot);

    let mut location_slot = SlotDefinition::default();
    location_slot.range = Some("string".to_string());
    schema.slots.insert("location".to_string(), location_slot);

    let mut scheduled_time = SlotDefinition::default();
    scheduled_time.range = Some("datetime".to_string());
    scheduled_time.required = Some(true);
    schema
        .slots
        .insert("scheduled_time".to_string(), scheduled_time);

    let mut duration = SlotDefinition::default();
    duration.range = Some("integer".to_string());
    duration.minimum_value = Some(Value::Number(serde_json::Number::from(15)));
    duration.maximum_value = Some(Value::Number(serde_json::Number::from(480)));
    schema
        .slots
        .insert("duration_minutes".to_string(), duration);

    schema.classes.insert("Meeting".to_string(), meeting);

    schema
}

#[test]
fn test_enhanced_generator_comprehensive() {
    let schema = create_comprehensive_schema();
    let generator = EnhancedTypeQLGenerator::new();
    let options = GeneratorOptions::default();

    let result = generator.generate(&schema);
    assert!(result.is_ok(), "Generation failed: {:?}", result);

    let output = result.expect("Test operation failed");
    let content = &output;

    // Verify header
    assert!(content.contains("# TypeQL Schema generated from LinkML"));
    assert!(content.contains("# Schema: ComprehensiveTestSchema"));
    assert!(content.contains("# Version: 1.0.0"));

    // Verify attributes
    assert!(content.contains("id sub attribute"));
    assert!(content.contains("name sub attribute"));
    assert!(content.contains("email sub attribute"));
    assert!(content.contains("age sub attribute"));
    assert!(content.contains("regex \"^[A-Z]{3}-\\\\d{6}$\""));

    // Verify entities
    assert!(content.contains("named-thing sub entity, abstract"));
    assert!(content.contains("person sub named-thing"));
    assert!(content.contains("organization sub named-thing"));
    assert!(content.contains("owns id @key"));

    // Verify relations
    assert!(content.contains("employment sub relation"));
    assert!(content.contains("relates employee"));
    assert!(content.contains("relates employer"));
    assert!(content.contains("person plays employment:employee"));
    assert!(content.contains("organization plays employment:employer"));

    // Verify multi-way relation
    assert!(content.contains("meeting sub relation"));
    assert!(content.contains("relates organizer"));
    assert!(content.contains("relates participants"));

    // Verify rules
    assert!(content.contains("rule person-adult-validation"));
    assert!(content.contains("$age >= 18"));

    println!("Enhanced TypeQL Generator Output:
{}", content);
}

#[test]
fn test_edge_cases() {
    // Test empty schema
    let empty_schema = SchemaDefinition::default();
    let generator = EnhancedTypeQLGenerator::new();
    let result = generator.generate(&empty_schema);
    assert!(result.is_ok());

    // Test schema with only attributes
    let mut attr_only = SchemaDefinition::default();
    attr_only.name = "AttributeOnly".to_string();
    let mut slot = SlotDefinition::default();
    slot.range = Some("string".to_string());
    attr_only.slots.insert("test_attr".to_string(), slot);

    let result = generator.generate(&attr_only);
    assert!(result.is_ok());
    assert!(
        result
            .expect("Test operation failed")
            .contains("test-attr sub attribute")
    );

    // Test deeply nested inheritance
    let mut nested = SchemaDefinition::default();
    nested.name = "NestedInheritance".to_string();

    for i in 0..5 {
        let mut class = ClassDefinition::default();
        if i > 0 {
            class.is_a = Some(format!("Level{}", i - 1));
        }
        nested.classes.insert(format!("Level{}", i), class);
    }

    let result = generator.generate(&nested);
    assert!(result.is_ok());
    let content = result.expect("Test operation failed");
    assert!(content.contains("level-0 sub entity"));
    assert!(content.contains("level-4 sub level-3"));
}

#[test]
fn test_complex_constraints() {
    let mut schema = SchemaDefinition::default();
    schema.name = "ConstraintTest".to_string();

    // Add slot with all constraint types
    let mut complex_slot = SlotDefinition::default();
    complex_slot.range = Some("string".to_string());
    complex_slot.pattern = Some(r"^[A-Z][a-z]+$".to_string());
    // minimum_length and maximum_length not available in current API
    complex_slot.required = Some(true);
    complex_slot.multivalued = Some(true);

    let mut usage = SlotDefinition::default();
    usage.identifier = Some(true);

    let mut test_class = ClassDefinition::default();
    test_class.slots.push("complex_field".to_string());
    test_class
        .slot_usage
        .insert("complex_field".to_string(), usage);

    schema
        .slots
        .insert("complex_field".to_string(), complex_slot);
    schema.classes.insert("TestClass".to_string(), test_class);

    let generator = EnhancedTypeQLGenerator::new();
    let result = generator.generate(&schema);
    assert!(result.is_ok());

    let content = result.expect("Test operation failed");
    assert!(content.contains("complex-field sub attribute"));
    assert!(content.contains("owns complex-field @key"));
    assert!(content.contains("regex"));
}

#[test]
fn test_performance_scaling() {
    use std::time::Instant;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

    let sizes = vec![10, 50, 100, 500];
    let generator = EnhancedTypeQLGenerator::new();

    for size in sizes {
        let mut schema = SchemaDefinition::default();
        schema.name = format!("PerfTest{}", size);

        // Create many classes
        for i in 0..size {
            let mut class = ClassDefinition::default();
            class.slots.push("id".to_string());
            class.slots.push("name".to_string());
            schema.classes.insert(format!("Class{}", i), class);
        }

        // Create slots
        let mut id_slot = SlotDefinition::default();
        id_slot.range = Some("string".to_string());
        schema.slots.insert("id".to_string(), id_slot);

        let mut name_slot = SlotDefinition::default();
        name_slot.range = Some("string".to_string());
        schema.slots.insert("name".to_string(), name_slot);

        let start = Instant::now();
        let result = generator.generate(&schema);
        let duration = start.elapsed();

        assert!(result.is_ok());
        println!("Generated {} classes in {:?}", size, duration);

        // Verify performance targets
        if size == 100 {
            assert!(
                duration.as_millis() < 100,
                "Failed 100ms target for 100 classes"
            );
        }
    }
}
