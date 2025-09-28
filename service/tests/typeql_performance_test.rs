//! Performance tests for TypeQL generation

use linkml_core::prelude::*;
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::generator::{Generator, typeql_generator_enhanced::EnhancedTypeQLGenerator};
use std::time::Instant;
use tokio;

/// Create a schema with the specified number of classes
fn create_test_schema(num_classes: usize) -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = format!("PerfTestSchema{}", num_classes);

    // Add common slots
    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut created_at = SlotDefinition::default();
    created_at.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_at);

    // Create classes
    for i in 0..num_classes {
        let mut class = ClassDefinition::default();
        class.description = Some(format!("Test class {}", i));
        class.slots.extend(vec![
            "id".to_string(),
            "name".to_string(),
            "created_at".to_string(),
        ]);

        // Add inheritance for some classes
        if i > 0 && i % 3 == 0 {
            class.is_a = Some(format!("Class{}", i - 1));
        }

        // Add rules for some classes
        if i % 5 == 0 {
            let mut rule = Rule::default();
            rule.title = Some(format!("validation_rule_{}", i));
            rule.description = Some("Validate name is not empty".to_string());
            class.rules.push(rule);
        }

        schema.classes.insert(format!("Class{}", i), class);
    }

    schema
}

#[tokio::test]
async fn test_small_schema_performance() {
    let schema = create_test_schema(10);
    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());
    println!("Generated 10 classes in {:?}", duration);
    assert!(duration.as_millis() < 50, "Small schema took too long");
}

#[tokio::test]
async fn test_medium_schema_performance() {
    let schema = create_test_schema(100);
    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());
    println!("Generated 100 classes in {:?}", duration);

    // Target: <100ms for 100 classes
    assert!(
        duration.as_millis() < 100,
        "Failed 100ms target for 100 classes: took {:?}",
        duration
    );
}

#[tokio::test]
async fn test_large_schema_performance() {
    let schema = create_test_schema(1000);
    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());
    println!("Generated 1000 classes in {:?}", duration);

    // Target: <1s for 1000 classes
    assert!(
        duration.as_secs() < 1,
        "Failed 1s target for 1000 classes: took {:?}",
        duration
    );
}

#[tokio::test]
async fn test_rule_generation_performance() {
    let mut schema = SchemaDefinition::default();
    schema.name = "RuleHeavySchema".to_string();

    // Create 50 classes with 2 rules each = 100 rules
    for i in 0..50 {
        let mut class = ClassDefinition::default();
        class.slots.push("value".to_string());

        // Add validation rule
        let mut val_rule = Rule::default();
        val_rule.title = Some(format!("validation_{}", i));
        val_rule.description = Some("Validate value is positive".to_string());
        class.rules.push(val_rule);

        // Add inference rule
        let mut inf_rule = Rule::default();
        inf_rule.title = Some(format!("inference_{}", i));
        inf_rule.description = Some("Infer derived attributes".to_string());
        class.rules.push(inf_rule);

        schema.classes.insert(format!("Class{}", i), class);
    }

    let mut value_slot = SlotDefinition::default();
    value_slot.range = Some("integer".to_string());
    schema.slots.insert("value".to_string(), value_slot);

    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());
    println!("Generated 100 rules in {:?}", duration);

    // Target: <10ms per rule = <1s for 100 rules
    assert!(
        duration.as_millis() < 1000,
        "Rule generation too slow: {:?} for 100 rules",
        duration
    );
}

#[tokio::test]
async fn test_complex_inheritance_performance() {
    let mut schema = SchemaDefinition::default();
    schema.name = "InheritanceSchema".to_string();

    // Create deep inheritance chain
    for i in 0..100 {
        let mut class = ClassDefinition::default();
        if i > 0 {
            class.is_a = Some(format!("Level{}", i - 1));
        }
        class.slots.push("id".to_string());
        schema.classes.insert(format!("Level{}", i), class);
    }

    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());
    println!("Generated 100-level inheritance in {:?}", duration);

    assert!(
        duration.as_millis() < 200,
        "Deep inheritance generation too slow: {:?}",
        duration
    );
}

#[tokio::test]
async fn test_memory_efficiency() {
    // Generate a very large schema and ensure memory usage is reasonable
    let schema = create_test_schema(5000);
    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    let outputs = result.expect("Test operation failed");
    let total_size: usize = outputs.iter().map(|o| o.len()).sum();

    println!("Generated 5000 classes in {:?}", duration);
    println!(
        "Total output size: {} bytes ({:.2} MB)",
        total_size,
        total_size as f64 / 1_048_576.0
    );

    // Ensure reasonable memory usage (rough estimate)
    let bytes_per_class = total_size / 5000;
    assert!(
        bytes_per_class < 1000,
        "Too much memory per class: {} bytes",
        bytes_per_class
    );
}
