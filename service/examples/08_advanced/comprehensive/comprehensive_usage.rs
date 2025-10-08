//! Minimal end-to-end demonstration of the LinkML service.
//!
//! The example parses a schema, validates sample data, and generates Python
//! dataclasses. It replaces an earlier oversized example that no longer built.

use anyhow::Result;
use linkml_core::prelude::*;
use linkml_service::generator::PythonDataclassGenerator;
use linkml_service::parser::YamlParser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LinkML comprehensive usage (condensed)");
    println!("======================================\n");

    let schema = parse_schema()?;
    validate_examples(&schema).await?;
    generate_python(&schema)?;

    Ok(())
}

fn parse_schema() -> Result<SchemaDefinition> {
    let schema_yaml = r#"
id: https://example.org/catalog
name: product_catalog
description: Demonstration schema for LinkML service examples

classes:
  Product:
    description: An item in the catalogue
    slots:
      - sku
      - name
      - price
      - status
    slot_usage:
      sku:
        identifier: true
        required: true
      name:
        required: true
      price:
        range: decimal
        minimum_value: 0
      status:
        range: ProductStatus

slots:
  sku: {range: string, required: true}
  name: {range: string, required: true}
  price: {range: decimal}
  status: {range: ProductStatus}

enums:
  ProductStatus:
    permissible_values:
      ACTIVE:
        description: Available for sale
      DISCONTINUED:
        description: No longer sold
      BACKORDER:
        description: Temporarily unavailable
"#;

    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    println!("✓ Parsed schema '{}'", schema.name);
    Ok(schema)
}

async fn validate_examples(schema: &SchemaDefinition) -> Result<()> {
    let engine = ValidationEngine::new(Arc::new(schema.clone()));

    let valid = json!({
        "sku": "SKU-001",
        "name": "Example Product",
        "price": 19.95,
        "status": "ACTIVE"
    });

    let invalid = json!({
        "sku": "SKU-002",
        "name": "Broken",
        "price": -10,
        "status": "UNKNOWN"
    });

    let valid_report = engine.validate_instance(&valid, "Product").await?;
    println!("✓ Valid product: {}", valid_report.is_valid());

    let invalid_report = engine.validate_instance(&invalid, "Product").await?;
    println!("✓ Invalid product rejected: {}", !invalid_report.is_valid());
    if !invalid_report.is_valid() {
        for issue in invalid_report.issues.iter().take(2) {
            println!("  • {}", issue.message);
        }
    }

    Ok(())
}

fn generate_python(schema: &SchemaDefinition) -> Result<()> {
    let generator = PythonDataclassGenerator::new();
    generator.validate_schema(schema)?;
    let code = generator.generate(schema)?;

    println!("\nGenerated Python dataclasses (first 12 lines):");
    for line in code.lines().take(12) {
        println!("  {line}");
    }

    Ok(())
}
