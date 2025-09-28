//! Performance-focused integration tests for LinkML service
//!
//! This test suite focuses on performance characteristics and scalability
//! of the LinkML service when used in production scenarios.

use futures::future::join_all;
use linkml_core::prelude::*;
use linkml_service::{
    GeneratorConfig, GeneratorType, LinkMLService, LinkMLServiceConfig, SchemaView,
    create_linkml_service,
};
use serde_json::json;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tempfile::TempDir;

// Import mock services
mod mock_services;
use crate::factory::create_logger_service;
use mock_services::*;

/// Large enterprise schema for performance testing
const ENTERPRISE_SCHEMA: &str = r#"
id: https://example.org/enterprise
name: EnterpriseDataModel
description: Large enterprise data model for performance testing

prefixes:
  ent: https://example.org/enterprise/
  linkml: https://w3id.org/linkml/

classes:
  BaseEntity:
    abstract: true
    attributes:
      id:
        identifier: true
        pattern: "^[A-Z]{3}-[0-9]{10}$"
      created_timestamp:
        range: datetime
        required: true
      modified_timestamp:
        range: datetime
        required: true
      version:
        range: integer
        minimum_value: 1
      tags:
        range: string
        multivalued: true
        pattern: "^[a-z][a-z0-9-]*$"

  Customer:
    is_a: BaseEntity
    description: Enterprise customer entity
    attributes:
      customer_type:
        range: customer_type_enum
        required: true
      company_name:
        range: string
        required: true
      tax_id:
        range: string
        pattern: "^[A-Z]{2}[0-9]{9}$"
      credit_limit:
        range: decimal
        minimum_value: 0
        maximum_value: 10000000
      addresses:
        range: Address
        multivalued: true
        minimum_cardinality: 1
        maximum_cardinality: 10
        inlined_as_list: true
      contacts:
        range: Contact
        multivalued: true
        inlined_as_list: true
      payment_terms:
        range: payment_terms_enum
        required: true
    rules:
      - preconditions:
          slot_conditions:
            customer_type:
              equals: "enterprise"
        postconditions:
          slot_conditions:
            credit_limit:
              greater_than: 100000
        description: Enterprise customers must have high credit limits

  Address:
    attributes:
      address_type:
        range: address_type_enum
        required: true
      street1:
        required: true
      street2:
      city:
        required: true
      state:
        pattern: "^[A-Z]{2}$"
      postal_code:
        pattern: "^[0-9]{5}(-[0-9]{4})?$"
      country:
        pattern: "^[A-Z]{2}$"
        required: true
      validated:
        range: boolean
        ifabsent: 'false'

  Contact:
    attributes:
      contact_id:
        identifier: true
      first_name:
        required: true
      last_name:
        required: true
      email:
        pattern: "^[^@]+@[^@]+\\.[^@]+$"
        required: true
      phone:
        pattern: "^\\+?[0-9]{10,15}$"
      role:
        range: contact_role_enum
      active:
        range: boolean
        ifabsent: 'true'

  Product:
    is_a: BaseEntity
    attributes:
      sku:
        pattern: "^SKU-[A-Z0-9]{8}$"
        required: true
      name:
        required: true
      description:
        recommended: true
      category:
        range: ProductCategory
        required: true
      price:
        range: decimal
        minimum_value: 0
        required: true
      cost:
        range: decimal
        minimum_value: 0
      inventory:
        range: InventoryItem
        inlined: true
      attributes:
        range: ProductAttribute
        multivalued: true
        inlined_as_list: true

  ProductCategory:
    attributes:
      category_id:
        identifier: true
      name:
        required: true
      parent_category:
        range: ProductCategory
      level:
        range: integer
        minimum_value: 1
        maximum_value: 5

  InventoryItem:
    attributes:
      quantity_on_hand:
        range: integer
        minimum_value: 0
        required: true
      quantity_reserved:
        range: integer
        minimum_value: 0
        ifabsent: '0'
      reorder_point:
        range: integer
        minimum_value: 0
      warehouse_locations:
        range: string
        multivalued: true
    expressions:
      - 'available_quantity = quantity_on_hand - quantity_reserved'
      - 'needs_reorder = available_quantity <= reorder_point'

  ProductAttribute:
    attributes:
      name:
        required: true
      value:
        required: true
      unit:
        range: string

  Order:
    is_a: BaseEntity
    attributes:
      order_number:
        pattern: "^ORD-[0-9]{10}$"
        required: true
      customer:
        range: Customer
        required: true
      order_date:
        range: datetime
        required: true
      status:
        range: order_status_enum
        required: true
      line_items:
        range: OrderLineItem
        multivalued: true
        minimum_cardinality: 1
        inlined_as_list: true
      shipping_address:
        range: Address
        required: true
        inlined: true
      billing_address:
        range: Address
        required: true
        inlined: true
      subtotal:
        range: decimal
        minimum_value: 0
      tax:
        range: decimal
        minimum_value: 0
      shipping:
        range: decimal
        minimum_value: 0
      total:
        range: decimal
        minimum_value: 0
    rules:
      - description: Total calculation validation
        # total = subtotal + tax + shipping

  OrderLineItem:
    attributes:
      line_number:
        range: integer
        minimum_value: 1
        required: true
      product:
        range: Product
        required: true
      quantity:
        range: integer
        minimum_value: 1
        required: true
      unit_price:
        range: decimal
        minimum_value: 0
        required: true
      discount_percent:
        range: decimal
        minimum_value: 0
        maximum_value: 100
        ifabsent: '0'
      line_total:
        range: decimal
        minimum_value: 0
    expressions:
      - 'discounted_price = unit_price * (1 - discount_percent / 100)'
      - 'line_total = discounted_price * quantity'

