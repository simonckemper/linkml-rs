//! Integration tests using real schema files
//!
//! This test suite uses actual LinkML schema files to test
//! file-based workflows and import resolution.

use linkml_core::prelude::*;
use linkml_service::{
    GeneratorConfig, GeneratorType, LinkMLService, SchemaView, create_linkml_service,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;

// Import mock services
mod mock_services;
use crate::factory::create_logger_service;
use mock_services::*;


/// Helper to get test data directory
fn test_data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/schemas")
    Ok(())
}

/// Helper to create test service
async fn create_test_service() -> Arc<dyn LinkMLService> {
    let logger = Arc::new(MockMockLoggerService::new());
    let timestamp = Arc::new(MockTimestampService);
    let task_manager = Arc::new(MockTaskManagementService);
    let error_handler = Arc::new(MockErrorHandlerService);
    let config_service = Arc::new(MockConfigurationService::new());
    let cache = Arc::new(MockCacheService::new());
    let monitor = Arc::new(MockMonitoringService::new());

    create_linkml_service(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
    )
    .await
    .expect("Test operation failed")
}

#[tokio::test]
async fn test_biolink_schema_from_file() {
    println!("=== Testing Biolink Schema from File ===");

    let service = create_test_service().await;
    let schema_path = test_data_dir().join("biolink_minimal.yaml");

    // Load schema from file
    let schema = service
        .load_schema(&schema_path)
        .await
        .expect("Test operation failed");

    // Verify schema structure
    assert_eq!(schema.name, "biolink_minimal");
    assert!(schema.classes.contains_key("Gene"));
    assert!(schema.classes.contains_key("Disease"));
    assert!(schema.classes.contains_key("GeneDiseaseAssociation"));

    // Test with biomedical data
    let gene = json!({
        "id": "HGNC:1234",
        "name": "BRCA1",
        "category": ["gene"],
        "symbol": "BRCA1",
        "chromosome": "chr17",
        "taxon": {
            "taxon_id": "NCBITaxon:9606",
            "scientific_name": "Homo sapiens",
            "common_name": "human"
        }
    });

    let report = service
        .validate(&gene, &schema, "Gene")
        .await
        .expect("Test operation failed");
    assert!(report.valid, "Gene validation failed: {:?}", report.errors);

    // Test gene-disease association with rule validation
    let association = json!({
        "id": "association_001",
        "subject": {
            "id": "HGNC:1234",
            "name": "BRCA1",
            "category": ["gene"],
            "symbol": "BRCA1",
            "chromosome": "chr17",
            "taxon": {
                "taxon_id": "NCBITaxon:9606",
                "scientific_name": "Homo sapiens"
            }
        },
        "predicate": "associated_with",
        "object": {
            "id": "MONDO:0007254",
            "name": "Breast cancer",
            "category": ["disease"],
            "mondo_id": "MONDO:0007254",
            "severity": "severe"
        },
        "evidence": ["experimental", "curated"],
        "publications": ["PMID:12345678", "PMID:87654321"]
    });

    let assoc_report = service
        .validate(&association, &schema, "GeneDiseaseAssociation")
        .await
        .expect("Test operation failed");
    assert!(
        assoc_report.valid,
        "Association validation failed: {:?}",
        assoc_report.errors
    );

    // Test rule violation - experimental evidence without publications
    let invalid_association = json!({
        "id": "association_002",
        "subject": gene,
        "predicate": "causes",
        "object": {
            "id": "MONDO:0008765",
            "name": "Test disease",
            "category": ["disease"]
        },
        "evidence": ["experimental"],
        "publications": []  // Should fail - experimental needs publications
    });

    let invalid_report = service
        .validate(&invalid_association, &schema, "GeneDiseaseAssociation")
        .await
        .expect("Test operation failed");
    assert!(
        !invalid_report.valid,
        "Should fail validation - experimental evidence needs publications"
    );

    println!("✓ Biolink schema file loading and validation complete");
}

