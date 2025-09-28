//! Benchmarks evaluating parser, validation, and memory characteristics of the
//! `LinkML` service.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use linkml_service::parser::Parser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::fmt::Display;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn require_ok<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

fn create_test_schema() -> String {
    r"
id: https://example.org/benchmark
name: BenchmarkSchema

classes:
  Person:
    name: Person
    description: A person for benchmarking
    slots:
      - id
      - name
      - email
      - age
      - address

slots:
  id:
    name: id
    range: string
    required: true
  name:
    name: name
    range: string
    required: true
  email:
    name: email
    range: string
    pattern: '^[^@]+@[^@]+\.[^@]+$'
  age:
    name: age
    range: integer
  address:
    name: address
    range: string
"
    .to_string()
}

fn create_test_data() -> serde_json::Value {
    json!({
        "id": "person-001",
        "name": "John Doe",
        "email": "john.doe@example.com",
        "age": 30,
        "address": "123 Main St, Anytown, USA"
    })
}

/// Benchmark parsing of a moderately sized schema.
fn bench_schema_parsing(c: &mut Criterion) {
    let schema_yaml = create_test_schema();

    c.bench_function("schema_parsing", |b| {
        b.iter(|| {
            let parser = Parser::new();
            let schema = require_ok(
                parser.parse_str(black_box(&schema_yaml), "yaml"),
                "Schema parsing benchmark should succeed",
            );
            black_box(schema)
        })
    });
}

/// Benchmark creation of validation engine instances.
fn bench_validation_engine_creation(c: &mut Criterion) {
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = require_ok(
        parser.parse_str(&schema_yaml, "yaml"),
        "Failed to parse schema for validation engine benchmark",
    );

    c.bench_function("validation_engine_creation", |b| {
        b.iter(|| {
            let engine = require_ok(
                ValidationEngine::new(black_box(&schema)),
                "Validation engine creation should succeed",
            );
            black_box(engine);
        })
    });
}

/// Benchmark validating sample instances against the test schema.
fn bench_instance_validation(c: &mut Criterion) {
    let runtime = require_ok(Runtime::new(), "Tokio runtime creation failed");
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = require_ok(
        parser.parse_str(&schema_yaml, "yaml"),
        "Failed to parse schema for instance validation benchmark",
    );
    let validation_engine = Arc::new(require_ok(
        ValidationEngine::new(&schema),
        "Failed to create validation engine",
    ));
    let test_data = create_test_data();

    c.bench_function("instance_validation", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&validation_engine);
        let payload = test_data.clone();
        b.iter(|| {
            let engine = Arc::clone(&engine);
            let data = payload.clone();
            rt.block_on(async move {
                let report = require_ok(
                    engine
                        .validate_as_class(black_box(&data), "Person", None)
                        .await,
                    "Instance validation should succeed",
                );
                black_box(report);
            });
        })
    });
}

/// Benchmark concurrent validation in a Tokio runtime.
fn bench_concurrent_validation(c: &mut Criterion) {
    let runtime = require_ok(Runtime::new(), "Tokio runtime creation failed");
    let schema_yaml = create_test_schema();
    let parser = Parser::new();
    let schema = require_ok(
        parser.parse_str(&schema_yaml, "yaml"),
        "Failed to parse schema for concurrent validation benchmark",
    );
    let validation_engine = Arc::new(require_ok(
        ValidationEngine::new(&schema),
        "Failed to create validation engine",
    ));
    let test_data = create_test_data();

    c.bench_function("concurrent_validation_10", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&validation_engine);
        let payload = test_data.clone();
        b.iter(|| {
            let engine_outer = Arc::clone(&engine);
            let data = payload.clone();
            rt.block_on(async move {
                let engine_for_tasks = Arc::clone(&engine_outer);
                let tasks = (0..10).map(|_| {
                    let engine = Arc::clone(&engine_for_tasks);
                    let instance = data.clone();
                    async move { engine.validate_as_class(&instance, "Person", None).await }
                });

                let results = futures::future::join_all(tasks).await;
                for result in results {
                    let report = require_ok(result, "Concurrent validation should succeed");
                    black_box(report.summary());
                }
            });
        })
    });
}

/// Benchmark parsing of a large, synthetically generated schema.
fn bench_large_schema_parsing(c: &mut Criterion) {
    // Create a larger schema with many classes and slots
    let mut large_schema = String::from(
        r"
id: https://example.org/large-benchmark
name: LargeBenchmarkSchema

classes:
",
    );

    // Add 100 classes
    for i in 0..100 {
        large_schema.push_str(&format!(
            r"
  Class{}:
    name: Class{}
    description: Test class {}
    slots:
      - field{}_1
      - field{}_2
      - field{}_3
",
            i, i, i, i, i, i
        ));
    }

    large_schema.push_str(
        "
slots:
",
    );

    // Add 300 slots (3 per class)
    for i in 0..100 {
        for j in 1..=3 {
            large_schema.push_str(&format!(
                r"
  field{}_{j}:
    name: field{}_{j}
    range: string
",
                i, i
            ));
        }
    }

    c.bench_function("large_schema_parsing", |b| {
        b.iter(|| {
            let parser = Parser::new();
            let schema = require_ok(
                parser.parse_str(black_box(&large_schema), "yaml"),
                "Large schema parsing benchmark should succeed",
            );
            black_box(schema)
        })
    });
}

/// Benchmark memory usage while parsing multiple schemas.
fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_efficiency", |b| {
        b.iter(|| {
            let mut schemas = Vec::new();

            // Create multiple schemas to test memory usage
            for i in 0..10 {
                let schema_yaml = format!(
                    r"
id: https://example.org/memory-test-{}
name: MemoryTest{}

classes:
  TestClass{}:
    name: TestClass{}
    slots:
      - test_field

slots:
  test_field:
    name: test_field
    range: string
",
                    i, i, i, i
                );

                let parser = Parser::new();
                let schema = require_ok(
                    parser.parse_str(&schema_yaml, "yaml"),
                    "Schema parsing in memory benchmark should succeed",
                );
                schemas.push(schema);
            }

            black_box(schemas)
        })
    });
}

criterion_group!(
    benches,
    bench_schema_parsing,
    bench_validation_engine_creation,
    bench_instance_validation,
    bench_concurrent_validation,
    bench_large_schema_parsing,
    bench_memory_usage
);
criterion_main!(benches);
