// Validation Patterns Example
//
// This example demonstrates various validation patterns including:
// - Pattern matching with regex
// - Range constraints
// - Enum validation
// - Cross-field validation
// - Custom validation rules

use linkml_core::prelude::*;
use linkml_service::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Validation Patterns Example");
    println!(
        "=================================
"
    );

    // Comprehensive validation schema
    let schema_yaml = r#"
id: https://example.com/validation-patterns
name: ValidationPatterns
description: Comprehensive validation examples

classes:
  Person:
    description: Person with various validation constraints
    slots:
      - id
      - email
      - phone
      - age
      - username
      - password
      - confirm_password
      - country_code
      - postal_code
      - website
      - social_security
    rules:
      - title: password_match
        description: Password and confirmation must match
        preconditions:
          expression: "password != confirm_password"
        postconditions:
          expression: "false"
      - title: adult_only
        description: Must be 18 or older
        preconditions:
          expression: "age < 18"
        postconditions:
          expression: "false"

slots:
  id:
    identifier: true
    range: string
    pattern: "^[A-Z]{3}-\\d{6}$"
    description: Format XXX-000000

  email:
    range: string
    required: true
    pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
    description: Valid email address

  phone:
    range: string
    pattern: "^\\+?1?[-.]?(\\([0-9]{3}\\)|[0-9]{3})[-.]?[0-9]{3}[-.]?[0-9]{4}$"
    description: US phone number format

  age:
    range: integer
    minimum_value: 0
    maximum_value: 150

  username:
    range: string
    required: true
    pattern: "^[a-zA-Z0-9_]{3,20}$"
    description: Alphanumeric, 3-20 characters

  password:
    range: string
    required: true
    pattern: "^(?=.*[a-z])(?=.*[A-Z])(?=.*\\d)(?=.*[@$!%*?&])[A-Za-z\\d@$!%*?&]{8,}$"
    description: Min 8 chars, uppercase, lowercase, number, special char

  confirm_password:
    range: string
    required: true

  country_code:
    range: CountryCode

  postal_code:
    range: string
    description: Validated based on country

  website:
    range: uri
    pattern: "^https?://[\\w.-]+(\\.[\\w.-]+)+[\\w.,@?^=%&:/~+#-]*$"

  social_security:
    range: string
    pattern: "^\\d{3}-\\d{2}-\\d{4}$"
    description: US SSN format XXX-XX-XXXX

enums:
  CountryCode:
    description: ISO country codes
    permissible_values:
      US:
        description: United States
        meaning: ISO:US
      CA:
        description: Canada
        meaning: ISO:CA
      UK:
        description: United Kingdom
        meaning: ISO:GB
      DE:
        description: Germany
        meaning: ISO:DE
      FR:
        description: France
        meaning: ISO:FR

classes:
  Product:
    description: Product with business rules
    slots:
      - sku
      - name
      - price
      - cost
      - margin
      - category
      - stock_level
      - reorder_point
      - discontinued
    rules:
      - title: positive_margin
        description: Margin must be positive
        preconditions:
          expression: "price <= cost"
        postconditions:
          expression: "false"
      - title: reorder_check
        description: Must reorder when stock below reorder point
        preconditions:
          expression: "stock_level < reorder_point && !discontinued"
        postconditions:
          expression: "needs_reorder"

slots:
  sku:
    identifier: true
    range: string
    pattern: "^[A-Z]{2}-\\d{4}-[A-Z0-9]{4}$"
    description: Format XX-0000-XXXX

  name:
    range: string
    required: true

  price:
    range: decimal
    minimum_value: 0

  cost:
    range: decimal
    minimum_value: 0

  margin:
    range: decimal
    equals_expression: "(price - cost) / price"

  category:
    range: ProductCategory
    required: true

  stock_level:
    range: integer
    minimum_value: 0

  reorder_point:
    range: integer
    minimum_value: 0

  discontinued:
    range: boolean

  needs_reorder:
    range: boolean

