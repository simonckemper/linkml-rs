//! Integration tests for LinkML service
//!
//! These tests demonstrate real-world usage patterns and ensure the service
//! works correctly for complex scenarios.

mod mock_services;

use crate::factory::create_logger_service;
use linkml_service::{
    LinkMLServiceImpl, create_linkml_service,
    generator::{Generator, GeneratorOptions},
    parser::Parser,
    schema_view::SchemaView,
    validator::ValidationOptions,
};
use mock_services::*;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Biomedical research schema with complex relationships
const BIOMEDICAL_SCHEMA: &str = r#"
id: https://example.org/biomedical
name: BiomedicalResearch
description: Schema for biomedical research data

prefixes:
  biomedical: https://example.org/biomedical/
  linkml: https://w3id.org/linkml/

classes:
  Study:
    description: A research study
    attributes:
      study_id:
        identifier: true
        pattern: "^STUDY-[0-9]{6}$"
      title:
        required: true
      participants:
        range: Participant
        multivalued: true
      measurements:
        range: Measurement
        multivalued: true

  Participant:
    description: Study participant
    attributes:
      participant_id:
        identifier: true
        pattern: "^P[0-9]{5}$"
      age:
        range: integer
        minimum_value: 18
        maximum_value: 120
      condition:
        range: condition_enum

  Measurement:
    description: Clinical measurement
    attributes:
      measurement_id:
        identifier: true
      participant_id:
        range: string
      value:
        range: float
        minimum_value: 0
      unit:
        range: unit_enum
      timestamp:
        range: datetime

enums:
  condition_enum:
    permissible_values:
      healthy:
        description: Healthy control
      diabetes_type1:
        description: Type 1 diabetes
      diabetes_type2:
        description: Type 2 diabetes

  unit_enum:
    permissible_values:
      mg_dl:
        text: mg/dL
        description: Milligrams per deciliter
      mmol_l:
        text: mmol/L
        description: Millimoles per liter
"#;

/// Helper function to create test service
async fn create_test_service() -> Arc<
    LinkMLServiceImpl<
        MockTaskManagementService,
        MockErrorHandlerService,
        MockConfigurationService,
        MockDBMSService,
        MockTimeoutService,
    >,
> {
    let logger = Arc::new(MockMockLoggerService::new());
    let timestamp = Arc::new(MockTimestampService);
    let task_manager = Arc::new(MockTaskManagementService);
    let error_handler = Arc::new(MockErrorHandlerService);
    let config_service = Arc::new(MockConfigurationService::new());
    let dbms_service = Arc::new(MockDBMSService);
    let timeout_service = Arc::new(MockTimeoutService);
    let cache = Arc::new(MockCacheService::new());
    let monitor = Arc::new(MockMonitoringService::new());

    create_linkml_service(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        dbms_service,
        timeout_service,
        cache,
        monitor,
    )
    .await
    .expect("LinkML operation in test should succeed")
}

#[tokio::test]
async fn test_biomedical_research_workflow() {
    println!("=== Testing Biomedical Research Workflow ===");
    let service = create_test_service().await;
    let start = Instant::now();

    // Parse schema
    let parser = Parser::new();
    let schema = parser.parse(BIOMEDICAL_SCHEMA, "yaml").expect("LinkML operation in test should succeed");
    println!("✓ Schema parsed in {:?}", start.elapsed());

    // Create sample data
    let study_data = json!({
        "study_id": "STUDY-000001",
        "title": "Glucose Monitoring Study",
        "participants": [
            {
                "participant_id": "P00001",
                "age": 45,
                "condition": "diabetes_type2"
            },
            {
                "participant_id": "P00002",
                "age": 38,
                "condition": "healthy"
            }
        ],
        "measurements": [
            {
                "measurement_id": "M001",
                "participant_id": "P00001",
                "value": 126.5,
                "unit": "mg_dl",
                "timestamp": "2024-01-15T08:30:00Z"
            },
            {
                "measurement_id": "M002",
                "participant_id": "P00002",
                "value": 95.0,
                "unit": "mg_dl",
                "timestamp": "2024-01-15T08:45:00Z"
            }
        ]
    });

    // Validate data
    let validation_start = Instant::now();
    let engine = linkml_service::validator::ValidationEngine::new(&schema).expect("LinkML operation in test should succeed");
    let report = engine
        .validate_as_class(&study_data, "Study", None)
        .await
        .expect("LinkML operation in test should succeed");

    println!("✓ Validation completed in {:?}", validation_start.elapsed());
    assert!(report.valid, "Study data should be valid");
    println!("  - {} issues found", report.issues.len());

    // Generate code
    let gen_start = Instant::now();
    let generator = linkml_service::generator::python::PythonDataclassGenerator::new();
    let code = generator
        .generate(&schema, GeneratorOptions::default())
        .expect("LinkML operation in test should succeed");
    println!("✓ Python code generated in {:?}", gen_start.elapsed());
    assert!(code.contains("class Study"));
    assert!(code.contains("class Participant"));

    // Create SchemaView for introspection
    let view = SchemaView::new(&schema);
    let study_class = view.get_class("Study").expect("LinkML operation in test should succeed");
    assert_eq!(study_class.name, "Study");

    let slots = view.class_slots("Study").expect("LinkML operation in test should succeed");
    assert!(slots.iter().any(|s| s.name == "participants"));

    println!("✓ Total test time: {:?}", start.elapsed());
}

#[tokio::test]
async fn test_schema_validation_basic() {
    let service = create_test_service().await;

    // Create a simple schema
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
classes:
  Person:
    attributes:
      name:
        required: true
      age:
        range: integer
        minimum_value: 0
"#;

    // Parse schema
    let parser = Parser::new();
    let schema = parser.parse(schema_yaml, "yaml").expect("LinkML operation in test should succeed");

    // Valid data
    let valid_data = json!({
        "name": "John Doe",
        "age": 30
    });

    // Validate
    let engine = linkml_service::validator::ValidationEngine::new(&schema).expect("LinkML operation in test should succeed");
    let report = engine
        .validate_as_class(&valid_data, "Person", None)
        .await
        .expect("LinkML operation in test should succeed");

    assert!(report.valid);

    // Invalid data (missing required field)
    let invalid_data = json!({
        "age": 30
    });

    let report = engine
        .validate_as_class(&invalid_data, "Person", None)
        .await
        .expect("LinkML operation in test should succeed");

    assert!(!report.valid);
    assert!(report.issues.iter().any(|i| i.message.contains("name"));
}
