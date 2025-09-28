//! Performance benchmarks for LinkML validation engine
//!
//! This benchmark suite measures validation performance under various scenarios
//! to ensure the service meets performance requirements and identifies bottlenecks.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_service::validator::{
    ValidationEngine, ValidationOptions, cache::CompiledValidatorCache,
};
use serde_json::{Value, json};
use std::sync::Arc;

/// Create a simple test schema
fn create_simple_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "bench-simple".to_string(),
        name: "SimpleBench".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "Person".to_string(),
        slots: vec!["name".to_string(), "age".to_string(), "email".to_string()],
        ..Default::default()
    };
    schema.classes.insert("Person".to_string(), class_def);

    let name_slot = SlotDefinition {
        name: "name".to_string(),
        range: Some("string".to_string()),
        required: Some(true),
        ..Default::default()
    };
    schema.slots.insert("name".to_string(), name_slot);

    let age_slot = SlotDefinition {
        name: "age".to_string(),
        range: Some("integer".to_string()),
        required: Some(false),
        ..Default::default()
    };
    schema.slots.insert("age".to_string(), age_slot);

    let email_slot = SlotDefinition {
        name: "email".to_string(),
        range: Some("string".to_string()),
        pattern: Some(r"^[^@]+@[^@]+\.[^@]+$".to_string()),
        required: Some(false),
        ..Default::default()
    };
    schema.slots.insert("email".to_string(), email_slot);

    schema
}

/// Create a complex schema with inheritance and patterns
fn create_complex_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition {
        id: "bench-complex".to_string(),
        name: "ComplexBench".to_string(),
        ..Default::default()
    };

    // Base class
    let entity_class = ClassDefinition {
        name: "Entity".to_string(),
        slots: vec!["id".to_string(), "created_at".to_string()],
        ..Default::default()
    };
    schema.classes.insert("Entity".to_string(), entity_class);

    // Person inherits from Entity
    let person_class = ClassDefinition {
        name: "Person".to_string(),
        is_a: Some("Entity".to_string()),
        slots: vec!["name".to_string(), "email".to_string(), "phone".to_string()],
        ..Default::default()
    };
    schema.classes.insert("Person".to_string(), person_class);

    // Employee inherits from Person
    let employee_class = ClassDefinition {
        name: "Employee".to_string(),
        is_a: Some("Person".to_string()),
        slots: vec![
            "employee_id".to_string(),
            "department".to_string(),
            "salary".to_string(),
        ],
        ..Default::default()
    };
    schema
        .classes
        .insert("Employee".to_string(), employee_class);

    // Define all slots
    let slots = vec![
        ("id", "string", true, Some(r"^[A-Z]{2}-\d{6}$")),
        ("created_at", "datetime", true, None),
        ("name", "string", true, None),
        (
            "email",
            "string",
            true,
            Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"),
        ),
        ("phone", "string", false, Some(r"^\+?[\d\s\-\(\)]{10,}$")),
        ("employee_id", "string", true, Some(r"^EMP-\d{4}$")),
        ("department", "string", true, None),
        ("salary", "float", false, None),
    ];

    for (name, range, required, pattern) in slots {
        let slot_def = SlotDefinition {
            name: name.to_string(),
            range: Some(range.to_string()),
            required: Some(required),
            pattern: pattern.map(|p| p.to_string()),
            ..Default::default()
        };
        schema.slots.insert(name.to_string(), slot_def);
    }

    schema
}

/// Generate test data for benchmarking
fn generate_test_data(count: usize, complex: bool) -> Vec<Value> {
    let mut data = Vec::with_capacity(count);

    for i in 0..count {
        let item = if complex {
            json!({
                "id": format!("US-{:06}", i),
                "created_at": "2024-01-01T00:00:00Z",
                "name": format!("Employee {}", i),
                "email": format!("employee{}@company.com", i),
                "phone": format!("+1-555-{:04}", i % 10000),
                "employee_id": format!("EMP-{:04}", i),
                "department": match i % 4 {
                    0 => "Engineering",
                    1 => "Sales",
                    2 => "Marketing",
                    _ => "Support"
                },
                "salary": 50000.0 + (f64::from(i) * 1000.0)
            })
        } else {
            json!({
                "name": format!("Person {}", i),
                "age": 20 + (i % 60),
                "email": format!("person{}@example.com", i)
            })
        };
        data.push(item);
    }

    data
}

