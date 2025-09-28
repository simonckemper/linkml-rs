//! Comprehensive tests for schema merging functionality

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, Definition, SlotDefinition,
    TypeDefinition,
};
use linkml_service::transform::SchemaMerger;
use linkml_core::types::SchemaDefinition;
use std::collections::HashMap;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

#[test]
fn test_basic_schema_merging() {
    let mut schema1 = SchemaDefinition::new("schema1");
    schema1.id = "https://example.org/schema1".to_string();

    let mut class1 = ClassDefinition::new("Person");
    class1.slots = vec!["name".to_string(), "age".to_string()];
    schema1.classes.insert("Person".to_string(), class1);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema1.slots.insert("name".to_string(), name_slot);

    let mut schema2 = SchemaDefinition::new("schema2");
    schema2.id = "https://example.org/schema2".to_string();

    let mut class2 = ClassDefinition::new("Employee");
    class2.is_a = Some("Person".to_string());
    class2.slots = vec!["employee_id".to_string()];
    schema2.classes.insert("Employee".to_string(), class2);

    let mut id_slot = SlotDefinition::new("employee_id");
    id_slot.range = Some("string".to_string());
    schema2.slots.insert("employee_id".to_string(), id_slot);

    // Merge schemas
    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Verify merge
    assert_eq!(merged.name, "schema1"); // First schema name is used
    assert_eq!(merged.classes.len(), 2);
    assert!(merged.classes.contains_key("Person"));
    assert!(merged.classes.contains_key("Employee"));
    assert_eq!(merged.slots.len(), 2);
}

#[test]
fn test_conflicting_class_definitions() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut person1 = ClassDefinition::new("Person");
    person1.description = Some("Person from schema1".to_string());
    person1.slots = vec!["name".to_string(), "age".to_string()];
    schema1.classes.insert("Person".to_string(), person1);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut person2 = ClassDefinition::new("Person");
    person2.description = Some("Person from schema2".to_string());
    person2.slots = vec!["full_name".to_string(), "birth_date".to_string()];
    person2.aliases = vec!["Individual".to_string()];
    schema2.classes.insert("Person".to_string(), person2);

    // Test different merge strategies
    let merger = SchemaMerger::new();

    // Default strategy (last wins)
    let merged = merger
        .merge(vec![schema1.clone(), schema2.clone()])
        .expect("Test operation failed");
    let person = merged.classes.get("Person").expect("Test operation failed");
    assert_eq!(person.description.as_deref(), Some("Person from schema2"));
    assert!(person.slots.contains(&"full_name".to_string());

    // Union strategy for slots
    let merger_union = SchemaMerger::new().with_merge_slots(true);
    let merged_union = merger_union
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");
    let person_union = merged_union
        .classes
        .get("Person")
        .expect("Test operation failed");
    assert_eq!(person_union.slots.len(), 4); // Combined slots
    assert!(person_union.slots.contains(&"name".to_string()));
    assert!(person_union.slots.contains(&"full_name".to_string()));
}

#[test]
fn test_slot_merging_with_conflicts() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut name_slot1 = SlotDefinition::new("name");
    name_slot1.range = Some("string".to_string());
    name_slot1.required = Some(true);
    name_slot1.description = Some("Full name".to_string());
    schema1.slots.insert("name".to_string(), name_slot1);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut name_slot2 = SlotDefinition::new("name");
    name_slot2.range = Some("PersonName".to_string()); // Different range
    name_slot2.required = Some(false); // Different required
    name_slot2.multivalued = Some(true);
    name_slot2.description = Some("Person's name".to_string());
    schema2.slots.insert("name".to_string(), name_slot2);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    let name_slot = merged.slots.get("name").expect("Test operation failed");
    // Last schema wins
    assert_eq!(name_slot.range.as_deref(), Some("PersonName"));
    assert_eq!(name_slot.required, Some(false));
    assert_eq!(name_slot.multivalued, Some(true));
}

