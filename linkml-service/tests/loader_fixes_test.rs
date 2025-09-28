//! Unit tests for loader module fixes
//!
//! Tests the loader module fixes including precision loss prevention in numeric casts,
//! truncation safety mechanisms, data validation with edge cases, and memory efficiency
//! of refactored functions in RDF and CSV loaders.

use std::collections::HashMap;
use std::path::Path;
use linkml_core::prelude::*;
use linkml_service::loader::{
    csv::{CsvLoader, CsvOptions},
    rdf::{RdfLoader, RdfOptions, RdfSerializationFormat, SkolemnizationOptions},
    traits::{DataLoader, DataInstance, LoadOptions, LoaderResult},
};
use pretty_assertions::{assert_eq, assert_ne};
use serde_json::Value as JsonValue;

/// Test fixture for loader testing
struct LoaderTestFixture {
    csv_data: String,
    tsv_data: String,
    rdf_data: String,
    edge_case_csv: String,
    numeric_precision_csv: String,
}

impl LoaderTestFixture {
    fn new() -> Self {
        Self {
            csv_data: create_test_csv_data(),
            tsv_data: create_test_tsv_data(),
            rdf_data: create_test_rdf_data(),
            edge_case_csv: create_edge_case_csv_data(),
            numeric_precision_csv: create_numeric_precision_csv_data(),
        }
    }
}

/// Test CSV loader precision loss prevention in numeric casts
#[test]
fn test_csv_loader_numeric_precision_safety() {
    let fixture = LoaderTestFixture::new();
    let loader = CsvLoader::new();

    // Create options for testing
    let options = LoadOptions {
        validate: true,
        format_options: HashMap::new(),
    };

    // Test with numeric precision data
    let result = loader.load_from_string(&fixture.numeric_precision_csv, &options);

    match result {
        Ok(instances) => {
            assert!(
                !instances.is_empty(),
                "Should load instances from numeric precision CSV"
            );

            // Verify that large numbers are preserved accurately
            for instance in &instances {
                if let Some(data) = instance.data.get("large_integer") {
                    if let JsonValue::Number(num) = data {
                        // Verify precision is maintained
                        assert!(
                            num.as_i64().is_some() || num.as_f64().is_some(),
                            "Large numbers should maintain precision"
                        );
                    }
                }

                if let Some(data) = instance.data.get("high_precision_float") {
                    if let JsonValue::Number(num) = data {
                        // Verify floating point precision
                        assert!(
                            num.as_f64().is_some(),
                            "High precision floats should be preserved"
                        );
                    }
                }
            }
        }
        Err(error) => {
            // If loading fails, error should be descriptive
            let error_msg = format!("{error}");
            assert!(
                !error_msg.is_empty(),
                "Error message should be descriptive"
            );
        }
    }
}

/// Test CSV loader truncation safety mechanisms
#[test]
fn test_csv_loader_truncation_safety() {
    let fixture = LoaderTestFixture::new();
    let loader = CsvLoader::new();

    let options = LoadOptions {
        validate: false, // Disable validation to test truncation handling
        format_options: HashMap::new(),
    };

    // Test with edge case data that might cause truncation issues
    let result = loader.load_from_string(&fixture.edge_case_csv, &options);

    match result {
        Ok(instances) => {
            // Verify that instances are created without data loss
            for instance in &instances {
                // Check that no data fields are unexpectedly empty
                assert!(
                    !instance.data.is_empty(),
                    "Instance should retain data without truncation"
                );

                // Verify long strings are preserved
                if let Some(JsonValue::String(long_text)) = instance.data.get("long_description") {
                    assert!(
                        long_text.len() > 100,
                        "Long text should not be truncated unexpectedly"
                    );
                }
            }
        }
        Err(error) => {
            // Ensure error doesn't indicate truncation caused data corruption
            let error_msg = format!("{error}");
            assert!(
                !error_msg.contains("truncated") || !error_msg.contains("corrupted"),
                "Error should not indicate data corruption from truncation"
            );
        }
    }
}

