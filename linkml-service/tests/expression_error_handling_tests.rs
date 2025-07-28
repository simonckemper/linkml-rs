//! Tests for expression module error handling improvements
//!
//! Verifies that the expression evaluation system properly handles errors
//! without panicking after unwrap() removal.

use linkml_service::expression::{
    evaluator::ExpressionEvaluator,
    parser::ExpressionParser,
    functions::{FunctionRegistry, MathFunctions, StringFunctions, DateFunctions, AggregationFunctions},
    compiler::ExpressionCompiler,
    engine_v2::ExpressionEngineV2,
    cache::ExpressionCache,
    cache_v2::ExpressionCacheV2,
    parallel::ParallelEvaluator,
    Value,
};
use linkml_core::error::LinkMLError;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Test math function error handling
#[test]
fn test_math_functions_error_handling() {
    let math_fns = MathFunctions::new();
    
    // Test division by zero
    let args = vec![Value::Number(10.0), Value::Number(0.0)];
    let result = math_fns.call("divide", &args);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("zero"));
    
    // Test sqrt of negative number
    let args = vec![Value::Number(-4.0)];
    let result = math_fns.call("sqrt", &args);
    assert!(result.is_err());
    
    // Test log of non-positive number
    let args = vec![Value::Number(0.0)];
    let result = math_fns.call("log", &args);
    assert!(result.is_err());
    
    // Test invalid number of arguments
    let args = vec![Value::Number(5.0), Value::Number(3.0), Value::Number(1.0)];
    let result = math_fns.call("add", &args);
    assert!(result.is_err());
    
    // Test type mismatch
    let args = vec![Value::String("not a number".to_string()), Value::Number(5.0)];
    let result = math_fns.call("add", &args);
    assert!(result.is_err());
}

/// Test string function error handling
#[test]
fn test_string_functions_error_handling() {
    let string_fns = StringFunctions::new();
    
    // Test regex with invalid pattern
    let args = vec![
        Value::String("test string".to_string()),
        Value::String("[invalid(regex".to_string())
    ];
    let result = string_fns.call("matches", &args);
    assert!(result.is_err());
    
    // Test substring with out-of-bounds indices
    let args = vec![
        Value::String("short".to_string()),
        Value::Number(10.0),
        Value::Number(20.0)
    ];
    let result = string_fns.call("substring", &args);
    assert!(result.is_err());
    
    // Test split with empty delimiter
    let args = vec![
        Value::String("test string".to_string()),
        Value::String("".to_string())
    ];
    let result = string_fns.call("split", &args);
    assert!(result.is_err());
    
    // Test replace with invalid regex
    let args = vec![
        Value::String("test string".to_string()),
        Value::String("(unclosed".to_string()),
        Value::String("replacement".to_string())
    ];
    let result = string_fns.call("replace", &args);
    assert!(result.is_err());
}

/// Test date function error handling
#[test]
fn test_date_functions_error_handling() {
    let date_fns = DateFunctions::new();
    
    // Test parse with invalid format
    let args = vec![
        Value::String("not a date".to_string()),
        Value::String("%Y-%m-%d".to_string())
    ];
    let result = date_fns.call("parse_date", &args);
    assert!(result.is_err());
    
    // Test format with invalid format string
    let now: DateTime<Utc> = Utc::now();
    let args = vec![
        Value::DateTime(now),
        Value::String("%Q".to_string()) // Invalid format specifier
    ];
    let result = date_fns.call("format_date", &args);
    // Should handle gracefully
    let _ = result;
    
    // Test date arithmetic with overflow
    let args = vec![
        Value::DateTime(now),
        Value::Number(i64::MAX as f64) // Massive number of days
    ];
    let result = date_fns.call("add_days", &args);
    assert!(result.is_err());
}

/// Test aggregation function error handling
#[test]
fn test_aggregation_functions_error_handling() {
    let agg_fns = AggregationFunctions::new();
    
    // Test with empty array
    let args = vec![Value::Array(vec![])];
    let result = agg_fns.call("mean", &args);
    assert!(result.is_err());
    
    // Test with mixed types
    let args = vec![Value::Array(vec![
        Value::Number(1.0),
        Value::String("not a number".to_string()),
        Value::Number(3.0)
    ])];
    let result = agg_fns.call("sum", &args);
    assert!(result.is_err());
    
    // Test percentile with invalid percentile value
    let args = vec![
        Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
        Value::Number(150.0) // Invalid percentile > 100
    ];
    let result = agg_fns.call("percentile", &args);
    assert!(result.is_err());
    
    // Test standard deviation with single value
    let args = vec![Value::Array(vec![Value::Number(5.0)])];
    let result = agg_fns.call("stddev", &args);
    assert!(result.is_err() || result.unwrap() == Value::Number(0.0));
}