#[test]
fn test_type_and_enum_merging() {
    let mut schema1 = SchemaDefinition::new("schema1");

    // Add custom type
    let mut phone_type = TypeDefinition::default();
    type_def.name = "PhoneNumber".to_string();
    phone_type.typeof_value = Some("string".to_string());
    phone_type.pattern = Some(r"\d{3}-\d{3}-\d{4}".to_string());
    schema1.types.insert("PhoneNumber".to_string(), phone_type);

    // Add enum
    let mut status_enum = EnumDefinition::default();
    enum_def.name = "Status".to_string();
    status_enum.permissible_values = vec![
        PermissibleValue { text: "active"),
        PermissibleValue { text: "inactive"),
    ];
    schema1.enums.insert("Status".to_string(), status_enum);

    let mut schema2 = SchemaDefinition::new("schema2");

    // Add overlapping enum with additional values
    let mut status_enum2 = EnumDefinition::default();
    enum_def.name = "Status".to_string();
    status_enum2.permissible_values = vec![
        PermissibleValue { text: "active"),
        PermissibleValue { text: "pending"),
        PermissibleValue { text: "suspended"),
    ];
    schema2.enums.insert("Status".to_string(), status_enum2);

    // Add another type
    let mut email_type = TypeDefinition::default();
    type_def.name = "Email".to_string();
    email_type.typeof_value = Some("string".to_string());
    email_type.pattern = Some(r"[^@]+@[^@]+".to_string());
    schema2.types.insert("Email".to_string(), email_type);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Check types were merged
    assert_eq!(merged.types.len(), 2);
    assert!(merged.types.contains_key("PhoneNumber"));
    assert!(merged.types.contains_key("Email"));

    // Check enum (last wins)
    let status = merged.enums.get("Status").expect("Test operation failed");
    assert_eq!(status.permissible_values.len(), 3);
    assert!(
        status
            .permissible_values
            .iter()
            .any(|v| v.text == "pending")
    );
}

#[test]
fn test_prefix_merging() {
    let mut schema1 = SchemaDefinition::new("schema1");
    schema1
        .prefixes
        .insert("ex".to_string(), "https://example.org/".to_string());
    schema1
        .prefixes
        .insert("schema".to_string(), "https://schema.org/".to_string());

    let mut schema2 = SchemaDefinition::new("schema2");
    schema2
        .prefixes
        .insert("ex".to_string(), "https://example.com/".to_string()); // Conflict
    schema2
        .prefixes
        .insert("bio".to_string(), "https://bio.org/".to_string());

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Check prefixes
    assert_eq!(merged.prefixes.len(), 3);
    assert_eq!(
        merged.prefixes.get("ex"),
        Some(&"https://example.com/".to_string())
    ); // Last wins
    assert_eq!(
        merged.prefixes.get("schema"),
        Some(&"https://schema.org/".to_string())
    );
    assert_eq!(
        merged.prefixes.get("bio"),
        Some(&"https://bio.org/".to_string())
    );
}

