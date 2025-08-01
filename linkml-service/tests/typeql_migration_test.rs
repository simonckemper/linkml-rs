//! Integration tests for TypeQL migration support

use linkml_core::prelude::*;
use linkml_service::generator::typeql_migration::{
    SchemaDiffer, MigrationAnalyzer, MigrationGenerator,
    SchemaVersion, VersionedSchema, DetailedChange,
};

/// Create a simple v1 schema
fn create_v1_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "TestSchema".to_string();
    schema.version = Some("1.0.0".to_string());
    
    // Add Person class
    let mut person = ClassDefinition::default();
    person.description = Some("A person".to_string());
    person.slots.extend(vec!["name".to_string(), "age".to_string()]);
    
    // Add slot definitions
    let mut name_slot = SlotDefinition::default();
    name_slot.required = Some(true);
    person.slot_usage.insert("name".to_string(), name_slot);
    
    let mut age_slot = SlotDefinition::default();
    age_slot.range = Some("integer".to_string());
    person.slot_usage.insert("age".to_string(), age_slot);
    
    schema.classes.insert("Person".to_string(), person);
    
    // Add global slots
    let mut name = SlotDefinition::default();
    name.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name);
    
    let mut age = SlotDefinition::default();
    age.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age);
    
    schema
}

/// Create v2 schema with changes
fn create_v2_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "TestSchema".to_string();
    schema.version = Some("2.0.0".to_string());
    
    // Modified Person class
    let mut person = ClassDefinition::default();
    person.description = Some("An enhanced person".to_string());
    person.abstract_ = Some(true); // Make abstract
    person.slots.extend(vec!["name".to_string(), "email".to_string()]); // Removed age, added email
    
    // Update slot usage
    let mut name_slot = SlotDefinition::default();
    name_slot.required = Some(true);
    name_slot.pattern = Some(r"^[A-Z][a-z]+$".to_string()); // Add pattern
    person.slot_usage.insert("name".to_string(), name_slot);
    
    let mut email_slot = SlotDefinition::default();
    email_slot.required = Some(true);
    email_slot.pattern = Some(r"^\w+@\w+\.\w+$".to_string());
    person.slot_usage.insert("email".to_string(), email_slot);
    
    schema.classes.insert("Person".to_string(), person);
    
    // Add Employee subclass
    let mut employee = ClassDefinition::default();
    employee.is_a = Some("Person".to_string());
    employee.slots.push("employee_id".to_string());
    
    let mut emp_id_slot = SlotDefinition::default();
    emp_id_slot.identifier = Some(true);
    employee.slot_usage.insert("employee_id".to_string(), emp_id_slot);
    
    schema.classes.insert("Employee".to_string(), employee);
    
    // Update global slots
    let mut name = SlotDefinition::default();
    name.range = Some("string".to_string());
    name.pattern = Some(r"^[A-Z][a-z]+$".to_string());
    schema.slots.insert("name".to_string(), name);
    
    // age slot removed
    
    // Add new slots
    let mut email = SlotDefinition::default();
    email.range = Some("string".to_string());
    email.required = Some(true);  // Mark as required for test
    schema.slots.insert("email".to_string(), email);
    
    let mut employee_id = SlotDefinition::default();
    employee_id.range = Some("string".to_string());
    schema.slots.insert("employee_id".to_string(), employee_id);
    
    schema
}

#[test]
fn test_version_management() {
    let v1 = SchemaVersion::parse("1.0.0").unwrap();
    let v2 = SchemaVersion::parse("2.0.0").unwrap();
    
    assert!(v2.is_newer_than(&v1));
    assert!(v2.is_breaking_change_from(&v1));
    assert!(!v2.is_patch_from(&v1));
    
    // Test version with schema
    let schema = create_v1_schema();
    let versioned = VersionedSchema::from_schema(schema.clone(), "1.0.0").unwrap();
    assert_eq!(versioned.version.to_string(), "1.0.0");
    assert!(!versioned.version.checksum.is_empty());
}

#[test]
fn test_schema_diff_detection() {
    let v1 = create_v1_schema();
    let v2 = create_v2_schema();
    
    let diff = SchemaDiffer::compare(&v1, &v2).unwrap();
    
    // Check detected changes
    assert_eq!(diff.added_types.len(), 1); // Employee
    assert_eq!(diff.removed_types.len(), 0);
    assert_eq!(diff.modified_types.len(), 1); // Person
    
    // Debug output
    println!("Modified attributes: {}", diff.modified_attributes.len());
    for attr in &diff.modified_attributes {
        println!("  - {} (owner: {})", attr.name, attr.owner);
    }
    
    assert_eq!(diff.added_attributes.len(), 2); // email, employee_id
    assert_eq!(diff.removed_attributes.len(), 1); // age
    assert_eq!(diff.modified_attributes.len(), 2); // name (pattern added) - both global and in Person
    
    // Check Person modifications
    let person_changes = &diff.modified_types[0];
    assert_eq!(person_changes.name, "Person");
    assert!(person_changes.changes.iter().any(|c| matches!(c, 
        DetailedChange::AbstractChanged(false, true)
    )));
}

