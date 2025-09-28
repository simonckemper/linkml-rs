//! Comprehensive integration tests for LinkML service
//!
//! This test suite demonstrates how different LinkML features work together
//! in real-world scenarios, including complex schemas, validation, code generation,
//! expression language, rules engine, and performance characteristics.

mod mock_services;

use crate::factory::create_logger_service;
use linkml_core::prelude::*;
use linkml_service::{
    // TODO: Fix generator API - generator::{Generator, GeneratorOptions},
    schema_view::SchemaView,
};
use mock_services::*;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;

/// Test data representing a biomedical research schema with complex validation rules
const BIOMEDICAL_SCHEMA: &str = r#"
id: https://example.org/biomedical
name: BiomedicalResearch
description: Schema for biomedical research data with complex validation
version: 1.0.0

default_prefix: biomed
default_range: string

prefixes:
  biomed: https://example.org/biomedical/
  linkml: https://w3id.org/linkml/
  xsd: http://www.w3.org/2001/XMLSchema#

imports:
  - linkml:types

subsets:
  required_fields:
    description: Fields required for all submissions
  clinical_data:
    description: Clinical research data fields

types:
  concentration_value:
    uri: xsd:float
    description: Concentration in mg/dL
    base: float
    minimum_value: 0.0
    maximum_value: 1000.0

classes:
  NamedEntity:
    abstract: true
    description: Base class for named entities
    attributes:
      id:
        identifier: true
        required: true
        pattern: "^[A-Z]{3}[0-9]{6}$"
        description: Unique identifier with specific format
      name:
        required: true
        in_subset:
          - required_fields
      created_date:
        range: datetime
        required: true
        ifabsent: 'datetime(now)'

  Patient:
    is_a: NamedEntity
    description: Patient information with PHI protections
    attributes:
      age:
        range: integer
        minimum_value: 0
        maximum_value: 150
        required: true
      gender:
        range: gender_enum
        required: true
      blood_type:
        range: blood_type_enum
      medical_history:
        range: MedicalCondition
        multivalued: true
        inlined: true
    rules:
      - preconditions:
          slot_conditions:
            age:
              less_than: 18
        postconditions:
          slot_conditions:
            medical_history:
              required: true
        description: Minors must have medical history documented

  MedicalCondition:
    description: Medical condition with temporal information
    attributes:
      condition_code:
        range: string
        pattern: "^ICD10:[A-Z][0-9]{2}(\\.[0-9]{1,2})?$"
        required: true
      severity:
        range: severity_enum
        required: true
      onset_date:
        range: date
        required: true
      notes:
        range: string
        recommended: true

  LabResult:
    is_a: NamedEntity
    description: Laboratory test results
    attributes:
      patient:
        range: Patient
        required: true
      test_code:
        range: string
        pattern: "^LAB[0-9]{4}$"
        required: true
      value:
        range: concentration_value
        required: true
      unit:
        range: unit_enum
        required: true
      reference_range:
        range: string
        pattern: "^[0-9.]+-[0-9.]+$"
      abnormal_flag:
        range: boolean
        ifabsent: 'false'
    rules:
      - preconditions:
          slot_conditions:
            value:
              greater_than: 100
        postconditions:
          slot_conditions:
            abnormal_flag:
              equals: true
        description: High values must be flagged as abnormal
    expressions:
      - 'result_category = value < 50 ? "Low" : value > 100 ? "High" : "Normal"'

  ResearchStudy:
    is_a: NamedEntity
    description: Clinical research study
    attributes:
      protocol_number:
        range: string
        pattern: "^PROTO-[0-9]{4}-[A-Z]{2}$"
        required: true
      principal_investigator:
        range: string
        required: true
      participants:
        range: Patient
        multivalued: true
        minimum_cardinality: 10
        maximum_cardinality: 1000
      lab_results:
        range: LabResult
        multivalued: true
        inlined_as_list: true
      status:
        range: study_status_enum
        required: true
    unique_keys:
      protocol_key:
        unique_key_slots:
          - protocol_number
    rules:
      - preconditions:
          slot_conditions:
            status:
              equals: "active"
        postconditions:
          slot_conditions:
            participants:
              minimum_cardinality: 10
        description: Active studies must have minimum participants

enums:
  gender_enum:
    permissible_values:
      male:
        description: Male gender
      female:
        description: Female gender
      other:
        description: Other gender

  blood_type_enum:
    permissible_values:
      A_positive:
        text: A+
      A_negative:
        text: A-
      B_positive:
        text: B+
      B_negative:
        text: B-
      O_positive:
        text: O+
      O_negative:
        text: O-
      AB_positive:
        text: AB+
      AB_negative:
        text: AB-

  severity_enum:
    permissible_values:
      mild:
        description: Mild severity
      moderate:
        description: Moderate severity
      severe:
        description: Severe severity
      critical:
        description: Critical severity

  unit_enum:
    permissible_values:
      mg_dl:
        text: mg/dL
        description: Milligrams per deciliter
      mmol_l:
        text: mmol/L
        description: Millimoles per liter

  study_status_enum:
    permissible_values:
      planning:
        description: Study in planning phase
      active:
        description: Study actively recruiting
      completed:
        description: Study completed
      suspended:
        description: Study suspended
"#;

