//! Simple performance test for TypeQL generation

use linkml_core::prelude::*;
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::generator::{Generator, typeql_generator_enhanced::EnhancedTypeQLGenerator};
use std::time::Instant;

#[tokio::test]
async fn test_typeql_performance() {
    // Create a simple schema
    let mut schema = SchemaDefinition::default();
    schema.name = "PerfTest".to_string();

    // Add 100 classes
    for i in 0..100 {
        let mut class = ClassDefinition::default();
        class.description = Some(format!("Test class {}", i));
        class.slots.push("id".to_string());
        class.slots.push("name".to_string());

        if i > 0 && i % 10 == 0 {
            class.is_a = Some(format!("Class{}", i - 1));
        }

        schema.classes.insert(format!("Class{}", i), class);
    }

    // Add slots
    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    // Generate TypeQL
    let generator = EnhancedTypeQLGenerator::new();

    let start = Instant::now();
    let result = generator.generate(&schema);
    let duration = start.elapsed();

    // Verify success
    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Print timing
    println!("Generated 100 classes in {:?}", duration);
    println!("Target: <100ms, Actual: {}ms", duration.as_millis());

    // Check performance target
    assert!(
        duration.as_millis() < 100,
        "Failed 100ms target for 100 classes: took {:?}",
        duration
    );

    let outputs = result.expect("Test operation failed");
    let total_size: usize = outputs.len();
    println!("Output size: {} bytes", total_size);
}