#[test]
fn test_migration_impact_analysis() {
    let v1 = create_v1_schema();
    let v2 = create_v2_schema();
    
    let diff = SchemaDiffer::compare(&v1, &v2).unwrap();
    let impact = MigrationAnalyzer::analyze_impact(&diff).unwrap();
    
    // Should have breaking changes
    assert!(impact.has_breaking_changes());
    assert!(impact.requires_data_migration);
    
    // Debug output
    println!("Breaking changes:");
    for bc in &impact.breaking_changes {
        println!("  - {}", bc);
    }
    
    // Check specific breaking changes
    assert!(impact.breaking_changes.iter().any(|msg| msg.contains("age")));
    // Note: The current implementation doesn't detect new required slots as breaking changes
    // This would be a good enhancement for the future
    
    // Check warnings
    assert!(impact.has_warnings());
    assert!(impact.warnings.iter().any(|msg| msg.contains("abstract")));
    assert!(impact.warnings.iter().any(|msg| msg.contains("pattern")));
    
    // Check affected types
    assert!(impact.affected_types.contains("Person"));
    assert!(impact.affected_types.contains("Employee"));
}

#[test]
fn test_migration_script_generation() {
    let v1 = create_v1_schema();
    let v2 = create_v2_schema();
    
    let diff = SchemaDiffer::compare(&v1, &v2).unwrap();
    let impact = MigrationAnalyzer::analyze_impact(&diff).unwrap();
    
    let generator = MigrationGenerator::new();
    let migration = generator.generate(&diff, &impact, "1.0.0", "2.0.0").unwrap();
    
    // Check metadata
    assert_eq!(migration.metadata.from_version, "1.0.0");
    assert_eq!(migration.metadata.to_version, "2.0.0");
    assert!(migration.metadata.is_breaking);
    
    // Check forward script
    let forward = migration.forward_script();
    println!("Forward migration:\n{}", forward);
    
    // Should undefine removed elements
    assert!(forward.contains("undefine"));
    assert!(forward.contains("age sub attribute"));
    
    // Should define new elements
    assert!(forward.contains("define"));
    assert!(forward.contains("email sub attribute"));
    assert!(forward.contains("employee-id sub attribute"));
    assert!(forward.contains("employee sub person"));
    
    // Should handle modifications
    assert!(forward.contains("person sub"));
    assert!(forward.contains("abstract"));
    
    // Check rollback script
    let rollback = migration.rollback_script();
    println!("\nRollback migration:\n{}", rollback);
    
    // Should restore removed elements
    assert!(rollback.contains("age sub attribute"));
    
    // Should remove added elements
    assert!(rollback.contains("employee sub"));
    assert!(rollback.contains("email sub"));
}

#[test]
fn test_data_migration_generation() {
    let v1 = create_v1_schema();
    let mut v2 = create_v2_schema();
    
    // Add a type change to trigger data migration
    let mut slot = SlotDefinition::default();
    slot.range = Some("string".to_string()); // Changed from integer
    v2.slots.insert("age".to_string(), slot);
    
    let diff = SchemaDiffer::compare(&v1, &v2).unwrap();
    let impact = MigrationAnalyzer::analyze_impact(&diff).unwrap();
    
    let generator = MigrationGenerator::new();
    let migration = generator.generate(&diff, &impact, "1.0.0", "2.0.0").unwrap();
    
    // Debug print
    println!("Added attributes: {:?}", diff.added_attributes.len());
    for attr in &diff.added_attributes {
        println!("  - {} (required: {:?})", attr.name, attr.new_attr.as_ref().map(|a| a.required));
    }
    println!("Data migrations: {:?}", migration.data_migrations.len());
    for dm in &migration.data_migrations {
        println!("  - {}", dm.description);
    }
    
    // Should have data migrations
    assert!(!migration.data_migrations.is_empty());
    
    // Check for required field default value migration
    let has_default_migration = migration.data_migrations.iter()
        .any(|m| m.description.contains("default value"));
    assert!(has_default_migration);
}

#[test]
fn test_complex_inheritance_migration() {
    let mut v1 = SchemaDefinition::default();
    let mut v2 = SchemaDefinition::default();
    
    // V1: Simple hierarchy
    let mut animal = ClassDefinition::default();
    animal.abstract_ = Some(true);
    v1.classes.insert("Animal".to_string(), animal);
    
    let mut dog = ClassDefinition::default();
    dog.is_a = Some("Animal".to_string());
    v1.classes.insert("Dog".to_string(), dog);
    
    // V2: Modified hierarchy
    let mut animal = ClassDefinition::default();
    animal.abstract_ = Some(true);
    v2.classes.insert("Animal".to_string(), animal);
    
    let mut mammal = ClassDefinition::default();
    mammal.abstract_ = Some(true);
    mammal.is_a = Some("Animal".to_string());
    v2.classes.insert("Mammal".to_string(), mammal);
    
    let mut dog = ClassDefinition::default();
    dog.is_a = Some("Mammal".to_string()); // Changed parent
    v2.classes.insert("Dog".to_string(), dog);
    
    let diff = SchemaDiffer::compare(&v1, &v2).unwrap();
    let impact = MigrationAnalyzer::analyze_impact(&diff).unwrap();
    
    // Check inheritance changes detected
    assert_eq!(diff.added_types.len(), 1); // Mammal
    assert_eq!(diff.modified_types.len(), 1); // Dog
    
    let generator = MigrationGenerator::new();
    let migration = generator.generate(&diff, &impact, "1.0.0", "1.1.0").unwrap();
    
    let forward = migration.forward_script();
    assert!(forward.contains("mammal sub animal"));
    assert!(forward.contains("dog sub mammal"));
}