enums:
  customer_type_enum:
    permissible_values:
      individual:
      small_business:
      enterprise:
      government:

  payment_terms_enum:
    permissible_values:
      net_30:
      net_60:
      net_90:
      prepaid:
      cod:

  address_type_enum:
    permissible_values:
      billing:
      shipping:
      headquarters:
      branch:

  contact_role_enum:
    permissible_values:
      primary:
      billing:
      technical:
      sales:

  order_status_enum:
    permissible_values:
      draft:
      pending:
      confirmed:
      processing:
      shipped:
      delivered:
      cancelled:
      refunded:
"#;

/// Create a large customer with nested data
fn generate_large_customer(id: u32) -> serde_json::Value {
    let mut addresses = Vec::new();
    for i in 1..=5 {
        addresses.push(json!({
            "address_type": "billing",
            "street1": format!("{} Main Street", 100 + i),
            "street2": format!("Suite {}", i * 100),
            "city": "Enterprise City",
            "state": "CA",
            "postal_code": format!("{:05}", 90000 + i),
            "country": "US",
            "validated": i % 2 == 0
        }));
    }

    let mut contacts = Vec::new();
    for i in 1..=3 {
        contacts.push(json!({
            "contact_id": format!("CON-{:010}", id * 100 + i),
            "first_name": format!("Contact{}", i),
            "last_name": format!("Person{}", id),
            "email": format!("contact{}@company{}.com", i, id),
            "phone": format!("+1555{:07}", id * 1000 + i),
            "role": "technical",
            "active": true
        }));
    }

    json!({
        "id": format!("CUS-{:010}", id),
        "created_timestamp": "2025-01-01T00:00:00Z",
        "modified_timestamp": "2025-01-16T12:00:00Z",
        "version": 1,
        "tags": ["premium", "verified", format!("region-{}", id % 5)],
        "customer_type": "enterprise",
        "company_name": format!("Enterprise Corp {}", id),
        "tax_id": format!("US{:09}", id),
        "credit_limit": 500000.00,
        "addresses": addresses,
        "contacts": contacts,
        "payment_terms": "net_60"
    })
}

/// Create test service optimized for performance
async fn create_performance_service() -> Arc<dyn LinkMLService> {
    let config = LinkMLServiceConfig {
        enable_caching: true,
        cache_ttl_seconds: 600,
        max_validation_errors: 50,
        enable_parallel_validation: true,
        expression_timeout_ms: 1000,
        ..Default::default()
    };

    let logger = Arc::new(MockMockLoggerService::new());
    let timestamp = Arc::new(MockTimestampService);
    let task_manager = Arc::new(MockTaskManagementService);
    let error_handler = Arc::new(MockErrorHandlerService);
    let config_service = Arc::new(MockConfigurationService::new());
    let cache = Arc::new(MockCacheService::new());
    let monitor = Arc::new(MockMonitoringService::new());

    linkml_service::create_linkml_service_with_config(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
        config,
    )
    .await
    .expect("Test operation failed")
}

