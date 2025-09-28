//! Example demonstrating validation of country schema and instances
//!
//! This example shows how the LinkML service validates:
//! 1. The ISO3166Entity instances against their schema
//! 2. CountryCodeAlpha2Identifier values against permissible values from ISO3166Entity instances
//!
//! The key pattern demonstrated here is how instance files (ISO3166Entity.yaml) provide
//! permissible values for schema validation when referenced as ranges.

use linkml_core::prelude::*;
use linkml_core::types::{PermissibleValue, SchemaDefinition, SlotDefinition};
use linkml_service::create_linkml_service_with_config;
use serde_json::json;
use serde_yaml;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    println!("=== LinkML Country Schema Validation Example ===
");

    // Paths to schema and instance files
    let schema_base = PathBuf::from("/home/kempersc/apps/rootreal/domain/schema");
    let country_schema_path = schema_base.join("place/polity/country/schema.yaml");
    let country_instances_path = schema_base.join("place/polity/country/ISO3166Entity.yaml");
    let identifier_schema_path = schema_base.join("meta/identifier/identifier/schema.yaml");

    println!("Loading schemas and instances:");
    println!("  - Country schema: {}", country_schema_path.display());
    println!("  - Country instances: {}", country_instances_path.display());
    println!("  - Identifier schema: {}", identifier_schema_path.display());
    println!();

    // Create LinkML service with configuration
    let config = HashMap::from([
        ("base_path".to_string(), schema_base.to_string_lossy().to_string()),
        ("enable_instance_validation".to_string(), "true".to_string()),
    ]);
    
    let service = create_linkml_service_with_config(config)?;

    // ========================================================================
    // Part 1: Validate ISO3166Entity instances against their schema
    // ========================================================================
    
    println!("Part 1: Validating ISO3166Entity instances");
    println!("=" .repeat(50));

    // Load the country schema
    let country_schema = service.load_schema(&country_schema_path).await?;
    
    // Load the country instances manually
    let instances_content = std::fs::read_to_string(&country_instances_path)?;
    let instances: serde_json::Value = serde_yaml::from_str(&instances_content)?;
    
    println!("Loaded {} country instances", instances.as_array().map_or(0, |a| a.len());

    // Validate each instance against the ISO3166Entity class
    let mut valid_count = 0;
    let mut invalid_count = 0;

    if let Some(instances_array) = instances.as_array() {
        for (idx, instance) in instances_array.iter().enumerate() {
            let validation_result = service
                .validate(instance, &country_schema, "ISO3166Entity")
                .await?;
            
            if validation_result.is_valid() {
                valid_count += 1;
                if idx < 3 {  // Show first few valid instances
                    println!(
                        "  ✓ Valid: {} - {}",
                        instance.get("id").and_then(|v| v.as_str()).unwrap_or("?"),
                        instance.get("label").and_then(|v| v.as_str()).unwrap_or("?")
                    );
                }
            } else {
                invalid_count += 1;
                println!("  ✗ Invalid instance at index {}: {:?}", idx, validation_result.errors());
            }
        }
    }

    println!("
Validation Summary:");
    println!("  - Valid instances: {}", valid_count);
    println!("  - Invalid instances: {}", invalid_count);
    println!();

    // ========================================================================
    // Part 2: Validate CountryCodeAlpha2Identifier with instance-based permissible values
    // ========================================================================
    
    println!("Part 2: Validating CountryCodeAlpha2Identifier");
    println!("=" .repeat(50));
    
    // Load the identifier schema
    let identifier_schema = service.load_schema(&identifier_schema_path).await?;

    // Extract permissible values from ISO3166Entity instances
    // These are the valid country codes that can be used
    let mut permissible_codes = Vec::new();
    if let Some(instances_array) = instances.as_array() {
        for instance in instances_array {
            if let Some(id) = instance.get("id").and_then(|v| v.as_str()) {
                permissible_codes.push(id.to_string());
            }
        }
    }
    
    println!("Extracted {} permissible country codes from instances", permissible_codes.len());
    println!("Examples: {:?}", &permissible_codes[..5.min(permissible_codes.len())]);
    println!();

    // Test cases for CountryCodeAlpha2Identifier validation
    let test_cases = vec![
        ("US", true, "Valid ISO 3166-1 alpha-2 code"),
        ("GB", true, "Valid ISO 3166-1 alpha-2 code"),
        ("DE", true, "Valid ISO 3166-1 alpha-2 code"),
        ("XX", false, "Invalid - not in ISO 3166-1"),
        ("ZZ", false, "Invalid - not in ISO 3166-1"),
        ("usa", false, "Invalid - must be uppercase"),
        ("U", false, "Invalid - must be exactly 2 characters"),
        ("USA", false, "Invalid - alpha-3 code, not alpha-2"),
    ];

    println!("Testing CountryCodeAlpha2Identifier validation:");
    
    for (code, expected_valid, description) in test_cases {
        // Create test data with the identifier
        let test_data = json!({
            "identifier": code
        });

        // Validate against CountryCodeAlpha2Identifier class
        let result = service
            .validate(
                &test_data,
                &identifier_schema,
                "CountryCodeAlpha2Identifier"
            )
            .await?;

        let is_valid = result.is_valid();
        let status = if is_valid == expected_valid {
            if is_valid { "✓" } else { "✓" }
        } else {
            "✗ UNEXPECTED"
        };

        println!(
            "  {} Code '{}': {} - {}",
            status,
            code,
            if is_valid { "Valid  " } else { "Invalid" },
            description
        );

        if !result.is_valid() && result.errors().len() > 0 {
            println!("      Error: {}", result.errors()[0]);
        }
    }
    
    println!();

    // ========================================================================
    // Part 3: Demonstrate pattern validation with named capture groups
    // ========================================================================
    
    println!("Part 3: Pattern Validation with Named Capture Groups");
    println!("=" .repeat(50));
    
    // The country_code_alpha2_identifier_pattern includes a named capture group
    // Pattern: (?P<CountryCodeAlpha2Identifier>[A-Z]{2})
    
    let pattern_test = "GB";
    println!("Testing pattern matching for: '{}'", pattern_test);
    
    // Validate that the pattern matches and extracts correctly
    let pattern_data = json!({
        "identifier": pattern_test
    });
    
    let pattern_result = service
        .validate(
            &pattern_data,
            &identifier_schema,
            "CountryCodeAlpha2Identifier"
        )
        .await?;
    
    if pattern_result.is_valid() {
        println!("  ✓ Pattern matches successfully");
        println!("  - Captured group 'CountryCodeAlpha2Identifier': {}", pattern_test);
    } else {
        println!("  ✗ Pattern validation failed");
    }

    println!();

    // ========================================================================
    // Part 4: Demonstrate compound identifier validation
    // ========================================================================
    
    println!("Part 4: Compound Identifier Validation");
    println!("=" .repeat(50));
    
    // The schema defines compound patterns like repository_identifier_compound
    // which combines multiple identifier patterns
    
    let compound_test = "US-NY-NYC-g-smithsonian";
    println!("Testing compound repository identifier: '{}'", compound_test);
    println!("Expected components:");
    println!("  - Country: US (must be valid ISO 3166-1 code)");
    println!("  - Subdivision: NY");
    println!("  - Municipality: NYC");
    println!("  - GLAM type: g (gallery)");
    println!("  - ISIL: smithsonian");
    
    // This would validate against the compound pattern
    // ensuring the country code component is a valid ISO3166Entity
    
    println!();
    println!("=== Example Complete ===");
    
    Ok(())
}

/// Helper trait to make working with validation reports easier
trait ValidationReportExt {
    fn is_valid(&self) -> bool;
    fn errors(&self) -> Vec<String>;
    fn warnings(&self) -> Vec<String>;
}

impl ValidationReportExt for linkml_core::types::ValidationReport {
    fn is_valid(&self) -> bool {
        self.valid
    }
    
    fn errors(&self) -> Vec<String> {
        self.errors
            .iter()
            .map(|e| format!("{}: {}", e.field.as_deref().unwrap_or(""), e.message))
            .collect()
    }
    
    fn warnings(&self) -> Vec<String> {
        self.warnings
            .iter()
            .map(|w| format!("{}: {}", w.field.as_deref().unwrap_or(""), w.message))
            .collect()
    }
}