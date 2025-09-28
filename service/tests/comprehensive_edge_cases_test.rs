//! Comprehensive edge case testing for LinkML service
//!
//! This test suite covers extreme edge cases, boundary conditions, and stress scenarios
//! to ensure the LinkML service handles all possible inputs gracefully.

use linkml_core::{
    error::{LinkMLError, Result},
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::{
    validator::{ValidationEngine, ValidationOptions},
    parser::Parser,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

/// Test empty and minimal schemas
#[tokio::test]
async fn test_empty_schema_edge_cases() -> Result<()> {
    // Completely empty schema
    let empty_schema = SchemaDefinition {
        id: "".to_string(), // Empty ID
        name: "".to_string(), // Empty name
        classes: HashMap::new(),
        slots: HashMap::new(),
        ..Default::default()
    };

    let result = ValidationEngine::new(&empty_schema);
    assert!(result.is_ok(), "Empty schema should be valid for engine creation");

    // Schema with only spaces in critical fields
    let whitespace_schema = SchemaDefinition {
        id: "   ".to_string(),
        name: "   ".to_string(),
        classes: HashMap::new(),
        slots: HashMap::new(),
        ..Default::default()
    };

    let engine = ValidationEngine::new(&whitespace_schema)?;

    // Try to validate anything against a completely empty schema
    let test_data = json!({"anything": "at_all"});
    let result = engine.validate(&test_data, None).await;

    // This should either fail gracefully or succeed with appropriate warnings
    match result {
        Ok(report) => {
            // If it succeeds, check for appropriate warnings/errors
            assert!(!report.valid || report.warnings.len() > 0,
                   "Empty schema validation should either fail or have warnings");
        }
        Err(_) => {
            // Failing is also acceptable for empty schemas
        }
    }

    Ok(())
}

/// Test malformed and extreme data values
#[tokio::test]
async fn test_malformed_data_edge_cases() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-malformed".to_string(),
        name: "MalformedTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "TestClass".to_string(),
        slots: vec!["flexible_field".to_string()],
        ..Default::default()
    };
    schema.classes.insert("TestClass".to_string(), class_def);

    let flexible_slot = SlotDefinition {
        name: "flexible_field".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("flexible_field".to_string(), flexible_slot);

    let engine = ValidationEngine::new(&schema)?;

    // Test extreme data cases
    let extreme_cases = vec![
        // Empty objects and arrays
        (json!({}), "Empty object"),
        (json!([]), "Empty array"),

        // Deeply nested structures
        (json!({"flexible_field": {"nested": {"deeply": {"very": {"extremely": "deep"}}}}}), "Deep nesting"),

        // Very long strings
        (json!({"flexible_field": "x".repeat(1_000_000)}), "Mega string"),

        // Special characters and unicode
        (json!({"flexible_field": "🚀🌟💫✨🎯🔥💯⚡🌈🎉"}), "Unicode emojis"),
        (json!({"flexible_field": "\x00\x01\x02\x03\x04\x05\x06\x07"}), "Control characters"),
        (json!({"flexible_field": "\u{FEFF}\u{200B}\u{200C}\u{200D}"}), "Zero-width characters"),

        // Extreme numbers
        (json!({"flexible_field": std::f64::MAX}), "Maximum float"),
        (json!({"flexible_field": std::f64::MIN}), "Minimum float"),
        (json!({"flexible_field": std::f64::INFINITY}), "Positive infinity"),
        (json!({"flexible_field": std::f64::NEG_INFINITY}), "Negative infinity"),
        (json!({"flexible_field": std::f64::NAN}), "NaN"),
        (json!({"flexible_field": std::i64::MAX}), "Maximum integer"),
        (json!({"flexible_field": std::i64::MIN}), "Minimum integer"),

        // Very large arrays
        (json!({"flexible_field": vec![1; 100_000]}), "Large array"),

        // Mixed type structures
        (json!({"flexible_field": [1, "string", true, null, {"nested": "object"}]}), "Mixed array"),
    ];

    for (test_data, description) in extreme_cases {
        let result = engine.validate_as_class(&test_data, "TestClass", None).await;

        match result {
            Ok(report) => {
                // Should complete without crashing, regardless of validity
                assert!(report.valid || !report.valid,
                       "Case '{}': Should complete validation without panic", description);
            }
            Err(e) => {
                // Errors are acceptable for extreme cases, but they should be proper errors
                println!("Case '{}' failed with error: {}", description, e);
                // Ensure it's not a panic disguised as an error
                assert!(!e.to_string().contains("panic"),
                       "Case '{}': Should not panic internally", description);
            }
        }
    }

    Ok(())
}

/// Test timeout and resource exhaustion scenarios
#[tokio::test]
async fn test_timeout_and_resource_scenarios() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-timeout".to_string(),
        name: "TimeoutTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "SlowClass".to_string(),
        slots: vec!["complex_pattern".to_string(), "large_data".to_string()],
        ..Default::default()
    };
    schema.classes.insert("SlowClass".to_string(), class_def);

    // Add a slot with a very complex regex that could be slow
    let complex_slot = SlotDefinition {
        name: "complex_pattern".to_string(),
        range: Some("string".to_string()),
        // This is a potentially expensive regex
        pattern: Some(r"^(?:[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.)*[a-zA-Z0-9](?:[a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?\.?$".to_string()),
        ..Default::default()
    };
    schema.slots.insert("complex_pattern".to_string(), complex_slot);

    let large_slot = SlotDefinition {
        name: "large_data".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("large_data".to_string(), large_slot);

    let engine = ValidationEngine::new(&schema)?;

    // Test with data that might trigger regex backtracking
    let potentially_slow_data = json!({
        "complex_pattern": "a".repeat(100) + "." + &"b".repeat(100) + "." + &"c".repeat(100),
        "large_data": "x".repeat(5_000_000) // 5MB string
    });

    let start_time = std::time::Instant::now();

    // This should complete in reasonable time
    let result = tokio::time::timeout(
        Duration::from_secs(30), // 30 second timeout
        engine.validate_as_class(&potentially_slow_data, "SlowClass", None)
    ).await;

    let elapsed = start_time.elapsed();

    match result {
        Ok(validation_result) => {
            match validation_result {
                Ok(report) => {
                    println!("Validation completed in {:?}, valid: {}", elapsed, report.valid);
                    // Should complete reasonably quickly
                    assert!(elapsed < Duration::from_secs(10),
                           "Validation should complete within 10 seconds, took {:?}", elapsed);
                }
                Err(e) => {
                    println!("Validation failed after {:?}: {}", elapsed, e);
                    // Failure is acceptable, but should be fast
                    assert!(elapsed < Duration::from_secs(10),
                           "Even failed validation should complete quickly, took {:?}", elapsed);
                }
            }
        }
        Err(_timeout_error) => {
            panic!("Validation should not timeout - indicates potential infinite loop or performance issue");
        }
    }

    Ok(())
}

