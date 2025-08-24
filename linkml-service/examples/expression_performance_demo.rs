//! Demonstration of expression performance optimizations
//!
//! This example shows the performance benefits of JIT compilation and caching
//! for LinkML expressions.

use linkml_service::expression::{
    ExpressionEngine,
    engine_v2::{EngineBuilder, ExpressionEngineV2},
};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

fn main() {
    println!("LinkML Expression Performance Demo");
    println!("==================================\n");

    // Create test data
    let context = create_test_context();
    let expressions = create_test_expressions();

    // Compare different engine configurations
    compare_engines(&expressions, &context);

    // Demonstrate caching benefits
    demonstrate_caching();

    // Show compilation benefits for complex expressions
    demonstrate_compilation();

    // Benchmark real-world scenarios
    benchmark_scenarios();
}

fn create_test_context() -> HashMap<String, serde_json::Value> {
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(25));
    context.insert("name".to_string(), json!("John Doe"));
    context.insert("scores".to_string(), json!([85, 90, 78, 92, 88]));
    context.insert(
        "profile".to_string(),
        json!({
            "active": true,
            "role": "user",
            "permissions": ["read", "write"]
        }),
    );
    context
}

fn create_test_expressions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Simple arithmetic", "age + 5"),
        ("String manipulation", "upper(name)"),
        ("Complex calculation", "avg(scores) * 1.1 + stddev(scores)"),
        (
            "Conditional logic",
            "case(age >= 18, \"adult\", age >= 13, \"teen\", \"child\")",
        ),
        (
            "Field access",
            "profile.active && contains(profile.permissions, \"write\")",
        ),
        (
            "Function composition",
            "round(sqrt(sum(scores)) / count(scores), 2)",
        ),
    ]
}

fn compare_engines(expressions: &[(&str, &str)], context: &HashMap<String, serde_json::Value>) {
    println!("Engine Performance Comparison");
    println!("-----------------------------\n");

    // V1 Engine (baseline)
    let v1_engine = ExpressionEngine::new();

    // V2 Engine with all optimizations
    let v2_engine = EngineBuilder::new()
        .use_compilation(true)
        .use_caching(true)
        .optimization_level(3)
        .collect_metrics(true)
        .build();

    // V2 Engine without optimizations
    let v2_no_opt = EngineBuilder::new()
        .use_compilation(false)
        .use_caching(false)
        .collect_metrics(true)
        .build();

    // Warm up caches
    for (_, expr) in expressions {
        let _ = v2_engine.evaluate(expr, context);
    }

    // Benchmark each expression
    for (name, expr) in expressions {
        println!("Expression: {} - {}", name, expr);

        // V1 timing
        let start = Instant::now();
        for _ in 0..1000 {
            v1_engine.evaluate(expr, context).unwrap();
        }
        let v1_time = start.elapsed();

        // V2 with optimizations
        let start = Instant::now();
        for _ in 0..1000 {
            v2_engine.evaluate(expr, context).unwrap();
        }
        let v2_opt_time = start.elapsed();

        // V2 without optimizations
        let start = Instant::now();
        for _ in 0..1000 {
            v2_no_opt.evaluate(expr, context).unwrap();
        }
        let v2_no_opt_time = start.elapsed();

        println!("  V1 Engine:          {:?}", v1_time);
        println!("  V2 (no opt):        {:?}", v2_no_opt_time);
        println!("  V2 (optimized):     {:?}", v2_opt_time);
        println!(
            "  Speedup:            {:.2}x",
            v1_time.as_nanos() as f64 / v2_opt_time.as_nanos() as f64
        );
        println!();
    }

    // Show V2 metrics
    let metrics = v2_engine.metrics();
    println!("V2 Engine Metrics:");
    println!("  Total evaluations:     {}", metrics.total_evaluations);
    println!("  Compiled evaluations:  {}", metrics.compiled_evaluations);
    println!(
        "  Cache hit rate:        {:.2}%",
        metrics.cache_hit_rate * 100.0
    );
    println!("  Parse time:            {} µs", metrics.parse_time_us);
    println!("  Compile time:          {} µs", metrics.compile_time_us);
    println!("  Eval time:             {} µs", metrics.eval_time_us);
    println!();
}

fn demonstrate_caching() {
    println!("Cache Performance Demonstration");
    println!("-------------------------------\n");

    let engine = EngineBuilder::new()
        .cache_capacity(100)
        .collect_metrics(true)
        .build();

    let context: HashMap<String, serde_json::Value> = HashMap::new();

    // First pass - populate cache
    println!("First pass (cache misses):");
    let start = Instant::now();
    for i in 0..50 {
        let expr = format!("{} + {}", i, i + 1);
        engine.evaluate(&expr, &context).unwrap();
    }
    let first_pass = start.elapsed();
    println!("  Time: {:?}", first_pass);

    // Second pass - cache hits
    println!("Second pass (cache hits):");
    let start = Instant::now();
    for i in 0..50 {
        let expr = format!("{} + {}", i, i + 1);
        engine.evaluate(&expr, &context).unwrap();
    }
    let second_pass = start.elapsed();
    println!("  Time: {:?}", second_pass);
    println!(
        "  Speedup: {:.2}x",
        first_pass.as_nanos() as f64 / second_pass.as_nanos() as f64
    );

    let metrics = engine.metrics();
    println!("  Cache hit rate: {:.2}%", metrics.cache_hit_rate * 100.0);
    println!();
}

