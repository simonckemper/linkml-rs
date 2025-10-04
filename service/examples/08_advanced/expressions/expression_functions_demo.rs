//! Demonstration of LinkML expression functions
//!
//! This example shows all the built-in functions available in LinkML expressions.

use linkml_service::expression::{ExpressionEngine, FunctionRegistry};
use serde_json::json;
use std::collections::HashMap;

fn main() {
    println!("LinkML Expression Functions Demo");
    println!(
        "================================
"
    );

    let engine = ExpressionEngine::new();

    // String functions demo
    demo_string_functions(&engine);

    // Date functions demo
    demo_date_functions(&engine);

    // Math functions demo
    demo_math_functions(&engine);

    // Aggregation functions demo
    demo_aggregation_functions(&engine);

    // Show all available functions
    list_all_functions();
}

fn demo_string_functions(engine: &ExpressionEngine) {
    println!("String Functions");
    println!("----------------");

    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("hello world"));
    context.insert("email".to_string(), json!("user@example.com"));

    let examples = vec![
        ("upper(name)", "Convert to uppercase"),
        ("lower(name)", "Convert to lowercase"),
        ("trim(\"  spaced  \")", "Remove leading/trailing whitespace"),
        (
            "starts_with(email, \"user\")",
            "Check if string starts with prefix",
        ),
        (
            "ends_with(email, \".com\")",
            "Check if string ends with suffix",
        ),
        ("replace(name, \"world\", \"rust\")", "Replace substring"),
        ("split(email, \"@\")", "Split string into array"),
        (
            "join(split(email, \"@\"), \" at \")",
            "Join array into string",
        ),
        ("substring(name, 6)", "Extract substring from position"),
        ("substring(name, 0, 5)", "Extract substring with length"),
    ];

    for (expr, desc) in examples {
        match engine.evaluate(expr, &context) {
            Ok(result) => println!("  {}: {} = {:?}", desc, expr, result),
            Err(e) => println!("  Error in {}: {:?}", expr, e),
        }
    }
    println!();
}

fn demo_date_functions(engine: &ExpressionEngine) {
    println!("Date Functions");
    println!("--------------");

    let mut context = HashMap::new();
    context.insert("date".to_string(), json!("2024-01-15"));
    context.insert("date2".to_string(), json!("2024-03-15"));

    let examples = vec![
        ("now()", "Current timestamp"),
        ("today()", "Today's date"),
        (
            "date_parse(\"15/01/2024\", \"%d/%m/%Y\")",
            "Parse date with format",
        ),
        ("date_format(date, \"%B %d, %Y\")", "Format date"),
        ("date_add(date, 10, \"days\")", "Add days to date"),
        ("date_add(date, 2, \"months\")", "Add months to date"),
        (
            "date_diff(date, date2, \"days\")",
            "Difference between dates",
        ),
        ("year(date)", "Extract year"),
        ("month(date)", "Extract month"),
        ("day(date)", "Extract day"),
    ];

    for (expr, desc) in examples {
        match engine.evaluate(expr, &context) {
            Ok(result) => println!("  {}: {} = {:?}", desc, expr, result),
            Err(e) => println!("  Error in {}: {:?}", expr, e),
        }
    }
    println!();
}

fn demo_math_functions(engine: &ExpressionEngine) {
    println!("Math Functions");
    println!("--------------");

    let context = HashMap::new();

    let examples = vec![
        ("abs(-42)", "Absolute value"),
        ("sqrt(16)", "Square root"),
        ("pow(2, 8)", "Power (2^8)"),
        ("sin(0)", "Sine (radians)"),
        ("cos(0)", "Cosine (radians)"),
        ("tan(0)", "Tangent (radians)"),
        ("log(2.718281828459045)", "Natural logarithm"),
        ("log(1000, 10)", "Logarithm base 10"),
        ("exp(1)", "Exponential (e^x)"),
        ("floor(3.7)", "Round down"),
        ("ceil(3.2)", "Round up"),
        ("round(3.14159)", "Round to nearest"),
        ("round(3.14159, 2)", "Round to 2 decimal places"),
        ("mod(17, 5)", "Modulo operation"),
    ];

    for (expr, desc) in examples {
        match engine.evaluate(expr, &context) {
            Ok(result) => println!("  {}: {} = {:?}", desc, expr, result),
            Err(e) => println!("  Error in {}: {:?}", expr, e),
        }
    }
    println!();
}

