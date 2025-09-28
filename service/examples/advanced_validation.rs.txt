//! Advanced validation example for LinkML service
//!
//! This example demonstrates:
//! - Custom validation rules
//! - Complex constraints
//! - Cross-field validation
//! - Performance optimization
//! - Error recovery strategies

use linkml_service::{create_linkml_service_with_config, LinkMLService};
use linkml_core::{prelude::*, config::LinkMLConfig, error::Result};
use serde_json::json;
use std::time::Instant;
use std::sync::Arc;
use futures::future;

// RootReal service dependencies - following dyn-compatibility guidelines
use logger_core::LoggerService;
use timestamp_core::TimestampService;
use task_management_service::StandardTaskManagementService;
use error_handling_service::StandardErrorHandlingService;
use configuration_service::StandardConfigurationService;
use cache_service::ValkeyCache;
use monitoring_service::StandardMonitoringService;

#[tokio::main]
async fn main() -> Result<()> {
    println!("LinkML Advanced Validation Example");
    println!("=================================\n");

    // Configure service for performance
    let config = LinkMLConfig {
        validation: linkml_core::config::ValidationConfig {
            strict_mode: true,
            enable_patterns: true,
            enable_instances: true,
            max_errors: 100,
            timeout: std::time::Duration::from_secs(30),
            enable_coercion: true,
            ..Default::default()
        },
        performance: linkml_core::config::PerformanceConfig {
            enable_compilation: true,
            max_concurrent_validations: 100,
            cache_size_mb: 256,
            ..Default::default()
        },
        ..Default::default()
    };

    // Initialize RootReal services (in production, these would be properly initialized at startup)
    // Following the dyn-compatibility guidelines - concrete types for non-dyn-compatible services
    let logger: Arc<dyn LoggerService<Error = logger_core::LoggerError>> =
        Arc::new(logger_service::StandardLoggerService::new()?);
    let timestamp: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>> =
        Arc::new(timestamp_service::StandardTimestampService::new()?);

    // Non-dyn-compatible services use concrete types
    let task_manager = Arc::new(StandardTaskManagementService::new()?);
    let error_handler = Arc::new(StandardErrorHandlingService::new(
        logger.clone(),
        timestamp.clone(),
    )?);
    let config_service = Arc::new(StandardConfigurationService::new()?);

    // Dyn-compatible services
    let cache: Arc<dyn cache_core::CacheService<Error = cache_core::CacheError>> =
        Arc::new(ValkeyCache::new(
            cache_core::CacheConfig::default(),
            logger.clone(),
            Arc::new(container_management_service::StandardContainerManagementService::new()?),
            task_manager.clone(),
            Arc::new(memory_service::StandardMemoryService::new()?),
        ).await?);

    let monitor: Arc<dyn monitoring_core::MonitoringService<Error = monitoring_core::MonitoringError>> =
        Arc::new(StandardMonitoringService::new(
            logger.clone(),
            timestamp.clone(),
            task_manager.clone(),
        )?);

    let service = create_linkml_service_with_config(
        config,
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    ).await?;

    // Example 1: Advanced constraints
    println!("1. Advanced constraint validation:");
    let schema_yaml = r#"
id: https://example.org/advanced-schema
name: AdvancedSchema
description: Schema with advanced validation rules

classes:
  Product:
    description: A product with complex validation rules
    slots:
      - sku
      - name
      - price
      - discount_price
      - stock_quantity
      - reorder_level
      - categories
      - specifications
      - launch_date
      - discontinue_date
    slot_usage:
      discount_price:
        description: Must be less than regular price
      reorder_level:
        description: Must be less than stock quantity

  Specification:
    description: Product specification
    slots:
      - key
      - value
      - unit

slots:
  sku:
    description: Stock keeping unit
    identifier: true
    range: string
    pattern: "^[A-Z]{3}-[0-9]{6}$"
    required: true

  name:
    description: Product name
    range: string
    required: true
    minimum_length: 3
    maximum_length: 100

  price:
    description: Regular price
    range: decimal
    required: true
    minimum_value: 0.01
    maximum_value: 999999.99

  discount_price:
    description: Discounted price
    range: decimal
    minimum_value: 0.01

  stock_quantity:
    description: Current stock level
    range: integer
    minimum_value: 0
    required: true

  reorder_level:
    description: Reorder when stock reaches this level
    range: integer
    minimum_value: 0

  categories:
    description: Product categories
    range: Category
    multivalued: true
    minimum_cardinality: 1
    maximum_cardinality: 5

  specifications:
    description: Technical specifications
    range: Specification
    multivalued: true
    inlined: true
    inlined_as_list: true

  launch_date:
    description: Product launch date
    range: date

  discontinue_date:
    description: Product discontinuation date
    range: date

  key:
    description: Specification key
    range: string
    required: true

  value:
    description: Specification value
    range: string
    required: true

  unit:
    description: Unit of measurement
    range: UnitType

enums:
  Category:
    permissible_values:
      electronics:
        description: Electronic devices
      clothing:
        description: Apparel and accessories
      home:
        description: Home and garden
      sports:
        description: Sports and outdoors
      books:
        description: Books and media

  UnitType:
    permissible_values:
      - kg
      - g
      - lb
      - oz
      - cm
      - m
      - inch
      - feet
      - watts
      - volts
"#;

    let schema = service.load_schema_str(schema_yaml, SchemaFormat::Yaml).await?;

    // Test various validation scenarios
    let test_products = vec![
        // Valid product
        json!({
            "sku": "ABC-123456",
            "name": "Premium Laptop",
            "price": 999.99,
            "discount_price": 799.99,
            "stock_quantity": 50,
            "reorder_level": 10,
            "categories": ["electronics"],
            "specifications": [
                {"key": "weight", "value": "1.5", "unit": "kg"},
                {"key": "screen", "value": "15.6", "unit": "inch"}
            ],
            "launch_date": "2024-01-15"
        }),

        // Invalid SKU pattern
        json!({
            "sku": "ABC123456",  // Missing dash
            "name": "Test Product",
            "price": 99.99,
            "stock_quantity": 10,
            "categories": ["home"]
        }),

        // Price constraint violations
        json!({
            "sku": "XYZ-999999",
            "name": "Overpriced Item",
            "price": 1000000.00,  // Exceeds max
            "stock_quantity": 1,
            "categories": ["electronics"]
        }),

        // Too many categories
        json!({
            "sku": "DEF-111111",
            "name": "Multi-category Product",
            "price": 49.99,
            "stock_quantity": 100,
            "categories": ["electronics", "home", "sports", "books", "clothing", "electronics"]  // 6 categories
        }),
    ];

    for (i, product) in test_products.iter().enumerate() {
        println!("\n   Product {}:", i + 1);
        let start = Instant::now();
        let report = service.validate(product, &schema, "Product").await?;
        let duration = start.elapsed();

        println!("   Result: {} (validated in {:.2}ms)",
            if report.valid { "VALID ✓" } else { "INVALID ✗" },
            duration.as_secs_f64() * 1000.0
        );

        if !report.valid {
            for error in &report.errors {
                println!("   - {}", error.message);
            }
        }
    }

    // Example 2: Cross-field validation
    println!("\n\n2. Cross-field validation:");
    let cross_validation_schema = r#"
id: https://example.org/cross-validation
name: CrossValidationSchema

classes:
  DateRange:
    description: A date range with validation
    slots:
      - start_date
      - end_date
      - duration_days
    rules:
      - preconditions:
          slot_conditions:
            start_date:
              value_presence: PRESENT
            end_date:
              value_presence: PRESENT
        postconditions:
          slot_conditions:
            end_date:
              all_of:
                - range: date
                - greater_than_or_equals:
                    slot_name: start_date

  PriceRange:
    description: Price range with percentage validation
    slots:
      - min_price
      - max_price
      - average_price

slots:
  start_date:
    range: date
    required: true

  end_date:
    range: date
    required: true

  duration_days:
    range: integer
    minimum_value: 1

  min_price:
    range: float
    minimum_value: 0
    required: true

  max_price:
    range: float
    minimum_value: 0
    required: true

  average_price:
    range: float
    minimum_value: 0
"#;

    let cross_schema = service.load_schema_str(cross_validation_schema, SchemaFormat::Yaml).await?;

    // Test cross-field validation
    let date_ranges = vec![
        json!({
            "start_date": "2024-01-01",
            "end_date": "2024-12-31",
            "duration_days": 365
        }),
        json!({
            "start_date": "2024-12-31",
            "end_date": "2024-01-01",  // End before start
            "duration_days": -364
        }),
    ];

    for (i, range) in date_ranges.iter().enumerate() {
        let report = service.validate(range, &cross_schema, "DateRange").await?;
        println!("   Date range {}: {}",
            i + 1,
            if report.valid { "VALID ✓" } else { "INVALID ✗" }
        );
    }

    // Example 3: Performance testing with batch validation
    println!("\n\n3. Performance testing with batch validation:");

    // Generate test data
    let batch_size = 1000;
    let mut batch_data = Vec::new();
    for i in 0..batch_size {
        batch_data.push(json!({
            "sku": format!("TST-{:06}", i),
            "name": format!("Test Product {}", i),
            "price": 99.99 + (i as f64 * 0.01),
            "stock_quantity": i % 100,
            "categories": vec!["electronics"],
            "launch_date": "2024-01-01"
        }));
    }

    let start = Instant::now();
    let mut valid_count = 0;

    // Validate in parallel batches
    let chunk_size = 100;
    for chunk in batch_data.chunks(chunk_size) {
        let futures: Vec<_> = chunk.iter()
            .map(|data| service.validate(data, &schema, "Product"))
            .collect();

        let results = future::join_all(futures).await;
        valid_count += results.iter().filter(|r| r.as_ref().unwrap().valid).count();
    }

    let duration = start.elapsed();
    println!("   Validated {} records in {:.2}ms", batch_size, duration.as_secs_f64() * 1000.0);
    println!("   Average: {:.2}ms per record", duration.as_secs_f64() * 1000.0 / batch_size as f64);
    println!("   Valid records: {}/{}", valid_count, batch_size);

    // Example 4: Custom validation with rules
    println!("\n\n4. Custom validation rules:");
    let rules_schema = r#"
id: https://example.org/rules-schema
name: RulesSchema

classes:
  User:
    description: User with complex validation rules
    slots:
      - username
      - email
      - password
      - age
      - country
      - preferences
    rules:
      - description: Premium users must be 18+
        preconditions:
          slot_conditions:
            preferences:
              contains: "premium"
        postconditions:
          slot_conditions:
            age:
              minimum_value: 18

      - description: EU users need GDPR consent
        preconditions:
          slot_conditions:
            country:
              in:
                - DE
                - FR
                - IT
                - ES
        postconditions:
          slot_conditions:
            preferences:
              contains: "gdpr_consent"

slots:
  username:
    range: string
    pattern: "^[a-zA-Z0-9_]{3,20}$"
    required: true

  email:
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[A-Z|a-z]{2,}$"
    required: true

  password:
    range: string
    minimum_length: 8
    pattern: "^(?=.*[a-z])(?=.*[A-Z])(?=.*\\d)(?=.*[@$!%*?&])[A-Za-z\\d@$!%*?&]+$"
    required: true
    description: Must contain uppercase, lowercase, number, and special character

  age:
    range: integer
    minimum_value: 13
    maximum_value: 120

  country:
    range: string
    pattern: "^[A-Z]{2}$"

  preferences:
    range: string
    multivalued: true
"#;

    let rules_schema = service.load_schema_str(rules_schema, SchemaFormat::Yaml).await?;

    let test_users = vec![
        json!({
            "username": "premium_user",
            "email": "premium@example.com",
            "password": "SecureP@ss123",
            "age": 25,
            "country": "US",
            "preferences": ["premium", "notifications"]
        }),
        json!({
            "username": "young_premium",
            "email": "young@example.com",
            "password": "SecureP@ss123",
            "age": 16,  // Too young for premium
            "country": "US",
            "preferences": ["premium"]
        }),
        json!({
            "username": "eu_user",
            "email": "user@example.de",
            "password": "SecureP@ss123",
            "age": 30,
            "country": "DE",
            "preferences": ["notifications"]  // Missing GDPR consent
        }),
    ];

    for (i, user) in test_users.iter().enumerate() {
        let report = service.validate(user, &rules_schema, "User").await?;
        println!("\n   User {}: {}",
            i + 1,
            if report.valid { "VALID ✓" } else { "INVALID ✗" }
        );

        if !report.valid {
            for error in &report.errors {
                println!("   - {}", error.message);
            }
        }
    }

    // Example 5: Error recovery and suggestions
    println!("\n\n5. Error recovery and suggestions:");

    let invalid_product = json!({
        "sku": "ABC123",  // Missing dash
        "name": "Te",     // Too short
        "price": -10.0,   // Negative
        "stock_quantity": "fifty",  // Wrong type
        "categories": [],  // Empty
        "launch_date": "2024-13-45"  // Invalid date
    });

    let report = service.validate(&invalid_product, &schema, "Product").await?;

    println!("   Validation errors with recovery suggestions:");
    for error in &report.errors {
        println!("\n   Issue: {}", error.message);
        println!("   Location: {}", error.path.as_deref().unwrap_or("root"));

        // Provide recovery suggestions based on error type
        if error.message.contains("pattern") {
            println!("   Suggestion: Check the format requirements for this field");
        } else if error.message.contains("minimum_length") {
            println!("   Suggestion: Ensure the value meets minimum length requirements");
        } else if error.message.contains("minimum_value") {
            println!("   Suggestion: Value must be greater than or equal to the minimum");
        } else if error.message.contains("type") {
            println!("   Suggestion: Check that the value is of the correct data type");
        } else if error.message.contains("minimum_cardinality") {
            println!("   Suggestion: At least one value is required for this field");
        }
    }

    println!("\n\nAdvanced validation example completed!");

    Ok(())
}