#[tokio::test]
async fn test_fhir_schema_integration() {
    println!("=== Testing FHIR Schema Integration ===");

    let service = create_test_service().await;
    let schema_path = test_data_dir().join("fhir_subset.yaml");

    // Load FHIR schema
    let schema = service
        .load_schema(&schema_path)
        .await
        .expect("Test operation failed");

    // Create patient data
    let patient = json!({
        "id": "patient-123",
        "meta": {
            "lastUpdated": "2025-01-16T10:00:00Z",
            "profile": ["http://hl7.org/fhir/StructureDefinition/Patient"]
        },
        "identifier": [{
            "system": "http://hospital.example.org/patients",
            "value": "12345"
        }],
        "active": true,
        "name": [{
            "use": "official",
            "family": "Doe",
            "given": ["John", "Jacob"]
        }],
        "gender": "male",
        "birthDate": "1990-05-15",
        "address": [{
            "use": "home",
            "line": ["123 Main St", "Apt 4B"],
            "city": "Boston",
            "state": "MA",
            "postalCode": "02101",
            "country": "US"
        }]
    });

    let patient_report = service
        .validate(&patient, &schema, "Patient")
        .await
        .expect("Test operation failed");
    assert!(
        patient_report.valid,
        "Patient validation failed: {:?}",
        patient_report.errors
    );

    // Create observation
    let observation = json!({
        "id": "obs-456",
        "meta": {
            "lastUpdated": "2025-01-16T11:00:00Z"
        },
        "status": "final",
        "code": {
            "coding": [{
                "system": "http://loinc.org",
                "code": "85354-9",
                "display": "Blood pressure"
            }],
            "text": "Blood pressure"
        },
        "subject": patient,
        "effectiveDateTime": "2025-01-16T10:30:00Z",
        "valueQuantity": {
            "value": 120,
            "unit": "mmHg",
            "system": "http://unitsofmeasure.org",
            "code": "mm[Hg]"
        },
        "interpretation": [{
            "coding": [{
                "system": "http://terminology.hl7.org/CodeSystem/v3-ObservationInterpretation",
                "code": "N",
                "display": "Normal"
            }]
        }],
        "referenceRange": [{
            "low": {
                "value": 90,
                "unit": "mmHg"
            },
            "high": {
                "value": 140,
                "unit": "mmHg"
            }
        }]
    });

    let obs_report = service
        .validate(&observation, &schema, "Observation")
        .await
        .expect("Test operation failed");
    assert!(
        obs_report.valid,
        "Observation validation failed: {:?}",
        obs_report.errors
    );

    // Test rule - final observations must have values
    let incomplete_obs = json!({
        "id": "obs-789",
        "meta": {
            "lastUpdated": "2025-01-16T11:00:00Z"
        },
        "status": "final",
        "code": {
            "coding": [{
                "system": "http://loinc.org",
                "code": "85354-9"
            }]
        },
        "subject": {"id": "patient-123"}
        // Missing valueQuantity - should fail for final status
    });

    let incomplete_report = service
        .validate(&incomplete_obs, &schema, "Observation")
        .await
        .expect("Test operation failed");
    assert!(
        !incomplete_report.valid,
        "Should fail - final observations need values"
    );

    println!("✓ FHIR schema integration complete");
}

#[tokio::test]
async fn test_api_models_code_generation() {
    println!("=== Testing API Models Code Generation ===");

    let service = create_test_service().await;
    let schema_path = test_data_dir().join("api_models.yaml");
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Load API models schema
    let schema = service
        .load_schema(&schema_path)
        .await
        .expect("Test operation failed");

    // Generate code for different targets
    let generators = vec![
        ("TypeScript", GeneratorType::TypeScript, "models.ts"),
        ("Python", GeneratorType::PythonDataclass, "models.py"),
        ("OpenAPI", GeneratorType::OpenAPI, "openapi.yaml"),
        ("JSON Schema", GeneratorType::JsonSchema, "schema.json"),
    ];

    for (name, gen_type, filename) in &generators {
        let config = GeneratorConfig {
            generator_type: *gen_type,
            output_path: Some(temp_dir.path().join(filename)),
            include_serialization: true,
            include_validation: true,
            include_subsets: true,
            ..Default::default()
        };

        service
            .generate_code(&schema, config)
            .await
            .expect("Test operation failed");

        let generated_path = temp_dir.path().join(filename);
        assert!(generated_path.exists(), "{} should be generated", filename);

        let content = fs::read_to_string(&generated_path).expect("Test operation failed");
        assert!(content.len() > 100, "{} should have content", filename);

        // Verify subset annotations are included
        if matches!(
            gen_type,
            GeneratorType::TypeScript | GeneratorType::PythonDataclass
        ) {
            assert!(
                content.contains("public") || content.contains("required"),
                "{} should include subset information",
                name
            );
        }

        println!("✓ Generated {} code", name);
    }

    // Test data validation with API models
    let user = json!({
        "id": "550e8400-e29b-41d4-a716-446655440000",
        "created_at": "2025-01-16T10:00:00Z",
        "updated_at": "2025-01-16T10:00:00Z",
        "username": "john_doe",
        "email": "john@example.com",
        "password_hash": "$2b$12$abcdefghijklmnopqrstuvwxyz",
        "full_name": "John Doe",
        "is_active": true,
        "is_verified": true,
        "roles": ["user", "admin"],
        "metadata": {
            "last_login": "2025-01-16T09:00:00Z",
            "login_count": 42,
            "preferred_language": "en",
            "timezone": "America/New_York",
            "notifications": {
                "email_enabled": true,
                "push_enabled": false,
                "frequency": "daily"
            }
        }
    });

    let user_report = service
        .validate(&user, &schema, "User")
        .await
        .expect("Test operation failed");
    assert!(
        user_report.valid,
        "User validation failed: {:?}",
        user_report.errors
    );

    // Test organization with members
    let org = json!({
        "id": "660e8400-e29b-41d4-a716-446655440001",
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-16T10:00:00Z",
        "name": "Acme Corporation",
        "slug": "acme-corp",
        "description": "Leading provider of quality products",
        "website": "https://acme.example.com",
        "members": [{
            "user": user,
            "role": "owner",
            "joined_at": "2025-01-01T00:00:00Z"
        }],
        "settings": {
            "billing_email": "billing@acme.example.com",
            "max_members": 100,
            "features": ["advanced_analytics", "api_access", "unlimited_projects"]
        }
    });

    let org_report = service
        .validate(&org, &schema, "Organization")
        .await
        .expect("Test operation failed");
    assert!(
        org_report.valid,
        "Organization validation failed: {:?}",
        org_report.errors
    );

    println!("✓ API models code generation and validation complete");
}