/// Configuration schema for multi-tenant application
const CONFIG_SCHEMA: &str = r#"
id: https://example.org/config
name: ApplicationConfig
description: Multi-tenant application configuration with validation rules

prefixes:
  config: https://example.org/config/
  linkml: https://w3id.org/linkml/

classes:
  TenantConfig:
    description: Configuration for a single tenant
    attributes:
      tenant_id:
        identifier: true
        pattern: "^[a-z][a-z0-9-]{2,31}$"
        description: Lowercase alphanumeric with hyphens
      display_name:
        required: true
      features:
        range: FeatureConfig
        inlined: true
      api_keys:
        range: ApiKeyConfig
        multivalued: true
        inlined_as_list: true
      rate_limits:
        range: RateLimitConfig
        inlined: true
    rules:
      - preconditions:
          slot_conditions:
            features:
              slot_conditions:
                advanced_analytics:
                  equals: true
        postconditions:
          slot_conditions:
            rate_limits:
              slot_conditions:
                requests_per_minute:
                  greater_than_or_equals: 1000
        description: Advanced analytics requires higher rate limits

  FeatureConfig:
    description: Feature flags for tenant
    attributes:
      basic_auth:
        range: boolean
        ifabsent: 'true'
      oauth2:
        range: boolean
        ifabsent: 'false'
      advanced_analytics:
        range: boolean
        ifabsent: 'false'
      custom_branding:
        range: boolean
        ifabsent: 'true'
      api_access:
        range: boolean
        ifabsent: 'true'

  ApiKeyConfig:
    description: API key configuration
    attributes:
      key_id:
        identifier: true
        pattern: "^ak_[a-zA-Z0-9]{32}$"
      name:
        required: true
      scopes:
        range: string
        multivalued: true
        pattern: "^(read|write|admin):[a-z]+$"
      expires_at:
        range: datetime
        required: true
      rate_limit_override:
        range: integer
        minimum_value: 0
        maximum_value: 10000
    expressions:
      - 'is_expired = expires_at < datetime(now)'
      - 'is_admin = scopes.some(s => s.startsWith("admin:"))'

  RateLimitConfig:
    description: Rate limiting configuration
    attributes:
      requests_per_minute:
        range: integer
        minimum_value: 10
        maximum_value: 10000
        ifabsent: '100'
      requests_per_hour:
        range: integer
        minimum_value: 100
        maximum_value: 100000
        ifabsent: '5000'
      burst_size:
        range: integer
        minimum_value: 1
        maximum_value: 1000
        ifabsent: '20'
    rules:
      - preconditions:
          slot_conditions:
            requests_per_minute:
              greater_than: 60
        postconditions:
          slot_conditions:
            requests_per_hour:
              greater_than_or_equals: 3600
        description: Hourly limit must be at least 60x minute limit
"#;

/// API data model schema with code generation targets
const API_SCHEMA: &str = r#"
id: https://example.org/api
name: APIDataModels
description: Data models for REST API with multi-language code generation

prefixes:
  api: https://example.org/api/
  linkml: https://w3id.org/linkml/

classes:
  Resource:
    abstract: true
    description: Base resource for API
    attributes:
      id:
        identifier: true
        range: string
        required: true
      created_at:
        range: datetime
        required: true
        readonly: true
      updated_at:
        range: datetime
        required: true
        readonly: true
      version:
        range: integer
        minimum_value: 1
        ifabsent: '1'

  User:
    is_a: Resource
    description: User account
    attributes:
      username:
        range: string
        pattern: "^[a-zA-Z0-9_]{3,20}$"
        required: true
      email:
        range: string
        pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"
        required: true
      full_name:
        range: string
        required: true
      roles:
        range: role_enum
        multivalued: true
      preferences:
        range: UserPreferences
        inlined: true

  UserPreferences:
    description: User preferences
    attributes:
      theme:
        range: theme_enum
        ifabsent: 'light'
      language:
        range: string
        pattern: "^[a-z]{2}(-[A-Z]{2})?$"
        ifabsent: 'en'
      notifications_enabled:
        range: boolean
        ifabsent: 'true'

  Project:
    is_a: Resource
    description: Project resource
    attributes:
      name:
        range: string
        required: true
      description:
        range: string
        recommended: true
      owner:
        range: User
        required: true
      members:
        range: ProjectMember
        multivalued: true
        inlined_as_list: true
      status:
        range: project_status_enum
        required: true
      tags:
        range: string
        multivalued: true
        pattern: "^[a-z][a-z0-9-]*$"

  ProjectMember:
    description: Project membership
    attributes:
      user:
        range: User
        required: true
      role:
        range: project_role_enum
        required: true
      joined_at:
        range: datetime
        required: true

enums:
  role_enum:
    permissible_values:
      admin:
      user:
      guest:

  theme_enum:
    permissible_values:
      light:
      dark:
      auto:

  project_status_enum:
    permissible_values:
      draft:
      active:
      archived:
      deleted:

  project_role_enum:
    permissible_values:
      owner:
      maintainer:
      contributor:
      viewer:
"#;

