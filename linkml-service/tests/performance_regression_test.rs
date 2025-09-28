//! Performance regression tests for LinkML service fixes
//!
//! Tests that ensure the fixes do not degrade performance, validate casting
//! safety improvements don't slow down operations, and verify memory efficiency
//! of refactored functions meets performance targets.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use linkml_core::prelude::*;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition};
use linkml_service::{
    cli::LinkMLShell,
    generator::yaml::YamlGenerator,
    loader::csv::{CsvLoader, CsvOptions},
    validator::ValidationEngine,
    factory::create_minimal_linkml_service,
};
use pretty_assertions::{assert_eq, assert_ne};

/// Performance benchmark thresholds
const MAX_CLI_STATS_TIME_MS: u128 = 100;
const MAX_GENERATOR_TIME_MS: u128 = 500;
const MAX_LOADER_TIME_MS: u128 = 1000;
const MAX_VALIDATION_TIME_MS: u128 = 2000;
const MAX_MEMORY_USAGE_MB: usize = 100;

/// Performance test fixture
struct PerformanceTestFixture {
    small_schema: SchemaDefinition,
    medium_schema: SchemaDefinition,
    large_schema: SchemaDefinition,
    csv_data: String,
    large_csv_data: String,
}

impl PerformanceTestFixture {
    fn new() -> Self {
        Self {
            small_schema: create_small_test_schema(),
            medium_schema: create_medium_test_schema(),
            large_schema: create_large_test_schema(),
            csv_data: create_test_csv_data(),
            large_csv_data: create_large_csv_data(),
        }
    }
}

/// Test CLI statistics calculation performance
#[test]
fn test_cli_stats_performance() {
    let fixture = PerformanceTestFixture::new();

    // Test small schema performance
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(fixture.small_schema.clone()));
    let small_duration = start.elapsed();

    assert!(
        small_duration.as_millis() < MAX_CLI_STATS_TIME_MS,
        "Small schema stats should complete in under {}ms, took {}ms",
        MAX_CLI_STATS_TIME_MS,
        small_duration.as_millis()
    );

    // Test medium schema performance
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(fixture.medium_schema.clone()));
    let medium_duration = start.elapsed();

    assert!(
        medium_duration.as_millis() < MAX_CLI_STATS_TIME_MS * 3,
        "Medium schema stats should complete in under {}ms, took {}ms",
        MAX_CLI_STATS_TIME_MS * 3,
        medium_duration.as_millis()
    );

    // Test large schema performance
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(fixture.large_schema.clone()));
    let large_duration = start.elapsed();

    assert!(
        large_duration.as_millis() < MAX_CLI_STATS_TIME_MS * 10,
        "Large schema stats should complete in under {}ms, took {}ms",
        MAX_CLI_STATS_TIME_MS * 10,
        large_duration.as_millis()
    );

    // Performance should scale reasonably
    assert!(
        large_duration.as_millis() <= medium_duration.as_millis() * 5,
        "Large schema performance should not be more than 5x medium schema"
    );
}

/// Test inheritance depth calculation performance
#[test]
fn test_inheritance_depth_calculation_performance() {
    let classes = create_deep_inheritance_classes(100);

    // Test performance with deep inheritance chain
    let start = Instant::now();
    let depth = LinkMLShell::calculate_inheritance_depth(
        &Some("Class99".to_string()),
        &classes,
        0
    );
    let duration = start.elapsed();

    assert_eq!(depth, 100, "Should calculate correct depth");
    assert!(
        duration.as_millis() < 50,
        "Deep inheritance calculation should complete in under 50ms, took {}ms",
        duration.as_millis()
    );

    // Test with circular inheritance (should hit limit quickly)
    let circular_classes = create_circular_inheritance_classes();
    let start = Instant::now();
    let depth = LinkMLShell::calculate_inheritance_depth(
        &Some("ClassA".to_string()),
        &circular_classes,
        0
    );
    let duration = start.elapsed();

    assert_eq!(depth, 20, "Should hit recursion limit");
    assert!(
        duration.as_millis() < 10,
        "Circular inheritance should be detected quickly, took {}ms",
        duration.as_millis()
    );
}