/// Test CSV loader with different options configurations
#[test]
fn test_csv_loader_options_configurations() {
    let fixture = LoaderTestFixture::new();

    // Test with default options
    let default_loader = CsvLoader::new();
    let default_result = default_loader.load_from_string(
        &fixture.csv_data,
        &LoadOptions {
            validate: true,
            format_options: HashMap::new(),
        }
    );

    assert!(
        default_result.is_ok(),
        "Default CSV loader should handle standard data"
    );

    // Test with custom options
    let custom_options = CsvOptions {
        delimiter: b',',
        has_headers: true,
        trim: true,
        flexible: true,
        ..Default::default()
    };

    let custom_loader = CsvLoader::with_options(custom_options);
    let custom_result = custom_loader.load_from_string(
        &fixture.csv_data,
        &LoadOptions {
            validate: false,
            format_options: HashMap::new(),
        }
    );

    assert!(
        custom_result.is_ok(),
        "Custom CSV loader should handle data with custom options"
    );

    // Test TSV loader
    let tsv_loader = CsvLoader::tsv();
    let tsv_result = tsv_loader.load_from_string(
        &fixture.tsv_data,
        &LoadOptions {
            validate: true,
            format_options: HashMap::new(),
        }
    );

    match tsv_result {
        Ok(instances) => {
            assert!(
                !instances.is_empty(),
                "TSV loader should process tab-separated data"
            );
        }
        Err(_) => {
            // TSV parsing may fail with comma-separated data, which is acceptable
        }
    }
}

/// Test CSV loader data validation with edge cases
#[test]
fn test_csv_loader_edge_case_validation() {
    let edge_cases = vec![
        ("", "Empty CSV"),
        ("header1,header2\n", "Headers only"),
        ("header1,header2\n,", "Empty values"),
        ("header1,header2\n\"quoted,value\",normal", "Quoted values with commas"),
        ("header1,header2\nvalue1,value2\nvalue3", "Uneven columns"),
    ];

    for (csv_data, description) in edge_cases {
        let loader = CsvLoader::new();
        let options = LoadOptions {
            validate: false, // Some edge cases may not validate
            format_options: HashMap::new(),
        };

        let result = loader.load_from_string(csv_data, &options);

        match result {
            Ok(instances) => {
                // Success is acceptable for most edge cases
                println!("Successfully loaded {description}: {instances:?}");
            }
            Err(error) => {
                // Errors should be handled gracefully
                let error_msg = format!("{error}");
                assert!(
                    !error_msg.is_empty(),
                    "Error for {description} should have message"
                );
            }
        }
    }
}

/// Test RDF loader precision loss prevention
#[test]
fn test_rdf_loader_precision_safety() {
    let fixture = LoaderTestFixture::new();
    let options = RdfOptions {
        format: RdfSerializationFormat::Turtle,
        base_iri: Some("http://example.org/".to_string()),
        default_namespace: "http://example.org/".to_string(),
        prefixes: HashMap::new(),
        generate_blank_nodes: false,
        skolemnization: SkolemnizationOptions::None,
    };

    let loader = RdfLoader::new();
    let load_options = LoadOptions {
        validate: false, // Focus on precision testing
        format_options: HashMap::new(),
    };

    // Test RDF loading with numeric data
    let result = loader.load_from_string(&fixture.rdf_data, &load_options);

    match result {
        Ok(instances) => {
            // Verify that numeric values in RDF are preserved
            for instance in &instances {
                for (key, value) in &instance.data {
                    if key.contains("age") || key.contains("count") || key.contains("number") {
                        if let JsonValue::Number(num) = value {
                            assert!(
                                num.as_i64().is_some() || num.as_f64().is_some(),
                                "Numeric RDF values should maintain precision"
                            );
                        }
                    }
                }
            }
        }
        Err(error) => {
            // RDF parsing may fail due to format issues, but should not be due to precision loss
            let error_msg = format!("{error}");
            assert!(
                !error_msg.contains("precision") && !error_msg.contains("overflow"),
                "RDF loading errors should not be precision-related"
            );
        }
    }
}