// TODO: Fix mock service implementations - need proper trait implementations
// /// Helper function to create a test service with all dependencies
// async fn create_test_service() -> Arc<dyn LinkMLService> {
//     let logger = Arc::new(MockMockLoggerService::new());
//     let timestamp = Arc::new(MockTimestampService);
//     let task_manager = Arc::new(MockTaskManagementService);
//     let error_handler = Arc::new(MockErrorHandlerService);
//     let config_service = Arc::new(MockConfigurationService::new());
//     let dbms_service = Arc::new(MockDBMSService);
//     let timeout_service = Arc::new(MockTimeoutService);
//     let cache = Arc::new(MockCacheService::new());
//     let monitor = Arc::new(MockMonitoringService::new());
//
//     create_linkml_service(
//         logger,
//         timestamp,
//         task_manager,
//         error_handler,
//         config_service,
//         dbms_service,
//         timeout_service,
//         cache,
//         monitor,
//     )
//     .await
//     ?
// }

// TODO: Fix service factory API - requires DBMS and Timeout services
/*
#[tokio::test]
async fn test_biomedical_research_workflow() {
    println!("=== Testing Biomedical Research Workflow ===");

    let service = create_test_service().await;
    let start = Instant::now();

    // Load the complex biomedical schema
    let schema = service
        .load_schema_str(BIOMEDICAL_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;
    println!("Schema loaded in {:?}", start.elapsed());

    // Create test data representing a research study
    let study_data = json!({
        "id": "STU000001",
        "name": "COVID-19 Antibody Study",
        "created_date": "2025-01-16T10:00:00Z",
        "protocol_number": "PROTO-2025-CV",
        "principal_investigator": "Dr. Jane Smith",
        "status": "active",
        "participants": [
            {
                "id": "PAT000001",
                "name": "John Doe",
                "created_date": "2025-01-10T09:00:00Z",
                "age": 45,
                "gender": "male",
                "blood_type": "A_positive",
                "medical_history": [
                    {
                        "condition_code": "ICD10:I10",
                        "severity": "mild",
                        "onset_date": "2020-05-15",
                        "notes": "Well controlled with medication"
                    }
                ]
            },
            {
                "id": "PAT000002",
                "name": "Jane Smith",
                "created_date": "2025-01-11T10:00:00Z",
                "age": 16,  // Minor - should require medical history
                "gender": "female",
                "blood_type": "O_negative",
                "medical_history": [
                    {
                        "condition_code": "ICD10:J45.9",
                        "severity": "moderate",
                        "onset_date": "2018-03-20"
                    }
                ]
            }
        ],
        "lab_results": [
            {
                "id": "LAB000001",
                "name": "Antibody Test Result",
                "created_date": "2025-01-15T14:30:00Z",
                "patient": {"id": "PAT000001"},
                "test_code": "LAB1234",
                "value": 125.5,  // High value - should be flagged
                "unit": "mg_dl",
                "reference_range": "10.0-100.0",
                "abnormal_flag": true
            },
            {
                "id": "LAB000002",
                "name": "Antibody Test Result",
                "created_date": "2025-01-15T15:00:00Z",
                "patient": {"id": "PAT000002"},
                "test_code": "LAB1234",
                "value": 75.0,
                "unit": "mg_dl",
                "reference_range": "10.0-100.0",
                "abnormal_flag": false
            }
        ]
    });

    // Add more participants to meet minimum requirement
    let mut participants = study_data["participants"].as_array()?.clone();
    for i in 3..=10 {
        participants.push(json!({
            "id": format!("PAT{:06}", i),
            "name": format!("Patient {}", i),
            "created_date": "2025-01-12T10:00:00Z",
            "age": 30 + i,
            "gender": if i % 2 == 0 { "male" } else { "female" },
            "medical_history": []
        }));
    }

    let mut complete_study = study_data.as_object()?.clone();
    complete_study["participants"] = json!(participants);

    // Validate the complete research study
    println!("
Validating research study data...");
    let validation_start = Instant::now();
    let report = service
        .validate(&json!(complete_study), &schema, "ResearchStudy")
        .await
        ?;
    println!("Validation completed in {:?}", validation_start.elapsed());

    assert!(report.valid, "Study validation failed: {:?}", report.errors);
    println!("✓ Research study validation passed");

    // Test expression evaluation for lab results
    println!("
Evaluating expressions for lab results...");
    let lab_result = &complete_study["lab_results"][0];
    let expr_result = service
        .evaluate_expression(
            "value < 50 ? \"Low\" : value > 100 ? \"High\" : \"Normal\"",
            lab_result,
            &schema,
            Some("LabResult"),
        )
        .await
        ?;
    println!("Lab result category: {}", expr_result);
    assert_eq!(expr_result.as_str()?, "High");

    // Test rule validation for minors requiring medical history
    println!("
Testing rule validation for minors...");
    let minor_without_history = json!({
        "id": "PAT000003",
        "name": "Young Patient",
        "created_date": "2025-01-16T10:00:00Z",
        "age": 10,
        "gender": "male",
        "medical_history": []  // Should fail - minors need medical history
    });

    let minor_report = service
        .validate(&minor_without_history, &schema, "Patient")
        .await
        ?;
    assert!(
        !minor_report.valid,
        "Minor without medical history should fail validation"
    );
    println!("✓ Rule validation correctly enforced");

    // TODO: Fix generator API - Generate code for multiple languages
    println!("
Generating code for biomedical schema...");
    let temp_dir = TempDir::new()?;

    // TODO: Fix generator API
    /*
    // Python dataclass generation
    let py_config = GeneratorConfig {
        generator_type: GeneratorType::PythonDataclass,
        output_path: Some(temp_dir.path().join("biomedical.py")),
        package_name: Some("biomedical".to_string()),
        ..Default::default()
    };
    service.generate_code(&schema, py_config).await?;
    assert!(temp_dir.path().join("biomedical.py").exists());
    println!("✓ Python dataclass generated");

    // TypeScript generation
    let ts_config = GeneratorConfig {
        generator_type: GeneratorType::TypeScript,
        output_path: Some(temp_dir.path().join("biomedical.ts")),
        ..Default::default()
    };
    service.generate_code(&schema, ts_config).await?;
    assert!(temp_dir.path().join("biomedical.ts").exists());
    println!("✓ TypeScript interfaces generated");
    */

    println!("
Biomedical workflow completed in {:?}", start.elapsed());
}
*/

