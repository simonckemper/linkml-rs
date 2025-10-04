//! Example demonstrating parsing and validating ISO3166 data
//!
//! This example shows a practical workflow:
//! 1. Parse ISO3166Entity.yaml file
//! 2. Extract country codes
//! 3. Validate specific identifiers against the extracted values
//! 4. Demonstrate the complete validation pipeline

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Represents an ISO 3166 entity from the instance file
#[derive(Debug, Serialize, Deserialize)]
struct ISO3166Entity {
    id: String,
    label: String,
    tld: Option<String>,
    exact_mappings: Option<Vec<String>>,
    notes: Option<String>,
}

/// Parse result containing extracted data
#[derive(Debug)]
struct ParseResult {
    entities: Vec<ISO3166Entity>,
    valid_codes: Vec<String>,
    code_to_label: HashMap<String, String>,
}

/// Validation result for a country code
#[derive(Debug)]
struct ValidationResult {
    code: String,
    is_valid: bool,
    pattern_valid: bool,
    instance_valid: bool,
    errors: Vec<String>,
}

/// Main parse and validation service
struct CountryCodeValidator {
    pattern: Regex,
    parse_result: Option<ParseResult>,
}

impl CountryCodeValidator {
    fn new() -> Self {
        // Pattern for ISO 3166-1 alpha-2 codes: exactly 2 uppercase letters
        let pattern = Regex::new(r"^[A-Z]{2}$").expect("regex in test should be valid");
        Self {
            pattern,
            parse_result: None,
        }
    }

    /// Parse the ISO3166Entity.yaml file
    fn parse_instances(
        &mut self,
        file_path: &PathBuf,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!(
            "Parsing ISO3166Entity instances from: {}",
            file_path.display()
        );

        // Read and parse the YAML file
        let content = fs::read_to_string(file_path)?;
        let entities: Vec<ISO3166Entity> = serde_yaml::from_str(&content)?;

        // Extract valid codes and create lookup map
        let mut valid_codes = Vec::new();
        let mut code_to_label = HashMap::new();

        for entity in &entities {
            valid_codes.push(entity.id.clone());
            code_to_label.insert(entity.id.clone(), entity.label.clone());
        }

        println!("✓ Parsed {} ISO 3166 entities", entities.len());
        println!("  - Valid country codes: {}", valid_codes.len());
        println!(
            "  - Sample codes: {:?}",
            &valid_codes[..5.min(valid_codes.len())]
        );

        self.parse_result = Some(ParseResult {
            entities,
            valid_codes,
            code_to_label,
        });

        Ok(())
    }

    /// Validate a country code
    fn validate_code(&self, code: &str) -> ValidationResult {
        let mut errors = Vec::new();

        // Step 1: Pattern validation
        let pattern_valid = self.pattern.is_match(code);
        if !pattern_valid {
            errors.push(format!(
                "Pattern validation failed: '{}' must be exactly 2 uppercase letters",
                code
            ));
        }

        // Step 2: Instance validation (only if pattern is valid)
        let instance_valid = if pattern_valid {
            if let Some(ref result) = self.parse_result {
                result.valid_codes.contains(&code.to_string())
            } else {
                errors.push("No instance data loaded".to_string());
                false
            }
        } else {
            false // Skip instance check if pattern fails
        };

        if pattern_valid && !instance_valid {
            errors.push(format!(
                "Instance validation failed: '{}' is not a valid ISO 3166-1 alpha-2 code",
                code
            ));
        }

        ValidationResult {
            code: code.to_string(),
            is_valid: pattern_valid && instance_valid,
            pattern_valid,
            instance_valid,
            errors,
        }
    }

    /// Get country label for a valid code
    fn get_country_label(&self, code: &str) -> Option<String> {
        self.parse_result
            .as_ref()
            .and_then(|r| r.code_to_label.get(code).cloned())
    }

    /// Extract specific fields using JSONPath-like queries
    fn extract_field(&self, field: &str) -> Vec<String> {
        if let Some(ref result) = self.parse_result {
            match field {
                "id" | "code" => result.valid_codes.clone(),
                "label" | "name" => result.entities.iter().map(|e| e.label.clone()).collect(),
                "tld" | "domain" => result
                    .entities
                    .iter()
                    .filter_map(|e| e.tld.clone())
                    .collect(),
                _ => vec![],
            }
        } else {
            vec![]
        }
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "=== Parse and Validate ISO 3166 Country Codes ===
"
    );

    // Initialize the validator
    let mut validator = CountryCodeValidator::new();

    // Path to the ISO3166Entity.yaml file
    let schema_base = PathBuf::from("/home/kempersc/apps/rootreal/domain/schema");
    let instances_path = schema_base.join("place/polity/country/ISO3166Entity.yaml");

    // ========================================================================
    // Part 1: Parse the instance file
    // ========================================================================

    println!("Part 1: Parsing Instance File");
    println!("{}", "=".repeat(50));

    validator.parse_instances(&instances_path)?;
    println!();

    // ========================================================================
    // Part 2: Extract specific data fields
    // ========================================================================

    println!("Part 2: Data Extraction");
    println!("{}", "=".repeat(50));

