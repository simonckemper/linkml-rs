//! Integration tests for all expression functions

use linkml_service::expression::{ExpressionEngine, FunctionRegistry};
use serde_json::json;
use std::collections::HashMap;

#[test]
fn test_string_functions_integration() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("text".to_string(), json!("Hello World"));

    // Test upper
    let result = engine
        .evaluate("upper(text)", &context)
        ?;
    assert_eq!(result, json!("HELLO WORLD"));

    // Test lower
    let result = engine
        .evaluate("lower(text)", &context)
        ?;
    assert_eq!(result, json!("hello world"));

    // Test trim
    context.insert("padded".to_string(), json!("  trim me  "));
    let result = engine
        .evaluate("trim(padded)", &context)
        ?;
    assert_eq!(result, json!("trim me"));

    // Test starts_with
    let result = engine
        .evaluate("starts_with(text, \"Hello\")", &context)
        ?;
    assert_eq!(result, json!(true));

    // Test ends_with
    let result = engine
        .evaluate("ends_with(text, \"World\")", &context)
        ?;
    assert_eq!(result, json!(true));

    // Test replace
    let result = engine
        .evaluate("replace(text, \"World\", \"Rust\")", &context)
        ?;
    assert_eq!(result, json!("Hello Rust"));

    // Test split
    let result = engine
        .evaluate("split(text, \" \")", &context)
        ?;
    assert_eq!(result, json!(["Hello", "World"]));

    // Test join
    context.insert("words".to_string(), json!(["Hello", "World"]));
    let result = engine
        .evaluate("join(words, \"-\")", &context)
        ?;
    assert_eq!(result, json!("Hello-World"));

    // Test substring
    let result = engine
        .evaluate("substring(text, 6)", &context)
        ?;
    assert_eq!(result, json!("World"));

    let result = engine
        .evaluate("substring(text, 0, 5)", &context)
        ?;
    assert_eq!(result, json!("Hello"));
    Ok(())
}

#[test]
fn test_date_functions_integration() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("date".to_string(), json!("2024-01-15"));

    // Test now and today (just verify they return strings)
    let result = engine
        .evaluate("now()", &context)
        ?;
    assert!(result.is_string());

    let result = engine
        .evaluate("today()", &context)
        ?;
    assert!(result.is_string());

    // Test date_parse
    let result = engine
        .evaluate("date_parse(\"15/01/2024\", \"%d/%m/%Y\")", &context)
        ?;
    assert_eq!(result, json!("2024-01-15"));

    // Test date_format
    let result = engine
        .evaluate("date_format(date, \"%Y/%m/%d\")", &context)
        ?;
    assert_eq!(result, json!("2024/01/15"));

    // Test date_add
    let result = engine
        .evaluate("date_add(date, 10, \"days\")", &context)
        ?;
    assert_eq!(result, json!("2024-01-25"));

    let result = engine
        .evaluate("date_add(date, 2, \"months\")", &context)
        ?;
    assert_eq!(result, json!("2024-03-15"));

    // Test date_diff
    context.insert("date2".to_string(), json!("2024-01-25"));
    let result = engine
        .evaluate("date_diff(date, date2, \"days\")", &context)
        ?;
    assert_eq!(result, json!(10));

    // Test year, month, day
    let result = engine
        .evaluate("year(date)", &context)
        ?;
    assert_eq!(result, json!(2024));

    let result = engine
        .evaluate("month(date)", &context)
        ?;
    assert_eq!(result, json!(1));

    let result = engine
        .evaluate("day(date)", &context)
        ?;
    assert_eq!(result, json!(15));
    Ok(())
}

#[test]
fn test_math_functions_integration() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ExpressionEngine::new();
    let context = HashMap::new();

    // Test abs
    let result = engine
        .evaluate("abs(-5)", &context)
        ?;
    assert_eq!(result, json!(5.0));

    // Test sqrt
    let result = engine
        .evaluate("sqrt(16)", &context)
        ?;
    assert_eq!(result, json!(4.0));

    // Test pow
    let result = engine
        .evaluate("pow(2, 3)", &context)
        ?;
    assert_eq!(result, json!(8.0));

    // Test trigonometric functions
    let result = engine
        .evaluate("sin(0)", &context)
        ?;
    assert_eq!(result, json!(0.0));

    let result = engine
        .evaluate("cos(0)", &context)
        ?;
    assert_eq!(result, json!(1.0));

    let result = engine
        .evaluate("tan(0)", &context)
        ?;
    assert_eq!(result, json!(0.0));

    // Test log
    let result = engine
        .evaluate("log(2.718281828459045)", &context)
        ?;
    assert_eq!(result, json!(1.0));

    let result = engine
        .evaluate("log(100, 10)", &context)
        ?;
    assert_eq!(result, json!(2.0));

    // Test exp
    let result = engine
        .evaluate("exp(0)", &context)
        ?;
    assert_eq!(result, json!(1.0));

    // Test rounding functions
    let result = engine
        .evaluate("floor(3.7)", &context)
        ?;
    assert_eq!(result, json!(3));

    let result = engine
        .evaluate("ceil(3.2)", &context)
        ?;
    assert_eq!(result, json!(4));

    let result = engine
        .evaluate("round(3.5)", &context)
        ?;
    assert_eq!(result, json!(4.0));

    let result = engine
        .evaluate("round(3.14159, 2)", &context)
        ?;
    assert_eq!(result, json!(3.14));

    // Test mod
    let result = engine
        .evaluate("mod(10, 3)", &context)
        ?;
    assert_eq!(result, json!(1.0));
    Ok(())
}