// TODO: Fix service factory API - requires DBMS and Timeout services
/*
#[tokio::test]
async fn test_multi_tenant_config_validation() {
    println!("=== Testing Multi-Tenant Configuration Workflow ===");

    let service = create_test_service().await;

    // Load configuration schema
    let schema = service
        .load_schema_str(CONFIG_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // Test valid tenant configuration
    let valid_tenant = json!({
        "tenant_id": "acme-corp",
        "display_name": "ACME Corporation",
        "features": {
            "basic_auth": true,
            "oauth2": true,
            "advanced_analytics": true,
            "custom_branding": true,
            "api_access": true
        },
        "api_keys": [
            {
                "key_id": "ak_1234567890abcdef1234567890abcdef",
                "name": "Production API Key",
                "scopes": ["read:users", "write:projects", "admin:billing"],
                "expires_at": "2025-12-31T23:59:59Z",
                "rate_limit_override": 5000
            },
            {
                "key_id": "ak_abcdef1234567890abcdef1234567890",
                "name": "Development API Key",
                "scopes": ["read:users", "read:projects"],
                "expires_at": "2025-06-30T23:59:59Z"
            }
        ],
        "rate_limits": {
            "requests_per_minute": 2000,  // High limit for advanced analytics
            "requests_per_hour": 120000,
            "burst_size": 100
        }
    });

    let report = service
        .validate(&valid_tenant, &schema, "TenantConfig")
        .await
        ?;
    assert!(
        report.valid,
        "Valid tenant config failed: {:?}",
        report.errors
    );
    println!("✓ Valid tenant configuration passed");

    // Test rule violation - advanced analytics without sufficient rate limits
    let invalid_tenant = json!({
        "tenant_id": "small-startup",
        "display_name": "Small Startup Inc",
        "features": {
            "advanced_analytics": true  // Requires high rate limits
        },
        "api_keys": [],
        "rate_limits": {
            "requests_per_minute": 100,  // Too low for advanced analytics
            "requests_per_hour": 6000,
            "burst_size": 20
        }
    });

    let invalid_report = service
        .validate(&invalid_tenant, &schema, "TenantConfig")
        .await
        ?;
    assert!(
        !invalid_report.valid,
        "Should fail - advanced analytics needs higher rate limits"
    );
    println!("✓ Rule validation correctly enforced rate limits");

    // Test expression evaluation for API keys
    let api_key = &valid_tenant["api_keys"][0];
    let is_admin = service
        .evaluate_expression(
            "scopes.some(s => s.startsWith(\"admin:\"))",
            api_key,
            &schema,
            Some("ApiKeyConfig"),
        )
        .await
        ?;
    assert_eq!(is_admin.as_bool()?, true);
    println!("✓ Expression evaluation for API key scopes");
}

#[tokio::test]
async fn test_api_code_generation_workflow() {
    println!("=== Testing API Code Generation Workflow ===");

    let service = create_test_service().await;
    let temp_dir = TempDir::new()?;

    // Load API schema
    let schema = service
        .load_schema_str(API_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // TODO: Fix generator API - Generate code for multiple targets
    /*
    let generators = vec![
        (GeneratorType::TypeScript, "api.ts"),
        (GeneratorType::PythonDataclass, "api.py"),
        (GeneratorType::Rust, "api.rs"),
        (GeneratorType::OpenAPI, "api-spec.yaml"),
        (GeneratorType::JsonSchema, "api-schema.json"),
    ];

    for (gen_type, filename) in generators {
        let config = GeneratorConfig {
            generator_type: gen_type,
            output_path: Some(temp_dir.path().join(filename)),
            package_name: Some("api_models".to_string()),
            include_serialization: true,
            include_validation: true,
            ..Default::default()
        };

        let start = Instant::now();
        service.generate_code(&schema, config).await?;
        println!("✓ Generated {} in {:?}", filename, start.elapsed());

        // Verify file exists and has content
        let file_path = temp_dir.path().join(filename);
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path)?;
        assert!(
            content.len() > 100,
            "Generated file should have substantial content"
        );
    }
    */

    // Test that generated TypeScript can handle the data model
    let user_data = json!({
        "id": "usr_123",
        "created_at": "2025-01-16T10:00:00Z",
        "updated_at": "2025-01-16T10:00:00Z",
        "version": 1,
        "username": "john_doe",
        "email": "john@example.com",
        "full_name": "John Doe",
        "roles": ["user", "admin"],
        "preferences": {
            "theme": "dark",
            "language": "en-US",
            "notifications_enabled": true
        }
    });

    let validation_report = service.validate(&user_data, &schema, "User").await?;
    assert!(validation_report.valid);
    println!("✓ API data model validation passed");
}

