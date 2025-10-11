//! Performance benchmarks for Excel data loader
//!
//! Tests loading performance for various workbook sizes and complexity.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use linkml_core::prelude::*;
use linkml_service::loader::{DataLoader, ExcelLoader, LoadOptions};
use logger_service::wiring::wire_logger;
use rust_xlsxwriter::Workbook;
use std::sync::Arc;
use tempfile::TempDir;
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

/// Helper to create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("benchmark_schema");
    schema.id = "benchmark_schema".to_string();

    let mut data_class = ClassDefinition::new("Data");
    data_class.attributes.insert(
        "id".to_string(),
        SlotDefinition {
            name: "id".to_string(),
            range: Some("integer".to_string()),
            required: Some(true),
            identifier: Some(true),
            ..Default::default()
        },
    );
    data_class.attributes.insert(
        "value1".to_string(),
        SlotDefinition {
            name: "value1".to_string(),
            range: Some("string".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );
    data_class.attributes.insert(
        "value2".to_string(),
        SlotDefinition {
            name: "value2".to_string(),
            range: Some("float".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );
    data_class.attributes.insert(
        "value3".to_string(),
        SlotDefinition {
            name: "value3".to_string(),
            range: Some("integer".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );
    data_class.attributes.insert(
        "active".to_string(),
        SlotDefinition {
            name: "active".to_string(),
            range: Some("boolean".to_string()),
            required: Some(false),
            ..Default::default()
        },
    );

    schema.classes.insert("Data".to_string(), data_class);
    schema
}

/// Create an Excel file with specified number of rows
fn create_test_excel(rows: usize) -> std::result::Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let excel_path = temp_dir.path().join("benchmark.xlsx");

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Write headers
    worksheet.write_string(0, 0, "id")?;
    worksheet.write_string(0, 1, "value1")?;
    worksheet.write_string(0, 2, "value2")?;
    worksheet.write_string(0, 3, "value3")?;
    worksheet.write_string(0, 4, "active")?;

    // Write data rows
    for i in 0..rows {
        let row = (i + 1) as u32;
        worksheet.write_number(row, 0, i as f64)?;
        worksheet.write_string(row, 1, &format!("Value {i}"))?;
        worksheet.write_number(row, 2, (i as f64) * 1.5)?;
        worksheet.write_number(row, 3, (i * 2) as f64)?;
        worksheet.write_boolean(row, 4, i % 2 == 0)?;
    }

    workbook.save(&excel_path)?;
    Ok(temp_dir)
}

/// Create multi-sheet Excel file
fn create_multi_sheet_excel(
    sheets: usize,
    rows_per_sheet: usize,
) -> std::result::Result<TempDir, Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let excel_path = temp_dir.path().join("multi_benchmark.xlsx");

    let mut workbook = Workbook::new();

    for sheet_idx in 0..sheets {
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&format!("Sheet{sheet_idx}"))?;

        // Write headers
        worksheet.write_string(0, 0, "id")?;
        worksheet.write_string(0, 1, "data")?;
        worksheet.write_string(0, 2, "value")?;

        // Write data
        for row_idx in 0..rows_per_sheet {
            let row = (row_idx + 1) as u32;
            worksheet.write_number(row, 0, (sheet_idx * rows_per_sheet + row_idx) as f64)?;
            worksheet.write_string(row, 1, &format!("Sheet{sheet_idx}_Row{row_idx}"))?;
            worksheet.write_number(row, 2, row_idx as f64)?;
        }
    }

    workbook.save(&excel_path)?;
    Ok(temp_dir)
}

/// Benchmark: Load various row counts
fn bench_excel_loader_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("excel_loader_sizes");

    for size in [10, 100, 500, 1000, 5000].iter() {
        let temp_dir = create_test_excel(*size).expect("Failed to create test Excel");
        let excel_path = temp_dir.path().join("benchmark.xlsx");
        let (logger, timestamp) = create_test_services();
        let loader = ExcelLoader::new(logger, timestamp);
        let schema = create_test_schema();
        let options = LoadOptions {
            target_class: Some("Data".to_string()),
            ..Default::default()
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    black_box(
                        loader
                            .load_file(&excel_path, &schema, &options)
                            .await
                            .unwrap(),
                    )
                })
            });
        });
    }

    group.finish();
}