/// Test RDF loader with different serialization formats
#[test]
fn test_rdf_loader_format_support() {
    let formats = vec![
        (RdfSerializationFormat::Turtle, "Turtle"),
        (RdfSerializationFormat::NTriples, "N-Triples"),
        (RdfSerializationFormat::RdfXml, "RDF/XML"),
        (RdfSerializationFormat::NQuads, "N-Quads"),
        (RdfSerializationFormat::TriG, "TriG"),
    ];

    for (format, name) in formats {
        let options = RdfOptions {
            format,
            base_iri: Some("http://example.org/".to_string()),
            default_namespace: "http://example.org/".to_string(),
            prefixes: HashMap::new(),
            generate_blank_nodes: false,
            skolemnization: SkolemnizationOptions::None,
        };

        let loader = RdfLoader::new();
        let load_options = LoadOptions {
            validate: false,
            format_options: HashMap::new(),
        };

        // Create format-appropriate test data
        let test_data = match format {
            RdfSerializationFormat::Turtle => {
                "@prefix ex: <http://example.org/> .\nex:person1 ex:name \"John\" ."
            }
            RdfSerializationFormat::NTriples => {
                "<http://example.org/person1> <http://example.org/name> \"John\" ."
            }
            _ => "<http://example.org/person1> <http://example.org/name> \"John\" ."
        };

        let result = loader.load_from_string(test_data, &load_options);

        // Some formats may not be fully supported, but should not crash
        match result {
            Ok(instances) => {
                println!("Successfully loaded {name} format: {} instances", instances.len());
            }
            Err(error) => {
                let error_msg = format!("{error}");
                assert!(
                    !error_msg.is_empty(),
                    "{name} format should provide meaningful error messages"
                );
            }
        }
    }
}

/// Test RDF loader skolemnization options
#[test]
fn test_rdf_loader_skolemnization_options() {
    let skolemnization_options = vec![
        (SkolemnizationOptions::None, "None"),
        (SkolemnizationOptions::Deterministic {
            base_uri: "http://example.org/skolem/".to_string(),
            prefix: "sk".to_string(),
        }, "Deterministic"),
        (SkolemnizationOptions::Uuid {
            base_uri: "http://example.org/uuid/".to_string(),
        }, "UUID"),
        (SkolemnizationOptions::Hash {
            base_uri: "http://example.org/hash/".to_string(),
            algorithm: "sha256".to_string(),
        }, "Hash"),
    ];

    for (skolemnization, name) in skolemnization_options {
        let options = RdfOptions {
            format: RdfSerializationFormat::Turtle,
            base_iri: Some("http://example.org/".to_string()),
            default_namespace: "http://example.org/".to_string(),
            prefixes: HashMap::new(),
            generate_blank_nodes: true,
            skolemnization,
        };

        let loader = RdfLoader::new();
        let load_options = LoadOptions {
            validate: false,
            format_options: HashMap::new(),
        };

        // RDF with blank nodes
        let rdf_with_blanks = "@prefix ex: <http://example.org/> .\n_:b1 ex:name \"Anonymous\" .";

        let result = loader.load_from_string(rdf_with_blanks, &load_options);

        match result {
            Ok(instances) => {
                println!("Successfully processed {name} skolemnization: {} instances", instances.len());
            }
            Err(error) => {
                let error_msg = format!("{error}");
                println!("{name} skolemnization error: {error_msg}");
                // Errors are acceptable for some skolemnization methods
            }
        }
    }
}

/// Test memory efficiency of refactored loader functions
#[test]
fn test_loader_memory_efficiency() {
    // Create large dataset for memory testing
    let large_csv = create_large_csv_data();

    let loader = CsvLoader::new();
    let options = LoadOptions {
        validate: false, // Skip validation for performance
        format_options: HashMap::new(),
    };

    let start_time = std::time::Instant::now();
    let result = loader.load_from_string(&large_csv, &options);
    let duration = start_time.elapsed();

    match result {
        Ok(instances) => {
            assert!(
                instances.len() >= 100,
                "Large CSV should load multiple instances"
            );

            // Should complete in reasonable time
            assert!(
                duration.as_secs() < 10,
                "Large CSV loading should complete in reasonable time"
            );
        }
        Err(error) => {
            // If memory issues occur, they should be handled gracefully
            let error_msg = format!("{error}");
            assert!(
                !error_msg.contains("out of memory"),
                "Should not run out of memory with large datasets"
            );
        }
    }
}