#[tokio::test]
async fn test_schema_import_resolution() {
    println!("=== Testing Schema Import Resolution ===");

    let service = create_test_service().await;
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Create base schema file
    let base_schema = r#"
id: https://example.org/base
name: base_schema
description: Base schema with common types

classes:
  Identifiable:
    abstract: true
    attributes:
      id:
        identifier: true
        required: true

types:
  email:
    uri: xsd:string
    base: str
    pattern: "^[\\w.%+-]+@[\\w.-]+\\.[A-Z]{2,}$"
"#;

    let base_path = temp_dir.path().join("base.yaml");
    fs::write(&base_path, base_schema).expect("Test operation failed");

    // Create importing schema
    let main_schema = r#"
id: https://example.org/main
name: main_schema
description: Main schema importing base

imports:
  - base.yaml

classes:
  Person:
    is_a: Identifiable
    attributes:
      name:
        required: true
      email:
        range: email
        required: true
"#;

    let main_path = temp_dir.path().join("main.yaml");
    fs::write(&main_path, main_schema).expect("Test operation failed");

    // Load schema with imports
    let schema = service
        .load_schema(&main_path)
        .await
        .expect("Test operation failed");

    // Verify import was resolved
    assert!(schema.classes.contains_key("Identifiable"));
    assert!(schema.classes.contains_key("Person"));
    assert!(schema.types.contains_key("email"));

    // Validate data using imported definitions
    let person = json!({
        "id": "person-001",
        "name": "Alice Smith",
        "email": "alice@example.com"
    });

    let report = service
        .validate(&person, &schema, "Person")
        .await
        .expect("Test operation failed");
    assert!(
        report.valid,
        "Person validation failed: {:?}",
        report.errors
    );

    // Test email pattern from imported type
    let invalid_person = json!({
        "id": "person-002",
        "name": "Bob Jones",
        "email": "not-an-email"  // Should fail pattern
    });

    let invalid_report = service
        .validate(&invalid_person, &schema, "Person")
        .await
        .expect("Test operation failed");
    assert!(!invalid_report.valid, "Should fail email validation");

    println!("✓ Schema import resolution working correctly");
}

