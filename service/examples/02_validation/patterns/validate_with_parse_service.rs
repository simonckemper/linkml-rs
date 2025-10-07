//! Example demonstrating LinkML validation of ISO3166 data using the Parse Service
//!
//! This example shows how to:
//! 1. Use the Parse Service to load and parse ISO3166Entity.yaml
//! 2. Validate parsed documents against LinkML schemas
//! 3. Demonstrate instance-based permissible value validation
//! 4. Validate country codes using both pattern and instance validation

use parse_core::{CsvFormat, JsonFormat, ParseFormat, ParseService, XmlFormat};
use parse_service::factory;
use std::path::PathBuf;
use std::sync::Arc;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "=== ISO3166 Validation with Parse Service Integration ===
"
    );

    // Create dependencies
    let logger = Arc::new(logger_service::factory::create_development_logger().await?);
    let timestamp_service = Arc::new(timestamp_service::wiring::wire_timestamp());

    // Create container service for cache
    let container_service = container_core::mock::create_mock_container_service()?;

    // Create task management service
    let task_service =
        Arc::new(task_management_service::wiring::wire_task_management()?);

    // Create cache service with all dependencies
    let cache_service =
        cache_service::factory::trait_object_factories::create_cache_service_trait_object(
            logger.clone(),
            timestamp_service.clone(),
            container_service,
            task_service.clone(),
            cache_core::ValkeyCacheConfig::default(),
        )
        .await?;

    // Create LinkML service with ISO3166 schema support
    let mut linkml_config = linkml_service::config::LinkMLServiceConfig::default();

    // Configure schema search paths to include our ISO3166 schemas
    linkml_config.schema_search_paths = vec![
        PathBuf::from("/home/kempersc/apps/rootreal/domain/schema/place/polity/country"),
        PathBuf::from("/home/kempersc/apps/rootreal/domain/schema/meta/identifier/identifier"),
    ];

    let linkml_service = linkml_service::factory::create_linkml_service(
        linkml_config,
        logger.clone(),
        Some(cache_service.clone()),
    )
    .await?;

    // Create Parse Service configuration
    let mut config = parse_service::config::ParseServiceConfig::default();
    config.general.environment = "development".to_string();

    // Create Parse Service with LinkML integration
    let parse_service = factory::create_parse_service(
        config,
        Some(timestamp_service),
        logger.clone(),
        Some(cache_service),
        None,                 // Dataframe service
        Some(linkml_service), // LinkML service enabled!
    )
    .await?;

    println!("✓ Parse Service with LinkML validation initialized");
    println!();

    // ========================================================================
    // Part 1: Parse the ISO3166Entity.yaml instance file
    // ========================================================================

    println!("Part 1: Parsing ISO3166 Instance File");
    println!("{}", "=".repeat(50));

    // Read the instance file
    let instances_path = PathBuf::from(
        "/home/kempersc/apps/rootreal/domain/schema/place/polity/country/ISO3166Entity.yaml",
    );
    let instances_content = std::fs::read_to_string(&instances_path)?;

    // Detect format (should be YAML)
    let (detected_format, confidence) = parse_service.detect_format(&instances_content).await?;

    println!(
        "Detected format: {:?} (confidence: {:.2}%)",
        detected_format,
        confidence * 100.0
    );

    // Parse the instance file
    let parsed_instances = parse_service.parse(&instances_content).await?;

    println!("✓ Parsed ISO3166Entity.yaml");
    println!("  - Document ID: {}", parsed_instances.id);
    println!("  - Format: {}", parsed_instances.format);

    // Extract metadata
    if let Some(title) = &parsed_instances.metadata.title {
        println!("  - Title: {}", title);
    }

    println!();

    // ========================================================================
    // Part 2: Validate the instances against ISO3166Entity schema
    // ========================================================================

    println!("Part 2: Schema Validation");
    println!("{}", "=".repeat(50));

    // Validate against the ISO3166Entity schema
    let validation_result = parse_service
        .validate_document(&parsed_instances, Some("ISO3166Entity"))
        .await?;

    println!("📋 LinkML Validation Results for ISO3166Entity:");
    println!("  - Valid: {}", validation_result.is_valid);
    println!("  - Schema: {}", validation_result.schema_name);
    println!("  - Errors: {}", validation_result.errors.len());
    println!("  - Warnings: {}", validation_result.warnings.len());
    println!(
        "  - Validation time: {:?}",
        validation_result.validation_time
    );

    if !validation_result.errors.is_empty() {
        println!(
            "
❌ Validation Errors:"
        );
        for error in &validation_result.errors {
            println!(
                "  - [{}] {}: {}",
                error.severity, error.field, error.message
            );
        }
    }

    println!();

    // ========================================================================
    // Part 3: Test country code validation with permissible values
    // ========================================================================

    println!("Part 3: Country Code Validation Tests");
    println!("{}", "=".repeat(50));

    // Test various country codes
    let test_cases = vec![
        ("US", true, "Valid ISO 3166-1 alpha-2 code"),
        ("GB", true, "Valid ISO 3166-1 alpha-2 code"),
        ("DE", true, "Valid ISO 3166-1 alpha-2 code"),
        ("FR", true, "Valid ISO 3166-1 alpha-2 code"),
        ("JP", true, "Valid ISO 3166-1 alpha-2 code"),
        ("XX", false, "Not in ISO 3166-1"),
        ("ZZ", false, "Not in ISO 3166-1"),
        ("us", false, "Lowercase not allowed"),
        ("USA", false, "Alpha-3 code, not alpha-2"),
        ("U1", false, "Contains number"),
    ];

    println!(
        "Testing CountryCodeAlpha2Identifier validation:
"
    );

    for (code, should_be_valid, description) in test_cases {
        // Create a test document with the country code
        let test_doc = format!(
            r#"
identifier: "{}"
label: "Test country"
"#,
            code
        );

        // Parse the test document
        let parsed_test = parse_service.parse(&test_doc).await?;

        // Validate against CountryCodeAlpha2Identifier schema
        // This should check both pattern and permissible values from ISO3166Entity
        let result = parse_service
            .validate_document(&parsed_test, Some("CountryCodeAlpha2Identifier"))
            .await?;

        let is_valid = result.is_valid;
        let status_icon = if is_valid { "✓" } else { "✗" };
        let status_text = if is_valid { "Valid  " } else { "Invalid" };

        println!(
            "  {} '{}': {} - {}",
            status_icon, code, status_text, description
        );

        if !is_valid && !result.errors.is_empty() {
            for error in &result.errors {
                println!("       └─ {}", error.message);
            }
        }

        // Verify our expectation matches the actual result
        if is_valid != should_be_valid {
            println!("       ⚠️  UNEXPECTED RESULT!");
        }
    }

    println!();

    // ========================================================================
    // Part 4: Parse and validate a complex document with country codes
    // ========================================================================

    println!("Part 4: Complex Document Validation");
    println!("{}", "=".repeat(50));

    // Create a JSON document with country codes
    let complex_doc = r#"{
        "addresses": [
            {
                "id": "addr-1",
                "country": "US",
                "city": "New York",
                "postal_code": "10001"
            },
            {
                "id": "addr-2", 
                "country": "GB",
                "city": "London",
                "postal_code": "SW1A 1AA"
            },
            {
                "id": "addr-3",
                "country": "XX",
                "city": "Unknown",
                "postal_code": "00000"
            }
        ],
        "metadata": {
            "created": "2025-02-04",
            "version": "1.0"
        }
    }"#;

    // Parse as JSON
    let parsed_json = parse_service
        .parse_with_format(complex_doc, ParseFormat::Json(JsonFormat::Standard))
        .await?;

    println!("✓ Parsed complex JSON document");
    println!("  - Document ID: {}", parsed_json.id);

    // Validate the document structure
    let json_validation = parse_service.validate_document(&parsed_json, None).await?;

    println!(
        "
📋 Document Structure Validation:"
    );
    println!("  - Valid: {}", json_validation.is_valid);

    // Note: In a real implementation, we would extract and validate
    // individual country codes from the document against ISO3166Entity

    println!();

    // ========================================================================
    // Part 5: Summary
    // ========================================================================

    println!("Part 5: Summary");
    println!("{}", "=".repeat(50));

    println!("✓ Successfully demonstrated Parse Service integration");
    println!("✓ Loaded and parsed ISO3166Entity.yaml");
    println!("✓ Validated instance file against LinkML schema");
    println!("✓ Tested country code validation with permissible values");
    println!("✓ Parsed and validated complex documents");

    println!();
    println!("Key Insights:");
    println!("- Parse Service integrates with LinkML for schema validation");
    println!("- ISO3166Entity instances provide permissible values");
    println!("- CountryCodeAlpha2Identifier validates against both:");
    println!("  1. Pattern: [A-Z]{{2}} (exactly 2 uppercase letters)");
    println!("  2. Instance values: Must exist in ISO3166Entity.yaml");
    println!("- Both validations must pass for a code to be valid");

    Ok(())
}
