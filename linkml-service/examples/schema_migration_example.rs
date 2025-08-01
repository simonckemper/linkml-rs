//! Example demonstrating LinkML to TypeQL schema migration
//!
//! This example shows how to:
//! 1. Compare two schema versions
//! 2. Analyze the impact of changes
//! 3. Generate migration scripts

use linkml_core::prelude::*;
use linkml_service::generator::typeql_migration::{
    SchemaDiffer, MigrationAnalyzer, MigrationGenerator,
    SchemaVersion,
};

fn main() {
    // Create a v1 schema
    let mut v1_schema = SchemaDefinition::default();
    v1_schema.name = "ProductCatalog".to_string();
    v1_schema.version = Some("1.0.0".to_string());
    
    // Define Product class
    let mut product = ClassDefinition::default();
    product.description = Some("A product in our catalog".to_string());
    product.slots.extend(vec![
        "name".to_string(),
        "price".to_string(),
        "description".to_string(),
    ]);
    
    // Define slots
    let mut name_slot = SlotDefinition::default();
    name_slot.required = Some(true);
    product.slot_usage.insert("name".to_string(), name_slot);
    
    v1_schema.classes.insert("Product".to_string(), product);
    
    // Global slot definitions
    let mut name = SlotDefinition::default();
    name.range = Some("string".to_string());
    v1_schema.slots.insert("name".to_string(), name);
    
    let mut price = SlotDefinition::default();
    price.range = Some("float".to_string());
    v1_schema.slots.insert("price".to_string(), price);
    
    let mut description = SlotDefinition::default();
    description.range = Some("string".to_string());
    v1_schema.slots.insert("description".to_string(), description);
    
    // Create a v2 schema with changes
    let mut v2_schema = v1_schema.clone();
    v2_schema.version = Some("2.0.0".to_string());
    
    // Modify Product class
    if let Some(product) = v2_schema.classes.get_mut("Product") {
        // Add SKU field (required)
        product.slots.push("sku".to_string());
        let mut sku_slot = SlotDefinition::default();
        sku_slot.required = Some(true);
        sku_slot.identifier = Some(true);
        product.slot_usage.insert("sku".to_string(), sku_slot);
        
        // Add category field
        product.slots.push("category".to_string());
        
        // Remove description field
        product.slots.retain(|s| s != "description");
    }
    
    // Add new global slots
    let mut sku = SlotDefinition::default();
    sku.range = Some("string".to_string());
    sku.pattern = Some(r"^SKU-\d{6}$".to_string());
    v2_schema.slots.insert("sku".to_string(), sku);
    
    let mut category = SlotDefinition::default();
    category.range = Some("string".to_string());
    v2_schema.slots.insert("category".to_string(), category);
    
    // Remove description slot
    v2_schema.slots.remove("description");
    
    // Add a new class
    let mut category_class = ClassDefinition::default();
    category_class.description = Some("Product category".to_string());
    category_class.slots.push("name".to_string());
    v2_schema.classes.insert("Category".to_string(), category_class);
    
    println!("=== Schema Migration Example ===\n");
    
    // Step 1: Version management
    println!("1. Version Management");
    let v1_version = SchemaVersion::parse("1.0.0").unwrap();
    let v2_version = SchemaVersion::parse("2.0.0").unwrap();
    println!("   - From version: {}", v1_version);
    println!("   - To version: {}", v2_version);
    println!("   - Is major version change: {}", v2_version.is_breaking_change_from(&v1_version));
    
    // Step 2: Compare schemas
    println!("\n2. Schema Comparison");
    let diff = SchemaDiffer::compare(&v1_schema, &v2_schema).unwrap();
    println!("   - Added types: {}", diff.added_types.len());
    println!("   - Modified types: {}", diff.modified_types.len());
    println!("   - Added attributes: {}", diff.added_attributes.len());
    println!("   - Removed attributes: {}", diff.removed_attributes.len());
    
    // Step 3: Analyze impact
    println!("\n3. Impact Analysis");
    let impact = MigrationAnalyzer::analyze_impact(&diff).unwrap();
    println!("   - Category: {:?}", impact.category);
    println!("   - Breaking changes: {}", impact.breaking_changes.len());
    println!("   - Warnings: {}", impact.warnings.len());
    println!("   - Safe changes: {}", impact.safe_changes.len());
    println!("   - Requires data migration: {}", impact.requires_data_migration);
    println!("   - Complexity score: {}/10", impact.complexity_score);
    
    if !impact.breaking_changes.is_empty() {
        println!("\n   Breaking changes:");
        for change in &impact.breaking_changes {
            println!("   - {}", change);
        }
    }
    
    if !impact.warnings.is_empty() {
        println!("\n   Warnings:");
        for warning in &impact.warnings {
            println!("   - {}", warning);
        }
    }
    
    // Step 4: Generate migration scripts
    println!("\n4. Migration Script Generation");
    let generator = MigrationGenerator::new();
    let migration = generator.generate(&diff, &impact, "1.0.0", "2.0.0").unwrap();
    
    println!("\n=== Forward Migration Script ===");
    println!("{}", migration.forward_script());
    
    println!("\n=== Rollback Script ===");
    println!("{}", migration.rollback_script());
    
    // Step 5: Data migrations
    if !migration.data_migrations.is_empty() {
        println!("\n=== Data Migrations ===");
        for (i, data_migration) in migration.data_migrations.iter().enumerate() {
            println!("\nStep {}: {}", i + 1, data_migration.description);
            println!("Query:\n{}", data_migration.query);
            println!("Idempotent: {}", data_migration.idempotent);
        }
    }
    
    println!("\n=== Migration Summary ===");
    println!("- Migration is breaking: {}", migration.metadata.is_breaking);
    println!("- Complexity: {}/10", migration.metadata.complexity);
    println!("- Generated at: {}", migration.metadata.generated_at);
    println!("\nMigration planning complete!");
}