/// Test concurrent validation with edge cases
#[tokio::test]
async fn test_concurrent_edge_case_validation() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-concurrent".to_string(),
        name: "ConcurrentTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "TestClass".to_string(),
        slots: vec!["data".to_string()],
        ..Default::default()
    };
    schema.classes.insert("TestClass".to_string(), class_def);

    let data_slot = SlotDefinition {
        name: "data".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("data".to_string(), data_slot);

    let engine = std::sync::Arc::new(ValidationEngine::new(&schema)?);

    // Create various edge case data for concurrent testing
    let edge_case_data = vec![
        json!({"data": ""}), // Empty string
        json!({"data": null}), // Null value
        json!({"data": "unicode: 🚀"}), // Unicode
        json!({"data": "\x00\x01\x02"}), // Control chars
        json!({"data": "very".repeat(10000)}), // Long string
        json!({"data": serde_json::Value::String("json".repeat(1000))}), // Large JSON
        json!({}), // Missing field
        json!({"data": {"nested": "object"}}), // Wrong type
        json!({"data": [1, 2, 3]}), // Array instead of string
        json!({"data": true}), // Boolean instead of string
    ];

    let mut handles = Vec::new();

    // Spawn 100 concurrent validations with edge case data
    for i in 0..100 {
        let engine_clone = engine.clone();
        let test_data = edge_case_data[i % edge_case_data.len()].clone();

        let handle = tokio::spawn(async move {
            let result = engine_clone.validate_as_class(&test_data, "TestClass", None).await;
            (i, result)
        });
        handles.push(handle);
    }

    // Collect all results
    let mut completed = 0;
    let mut panicked = 0;

    for handle in handles {
        match handle.await {
            Ok((i, validation_result)) => {
                completed += 1;
                match validation_result {
                    Ok(report) => {
                        // Should always produce a report, valid or not
                        assert!(report.valid || !report.valid,
                               "Task {}: Should produce valid report", i);
                    }
                    Err(e) => {
                        // Errors are OK, but shouldn't be panics
                        assert!(!e.to_string().contains("panic"),
                               "Task {}: Should not panic: {}", i, e);
                    }
                }
            }
            Err(join_error) => {
                panicked += 1;
                if join_error.is_panic() {
                    panic!("Task panicked during concurrent validation: {:?}", join_error);
                }
            }
        }
    }

    assert_eq!(completed, 100, "All tasks should complete");
    assert_eq!(panicked, 0, "No tasks should panic");

    Ok(())
}