#[test]
fn test_subset_merging() {
    let mut schema1 = SchemaDefinition::new("schema1");
    schema1.subsets.insert(
        "clinical".to_string(),
        vec!["Patient".to_string(), "Diagnosis".to_string()],
    );
    schema1
        .subsets
        .insert("research".to_string(), vec!["Study".to_string()]);

    let mut schema2 = SchemaDefinition::new("schema2");
    schema2
        .subsets
        .insert("clinical".to_string(), vec!["Treatment".to_string()]);
    schema2
        .subsets
        .insert("administrative".to_string(), vec!["Billing".to_string()]);

    let merger = SchemaMerger::new().with_merge_subsets(true);
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Check subsets were merged
    assert_eq!(merged.subsets.len(), 3);

    let clinical = merged
        .subsets
        .get("clinical")
        .expect("Test operation failed");
    assert_eq!(clinical.len(), 3); // Combined values
    assert!(clinical.contains(&"Patient".to_string());
    assert!(clinical.contains(&"Treatment".to_string());
}

#[test]
fn test_inheritance_preservation() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut base = ClassDefinition::new("Base");
    base.slots = vec!["id".to_string()];
    schema1.classes.insert("Base".to_string(), base);

    let mut derived1 = ClassDefinition::new("Derived1");
    derived1.is_a = Some("Base".to_string());
    derived1.slots = vec!["field1".to_string()];
    schema1.classes.insert("Derived1".to_string(), derived1);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut derived2 = ClassDefinition::new("Derived2");
    derived2.is_a = Some("Base".to_string());
    derived2.slots = vec!["field2".to_string()];
    schema2.classes.insert("Derived2".to_string(), derived2);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Check inheritance is preserved
    assert_eq!(merged.classes.len(), 3);
    assert_eq!(
        merged
            .classes
            .get("Derived1")
            .expect("Test operation failed")
            .is_a
            .as_deref(),
        Some("Base")
    );
    assert_eq!(
        merged
            .classes
            .get("Derived2")
            .expect("Test operation failed")
            .is_a
            .as_deref(),
        Some("Base")
    );
}

#[test]
fn test_mixin_merging() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut timestamped = ClassDefinition::new("Timestamped");
    timestamped.mixin = Some(true);
    timestamped.slots = vec!["created_at".to_string(), "updated_at".to_string()];
    schema1
        .classes
        .insert("Timestamped".to_string(), timestamped);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut auditable = ClassDefinition::new("Auditable");
    auditable.mixin = Some(true);
    auditable.slots = vec!["created_by".to_string(), "updated_by".to_string()];
    schema2.classes.insert("Auditable".to_string(), auditable);

    let mut document = ClassDefinition::new("Document");
    document.mixins = vec!["Timestamped".to_string(), "Auditable".to_string()];
    schema2.classes.insert("Document".to_string(), document);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Check all mixins are present
    assert!(
        merged
            .classes
            .get("Timestamped")
            .expect("Test operation failed")
            .mixin
            .unwrap_or(false)
    );
    assert!(
        merged
            .classes
            .get("Auditable")
            .expect("Test operation failed")
            .mixin
            .unwrap_or(false)
    );

    let doc = merged
        .classes
        .get("Document")
        .expect("Test operation failed");
    assert_eq!(doc.mixins.len(), 2);
}

#[test]
fn test_circular_reference_handling() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut person = ClassDefinition::new("Person");
    person.slots = vec!["name".to_string(), "spouse".to_string()];
    schema1.classes.insert("Person".to_string(), person);

    let mut spouse_slot = SlotDefinition::new("spouse");
    spouse_slot.range = Some("Person".to_string()); // Self-reference
    schema1.slots.insert("spouse".to_string(), spouse_slot);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut organization = ClassDefinition::new("Organization");
    organization.slots = vec!["name".to_string(), "parent".to_string()];
    schema2
        .classes
        .insert("Organization".to_string(), organization);

    let mut parent_slot = SlotDefinition::new("parent");
    parent_slot.range = Some("Organization".to_string()); // Self-reference
    schema2.slots.insert("parent".to_string(), parent_slot);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Verify circular references are preserved
    assert_eq!(
        merged
            .slots
            .get("spouse")
            .expect("Test operation failed")
            .range
            .as_deref(),
        Some("Person")
    );
    assert_eq!(
        merged
            .slots
            .get("parent")
            .expect("Test operation failed")
            .range
            .as_deref(),
        Some("Organization")
    );
}

#[test]
fn test_empty_schema_merging() {
    let schema1 = SchemaDefinition::new("schema1");
    let mut schema2 = SchemaDefinition::new("schema2");

    let mut class = ClassDefinition::new("TestClass");
    schema2.classes.insert("TestClass".to_string(), class);

    let merger = SchemaMerger::new();

    // Merge empty with non-empty
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");
    assert_eq!(merged.classes.len(), 1);

    // Merge two empty schemas
    let empty1 = SchemaDefinition::new("empty1");
    let empty2 = SchemaDefinition::new("empty2");
    let merged_empty = merger
        .merge(vec![empty1, empty2])
        .expect("Test operation failed");
    assert_eq!(merged_empty.classes.len(), 0);
}

