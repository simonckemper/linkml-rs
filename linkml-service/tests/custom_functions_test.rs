//! Tests for custom function support in the expression evaluator

use linkml_service::expression::{
    evaluator::Evaluator,
    functions::{CustomFunction, FunctionError, FunctionRegistry},
    parser::Parser,
};
use serde_json::{Value, json};

#[test]
fn test_custom_function_registration() {
    let mut registry = FunctionRegistry::new();

    // Register a custom uppercase function
    registry.register_custom(CustomFunction::new(
        "uppercase",
        1,
        Some(1),
        |args| match &args[0] {
            Value::String(s) => Ok(Value::String(s.to_uppercase())),
            _ => Err(FunctionError::new("Expected string argument")),
        },
    ));

    // Test direct function call
    let result = registry
        .call("uppercase", vec![json!("hello world")])
        .expect("Test operation failed");
    assert_eq!(result, json!("HELLO WORLD"));

    // Verify function is registered
    assert!(registry.has_function("uppercase"));
}

#[test]
fn test_custom_function_in_expression() {
    let mut evaluator = Evaluator::new();

    // Register a custom reverse function
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new(
            "reverse",
            1,
            Some(1),
            |args| match &args[0] {
                Value::String(s) => Ok(Value::String(s.chars().rev().collect())),
                _ => Err(FunctionError::new("Expected string argument")),
            },
        ));

    // Parse and evaluate expression with custom function
    let parser = Parser::new();
    let expr = parser
        .parse("reverse(name)")
        .expect("Test operation failed");

    let mut context = std::collections::HashMap::new();
    context.insert("name".to_string(), json!("LinkML"));

    let result = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    assert_eq!(result, json!("LMkniL"));
}

#[test]
fn test_custom_math_function() {
    let mut evaluator = Evaluator::new();

    // Register a custom square function
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new(
            "square",
            1,
            Some(1),
            |args| match &args[0] {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(json!(i * i))
                    } else if let Some(f) = n.as_f64() {
                        Ok(json!(f * f))
                    } else {
                        Err(FunctionError::new("Invalid number"))
                    }
                }
                _ => Err(FunctionError::new("Expected number argument")),
            },
        ));

    // Test with integer
    let parser = Parser::new();
    let expr = parser.parse("square(5)").expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!(25));

    // Test with float
    let expr = parser.parse("square(3.5)").expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!(12.25));
}

#[test]
fn test_custom_function_with_multiple_args() {
    let mut evaluator = Evaluator::new();

    // Register a custom join function
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new(
            "join",
            2,
            None, // Variable number of arguments (at least 2)
            |args| {
                let strings: Result<Vec<String>, _> = args
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(s.clone()),
                        _ => Err(FunctionError::new("All arguments must be strings")),
                    })
                    .collect();

                match strings {
                    Ok(strs) => Ok(Value::String(strs.join(" "))),
                    Err(e) => Err(e),
                }
            },
        ));

    // Test with 2 arguments
    let parser = Parser::new();
    let expr = parser
        .parse(r#"join("Hello", "World")"#)
        .expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!("Hello World"));

    // Test with 3 arguments
    let expr = parser
        .parse(r#"join("One", "Two", "Three")"#)
        .expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!("One Two Three"));
}

#[test]
fn test_custom_validation_function() {
    let mut evaluator = Evaluator::new();

    // Register a custom email validation function
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new("is_email", 1, Some(1), |args| {
            match &args[0] {
                Value::String(s) => {
                    // Simple email validation
                    let is_valid = s.contains('@') && s.contains('.') && s.len() > 5;
                    Ok(Value::Bool(is_valid))
                }
                _ => Err(FunctionError::new("Expected string argument")),
            }
        }));

    // Test valid email
    let parser = Parser::new();
    let expr = parser
        .parse(r#"is_email("user@example.com")"#)
        .expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!(true));

    // Test invalid email
    let expr = parser
        .parse(r#"is_email("not-an-email")"#)
        .expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!(false));
}

