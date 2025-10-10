// Copyright (C) 2025 Simon C. Kemper
// Licensed under Creative Commons BY-NC 4.0
//
// Simple, fast-compiling benchmark for Excel introspection performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use linkml_service::inference::{DataIntrospector, ExcelIntrospector};
use logger_core::LoggerConfig;
use rust_xlsxwriter::Workbook;
use std::time::Duration;
use tempfile::NamedTempFile;

fn create_test_services() -> (
    std::sync::Arc<dyn logger_core::LoggerService<Error = logger_core::LoggerError>>,
    std::sync::Arc<dyn timestamp_core::TimestampService<Error = timestamp_core::TimestampError>>,
) {
    let timestamp = timestamp_service::wiring::wire_timestamp().into_arc();
    let logger = logger_service::wiring::wire_logger(timestamp.clone(), LoggerConfig::default())
        .expect("Failed to wire logger")
        .into_arc();
    (logger, timestamp)
}

fn create_test_excel_file(rows: usize) -> NamedTempFile {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // Headers
    worksheet.write_string(0, 0, "id").unwrap();
    worksheet.write_string(0, 1, "name").unwrap();
    worksheet.write_string(0, 2, "age").unwrap();
    worksheet.write_string(0, 3, "email").unwrap();
    worksheet.write_string(0, 4, "active").unwrap();

    // Data rows
    for row in 1..=rows {
        worksheet.write_number(row as u32, 0, row as f64).unwrap();
        worksheet
            .write_string(row as u32, 1, &format!("Person {}", row))
            .unwrap();
        worksheet
            .write_number(row as u32, 2, (25 + (row % 50)) as f64)
            .unwrap();
        worksheet
            .write_string(row as u32, 3, &format!("person{}@example.com", row))
            .unwrap();
        worksheet
            .write_boolean(row as u32, 4, row % 2 == 0)
            .unwrap();
    }

    let temp_file = NamedTempFile::new().unwrap();
    workbook.save(temp_file.path()).unwrap();
    temp_file
}

fn bench_excel_small(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Create test file ONCE (not in the benchmark loop!)
    let temp_file = create_test_excel_file(10);
    let path = temp_file.path().to_path_buf();
    
    c.bench_function("excel_introspect_10_rows", |b| {
        b.to_async(&runtime).iter(|| async {
            let (logger, timestamp) = create_test_services();
            let introspector = ExcelIntrospector::new(logger, timestamp);
            let _ = black_box(introspector.analyze_file(&path).await.unwrap());
        });
    });
}

fn bench_excel_medium(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Create test file ONCE
    let temp_file = create_test_excel_file(100);
    let path = temp_file.path().to_path_buf();
    
    c.bench_function("excel_introspect_100_rows", |b| {
        b.to_async(&runtime).iter(|| async {
            let (logger, timestamp) = create_test_services();
            let introspector = ExcelIntrospector::new(logger, timestamp);
            let _ = black_box(introspector.analyze_file(&path).await.unwrap());
        });
    });
}

fn bench_excel_large(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Create test file ONCE
    let temp_file = create_test_excel_file(1000);
    let path = temp_file.path().to_path_buf();
    
    c.bench_function("excel_introspect_1000_rows", |b| {
        b.to_async(&runtime).iter(|| async {
            let (logger, timestamp) = create_test_services();
            let introspector = ExcelIntrospector::new(logger, timestamp);
            let _ = black_box(introspector.analyze_file(&path).await.unwrap());
        });
    });
}

fn bench_excel_schema_generation(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    
    // Create test file and analyze it ONCE
    let temp_file = create_test_excel_file(100);
    let path = temp_file.path().to_path_buf();
    
    let (logger, timestamp) = create_test_services();
    let introspector = ExcelIntrospector::new(logger, timestamp);
    let stats = runtime
        .block_on(async { introspector.analyze_file(&path).await.unwrap() });
    
    c.bench_function("excel_schema_generation_100_rows", |b| {
        b.to_async(&runtime).iter(|| async {
            let (logger, timestamp) = create_test_services();
            let introspector = ExcelIntrospector::new(logger, timestamp);
            let _ = black_box(
                introspector
                    .generate_schema(&stats, "test_schema")
                    .await
                    .unwrap(),
            );
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)  // Reduce sample size for faster benchmarking
        .measurement_time(Duration::from_secs(10));  // Reduce measurement time
    targets = bench_excel_small, bench_excel_medium, bench_excel_large, bench_excel_schema_generation
}

criterion_main!(benches);

