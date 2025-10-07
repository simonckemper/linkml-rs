//! Advanced validation API demonstration for LinkML service
//!
//! This example demonstrates the LinkML validation API including:
//! - Custom validation rules
//! - Complex constraints
//! - Cross-field validation
//! - Performance optimization
//! - Error recovery strategies
//!
//! NOTE: This is an API demonstration. In a real RootReal application,
//! you would initialize the service with actual implementations following
//! the pattern in docs/architecture/dyn-compatibility-guidelines.md

use linkml_core::{config::LinkMLConfig, prelude::*};
use serde_json::json;

/// This example demonstrates the LinkML validation API
///
/// In production, you would:
/// 1. Initialize all RootReal services at application startup
/// 2. Create the LinkML service with those dependencies
/// 3. Use the service throughout your application
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Advanced Validation API Demonstration");
    println!(
        "==========================================
"
    );

    // Show configuration options
    demonstrate_configuration()?;

    // Show schema definition
    demonstrate_schema();

    // Show validation examples
    demonstrate_validation()?;

    // Show error handling
    demonstrate_error_handling()?;

    // Show performance patterns
    demonstrate_performance_patterns()?;

    Ok(())
}

fn demonstrate_configuration() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "1. Configuration Options:
"
    );

    // Show how to configure the service for different use cases
    let strict_config = LinkMLConfig {
        validation: linkml_core::config::ValidationConfig {
            strict_mode: true,
            enable_patterns: true,
            enable_instances: true,
            max_errors: 100,
            timeout: std::time::Duration::from_secs(30),
            enable_coercion: false, // No type coercion in strict mode
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

    println!(
        "Strict validation config: {:#?}
",
        strict_config.validation
    );

    let lenient_config = LinkMLConfig {
        validation: linkml_core::config::ValidationConfig {
            strict_mode: false,
            enable_coercion: true, // Allow type coercion
            max_errors: 10,        // Stop after 10 errors
            ..Default::default()
        },
        ..Default::default()
    };

    println!(
        "Lenient validation config: {:#?}
",
        lenient_config.validation
    );

    Ok(())
}

fn demonstrate_schema() {
    println!(
        "
2. Schema Definition with Advanced Constraints:
"
    );

    let _schema_yaml = r#"
id: https://example.org/advanced-schema
name: AdvancedSchema
description: Schema with advanced validation rules

classes:
  Person:
    description: Person with complex validation rules
    slots:
      - id
      - name
      - email
      - age
      - employment_status
      - salary
      - supervisor
    rules:
      - description: Employed persons must have a salary
        preconditions:
          slot_conditions:
            employment_status:
              equals_string: employed
        postconditions:
          slot_conditions:
            salary:
              value_presence: PRESENT
              minimum_value: 0

      - description: Minors cannot be supervisors
        preconditions:
          slot_conditions:
            age:
              maximum_value: 17
        postconditions:
          slot_conditions:
            supervisor:
              value_presence: ABSENT

slots:
  id:
    identifier: true
    range: string
    pattern: "^[A-Z]{2}[0-9]{6}$"

  name:
    range: string
    required: true
    pattern: "^[A-Za-z ,.'-]+$"
    minimum_length: 2
    maximum_length: 100

  email:
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[A-Z|a-z]{2,}$"

  age:
    range: integer
    minimum_value: 0
    maximum_value: 150

  employment_status:
    range: EmploymentStatus
    required: true

  salary:
    range: decimal
    minimum_value: 0
    maximum_value: 10000000

  supervisor:
    range: Person

enums:
  EmploymentStatus:
    permissible_values:
      - employed
      - unemployed
      - retired
      - student
"#;

    println!("Schema demonstrates:");
    println!("- Pattern validation (regex)");
    println!("- Range constraints (min/max values)");
    println!("- Cross-field rules (employment → salary)");
    println!("- Conditional constraints (age → supervisor)");
    println!("- Enum validation");
}

fn demonstrate_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
3. Validation Examples:
"
    );

    // Valid person
    let valid_person = json!({
        "id": "US123456",
        "name": "John Doe",
        "email": "john.doe@example.com",
        "age": 30,
        "employment_status": "employed",
        "salary": 75000.00
    });

    println!("Valid person data:");
    println!("{}", serde_json::to_string_pretty(&valid_person)?);

    // Invalid: employed but no salary
    let invalid_employed = json!({
        "id": "UK789012",
        "name": "Jane Smith",
        "email": "jane@example.com",
        "age": 25,
        "employment_status": "employed"
        // Missing salary!
    });

    println!(
        "
Invalid - employed without salary:"
    );
    println!("{}", serde_json::to_string_pretty(&invalid_employed)?);
    println!("Expected error: Rule violation - employed persons must have salary");

    // Invalid: minor as supervisor
    let invalid_supervisor = json!({
        "id": "CA345678",
        "name": "Young Manager",
        "age": 16,
        "employment_status": "student",
        "supervisor": {"id": "US123456"}
    });

    println!(
        "
Invalid - minor as supervisor:"
    );
    println!("{}", serde_json::to_string_pretty(&invalid_supervisor)?);
    println!("Expected error: Rule violation - minors cannot be supervisors");

    Ok(())
}