fn demonstrate_compilation() {
    println!("Compilation Benefits for Complex Expressions");
    println!("-------------------------------------------\n");

    let mut context = HashMap::new();
    context.insert("values".to_string(), json!((0..100).collect::<Vec<_>>()));

    // Complex expression that benefits from compilation
    let complex_expr = "sum(values) + avg(values) * stddev(values) - median(values) / 2";

    // Engine with compilation
    let compiled_engine = EngineBuilder::new()
        .use_compilation(true)
        .optimization_level(3)
        .collect_metrics(true)
        .build();

    // Engine without compilation
    let interpreted_engine = EngineBuilder::new()
        .use_compilation(false)
        .collect_metrics(true)
        .build();

    // Warm up
    compiled_engine.evaluate(complex_expr, &context).unwrap();
    interpreted_engine.evaluate(complex_expr, &context).unwrap();

    // Benchmark
    let iterations = 100;

    let start = Instant::now();
    for _ in 0..iterations {
        interpreted_engine.evaluate(complex_expr, &context).unwrap();
    }
    let interpreted_time = start.elapsed();

    let start = Instant::now();
    for _ in 0..iterations {
        compiled_engine.evaluate(complex_expr, &context).unwrap();
    }
    let compiled_time = start.elapsed();

    println!("Expression: {}", complex_expr);
    println!("  Interpreted: {:?}", interpreted_time);
    println!("  Compiled:    {:?}", compiled_time);
    println!(
        "  Speedup:     {:.2}x",
        interpreted_time.as_nanos() as f64 / compiled_time.as_nanos() as f64
    );
    println!();
}

fn benchmark_scenarios() {
    println!("Real-World Scenario Benchmarks");
    println!("------------------------------\n");

    // Scenario 1: Validation rules
    benchmark_validation_scenario();

    // Scenario 2: Computed fields
    benchmark_computed_fields_scenario();

    // Scenario 3: Filtering and transformation
    benchmark_filtering_scenario();
}

fn benchmark_validation_scenario() {
    println!("Scenario: Schema Validation Rules");

    let engine = EngineBuilder::new().cache_capacity(500).build();

    // Simulate validating many records
    let rules = vec![
        "age >= 0 && age <= 150",
        "len(email) > 0 && contains(email, \"@\")",
        "salary >= min_salary && salary <= max_salary",
        "start_date < end_date",
        "status == \"active\" || status == \"pending\"",
    ];

    let mut total_time = std::time::Duration::ZERO;

    for i in 0..1000 {
        let mut context = HashMap::new();
        context.insert("age".to_string(), json!(25 + i % 50));
        context.insert("email".to_string(), json!(format!("user{}@example.com", i)));
        context.insert("salary".to_string(), json!(50000 + i * 100));
        context.insert("min_salary".to_string(), json!(30000));
        context.insert("max_salary".to_string(), json!(200000));
        context.insert("start_date".to_string(), json!("2024-01-01"));
        context.insert("end_date".to_string(), json!("2024-12-31"));
        context.insert(
            "status".to_string(),
            json!(if i % 2 == 0 { "active" } else { "pending" }),
        );

        let start = Instant::now();
        for rule in &rules {
            engine.evaluate(rule, &context).unwrap();
        }
        total_time += start.elapsed();
    }

    println!("  Validated 1000 records with 5 rules each");
    println!("  Total time: {:?}", total_time);
    println!("  Per record: {:?}", total_time / 1000);
    println!();
}

fn benchmark_computed_fields_scenario() {
    println!("Scenario: Computed Fields");

    let engine = EngineBuilder::new().build();

    // Precompile common computed field expressions
    engine
        .precompile("first_name + \" \" + last_name", Some("person"))
        .unwrap();
    engine
        .precompile("round(price * (1 - discount / 100), 2)", Some("product"))
        .unwrap();
    engine
        .precompile("year(now()) - year(birth_date)", Some("person"))
        .unwrap();

    let mut context = HashMap::new();
    context.insert("first_name".to_string(), json!("John"));
    context.insert("last_name".to_string(), json!("Doe"));
    context.insert("price".to_string(), json!(99.99));
    context.insert("discount".to_string(), json!(15));
    context.insert("birth_date".to_string(), json!("1990-05-15"));

    let start = Instant::now();
    for _ in 0..10000 {
        engine
            .evaluate_with_schema("first_name + \" \" + last_name", &context, Some("person"))
            .unwrap();
        engine
            .evaluate_with_schema(
                "round(price * (1 - discount / 100), 2)",
                &context,
                Some("product"),
            )
            .unwrap();
        engine
            .evaluate_with_schema("year(now()) - year(birth_date)", &context, Some("person"))
            .unwrap();
    }
    let elapsed = start.elapsed();

    println!("  Computed 30,000 fields (10,000 records × 3 fields)");
    println!("  Total time: {:?}", elapsed);
    println!("  Per field: {:?}", elapsed / 30_000);
    println!();
}

fn benchmark_filtering_scenario() {
    println!("Scenario: Data Filtering and Transformation");

    let engine = EngineBuilder::new().optimization_level(3).build();

    // Create a dataset
    let mut records = Vec::new();
    for i in 0..100 {
        records.push(json!({
            "id": i,
            "type": if i % 3 == 0 { "A" } else if i % 3 == 1 { "B" } else { "C" },
            "value": i * 10,
            "active": i % 2 == 0
        }));
    }

    let mut context = HashMap::new();
    context.insert("records".to_string(), json!(records));

    // Complex filtering and aggregation
    let expressions = vec![
        "count(records, \"non-null\")",
        "avg(group_by(records, \"type\").\"\\\"A\\\"\".value)",
        "sum(records.value)",
    ];

    let start = Instant::now();
    for _ in 0..1000 {
        for expr in &expressions {
            engine.evaluate(expr, &context).unwrap();
        }
    }
    let elapsed = start.elapsed();

    println!("  Performed 3,000 filtering/aggregation operations");
    println!("  Total time: {:?}", elapsed);
    println!("  Per operation: {:?}", elapsed / 3_000);
}
