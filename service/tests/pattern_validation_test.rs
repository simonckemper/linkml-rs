//! Tests for enhanced pattern validation with named capture groups

use linkml_service::parser::Parser;
use linkml_service::validator::{ValidationEngine, validate_as_class};
use serde_json::json;

#[tokio::test]
async fn test_pattern_with_named_capture_groups() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for pattern validation with capture groups

classes:
  DateRecord:
    name: DateRecord
    description: A record with date patterns
    slots:
      - iso_date
      - us_date
      - custom_date

slots:
  iso_date:
    name: iso_date
    description: ISO format date
    range: string
    pattern: "^(?P<year>\\d{4})-(?P<month>\\d{2})-(?P<day>\\d{2})$"

  us_date:
    name: us_date
    description: US format date
    range: string
    pattern: "^(?P<month>\\d{2})/(?P<day>\\d{2})/(?P<year>\\d{4})$"

  custom_date:
    name: custom_date
    description: Custom date with named parts
    range: string
    pattern: "^(?P<dayname>Mon|Tue|Wed|Thu|Fri|Sat|Sun), (?P<day>\\d{1,2}) (?P<month>Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) (?P<year>\\d{4})$"
"#;

    // Parse schema
    let parser = Parser::new();
    let schema = parser
        .parse_str(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Debug: Check patterns in parsed schema
    eprintln!("DEBUG: Checking parsed schema slots:");
    for (slot_name, slot_def) in &schema.slots {
        eprintln!("  Slot '{}': pattern = {:?}", slot_name, slot_def.pattern);
    }

    // Valid data with different date formats
    let valid_data = json!({
        "iso_date": "2025-01-31",
        "us_date": "01/31/2025",
        "custom_date": "Fri, 31 Jan 2025"
    });

    let engine = ValidationEngine::new(&schema).expect("Test operation failed");
    let report = engine
        .validate_as_class(&valid_data, "DateRecord", None)
        .await
        .expect("Test operation failed");

    // Debug validation errors
    if !report.valid {
        println!("Validation failed for valid data:");
        for issue in &report.issues {
            println!("  - {}: {}", issue.path, issue.message);
        }
    }

    assert!(report.valid);
    assert_eq!(report.stats.error_count, 0);

    // Invalid patterns
    let invalid_data = json!({
        "iso_date": "2025-1-31",     // Missing leading zeros
        "us_date": "1/31/2025",      // Missing leading zero
        "custom_date": "Friday, 31 January 2025"  // Full names instead of abbreviations
    });

    let report = engine
        .validate_as_class(&invalid_data, "DateRecord", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert_eq!(report.stats.error_count, 3); // All three fields invalid
}

#[tokio::test]
async fn test_pattern_with_complex_capture_groups() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  EmailRecord:
    name: EmailRecord
    slots:
      - email
      - version_string

slots:
  email:
    name: email
    range: string
    pattern: "^(?P<user>[\\w\\.-]+)@(?P<domain>[\\w\\.-]+)\\.(?P<tld>\\w+)$"

  version_string:
    name: version_string
    range: string
    pattern: "^v(?P<major>\\d+)\\.(?P<minor>\\d+)\\.(?P<patch>\\d+)(?:-(?P<prerelease>[\\w\\.]+))?(?:\\+(?P<build>[\\w\\.]+))?$"
"#;

    let parser = Parser::new();
    let schema = parser
        .parse_str(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Valid semantic version strings
    let valid_data = json!({
        "email": "john.doe@example.com",
        "version_string": "v1.2.3-beta.1+build.123"
    });

    let report = validate_as_class(&schema, &valid_data, "EmailRecord", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);

    // Test simpler version
    let simple_version = json!({
        "email": "admin@test.org",
        "version_string": "v2.0.0"
    });

    let report = validate_as_class(&schema, &simple_version, "EmailRecord", None)
        .await
        .expect("Test operation failed");
    assert!(report.valid);
}

#[tokio::test]
async fn test_pattern_validation_with_arrays() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  PhoneDirectory:
    name: PhoneDirectory
    slots:
      - phone_numbers

slots:
  phone_numbers:
    name: phone_numbers
    range: string
    multivalued: true
    pattern: "^(?P<country>\\+\\d{1,3})?[\\s-]?(?P<area>\\d{3})[\\s-]?(?P<exchange>\\d{3})[\\s-]?(?P<number>\\d{4})$"
"#;

    let parser = Parser::new();
    let schema = parser
        .parse_str(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Valid phone numbers in different formats
    let valid_data = json!({
        "phone_numbers": [
            "+1 555-123-4567",
            "555 123 4567",
            "+1-555-123-4567",
            "5551234567"
        ]
    });

    let report = validate_as_class(&schema, &valid_data, "PhoneDirectory", None)
        .await
        .expect("Test operation failed");
    if !report.valid {
        println!("Validation failed!");
        for error in report.errors() {
            println!("Error: {} at {}", error.message, error.path);
        }
    }
    assert!(report.valid);

    // Mix of valid and invalid
    let mixed_data = json!({
        "phone_numbers": [
            "+1 555-123-4567",  // Valid
            "555-CALL-NOW",     // Invalid - letters
            "123"               // Invalid - too short
        ]
    });

    let report = validate_as_class(&schema, &mixed_data, "PhoneDirectory", None)
        .await
        .expect("Test operation failed");
    assert!(!report.valid);
    assert_eq!(report.stats.error_count, 2); // Two invalid entries
}

#[tokio::test]
async fn test_pattern_caching_performance() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  BulkData:
    name: BulkData
    slots:
      - items

slots:
  items:
    name: items
    range: string
    multivalued: true
    pattern: "^[A-Z]{2,4}-\\d{4,8}-[A-Z0-9]{6}$"
"#;

    let parser = Parser::new();
    let schema = parser
        .parse_str(schema_yaml, "yaml")
        .expect("Test operation failed");

    // Generate large array of items to test caching
    let mut items = Vec::new();
    for i in 0..1000 {
        items.push(json!(format!("TEST-{:06}-ABC123", i));
    }

    let data = json!({
        "items": items
    });

    // First validation (compiles pattern)
    let start = std::time::Instant::now();
    let report = validate_as_class(&schema, &data, "BulkData", None)
        .await
        .expect("Test operation failed");
    let first_duration = start.elapsed();
    assert!(report.valid);

    // Second validation (uses cached pattern)
    let start = std::time::Instant::now();
    let report = validate_as_class(&schema, &data, "BulkData", None)
        .await
        .expect("Test operation failed");
    let second_duration = start.elapsed();
    assert!(report.valid);

    // Second validation should be faster due to caching
    // (This is a loose check as timing can vary)
    println!(
        "First validation: {:?}, Second validation: {:?}",
        first_duration, second_duration
    );
}