fn demonstrate_error_handling() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
4. Error Handling Patterns:
"
    );

    println!("The LinkML service provides detailed error information:");
    println!("- Error path (e.g., 'Person.email')");
    println!("- Error type (pattern_mismatch, range_violation, etc.)");
    println!("- Human-readable message");
    println!("- Suggestion for fixes when available");

    // Example error structure
    let example_errors = vec![
        ValidationError {
            path: Some("Person.id".to_string()),
            message: "Value 'ABC123' does not match pattern '^[A-Z]{2}[0-9]{6}$'".to_string(),
            expected: Some("Format: 2 uppercase letters followed by 6 digits".to_string()),
            actual: Some("ABC123".to_string()),
            severity: Severity::Error,
        },
        ValidationError {
            path: Some("Person.age".to_string()),
            message: "Value 200 exceeds maximum value 150".to_string(),
            expected: Some("0..150".to_string()),
            actual: Some("200".to_string()),
            severity: Severity::Error,
        },
    ];

    for error in &example_errors {
        println!(
            "
Error at {}: {}",
            error.path.as_ref()?,
            error.message
        );
        if let Some(expected) = &error.expected {
            println!("  Expected: {}", expected);
        }
    }

    Ok(())
}

fn demonstrate_performance_patterns() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "
5. Performance Optimization Patterns:
"
    );

    println!("a) Parallel Validation:");
    println!("   - Validate multiple documents concurrently");
    println!("   - Use batch validation API for large datasets");
    println!("   - Configure max_concurrent_validations");

    println!(
        "
b) Schema Compilation:"
    );
    println!("   - Enable compilation for frequently used schemas");
    println!("   - Compiled validators are 10-100x faster");
    println!("   - Trade memory for speed with caching");

    println!(
        "
c) Streaming Validation:"
    );
    println!("   - Process large files without loading into memory");
    println!("   - Early termination on first error (fail-fast)");
    println!("   - Progressive validation with partial results");

    // Example: Batch validation pattern
    println!(
        "
Batch validation pattern:"
    );
    println!(
        r#"
// In production code:
let documents = vec![doc1, doc2, doc3, ...];
let results = linkml_service.validate_batch(
    &documents,
    &schema,
    "Person",
    BatchOptions {{
        parallel: true,
        fail_fast: false,
        progress_callback: Some(|completed, total| {{
            println!("Progress: {{}}/{{}}", completed, total);
        }}),
    }}
).await?;
"#
    );

    Ok(())
}

/// Production initialization pattern
///
/// This shows how you would initialize the service in a real application
fn _show_production_pattern() {
    println!(
        "
6. Production Initialization Pattern:
"
    );

    println!(
        r#"
// At application startup (e.g., in main.rs):
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {{
    // 1. Initialize concrete service implementations using factory functions
    let logger = logger_service::wiring::wire_logger_with_config(timestamp.clone(), ).await?;
    let timestamp = timestamp_service::wiring::wire_timestamp();
    let task_manager = task_management_service::factory::create_standard_task_management_service().await?;
    let error_handler = error_handling_service::factory::create_standard_error_handling_service(
        logger.clone(),
        timestamp.clone(),
    ).await?;
    let config_service = configuration_service::factory::create_standard_configuration_service().await?;
    let cache = cache_service::factory::create_valkey_cache_service().await?;
    let monitor = monitoring_service::factory::create_standard_monitoring_service().await?;

    // 2. Create LinkML service with all dependencies
    let linkml = create_linkml_service_with_config(
        config,
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    ).await?;

    // 3. Use throughout application
    run_application(linkml).await
}}
"#
    );

    println!(
        "
See docs/architecture/dyn-compatibility-guidelines.md for details."
    );
}