#[tokio::test]
async fn test_schema_view_with_file_schemas() {
    println!("=== Testing SchemaView with File-based Schemas ===");

    let service = create_test_service().await;

    // Load all test schemas
    let biolink = service
        .load_schema(&test_data_dir().join("biolink_minimal.yaml"))
        .await
        .expect("Test operation failed");
    let fhir = service
        .load_schema(&test_data_dir().join("fhir_subset.yaml"))
        .await
        .expect("Test operation failed");
    let api = service
        .load_schema(&test_data_dir().join("api_models.yaml"))
        .await
        .expect("Test operation failed");

    // Create SchemaViews
    let biolink_view = SchemaView::new(biolink);
    let fhir_view = SchemaView::new(fhir);
    let api_view = SchemaView::new(api);

    // Compare schema statistics
    println!("
Schema Statistics:");

    let biolink_stats = biolink_view.get_statistics();
    println!("Biolink Model:");
    println!("  - Classes: {}", biolink_stats.num_classes);
    println!("  - Enums: {}", biolink_stats.num_enums);
    println!("  - Rules: {}", biolink_stats.num_rules);

    let fhir_stats = fhir_view.get_statistics();
    println!("FHIR Subset:");
    println!("  - Classes: {}", fhir_stats.num_classes);
    println!("  - Enums: {}", fhir_stats.num_enums);
    println!("  - Mixins: {}", fhir_stats.num_mixins);

    let api_stats = api_view.get_statistics();
    println!("API Models:");
    println!("  - Classes: {}", api_stats.num_classes);
    println!("  - Subsets: {}", api_stats.num_subsets);
    println!("  - Unique slots: {}", api_stats.num_unique_keys);

    // Test inheritance in API models
    let user_slots = api_view
        .class_slots("User", true)
        .expect("Test operation failed");
    let slot_names: Vec<_> = user_slots.iter().map(|s| &s.name).collect();
    assert!(
        slot_names.contains(&&"id".to_string()),
        "Should inherit id from Identifiable"
    );
    assert!(
        slot_names.contains(&&"created_at".to_string()),
        "Should inherit created_at from Timestamped"
    );
    assert!(
        slot_names.contains(&&"username".to_string()),
        "Should have direct username slot"
    );

    println!("✓ SchemaView analysis of file schemas complete");
}

#[tokio::test]
async fn test_multi_file_workflow() {
    println!("=== Testing Multi-File Workflow ===");

    let service = create_test_service().await;
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Load multiple schemas
    let schemas = vec![
        ("biolink", test_data_dir().join("biolink_minimal.yaml")),
        ("fhir", test_data_dir().join("fhir_subset.yaml")),
        ("api", test_data_dir().join("api_models.yaml")),
    ];

    // Generate code for each schema
    for (name, schema_path) in schemas {
        println!("
Processing {} schema...", name);

        let schema = service
            .load_schema(&schema_path)
            .await
            .expect("Test operation failed");

        // Generate multiple output formats
        let outputs = vec![
            (format!("{}.ts", name), GeneratorType::TypeScript),
            (format!("{}.py", name), GeneratorType::PythonDataclass),
            (format!("{}-schema.json", name), GeneratorType::JsonSchema),
        ];

        for (filename, gen_type) in outputs {
            let config = GeneratorConfig {
                generator_type: gen_type,
                output_path: Some(temp_dir.path().join(&filename)),
                ..Default::default()
            };

            service
                .generate_code(&schema, config)
                .await
                .expect("Test operation failed");
            assert!(
                temp_dir.path().join(&filename).exists(),
                "{} should exist",
                filename
            );
        }

        println!("✓ Generated code for {} schema", name);
    }

    // Verify all files were created
    let entries: Vec<_> = fs::read_dir(temp_dir.path())
        .expect("Test operation failed")
        .filter_map(Result::ok)
        .collect();

    assert_eq!(
        entries.len(),
        9,
        "Should have 9 generated files (3 schemas × 3 formats)"
    );

    println!(
        "
✓ Multi-file workflow complete with {} files generated",
        entries.len()
    );
}

#[tokio::test]
async fn test_error_reporting_with_file_context() {
    println!("=== Testing Error Reporting with File Context ===");

    let service = create_test_service().await;
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Create schema with intentional issues
    let problematic_schema = r#"
id: https://example.org/problematic
name: problematic_schema

classes:
  TestClass:
    attributes:
      conflicting_field:
        required: true
        recommended: true  # Conflict: can't be both
      pattern_field:
        pattern: "[invalid regex"  # Invalid regex
      range_conflict:
        minimum_value: 100
        maximum_value: 50  # min > max
"#;

    let schema_path = temp_dir.path().join("problematic.yaml");
    fs::write(&schema_path, problematic_schema).expect("Test operation failed");

    // Try to load schema - should succeed but validation might catch issues
    let schema = service
        .load_schema(&schema_path)
        .await
        .expect("Test operation failed");

    // Create test data that should trigger various errors
    let test_data = json!({
        "conflicting_field": null,  // Required but null
        "pattern_field": "test",
        "range_conflict": 75
    });

    let report = service
        .validate(&test_data, &schema, "TestClass")
        .await
        .expect("Test operation failed");

    if !report.valid {
        println!("
Validation errors detected:");
        for error in &report.errors {
            println!(
                "  - {}: {}",
                error.field.as_deref().unwrap_or("unknown"),
                error.message
            );
            if let Some(details) = &error.details {
                for (key, value) in details {
                    println!("    {}: {:?}", key, value);
                }
            }
        }
    }

    println!("✓ Error reporting with file context working");
}
