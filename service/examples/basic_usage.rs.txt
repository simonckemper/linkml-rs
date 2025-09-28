//! Basic usage example for LinkML validation service
//!
//! This example demonstrates how to:
//! - Load a LinkML schema
//! - Validate data against the schema
//! - Handle validation errors
//! - Use different schema formats
//!
//! NOTE: This example shows how to properly initialize LinkML service
//! using RootReal's factory pattern for dependency injection.

use linkml_service::{create_linkml_service, LinkMLService};
use linkml_core::{prelude::*, error::Result};
use serde_json::json;
use std::sync::Arc;

// RootReal service dependencies - import factory functions only
use logger_service::factory as logger_factory;
use timestamp_service::factory as timestamp_factory;
use task_management_service::factory as task_factory;
use error_handling_service::factory as error_factory;
use configuration_service::factory as config_factory;
use cache_service::factory as cache_factory;
use monitoring_service::factory as monitoring_factory;
use random_service::factory as random_factory;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize RootReal services using proper factory functions
    // This follows RootReal's dependency injection standards
    let logger = logger_factory::create_standard_logger().await?;
    let timestamp = timestamp_factory::create_timestamp_service();
    let task_manager = task_factory::create_task_manager().await?;
    let error_handler = error_factory::create_error_handler(
        logger.clone(),
        timestamp.clone(),
    ).await?;
    let config_service = config_factory::create_configuration_service().await?;

    // Initialize remaining services using factory functions
    let cache = cache_factory::create_valkey_cache(
        logger.clone(),
        task_manager.clone(),
    ).await?;

    let monitor = monitoring_factory::create_monitoring_service(
        logger.clone(),
        timestamp.clone(),
    ).await?;

    let random_service = random_factory::create_random_service().await?;

    // Create mock DBMS and Timeout services for example
    let dbms_service = Arc::new(test_utils::MockDBMSService::new());
    let timeout_service = Arc::new(test_utils::MockTimeoutService::new());

    // Initialize the LinkML service with all required dependencies
    let service = create_linkml_service(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        dbms_service,
        timeout_service,
        cache,
        monitor,
        random_service,
    ).await?;

    println!("LinkML Basic Usage Example");
    println!("=========================\n");

    // Example 1: Load a simple schema from YAML
    println!("1. Loading a simple schema from YAML:");
    let schema_yaml = r#"
id: https://example.org/person-schema
name: PersonSchema
description: A simple schema for person data

classes:
  Person:
    description: A person with basic information
    slots:
      - id
      - name
      - age
      - email
      - occupation

slots:
  id:
    description: Unique identifier
    identifier: true
    range: string
    required: true

  name:
    description: Full name of the person
    range: string
    required: true
    pattern: "^[A-Za-z ]+$"

  age:
    description: Age in years
    range: integer
    minimum_value: 0
    maximum_value: 150

  email:
    description: Email address
    range: string
    pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"

  occupation:
    description: Current occupation
    range: OccupationType

enums:
  OccupationType:
    description: Types of occupations
    permissible_values:
      engineer:
        description: Software or hardware engineer
      doctor:
        description: Medical professional
      teacher:
        description: Education professional
      artist:
        description: Creative professional
      other:
        description: Other occupation
"#;

    let schema = service.load_schema_str(schema_yaml, SchemaFormat::Yaml).await?;
    println!("✓ Schema loaded successfully: {}\n", schema.name);

    // Example 2: Validate valid data
    println!("2. Validating valid person data:");
    let valid_person = json!({
        "id": "person001",
        "name": "John Smith",
        "age": 30,
        "email": "john.smith@example.com",
        "occupation": "engineer"
    });

    let report = service.validate(&valid_person, &schema, "Person").await?;
    println!("   Validation result: {}", if report.valid { "PASSED ✓" } else { "FAILED ✗" });
    println!("   Errors: {}, Warnings: {}\n", report.errors.len(), report.warnings.len());

    // Example 3: Validate invalid data (multiple issues)
    println!("3. Validating invalid person data:");
    let invalid_person = json!({
        "id": "person002",
        "name": "Jane123",  // Contains numbers (invalid pattern)
        "age": 200,         // Exceeds maximum value
        "email": "invalid-email",  // Invalid email format
        "occupation": "astronaut"  // Not in enum
    });

    let report = service.validate(&invalid_person, &schema, "Person").await?;
    println!("   Validation result: {}", if report.valid { "PASSED ✓" } else { "FAILED ✗" });
    println!("   Issues found:");
    for error in &report.errors {
        println!("   - {}: {}",
            error.path.as_deref().unwrap_or(""),
            error.message
        );
    }
    println!();

    // Example 4: Working with complex schemas
    println!("4. Working with inheritance and mixins:");
    let complex_schema_yaml = r#"