fn demo_aggregation_functions(engine: &ExpressionEngine) {
    println!("Aggregation Functions");
    println!("--------------------");

    let mut context = HashMap::new();
    context.insert("numbers".to_string(), json!([10, 20, 30, 40, 50]));
    context.insert("grades".to_string(), json!([85, 90, 78, 92, 88, 90, 85]));
    context.insert("mixed".to_string(), json!([1, null, 3, "", 5]));
    context.insert(
        "products".to_string(),
        json!([
            {"type": "electronics", "price": 299},
            {"type": "clothing", "price": 49},
            {"type": "electronics", "price": 599},
            {"type": "clothing", "price": 79},
            {"type": "books", "price": 19}
        ]),
    );

    let examples = vec![
        ("sum(numbers)", "Sum of values"),
        ("avg(numbers)", "Average of values"),
        ("count(numbers)", "Count of values"),
        ("count(mixed, \"non-null\")", "Count non-null values"),
        ("count(mixed, \"non-empty\")", "Count non-empty values"),
        ("median(numbers)", "Median value"),
        ("mode(grades)", "Most frequent value(s)"),
        ("stddev(grades)", "Standard deviation"),
        ("variance(grades)", "Variance"),
        ("unique(grades)", "Unique values"),
        ("group_by(products, \"type\")", "Group by field"),
    ];

    for (expr, desc) in examples {
        match engine.evaluate(expr, &context) {
            Ok(result) => {
                let result_str = if let Some(obj) = result.as_object() {
                    format!("Object with {} groups", obj.len())
                } else {
                    format!("{:?}", result)
                };
                println!("  {}: {} = {}", desc, expr, result_str);
            }
            Err(e) => println!("  Error in {}: {:?}", expr, e),
        }
    }
    println!();
}

fn list_all_functions() {
    println!("All Available Functions");
    println!("----------------------");

    let registry = FunctionRegistry::new();
    let mut functions = registry.function_names();
    functions.sort();

    println!("Total functions: {}", functions.len());
    println!();

    // Group by category
    let string_fns: Vec<_> = functions
        .iter()
        .filter(|f| {
            [
                "upper",
                "lower",
                "trim",
                "starts_with",
                "ends_with",
                "replace",
                "split",
                "join",
                "substring",
            ]
            .contains(f)
        })
        .collect();

    let date_fns: Vec<_> = functions
        .iter()
        .filter(|f| {
            [
                "now",
                "today",
                "date_parse",
                "date_format",
                "date_add",
                "date_diff",
                "year",
                "month",
                "day",
            ]
            .contains(f)
        })
        .collect();

    let math_fns: Vec<_> = functions
        .iter()
        .filter(|f| {
            [
                "abs", "sqrt", "pow", "sin", "cos", "tan", "log", "exp", "floor", "ceil", "round",
                "mod",
            ]
            .contains(f)
        })
        .collect();

    let agg_fns: Vec<_> = functions
        .iter()
        .filter(|f| {
            [
                "sum", "avg", "count", "median", "mode", "stddev", "variance", "unique", "group_by",
            ]
            .contains(f)
        })
        .collect();

    let core_fns: Vec<_> = functions
        .iter()
        .filter(|f| ["len", "max", "min", "case", "matches", "contains"].contains(f))
        .collect();

    println!("Core functions: {:?}", core_fns);
    println!("String functions: {:?}", string_fns);
    println!("Date functions: {:?}", date_fns);
    println!("Math functions: {:?}", math_fns);
    println!("Aggregation functions: {:?}", agg_fns);
}
