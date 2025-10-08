//! Micro-benchmarks for the expression engine.
//!
//! Demonstrates the impact of caching and compilation on evaluation speed.

use anyhow::Result;
use linkml_service::expression::engine_v2::{EngineBuilder, ExpressionEngineV2};
use serde_json::json;
use std::collections::HashMap;
use std::time::Instant;

fn main() -> Result<()> {
    println!("LinkML Expression Performance Demo");
    println!("==================================\n");

    compare_engines()?;
    demonstrate_caching()?;
    demonstrate_compilation()?;

    Ok(())
}

fn compare_engines() -> Result<()> {
    println!("Engine comparison\n------------------");

    let context = basic_context();
    let expressions = [
        ("Arithmetic", "age + 5"),
        ("String", "upper(name)"),
        ("Aggregation", "avg(scores)"),
    ];

    let interpreted = EngineBuilder::new().use_compilation(false).build();
    let optimized = EngineBuilder::new()
        .use_compilation(true)
        .use_caching(true)
        .optimization_level(3)
        .build();

    for (label, expr) in &expressions {
        warm_up(&interpreted, expr, &context);
        warm_up(&optimized, expr, &context);

        let interpreted_time = timed(|| {
            for _ in 0..1_000 {
                interpreted.evaluate(expr, &context).unwrap();
            }
        });

        let optimized_time = timed(|| {
            for _ in 0..1_000 {
                optimized.evaluate(expr, &context).unwrap();
            }
        });

        let speedup = interpreted_time.as_secs_f64() / optimized_time.as_secs_f64();

        println!("{label}");
        println!("  interpreted: {:?}", interpreted_time);
        println!("  optimized:   {:?}", optimized_time);
        println!("  speedup:     {:.2}×\n", speedup);
    }

    Ok(())
}

fn demonstrate_caching() -> Result<()> {
    println!("Cache effectiveness\n-------------------");

    let engine = EngineBuilder::new()
        .use_compilation(true)
        .use_caching(true)
        .cache_capacity(128)
        .build();
    let context = HashMap::new();

    let cold = timed(|| {
        for i in 0..50 {
            let expr = format!("value_{i}");
            engine.evaluate(&expr, &context).unwrap();
        }
    });

    let warm = timed(|| {
        for i in 0..50 {
            let expr = format!("value_{i}");
            engine.evaluate(&expr, &context).unwrap();
        }
    });

    println!("cold run: {:?}", cold);
    println!("warm run: {:?}", warm);
    println!(
        "speedup:  {:.2}×\n",
        cold.as_secs_f64() / warm.as_secs_f64()
    );

    Ok(())
}

fn demonstrate_compilation() -> Result<()> {
    println!("Compilation benefits\n--------------------");

    let expr = "sum(values) + avg(values)";
    let mut context = HashMap::new();
    context.insert("values".to_string(), json!((0..100).collect::<Vec<_>>()));

    let interpreted = EngineBuilder::new().use_compilation(false).build();
    let compiled = EngineBuilder::new().use_compilation(true).build();

    warm_up(&interpreted, expr, &context);
    warm_up(&compiled, expr, &context);

    let interpreted_time = timed(|| {
        for _ in 0..500 {
            interpreted.evaluate(expr, &context).unwrap();
        }
    });

    let compiled_time = timed(|| {
        for _ in 0..500 {
            compiled.evaluate(expr, &context).unwrap();
        }
    });

    println!("expression: {expr}");
    println!("  interpreted: {:?}", interpreted_time);
    println!("  compiled:    {:?}", compiled_time);
    println!(
        "  speedup:     {:.2}×\n",
        interpreted_time.as_secs_f64() / compiled_time.as_secs_f64()
    );

    Ok(())
}

fn basic_context() -> HashMap<String, serde_json::Value> {
    let mut context = HashMap::new();
    context.insert("age".to_string(), json!(32));
    context.insert("name".to_string(), json!("Ada Lovelace"));
    context.insert("scores".to_string(), json!([91, 87, 95, 89]));
    context
}

fn warm_up(engine: &ExpressionEngineV2, expr: &str, ctx: &HashMap<String, serde_json::Value>) {
    let _ = engine.evaluate(expr, ctx);
}

fn timed<F: FnOnce()>(f: F) -> std::time::Duration {
    let start = Instant::now();
    f();
    start.elapsed()
}