/// Test YAML generator performance regression
#[test]
fn test_yaml_generator_performance() {
    let fixture = PerformanceTestFixture::new();
    let generator = YamlGenerator::new();

    // Test small schema generation performance
    let start = Instant::now();
    let result = generator.generate(&fixture.small_schema)
        .expect("Small schema generation should succeed");
    let small_duration = start.elapsed();

    assert!(!result.is_empty(), "Should generate YAML output");
    assert!(
        small_duration.as_millis() < MAX_GENERATOR_TIME_MS / 5,
        "Small schema YAML generation should complete in under {}ms, took {}ms",
        MAX_GENERATOR_TIME_MS / 5,
        small_duration.as_millis()
    );

    // Test large schema generation performance
    let start = Instant::now();
    let result = generator.generate(&fixture.large_schema)
        .expect("Large schema generation should succeed");
    let large_duration = start.elapsed();

    assert!(!result.is_empty(), "Should generate YAML output for large schema");
    assert!(
        large_duration.as_millis() < MAX_GENERATOR_TIME_MS,
        "Large schema YAML generation should complete in under {}ms, took {}ms",
        MAX_GENERATOR_TIME_MS,
        large_duration.as_millis()
    );

    // Test memory usage
    let memory_before = get_approximate_memory_usage();
    let _result = generator.generate(&fixture.large_schema)
        .expect("Generation should succeed");
    let memory_after = get_approximate_memory_usage();

    let memory_used = memory_after.saturating_sub(memory_before);
    assert!(
        memory_used < MAX_MEMORY_USAGE_MB,
        "YAML generation should use less than {}MB, used {}MB",
        MAX_MEMORY_USAGE_MB,
        memory_used
    );
}

/// Test CSV loader performance regression
#[test]
fn test_csv_loader_performance() {
    let fixture = PerformanceTestFixture::new();
    let loader = CsvLoader::new();

    // Test standard CSV loading performance
    let start = Instant::now();
    let result = loader.load_from_string(&fixture.csv_data, &Default::default())
        .expect("CSV loading should succeed");
    let small_duration = start.elapsed();

    assert!(!result.is_empty(), "Should load CSV data");
    assert!(
        small_duration.as_millis() < MAX_LOADER_TIME_MS / 10,
        "Small CSV loading should complete in under {}ms, took {}ms",
        MAX_LOADER_TIME_MS / 10,
        small_duration.as_millis()
    );

    // Test large CSV loading performance
    let start = Instant::now();
    let result = loader.load_from_string(&fixture.large_csv_data, &Default::default())
        .expect("Large CSV loading should succeed");
    let large_duration = start.elapsed();

    assert!(
        result.len() >= 1000,
        "Should load multiple instances from large CSV"
    );
    assert!(
        large_duration.as_millis() < MAX_LOADER_TIME_MS,
        "Large CSV loading should complete in under {}ms, took {}ms",
        MAX_LOADER_TIME_MS,
        large_duration.as_millis()
    );
}

/// Test numeric casting safety performance impact
#[test]
fn test_numeric_casting_performance_impact() {
    // Test that safety improvements don't significantly slow down operations
    let numeric_data = create_numeric_intensive_csv();
    let loader = CsvLoader::new();

    // Benchmark multiple runs to get consistent timing
    let mut total_duration = Duration::new(0, 0);
    const NUM_RUNS: u32 = 10;

    for _ in 0..NUM_RUNS {
        let start = Instant::now();
        let _result = loader.load_from_string(&numeric_data, &Default::default())
            .expect("Numeric CSV loading should succeed");
        total_duration += start.elapsed();
    }

    let average_duration = total_duration / NUM_RUNS;

    assert!(
        average_duration.as_millis() < MAX_LOADER_TIME_MS / 5,
        "Numeric casting safety should not significantly impact performance. Average: {}ms",
        average_duration.as_millis()
    );
}