/// Benchmark single validation performance
fn bench_single_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let simple_schema = create_simple_schema();
    let complex_schema = create_complex_schema();

    let simple_engine = ValidationEngine::new(&simple_schema).unwrap();
    let complex_engine = ValidationEngine::new(&complex_schema).unwrap();

    let simple_data = json!({
        "name": "John Doe",
        "age": 30,
        "email": "john@example.com"
    });

    let complex_data = json!({
        "id": "US-123456",
        "created_at": "2024-01-01T00:00:00Z",
        "name": "John Smith",
        "email": "john.smith@company.com",
        "phone": "+1-555-1234",
        "employee_id": "EMP-1234",
        "department": "Engineering",
        "salary": 75000.0
    });

    let mut group = c.benchmark_group("single_validation");

    group.bench_function("simple_schema", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = simple_engine
                .validate_as_class(black_box(&simple_data), "Person", None)
                .await;
            black_box(result)
        })
    });

    group.bench_function("complex_schema", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = complex_engine
                .validate_as_class(black_box(&complex_data), "Employee", None)
                .await;
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark batch validation performance
fn bench_batch_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let simple_schema = create_simple_schema();
    let complex_schema = create_complex_schema();

    let simple_engine = ValidationEngine::new(&simple_schema).unwrap();
    let complex_engine = ValidationEngine::new(&complex_schema).unwrap();

    let batch_sizes = vec![10, 100, 1000];

    let mut group = c.benchmark_group("batch_validation");

    for size in batch_sizes {
        group.throughput(Throughput::Elements(u64::try_from(size).unwrap_or(0)));

        let simple_data = generate_test_data(size, false);
        let complex_data = generate_test_data(size, true);

        group.bench_with_input(
            BenchmarkId::new("simple_schema", size),
            &size,
            |b, &_size| {
                b.to_async(&runtime).iter(|| async {
                    let mut results = Vec::new();
                    for item in &simple_data {
                        let result = simple_engine
                            .validate_as_class(black_box(item), "Person", None)
                            .await;
                        results.push(result);
                    }
                    black_box(results)
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("complex_schema", size),
            &size,
            |b, &_size| {
                b.to_async(&runtime).iter(|| async {
                    let mut results = Vec::new();
                    for item in &complex_data {
                        let result = complex_engine
                            .validate_as_class(black_box(item), "Employee", None)
                            .await;
                        results.push(result);
                    }
                    black_box(results)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark collection validation performance
fn bench_collection_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let simple_schema = create_simple_schema();
    let complex_schema = create_complex_schema();

    let batch_sizes = vec![10, 100, 1000];

    let mut group = c.benchmark_group("collection_validation");

    for size in batch_sizes {
        group.throughput(Throughput::Elements(u64::try_from(size).unwrap_or(0)));

        let simple_data = generate_test_data(size, false);
        let complex_data = generate_test_data(size, true);

        group.bench_with_input(
            BenchmarkId::new("simple_schema", size),
            &size,
            |b, &_size| {
                b.to_async(&runtime).iter(|| {
                    let simple_schema = simple_schema.clone();
                    let simple_data = simple_data.clone();
                    async move {
                        let mut simple_engine = ValidationEngine::new(&simple_schema).unwrap();
                        let result = simple_engine
                            .validate_collection(black_box(&simple_data), "Person", None)
                            .await;
                        black_box(result)
                    }
                })
            },
        );

        group.bench_with_input(
            BenchmarkId::new("complex_schema", size),
            &size,
            |b, &_size| {
                b.to_async(&runtime).iter(|| {
                    let complex_schema = complex_schema.clone();
                    let complex_data = complex_data.clone();
                    async move {
                        let mut complex_engine = ValidationEngine::new(&complex_schema).unwrap();
                        let result = complex_engine
                            .validate_collection(black_box(&complex_data), "Employee", None)
                            .await;
                        black_box(result)
                    }
                })
            },
        );
    }

    group.finish();
}

/// Benchmark cached vs non-cached validation
fn bench_cached_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let schema = create_complex_schema();
    let cache = Arc::new(CompiledValidatorCache::with_config(100, 10 * 1024 * 1024));

    let engine_no_cache = ValidationEngine::new(&schema).unwrap();
    let engine_with_cache = ValidationEngine::with_cache(&schema, cache).unwrap();

    let test_data = json!({
        "id": "US-123456",
        "created_at": "2024-01-01T00:00:00Z",
        "name": "John Smith",
        "email": "john.smith@company.com",
        "phone": "+1-555-1234",
        "employee_id": "EMP-1234",
        "department": "Engineering",
        "salary": 75000.0
    });

    let mut group = c.benchmark_group("cached_validation");

    group.bench_function("no_cache", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine_no_cache
                .validate_as_class(black_box(&test_data), "Employee", None)
                .await;
            black_box(result)
        })
    });

    group.bench_function("with_cache_cold", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine_with_cache
                .validate_as_class(black_box(&test_data), "Employee", None)
                .await;
            black_box(result)
        })
    });

    // Warm up the cache
    runtime.block_on(async {
        let _ = engine_with_cache
            .validate_as_class(&test_data, "Employee", None)
            .await;
    });

    group.bench_function("with_cache_warm", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine_with_cache
                .validate_as_class(black_box(&test_data), "Employee", None)
                .await;
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark different validation options
fn bench_validation_options(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let schema = create_complex_schema();
    let engine = ValidationEngine::new(&schema).unwrap();

    let test_data = json!({
        "id": "US-123456",
        "created_at": "2024-01-01T00:00:00Z",
        "name": "John Smith",
        "email": "john.smith@company.com",
        "phone": "+1-555-1234",
        "employee_id": "EMP-1234",
        "department": "Engineering",
        "salary": 75000.0
    });

    let mut group = c.benchmark_group("validation_options");

    // Default options
    group.bench_function("default_options", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate_as_class(black_box(&test_data), "Employee", None)
                .await;
            black_box(result)
        })
    });

    // Fail fast enabled
    let mut fail_fast_options = ValidationOptions::default();
    fail_fast_options.fail_fast = Some(true);

    group.bench_function("fail_fast", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate_as_class(
                    black_box(&test_data),
                    "Employee",
                    Some(fail_fast_options.clone()),
                )
                .await;
            black_box(result)
        })
    });

    // Permissible checks disabled
    let mut no_permissible_options = ValidationOptions::default();
    no_permissible_options.check_permissibles = Some(false);

    group.bench_function("no_permissibles", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate_as_class(
                    black_box(&test_data),
                    "Employee",
                    Some(no_permissible_options.clone()),
                )
                .await;
            black_box(result)
        })
    });

    // Cache disabled
    let mut no_cache_options = ValidationOptions::default();
    no_cache_options.use_cache = Some(false);

    group.bench_function("no_cache", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate_as_class(
                    black_box(&test_data),
                    "Employee",
                    Some(no_cache_options.clone()),
                )
                .await;
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark pattern validation performance
fn bench_pattern_validation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let mut schema = SchemaDefinition {
        id: "bench-patterns".to_string(),
        name: "PatternBench".to_string(),
        ..Default::default()
    };

    let class_def = ClassDefinition {
        name: "PatternTest".to_string(),
        slots: vec![
            "simple_pattern".to_string(),
            "complex_pattern".to_string(),
            "email_pattern".to_string(),
        ],
        ..Default::default()
    };
    schema.classes.insert("PatternTest".to_string(), class_def);

    // Simple pattern
    let simple_slot = SlotDefinition {
        name: "simple_pattern".to_string(),
        range: Some("string".to_string()),
        pattern: Some(r"^\d{3}-\d{3}-\d{4}$".to_string()),
        ..Default::default()
    };
    schema
        .slots
        .insert("simple_pattern".to_string(), simple_slot);

    // Complex pattern (potentially slow)
    let complex_slot = SlotDefinition {
        name: "complex_pattern".to_string(),
        range: Some("string".to_string()),
        pattern: Some(r#"^(?:[a-z0-9!#$%&'*+/=?^_`{|}~-]+(?:\.[a-z0-9!#$%&'*+/=?^_`{|}~-]+)*|"(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21\x23-\x5b\x5d-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])*")@(?:(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\.)+[a-z0-9](?:[a-z0-9-]*[a-z0-9])?|\[(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?|[a-z0-9-]*[a-z0-9]:(?:[\x01-\x08\x0b\x0c\x0e-\x1f\x21-\x5a\x53-\x7f]|\\[\x01-\x09\x0b\x0c\x0e-\x7f])+)\])$"#.to_string()),
        ..Default::default()
    };
    schema
        .slots
        .insert("complex_pattern".to_string(), complex_slot);

    // Email pattern
    let email_slot = SlotDefinition {
        name: "email_pattern".to_string(),
        range: Some("string".to_string()),
        pattern: Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string()),
        ..Default::default()
    };
    schema.slots.insert("email_pattern".to_string(), email_slot);

    let engine = ValidationEngine::new(&schema).unwrap();

    let test_data = json!({
        "simple_pattern": "123-456-7890",
        "complex_pattern": "test@example.com",
        "email_pattern": "user@domain.com"
    });

    let mut group = c.benchmark_group("pattern_validation");

    group.bench_function("all_patterns", |b| {
        b.to_async(&runtime).iter(|| async {
            let result = engine
                .validate_as_class(black_box(&test_data), "PatternTest", None)
                .await;
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark memory usage during validation
fn bench_memory_usage(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    let schema = create_complex_schema();
    let engine = ValidationEngine::new(&schema).unwrap();

    let data_sizes = vec![1, 10, 100];

    let mut group = c.benchmark_group("memory_usage");

    for size in data_sizes {
        let large_string = "x".repeat(size * 1024); // Size in KB
        let test_data = json!({
            "id": "US-123456",
            "created_at": "2024-01-01T00:00:00Z",
            "name": large_string,
            "email": "john.smith@company.com",
            "phone": "+1-555-1234",
            "employee_id": "EMP-1234",
            "department": "Engineering",
            "salary": 75000.0
        });

        group.throughput(Throughput::Bytes(u64::try_from(size).unwrap_or(0) * 1024));

        group.bench_with_input(
            BenchmarkId::new("large_data", format!("{}KB", size)),
            &size,
            |b, &_size| {
                b.to_async(&runtime).iter(|| async {
                    let result = engine
                        .validate_as_class(black_box(&test_data), "Employee", None)
                        .await;
                    black_box(result)
                })
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_single_validation,
    bench_batch_validation,
    bench_collection_validation,
    bench_cached_validation,
    bench_validation_options,
    bench_pattern_validation,
    bench_memory_usage
);

criterion_main!(benches);
