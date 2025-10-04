//! Performance check for TypeQL generation

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, GeneratorOptions,
    typeql_generator_enhanced::{create_enhanced_typeql_generator},
};
use std::time::Instant;

#[tokio::main]
async fn main() {
    println!("TypeQL Generator Performance Check");
    println!("==================================");

    // Test different sizes
    let sizes = vec![10, 50, 100, 500, 1000];

    for size in sizes {
        let mut schema = SchemaDefinition::default();
        schema.name = format!("PerfTest{}", size);

        // Add classes
        for i in 0..size {
            let mut class = ClassDefinition::default();
            class.description = Some(format!("Test class {}", i));
            class.slots.push("id".to_string());
            class.slots.push("name".to_string());

            // Add inheritance
            if i > 0 && i % 10 == 0 {
                class.is_a = Some(format!("Class{}", i - 1));
            }

            // Add rules for some classes
            if i % 5 == 0 {
                let mut rule = Rule::default();
                rule.title = Some(format!("validation_rule_{}", i));
                rule.description = Some("Validate fields".to_string());
                class.rules.push(rule);
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
        name_slot.required = Some(true);
        schema.slots.insert("name".to_string(), name_slot);

        // Generate TypeQL
        let generator = create_enhanced_typeql_generator();
        let options = GeneratorOptions::default();

        let start = Instant::now();
        let result = generator.generate(&schema, &options).await;
        let duration = start.elapsed();

        match result {
            Ok(outputs) => {
                let total_size: usize = outputs.iter().map(|o| o.content.len()).sum();
                println!(
                    "{} classes: {:?} ({} bytes output)",
                    size, duration, total_size
                );

                // Check performance targets
                if size == 100 && duration.as_millis() >= 100 {
                    println!("  ⚠️  WARNING: Failed 100ms target for 100 classes!");
                }
                if size == 1000 && duration.as_secs() >= 1 {
                    println!("  ⚠️  WARNING: Failed 1s target for 1000 classes!");
                }
            }
            Err(e) => {
                println!("{} classes: ERROR - {}", size, e);
            }
        }
    }

    println!(
        "
Performance Targets:"
    );
    println!("- 100 classes: <100ms");
    println!("- 1000 classes: <1s");
}
