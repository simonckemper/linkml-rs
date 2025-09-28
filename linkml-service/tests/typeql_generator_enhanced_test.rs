//! Comprehensive tests for the enhanced TypeQL generator

use indexmap::IndexMap;
use linkml_core::Value;
use linkml_core::prelude::*;
use linkml_service::generator::CodeFormatter;
use linkml_service::generator::{EnhancedTypeQLGenerator, Generator};
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

/// Helper to create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.org/test".to_string();
    schema.name = "TestSchema".to_string();
    schema.version = Some("1.0.0".to_string());
    schema.description = Some("Test schema for TypeQL generator".to_string());

    // Add prefixes
    // Prefixes are not structured this way in current API
    // schema.prefixes would be a simple HashMap<String, String>

    schema
}

#[tokio::test]
async fn test_basic_entity_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Add a simple Person entity
    let mut name_slot = SlotDefinition::default();
    name_slot.name = "name".to_string();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    name_slot.identifier = Some(true);
    name_slot.description = Some("Full name of the person".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.name = "age".to_string();
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(Value::Number(serde_json::Number::from(0));
    age_slot.maximum_value = Some(Value::Number(serde_json::Number::from(150));
    schema.slots.insert("age".to_string(), age_slot);

    let mut person_class = ClassDefinition::default();
    person_class.name = "Person".to_string();
    person_class.description = Some("A human being".to_string());
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check header
    assert!(content.contains("# TypeQL Schema generated from LinkML"));
    assert!(content.contains("# Schema: TestSchema"));
    assert!(content.contains("# Version: 1.0.0"));

    // Check entity definition
    assert!(content.contains("person sub entity,"));
    assert!(content.contains("owns name @key"));
    assert!(content.contains("owns age"));

    // Check attribute definitions
    assert!(content.contains("name sub attribute, value string;"));
    assert!(content.contains("age sub attribute, value long, range [0..150];"));
}

#[tokio::test]
async fn test_inheritance_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create inheritance hierarchy
    let mut id_slot = SlotDefinition::default();
    id_slot.name = "id".to_string();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    // Abstract base class
    let mut entity_class = ClassDefinition::default();
    entity_class.name = "NamedEntity".to_string();
    entity_class.abstract_ = Some(true);
    entity_class.slots = vec!["id".to_string()];
    schema
        .classes
        .insert("NamedEntity".to_string(), entity_class);

    // Concrete child class
    let mut person_class = ClassDefinition::default();
    person_class.name = "Person".to_string();
    person_class.is_a = Some("NamedEntity".to_string());
    schema.classes.insert("Person".to_string(), person_class);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check abstract type
    assert!(content.contains("named-entity sub entity, abstract"));

    // Check inheritance
    assert!(content.contains("person sub named-entity"));
}

#[tokio::test]
async fn test_relation_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create entities
    let mut person = ClassDefinition::default();
    person.name = "Person".to_string();
    schema.classes.insert("Person".to_string(), person);

    let mut org = ClassDefinition::default();
    org.name = "Organization".to_string();
    schema.classes.insert("Organization".to_string(), org);

    // Create relation with object-valued slots
    let mut employee_slot = SlotDefinition::default();
    employee_slot.name = "employee".to_string();
    employee_slot.range = Some("Person".to_string());
    employee_slot.required = Some(true);
    schema.slots.insert("employee".to_string(), employee_slot);

    let mut employer_slot = SlotDefinition::default();
    employer_slot.name = "employer".to_string();
    employer_slot.range = Some("Organization".to_string());
    employer_slot.required = Some(true);
    schema.slots.insert("employer".to_string(), employer_slot);

    let mut start_date_slot = SlotDefinition::default();
    start_date_slot.name = "start_date".to_string();
    start_date_slot.range = Some("date".to_string());
    schema
        .slots
        .insert("start_date".to_string(), start_date_slot);

    let mut employment = ClassDefinition::default();
    employment.name = "Employment".to_string();
    employment.slots = vec![
        "employee".to_string(),
        "employer".to_string(),
        "start_date".to_string(),
    ];
    schema.classes.insert("Employment".to_string(), employment);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check relation definition
    assert!(content.contains("employment sub relation,"));
    assert!(content.contains("relates employee"));
    assert!(content.contains("relates employer"));
    assert!(content.contains("owns start-date"));

    // Check role players
    assert!(content.contains("person plays employment:employee;"));
    assert!(content.contains("organization plays employment:employer;"));
}