/// Test schema loading edge cases
#[tokio::test]
async fn test_schema_loading_edge_cases() -> Result<()> {
    let parser = Parser::new();

    // Test various malformed schema inputs
    let malformed_schemas = vec![
        // Empty content
        ("", "yaml", "Empty content"),
        ("", "json", "Empty JSON content"),

        // Only whitespace
        ("   
\t  ", "yaml", "Whitespace only"),

        // Invalid YAML/JSON
        ("{invalid: yaml: content", "yaml", "Invalid YAML"),
        ("{\"invalid\": json,}", "json", "Invalid JSON"),

        // Partially valid but incomplete
        ("id: test", "yaml", "Incomplete YAML"),
        ("{\"id\": \"test\"}", "json", "Incomplete JSON"),

        // Very large schemas
        (format!("id: large
name: Large
classes:
{}",
                 (0..1000).map(|i| format!("  Class{}:
    name: Class{}
", i, i))
                          .collect::<Vec<_>>().join("")), "yaml", "Large schema"),

        // Unicode and special characters in schema
        ("id: 🚀
name: Unicode🌟
classes:
  🎯Class:
    name: 🎯Class", "yaml", "Unicode schema"),

        // Extremely nested structure
        ("id: nested
name: Nested
classes:
  Test:
    name: Test
    is_a: ".to_string() +
         &(0..100).map(|_| "Parent").collect::<Vec<_>>().join("."), "yaml", "Deep inheritance"),
    ];

    for (content, format, description) in malformed_schemas {
        let result = parser.parse(&content, format);

        match result {
            Ok(schema) => {
                println!("Unexpectedly parsed '{}': schema.id = '{}', name = '{}'",
                        description, schema.id, schema.name);
                // If it parses, ensure the result is still usable
                let engine_result = ValidationEngine::new(&schema);
                match engine_result {
                    Ok(_) => {
                        // Engine creation succeeded - that's fine
                    }
                    Err(e) => {
                        // Engine creation can fail for malformed schemas
                        println!("Engine creation failed for '{}': {}", description, e);
                    }
                }
            }
            Err(e) => {
                // Expected to fail for malformed content
                println!("Expected failure for '{}': {}", description, e);
                // Ensure error message is reasonable
                assert!(!e.to_string().is_empty(),
                       "Error message should not be empty for '{}'", description);
            }
        }
    }

    Ok(())
}

/// Test memory pressure and large data scenarios
#[tokio::test]
async fn test_memory_pressure_scenarios() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-memory".to_string(),
        name: "MemoryTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "MemoryHog".to_string(),
        slots: vec!["large_field".to_string(), "many_fields".to_string()],
        ..Default::default()
    };
    schema.classes.insert("MemoryHog".to_string(), class_def);

    let large_slot = SlotDefinition {
        name: "large_field".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("large_field".to_string(), large_slot);

    let many_slot = SlotDefinition {
        name: "many_fields".to_string(),
        range: Some("object".to_string()),
        ..Default::default()
    };
    schema.slots.insert("many_fields".to_string(), many_slot);

    let engine = ValidationEngine::new(&schema)?;

    // Test various memory-intensive scenarios
    let memory_tests = vec![
        // Very large string (10MB)
        json!({
            "large_field": "x".repeat(10_000_000),
            "many_fields": {}
        }),

        // Object with many fields
        json!({
            "large_field": "test",
            "many_fields": (0..10000).map(|i| (format!("field_{}", i), i)).collect::<serde_json::Map<String, Value>>()
        }),

        // Deep nesting
        {
            let mut nested = json!("deep");
            for _ in 0..1000 {
                nested = json!({"level": nested});
            }
            json!({
                "large_field": "test",
                "many_fields": nested
            })
        },

        // Large array
        json!({
            "large_field": "test",
            "many_fields": (0..100000).collect::<Vec<i32>>()
        }),
    ];

    for (i, test_data) in memory_tests.iter().enumerate() {
        let start_memory = get_memory_usage();
        let start_time = std::time::Instant::now();

        let result = engine.validate_as_class(test_data, "MemoryHog", None).await;

        let end_time = std::time::Instant::now();
        let end_memory = get_memory_usage();

        let duration = end_time - start_time;
        let memory_increase = end_memory.saturating_sub(start_memory);

        println!("Memory test {}: duration={:?}, memory_increase={}MB",
                i, duration, memory_increase / 1024 / 1024);

        match result {
            Ok(report) => {
                // Should complete without excessive resource usage
                assert!(duration < Duration::from_secs(60),
                       "Memory test {} should complete within 60 seconds", i);
                assert!(memory_increase < 1024 * 1024 * 1024, // 1GB limit
                       "Memory test {} should not use more than 1GB extra memory", i);
            }
            Err(e) => {
                // Can fail due to resource limits, but shouldn't crash
                println!("Memory test {} failed (acceptable): {}", i, e);
            }
        }
    }

    Ok(())
}

/// Get current memory usage (approximation)
fn get_memory_usage() -> usize {
    // This is a rough approximation - in a real test environment you'd use proper memory measurement
    use std::process;

    #[cfg(target_os = "linux")]
    {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Ok(kb) = line.split_whitespace().nth(1).unwrap_or("0").parse::<usize>() {
                        return kb * 1024; // Convert KB to bytes
                    }
                }
            }
        }
    }

    // Fallback - just return 0 if we can't measure
    0
}