    // Extract different fields
    let codes = validator.extract_field("code");
    let labels = validator.extract_field("label");
    let domains = validator.extract_field("tld");

    println!("Extracted data:");
    println!("  - Country codes: {} total", codes.len());
    println!("    First 5: {:?}", &codes[..5.min(codes.len())]);
    println!("  - Country labels: {} total", labels.len());
    println!("    First 3: {:?}", &labels[..3.min(labels.len())]);
    println!("  - Top-level domains: {} total", domains.len());
    println!("    First 5: {:?}", &domains[..5.min(domains.len())]);
    println!();

    // ========================================================================
    // Part 3: Validate test cases
    // ========================================================================

    println!("Part 3: Validation Tests");
    println!("{}", "=".repeat(50));

    let test_cases = vec![
        ("US", "United States"),
        ("GB", "United Kingdom"),
        ("DE", "Germany"),
        ("FR", "France"),
        ("JP", "Japan"),
        ("CN", "China"),
        ("AU", "Australia"),
        ("XX", "Invalid - not in ISO 3166"),
        ("ZZ", "Invalid - not in ISO 3166"),
        ("us", "Invalid - lowercase"),
        ("USA", "Invalid - alpha-3 code"),
        ("U1", "Invalid - contains number"),
        ("123", "Invalid - all numbers"),
    ];

    println!(
        "Validating country codes:
"
    );

    for (code_input, description) in test_cases {
        let result = validator.validate_code(code_input);

        if result.is_valid {
            // For valid codes, also show the actual country name
            let country = validator.get_country_label(&result.code).unwrap_or_default();
            println!(
                "✓ {:3} - Valid   | {} ({}) [Pattern: ✓, Instance: ✓]",
                result.code, country, description
            );
        } else {
            // Show detailed validation status
            let pattern_status = if result.pattern_valid { "✓" } else { "✗" };
            let instance_status = if result.instance_valid { "✓" } else { "✗" };
            println!(
                "✗ {:3} - Invalid | {} [Pattern: {}, Instance: {}]",
                result.code, description, pattern_status, instance_status
            );
            for error in &result.errors {
                println!("       └─ {}", error);
            }
        }
    }

    println!();

    // ========================================================================
    // Part 4: Advanced validation with context
    // ========================================================================

    println!("Part 4: Contextual Validation");
    println!("{}", "=".repeat(50));

    // Simulate validating a document with country codes
    let document = json!({
        "addresses": [
            {"country": "US", "city": "New York"},
            {"country": "GB", "city": "London"},
            {"country": "XX", "city": "Unknown"},
        ]
    });

    println!("Validating country codes in document:");
    println!("{}", serde_json::to_string_pretty(&document)?);
    println!();

    if let Some(addresses) = document["addresses"].as_array() {
        for (idx, address) in addresses.iter().enumerate() {
            if let Some(country_input) = address["country"].as_str() {
                let result = validator.validate_code(country_input);
                let city = address["city"].as_str().unwrap_or("Unknown");

                if result.is_valid {
                    let country_name = validator.get_country_label(&result.code).unwrap_or_default();
                    println!(
                        "  Address {}: ✓ {} ({}) - {}",
                        idx + 1,
                        result.code,
                        country_name,
                        city
                    );
                } else {
                    let pattern_status = if result.pattern_valid { "✓" } else { "✗" };
                    let instance_status = if result.instance_valid { "✓" } else { "✗" };
                    println!(
                        "  Address {}: ✗ {} - {} [Pattern: {}, Instance: {}]",
                        idx + 1,
                        result.code,
                        city,
                        pattern_status,
                        instance_status
                    );
                }
            }
        }
    }

    println!();

    // ========================================================================
    // Part 5: Statistics and Summary
    // ========================================================================

    println!("Part 5: Parse Statistics");
    println!("{}", "=".repeat(50));

    if let Some(ref parse_result) = validator.parse_result {
        // Count entities by various criteria
        let with_notes = parse_result
            .entities
            .iter()
            .filter(|e| {
                e.notes.is_some()
                    && !e
                        .notes
                        .as_ref()
                        .expect("LinkML operation in test should succeed")
                        .is_empty()
            })
            .count();

        let with_mappings = parse_result
            .entities
            .iter()
            .filter(|e| e.exact_mappings.is_some())
            .count();

        println!("Parse statistics:");
        println!("  - Total entities: {}", parse_result.entities.len());
        println!("  - Entities with notes: {}", with_notes);
        println!("  - Entities with Wikidata mappings: {}", with_mappings);
        println!(
            "  - Unique TLDs: {}",
            parse_result
                .entities
                .iter()
                .filter_map(|e| e.tld.as_ref())
                .collect::<std::collections::HashSet<_>>()
                .len()
        );
    }

    println!();
    println!("=== Example Complete ===");
    println!();
    println!("Summary:");
    println!("✓ Successfully parsed ISO3166Entity.yaml");
    println!("✓ Extracted country codes, labels, and domains");
    println!("✓ Validated codes against pattern and instance data");
    println!("✓ Demonstrated contextual validation in documents");
    println!("✓ This represents the core parse → extract → validate pipeline");

    Ok(())
}