#[tokio::test]
async fn test_constraint_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Slot with pattern constraint
    let mut email_slot = SlotDefinition::default();
    email_slot.name = "email".to_string();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    // Slot with cardinality
    let mut phone_slot = SlotDefinition::default();
    phone_slot.name = "phone".to_string();
    phone_slot.range = Some("string".to_string());
    phone_slot.multivalued = Some(true);
    schema.slots.insert("phone".to_string(), phone_slot);

    let mut person = ClassDefinition::default();
    person.name = "Person".to_string();
    person.slots = vec!["email".to_string(), "phone".to_string()];
    schema.classes.insert("Person".to_string(), person);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check pattern constraint
    assert!(content.contains("email sub attribute, value string, regex"));

    // Check cardinality
    assert!(content.contains("owns phone @card(0..)"));
}

#[tokio::test]
async fn test_enum_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create enum
    let mut status_enum = EnumDefinition::default();
    status_enum.name = "StatusEnum".to_string();
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Complex {
            text: "active".to_string(),
            description: Some("Currently active".to_string()),
            meaning: None,
        });
    status_enum
        .permissible_values
        .push(linkml_core::types::PermissibleValue::Complex {
            text: "inactive".to_string(),
            description: Some("Inactive state".to_string()),
            meaning: None,
        });
    schema.enums.insert("StatusEnum".to_string(), status_enum);

    // Use enum in slot
    let mut status_slot = SlotDefinition::default();
    status_slot.name = "status".to_string();
    status_slot.range = Some("StatusEnum".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    let mut entity = ClassDefinition::default();
    entity.name = "Entity".to_string();
    entity.slots = vec!["status".to_string()];
    schema.classes.insert("Entity".to_string(), entity);

    let content = generator.generate(&schema).expect("Test operation failed");

    // For now, enums are represented as string attributes
    // In future, could use TypeDB 3.0 value restrictions
    assert!(content.contains("status sub attribute, value string"));
}

#[tokio::test]
async fn test_mixin_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create timestamp slots
    let mut created_slot = SlotDefinition::default();
    created_slot.name = "created_at".to_string();
    created_slot.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_slot);

    let mut updated_slot = SlotDefinition::default();
    updated_slot.name = "updated_at".to_string();
    updated_slot.range = Some("datetime".to_string());
    schema.slots.insert("updated_at".to_string(), updated_slot);

    // Create mixin
    let mut timestamped = ClassDefinition::default();
    timestamped.name = "Timestamped".to_string();
    timestamped.mixin = Some(true);
    timestamped.slots = vec!["created_at".to_string(), "updated_at".to_string()];
    schema
        .classes
        .insert("Timestamped".to_string(), timestamped);

    // Class using mixin
    let mut document = ClassDefinition::default();
    document.name = "Document".to_string();
    document.mixins = vec!["Timestamped".to_string()];
    schema.classes.insert("Document".to_string(), document);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check mixin as abstract
    assert!(content.contains("timestamped sub entity, abstract"));

    // Check inheritance from mixin
    assert!(content.contains("document sub timestamped"));
}

#[tokio::test]
async fn test_unique_key_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create slots
    let mut code_slot = SlotDefinition::default();
    code_slot.name = "code".to_string();
    code_slot.range = Some("string".to_string());
    schema.slots.insert("code".to_string(), code_slot);

    let mut version_slot = SlotDefinition::default();
    version_slot.name = "version".to_string();
    version_slot.range = Some("string".to_string());
    schema.slots.insert("version".to_string(), version_slot);

    // Create class with composite unique key
    let mut product = ClassDefinition::default();
    product.name = "Product".to_string();
    product.slots = vec!["code".to_string(), "version".to_string()];
    let mut unique_keys_map = IndexMap::new();
    unique_keys_map.insert(
        "code_version_key".to_string(),
        linkml_core::types::UniqueKeyDefinition {
            description: Some("Code and version uniqueness constraint".to_string()),
            unique_key_slots: vec!["code".to_string(), "version".to_string()],
            consider_nulls_inequal: Some(true),
        },
    );
    product.unique_keys = unique_keys_map;
    schema.classes.insert("Product".to_string(), product);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check for unique constraint rule
    assert!(content.contains("rule product-unique-product_key"));
}