id: https://example.org/employee-schema
name: EmployeeSchema
imports:
  - linkml:types

classes:
  NamedThing:
    abstract: true
    slots:
      - id
      - name

  ContactInfo:
    mixin: true
    slots:
      - email
      - phone

  Employee:
    is_a: NamedThing
    mixins:
      - ContactInfo
    slots:
      - employee_id
      - department
      - hire_date

  Manager:
    is_a: Employee
    slots:
      - team_size
      - budget

slots:
  id:
    identifier: true
    range: string

  name:
    range: string
    required: true

  email:
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[A-Z|a-z]{2,}$"

  phone:
    range: string
    pattern: "^\\+?[0-9]{10,}$"

  employee_id:
    range: string
    pattern: "^EMP[0-9]{6}$"
    required: true

  department:
    range: Department

  hire_date:
    range: date

  team_size:
    range: integer
    minimum_value: 1

  budget:
    range: float
    minimum_value: 0

enums:
  Department:
    permissible_values:
      engineering: {}
      sales: {}
      marketing: {}
      hr: {}
      finance: {}
"#;

    let complex_schema = service.load_schema_str(complex_schema_yaml, SchemaFormat::Yaml).await?;
    println!("✓ Complex schema loaded: {}", complex_schema.name);

    let manager = json!({
        "id": "mgr001",
        "name": "Alice Johnson",
        "email": "alice.johnson@company.com",
        "phone": "+1234567890",
        "employee_id": "EMP000123",
        "department": "engineering",
        "hire_date": "2020-01-15",
        "team_size": 12,
        "budget": 1500000.00
    });

    let report = service.validate(&manager, &complex_schema, "Manager").await?;
    println!("   Manager validation: {}\n", if report.valid { "PASSED ✓" } else { "FAILED ✗" });

    // Example 5: Schema introspection
    println!("5. Schema introspection:");
    println!("   Classes in schema:");
    for (name, class) in &complex_schema.classes {
        println!("   - {}{}",
            name,
            if class.abstract_.unwrap_or(false) { " (abstract)" } else { "" }
        );
    }

    println!("\n   Slots in Employee class:");
    if let Some(employee_class) = complex_schema.classes.get("Employee") {
        // Get all slots including inherited ones
        let mut all_slots = employee_class.slots.clone();

        // Add slots from parent class
        if let Some(parent) = &employee_class.is_a {
            if let Some(parent_class) = complex_schema.classes.get(parent) {
                all_slots.extend(parent_class.slots.clone());
            }
        }

        // Add slots from mixins
        for mixin in &employee_class.mixins {
            if let Some(mixin_class) = complex_schema.classes.get(mixin) {
                all_slots.extend(mixin_class.slots.clone());
            }
        }

        for slot_name in &all_slots {
            if let Some(slot) = complex_schema.slots.get(slot_name) {
                println!("   - {}: {} {}",
                    slot_name,
                    slot.range.as_deref().unwrap_or("string"),
                    if slot.required.unwrap_or(false) { "(required)" } else { "" }
                );
            }
        }
    }

    // Example 6: Batch validation
    println!("\n6. Batch validation of multiple records:");
    let people = vec![
        json!({
            "id": "p1",
            "name": "Valid Person",
            "age": 25,
            "email": "valid@example.com"
        }),
        json!({
            "id": "p2",
            "name": "Another Person",
            "age": -5,  // Invalid age
            "email": "another@example.com"
        }),
        json!({
            "id": "p3",
            "name": "Third Person",
            "age": 40,
            "email": "not-an-email"  // Invalid email
        }),
    ];

    let mut valid_count = 0;
    let mut total_errors = 0;

    for (i, person) in people.iter().enumerate() {
        let report = service.validate(person, &schema, "Person").await?;
        if report.valid {
            valid_count += 1;
        } else {
            total_errors += report.errors.len();
        }
        println!("   Person {}: {}",
            i + 1,
            if report.valid { "✓" } else { "✗" }
        );
    }

    println!("   Summary: {}/{} valid, {} total errors\n",
        valid_count, people.len(), total_errors);

    // Example 7: Error details and recovery suggestions
    println!("7. Detailed error analysis:");
    let problematic_data = json!({
        "name": "Test User",
        "age": "thirty",  // Should be integer
        "email": "test@"  // Incomplete email
    });

    let report = service.validate(&problematic_data, &schema, "Person").await?;
    for error in &report.errors {
        println!("   Error: {}", error.message);
        println!("   Path: {}", error.path.as_deref().unwrap_or("root"));
        if let Some(expected) = &error.expected {
            println!("   Expected: {}", expected);
        }
        if let Some(actual) = &error.actual {
            println!("   Actual: {}", actual);
        }
        println!();
    }

    println!("Example completed successfully!");

    Ok(())
}
