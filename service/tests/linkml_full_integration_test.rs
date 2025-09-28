//! Comprehensive integration tests for LinkML service
//!
//! This test suite demonstrates how different LinkML features work together
//! in real-world scenarios, including complex schemas, validation, code generation,
//! expression language, rules engine, and performance characteristics.

use linkml_service::{
    expression::{Evaluator, Parser as ExpressionParser},
    generator::{
        java::JavaGenerator,
        javascript::JavaScriptGenerator,
        json_ld::JsonLdGenerator,
        json_schema::JsonSchemaGenerator,
        protobuf::ProtobufGenerator,
        pydantic::PydanticGenerator,
        python_dataclass::PythonDataclassGenerator,
        rust_generator::RustGenerator,
        shacl::ShaclGenerator,
        traits::{Generator, GeneratorOptions},
        typescript::TypeScriptGenerator,
    },
    parser::yaml_parser::YamlParser,
    rule_engine::{RuleEngine, RuleExecutionStrategy},
    schema_view::SchemaView,
    transform::{
        inheritance_resolver::InheritanceResolver,
        schema_merger::{MergeStrategy, SchemaMerger},
    },
    validator::{
        engine::{ValidationEngine, ValidationOptions},
        report::ValidationReport,
    },
};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use tempfile::TempDir;
use linkml_core::types::SchemaDefinition;
use linkml_core::types::{ValidationReport};


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

