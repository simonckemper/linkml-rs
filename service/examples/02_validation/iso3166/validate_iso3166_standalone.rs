//! Standalone example demonstrating LinkML validation with instance-based permissible values
//!
//! This example shows the core validation principle where:
//! 1. ISO3166Entity.yaml contains country codes as instances
//! 2. CountryCodeAlpha2Identifier references ISO3166Entity as its range
//! 3. Values must match the 'id' field from ISO3166Entity instances

use regex::Regex;
use serde_json::json;
use serde_yaml;
use std::fs;
use std::path::PathBuf;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== LinkML Instance-Based Validation (Standalone) ===
");

    // Paths
    let schema_base = PathBuf::from("/home/kempersc/apps/rootreal/domain/schema");
    let country_instances_path = schema_base.join("place/polity/country/ISO3166Entity.yaml");

    // Load and parse ISO3166Entity instances
    println!("Loading ISO3166Entity instances...");
    let instances_content = fs::read_to_string(&country_instances_path)?;
    let instances: serde_json::Value = serde_yaml::from_str(&instances_content)?;

    // Extract valid country codes from instances
    let mut valid_codes = Vec::new();
    if let Some(instances_array) = instances.as_array() {
        for instance in instances_array {
            if let Some(id) = instance.get("id").and_then(|v| v.as_str()) {
                valid_codes.push(id.to_string());
            }
        }
    }

    println!("Loaded {} valid country codes", valid_codes.len());
    println!("Examples: {:?}", &valid_codes[..5.min(valid_codes.len())]);
    println!();

    // Pattern from schema: (?P<CountryCodeAlpha2Identifier>[A-Z]{2})
    let pattern = Regex::new(r"^[A-Z]{2}$")?;

    println!("Validation Rules for CountryCodeAlpha2Identifier:");
    println!("  1. Must match pattern: [A-Z]{{2}} (exactly 2 uppercase letters)");
    println!("  2. Must be a valid ISO 3166-1 alpha-2 code from instances");
    println!();

    // Test cases
    let test_cases = vec![
        ("US", "United States - valid code"),
        ("GB", "United Kingdom - valid code"),
        ("DE", "Germany - valid code"),
        ("FR", "France - valid code"),
        ("JP", "Japan - valid code"),
        ("XX", "Invalid - not in ISO 3166-1"),
        ("ZZ", "Invalid - not in ISO 3166-1"),
        ("us", "Invalid - must be uppercase"),
        ("USA", "Invalid - three letters (alpha-3, not alpha-2)"),
        ("U1", "Invalid - contains number"),
        ("1A", "Invalid - starts with number"),
    ];

    println!("Validating test cases:");
    println!("{}", "-".repeat(60));

    for (code, description) in test_cases {
        // Step 1: Check pattern
        let pattern_valid = pattern.is_match(code);

        // Step 2: Check if in permissible values
        let instance_valid = valid_codes.contains(&code.to_string());

        // Both must be true for validation to pass
        let is_valid = pattern_valid && instance_valid;

        println!(
            "{} Code '{}' - {}",
            if is_valid { "✓" } else { "✗" },
            code,
            description
        );

        if !pattern_valid {
            println!("    └─ Failed pattern validation: must be exactly 2 uppercase letters");
        }
        if pattern_valid && !instance_valid {
            println!("    └─ Not in ISO 3166-1 permissible values");
        }
    }

    println!();
    println!("Key Insights:");
    println!("=============");
    println!("1. The pattern validates the FORMAT (2 uppercase letters)");
    println!("2. The instance reference validates the VALUES (must be real country codes)");
    println!("3. Both constraints must be satisfied for validation to pass");
    println!();
    println!("This is how LinkML combines structural validation (patterns)");
    println!("with semantic validation (permissible values from instances).");

    // Show how this would work in the schema
    println!();
    println!("Schema Definition:");
    println!("{}", "-".repeat(40));
    println!("CountryCodeAlpha2Identifier:");
    println!("  slot_usage:");
    println!("    identifier:");
    println!("      range: ISO3166Entity  # <-- References instance file");
    println!("      structured_pattern:");
    println!("        syntax: '{{country_code_alpha2_identifier_pattern}}'");
    println!();
    println!("The 'range: ISO3166Entity' tells the validator to check");
    println!("values against the 'id' field in ISO3166Entity.yaml instances.");

    Ok(())
}