enums:
  ProductCategory:
    permissible_values:
      electronics:
        description: Electronic devices
      clothing:
        description: Apparel and accessories
      food:
        description: Food and beverages
      books:
        description: Books and publications
"#;

    // Parse schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;

    // Create LinkML service
    let service = create_linkml_service().await?;

    // Example 1: Valid person data
    println!("Example 1: Valid Person Data");
    println!("----------------------------");

    let valid_person = json!({
        "id": "ABC-123456",
        "email": "john.doe@example.com",
        "phone": "+1 (555) 123-4567",
        "age": 25,
        "username": "john_doe123",
        "password": "SecureP@ss123",
        "confirm_password": "SecureP@ss123",
        "country_code": "US",
        "postal_code": "12345",
        "website": "https://johndoe.com",
        "social_security": "123-45-6789"
    });

    let report = service.validate(&valid_person, &schema, "Person").await?;
    if report.valid {
        println!("✓ All validations passed!");
    }

    // Example 2: Invalid patterns
    println!(
        "

Example 2: Pattern Validation Failures"
    );
    println!("--------------------------------------");

    let invalid_patterns = json!({
        "id": "123-ABCDEF",  // Wrong format
        "email": "invalid.email",  // Missing @domain
        "phone": "555-CALL-NOW",  // Not a valid number
        "age": 25,
        "username": "a",  // Too short
        "password": "weak",  // Doesn't meet requirements
        "confirm_password": "weak",
        "country_code": "US"
    });

    let report = service
        .validate(&invalid_patterns, &schema, "Person")
        .await?;
    if !report.valid {
        println!("✗ Pattern validation failures:");
        for error in &report.errors {
            if error.field.is_some() {
                println!("  - {}: {}", error.field.as_ref()?, error.message);
            }
        }
    }

    // Example 3: Cross-field validation
    println!(
        "

Example 3: Cross-field Validation"
    );
    println!("---------------------------------");

    let password_mismatch = json!({
        "id": "ABC-123456",
        "email": "jane@example.com",
        "age": 30,
        "username": "jane_doe",
        "password": "SecureP@ss123",
        "confirm_password": "DifferentP@ss456",  // Doesn't match
        "country_code": "US"
    });

    let report = service
        .validate(&password_mismatch, &schema, "Person")
        .await?;
    if !report.valid {
        println!("✗ Cross-field validation failed:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }

    // Example 4: Business rule validation
    println!(
        "

Example 4: Business Rule Validation"
    );
    println!("-----------------------------------");

    let product_with_loss = json!({
        "sku": "EL-2024-A1B2",
        "name": "Smartphone",
        "price": 299.99,
        "cost": 350.00,  // Cost higher than price!
        "category": "electronics",
        "stock_level": 5,
        "reorder_point": 10,
        "discontinued": false
    });

    let report = service
        .validate(&product_with_loss, &schema, "Product")
        .await?;
    if !report.valid {
        println!("✗ Business rule violations:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }

    // Example 5: Enum validation
    println!(
        "

Example 5: Enum Validation"
    );
    println!("--------------------------");

    let invalid_enum = json!({
        "id": "ABC-123456",
        "email": "test@example.com",
        "age": 25,
        "username": "test_user",
        "password": "SecureP@ss123",
        "confirm_password": "SecureP@ss123",
        "country_code": "XX"  // Invalid country code
    });

    let report = service.validate(&invalid_enum, &schema, "Person").await?;
    if !report.valid {
        println!("✗ Enum validation failed:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }

    // Summary of validation patterns
    println!(
        "

Validation Pattern Summary"
    );
    println!("=========================");
    println!("1. Pattern Matching: Use regex for format validation");
    println!("2. Range Constraints: Set min/max values for numbers");
    println!("3. Required Fields: Mark fields as mandatory");
    println!("4. Enum Validation: Restrict to predefined values");
    println!("5. Cross-field Rules: Validate field relationships");
    println!("6. Computed Fields: Calculate values from other fields");
    println!("7. Business Rules: Implement domain-specific logic");
    println!(
        "
All patterns support clear error messages for user feedback!"
    );

    Ok(())
}
