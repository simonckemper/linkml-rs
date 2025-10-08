//! Demonstrates the LinkML expression language with computed fields and rules.

use anyhow::Result;
use linkml_core::prelude::*;
use linkml_service::parser::YamlParser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Expression language example");
    println!("===========================\n");

    computed_field_demo().await?;
    conditional_rules_demo().await?;

    Ok(())
}

async fn computed_field_demo() -> Result<()> {
    println!("Computed fields");
    println!("---------------");

    let schema_yaml = r#"
id: https://example.org/pricing
name: pricing_schema

classes:
  Product:
    slots:
      - base_price
      - tax_rate
      - discount
      - final_price
    slot_usage:
      final_price:
        equals_expression: "round(base_price * (1 + tax_rate) * (1 - discount / 100), 2)"

slots:
  base_price: {range: float, required: true}
  tax_rate: {range: float, required: true}
  discount: {range: float, required: true}
  final_price: {range: float}
"#;

    let schema = YamlParser::new().parse_str(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    let product = json!({
        "base_price": 150.0,
        "tax_rate": 0.07,
        "discount": 5.0,
        "final_price": 0.0
    });

    let report = engine.validate_instance(&product, "Product").await?;
    println!("  valid: {}", report.is_valid());
    println!(
        "  expressions evaluated: {}\n",
        report.expression_results.len()
    );

    Ok(())
}

async fn conditional_rules_demo() -> Result<()> {
    println!("Conditional rules");
    println!("------------------");

    let schema_yaml = r#"
id: https://example.org/orders
name: order_schema

classes:
  Order:
    slots:
      - total_value
      - express_shipping
    rules:
      - title: express_shipping_minimum
        description: Express shipping requires order total >= 50
        preconditions:
          expression: "express_shipping && total_value < 50"
        postconditions:
          expression: "false"

slots:
  total_value: {range: float, required: true}
  express_shipping: {range: boolean, required: true}
"#;

    let schema = YamlParser::new().parse_str(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    let order = json!({
        "total_value": 30.0,
        "express_shipping": true
    });

    let report = engine.validate_instance(&order, "Order").await?;
    println!("  valid: {}", report.is_valid());
    for issue in &report.issues {
        println!("  • {}", issue.message);
    }
    println!();

    Ok(())
}