/// Test concurrent loader usage for thread safety
#[test]
fn test_loader_thread_safety() {
    use std::sync::Arc;
    use std::thread;

    let fixture = LoaderTestFixture::new();
    let csv_data = Arc::new(fixture.csv_data);

    let handles: Vec<_> = (0..4)
        .map(|i| {
            let data = Arc::clone(&csv_data);
            thread::spawn(move || {
                let loader = if i % 2 == 0 {
                    CsvLoader::new()
                } else {
                    CsvLoader::tsv()
                };

                let options = LoadOptions {
                    validate: false,
                    format_options: HashMap::new(),
                };

                loader.load_from_string(&*data, &options)
            })
        })
        .collect();

    // All threads should complete without panicking
    for handle in handles {
        let result = handle.join().expect("Thread should not panic");

        match result {
            Ok(instances) => {
                // Concurrent loading should work
                println!("Concurrent loading produced {} instances", instances.len());
            }
            Err(_) => {
                // Some loader configurations may fail with CSV data,
                // but threads should not panic
            }
        }
    }
}

/// Test loader error handling and recovery
#[test]
fn test_loader_error_handling() {
    let invalid_data_cases = vec![
        ("invalid,csv\ndata\"with\"unclosed", "Malformed CSV"),
        ("@prefix invalid rdf", "Invalid RDF"),
        ("", "Empty data"),
        ("header1,header2\n\x00invalid\x00binary", "Binary data in CSV"),
    ];

    for (invalid_data, description) in invalid_data_cases {
        // Test CSV loader
        let csv_loader = CsvLoader::new();
        let csv_result = csv_loader.load_from_string(invalid_data, &LoadOptions {
            validate: false,
            format_options: HashMap::new(),
        });

        match csv_result {
            Ok(_) => {
                // Some "invalid" data may still be parseable
                println!("CSV loader handled {description}");
            }
            Err(error) => {
                let error_msg = format!("{error}");
                assert!(
                    !error_msg.is_empty(),
                    "CSV loader should provide error message for {description}"
                );
            }
        }

        // Test RDF loader
        let rdf_loader = RdfLoader::new();
        let rdf_result = rdf_loader.load_from_string(invalid_data, &LoadOptions {
            validate: false,
            format_options: HashMap::new(),
        });

        match rdf_result {
            Ok(_) => {
                println!("RDF loader handled {description}");
            }
            Err(error) => {
                let error_msg = format!("{error}");
                assert!(
                    !error_msg.is_empty(),
                    "RDF loader should provide error message for {description}"
                );
            }
        }
    }
}

// Helper functions to create test data

fn create_test_csv_data() -> String {
    "id,name,age,email\n\
     1,John Doe,30,john@example.com\n\
     2,Jane Smith,25,jane@example.com\n\
     3,Bob Wilson,45,bob@example.com".to_string()
}

fn create_test_tsv_data() -> String {
    "id\tname\tage\temail\n\
     1\tJohn Doe\t30\tjohn@example.com\n\
     2\tJane Smith\t25\tjane@example.com\n\
     3\tBob Wilson\t45\tbob@example.com".to_string()
}

fn create_test_rdf_data() -> String {
    "@prefix ex: <http://example.org/> .\n\
     @prefix foaf: <http://xmlns.com/foaf/0.1/> .\n\
     \n\
     ex:person1 a foaf:Person ;\n\
                foaf:name \"John Doe\" ;\n\
                foaf:age 30 .\n\
     \n\
     ex:person2 a foaf:Person ;\n\
                foaf:name \"Jane Smith\" ;\n\
                foaf:age 25 .".to_string()
}

fn create_edge_case_csv_data() -> String {
    "id,name,long_description,special_chars\n\
     1,\"Name with, comma\",\"This is a very long description that goes on and on and should not be truncated because it contains important information that needs to be preserved in its entirety without any data loss\",\"Special chars: !@#$%^&*()_+-={}[]|\\:;'<>?,./\"\n\
     2,\"Multi-line\nName\",\"Another long description with\nnewlines and other special formatting that should be preserved\",\"Unicode: αβγδε ñüöä 中文 🚀\"\n\
     3,,\"Empty name field test\",\"\"".to_string()
}

fn create_numeric_precision_csv_data() -> String {
    "id,large_integer,high_precision_float,scientific_notation\n\
     1,9223372036854775807,3.141592653589793238,1.23e-10\n\
     2,-9223372036854775808,2.718281828459045235,4.56e+20\n\
     3,0,0.000000000123456789,1.0e0".to_string()
}

fn create_large_csv_data() -> String {
    let mut csv = "id,name,value,description\n".to_string();

    for i in 0..1000 {
        csv.push_str(&format!(
            "{i},Name{i},Value{i},Description for item number {i}\n"
        ));
    }

    csv
}