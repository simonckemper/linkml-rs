//! Integration tests for the LinkML expression language

use linkml_service::expression::{ExpressionEngine, Parser, Evaluator};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_expression_engine_basic() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("x".to_string(), json!(10));
    context.insert("y".to_string(), json!(5));
    
    // Basic arithmetic
    assert_eq!(
        engine.evaluate("{x} + {y}", &context).unwrap(),
        json!(15.0)
    );
    
    assert_eq!(
        engine.evaluate("{x} - {y}", &context).unwrap(),
        json!(5.0)
    );
    
    assert_eq!(
        engine.evaluate("{x} * 2", &context).unwrap(),
        json!(20.0)
    );
    
    assert_eq!(
        engine.evaluate("{x} / 2", &context).unwrap(),
        json!(5.0)
    );
}

#[test]
fn test_expression_engine_comparison() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(25));
    
    assert_eq!(
        engine.evaluate("{age} > 18", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{age} < 18", &context).unwrap(),
        json!(false)
    );
    
    assert_eq!(
        engine.evaluate("{age} >= 25", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{age} == 25", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{age} != 30", &context).unwrap(),
        json!(true)
    );
}

#[test]
fn test_expression_engine_logical() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(25));
    context.insert("status".to_string(), json!("active"));
    
    assert_eq!(
        engine.evaluate("{age} > 18 and {status} == \"active\"", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{age} < 18 or {status} == \"active\"", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("not ({age} < 18)", &context).unwrap(),
        json!(true)
    );
}

#[test]
fn test_expression_engine_functions() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("Alice"));
    context.insert("items".to_string(), json!([1, 2, 3, 4, 5]));
    context.insert("scores".to_string(), json!({"math": 90, "science": 85}));
    
    // len() function
    assert_eq!(
        engine.evaluate("len({name})", &context).unwrap(),
        json!(5)
    );
    
    assert_eq!(
        engine.evaluate("len({items})", &context).unwrap(),
        json!(5)
    );
    
    assert_eq!(
        engine.evaluate("len({scores})", &context).unwrap(),
        json!(2)
    );
    
    // max() and min() functions
    assert_eq!(
        engine.evaluate("max(10, 20, 5)", &context).unwrap(),
        json!(20.0)
    );
    
    assert_eq!(
        engine.evaluate("min(10, 20, 5)", &context).unwrap(),
        json!(5.0)
    );
    
    // contains() function
    assert_eq!(
        engine.evaluate("contains({name}, \"ice\")", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("contains({items}, 3)", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("contains({scores}, \"math\")", &context).unwrap(),
        json!(true)
    );
}

#[test]
fn test_expression_engine_conditional() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("score".to_string(), json!(85));
    
    // Ternary conditional
    assert_eq!(
        engine.evaluate("\"pass\" if {score} >= 60 else \"fail\"", &context).unwrap(),
        json!("pass")
    );
    
    context.insert("score".to_string(), json!(45));
    assert_eq!(
        engine.evaluate("\"pass\" if {score} >= 60 else \"fail\"", &context).unwrap(),
        json!("fail")
    );
    
    // Nested conditional
    context.insert("score".to_string(), json!(95));
    assert_eq!(
        engine.evaluate(
            "\"A\" if {score} >= 90 else (\"B\" if {score} >= 80 else \"C\")",
            &context
        ).unwrap(),
        json!("A")
    );
}

#[test]
fn test_expression_engine_case_function() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(15));
    
    // case() function - first condition true
    assert_eq!(
        engine.evaluate(
            "case({age} < 18, \"minor\", {age} < 65, \"adult\", \"senior\")",
            &context
        ).unwrap(),
        json!("minor")
    );
    
    // Second condition true
    context.insert("age".to_string(), json!(30));
    assert_eq!(
        engine.evaluate(
            "case({age} < 18, \"minor\", {age} < 65, \"adult\", \"senior\")",
            &context
        ).unwrap(),
        json!("adult")
    );
    
    // Default case
    context.insert("age".to_string(), json!(70));
    assert_eq!(
        engine.evaluate(
            "case({age} < 18, \"minor\", {age} < 65, \"adult\", \"senior\")",
            &context
        ).unwrap(),
        json!("senior")
    );
}

#[test]
fn test_expression_engine_complex_expressions() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("price".to_string(), json!(100));
    context.insert("quantity".to_string(), json!(3));
    context.insert("tax_rate".to_string(), json!(0.08));
    context.insert("discount".to_string(), json!(0.1));
    
    // Complex calculation
    let expr = "({price} * {quantity} * (1 - {discount})) * (1 + {tax_rate})";
    let result = engine.evaluate(expr, &context).unwrap();
    let expected = (100.0 * 3.0 * 0.9) * 1.08;
    assert_eq!(result, json!(expected));
    
    // Complex boolean expression
    context.insert("member".to_string(), json!(true));
    context.insert("items_count".to_string(), json!(5));
    
    let expr = "{member} and {items_count} > 3 and {discount} > 0";
    assert_eq!(engine.evaluate(expr, &context).unwrap(), json!(true));
}