#[tokio::test]
async fn test_schema_view_introspection() {
    println!("=== Testing SchemaView Introspection ===");

    let service = create_test_service().await;

    // Load biomedical schema
    let schema = service
        .load_schema_str(BIOMEDICAL_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // Create SchemaView for introspection
    let view = SchemaView::new(schema.clone())?

    // Test class hierarchy
    let patient_ancestors = view.class_ancestors("Patient")?
    assert!(patient_ancestors.contains(&"NamedEntity".to_string());
    println!(
        "✓ Class hierarchy: Patient inherits from {:?}",
        patient_ancestors
    );

    // Test slot inheritance
    let patient_slots = view.class_slots("Patient")?
    assert!(patient_slots.contains(&"id".to_string())); // Inherited from NamedEntity
    assert!(patient_slots.contains(&"age".to_string())); // Direct slot
    println!(
        "✓ Slot inheritance: Patient has {} total slots",
        patient_slots.len()
    );

    // Test induced slots (with facets applied)
    let induced_id = view.induced_slot("id", "Patient")?
    assert_eq!(induced_id.identifier, Some(true));
    assert_eq!(induced_id.required, Some(true));
    println!("✓ Induced slot properties correctly computed");

    // Test schema statistics
    // TODO: get_statistics() not implemented yet
    let classes = view.all_classes()?;
    let slots = view.all_slots()?;
    let enums = view.all_enums()?;
    let types = view.all_types()?;
    println!("
Schema statistics:");
    println!("  - Classes: {}", classes.len());
    println!("  - Slots: {}", slots.len());
    println!("  - Types: {}", types.len());
    println!("  - Enums: {}", enums.len());
    assert!(classes.len() > 0);
    assert!(enums.len() > 0);
}

#[tokio::test]
async fn test_multi_schema_merge_validation() {
    println!("=== Testing Multi-Schema Merge and Validation ===");

    let service = create_test_service().await;

    // Create a base schema
    let base_schema_str = r#"
id: https://example.org/base
name: BaseSchema
prefixes:
  base: https://example.org/base/

classes:
  BaseEntity:
    attributes:
      id:
        identifier: true
        required: true
      created:
        range: datetime
        required: true
"#;

    // Create an extension schema
    let extension_schema_str = r#"
id: https://example.org/extension
name: ExtensionSchema
imports:
  - https://example.org/base

prefixes:
  ext: https://example.org/extension/

classes:
  ExtendedEntity:
    is_a: BaseEntity
    attributes:
      extra_field:
        range: string
        required: true
"#;

    let base_schema = service
        .load_schema_str(base_schema_str, SchemaFormat::Yaml)
        .await?;
    let ext_schema = service
        .load_schema_str(extension_schema_str, SchemaFormat::Yaml)
        .await?;

    // Merge schemas
    let merged = service
        .merge_schemas(vec![base_schema, ext_schema])
        .await?;

    // Validate data against merged schema
    let data = json!({
        "id": "ext_001",
        "created": "2025-01-16T10:00:00Z",
        "extra_field": "Extended data"
    });

    let report = service
        .validate(&data, &merged, "ExtendedEntity")
        .await?;
    assert!(
        report.valid,
        "Merged schema validation failed: {:?}",
        report.errors
    );
    println!("✓ Multi-schema merge and validation successful");
}

#[tokio::test]
async fn test_performance_large_dataset() {
    println!("=== Testing Performance with Large Dataset ===");

    let service = create_test_service().await;

    // Load schema
    let schema = service
        .load_schema_str(API_SCHEMA, SchemaFormat::Yaml)
        .await?;

    // Generate large dataset
    let num_records = 1000;
    let mut users = Vec::new();

    for i in 0..num_records {
        users.push(json!({
            "id": format!("usr_{:05}", i),
            "created_at": "2025-01-16T10:00:00Z",
            "updated_at": "2025-01-16T10:00:00Z",
            "version": 1,
            "username": format!("user_{:05}", i),
            "email": format!("user{}@example.com", i),
            "full_name": format!("User Number {}", i),
            "roles": ["user"],
            "preferences": {
                "theme": if i % 2 == 0 { "light" } else { "dark" },
                "language": "en",
                "notifications_enabled": i % 3 != 0
            }
        }));
    }

    // Measure validation performance
    let start = Instant::now();
    let mut valid_count = 0;

    for user in &users {
        let report = service.validate(user, &schema, "User").await?;
        if report.valid {
            valid_count += 1;
        }
    }

    let elapsed = start.elapsed();
    let per_record = elapsed / num_records as u32;

    println!("Performance results:");
    println!("  - Total records: {}", num_records);
    println!("  - Valid records: {}", valid_count);
    println!("  - Total time: {:?}", elapsed);
    println!("  - Per record: {:?}", per_record);
    println!(
        "  - Records/second: {:.0}",
        num_records as f64 / elapsed.as_secs_f64()
    );

    assert_eq!(valid_count, num_records);
    assert!(
        per_record < std::time::Duration::from_millis(10),
        "Validation should be fast"
    );
}

#[tokio::test]
async fn test_custom_validators_with_context() {
    println!("=== Testing Custom Validators with Context ===");

    let service = create_test_service().await;

    // Schema with custom validation logic
    let schema_str = r#"
id: https://example.org/custom
name: CustomValidation

classes:
  Order:
    attributes:
      order_id:
        identifier: true
        pattern: "^ORD-[0-9]{6}$"
      customer_email:
        required: true
        pattern: "^[^@]+@[^@]+$"
      items:
        range: OrderItem
        multivalued: true
        minimum_cardinality: 1
        inlined_as_list: true
      subtotal:
        range: float
        minimum_value: 0
      tax:
        range: float
        minimum_value: 0
      total:
        range: float
        minimum_value: 0
    rules:
      - preconditions:
          description: Check total calculation
        postconditions:
          description: Total must equal subtotal + tax
        # This would be implemented as custom validation logic

  OrderItem:
    attributes:
      sku:
        required: true
        pattern: "^SKU-[A-Z0-9]{6}$"
      quantity:
        range: integer
        minimum_value: 1
      price:
        range: float
        minimum_value: 0
      discount_percent:
        range: float
        minimum_value: 0
        maximum_value: 100
"#;

    let schema = service
        .load_schema_str(schema_str, SchemaFormat::Yaml)
        .await?;

    // Test order with correct totals
    let valid_order = json!({
        "order_id": "ORD-123456",
        "customer_email": "customer@example.com",
        "items": [
            {
                "sku": "SKU-ABC123",
                "quantity": 2,
                "price": 49.99,
                "discount_percent": 10
            },
            {
                "sku": "SKU-XYZ789",
                "quantity": 1,
                "price": 99.99,
                "discount_percent": 0
            }
        ],
        "subtotal": 189.97,  // (2 * 49.99 * 0.9) + 99.99
        "tax": 15.20,        // 8% tax
        "total": 205.17      // subtotal + tax
    });

    let report = service
        .validate(&valid_order, &schema, "Order")
        .await
        ?;
    assert!(report.valid, "Valid order failed: {:?}", report.errors);
    println!("✓ Custom validation with business logic passed");
}

#[tokio::test]
async fn test_schema_evolution_compatibility() {
    println!("=== Testing Schema Evolution and Compatibility ===");

    let service = create_test_service().await;

    // Version 1 of schema
    let schema_v1 = r#"
id: https://example.org/product/v1
name: ProductSchemaV1
version: 1.0.0

classes:
  Product:
    attributes:
      id:
        identifier: true
      name:
        required: true
      price:
        range: float
        required: true
"#;

    // Version 2 with backward compatible changes
    let schema_v2 = r#"
id: https://example.org/product/v2
name: ProductSchemaV2
version: 2.0.0

classes:
  Product:
    attributes:
      id:
        identifier: true
      name:
        required: true
      price:
        range: float
        required: true
      description:
        range: string
        recommended: true  # New optional field
      category:
        range: string
        ifabsent: 'general'  # New field with default
"#;

    let schema1 = service
        .load_schema_str(schema_v1, SchemaFormat::Yaml)
        .await
        ?;
    let schema2 = service
        .load_schema_str(schema_v2, SchemaFormat::Yaml)
        .await
        ?;

    // Data that's valid for v1
    let v1_data = json!({
        "id": "prod_001",
        "name": "Widget",
        "price": 19.99
    });

    // Validate v1 data against both schemas
    let v1_report1 = service
        .validate(&v1_data, &schema1, "Product")
        .await
        ?;
    let v1_report2 = service
        .validate(&v1_data, &schema2, "Product")
        .await
        ?;

    assert!(v1_report1.valid, "V1 data should be valid for V1 schema");
    assert!(
        v1_report2.valid,
        "V1 data should be valid for V2 schema (backward compatible)"
    );

    // V2 data with new fields
    let v2_data = json!({
        "id": "prod_002",
        "name": "Enhanced Widget",
        "price": 29.99,
        "description": "New and improved",
        "category": "electronics"
    });

    let v2_report = service
        .validate(&v2_data, &schema2, "Product")
        .await
        ?;
    assert!(v2_report.valid, "V2 data should be valid for V2 schema");

    println!("✓ Schema evolution maintains backward compatibility");
}

#[tokio::test]
async fn test_end_to_end_clinical_trial_system() {
    println!("=== Testing End-to-End Clinical Trial System ===");

    let service = create_test_service().await;
    let temp_dir = TempDir::new()?

    // Load biomedical schema
    let schema = service
        .load_schema_str(BIOMEDICAL_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // Simulate clinical trial workflow
    println!("
1. Creating new research study...");
    let study = json!({
        "id": "STU999999",
        "name": "Innovative Treatment Study",
        "created_date": "2025-01-16T08:00:00Z",
        "protocol_number": "PROTO-2025-IT",
        "principal_investigator": "Dr. Innovation",
        "status": "planning",
        "participants": [],
        "lab_results": []
    });

    let study_report = service
        .validate(&study, &schema, "ResearchStudy")
        .await
        ?;
    assert!(
        study_report.valid,
        "New study should be valid in planning phase"
    );

    println!("2. Transitioning to active recruitment...");
    let mut active_study = study.as_object()?.clone();
    active_study["status"] = json!("active");

    // Should fail - active studies need minimum participants
    let active_report = service
        .validate(&json!(active_study), &schema, "ResearchStudy")
        .await
        ?;
    assert!(
        !active_report.valid,
        "Active study without participants should fail"
    );

    println!("3. Adding participants...");
    let mut participants = Vec::new();
    for i in 1..=15 {
        participants.push(json!({
            "id": format!("PAT{:06}", 100000 + i),
            "name": format!("Participant {}", i),
            "created_date": "2025-01-16T09:00:00Z",
            "age": 25 + i * 2,
            "gender": if i % 2 == 0 { "male" } else { "female" },
            "blood_type": "O_positive",
            "medical_history": if i < 18 {
                vec![json!({
                    "condition_code": "ICD10:Z00.0",
                    "severity": "mild",
                    "onset_date": "2024-01-01"
                })]
            } else {
                vec![]
            }
        }));
    }
    active_study["participants"] = json!(participants);

    let populated_report = service
        .validate(&json!(active_study), &schema, "ResearchStudy")
        .await
        ?;
    assert!(
        populated_report.valid,
        "Active study with participants should be valid"
    );

    println!("4. Recording lab results...");
    let mut lab_results = Vec::new();
    for i in 1..=5 {
        let value = 50.0 + (i as f64 * 20.0);
        lab_results.push(json!({
            "id": format!("LAB{:06}", 100000 + i),
            "name": "Treatment Response Test",
            "created_date": "2025-01-16T14:00:00Z",
            "patient": {"id": format!("PAT{:06}", 100000 + i)},
            "test_code": format!("LAB{:04}", 5000 + i),
            "value": value,
            "unit": "mg_dl",
            "reference_range": "50.0-100.0",
            "abnormal_flag": value > 100.0
        }));
    }
    active_study["lab_results"] = json!(lab_results);

    let complete_report = service
        .validate(&json!(active_study), &schema, "ResearchStudy")
        .await
        ?;
    assert!(complete_report.valid, "Complete study should be valid");

    println!("5. Generating compliance reports...");

    // TODO: Fix generator API - Generate multiple output formats
    /*
    let outputs = vec![
        ("study_report.py", GeneratorType::PythonDataclass),
        ("study_models.ts", GeneratorType::TypeScript),
        ("study_api.json", GeneratorType::JsonSchema),
    ];

    for (filename, gen_type) in outputs {
        let config = GeneratorConfig {
            generator_type: gen_type,
            output_path: Some(temp_dir.path().join(filename)),
            include_serialization: true,
            include_validation: true,
            ..Default::default()
        };
        service.generate_code(&schema, config).await?;
        println!("  ✓ Generated {}", filename);
    }
    */

    // Create SchemaView for analysis
    let view = SchemaView::new(schema)?;
    // TODO: get_statistics() not implemented yet
    let classes = view.all_classes()?;

    println!("
6. Study Schema Analysis:");
    println!("  - Total classes: {}", classes.len());
    // Stats for rules and required fields would need to be computed manually

    println!("
✓ End-to-end clinical trial system validation complete!");
}

#[tokio::test]
async fn test_concurrent_validation_performance() {
    println!("=== Testing Concurrent Validation Performance ===");

    let service = create_test_service().await;
    let schema = service
        .load_schema_str(API_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // Create test data
    let num_batches = 10;
    let batch_size = 100;

    let mut batches = Vec::new();
    for batch in 0..num_batches {
        let mut users = Vec::new();
        for i in 0..batch_size {
            users.push(json!({
                "id": format!("usr_{}_{}", batch, i),
                "created_at": "2025-01-16T10:00:00Z",
                "updated_at": "2025-01-16T10:00:00Z",
                "version": 1,
                "username": format!("user_{}_{}", batch, i),
                "email": format!("user{}@batch{}.com", i, batch),
                "full_name": format!("User {} Batch {}", i, batch),
                "roles": ["user"],
                "preferences": {
                    "theme": "light",
                    "language": "en",
                    "notifications_enabled": true
                }
            }));
        }
        batches.push(users);
    }

    // Sequential validation
    println!("
Sequential validation...");
    let seq_start = Instant::now();
    let mut seq_valid = 0;

    for batch in &batches {
        for user in batch {
            let report = service.validate(user, &schema, "User").await?;
            if report.valid {
                seq_valid += 1;
            }
        }
    }
    let seq_duration = seq_start.elapsed();

    // Concurrent validation
    println!("Concurrent validation...");
    let conc_start = Instant::now();
    let mut handles = Vec::new();

    for batch in batches {
        let service_clone = service.clone();
        let schema_clone = schema.clone();

        let handle = tokio::spawn(async move {
            let mut valid = 0;
            for user in batch {
                let report = service_clone
                    .validate(&user, &schema_clone, "User")
                    .await
                    ?;
                if report.valid {
                    valid += 1;
                }
            }
            valid
        });
        handles.push(handle);
    }

    let mut conc_valid = 0;
    for handle in handles {
        conc_valid += handle.await?;
    }
    let conc_duration = conc_start.elapsed();

    println!("
Performance comparison:");
    println!("  - Sequential: {:?} ({} valid)", seq_duration, seq_valid);
    println!("  - Concurrent: {:?} ({} valid)", conc_duration, conc_valid);
    println!(
        "  - Speedup: {:.2}x",
        seq_duration.as_secs_f64() / conc_duration.as_secs_f64()
    );

    assert_eq!(seq_valid, conc_valid, "Validation results should match");
    assert!(conc_duration < seq_duration, "Concurrent should be faster");
}

#[tokio::test]
async fn test_complex_inheritance_resolution() {
    println!("=== Testing Complex Inheritance Resolution ===");

    let service = create_test_service().await;

    // Schema with diamond inheritance pattern
    let schema_str = r#"
id: https://example.org/inheritance
name: InheritanceTest

classes:
  Entity:
    abstract: true
    attributes:
      id:
        identifier: true
      created:
        range: datetime
        required: true

  Trackable:
    abstract: true
    attributes:
      last_modified:
        range: datetime
        required: true
      modified_by:
        range: string

  Versioned:
    abstract: true
    attributes:
      version:
        range: integer
        minimum_value: 1
        ifabsent: '1'
      version_notes:
        range: string

  Document:
    is_a: Entity
    mixins:
      - Trackable
      - Versioned
    attributes:
      title:
        required: true
      content:
        required: true
      published:
        range: boolean
        ifabsent: 'false'

  SecureDocument:
    is_a: Document
    attributes:
      access_level:
        range: access_level_enum
        required: true
      encryption_key_id:
        range: string
        pattern: "^key-[a-f0-9]{32}$"

enums:
  access_level_enum:
    permissible_values:
      public:
      internal:
      confidential:
      secret:
"#;

    let schema = service
        .load_schema_str(schema_str, SchemaFormat::Yaml)
        .await?;
    let view = SchemaView::new(schema.clone())?;

    // Test inheritance chain
    let doc_slots = view.class_slots("Document")?;
    let slot_names: Vec<_> = doc_slots.iter().map(|s| s.as_str()).collect();

    assert!(slot_names.contains(&"id"), "Should inherit id from Entity");
    assert!(
        slot_names.contains(&"created"),
        "Should inherit created from Entity"
    );
    assert!(
        slot_names.contains(&"last_modified"),
        "Should inherit from Trackable mixin"
    );
    assert!(
        slot_names.contains(&"version"),
        "Should inherit from Versioned mixin"
    );
    assert!(slot_names.contains(&"title"), "Should have direct slot");

    println!("✓ Document class has all inherited slots: {:?}", slot_names);

    // Test deep inheritance
    let secure_doc_slots = view.class_slots("SecureDocument")?;
    assert!(
        secure_doc_slots.len() > doc_slots.len(),
        "SecureDocument should have more slots"
    );

    // Validate instance with all inherited fields
    let secure_doc = json!({
        "id": "doc-001",
        "created": "2025-01-16T10:00:00Z",
        "last_modified": "2025-01-16T11:00:00Z",
        "modified_by": "admin",
        "version": 2,
        "version_notes": "Updated content",
        "title": "Confidential Report",
        "content": "Classified information...",
        "published": false,
        "access_level": "confidential",
        "encryption_key_id": "key-1234567890abcdef1234567890abcdef"
    });

    let report = service
        .validate(&secure_doc, &schema, "SecureDocument")
        .await
        ?;
    assert!(
        report.valid,
        "SecureDocument validation failed: {:?}",
        report.errors
    );
    println!("✓ Complex inheritance validation successful");
}
*/

// TODO: Fix LinkMLServiceConfig API
/*
/// Integration test demonstrating real-world configuration management
#[tokio::test]
async fn test_configuration_management_system() {
    println!("=== Testing Configuration Management System ===");

    // Create service with custom configuration
    let config = LinkMLServiceConfig {
        enable_caching: true,
        cache_ttl_seconds: 300,
        max_validation_errors: 100,
        enable_parallel_validation: true,
        expression_timeout_ms: 5000,
        ..Default::default()
    };

    let logger = Arc::new(MockMockLoggerService::new());
    let timestamp = Arc::new(MockTimestampService);
    let task_manager = Arc::new(MockTaskManagementService);
    let error_handler = Arc::new(MockErrorHandlerService);
    let config_service = Arc::new(MockConfigurationService::new());
    let cache = Arc::new(MockCacheService::new());
    let monitor = Arc::new(MockMonitoringService::new());

    let service = create_linkml_service_with_config(
        logger,
        timestamp,
        task_manager,
        error_handler,
        config_service,
        cache,
        monitor,
        config,
    )
    .await
    ?;

    // Load configuration schema
    let schema = service
        .load_schema_str(CONFIG_SCHEMA, SchemaFormat::Yaml)
        .await
        ?;

    // Test caching behavior
    let tenant1 = json!({
        "tenant_id": "test-tenant-1",
        "display_name": "Test Tenant 1",
        "features": {
            "api_access": true
        },
        "api_keys": [],
        "rate_limits": {}
    });

    // First validation should cache
    let start1 = Instant::now();
    let report1 = service
        .validate(&tenant1, &schema, "TenantConfig")
        .await
        ?;
    let duration1 = start1.elapsed();

    // Second validation should be faster due to cache
    let start2 = Instant::now();
    let report2 = service
        .validate(&tenant1, &schema, "TenantConfig")
        .await
        ?;
    let duration2 = start2.elapsed();

    assert!(report1.valid && report2.valid);
    println!(
        "First validation: {:?}, Second validation: {:?}",
        duration1, duration2
    );
    println!("✓ Configuration caching working correctly");
}
*/