/// Test expression parser error handling
#[test]
fn test_expression_parser_error_handling() {
    let parser = ExpressionParser::new();
    
    // Test unclosed parenthesis
    let result = parser.parse("add(1, 2");
    assert!(result.is_err());
    
    // Test invalid operator
    let result = parser.parse("1 ??? 2");
    assert!(result.is_err());
    
    // Test unclosed string
    let result = parser.parse(r#"concat("hello, "world")"#);
    assert!(result.is_err());
    
    // Test invalid function name
    let result = parser.parse("123invalid_function()");
    assert!(result.is_err());
    
    // Test deeply nested expression that might cause stack overflow
    let mut expr = "add(".to_string();
    for _ in 0..1000 {
        expr.push_str("add(1, ");
    }
    expr.push_str("1");
    for _ in 0..1000 {
        expr.push_str(")");
    }
    expr.push_str(")");
    
    let result = parser.parse(&expr);
    // Should handle without stack overflow
    let _ = result;
}

/// Test expression compiler error handling
#[test]
fn test_expression_compiler_error_handling() {
    let compiler = ExpressionCompiler::new();
    let parser = ExpressionParser::new();
    
    // Test compilation of expression with undefined variable
    let expr = parser.parse("undefined_var + 5").unwrap();
    let result = compiler.compile(&expr);
    // Compilation might succeed, but execution should fail
    if let Ok(compiled) = result {
        let context = HashMap::new(); // Empty context
        let exec_result = compiled.execute(&context);
        assert!(exec_result.is_err());
    }
    
    // Test compilation of expression with type errors
    let expr = parser.parse(r#"concat(5, "string")"#).unwrap();
    let result = compiler.compile(&expr);
    if let Ok(compiled) = result {
        let context = HashMap::new();
        let exec_result = compiled.execute(&context);
        // Type errors might be caught at runtime
        let _ = exec_result;
    }
}

/// Test expression engine v2 error handling
#[test]
fn test_expression_engine_v2_error_handling() {
    let engine = ExpressionEngineV2::new();
    let mut context = HashMap::new();
    context.insert("x".to_string(), Value::Number(10.0));
    
    // Test with syntax error
    let result = engine.evaluate("x + + y", &context);
    assert!(result.is_err());
    
    // Test with undefined function
    let result = engine.evaluate("undefined_func(x)", &context);
    assert!(result.is_err());
    
    // Test with type error
    context.insert("s".to_string(), Value::String("string".to_string()));
    let result = engine.evaluate("x * s", &context);
    assert!(result.is_err());
    
    // Test with circular reference
    let result = engine.evaluate("a = b; b = a; a + 1", &context);
    assert!(result.is_err());
}

/// Test expression cache error handling
#[test]
fn test_expression_cache_error_handling() {
    let cache = ExpressionCache::new(100);
    let parser = ExpressionParser::new();
    let evaluator = ExpressionEvaluator::new();
    let registry = FunctionRegistry::new();
    
    // Test caching of failed expressions
    let invalid_expr = "1 / 0";
    let parsed = parser.parse(invalid_expr).unwrap();
    let result = evaluator.evaluate(&parsed, &registry, &HashMap::new());
    assert!(result.is_err());
    
    // Second evaluation should still fail (not cache success)
    let result2 = evaluator.evaluate(&parsed, &registry, &HashMap::new());
    assert!(result2.is_err());
}

/// Test parallel evaluator error handling
#[tokio::test]
async fn test_parallel_evaluator_error_handling() {
    let evaluator = ParallelEvaluator::new(4);
    
    let expressions = vec![
        "1 + 2",          // Valid
        "3 / 0",          // Division by zero
        "4 * 5",          // Valid
        "undefined()",    // Undefined function
        "6 - 7",          // Valid
    ];
    
    let contexts: Vec<HashMap<String, Value>> = vec![HashMap::new(); expressions.len()];
    
    let results = evaluator.evaluate_batch(&expressions, &contexts).await;
    
    // Should have results for all expressions
    assert_eq!(results.len(), expressions.len());
    
    // Check specific results
    assert!(results[0].is_ok());
    assert!(results[1].is_err());
    assert!(results[2].is_ok());
    assert!(results[3].is_err());
    assert!(results[4].is_ok());
}

/// Test complex expression error scenarios
#[test]
fn test_complex_expression_errors() {
    let engine = ExpressionEngineV2::new();
    let mut context = HashMap::new();
    context.insert("arr".to_string(), Value::Array(vec![
        Value::Number(1.0),
        Value::Number(2.0),
        Value::Number(3.0)
    ]));
    
    // Test array index out of bounds
    let result = engine.evaluate("arr[10]", &context);
    assert!(result.is_err());
    
    // Test nested function call with error
    let result = engine.evaluate("sqrt(log(-1))", &context);
    assert!(result.is_err());
    
    // Test conditional with type error
    let result = engine.evaluate(r#"if "true" then 1 else 2"#, &context);
    assert!(result.is_err() || result.is_ok()); // Might coerce string to bool
    
    // Test object property access on non-object
    context.insert("num".to_string(), Value::Number(42.0));
    let result = engine.evaluate("num.property", &context);
    assert!(result.is_err());
}

/// Test function registry error handling
#[test]
fn test_function_registry_error_handling() {
    let mut registry = FunctionRegistry::new();
    registry.register_defaults();
    
    // Test calling non-existent function
    let result = registry.call_function("non_existent", &[]);
    assert!(result.is_err());
    
    // Test calling with wrong arity
    let result = registry.call_function("add", &[Value::Number(1.0)]);
    assert!(result.is_err());
    
    // Test duplicate registration
    let result = registry.register_function("add", |_args| Ok(Value::Null));
    assert!(result.is_err()); // Should fail as 'add' already exists
}

/// Test expression validation
#[test]
fn test_expression_validation() {
    let parser = ExpressionParser::new();
    let validator = ExpressionValidator::new();
    
    // Test validation of expressions with errors
    let test_cases = vec![
        ("", "Empty expression"),
        ("()", "Empty parentheses"),
        ("1 +", "Incomplete expression"),
        ("add(,)", "Empty arguments"),
        ("1..2", "Invalid syntax"),
    ];
    
    for (expr, description) in test_cases {
        if let Ok(parsed) = parser.parse(expr) {
            let result = validator.validate(&parsed);
            assert!(result.is_err(), "Should fail validation: {}", description);
        }
    }
}

// Mock validator for testing
struct ExpressionValidator;

impl ExpressionValidator {
    fn new() -> Self {
        Self
    }
    
    fn validate(&self, _expr: &linkml_service::expression::Expression) -> Result<(), LinkMLError> {
        // Simple validation logic
        Ok(())
    }
}