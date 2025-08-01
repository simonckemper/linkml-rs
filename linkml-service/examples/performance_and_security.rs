//! Example demonstrating performance optimizations and security features
//!
//! This example shows how to use the LinkML service with:
//! - Performance profiling and optimization
//! - Security input validation
//! - Resource limiting
//! - Safe expression evaluation

use linkml_service::{
    expression::{Evaluator, EvaluatorConfig, parse_expression},
    expression::functions::{FunctionRegistry, CustomFunction},
    performance::{global_profiler, global_memory_profiler, intern, MemoryScope},
    security::{
        input_validation::{validate_string_input, validate_identifier},
        resource_limits::{ResourceLimits, create_monitor},
    },
    validator::{Validator, ValidatorBuilder},
    parser::yaml_parser::YamlParser,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("LinkML Performance and Security Example\n");
    
    // 1. Enable performance profiling
    println!("1. Enabling performance profiling...");
    let profiler = global_profiler();
    profiler.set_enabled(true);
    
    let memory_profiler = global_memory_profiler();
    memory_profiler.set_enabled(true);
    
    // 2. Demonstrate string interning for performance
    println!("\n2. String interning optimization:");
    {
        let _scope = MemoryScope::new("string_interning");
        
        // Intern common strings
        let types = vec!["string", "integer", "boolean", "string", "integer"];
        let interned: Vec<Arc<str>> = types.iter().map(|s| intern(s)).collect();
        
        // Check that duplicates share memory
        println!("   - Interned {} strings", types.len());
        println!("   - First and fourth strings share memory: {}", 
                 Arc::ptr_eq(&interned[0], &interned[3]));
    }
    
    // 3. Secure input validation
    println!("\n3. Input validation examples:");
    
    // Valid inputs
    let valid_string = "This is a normal string with\nnewlines";
    match validate_string_input(valid_string) {
        Ok(()) => println!("   ✓ Valid string accepted"),
        Err(e) => println!("   ✗ String rejected: {}", e),
    }
    
    // Invalid input (control characters)
    let invalid_string = "Hello\x01World";
    match validate_string_input(invalid_string) {
        Ok(()) => println!("   ✗ Invalid string accepted (unexpected)"),
        Err(e) => println!("   ✓ Invalid string rejected: {}", e),
    }
    
    // Identifier validation
    let valid_id = "my_class_name";
    match validate_identifier(valid_id) {
        Ok(()) => println!("   ✓ Valid identifier accepted"),
        Err(e) => println!("   ✗ Identifier rejected: {}", e),
    }
    
    // 4. Resource-limited validation
    println!("\n4. Resource-limited schema validation:");
    
    let schema_yaml = r#"
    id: https://example.org/performance-demo
    name: PerformanceDemo
    
    classes:
      Person:
        slots:
          - name
          - age
          - email
    
    slots:
      name:
        range: string
        required: true
        pattern: "^[A-Za-z ]+$"
      
      age:
        range: integer
        minimum_value: 0
        maximum_value: 150
      
      email:
        range: string
        pattern: "^[\\w.-]+@[\\w.-]+\\.\\w+$"
    "#;
    
    // Create resource monitor
    let limits = ResourceLimits {
        max_validation_time: Duration::from_secs(5),
        max_memory_usage: 100_000_000, // 100MB
        max_validation_errors: 100,
        ..Default::default()
    };
    let monitor = create_monitor(limits);
    
    // Parse schema with profiling
    let schema = profiler.time("parse_schema", || {
        YamlParser::parse_string(schema_yaml)
    })?;
    
    // Build validator
    let validator = profiler.time("build_validator", || {
        ValidatorBuilder::new()
            .with_schema(schema)
            .with_strict_mode(true)
            .build()
    })?;
    
    // Validate data with resource monitoring
    let test_data = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john.doe@example.com"
    });
    
    monitor.check_timeout()?;
    let validation_result = profiler.time("validate_data", || {
        validator.validate(&test_data, Some("Person"))
    });
    
    match validation_result {
        Ok(report) => {
            println!("   ✓ Validation successful");
            println!("   - Validated in: {:?}", profiler.get_stats("validate_data")
                .map(|s| s.mean_duration()).unwrap_or_default());
        }
        Err(e) => {
            println!("   ✗ Validation failed: {}", e);
        }
    }
    
    // 5. Secure expression evaluation
    println!("\n5. Secure expression evaluation:");
    
    // Create a restricted function registry (no custom functions allowed)
    let registry = FunctionRegistry::new_restricted();
    let evaluator = Evaluator::with_functions(registry);
    
    // Safe expression evaluation with caching
    let expr = parse_expression("age >= 18 and age < 65")?;
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(30));
    
    let result = profiler.time("evaluate_expression", || {
        evaluator.evaluate(&expr, &context)
    })?;
    
    println!("   - Expression result: {}", result);
    
    // Try with cached result (should be faster)
    let result2 = profiler.time("evaluate_expression_cached", || {
        evaluator.evaluate(&expr, &context)
    })?;
    
    println!("   - Cached result: {}", result2);
    
    // 6. Performance report
    println!("\n6. Performance Report:");
    println!("{}", profiler.report());
    
    // 7. Memory usage report
    println!("\n7. Memory Usage Report:");
    println!("{}", memory_profiler.category_report());
    
    // 8. Resource usage summary
    println!("\n8. Resource Usage Summary:");
    let usage = monitor.current_usage();
    println!("{}", usage.format_summary());
    
    // Cleanup
    profiler.set_enabled(false);
    memory_profiler.set_enabled(false);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_example_runs() {
        // Run the example to ensure it doesn't panic
        main().expect("Example should run without errors");
    }
}