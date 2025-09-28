//! Debug test for pattern resolution issue

use linkml_service::parser::Parser;
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;

#[tokio::test]
async fn test_pattern_resolution_debug() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for pattern validation

classes:
  DateRecord:
    name: DateRecord
    description: A record with date patterns
    slots:
      - iso_date
      - us_date

slots:
  iso_date:
    name: iso_date
    description: ISO format date
    range: string
    pattern: "^\\d{4}-\\d{2}-\\d{2}$"

  us_date:
    name: us_date
    description: US format date
    range: string
    pattern: "^\\d{2}/\\d{2}/\\d{4}$"
"#;

    // Parse schema
    let parser = Parser::new();
    let schema = parser
        .parse(schema_yaml, "yaml")
        .expect("LinkML operation in test should succeed");

    // Check parsed patterns
    eprintln!("
=== Parsed Schema Slots ===");
    for (slot_name, slot_def) in &schema.slots {
        eprintln!("Slot '{}': pattern = {:?}", slot_name, slot_def.pattern);
    }

    // Create test data
    let valid_data = json!({
        "iso_date": "2025-01-31",
        "us_date": "01/31/2025"
    });

    // Create validation engine
    let engine = ValidationEngine::new(&schema).expect("LinkML operation in test should succeed");

    // Validate
    eprintln!("
=== Starting Validation ===");
    let report = engine
        .validate_as_class(&valid_data, "DateRecord", None)
        .await
        .expect("LinkML operation in test should succeed");

    // Check results
    eprintln!("
=== Validation Results ===");
    eprintln!("Valid: {}", report.valid);
    eprintln!("Error count: {}", report.stats.error_count);

    for issue in &report.issues {
        eprintln!("Issue: {} at {}", issue.message, issue.path);
    }

    // Assert validation passed
    assert!(report.valid, "Both date formats should be valid");
}
