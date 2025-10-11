//! Performance benchmarks for round-trip operations
//!
//! Tests round-trip performance for Schema → Excel → Schema and Data → Excel → Data cycles.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use linkml_core::prelude::*;
use linkml_service::generator::excel::ExcelGenerator;
use linkml_service::inference::introspectors::excel::ExcelIntrospector;
use linkml_service::inference::traits::DataIntrospector;
use logger_service::wiring::wire_logger;
use std::sync::Arc;
use timestamp_service::wiring::wire_timestamp;

/// Helper to create test services
fn create_test_services() -> (
    Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = wire_timestamp().into_arc();
    let logger = wire_logger(timestamp.clone(), logger_core::LoggerConfig::default())
        .expect("Failed to wire logger")
        .into_arc();
    (logger, timestamp)
}

/// Create a test schema with specified number of classes
fn create_test_schema(num_classes: usize) -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("benchmark_schema");
    schema.id = "benchmark_schema".to_string();

    for i in 0..num_classes {
        let class_name = format!("Class{i}");
        let mut class_def = ClassDefinition::new(&class_name);
        class_def.name = class_name.clone();

        // Add 10 attributes per class
        for j in 0..10 {
            let attr_name = format!("attr{j}");
            class_def.attributes.insert(
                attr_name.clone(),
                SlotDefinition {
                    name: attr_name,
                    range: Some(if j % 3 == 0 {
                        "integer".to_string()
                    } else if j % 3 == 1 {
                        "string".to_string()
                    } else {
                        "float".to_string()
                    }),
                    required: Some(j < 5),    // First 5 are required
                    identifier: Some(j == 0), // First attr is identifier
                    ..Default::default()
                },
            );
        }

        schema.classes.insert(class_name.clone(), class_def);
    }

    schema
}

/// Benchmark: Schema round-trip with varying complexity
fn bench_schema_roundtrip_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("schema_roundtrip_sizes");

    for num_classes in [1, 5, 10, 25, 50].iter() {
        let (logger, timestamp) = create_test_services();
        let schema = create_test_schema(*num_classes);
        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let excel_path = temp_dir.path().join("benchmark.xlsx");

        group.bench_with_input(
            BenchmarkId::from_parameter(num_classes),
            num_classes,
            |b, _| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                        // Schema → Excel
                        let generator = ExcelGenerator::new();
                        generator
                            .generate_file(&schema, excel_path.to_str().unwrap())
                            .expect("Failed to generate Excel");

                        // Excel → Schema
                        let introspector =
                            ExcelIntrospector::new(logger.clone(), timestamp.clone());
                        let result = introspector
                            .analyze_file(&excel_path)
                            .await
                            .expect("Failed to analyze");
                        black_box(result)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark: Critical 10-class schema round-trip (<200ms target)
fn bench_schema_roundtrip_typical(c: &mut Criterion) {
    let (logger, timestamp) = create_test_services();
    let schema = create_test_schema(10); // Typical workbook: 10 classes
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let excel_path = temp_dir.path().join("typical.xlsx");

    c.bench_function("schema_roundtrip_typical", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Schema → Excel
                let generator = ExcelGenerator::new();
                generator
                    .generate_file(&schema, excel_path.to_str().unwrap())
                    .expect("Failed to generate Excel");

                // Excel → Schema
                let introspector = ExcelIntrospector::new(logger.clone(), timestamp.clone());
                black_box(
                    introspector
                        .analyze_file(&excel_path)
                        .await
                        .expect("Failed to analyze"),
                )
            })
        });
    });
}

/// Benchmark: Schema generation only
fn bench_schema_generation(c: &mut Criterion) {
    let schema = create_test_schema(10);
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let excel_path = temp_dir.path().join("gen_only.xlsx");

    c.bench_function("schema_generation", |b| {
        b.iter(|| {
            let generator = ExcelGenerator::new();
            black_box(
                generator
                    .generate_file(&schema, excel_path.to_str().unwrap())
                    .expect("Failed to generate"),
            )
        });
    });
}

/// Benchmark: Schema introspection only
fn bench_schema_introspection(c: &mut Criterion) {
    let (logger, timestamp) = create_test_services();
    let schema = create_test_schema(10);
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let excel_path = temp_dir.path().join("intro_only.xlsx");

    // Generate once
    let generator = ExcelGenerator::new();
    generator
        .generate_file(&schema, excel_path.to_str().unwrap())
        .expect("Failed to generate");

    c.bench_function("schema_introspection", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let introspector = ExcelIntrospector::new(logger.clone(), timestamp.clone());
                black_box(
                    introspector
                        .analyze_file(&excel_path)
                        .await
                        .expect("Failed to analyze"),
                )
            })
        });
    });
}

/// Benchmark: Schema round-trip with inheritance hierarchies
fn bench_schema_roundtrip_inheritance(c: &mut Criterion) {
    let (logger, timestamp) = create_test_services();
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let excel_path = temp_dir.path().join("inheritance.xlsx");

    // Create schema with inheritance
    let mut schema = SchemaDefinition::new("inheritance_schema");
    schema.id = "inheritance_schema".to_string();

    // Base class
    let mut base_class = ClassDefinition::new("BaseClass");
    base_class.name = "BaseClass".to_string();
    base_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );

    // 5 derived classes
    for i in 0..5 {
        let class_name = format!("Derived{i}");
        let mut class_def = ClassDefinition::new(&class_name);
        class_def.name = class_name.clone();
        class_def.is_a = Some("BaseClass".to_string());

        for j in 0..5 {
            let attr_name = format!("attr{j}");
            class_def.attributes.insert(
                attr_name.clone(),
                SlotDefinition {
                    name: attr_name,
                    range: Some("string".to_string()),
                    ..Default::default()
                },
            );
        }

        schema.classes.insert(class_name, class_def);
    }

    schema.classes.insert("BaseClass".to_string(), base_class);

    c.bench_function("schema_roundtrip_inheritance", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                // Schema → Excel
                let generator = ExcelGenerator::new();
                generator
                    .generate_file(&schema, excel_path.to_str().unwrap())
                    .expect("Failed to generate");

                // Excel → Schema
                let introspector = ExcelIntrospector::new(logger.clone(), timestamp.clone());
                black_box(
                    introspector
                        .analyze_file(&excel_path)
                        .await
                        .expect("Failed to analyze"),
                )
            })
        });
    });
}

criterion_group!(
    benches,
    bench_schema_roundtrip_sizes,
    bench_schema_roundtrip_typical,
    bench_schema_generation,
    bench_schema_introspection,
    bench_schema_roundtrip_inheritance,
);
criterion_main!(benches);