#[test]
fn test_custom_function_error_handling() {
    let mut registry = FunctionRegistry::new();

    // Register a function that requires specific types
    registry.register_custom(CustomFunction::new("add_days", 2, Some(2), |args| {
        match (&args[0], &args[1]) {
            (Value::String(date_str), Value::Number(days)) => {
                // Simplified date addition (just for testing)
                if let Some(d) = days.as_i64() {
                    Ok(Value::String(format!("{} + {} days", date_str, d)))
                } else {
                    Err(FunctionError::new("Days must be an integer"))
                }
            }
            _ => Err(FunctionError::new("Expected (string, number) arguments")),
        }
    }));

    // Test with correct types
    let result = registry
        .call("add_days", vec![json!("2025-01-31"), json!(7)])
        .expect("Test operation failed");
    assert_eq!(result, json!("2025-01-31 + 7 days"));

    // Test with wrong types
    let err = registry
        .call("add_days", vec![json!(123), json!("not a number")])
        .unwrap_err();
    assert!(err.message.contains("Expected (string, number) arguments"));

    // Test with wrong arity
    let err = registry
        .call("add_days", vec![json!("2025-01-31")])
        .unwrap_err();
    assert!(err.message.contains("expects at least 2 arguments, got 1"));
}

#[test]
fn test_custom_function_list_operations() {
    let mut evaluator = Evaluator::new();

    // Register a custom sum function for arrays
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new("sum", 1, Some(1), |args| {
            match &args[0] {
                Value::Array(arr) => {
                    let mut sum = 0.0;
                    for val in arr {
                        match val {
                            Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    sum += i as f64;
                                } else if let Some(f) = n.as_f64() {
                                    sum += f;
                                } else {
                                    return Err(FunctionError::new("Invalid number in array"));
                                }
                            }
                            _ => {
                                return Err(FunctionError::new(
                                    "All array elements must be numbers",
                                ));
                            }
                        }
                    }
                    Ok(json!(sum))
                }
                _ => Err(FunctionError::new("Expected array argument")),
            }
        }));

    // Test sum function
    let parser = Parser::new();
    let expr = parser.parse("sum(numbers)").expect("Test operation failed");

    let mut context = std::collections::HashMap::new();
    context.insert("numbers".to_string(), json!([1, 2, 3, 4, 5]));

    let result = evaluator
        .evaluate(&expr, &context)
        .expect("Test operation failed");
    assert_eq!(result, json!(15.0));
}

#[test]
fn test_custom_function_chaining() {
    let mut evaluator = Evaluator::new();

    // Register custom functions that can be chained
    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new(
            "double",
            1,
            Some(1),
            |args| match &args[0] {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(json!(i * 2))
                    } else if let Some(f) = n.as_f64() {
                        Ok(json!(f * 2.0))
                    } else {
                        Err(FunctionError::new("Invalid number"))
                    }
                }
                _ => Err(FunctionError::new("Expected number argument")),
            },
        ));

    evaluator
        .function_registry_mut()
        .register_custom(CustomFunction::new(
            "add_one",
            1,
            Some(1),
            |args| match &args[0] {
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Ok(json!(i + 1))
                    } else if let Some(f) = n.as_f64() {
                        Ok(json!(f + 1.0))
                    } else {
                        Err(FunctionError::new("Invalid number"))
                    }
                }
                _ => Err(FunctionError::new("Expected number argument")),
            },
        ));

    // Test function chaining: add_one(double(5)) = add_one(10) = 11
    let parser = Parser::new();
    let expr = parser
        .parse("add_one(double(5))")
        .expect("Test operation failed");
    let result = evaluator
        .evaluate(&expr, &std::collections::HashMap::new())
        .expect("Test operation failed");
    assert_eq!(result, json!(11));
}

#[test]
fn test_function_registry_listing() {
    let mut registry = FunctionRegistry::new();

    // Register some custom functions
    registry.register_custom(CustomFunction::new("custom1", 0, Some(0), |_| Ok(json!(1))));
    registry.register_custom(CustomFunction::new("custom2", 0, Some(0), |_| Ok(json!(2))));

    // Get function names
    let mut names = registry.function_names();
    names.sort(); // Sort for predictable order

    // Should include built-in functions and custom functions
    assert!(names.contains(&"len"));
    assert!(names.contains(&"max"));
    assert!(names.contains(&"min"));
    assert!(names.contains(&"custom1"));
    assert!(names.contains(&"custom2"));
}