#[test]
fn test_aggregation_functions_integration() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("numbers".to_string(), json!([1, 2, 3, 4, 5]));

    // Test sum
    let result = engine
        .evaluate("sum(numbers)", &context)
        ?;
    assert_eq!(result, json!(15.0));

    // Test avg
    let result = engine
        .evaluate("avg(numbers)", &context)
        ?;
    assert_eq!(result, json!(3.0));

    // Test count
    let result = engine
        .evaluate("count(numbers)", &context)
        ?;
    assert_eq!(result, json!(5));

    context.insert("mixed".to_string(), json!([1, null, 3, null, 5]));
    let result = engine
        .evaluate("count(mixed, \"non-null\")", &context)
        ?;
    assert_eq!(result, json!(3));

    // Test median
    let result = engine
        .evaluate("median(numbers)", &context)
        ?;
    assert_eq!(result, json!(3.0));

    // Test mode
    context.insert("modes".to_string(), json!([1, 2, 2, 3, 2, 4]));
    let result = engine
        .evaluate("mode(modes)", &context)
        ?;
    assert_eq!(result, json!(2));

    // Test stddev and variance
    context.insert("stats".to_string(), json!([2, 4, 4, 4, 5, 5, 7, 9]));
    let result = engine
        .evaluate("variance(stats)", &context)
        ?;
    assert_eq!(result, json!(4.0));

    let result = engine
        .evaluate("stddev(stats)", &context)
        ?;
    assert_eq!(result, json!(2.0));

    // Test unique
    context.insert("dupes".to_string(), json!([1, 2, 2, 3, 1, 4, 3]));
    let result = engine
        .evaluate("unique(dupes)", &context)
        ?;
    assert_eq!(result, json!([1, 2, 3, 4]));

    // Test group_by
    context.insert(
        "objects".to_string(),
        json!([
            {"type": "fruit", "name": "apple"},
            {"type": "vegetable", "name": "carrot"},
            {"type": "fruit", "name": "banana"}
        ]),
    );
    let result = engine
        .evaluate("group_by(objects, \"type\")", &context)
        ?;
    assert!(result.is_object());
    assert!(result.get("\"fruit\"").is_some());
    assert!(result.get("\"vegetable\"").is_some());
    Ok(())
}

#[test]
fn test_function_registry_completeness() -> Result<(), Box<dyn std::error::Error>> {
    let registry = FunctionRegistry::new();

    // Verify all string functions are registered
    assert!(registry.has_function("upper"));
    assert!(registry.has_function("lower"));
    assert!(registry.has_function("trim"));
    assert!(registry.has_function("starts_with"));
    assert!(registry.has_function("ends_with"));
    assert!(registry.has_function("replace"));
    assert!(registry.has_function("split"));
    assert!(registry.has_function("join"));
    assert!(registry.has_function("substring"));

    // Verify all date functions are registered
    assert!(registry.has_function("now"));
    assert!(registry.has_function("today"));
    assert!(registry.has_function("date_parse"));
    assert!(registry.has_function("date_format"));
    assert!(registry.has_function("date_add"));
    assert!(registry.has_function("date_diff"));
    assert!(registry.has_function("year"));
    assert!(registry.has_function("month"));
    assert!(registry.has_function("day"));

    // Verify all math functions are registered
    assert!(registry.has_function("abs"));
    assert!(registry.has_function("sqrt"));
    assert!(registry.has_function("pow"));
    assert!(registry.has_function("sin"));
    assert!(registry.has_function("cos"));
    assert!(registry.has_function("tan"));
    assert!(registry.has_function("log"));
    assert!(registry.has_function("exp"));
    assert!(registry.has_function("floor"));
    assert!(registry.has_function("ceil"));
    assert!(registry.has_function("round"));
    assert!(registry.has_function("mod"));

    // Verify all aggregation functions are registered
    assert!(registry.has_function("sum"));
    assert!(registry.has_function("avg"));
    assert!(registry.has_function("count"));
    assert!(registry.has_function("median"));
    assert!(registry.has_function("mode"));
    assert!(registry.has_function("stddev"));
    assert!(registry.has_function("variance"));
    assert!(registry.has_function("unique"));
    assert!(registry.has_function("group_by"));
    Ok(())
}

#[test]
fn test_complex_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let engine = ExpressionEngine::new();
    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("john doe"));
    context.insert("age".to_string(), json!(25));
    context.insert("scores".to_string(), json!([85, 90, 78, 92, 88]));

    // Complex string manipulation
    let result = engine
        .evaluate(
            "upper(substring(name, 0, 1)) + lower(substring(name, 1))",
            &context,
        )
        ?;
    assert_eq!(result, json!("John doe"));

    // Complex math with aggregation
    let result = engine
        .evaluate("round(avg(scores) + stddev(scores), 2)", &context)
        ?;
    // avg = 86.6, stddev ≈ 5.41, sum ≈ 92.01
    assert!(
        result.as_f64()? > 92.0
            && result.as_f64()? < 93.0
    );

    // Conditional with functions
    let result = engine
        .evaluate(
            "case(age > 18, \"Adult: \" + upper(name), \"Minor\")",
            &context,
        )
        ?;
    assert_eq!(result, json!("Adult: JOHN DOE"));
    Ok(())
}