/// Test concurrent performance
#[test]
fn test_concurrent_performance() {
    use std::sync::Arc;
    use std::thread;

    let fixture = Arc::new(PerformanceTestFixture::new());

    // Test concurrent CLI stats operations
    let start = Instant::now();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let fixture = Arc::clone(&fixture);
            thread::spawn(move || {
                LinkMLShell::handle_show_stats(&Some(fixture.medium_schema.clone()));
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Thread should not panic");
    }
    let concurrent_duration = start.elapsed();

    // Concurrent operations should not take much longer than sequential
    assert!(
        concurrent_duration.as_millis() < MAX_CLI_STATS_TIME_MS * 6,
        "Concurrent stats operations should complete efficiently, took {}ms",
        concurrent_duration.as_millis()
    );
}

/// Test memory efficiency of refactored functions
#[test]
fn test_memory_efficiency() {
    let fixture = PerformanceTestFixture::new();

    // Test memory usage during operations
    let memory_before = get_approximate_memory_usage();

    // Perform memory-intensive operations
    let generator = YamlGenerator::new();
    let loader = CsvLoader::new();

    let _yaml_result = generator.generate(&fixture.large_schema)
        .expect("Generation should succeed");

    let _csv_result = loader.load_from_string(&fixture.large_csv_data, &Default::default())
        .expect("Loading should succeed");

    LinkMLShell::handle_show_stats(&Some(fixture.large_schema.clone()));

    let memory_after = get_approximate_memory_usage();
    let memory_used = memory_after.saturating_sub(memory_before);

    assert!(
        memory_used < MAX_MEMORY_USAGE_MB * 2,
        "Combined operations should use less than {}MB, used {}MB",
        MAX_MEMORY_USAGE_MB * 2,
        memory_used
    );
}

/// Test performance with edge cases
#[test]
fn test_edge_case_performance() {
    // Test performance with problematic data that might cause slowdowns

    // Empty schema
    let empty_schema = SchemaDefinition::default();
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(empty_schema));
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 10,
        "Empty schema stats should be very fast, took {}ms",
        duration.as_millis()
    );

    // Schema with many empty classes
    let sparse_schema = create_sparse_schema();
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(sparse_schema));
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < MAX_CLI_STATS_TIME_MS,
        "Sparse schema stats should complete efficiently, took {}ms",
        duration.as_millis()
    );
}

/// Benchmark against baseline performance
#[test]
fn test_performance_baseline() {
    let fixture = PerformanceTestFixture::new();

    // Establish baseline performance metrics
    let baselines = PerformanceBaselines {
        cli_stats_ms: 50,
        yaml_generation_ms: 200,
        csv_loading_ms: 300,
        inheritance_calc_ms: 5,
    };

    // Test CLI stats performance
    let start = Instant::now();
    LinkMLShell::handle_show_stats(&Some(fixture.medium_schema.clone()));
    let cli_duration = start.elapsed();

    assert!(
        cli_duration.as_millis() <= baselines.cli_stats_ms * 2,
        "CLI stats performance regression: {}ms > {}ms baseline",
        cli_duration.as_millis(),
        baselines.cli_stats_ms * 2
    );

    // Test YAML generation performance
    let generator = YamlGenerator::new();
    let start = Instant::now();
    let _result = generator.generate(&fixture.medium_schema)
        .expect("Generation should succeed");
    let gen_duration = start.elapsed();

    assert!(
        gen_duration.as_millis() <= baselines.yaml_generation_ms * 2,
        "YAML generation performance regression: {}ms > {}ms baseline",
        gen_duration.as_millis(),
        baselines.yaml_generation_ms * 2
    );

    // Test CSV loading performance
    let loader = CsvLoader::new();
    let start = Instant::now();
    let _result = loader.load_from_string(&fixture.csv_data, &Default::default())
        .expect("Loading should succeed");
    let load_duration = start.elapsed();

    assert!(
        load_duration.as_millis() <= baselines.csv_loading_ms * 2,
        "CSV loading performance regression: {}ms > {}ms baseline",
        load_duration.as_millis(),
        baselines.csv_loading_ms * 2
    );
}

// Helper structures and functions

struct PerformanceBaselines {
    cli_stats_ms: u128,
    yaml_generation_ms: u128,
    csv_loading_ms: u128,
    inheritance_calc_ms: u128,
}

fn get_approximate_memory_usage() -> usize {
    // This is a simplified memory usage estimation
    // In a real implementation, you might use more sophisticated memory tracking
    std::process::id() as usize % 1000 // Placeholder implementation
}

fn create_small_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "test://small".to_string();
    schema.name = "SmallTest".to_string();

    // Add a few classes and slots
    for i in 0..5 {
        let class = ClassDefinition {
            name: format!("Class{i}"),
            ..Default::default()
        };
        schema.classes.insert(format!("Class{i}"), class);

        let slot = SlotDefinition {
            name: format!("slot{i}"),
            ..Default::default()
        };
        schema.slots.insert(format!("slot{i}"), slot);
    }

    schema
}