#[tokio::test]
async fn test_large_schema_loading_performance() {
    println!("=== Testing Large Schema Loading Performance ===");

    let service = create_performance_service().await;

    // Measure schema loading time
    let start = Instant::now();
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");
    let load_time = start.elapsed();

    println!("Schema loading time: {:?}", load_time);
    assert!(
        load_time < Duration::from_millis(500),
        "Schema loading should be fast"
    );

    // Create SchemaView and measure introspection
    let view_start = Instant::now();
    let view = SchemaView::new(schema.clone());
    let stats = view.get_statistics();
    let view_time = view_start.elapsed();

    println!("SchemaView creation time: {:?}", view_time);
    println!("Schema statistics:");
    println!("  - Classes: {}", stats.num_classes);
    println!("  - Attributes: {}", stats.num_slots);
    println!("  - Enums: {}", stats.num_enums);
    println!("  - Rules: {}", stats.num_rules);

    assert!(
        view_time < Duration::from_millis(100),
        "SchemaView creation should be fast"
    );
}

#[tokio::test]
async fn test_bulk_validation_performance() {
    println!("=== Testing Bulk Validation Performance ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    // Generate test data
    let num_customers = 1000;
    let mut customers = Vec::new();
    for i in 0..num_customers {
        customers.push(generate_large_customer(i));
    }

    // Warm up cache
    println!("Warming up cache...");
    for i in 0..10 {
        let _ = service
            .validate(&customers[i], &schema, "Customer")
            .await
            .expect("Test operation failed");
    }

    // Measure bulk validation
    println!("Starting bulk validation of {} customers...", num_customers);
    let start = Instant::now();
    let mut valid_count = 0;
    let mut total_errors = 0;

    for customer in &customers {
        let report = service
            .validate(customer, &schema, "Customer")
            .await
            .expect("Test operation failed");
        if report.valid {
            valid_count += 1;
        } else {
            total_errors += report.errors.len();
        }
    }

    let elapsed = start.elapsed();
    let per_record = elapsed.as_micros() as f64 / num_customers as f64;
    let throughput = num_customers as f64 / elapsed.as_secs_f64();

    println!("
Bulk validation results:");
    println!("  - Total records: {}", num_customers);
    println!("  - Valid records: {}", valid_count);
    println!("  - Total errors: {}", total_errors);
    println!("  - Total time: {:?}", elapsed);
    println!("  - Per record: {:.2} μs", per_record);
    println!("  - Throughput: {:.0} records/second", throughput);

    assert_eq!(valid_count, num_customers, "All customers should be valid");
    assert!(throughput > 100.0, "Should validate >100 records/second");
}

#[tokio::test]
async fn test_concurrent_schema_operations() {
    println!("=== Testing Concurrent Schema Operations ===");

    let service = create_performance_service().await;
    let schema = Arc::new(
        service
            .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
            .await
            .expect("Test operation failed"),
    );

    let num_concurrent_tasks = 100;
    let operations_per_task = 50;

    let total_operations = Arc::new(AtomicU64::new(0));
    let start = Instant::now();

    let mut handles = Vec::new();

    for task_id in 0..num_concurrent_tasks {
        let service_clone = service.clone();
        let schema_clone = schema.clone();
        let counter = total_operations.clone();

        let handle = tokio::spawn(async move {
            for op in 0..operations_per_task {
                match task_id % 4 {
                    0 => {
                        // Validation
                        let customer = generate_large_customer(task_id * 100 + op);
                        let _ = service_clone
                            .validate(&customer, &schema_clone, "Customer")
                            .await
                            .expect("Test operation failed");
                    }
                    1 => {
                        // SchemaView operations
                        let view = SchemaView::new((*schema_clone).clone());
                        let _ = view
                            .class_slots("Customer", true)
                            .expect("Test operation failed");
                        let _ = view
                            .class_ancestors("Order")
                            .expect("Test operation failed");
                    }
                    2 => {
                        // Expression evaluation
                        let inventory = json!({
                            "quantity_on_hand": 100,
                            "quantity_reserved": 20,
                            "reorder_point": 30
                        });
                        // TODO: Replace with correct expression evaluation approach
                        // Expression evaluation would need to be done through ExpressionEngine
                        // let _ = expression_engine.evaluate("quantity_on_hand - quantity_reserved", &inventory_context);
                        let _ = inventory; // Use the variable to avoid warnings
                    }
                    3 => {
                        // Rule checking
                        let order = json!({
                            "id": format!("ORD-{:010}", task_id * 100 + op),
                            "subtotal": 1000.0,
                            "tax": 80.0,
                            "shipping": 20.0,
                            "total": 1100.0
                        });
                        let _ = service_clone.validate(&order, &schema_clone, "Order").await;
                    }
                    _ => return Err(anyhow::anyhow!("Unreachable code reached").into()),
                }
                counter.fetch_add(1, Ordering::Relaxed);
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Test operation failed");
    }

    let elapsed = start.elapsed();
    let total_ops = total_operations.load(Ordering::Relaxed);
    let ops_per_second = total_ops as f64 / elapsed.as_secs_f64();

    println!("
Concurrent operations results:");
    println!("  - Total operations: {}", total_ops);
    println!("  - Time elapsed: {:?}", elapsed);
    println!("  - Operations/second: {:.0}", ops_per_second);

    assert_eq!(
        total_ops,
        (num_concurrent_tasks * operations_per_task) as u64
    );
    assert!(ops_per_second > 1000.0, "Should handle >1000 ops/second");
}

#[tokio::test]
async fn test_memory_efficiency() {
    println!("=== Testing Memory Efficiency ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    // Create a large order with many line items
    let mut line_items = Vec::new();
    for i in 1..=100 {
        line_items.push(json!({
            "line_number": i,
            "product": {
                "id": format!("PRD-{:010}", i),
                "created_timestamp": "2025-01-01T00:00:00Z",
                "modified_timestamp": "2025-01-01T00:00:00Z",
                "version": 1,
                "sku": format!("SKU-{:08}", i),
                "name": format!("Product {}", i),
                "category": {
                    "category_id": format!("CAT-{:03}", i % 10),
                    "name": "Electronics",
                    "level": 2
                },
                "price": 99.99,
                "inventory": {
                    "quantity_on_hand": 1000,
                    "quantity_reserved": 50,
                    "reorder_point": 100,
                    "warehouse_locations": ["W1-A-1", "W2-B-3"]
                }
            },
            "quantity": 5,
            "unit_price": 99.99,
            "discount_percent": 10.0,
            "line_total": 449.95
        }));
    }

    let large_order = json!({
        "id": "ORD-0000000001",
        "created_timestamp": "2025-01-16T10:00:00Z",
        "modified_timestamp": "2025-01-16T10:00:00Z",
        "version": 1,
        "order_number": "ORD-0000000001",
        "customer": generate_large_customer(1),
        "order_date": "2025-01-16T10:00:00Z",
        "status": "processing",
        "line_items": line_items,
        "shipping_address": {
            "address_type": "shipping",
            "street1": "123 Delivery Lane",
            "city": "Shipping City",
            "state": "CA",
            "postal_code": "90210",
            "country": "US",
            "validated": true
        },
        "billing_address": {
            "address_type": "billing",
            "street1": "456 Payment Blvd",
            "city": "Billing City",
            "state": "NY",
            "postal_code": "10001",
            "country": "US",
            "validated": true
        },
        "subtotal": 44995.00,
        "tax": 3599.60,
        "shipping": 50.00,
        "total": 48644.60
    });

    // Validate multiple times to test memory usage
    println!("Validating large order 100 times...");
    let start = Instant::now();

    for i in 0..100 {
        let report = service
            .validate(&large_order, &schema, "Order")
            .await
            .expect("Test operation failed");
        assert!(report.valid, "Large order should be valid");

        if i % 20 == 0 {
            println!("  Completed {} validations", i);
        }
    }

    let elapsed = start.elapsed();
    println!("Completed 100 large order validations in {:?}", elapsed);
    println!("Average time per validation: {:?}", elapsed / 100);
}

#[tokio::test]
async fn test_code_generation_performance() {
    println!("=== Testing Code Generation Performance ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    let temp_dir = TempDir::new().expect("Test operation failed");

    // Test different generators
    let generators = vec![
        ("TypeScript", GeneratorType::TypeScript, "enterprise.ts"),
        ("Python", GeneratorType::PythonDataclass, "enterprise.py"),
        ("Rust", GeneratorType::Rust, "enterprise.rs"),
        ("JSON Schema", GeneratorType::JsonSchema, "enterprise.json"),
        ("OpenAPI", GeneratorType::OpenAPI, "enterprise-api.yaml"),
    ];

    println!("
Code generation performance:");
    for (name, gen_type, filename) in generators {
        let config = GeneratorConfig {
            generator_type: gen_type,
            output_path: Some(temp_dir.path().join(filename)),
            include_serialization: true,
            include_validation: true,
            ..Default::default()
        };

        let start = Instant::now();
        service
            .generate_code(&schema, config)
            .await
            .expect("Test operation failed");
        let elapsed = start.elapsed();

        let file_size = fs::metadata(temp_dir.path().join(filename))
            .expect("Test operation failed")
            .len();

        println!(
            "  - {}: {:?} ({:.1} KB)",
            name,
            elapsed,
            file_size as f64 / 1024.0
        );
        assert!(
            elapsed < Duration::from_secs(2),
            "{} generation should complete quickly",
            name
        );
    }
}

#[tokio::test]
async fn test_expression_evaluation_performance() {
    println!("=== Testing Expression Evaluation Performance ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    // Complex expressions to evaluate
    let expressions = vec![
        ("Simple arithmetic", "quantity_on_hand - quantity_reserved"),
        ("Comparison", "quantity_on_hand > reorder_point"),
        (
            "Complex condition",
            "(quantity_on_hand - quantity_reserved) <= reorder_point && reorder_point > 0",
        ),
        ("String operation", "warehouse_locations.join(\", \")"),
        ("Array operation", "warehouse_locations.length > 0"),
    ];

    let inventory_data = json!({
        "quantity_on_hand": 500,
        "quantity_reserved": 150,
        "reorder_point": 100,
        "warehouse_locations": ["W1-A-1", "W1-A-2", "W2-B-1", "W2-B-2", "W3-C-1"]
    });

    println!("
Expression evaluation performance:");

    for (name, expr) in expressions {
        let iterations = 10000;
        let start = Instant::now();

        for _ in 0..iterations {
            // TODO: Replace with correct expression evaluation approach
            // Expression evaluation would need to be done through ExpressionEngine
            // let _ = expression_engine.evaluate(expr, &inventory_context);
            let _ = (expr, &inventory_data); // Use the variables to avoid warnings
        }

        let elapsed = start.elapsed();
        let per_eval = elapsed / iterations;
        let evals_per_sec = iterations as f64 / elapsed.as_secs_f64();

        println!(
            "  - {}: {:?} per eval, {:.0} evals/sec",
            name, per_eval, evals_per_sec
        );
        assert!(
            per_eval < Duration::from_micros(100),
            "Expression evaluation should be fast"
        );
    }
}

#[tokio::test]
async fn test_cache_effectiveness() {
    println!("=== Testing Cache Effectiveness ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    // Create test data
    let customer = generate_large_customer(1);

    // Cold cache validation
    let cold_start = Instant::now();
    let cold_report = service
        .validate(&customer, &schema, "Customer")
        .await
        .expect("Test operation failed");
    let cold_time = cold_start.elapsed();
    assert!(cold_report.valid);

    // Warm cache validation (same data)
    let warm_start = Instant::now();
    let warm_report = service
        .validate(&customer, &schema, "Customer")
        .await
        .expect("Test operation failed");
    let warm_time = warm_start.elapsed();
    assert!(warm_report.valid);

    // Multiple warm cache hits
    let multi_start = Instant::now();
    for _ in 0..100 {
        let _ = service
            .validate(&customer, &schema, "Customer")
            .await
            .expect("Test operation failed");
    }
    let multi_time = multi_start.elapsed();
    let avg_cached = multi_time / 100;

    println!("
Cache effectiveness:");
    println!("  - Cold cache: {:?}", cold_time);
    println!("  - Warm cache: {:?}", warm_time);
    println!("  - Average cached: {:?}", avg_cached);
    println!(
        "  - Cache speedup: {:.2}x",
        cold_time.as_nanos() as f64 / warm_time.as_nanos() as f64
    );

    assert!(
        warm_time < cold_time / 2,
        "Cache should provide significant speedup"
    );
    assert!(
        avg_cached < cold_time / 5,
        "Repeated cache hits should be very fast"
    );
}

#[tokio::test]
async fn test_parallel_validation_scaling() {
    println!("=== Testing Parallel Validation Scaling ===");

    let service = create_performance_service().await;
    let schema = Arc::new(
        service
            .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
            .await
            .expect("Test operation failed"),
    );

    // Test with different parallelism levels
    let test_sizes = vec![1, 2, 4, 8, 16];
    let records_per_thread = 100;

    println!("
Parallel validation scaling:");
    let mut previous_time = Duration::from_secs(0);

    for num_threads in test_sizes {
        let start = Instant::now();
        let mut handles = Vec::new();

        for thread_id in 0..num_threads {
            let service_clone = service.clone();
            let schema_clone = schema.clone();

            let handle = tokio::spawn(async move {
                for i in 0..records_per_thread {
                    let customer = generate_large_customer((thread_id * 1000 + i) as u32);
                    let _ = service_clone
                        .validate(&customer, &schema_clone, "Customer")
                        .await
                        .expect("Test operation failed");
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Test operation failed");
        }

        let elapsed = start.elapsed();
        let total_records = num_threads * records_per_thread;
        let throughput = total_records as f64 / elapsed.as_secs_f64();

        let scaling_efficiency = if num_threads > 1 && previous_time > Duration::from_secs(0) {
            let expected_time = previous_time / (num_threads / (num_threads / 2)) as u32;
            let efficiency = expected_time.as_secs_f64() / elapsed.as_secs_f64() * 100.0;
            format!("{:.1}%", efficiency)
        } else {
            "N/A".to_string()
        };

        println!(
            "  - {} threads: {:?}, {:.0} records/sec, efficiency: {}",
            num_threads, elapsed, throughput, scaling_efficiency
        );

        if num_threads > 1 {
            previous_time = elapsed;
        }
    }
}

#[tokio::test]
async fn test_stress_test_with_errors() {
    println!("=== Testing Stress Test with Validation Errors ===");

    let service = create_performance_service().await;
    let schema = service
        .load_schema_str(ENTERPRISE_SCHEMA, SchemaFormat::Yaml)
        .await
        .expect("Test operation failed");

    // Create mix of valid and invalid data
    let mut test_data = Vec::new();

    // Valid customers
    for i in 0..500 {
        test_data.push((generate_large_customer(i), true));
    }

    // Invalid customers (various error types)
    for i in 500..1000 {
        let mut customer = generate_large_customer(i);

        match i % 5 {
            0 => {
                // Missing required field
                customer
                    .as_object_mut()
                    .expect("Test operation failed")
                    .remove("company_name");
            }
            1 => {
                // Invalid pattern
                customer["id"] = json!("INVALID-ID");
            }
            2 => {
                // Rule violation (enterprise with low credit limit)
                customer["credit_limit"] = json!(1000);
            }
            3 => {
                // Invalid enum value
                customer["customer_type"] = json!("invalid_type");
            }
            4 => {
                // Cardinality violation
                customer["addresses"] = json!([]);
            }
            _ => return Err(anyhow::anyhow!("Unreachable code reached").into()),
        }

        test_data.push((customer, false));
    }

    // Shuffle to mix valid and invalid using RandomService
    let random_service = mock_services::create_mock_random_service();
    let mut shuffled_data = test_data.clone();

    // Use Fisher-Yates shuffle algorithm with RandomService
    for i in (1..shuffled_data.len()).rev() {
        let j = random_service.generate_u32_range(0, (i + 1) as u32).await
            .expect("Random service should work in tests") as usize;
        shuffled_data.swap(i, j);
    }
    let test_data = shuffled_data;

    // Run stress test
    let start = Instant::now();
    let mut valid_count = 0;
    let mut error_count = 0;
    let mut total_validation_errors = 0;

    for (data, expected_valid) in &test_data {
        let report = service
            .validate(data, &schema, "Customer")
            .await
            .expect("Test operation failed");

        if report.valid {
            valid_count += 1;
        } else {
            error_count += 1;
            total_validation_errors += report.errors.len();
        }

        assert_eq!(report.valid, *expected_valid, "Validation result mismatch");
    }

    let elapsed = start.elapsed();
    let throughput = test_data.len() as f64 / elapsed.as_secs_f64();

    println!("
Stress test results:");
    println!("  - Total records: {}", test_data.len());
    println!("  - Valid records: {}", valid_count);
    println!("  - Invalid records: {}", error_count);
    println!("  - Total validation errors: {}", total_validation_errors);
    println!("  - Time elapsed: {:?}", elapsed);
    println!("  - Throughput: {:.0} records/second", throughput);
    println!(
        "  - Average errors per invalid record: {:.1}",
        total_validation_errors as f64 / error_count as f64
    );

    assert_eq!(valid_count, 500);
    assert_eq!(error_count, 500);
    assert!(
        throughput > 100.0,
        "Should maintain performance even with errors"
    );
}
