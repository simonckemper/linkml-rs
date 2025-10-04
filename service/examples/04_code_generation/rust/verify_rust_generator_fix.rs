//! Verify the RustGenerator fix generates proper struct fields
//!
//! This example demonstrates that the RustGenerator now generates actual
//! struct fields instead of TODO comments.

use linkml_service::generator::{GeneratorRegistry};
use linkml_service::parser::YamlParser;
use std::error::Error;

fn main() -> std::result::Result<(), Box<dyn Error>> {
    println!(
        "=== Verifying RustGenerator Fix ===
"
    );

    // Simple test schema with a class and slots
    let schema_yaml = r#"
id: https://example.org/verification
name: VerificationSchema
description: Schema to verify RustGenerator generates proper fields
prefixes:
  ex: https://example.org/
default_prefix: ex

classes:
  Product:
    description: A product in an e-commerce system
    slots:
      - sku
      - name
      - description
      - price
      - in_stock
      - categories
      - manufacturer

  Manufacturer:
    description: A product manufacturer
    slots:
      - id
      - company_name
      - founded_year

slots:
  sku:
    description: Stock Keeping Unit identifier
    identifier: true
    range: string
    required: true
  name:
    description: Product name
    range: string
    required: true
  description:
    description: Product description
    range: string
  price:
    description: Product price in cents
    range: integer
    required: true
  in_stock:
    description: Whether product is in stock
    range: boolean
  categories:
    description: Product categories
    range: string
    multivalued: true
  manufacturer:
    description: Product manufacturer
    range: Manufacturer
  id:
    description: Manufacturer ID
    identifier: true
    range: string
    required: true
  company_name:
    description: Company name
    range: string
    required: true
  founded_year:
    description: Year company was founded
    range: integer

enums:
  ProductStatus:
    description: Product availability status
    permissible_values:
      IN_STOCK:
        description: Product is available for purchase
      OUT_OF_STOCK:
        description: Product is temporarily unavailable
      DISCONTINUED:
        description: Product is no longer sold
      PRE_ORDER:
        description: Product available for pre-order
"#;

    println!("Input Schema:");
    println!("{}", "─".repeat(60));
    println!("{}", schema_yaml);
    println!("{}", "─".repeat(60));

    // Parse the schema
    let parser = YamlParser::new();
    let schema = parser.parse(schema_yaml)?;

    println!(
        "
✅ Schema parsed successfully"
    );
    println!(
        "  - Classes: {:?}",
        schema.classes.keys().collect::<Vec<_>>()
    );
    println!("  - Slots: {:?}", schema.slots.keys().collect::<Vec<_>>());
    println!("  - Enums: {:?}", schema.enums.keys().collect::<Vec<_>>());

    // Get the Rust generator
    let registry = GeneratorRegistry::new();
    let generator = registry
        .get_generator("rust")
        .ok_or("Rust generator not found")?;

    println!(
        "
✅ RustGenerator loaded: {}",
        generator.description()
    );

    // Generate Rust code
    println!(
        "
🔧 Generating Rust code..."
    );
    let generated_code = generator.generate(&schema)?;

    println!(
        "
📄 Generated Rust Code:"
    );
    println!("{}", "─".repeat(60));
    println!("{}", generated_code);
    println!("{}", "─".repeat(60));

    // Verify the fix worked
    println!(
        "
🔍 Verification Results:"
    );

    if generated_code.contains("// TODO: Add fields based on slots") {
        println!("❌ FAIL: Generated code still contains TODO comment!");
        println!("   The RustGenerator is NOT fixed properly.");
        return Err("RustGenerator still generating TODO comments".into());
    } else {
        println!("✅ PASS: No TODO comments found");
    }

    // Check for expected fields
    let expected_fields = vec![
        "pub sku: String",
        "pub name: String",
        "pub description: Option<String>",
        "pub price: i64",
        "pub in_stock: Option<bool>",
        "pub categories: Vec<String>",
        "pub manufacturer: Option<Box<Manufacturer>>",
        "pub company_name: String",
        "pub founded_year: Option<i64>",
    ];

    let mut all_fields_found = true;
    for field in &expected_fields {
        if generated_code.contains(field) {
            println!("✅ Found field: {}", field);
        } else {
            println!("❌ Missing field: {}", field);
            all_fields_found = false;
        }
    }

    // Check for enum generation
    if generated_code.contains("pub enum ProductStatus") {
        println!("✅ Enum ProductStatus generated");

        let enum_variants = vec!["InStock", "OutOfStock", "Discontinued", "PreOrder"];
        for variant in enum_variants {
            if generated_code.contains(&variant) {
                println!("  ✅ Found variant: {}", variant);
            } else {
                println!("  ❌ Missing variant: {}", variant);
            }
        }
    } else {
        println!("❌ Enum ProductStatus not generated");
        all_fields_found = false;
    }

    // Check for proper derive macros
    if generated_code.contains("#[derive(Debug, Clone, Serialize, Deserialize)]") {
        println!("✅ Proper derive macros found");
    } else {
        println!("❌ Missing proper derive macros");
        all_fields_found = false;
    }

    // Check for validation error enum
    if generated_code.contains("pub enum ValidationError") {
        println!("✅ ValidationError enum generated");
    } else {
        println!("❌ ValidationError enum not generated");
        all_fields_found = false;
    }

    if all_fields_found {
        println!(
            "
🎉 SUCCESS: RustGenerator is properly fixed!"
        );
        println!("   All expected fields and enums were generated correctly.");
    } else {
        println!(
            "
⚠️  PARTIAL SUCCESS: Some fields or enums missing"
        );
        println!("   The RustGenerator is partially working but may need more fixes.");
    }

    Ok(())
}