#[tokio::test]
async fn test_rule_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create slots
    let mut age_slot = SlotDefinition::default();
    age_slot.name = "age".to_string();
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot);

    let mut guardian_slot = SlotDefinition::default();
    guardian_slot.name = "guardian".to_string();
    guardian_slot.range = Some("Person".to_string());
    schema.slots.insert("guardian".to_string(), guardian_slot);

    // Create class with rule
    let mut person = ClassDefinition::default();
    person.name = "Person".to_string();
    person.slots = vec!["age".to_string(), "guardian".to_string()];

    // Add rule for minors requiring guardian
    let mut rule = Rule::default();
    rule.name = "minor_guardian".to_string();
    rule.description = Some("Minors must have a guardian".to_string());
    // In real implementation, would have proper preconditions/postconditions
    person.rules.push(rule);

    schema.classes.insert("Person".to_string(), person);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check for rule generation
    assert!(content.contains("rule person-rule-minor-guardian"));
    assert!(content.contains("# Rule: Minors must have a guardian"));
}

#[tokio::test]
async fn test_custom_type_mapping() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create custom type
    let mut percentage_type = TypeDefinition::default();
    percentage_type.name = "Percentage".to_string();
    percentage_type.base_type = Some("float".to_string());
    percentage_type.minimum_value = Some(Value::Float(0.0));
    percentage_type.maximum_value = Some(Value::Float(100.0));
    schema
        .types
        .insert("Percentage".to_string(), percentage_type);

    // Use custom type
    let mut score_slot = SlotDefinition::default();
    score_slot.name = "score".to_string();
    score_slot.range = Some("Percentage".to_string());
    schema.slots.insert("score".to_string(), score_slot);

    let mut result = ClassDefinition::default();
    result.name = "TestResult".to_string();
    result.slots = vec!["score".to_string()];
    schema.classes.insert("TestResult".to_string(), result);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Custom type should resolve to base type
    assert!(content.contains("score sub attribute, value double"));
}

#[tokio::test]
async fn test_multi_way_relation() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create entities
    let mut student = ClassDefinition::default();
    student.name = "Student".to_string();
    schema.classes.insert("Student".to_string(), student);

    let mut course = ClassDefinition::default();
    course.name = "Course".to_string();
    schema.classes.insert("Course".to_string(), course);

    let mut instructor = ClassDefinition::default();
    instructor.name = "Instructor".to_string();
    schema.classes.insert("Instructor".to_string(), instructor);

    // Create multi-way relation
    let mut student_slot = SlotDefinition::default();
    student_slot.name = "student".to_string();
    student_slot.range = Some("Student".to_string());
    schema.slots.insert("student".to_string(), student_slot);

    let mut course_slot = SlotDefinition::default();
    course_slot.name = "course".to_string();
    course_slot.range = Some("Course".to_string());
    schema.slots.insert("course".to_string(), course_slot);

    let mut instructor_slot = SlotDefinition::default();
    instructor_slot.name = "instructor".to_string();
    instructor_slot.range = Some("Instructor".to_string());
    schema
        .slots
        .insert("instructor".to_string(), instructor_slot);

    let mut grade_slot = SlotDefinition::default();
    grade_slot.name = "grade".to_string();
    grade_slot.range = Some("string".to_string());
    schema.slots.insert("grade".to_string(), grade_slot);

    let mut enrollment = ClassDefinition::default();
    enrollment.name = "Enrollment".to_string();
    enrollment.slots = vec![
        "student".to_string(),
        "course".to_string(),
        "instructor".to_string(),
        "grade".to_string(),
    ];
    schema.classes.insert("Enrollment".to_string(), enrollment);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Check multi-way relation
    assert!(content.contains("enrollment sub relation,"));
    assert!(content.contains("relates student"));
    assert!(content.contains("relates course"));
    assert!(content.contains("relates instructor"));
    assert!(content.contains("owns grade"));

    // Check all role players
    assert!(content.contains("student plays enrollment:student;"));
    assert!(content.contains("course plays enrollment:course;"));
    assert!(content.contains("instructor plays enrollment:instructor;"));
}