/// Benchmark: Critical 1000 row target
fn bench_excel_loader_1000_rows(c: &mut Criterion) {
    let temp_dir = create_test_excel(1000).expect("Failed to create test Excel");
    let excel_path = temp_dir.path().join("benchmark.xlsx");
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);
    let schema = create_test_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Data".to_string());

    c.bench_function("excel_loader_1000_rows", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                black_box(
                    loader
                        .load_file(&excel_path, &schema, &options)
                        .await
                        .unwrap(),
                )
            })
        });
    });
}

/// Benchmark: Multi-sheet loading
fn bench_excel_loader_multi_sheet(c: &mut Criterion) {
    let mut group = c.benchmark_group("excel_loader_multi_sheet");

    for sheets in [1, 5, 10, 20].iter() {
        let temp_dir =
            create_multi_sheet_excel(*sheets, 100).expect("Failed to create multi-sheet Excel");
        let excel_path = temp_dir.path().join("multi_benchmark.xlsx");
        let (logger, timestamp) = create_test_services();

        use linkml_service::loader::ExcelOptions;
        let mut excel_options = ExcelOptions::default();
        excel_options.target_sheet = Some("*".to_string());
        let loader = ExcelLoader::with_options(logger, timestamp, excel_options);

        let mut schema = SchemaDefinition::new("multi_schema");
        for i in 0..*sheets {
            let mut class_def = ClassDefinition::new(&format!("Sheet{i}"));
            class_def.attributes.insert(
                "id".to_string(),
                SlotDefinition {
                    name: "id".to_string(),
                    range: Some("integer".to_string()),
                    required: Some(true),
                    identifier: Some(true),
                    ..Default::default()
                },
            );
            class_def.attributes.insert(
                "data".to_string(),
                SlotDefinition {
                    name: "data".to_string(),
                    range: Some("string".to_string()),
                    required: Some(false),
                    ..Default::default()
                },
            );
            class_def.attributes.insert(
                "value".to_string(),
                SlotDefinition {
                    name: "value".to_string(),
                    range: Some("integer".to_string()),
                    required: Some(false),
                    ..Default::default()
                },
            );
            schema.classes.insert(format!("Sheet{i}"), class_def);
        }

        let options = LoadOptions::default();

        group.bench_with_input(BenchmarkId::from_parameter(sheets), sheets, |b, _| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    black_box(
                        loader
                            .load_file(&excel_path, &schema, &options)
                            .await
                            .unwrap(),
                    )
                })
            });
        });
    }

    group.finish();
}

/// Benchmark: Type conversion overhead
fn bench_excel_loader_type_conversion(c: &mut Criterion) {
    let temp_dir = create_test_excel(1000).expect("Failed to create test Excel");
    let excel_path = temp_dir.path().join("benchmark.xlsx");
    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);
    let schema = create_test_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Data".to_string());

    c.bench_function("excel_loader_type_conversion", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                black_box(
                    loader
                        .load_file(&excel_path, &schema, &options)
                        .await
                        .unwrap(),
                )
            })
        });
    });
}

/// Benchmark: Load from bytes
fn bench_excel_loader_bytes(c: &mut Criterion) {
    let temp_dir = create_test_excel(1000).expect("Failed to create test Excel");
    let excel_path = temp_dir.path().join("benchmark.xlsx");
    let bytes = std::fs::read(&excel_path).expect("Failed to read Excel file");

    let (logger, timestamp) = create_test_services();
    let loader = ExcelLoader::new(logger, timestamp);
    let schema = create_test_schema();
    let mut options = LoadOptions::default();
    options.target_class = Some("Data".to_string());

    c.bench_function("excel_loader_bytes", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                black_box(loader.load_bytes(&bytes, &schema, &options).await.unwrap())
            })
        });
    });
}

criterion_group!(
    benches,
    bench_excel_loader_sizes,
    bench_excel_loader_1000_rows,
    bench_excel_loader_multi_sheet,
    bench_excel_loader_type_conversion,
    bench_excel_loader_bytes,
);
criterion_main!(benches);