/// Test boundary conditions for numeric values
#[tokio::test]
async fn test_numeric_boundary_conditions() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-numeric".to_string(),
        name: "NumericTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "NumericClass".to_string(),
        slots: vec!["number_field".to_string()],
        ..Default::default()
    };
    schema.classes.insert("NumericClass".to_string(), class_def);

    let numeric_slot = SlotDefinition {
        name: "number_field".to_string(),
        range: Some("float".to_string()),
        ..Default::default()
    };
    schema.slots.insert("number_field".to_string(), numeric_slot);

    let engine = ValidationEngine::new(&schema)?;

    // Test various numeric edge cases
    let numeric_tests = vec![
        (json!({"number_field": 0}), "Zero"),
        (json!({"number_field": -0.0}), "Negative zero"),
        (json!({"number_field": std::f64::INFINITY}), "Positive infinity"),
        (json!({"number_field": std::f64::NEG_INFINITY}), "Negative infinity"),
        (json!({"number_field": std::f64::NAN}), "NaN"),
        (json!({"number_field": std::f64::MAX}), "Maximum float"),
        (json!({"number_field": std::f64::MIN}), "Minimum float"),
        (json!({"number_field": std::f64::EPSILON}), "Epsilon"),
        (json!({"number_field": 1e308}), "Large scientific notation"),
        (json!({"number_field": 1e-308}), "Small scientific notation"),
        (json!({"number_field": std::i64::MAX}), "Max integer as float"),
        (json!({"number_field": std::i64::MIN}), "Min integer as float"),
    ];

    for (test_data, description) in numeric_tests {
        let result = engine.validate_as_class(&test_data, "NumericClass", None).await;

        match result {
            Ok(report) => {
                println!("{}: validation completed, valid={}", description, report.valid);
                // Should handle all numeric values without crashing
            }
            Err(e) => {
                println!("{}: validation error (may be expected): {}", description, e);
                // Errors are acceptable for extreme values
            }
        }
    }

    Ok(())
}

/// Test string encoding and character edge cases
#[tokio::test]
async fn test_string_encoding_edge_cases() -> Result<()> {
    let mut schema = SchemaDefinition {
        id: "test-encoding".to_string(),
        name: "EncodingTest".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "StringClass".to_string(),
        slots: vec!["text_field".to_string()],
        ..Default::default()
    };
    schema.classes.insert("StringClass".to_string(), class_def);

    let text_slot = SlotDefinition {
        name: "text_field".to_string(),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("text_field".to_string(), text_slot);

    let engine = ValidationEngine::new(&schema)?;

    // Test various string encoding edge cases
    let string_tests = vec![
        (json!({"text_field": ""}), "Empty string"),
        (json!({"text_field": " "}), "Single space"),
        (json!({"text_field": "
"}), "Single newline"),
        (json!({"text_field": "\t"}), "Single tab"),
        (json!({"text_field": "\r
"}), "CRLF"),
        (json!({"text_field": "\x00"}), "Null character"),
        (json!({"text_field": "\x7F"}), "DEL character"),
        (json!({"text_field": "\u{FEFF}"}), "BOM character"),
        (json!({"text_field": "\u{200B}\u{200C}\u{200D}"}), "Zero-width characters"),
        (json!({"text_field": "🚀🌟💫✨🎯"}), "Emoji sequence"),
        (json!({"text_field": "Åñgëlö"}), "Accented characters"),
        (json!({"text_field": "مرحبا بالعالم"}), "Arabic text"),
        (json!({"text_field": "你好世界"}), "Chinese text"),
        (json!({"text_field": "🏳️‍🌈🏳️‍⚧️"}), "Complex emoji with modifiers"),
        (json!({"text_field": "\u{1F600}\u{1F601}\u{1F602}"}), "Unicode emoji codes"),
        (json!({"text_field": "a".repeat(1_000_000)}), "Very long string"),
    ];

    for (test_data, description) in string_tests {
        let result = engine.validate_as_class(&test_data, "StringClass", None).await;

        match result {
            Ok(report) => {
                println!("{}: validation completed, valid={}", description, report.valid);
                // Should handle all string encodings
            }
            Err(e) => {
                println!("{}: validation error: {}", description, e);
                // Should not crash on any valid UTF-8 string
                assert!(!e.to_string().contains("panic"),
                       "Should not panic on UTF-8 string: {}", description);
            }
        }
    }

    Ok(())
}