#[tokio::test]
async fn test_biomedical_research_workflow() {
    println!("=== Testing Biomedical Research Workflow ===");

    let start = Instant::now();

    // Load the complex biomedical schema
    let parser = YamlParser::new();
    let schema = parser
        .parse(BIOMEDICAL_SCHEMA)
        .expect("Test operation failed");
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
    let mut participants = study_data["participants"]
        .as_array()
        .expect("Test operation failed")
        .clone();
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

    let mut complete_study = study_data
        .as_object()
        .expect("Test operation failed")
        .clone();
    complete_study["participants"] = json!(participants);

    // Validate the complete research study
    println!("
Validating research study data...");
    let validation_start = Instant::now();
    let engine = ValidationEngine::new(schema.clone());
    let options = ValidationOptions::default();
    let report = engine
        .validate(&json!(complete_study), "ResearchStudy", &options)
        .expect("Test operation failed");
    println!("Validation completed in {:?}", validation_start.elapsed());

    assert!(report.valid, "Study validation failed: {:?}", report.errors);
    println!("✓ Research study validation passed");

    // Test expression evaluation for lab results
    println!("
Evaluating expressions for lab results...");
    let lab_result = &complete_study["lab_results"][0];
    let expr_parser = ExpressionParser::new();
    let expr_ast = expr_parser
        .parse("value < 50 ? \"Low\" : value > 100 ? \"High\" : \"Normal\"")
        .expect("Test operation failed");
    let evaluator = Evaluator::new();
    let mut context = EvaluationContext::new();
    context.set_variable("value", lab_result["value"].clone());
    let expr_result = evaluator
        .evaluate(&expr_ast, &context)
        .expect("Test operation failed");
    println!("Lab result category: {}", expr_result);
    assert_eq!(expr_result.as_str().expect("Test operation failed"), "High");

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

    let minor_report = engine
        .validate(&minor_without_history, "Patient", &options)
        .expect("Test operation failed");
    assert!(
        !minor_report.valid,
        "Minor without medical history should fail validation"
    );
    println!("✓ Rule validation correctly enforced");

    // Generate code for multiple languages
    println!("
Generating code for biomedical schema...");
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Python dataclass generation
    let py_generator = PythonDataclassGenerator::new();
    let py_options = GeneratorOptions::default();
    let py_outputs = py_generator
        .generate(&schema, &py_options)
        .await
        .expect("Test operation failed");
    for output in py_outputs {
        let path = temp_dir.path().join(&output.filename);
        fs::write(&path, &output.content).expect("Test operation failed");
        assert!(path.exists());
    }
    println!("✓ Python dataclass generated");

    // TypeScript generation
    let ts_generator = TypeScriptGenerator::new();
    let ts_outputs = ts_generator
        .generate(&schema, &GeneratorOptions::default())
        .await
        .expect("Test operation failed");
    for output in ts_outputs {
        let path = temp_dir.path().join(&output.filename);
        fs::write(&path, &output.content).expect("Test operation failed");
        assert!(path.exists());
    }
    println!("✓ TypeScript interfaces generated");

    println!("
Biomedical workflow completed in {:?}", start.elapsed());
}

#[tokio::test]
async fn test_multi_tenant_config_validation() {
    println!("=== Testing Multi-Tenant Configuration Workflow ===");

    // Load configuration schema
    let parser = YamlParser::new();
    let schema = parser
        .parse(CONFIG_SCHEMA)
        .expect("Test operation failed");

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

    let engine = ValidationEngine::new(schema.clone());
    let options = ValidationOptions::default();
    let report = engine
        .validate(&valid_tenant, "TenantConfig", &options)
        .expect("Test operation failed");
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

    let invalid_report = engine
        .validate(&invalid_tenant, "TenantConfig", &options)
        .expect("Test operation failed");
    assert!(
        !invalid_report.valid,
        "Should fail - advanced analytics needs higher rate limits"
    );
    println!("✓ Rule validation correctly enforced rate limits");
}

#[tokio::test]
async fn test_schema_view_introspection() {
    println!("=== Testing SchemaView Introspection ===");

    // Load biomedical schema
    let parser = YamlParser::new();
    let schema = parser
        .parse(BIOMEDICAL_SCHEMA)
        .expect("Test operation failed");

    // Create SchemaView for introspection
    let view = SchemaView::new(schema.clone());

    // Test class hierarchy
    let patient_ancestors = view.class_ancestors("Patient");
    assert!(patient_ancestors.contains(&"NamedEntity".to_string());
    println!(
        "✓ Class hierarchy: Patient inherits from {:?}",
        patient_ancestors
    );

    // Test slot inheritance
    let patient_slots = view.induced_slots("Patient");
    assert!(patient_slots.is_some());
    let slots = patient_slots.expect("Test operation failed");
    assert!(slots.iter().any(|s| s.name == "id")); // Inherited from NamedEntity
    assert!(slots.iter().any(|s| s.name == "age")); // Direct slot
    println!(
        "✓ Slot inheritance: Patient has {} total slots",
        slots.len()
    );

    // Test schema statistics
    let stats = view.schema_statistics();
    println!("
Schema statistics:");
    println!("  - Classes: {}", stats.classes);
    println!("  - Slots: {}", stats.slots);
    println!("  - Types: {}", stats.types);
    println!("  - Enums: {}", stats.enums);
    assert!(stats.classes > 0);
    assert!(stats.enums > 0);
}

#[tokio::test]
async fn test_multi_schema_merge_validation() {
    println!("=== Testing Multi-Schema Merge and Validation ===");

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

    let parser = YamlParser::new();
    let base_schema = parser
        .parse(base_schema_str)
        .expect("Test operation failed");
    let ext_schema = parser
        .parse(extension_schema_str)
        .expect("Test operation failed");

    // Merge schemas
    let merger = SchemaMerger::new();
    let merge_options = MergeOptions {
        strategy: MergeStrategy::Override,
        conflict_resolution: ConflictResolution::UseOverlay,
        merge_imports: true,
    };
    let merged = merger
        .merge_with_options(base_schema, ext_schema, &merge_options)
        .expect("Test operation failed");

    // Validate data against merged schema
    let data = json!({
        "id": "ext_001",
        "created": "2025-01-16T10:00:00Z",
        "extra_field": "Extended data"
    });

    let engine = ValidationEngine::new(merged);
    let options = ValidationOptions::default();
    let report = engine
        .validate(&data, "ExtendedEntity", &options)
        .expect("Test operation failed");
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

    // Create simple schema for performance testing
    let schema_str = r#"
id: https://example.org/perf
name: PerformanceTest

classes:
  User:
    attributes:
      id:
        identifier: true
        required: true
      username:
        required: true
        pattern: "^[a-zA-Z0-9_]{3,20}$"
      email:
        required: true
        pattern: "^[^@]+@[^@]+\\.[^@]+$"
      age:
        range: integer
        minimum_value: 0
        maximum_value: 150
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(schema_str).expect("Test operation failed");
    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    // Generate large dataset
    let num_records = 1000;
    let mut users = Vec::new();

    for i in 0..num_records {
        users.push(json!({
            "id": format!("usr_{:05}", i),
            "username": format!("user_{:05}", i),
            "email": format!("user{}@example.com", i),
            "age": 25 + (i % 50)
        }));
    }

    // Measure validation performance
    let start = Instant::now();
    let mut valid_count = 0;

    for user in &users {
        let report = engine
            .validate(user, "User", &options)
            .expect("Test operation failed");
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
async fn test_code_generation_all_targets() {
    println!("=== Testing Code Generation for All Targets ===");

    let parser = YamlParser::new();
    let schema = parser
        .parse(CONFIG_SCHEMA)
        .expect("Test operation failed");
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Test all generators
    let generators: Vec<(Box<dyn Generator>, &str)> = vec![
        (
            Box::new(PythonDataclassGenerator::new()),
            "python_dataclass",
        ),
        (Box::new(PydanticGenerator::new()), "pydantic"),
        (Box::new(TypeScriptGenerator::new()), "typescript"),
        (Box::new(JavaScriptGenerator::new()), "javascript"),
        (Box::new(RustGenerator::new()), "rust"),
        (Box::new(JavaGenerator::new()), "java"),
        (Box::new(ProtobufGenerator::new()), "protobuf"),
        (Box::new(JsonSchemaGenerator::new()), "json_schema"),
        (Box::new(ShaclGenerator::new()), "shacl"),
        (Box::new(OwlRdfGenerator::new()), "owl_rdf"),
        (Box::new(JsonLdGenerator::new()), "json_ld"),
    ];

    let options = GeneratorOptions::default();

    for (generator, name) in generators {
        let start = Instant::now();
        let outputs = generator
            .generate(&schema, &options)
            .await
            .expect("Test operation failed");

        for output in outputs {
            let path = temp_dir.path().join(&output.filename);
            fs::write(&path, &output.content).expect("Test operation failed");
            assert!(path.exists());
            assert!(!output.content.is_empty());
        }

        println!("✓ {} generator completed in {:?}", name, start.elapsed());
    }
}

#[tokio::test]
async fn test_complex_inheritance_resolution() {
    println!("=== Testing Complex Inheritance Resolution ===");

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

    let parser = YamlParser::new();
    let mut schema = parser.parse(schema_str).expect("Test operation failed");

    // Resolve inheritance
    let resolver = InheritanceResolver::new();
    resolver
        .resolve(&mut schema)
        .expect("Test operation failed");

    let view = SchemaView::new(schema.clone());

    // Test inheritance chain
    let doc_slots = view
        .induced_slots("Document")
        .expect("Test operation failed");
    let slot_names: Vec<_> = doc_slots.iter().map(|s| s.name.as_str()).collect();

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

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();
    let report = engine
        .validate(&secure_doc, "SecureDocument", &options)
        .expect("Test operation failed");
    assert!(
        report.valid,
        "SecureDocument validation failed: {:?}",
        report.errors
    );
    println!("✓ Complex inheritance validation successful");
}

#[tokio::test]
async fn test_rule_engine_with_expressions() {
    println!("=== Testing Rule Engine with Complex Expressions ===");

    let parser = YamlParser::new();
    let schema = parser
        .parse(BIOMEDICAL_SCHEMA)
        .expect("Test operation failed");

    // Create rule engine
    let rule_engine = RuleEngine::new(schema.clone());

    // Test data that should trigger rules
    let lab_result_high = json!({
        "id": "LAB999999",
        "name": "Test Result",
        "created_date": "2025-01-16T10:00:00Z",
        "patient": {"id": "PAT000001"},
        "test_code": "LAB9999",
        "value": 150.0,  // High value
        "unit": "mg_dl",
        "reference_range": "10.0-100.0",
        "abnormal_flag": false  // Should be true according to rule
    });

    // Execute rules
    let mut context = HashMap::new();
    context.insert("data".to_string(), lab_result_high.clone());

    let result = rule_engine
        .execute_class_rules("LabResult", &context, &ExecutionStrategy::Sequential)
        .expect("Test operation failed");

    // The rule should have detected the violation
    assert!(
        !result.all_passed,
        "Rule should have failed for high value without abnormal flag"
    );
    println!("✓ Rule engine correctly detected violation");
}

#[tokio::test]
async fn test_end_to_end_api_modeling() {
    println!("=== Testing End-to-End API Modeling ===");

    // API schema with RESTful resource modeling
    let api_schema_str = r#"
id: https://example.org/api
name: APIDataModels
description: Data models for REST API

classes:
  Resource:
    abstract: true
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

  User:
    is_a: Resource
    attributes:
      username:
        range: string
        pattern: "^[a-zA-Z0-9_]{3,20}$"
        required: true
      email:
        range: string
        pattern: "^[^@]+@[^@]+\\.[^@]+$"
        required: true
      roles:
        range: role_enum
        multivalued: true

  Project:
    is_a: Resource
    attributes:
      name:
        range: string
        required: true
      owner:
        range: User
        required: true
      members:
        range: User
        multivalued: true
      status:
        range: project_status_enum
        required: true

enums:
  role_enum:
    permissible_values:
      admin:
      user:
      guest:

  project_status_enum:
    permissible_values:
      draft:
      active:
      archived:
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(api_schema_str)
        .expect("Test operation failed");
    let temp_dir = TempDir::new().expect("Test operation failed");

    // Generate OpenAPI spec
    let openapi_gen = JsonSchemaGenerator::new();
    let options = GeneratorOptions::default();
    let outputs = openapi_gen
        .generate(&schema, &options)
        .await
        .expect("Test operation failed");

    for output in outputs {
        let path = temp_dir.path().join(&output.filename);
        fs::write(&path, &output.content).expect("Test operation failed");

        // Verify JSON Schema is valid JSON
        let parsed: Value = serde_json::from_str(&output.content).expect("Test operation failed");
        assert!(parsed.is_object());
    }

    println!("✓ API modeling and generation complete");

    // Test validation of API data
    let user_data = json!({
        "id": "usr_123",
        "created_at": "2025-01-16T10:00:00Z",
        "updated_at": "2025-01-16T10:00:00Z",
        "username": "john_doe",
        "email": "john@example.com",
        "roles": ["user", "admin"]
    });

    let engine = ValidationEngine::new(schema);
    let validation_options = ValidationOptions::default();
    let report = engine
        .validate(&user_data, "User", &validation_options)
        .expect("Test operation failed");
    assert!(report.valid);
    println!("✓ API data validation passed");
}
