//! Compare two LinkML schemas to understand migration impact.
//!
//! This example builds two small schema versions programmatically and prints
//! the structural differences between them. It is intentionally lightweight so
//! the migration concepts are easy to follow without pulling in external
//! tooling.

use linkml_core::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let v1 = build_v1();
    let v2 = build_v2();

    println!("=== Schema Migration Example ===\n");
    println!(
        "From version: {}",
        v1.version.as_deref().unwrap_or("unknown")
    );
    println!(
        "To version:   {}\n",
        v2.version.as_deref().unwrap_or("unknown")
    );

    let class_diff = diff_map(v1.classes, v2.classes);
    let slot_diff = diff_map(v1.slots, v2.slots);

    report(&class_diff, "Classes");
    report(&slot_diff, "Slots");

    if !slot_diff.added.is_empty() {
        println!("\nNew slot details:");
        for name in &slot_diff.added {
            println!("  - {}", name);
        }
    }
}

fn build_v1() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "ProductCatalog".to_string();
    schema.version = Some("1.0.0".to_string());

    let mut product = ClassDefinition::default();
    product.description = Some("Product listing".to_string());
    product.slots = vec![
        "name".to_string(),
        "price".to_string(),
        "description".to_string(),
    ];
    schema.classes.insert("Product".to_string(), product);

    schema
        .slots
        .insert("name".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("price".to_string(), make_slot("float", true));
    schema
        .slots
        .insert("description".to_string(), make_slot("string", false));

    schema
}

fn build_v2() -> SchemaDefinition {
    let mut schema = build_v1();
    schema.version = Some("2.0.0".to_string());

    if let Some(product) = schema.classes.get_mut("Product") {
        product.slots.push("sku".to_string());
        product.slots.push("category".to_string());
        product.slots.retain(|slot| slot != "description");
    }

    schema
        .slots
        .insert("sku".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("category".to_string(), make_slot("Category", false));
    schema.slots.remove("description");

    let mut category = ClassDefinition::default();
    category.description = Some("Product category".to_string());
    category.slots = vec!["name".to_string()];
    schema.classes.insert("Category".to_string(), category);

    schema
}

fn make_slot(range: &str, required: bool) -> SlotDefinition {
    let mut slot = SlotDefinition::default();
    slot.range = Some(range.to_string());
    slot.required = Some(required);
    slot
}

struct MapDiff {
    added: BTreeSet<String>,
    removed: BTreeSet<String>,
    shared: BTreeSet<String>,
}

fn diff_map<T>(before: BTreeMap<String, T>, after: BTreeMap<String, T>) -> MapDiff {
    let before_keys: BTreeSet<_> = before.keys().cloned().collect();
    let after_keys: BTreeSet<_> = after.keys().cloned().collect();

    let added = after_keys.difference(&before_keys).cloned().collect();
    let removed = before_keys.difference(&after_keys).cloned().collect();
    let shared = before_keys.intersection(&after_keys).cloned().collect();

    MapDiff {
        added,
        removed,
        shared,
    }
}

fn report(diff: &MapDiff, label: &str) {
    println!("{label} summary:");
    println!("  + added:   {}", diff.added.len());
    println!("  - removed: {}", diff.removed.len());
    println!("    shared:  {}\n", diff.shared.len());

    if !diff.added.is_empty() {
        println!("  Added {label}:");
        for item in &diff.added {
            println!("    - {item}");
        }
        println!();
    }

    if !diff.removed.is_empty() {
        println!("  Removed {label}:");
        for item in &diff.removed {
            println!("    - {item}");
        }
        println!();
    }
}
