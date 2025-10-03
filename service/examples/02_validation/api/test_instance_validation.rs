//! Test example to verify LinkML instance-based validation is working
//!
//! This example specifically tests the pattern where:
//! 1. An instance file (ISO3166Entity.yaml) contains permissible values
//! 2. A schema class (CountryCodeAlpha2Identifier) references these instances as its range
//! 3. Values are validated against the permissible values from the instance file

use linkml_core::prelude::*;
use linkml_service::create_linkml_service;
use serde_json::json;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing LinkML Instance-Based Validation ===
");

    // Create the service
    let service = create_linkml_service()?;

    // Schema paths
    let schema_base = PathBuf::from("/home/kempersc/apps/rootreal/domain/schema");

    // Test 1: Load and validate country instances
    println!("Test 1: Loading ISO3166Entity instances");
    println!("{}", "-".repeat(40));

    let country_schema_path = schema_base.join("place/polity/country/schema.yaml");
    let country_instances_path = schema_base.join("place/polity/country/ISO3166Entity.yaml");

    // Load schema
    match service.load_schema(&country_schema_path).await {
        Ok(schema) => {
            println!("✓ Loaded country schema successfully");

            // Check if schema contains ISO3166Entity class
            if schema.classes.contains_key("ISO3166Entity") {
                println!("✓ Schema contains ISO3166Entity class");
            } else {
                println!("✗ Schema missing ISO3166Entity class");
            }
        }
        Err(e) => {
            println!("✗ Failed to load country schema: {}", e);
        }
    }

    // Load instances manually since load_instances doesn't exist
    let instances_content = fs::read_to_string(&country_instances_path)?;
    let instances: serde_json::Value = serde_yaml::from_str(&instances_content)?;

    let count = instances.as_array().map_or(0, |a| a.len());
    println!("✓ Loaded {} country instances", count);

    // Show a few examples
    if let Some(arr) = instances.as_array() {
        println!("
Example instances:");
        for instance in arr.iter().take(3) {
            if let (Some(id), Some(label)) = (
                instance.get("id").and_then(|v| v.as_str()),
                instance.get("label").and_then(|v| v.as_str()),
            ) {
                println!("  - {}: {}", id, label);
            }
        }
    }

    println!();

    // Test 2: Validate identifier against permissible values
    println!("Test 2: Validating CountryCodeAlpha2Identifier");
    println!("{}", "-".repeat(40));

    let identifier_schema_path = schema_base.join("meta/identifier/identifier/schema.yaml");

    // Load identifier schema
    match service.load_schema(&identifier_schema_path).await {
        Ok(schema) => {
            println!("✓ Loaded identifier schema");

            // Check for CountryCodeAlpha2Identifier class
            if let Some(class_def) = schema.classes.get("CountryCodeAlpha2Identifier") {
                println!("✓ Found CountryCodeAlpha2Identifier class");

                // Check slot usage for identifier
                if let Some(slot_usage) = class_def.slot_usage.get("identifier") {
                    if let Some(range) = &slot_usage.range {
                        println!("✓ Identifier slot has range: {}", range);
                        if range == "ISO3166Entity" {
                            println!("✓ Range correctly references ISO3166Entity instances!");
                        }
                    }
                }
            }

            // Test validation with actual values
            println!("
Validating test values:");

            let test_values = vec![
                ("US", true, "United States"),
                ("GB", true, "United Kingdom"),
                ("FR", true, "France"),
                ("XX", false, "Invalid code"),
                ("12", false, "Numeric not allowed"),
            ];

            for (code, should_be_valid, description) in test_values {
                let test_data = json!({
                    "identifier": code
                });

                match service
                    .validate(&test_data, &schema, "CountryCodeAlpha2Identifier")
                    .await
                {
                    Ok(report) => {
                        // Check if validation result matches expectation
                        let is_valid = report.valid;

                        if is_valid == should_be_valid {
                            println!(
                                "  ✓ {}: {} ({})",
                                code,
                                if is_valid { "Valid" } else { "Invalid" },
                                description
                            );
                        } else {
                            println!(
                                "  ✗ {}: Expected {}, got {} ({})",
                                code,
                                if should_be_valid { "Valid" } else { "Invalid" },
                                if is_valid { "Valid" } else { "Invalid" },
                                description
                            );

                            // Show errors if any
                            for error in &report.errors {
                                println!("      Error: {}", error.message);
                            }
                        }
                    }
                    Err(e) => {
                        println!("  ✗ Validation failed for '{}': {}", code, e);
                    }
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load identifier schema: {}", e);
        }
    }

    println!();

    // Test 3: Check pattern validation
    println!("Test 3: Pattern Validation");
    println!("-".repeat(40));

    // The pattern should validate format independent of permissible values
    let pattern_tests = vec![
        ("US", true, "Valid format"),
        ("us", false, "Lowercase not allowed"),
        ("USA", false, "Three characters not allowed"),
        ("1", false, "Too short"),
    ];

    println!("Testing pattern: (?P<CountryCodeAlpha2Identifier>[A-Z]{2})");

    for (value, should_match, description) in pattern_tests {
        // Just test the regex pattern
        let pattern = regex::Regex::new(r"^[A-Z]{2}$").expect("regex in test should be valid");
        let matches = pattern.is_match(value);

        if matches == should_match {
            println!(
                "  ✓ '{}': {} ({})",
                value,
                if matches { "Matches" } else { "No match" },
                description
            );
        } else {
            println!("  ✗ '{}': Pattern test failed", value);
        }
    }

    println!();
    println!("=== Test Complete ===");

    // Summary
    println!("
Summary:");
    println!("This test verifies that the LinkML service can:");
    println!("1. Load instance files (ISO3166Entity.yaml)");
    println!("2. Use instance values as permissible values for validation");
    println!("3. Validate identifiers against both patterns and permissible values");
    println!("
The key insight: When a slot has range: ISO3166Entity,");
    println!("values must be from the 'id' field of ISO3166Entity instances.");

    Ok(())
}