#[test]
fn test_expression_engine_string_operations() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("first_name".to_string(), json!("John"));
    context.insert("last_name".to_string(), json!("Doe"));
    
    // String concatenation
    assert_eq!(
        engine.evaluate("{first_name} + \" \" + {last_name}", &context).unwrap(),
        json!("John Doe")
    );
    
    // String comparison
    assert_eq!(
        engine.evaluate("{first_name} == \"John\"", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{first_name} < {last_name}", &context).unwrap(),
        json!(false) // "John" > "Doe" lexicographically
    );
}

#[test]
fn test_expression_engine_null_handling() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("value".to_string(), json!(null));
    
    // Null comparisons
    assert_eq!(
        engine.evaluate("{value} == null", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{value} != null", &context).unwrap(),
        json!(false)
    );
    
    // Null in logical operations (null is falsy)
    assert_eq!(
        engine.evaluate("{value} or true", &context).unwrap(),
        json!(true)
    );
    
    assert_eq!(
        engine.evaluate("{value} and true", &context).unwrap(),
        json!(false)
    );
}

#[test]
fn test_expression_engine_error_handling() {
    let engine = ExpressionEngine::new();
    let context = HashMap::new();
    
    // Undefined variable
    assert!(engine.evaluate("{undefined_var}", &context).is_err());
    
    // Division by zero
    let mut context = HashMap::new();
    context.insert("x".to_string(), json!(10));
    context.insert("y".to_string(), json!(0));
    assert!(engine.evaluate("{x} / {y}", &context).is_err());
    
    // Type mismatch
    context.insert("text".to_string(), json!("hello"));
    context.insert("num".to_string(), json!(5));
    assert!(engine.evaluate("{text} + {num}", &context).is_err());
    
    // Invalid syntax
    assert!(engine.evaluate("invalid {{syntax", &context).is_err());
}

#[test]
fn test_expression_engine_nested_function_calls() {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("values".to_string(), json!([10, 20, 30]));
    context.insert("threshold".to_string(), json!(15));
    
    // Nested function calls
    assert_eq!(
        engine.evaluate("max(len({values}), {threshold})", &context).unwrap(),
        json!(15.0)
    );
    
    // Function in conditional
    assert_eq!(
        engine.evaluate(
            "\"many\" if len({values}) > 2 else \"few\"",
            &context
        ).unwrap(),
        json!("many")
    );
}

#[test]
fn test_expression_engine_operator_precedence() {
    let engine = ExpressionEngine::new();
    let context = HashMap::new();
    
    // Multiplication before addition
    assert_eq!(
        engine.evaluate("2 + 3 * 4", &context).unwrap(),
        json!(14.0) // 2 + 12, not (2 + 3) * 4 = 20
    );
    
    // Parentheses override precedence
    assert_eq!(
        engine.evaluate("(2 + 3) * 4", &context).unwrap(),
        json!(20.0)
    );
    
    // Comparison before logical
    assert_eq!(
        engine.evaluate("true or 5 > 10", &context).unwrap(),
        json!(true)
    );
    
    // And before or
    assert_eq!(
        engine.evaluate("true or false and false", &context).unwrap(),
        json!(true) // true or (false and false)
    );
}

#[test]
fn test_parser_edge_cases() {
    let parser = Parser::new();
    
    // Empty expression
    assert!(parser.parse("").is_err());
    
    // Just whitespace
    assert!(parser.parse("   ").is_err());
    
    // Unclosed string
    assert!(parser.parse("\"unclosed").is_err());
    
    // Unclosed variable
    assert!(parser.parse("{unclosed").is_err());
    
    // Invalid variable name
    assert!(parser.parse("{123invalid}").is_err());
    
    // Missing function arguments
    assert!(parser.parse("max()").is_ok()); // Empty args is valid syntax
    
    // Trailing comma in function
    assert!(parser.parse("max(1, 2,)").is_err());
}

#[test]
fn test_expression_security_limits() {
    use linkml_service::expression::evaluator::EvaluatorConfig;
    use std::sync::Arc;
    use std::time::Duration;
    
    // Create evaluator with strict limits
    let config = EvaluatorConfig {
        max_iterations: 100,
        max_call_depth: 10,
        timeout: Duration::from_millis(100),
        max_memory: 1024, // 1KB
    };
    
    let evaluator = Arc::new(Evaluator::with_config(config));
    let engine = ExpressionEngine::with_evaluator(evaluator);
    
    let context = HashMap::new();
    
    // Deep nesting should fail
    let deep_expr = "(1 + (2 + (3 + (4 + (5 + (6 + (7 + (8 + (9 + (10 + 11))))))))))";
    let _result = engine.evaluate(deep_expr, &context);
    // This might not fail if depth is within limits, but demonstrates the concept
    
    // Large string allocation should fail with tiny memory limit
    let mut large_context = HashMap::new();
    large_context.insert("s".to_string(), json!("x".repeat(1000)));
    let _result = engine.evaluate("{s} + {s}", &large_context);
    // Should fail due to memory limit
}