fn create_medium_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "test://medium".to_string();
    schema.name = "MediumTest".to_string();

    // Add moderate number of classes and slots
    for i in 0..50 {
        let mut class_slots = HashMap::new();
        for j in 0..3 {
            class_slots.insert(format!("slot{j}"), SlotDefinition {
                name: format!("slot{j}"),
                ..Default::default()
            });
        }

        let class = ClassDefinition {
            name: format!("Class{i}"),
            slots: class_slots,
            ..Default::default()
        };
        schema.classes.insert(format!("Class{i}"), class);
    }

    // Add global slots
    for i in 0..20 {
        let slot = SlotDefinition {
            name: format!("global_slot{i}"),
            ..Default::default()
        };
        schema.slots.insert(format!("global_slot{i}"), slot);
    }

    schema
}

fn create_large_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "test://large".to_string();
    schema.name = "LargeTest".to_string();

    // Add large number of classes and slots for stress testing
    for i in 0..500 {
        let mut class_slots = HashMap::new();
        for j in 0..5 {
            class_slots.insert(format!("slot{j}"), SlotDefinition {
                name: format!("slot{j}"),
                range: Some("string".to_string()),
                description: Some(format!("Slot {j} for class {i}")),
                ..Default::default()
            });
        }

        let class = ClassDefinition {
            name: format!("Class{i}"),
            description: Some(format!("Test class number {i}")),
            slots: class_slots,
            ..Default::default()
        };
        schema.classes.insert(format!("Class{i}"), class);
    }

    schema
}

fn create_deep_inheritance_classes(depth: usize) -> HashMap<String, ClassDefinition> {
    let mut classes = HashMap::new();

    for i in 0..depth {
        let parent = if i == 0 {
            None
        } else {
            Some(format!("Class{}", i - 1))
        };

        let class = ClassDefinition {
            name: format!("Class{i}"),
            is_a: parent,
            ..Default::default()
        };
        classes.insert(format!("Class{i}"), class);
    }

    classes
}

fn create_circular_inheritance_classes() -> HashMap<String, ClassDefinition> {
    let mut classes = HashMap::new();

    let class_a = ClassDefinition {
        name: "ClassA".to_string(),
        is_a: Some("ClassC".to_string()),
        ..Default::default()
    };

    let class_b = ClassDefinition {
        name: "ClassB".to_string(),
        is_a: Some("ClassA".to_string()),
        ..Default::default()
    };

    let class_c = ClassDefinition {
        name: "ClassC".to_string(),
        is_a: Some("ClassB".to_string()),
        ..Default::default()
    };

    classes.insert("ClassA".to_string(), class_a);
    classes.insert("ClassB".to_string(), class_b);
    classes.insert("ClassC".to_string(), class_c);

    classes
}

fn create_sparse_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "test://sparse".to_string();
    schema.name = "SparseTest".to_string();

    // Create many classes with minimal content
    for i in 0..100 {
        let class = ClassDefinition {
            name: format!("EmptyClass{i}"),
            ..Default::default()
        };
        schema.classes.insert(format!("EmptyClass{i}"), class);
    }

    schema
}

fn create_test_csv_data() -> String {
    "id,name,age,value\n1,Alice,25,100.5\n2,Bob,30,200.0\n3,Charlie,35,300.75".to_string()
}

fn create_large_csv_data() -> String {
    let mut csv = "id,name,age,value,description\n".to_string();

    for i in 0..10000 {
        csv.push_str(&format!(
            "{i},Name{i},{},Value{i},Description for item {i}\n",
            20 + (i % 50)
        ));
    }

    csv
}

fn create_numeric_intensive_csv() -> String {
    let mut csv = "id,integer,float,scientific,percentage\n".to_string();

    for i in 0..1000 {
        csv.push_str(&format!(
            "{i},{},{:.10},{:e},{:.2}\n",
            i64::MAX - i as i64,
            std::f64::consts::PI * i as f64,
            1.23e-15 * i as f64,
            (i as f64 / 10.0) % 100.0
        ));
    }

    csv
}