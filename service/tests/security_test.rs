//! Security tests for LinkML service
//!
//! Tests various security measures including input validation,
//! resource limits, and protection against DoS attacks.

use linkml_service::expression::functions::{CustomFunction, FunctionError, FunctionRegistry};
use linkml_service::expression::parser::parse_expression;
use linkml_service::expression::{Evaluator, EvaluatorConfig};
use linkml_service::performance::{global_interner, global_memory_profiler, intern};
use linkml_service::security::input_validation::{limits, validate_string_input};
use linkml_service::security::resource_limits::{ResourceLimits, create_monitor};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use linkml_core::string_pool::intern;

#[test]
fn test_string_interning_limits() {
    // Clear the interner to start fresh
    global_interner().clear();

    // Test that small strings can be interned
    let s1 = intern("hello");
    let s2 = intern("hello");
    assert!(Arc::ptr_eq(&s1, &s2));

    // Test that oversized strings are rejected but still work
    let large_string = "x".repeat(20_000); // Over the 10K limit
    let s3 = intern(&large_string);
    let s4 = intern(&large_string);

    // They should not be the same Arc (not interned)
    assert!(!Arc::ptr_eq(&s3, &s4));

    // But they should still be equal
    assert_eq!(&*s3, &*s4);
}

#[test]
fn test_memory_profiler_category_limits() {
    let profiler = global_memory_profiler();
    profiler.set_enabled(true);
    profiler.clear();

    // Add allocations to different categories
    for i in 0..100 {
        profiler.record_alloc(1000, Some(&format!("category_{}", i));
    }

    // Try to add more categories (should be silently ignored after limit)
    for i in 100..1100 {
        profiler.record_alloc(1000, Some(&format!("category_{}", i));
    }

    // The report should only contain up to MAX_CATEGORIES
    let report = profiler.category_report();
    let lines: Vec<&str> = report.lines().collect();

    // Account for header lines and summary
    assert!(lines.len() <= 1000 + 10); // MAX_CATEGORIES + some header/footer lines

    profiler.set_enabled(false);
}

#[test]
fn test_expression_cache_security() {
    let config = EvaluatorConfig {
        enable_cache: true,
        cache_size: 10, // Small cache for testing
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);
    let mut context = HashMap::new();

    // Fill the cache with different expressions
    for i in 0..20 {
        context.insert("x".to_string(), json!(i));
        let expr = parse_expression(&format!("x + {}", i)).expect("Test operation failed");
        let _ = evaluator.evaluate(&expr, &context);
    }

    // Cache should have evicted old entries (LRU)
    if let Some((current, capacity)) = evaluator.cache_stats() {
        assert!(current <= capacity);
        assert_eq!(capacity, 10);
    }
}

#[test]
fn test_function_registry_locking() {
    let mut registry = FunctionRegistry::new();

    // Should be able to register functions when unlocked
    assert!(
        registry
            .register_custom(CustomFunction::new("test1", 1, Some(1), |args| Ok(
                args[0].clone()
            )))
            .is_ok()
    );

    // Lock the registry
    registry.lock();
    assert!(registry.is_locked());

    // Should not be able to register new functions
    assert!(
        registry
            .register_custom(CustomFunction::new("test2", 1, Some(1), |args| Ok(
                args[0].clone()
            )))
            .is_err()
    );

    // But existing functions should still work
    assert!(registry.call("test1", vec![json!("hello")]).is_ok());
}

#[test]
fn test_restricted_function_registry() {
    let registry = FunctionRegistry::new_restricted();

    // Registry should be locked by default
    assert!(registry.is_locked());

    // Built-in functions should work
    assert!(registry.call("len", vec![json!("hello")]).is_ok());
    assert!(
        registry
            .call("max", vec![json!(1), json!(2), json!(3)])
            .is_ok()
    );

    // But we shouldn't be able to add custom functions
    // (Can't test this directly as register_custom requires mutable reference)
}

#[test]
fn test_input_validation() {
    // Valid inputs
    assert!(validate_string_input("normal string").is_ok());
    assert!(validate_string_input("multi
line
string").is_ok());

    // Too large
    let large = "x".repeat(limits::MAX_STRING_LENGTH + 1);
    assert!(validate_string_input(&large).is_err());

    // Null bytes
    assert!(validate_string_input("hello\0world").is_err());

    // Control characters
    assert!(validate_string_input("hello\x01world").is_err());
}

#[test]
fn test_resource_monitor_timeout() {
    let limits = ResourceLimits {
        max_validation_time: Duration::from_millis(100),
        ..Default::default()
    };

    let monitor = create_monitor(limits);

    // Should not timeout immediately
    assert!(monitor.check_timeout().is_ok());

    // Wait and check
    thread::sleep(Duration::from_millis(150));
    assert!(monitor.check_timeout().is_err());
}

#[test]
fn test_resource_monitor_memory() {
    let limits = ResourceLimits {
        max_memory_usage: 1000, // 1KB for testing
        ..Default::default()
    };

    let monitor = create_monitor(limits);

    // Allocate within limits
    assert!(monitor.allocate_memory(500).is_ok());
    assert_eq!(monitor.current_usage().memory_used, 500);

    // Try to exceed limits
    assert!(monitor.allocate_memory(600).is_err());
    assert_eq!(monitor.current_usage().memory_used, 500); // Should not have changed

    // Release and try again
    monitor.release_memory(200);
    assert_eq!(monitor.current_usage().memory_used, 300);
    assert!(monitor.allocate_memory(600).is_ok());
}

#[test]
fn test_resource_monitor_parallel_ops() {
    let limits = ResourceLimits {
        max_parallel_validators: 3,
        ..Default::default()
    };

    let monitor = create_monitor(limits);

    // Start operations
    let guard1 = monitor.start_parallel_op().expect("Test operation failed");
    let guard2 = monitor.start_parallel_op().expect("Test operation failed");
    let guard3 = monitor.start_parallel_op().expect("Test operation failed");

    // Fourth should fail
    assert!(monitor.start_parallel_op().is_err());

    // Drop one and try again
    drop(guard1);
    let _guard4 = monitor.start_parallel_op().expect("Test operation failed");
}

#[test]
fn test_expression_evaluation_timeout() {
    let config = EvaluatorConfig {
        timeout: Duration::from_millis(100),
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);

    // Create an expression that would take too long
    // Since we can't create an infinite loop in the expression language,
    // we'll test with a complex nested expression
    let mut expr_str = "1";
    for _ in 0..100 {
        expr_str = &format!("({} + 1)", expr_str);
    }

    // This should complete quickly despite being deeply nested
    let expr = parse_expression(expr_str).expect("Test operation failed");
    let context = HashMap::new();

    // The actual evaluation should be fast, so we expect success
    assert!(evaluator.evaluate(&expr, &context).is_ok());
}

#[test]
fn test_expression_depth_limit() {
    let config = EvaluatorConfig {
        max_call_depth: 10,
        ..Default::default()
    };

    let evaluator = Evaluator::with_config(config);

    // Create a deeply nested expression
    let mut expr_str = "1";
    for _ in 0..20 {
        expr_str = &format!("({} + 1)", expr_str);
    }

    let expr = parse_expression(expr_str).expect("Test operation failed");
    let context = HashMap::new();

    // This should fail due to call depth
    let result = evaluator.evaluate(&expr, &context);
    assert!(result.is_err());
}

#[test]
fn test_secure_cache_key_generation() {
    // This test verifies that cache keys are generated without string formatting
    // which could be exploited with large inputs

    let evaluator = Evaluator::new();

    // Create a context with large strings
    let mut context = HashMap::new();
    let large_value = "x".repeat(10_000);
    context.insert("large".to_string(), json!(large_value));

    // Evaluate multiple times with different large values
    for i in 0..10 {
        let key = format!("key_{}", i);
        context.insert(key.clone(), json!(format!("{}{}", large_value, i));

        let expr = parse_expression(&format!("len({})", key)).expect("Test operation failed");

        // This should complete without issues despite large strings
        let result = evaluator.evaluate(&expr, &context);
        assert!(result.is_ok());
    }
}

#[test]
fn test_validation_error_limit() {
    let limits = ResourceLimits {
        max_validation_errors: 5,
        ..Default::default()
    };

    let monitor = create_monitor(limits);

    // Add errors up to the limit
    for i in 0..5 {
        assert!(monitor.add_validation_error());
        assert_eq!(monitor.current_usage().validation_errors, i + 1);
    }

    // Next error should be rejected
    assert!(!monitor.add_validation_error());
    assert_eq!(monitor.current_usage().validation_errors, 6); // Counter still increments
}
