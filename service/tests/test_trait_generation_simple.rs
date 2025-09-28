//! Simple test for trait generation in RustGenerator

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ClassDefinition, Definition, SlotDefinition};
use linkml_service::generator::{Generator, RustGenerator};
#[test]
fn test_trait_generation_simple() {
    let mut schema = SchemaDefinition::default();
    schema.name = "trait_test_schema".to_string();
    schema.id = "https://example.org/trait_test".to_string();

    // Create abstract base class
    let mut shape = ClassDefinition::default();
    shape.name = "Shape".to_string();
    shape.abstract_ = Some(true);
    shape.slots = vec!["color".to_string()];
    schema.classes.insert("Shape".to_string(), shape);

    // Create concrete subclass
    let mut circle = ClassDefinition::default();
    circle.name = "Circle".to_string();
    circle.is_a = Some("Shape".to_string());
    circle.slots = vec!["radius".to_string()];
    schema.classes.insert("Circle".to_string(), circle);

    // Create another concrete subclass
    let mut square = ClassDefinition::default();
    square.name = "Square".to_string();
    square.is_a = Some("Shape".to_string());
    square.slots = vec!["side".to_string()];
    schema.classes.insert("Square".to_string(), square);

    // Create slots
    let mut color_slot = SlotDefinition::default();
    color_slot.name = "color".to_string();
    color_slot.range = Some("string".to_string());
    schema.slots.insert("color".to_string(), color_slot);

    let mut radius_slot = SlotDefinition::default();
    radius_slot.name = "radius".to_string();
    radius_slot.range = Some("float".to_string());
    schema.slots.insert("radius".to_string(), radius_slot);

    let mut side_slot = SlotDefinition::default();
    side_slot.name = "side".to_string();
    side_slot.range = Some("float".to_string());
    schema.slots.insert("side".to_string(), side_slot);

    // Generate Rust code
    let generator = RustGenerator::new();
    let result = Generator::generate(&generator, &schema);

    let code = match result {
        Ok(code) => code,
        Err(e) => {
            panic!("Generator error: {}", e);
        }
    };

    // Debug output
    if !code.contains("pub trait ShapeTrait") {
        println!(
            "Generated code (first 2000 chars):
{}",
            &code[..code.len().min(2000)]
        );
    }

    // Verify trait generation
    assert!(
        code.contains("pub trait ShapeTrait"),
        "Should generate trait for abstract class"
    );
    assert!(
        code.contains("impl ShapeTrait for Circle"),
        "Should implement trait for Circle"
    );
    assert!(
        code.contains("impl ShapeTrait for Square"),
        "Should implement trait for Square"
    );
    assert!(
        code.contains("fn as_any(&self) -> &dyn std::any::Any"),
        "Should include as_any method for downcasting"
    );

    // Verify enum generation for polymorphism
    assert!(
        code.contains("pub enum ShapeOrSubtype") || code.contains("pub enum ShapeVariant"),
        "Should generate polymorphic enum"
    );

    println!("✅ Trait generation test passed!");
}