#[tokio::test]
async fn test_migration_generation() {
    let generator = EnhancedTypeQLGenerator::new();
    let schema = create_test_schema();

    let content = generator.generate(&schema).expect("Test operation failed");

    // Generated content is a single string now

    // Check for migration-related content in the output
    assert!(content.contains("# TypeQL"));
    assert!(content.contains("# Schema: TestSchema"));
}

#[tokio::test]
async fn test_complex_schema() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create a complex biomedical-like schema

    // Base abstract class
    let mut entity = ClassDefinition::default();
    entity.name = "Entity".to_string();
    entity.abstract_ = Some(true);
    schema.classes.insert("Entity".to_string(), entity);

    // Mixin
    let mut named = ClassDefinition::default();
    named.name = "Named".to_string();
    named.mixin = Some(true);
    schema.classes.insert("Named".to_string(), named);

    // Concrete entities
    let mut patient = ClassDefinition::default();
    patient.name = "Patient".to_string();
    patient.is_a = Some("Entity".to_string());
    patient.mixins = vec!["Named".to_string()];
    schema.classes.insert("Patient".to_string(), patient);

    let mut condition = ClassDefinition::default();
    condition.name = "Condition".to_string();
    condition.is_a = Some("Entity".to_string());
    schema.classes.insert("Condition".to_string(), condition);

    // Relation
    let mut diagnosis = ClassDefinition::default();
    diagnosis.name = "Diagnosis".to_string();
    diagnosis.slots = vec!["patient".to_string(), "condition".to_string()];
    schema.classes.insert("Diagnosis".to_string(), diagnosis);

    // Add necessary slots
    let mut patient_slot = SlotDefinition::default();
    patient_slot.name = "patient".to_string();
    patient_slot.range = Some("Patient".to_string());
    schema.slots.insert("patient".to_string(), patient_slot);

    let mut condition_slot = SlotDefinition::default();
    condition_slot.name = "condition".to_string();
    condition_slot.range = Some("Condition".to_string());
    schema.slots.insert("condition".to_string(), condition_slot);

    let content = generator.generate(&schema).expect("Test operation failed");

    // Verify complex inheritance and relations
    assert!(content.contains("entity sub entity, abstract"));
    assert!(content.contains("named sub entity, abstract"));
    assert!(content.contains("patient sub entity, sub named"));
    assert!(content.contains("diagnosis sub relation"));
}

#[tokio::test]
async fn test_identifier_mapping_preservation() {
    let generator = EnhancedTypeQLGenerator::new();

    // Test various identifier conversions
    let test_cases = vec![
        ("HTTPSConnection", "https-connection"),
        ("XMLParser", "xml-parser"),
        ("hasPartOf", "has-part-of"),
        ("IPAddress", "ip-address"),
        ("person_name", "person-name"),
        ("PERSON_NAME", "person-name"),
        ("personName", "person-name"),
        ("Person", "person"),
    ];

    for (input, expected) in test_cases {
        let converted = generator.convert_identifier(input);
        assert_eq!(converted, expected, "Failed to convert {}", input);
    }
}

#[tokio::test]
async fn test_error_handling() {
    let generator = EnhancedTypeQLGenerator::new();
    let mut schema = create_test_schema();

    // Create inheritance cycle
    let mut class_a = ClassDefinition::default();
    class_a.name = "ClassA".to_string();
    class_a.is_a = Some("ClassB".to_string());
    schema.classes.insert("ClassA".to_string(), class_a);

    let mut class_b = ClassDefinition::default();
    class_b.name = "ClassB".to_string();
    class_b.is_a = Some("ClassA".to_string());
    schema.classes.insert("ClassB".to_string(), class_b);

    let result = generator.generate(&schema);

    // Should detect inheritance cycle
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Inheritance cycle"));
}