#[test]
fn test_complex_multi_schema_merge() {
    // Create a complex scenario with 3+ schemas
    let mut base_schema = SchemaDefinition::new("base");
    base_schema.id = "https://example.org/base".to_string();
    base_schema.version = Some("1.0.0".to_string());

    let mut entity = ClassDefinition::new("Entity");
    entity.abstract_class = Some(true);
    entity.slots = vec!["id".to_string()];
    base_schema.classes.insert("Entity".to_string(), entity);

    let mut clinical_schema = SchemaDefinition::new("clinical");
    clinical_schema.imports = vec!["base".to_string()];

    let mut patient = ClassDefinition::new("Patient");
    patient.is_a = Some("Entity".to_string());
    patient.slots = vec!["name".to_string(), "dob".to_string()];
    clinical_schema
        .classes
        .insert("Patient".to_string(), patient);

    let mut research_schema = SchemaDefinition::new("research");
    research_schema.imports = vec!["base".to_string(), "clinical".to_string()];

    let mut study_patient = ClassDefinition::new("StudyPatient");
    study_patient.is_a = Some("Patient".to_string());
    study_patient.slots = vec!["study_id".to_string(), "enrollment_date".to_string()];
    research_schema
        .classes
        .insert("StudyPatient".to_string(), study_patient);

    // Merge all three
    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![base_schema, clinical_schema, research_schema])
        .expect("Test operation failed");

    // Verify complex inheritance chain
    assert_eq!(merged.classes.len(), 3);
    assert!(
        merged
            .classes
            .get("Entity")
            .expect("Test operation failed")
            .abstract_class
            .unwrap_or(false)
    );
    assert_eq!(
        merged
            .classes
            .get("Patient")
            .expect("Test operation failed")
            .is_a
            .as_deref(),
        Some("Entity")
    );
    assert_eq!(
        merged
            .classes
            .get("StudyPatient")
            .expect("Test operation failed")
            .is_a
            .as_deref(),
        Some("Patient")
    );
}

#[test]
fn test_merge_with_slot_usage() {
    let mut schema1 = SchemaDefinition::new("schema1");

    let mut base_slot = SlotDefinition::new("name");
    base_slot.range = Some("string".to_string());
    schema1.slots.insert("name".to_string(), base_slot);

    let mut person = ClassDefinition::new("Person");
    person.slots = vec!["name".to_string()];

    // Add slot usage override
    let mut name_usage = SlotDefinition::new("name");
    name_usage.required = Some(true);
    name_usage.pattern = Some("[A-Z][a-z]+".to_string());
    person.slot_usage.insert("name".to_string(), name_usage);

    schema1.classes.insert("Person".to_string(), person);

    let mut schema2 = SchemaDefinition::new("schema2");

    let mut employee = ClassDefinition::new("Employee");
    employee.is_a = Some("Person".to_string());

    // Different slot usage for same slot
    let mut name_usage2 = SlotDefinition::new("name");
    name_usage2.required = Some(true);
    name_usage2.pattern = Some("[A-Z]+ [A-Z]+".to_string()); // All caps
    employee.slot_usage.insert("name".to_string(), name_usage2);

    schema2.classes.insert("Employee".to_string(), employee);

    let merger = SchemaMerger::new();
    let merged = merger
        .merge(vec![schema1, schema2])
        .expect("Test operation failed");

    // Verify slot usage is preserved
    let person = merged.classes.get("Person").expect("Test operation failed");
    assert!(person.slot_usage.contains_key("name"));

    let employee = merged
        .classes
        .get("Employee")
        .expect("Test operation failed");
    assert_eq!(
        employee
            .slot_usage
            .get("name")
            .expect("Test operation failed")
            .pattern
            .as_deref(),
        Some("[A-Z]+ [A-Z]+")
    );
}
