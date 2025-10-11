//! Benchmarks demonstrating performance optimizations

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use linkml_service::performance::{Profiler, intern, str_eq_fast};
use std::sync::Arc;

/// Benchmark the impact of string interning on repeated comparisons.
fn bench_string_interning(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_interning");

    // Create test strings
    let strings: Vec<String> = (0..1000)
        .map(|i| match i % 10 {
            0 => "string".to_string(),
            1 => "integer".to_string(),
            2 => "boolean".to_string(),
            3 => "float".to_string(),
            4 => "date".to_string(),
            5 => "required".to_string(),
            6 => "multivalued".to_string(),
            7 => "identifier".to_string(),
            8 => "pattern".to_string(),
            _ => "range".to_string(),
        })
        .collect();

    // Benchmark regular string comparison
    group.bench_function("regular_comparison", |b| {
        b.iter(|| {
            let mut matches = 0;
            for s in &strings {
                if black_box(s.as_str()) == "string" {
                    matches += 1;
                }
            }
            matches
        })
    });

    // Pre-intern strings
    let interned: Vec<Arc<str>> = strings.iter().map(|s| intern(s)).collect();
    let target = intern("string");

    // Benchmark interned string comparison
    group.bench_function("interned_comparison", |b| {
        b.iter(|| {
            let mut matches = 0;
            for s in &interned {
                if str_eq_fast(black_box(s), &target) {
                    matches += 1;
                }
            }
            matches
        })
    });

    group.finish();
}

/// Benchmark the small-vector optimization used by the profiler.
fn bench_small_vec_optimization(c: &mut Criterion) {
    use linkml_service::performance::{IssueVec, issue_vec};

    let mut group = c.benchmark_group("small_vec");

    // Benchmark regular Vec
    group.bench_function("regular_vec_small", |b| {
        b.iter(|| {
            let v = vec!["error1", "error2"];
            black_box(v)
        })
    });

    // Benchmark SmallVec
    group.bench_function("small_vec_small", |b| {
        b.iter(|| {
            let mut v: IssueVec<&str> = issue_vec();
            v.push("error1");
            v.push("error2");
            black_box(v)
        })
    });

    // Benchmark with more elements (causes spill)
    group.bench_function("regular_vec_large", |b| {
        b.iter(|| {
            let mut v = Vec::new();
            for i in 0..10 {
                v.push(i);
            }
            black_box(v)
        })
    });

    group.bench_function("small_vec_large", |b| {
        b.iter(|| {
            let mut v: IssueVec<i32> = issue_vec();
            for i in 0..10 {
                v.push(i);
            }
            black_box(v)
        })
    });

    group.finish();
}

/// Benchmark memory estimation routines for validation artifacts.
fn bench_memory_estimation(c: &mut Criterion) {
    use linkml_service::performance::MemorySize;
    use serde_json::json;

    let mut group = c.benchmark_group("memory_size");

    // Small JSON
    let small_json = json!({
        "name": "test",
        "age": 30
    });

    group.bench_function("small_json", |b| {
        b.iter(|| black_box(small_json.heap_size()))
    });

    // Medium JSON
    let medium_json = json!({
        "id": "12345",
        "name": "Test User",
        "email": "test@example.com",
        "roles": ["admin", "user", "developer"],
        "metadata": {
            "created": "2025-01-31",
            "updated": "2025-01-31",
            "version": 1
        }
    });

    group.bench_function("medium_json", |b| {
        b.iter(|| black_box(medium_json.heap_size()))
    });

    // Large JSON (nested structure)
    let large_json = json!({
        "schema": {
            "name": "TestSchema",
            "classes": {
                "Person": {
                    "slots": ["name", "age", "email", "address"],
                    "description": "A person entity"
                },
                "Address": {
                    "slots": ["street", "city", "state", "zip"],
                    "description": "An address entity"
                }
            },
            "slots": {
                "name": { "range": "string", "required": true },
                "age": { "range": "integer", "minimum": 0, "maximum": 150 },
                "email": { "range": "string", "pattern": r"^\S+@\S+$" },
                "street": { "range": "string" },
                "city": { "range": "string" },
                "state": { "range": "string", "pattern": r"^[A-Z]{2}$" },
                "zip": { "range": "string", "pattern": r"^\d{5}$" }
            }
        }
    });

    group.bench_function("large_json", |b| {
        b.iter(|| black_box(large_json.heap_size()))
    });

    group.finish();
}

/// Benchmark the overhead of the profiler instrumentation.
fn bench_profiling_overhead(c: &mut Criterion) {
    let profiler = Profiler::default();

    let mut group = c.benchmark_group("profiling_overhead");

    // Function to profile
    fn compute_sum(n: usize) -> usize {
        (0..n).sum()
    }

    // Benchmark without profiling
    group.bench_function("no_profiling", |b| {
        profiler.set_enabled(false);
        b.iter(|| black_box(compute_sum(1000)))
    });

    // Benchmark with profiling
    group.bench_function("with_profiling", |b| {
        profiler.set_enabled(true);
        b.iter(|| profiler.time("compute_sum", || black_box(compute_sum(1000))))
    });

    profiler.set_enabled(false);
    group.finish();
}

criterion_group!(
    benches,
    bench_string_interning,
    bench_small_vec_optimization,
    bench_memory_estimation,
    bench_profiling_overhead
);

criterion_main!(benches);
