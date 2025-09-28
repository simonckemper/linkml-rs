mod common;
use common::create_example_service;

//! Expression Language Example
//!
//! This example demonstrates the LinkML expression language for validation rules,
//! computed fields, and conditional logic.

use linkml_core::prelude::*;
use linkml_service::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Expression Language Example");
    println!("=================================
");

    // Example 1: Basic expressions in slot definitions
    let basic_schema = r#"
id: https://example.com/expressions
name: ExpressionSchema
description: Schema demonstrating expression language

classes:
  Product:
    description: A product with computed fields
    slots:
      - id
      - name
      - base_price
      - tax_rate
      - total_price
      - discount_percentage
      - final_price
    slot_usage:
      total_price:
        equals_expression: "base_price + (base_price * tax_rate)"
      final_price:
        equals_expression: "total_price * (1 - discount_percentage / 100)"

slots:
  id:
    identifier: true
    range: string
  name:
    range: string
    required: true
  base_price:
    range: float
    minimum_value: 0
  tax_rate:
    range: float
    minimum_value: 0
    maximum_value: 1
  total_price:
    range: float
    description: Computed from base_price and tax_rate
  discount_percentage:
    range: float
    minimum_value: 0
    maximum_value: 100
  final_price:
    range: float
    description: Total price after discount
"#;

    // Parse schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(basic_schema)?;

    // Create LinkML service (simplified for example)
    let service = create_example_linkml_service().await?;

    // Example product data
    let product = json!({
        "id": "PROD001",
        "name": "Widget",
        "base_price": 100.0,
        "tax_rate": 0.08,
        "discount_percentage": 10.0
    });

    println!("Validating product with computed fields...");
    let report = service.validate(&product, &schema, "Product").await?;

    if report.valid {
        println!("✓ Product validation passed!");
        println!("  Base price: $100.00");
        println!("  Tax rate: 8%");
        println!("  Total price: $108.00 (computed)");
        println!("  Discount: 10%");
        println!("  Final price: $97.20 (computed)");
    }

    // Example 2: Validation rules with expressions
    let validation_schema = r#"
id: https://example.com/validation
name: ValidationSchema
description: Schema with expression-based validation

classes:
  Order:
    description: Order with complex validation
    slots:
      - id
      - items
      - shipping_cost
      - total_value
      - express_shipping
    rules:
      - title: minimum_order_value
        description: Orders must be at least $25
        preconditions:
          expression: "total_value < 25"
        postconditions:
          expression: "false"
      - title: express_shipping_minimum
        description: Express shipping requires order over $50
        preconditions:
          expression: "express_shipping && total_value < 50"
        postconditions:
          expression: "false"

slots:
  id:
    identifier: true
    range: string
  items:
    range: OrderItem
    multivalued: true
    minimum_cardinality: 1
  shipping_cost:
    range: float
    minimum_value: 0
  total_value:
    range: float
    minimum_value: 0
  express_shipping:
    range: boolean

classes:
  OrderItem:
    slots:
      - product_id
      - quantity
      - price

slots:
  product_id:
    range: string
  quantity:
    range: integer
    minimum_value: 1
  price:
    range: float
    minimum_value: 0
"#;

    let validation_schema = parser.parse_str(validation_schema)?;

    // Test order that violates minimum value rule
    let small_order = json!({
        "id": "ORDER001",
        "items": [{
            "product_id": "PROD001",
            "quantity": 1,
            "price": 20.0
        }],
        "shipping_cost": 5.0,
        "total_value": 20.0,
        "express_shipping": false
    });

    println!("

Validating order with rule expressions...");
    let report = service.validate(&small_order, &validation_schema, "Order").await?;

    if !report.valid {
        println!("✗ Order validation failed:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }

    // Example 3: Conditional requirements
    let conditional_schema = r#"
id: https://example.com/conditional
name: ConditionalSchema
description: Schema with conditional requirements

classes:
  UserAccount:
    description: User account with conditional fields
    slots:
      - username
      - account_type
      - business_name
      - tax_id
      - personal_email
      - date_of_birth
    conditional_requirements:
      - if_field: account_type
        equals_string: business
        required_fields: [business_name, tax_id]
      - if_field: account_type
        equals_string: personal
        required_fields: [personal_email, date_of_birth]

slots:
  username:
    identifier: true
    range: string
  account_type:
    range: AccountType
    required: true
  business_name:
    range: string
  tax_id:
    range: string
    pattern: "^\\d{2}-\\d{7}$"
  personal_email:
    range: string
    pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"
  date_of_birth:
    range: date

enums:
  AccountType:
    permissible_values:
      personal:
      business:
"#;

    let conditional_schema = parser.parse_str(conditional_schema)?;

    // Test business account
    let business_account = json!({
        "username": "acme_corp",
        "account_type": "business",
        "business_name": "ACME Corporation",
        "tax_id": "12-3456789"
    });

    println!("

Validating business account...");
    let report = service.validate(&business_account, &conditional_schema, "UserAccount").await?;

    if report.valid {
        println!("✓ Business account validation passed!");
        println!("  Required business fields present");
    }

    // Test personal account missing required field
    let incomplete_personal = json!({
        "username": "john_doe",
        "account_type": "personal",
        "personal_email": "john@example.com"
        // Missing date_of_birth
    });

    println!("
Validating incomplete personal account...");
    let report = service.validate(&incomplete_personal, &conditional_schema, "UserAccount").await?;

    if !report.valid {
        println!("✗ Personal account validation failed:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }

    // Example 4: Complex expressions with functions
    println!("

Expression Language Features");
    println!("---------------------------");
    println!("Supported operators:");
    println!("  - Arithmetic: +, -, *, /, %");
    println!("  - Comparison: ==, !=, <, >, <=, >=");
    println!("  - Logical: &&, ||, !");
    println!("  - Functions: len(), contains(), matches()");
    println!("
Example expressions:");
    println!("  - age >= 18 && age <= 65");
    println!("  - len(items) > 0");
    println!("  - status == 'active' || override");
    println!("  - price * quantity * (1 - discount)");
    println!("  - matches(email, '^[\\w.-]+@[\\w.-]+\\.\\w+$')");

    Ok(())
}

async fn create_example_linkml_service() -> std::result::Result<LinkMLService, Box<dyn std::error::Error>> {
    // In a real application, this would initialize with all dependencies
    // For this example, we'll use a simplified initialization
    create_example_service().await?
}
