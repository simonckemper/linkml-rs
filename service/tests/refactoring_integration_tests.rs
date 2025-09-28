//! Integration tests for the refactored LinkML service
//!
//! These tests verify that all refactoring changes work together correctly:
//! - No unwrap() panics in production code
//! - Configuration loading and hot-reload
//! - File System Service integration
//! - Memory optimizations
//! - Feature completeness

use linkml_service::LinkMLService;
use linkml_service::config::{LinkMLConfig, load_config};
use linkml_service::expression::ExpressionEngine;
use linkml_service::file_system_adapter::FileSystemAdapter;
use linkml_service::generator::{GeneratorRegistry, GeneratorType};
use linkml_service::parser::{SchemaParser, yaml_parser::YamlParser};
use linkml_service::schema_view::SchemaView;
use linkml_service::validator::Validator;

use std::path::Path;
use std::error::Error as StdError;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{Duration, sleep};

/// Test that configuration loads correctly without panics
#[tokio::test]
async fn test_configuration_loading_no_panics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create test configuration
    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("test_config.yaml");

    std::fs::write(
        &config_path,
        r#"
typedb:
  server_address: "localhost:1729"
  default_database: "test_linkml"
  batch_size: 100
  connection_timeout: 30s
  max_retries: 3
  cache_size: 1000

validation:
  max_depth: 10
  max_errors: 100
  strict_mode: true
  cache_enabled: true
  cache_ttl: 300s
  parallel_validation: true
  parallel_threshold: 100

performance:
  string_interning_threshold: 50
  parallel_evaluation_threshold: 10
  cache_warming_enabled: true
  memory_limit_mb: 1024
  gc_interval: 60s
"#,
    )?;

    // Test loading without panics (no unwrap() calls)
    let config: LinkMLConfig = load_config(&config_path)?;

    // Verify configuration values
    assert_eq!(config.typedb.server_address, "localhost:1729");
    assert_eq!(config.typedb.default_database, "test_linkml");
    assert_eq!(config.validation.max_depth, 10);
    assert!(config.validation.parallel_validation);

    Ok(())
}
    Ok(())
}

/// Test configuration hot-reload functionality
#[tokio::test]
async fn test_configuration_hot_reload() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use linkml_service::config::{ConfigManager, LinkMLConfig}
    Ok(())
};
    use notify::{RecursiveMode, Watcher};
    use std::sync::mpsc::channel;

    let temp_dir = TempDir::new()?;
    let config_path = temp_dir.path().join("hot_reload_config.yaml");

    // Write initial configuration
    std::fs::write(
        &config_path,
        r#"
typedb:
  server_address: "localhost:1729"
  default_database: "linkml_initial"
  batch_size: 100
validation:
  max_depth: 5
"#,
    )?;

    // Create config manager with hot-reload
    let config_manager = ConfigManager::new_with_reload(config_path.clone())?;

    // Verify initial values
    {
        let config = config_manager.get_config();
        assert_eq!(config.typedb.default_database, "linkml_initial");
        assert_eq!(config.validation.max_depth, 5);
    }

    // Update configuration file
    std::fs::write(
        &config_path,
        r#"
typedb:
  server_address: "localhost:1729"
  default_database: "linkml_updated"
  batch_size: 200
validation:
  max_depth: 10
"#,
    )?;

    // Wait for hot-reload to trigger
    sleep(Duration::from_millis(100)).await;

    // Verify updated values
    {
        let config = config_manager.get_config();
        assert_eq!(config.typedb.default_database, "linkml_updated");
        assert_eq!(config.validation.max_depth, 10);
    }

    Ok(())
}

/// Test File System Service integration
#[tokio::test]
async fn test_file_system_service_integration() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let fs_adapter = FileSystemAdapter::new();

    // Test schema file operations
    let schema_path = temp_dir.path().join("test_schema.yaml");
    let schema_content = r#"
id: https://example.org/test
name: TestSchema
prefixes:
  ex: https://example.org/
classes:
  Person:
    attributes:
      name:
        range: string
        required: true
      age:
        range: integer
"#;

    // Write schema using File System Service
    fs_adapter
        .write_file(&schema_path, schema_content.as_bytes())
        .await?;

    // Read schema using File System Service
    let read_content = fs_adapter.read_file_to_string(&schema_path).await?;
    assert_eq!(read_content, schema_content);

    // Test directory operations
    let sub_dir = temp_dir.path().join("schemas");
    fs_adapter.create_dir_all(&sub_dir).await?;
    assert!(fs_adapter.exists(&sub_dir).await?);

    // Test listing files
    let schema2_path = sub_dir.join("schema2.yaml");
    fs_adapter.write_file(&schema2_path, b"# Schema 2").await?;

    let files = fs_adapter.list_files(&sub_dir).await?;
    assert_eq!(files.len(), 1);
    assert_eq!(
        files[0].file_name().expect("Test operation failed"),
        "schema2.yaml"
    );

    Ok(())
}
    Ok(())
}

/// Test that parser works without unwrap() panics
#[tokio::test]
async fn test_parser_no_unwrap_panics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let fs_adapter = Arc::new(FileSystemAdapter::new());

    // Test with valid schema
    let valid_schema = r#"
id: https://example.org/valid
name: ValidSchema
classes:
  ValidClass:
    attributes:
      valid_attr:
        range: string
"#;

    let valid_path = temp_dir.path().join("valid.yaml");
    fs_adapter
        .write_file(&valid_path, valid_schema.as_bytes())
        .await?;

    let parser = SchemaParser::new(fs_adapter.clone());
    let result = parser.parse_file(&valid_path).await;
    assert!(result.is_ok());

    // Test with invalid schema (should not panic)
    let invalid_schema = "not: valid: yaml: at: all:";
    let invalid_path = temp_dir.path().join("invalid.yaml");
    fs_adapter
        .write_file(&invalid_path, invalid_schema.as_bytes())
        .await?;

    let result = parser.parse_file(&invalid_path).await;
    assert!(result.is_err());
    // Verify it returns an error instead of panicking
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse YAML")
    );

    Ok(())
}
    Ok(())
}

/// Test generator operations without unwrap() panics
#[tokio::test]
async fn test_generators_no_unwrap_panics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let fs_adapter = Arc::new(FileSystemAdapter::new());

    // Create a simple schema
    let schema_content = r#"
id: https://example.org/generator_test
name: GeneratorTest
classes:
  TestClass:
    attributes:
      test_field:
        range: string
        required: true
"#;

    let schema_path = temp_dir.path().join("generator_test.yaml");
    fs_adapter
        .write_file(&schema_path, schema_content.as_bytes())
        .await?;

    // Parse schema
    let parser = SchemaParser::new(fs_adapter.clone());
    let schema = parser.parse_file(&schema_path).await?;

    // Test various generators
    let registry = GeneratorRegistry::new();

    // Test Python generator
    let python_gen = registry.get_generator(GeneratorType::Python)?;
    let python_result = python_gen.generate(&schema)?;
    assert!(python_result.contains("class TestClass"));

    // Test TypeScript generator
    let ts_gen = registry.get_generator(GeneratorType::TypeScript)?;
    let ts_result = ts_gen.generate(&schema)?;
    assert!(ts_result.contains("interface TestClass"));

    // Test SQL generator
    let sql_gen = registry.get_generator(GeneratorType::SQL)?;
    let sql_result = sql_gen.generate(&schema)?;
    assert!(sql_result.contains("CREATE TABLE"));

    // Test with invalid input (should not panic)
    let empty_schema = linkml_core::SchemaDefinition::new("empty");
    let result = python_gen.generate(&empty_schema);
    assert!(result.is_ok()); // Should handle empty schema gracefully

    Ok(())
}
    Ok(())
}

/// Test expression engine without unwrap() panics
#[tokio::test]
async fn test_expression_engine_no_panics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use linkml_core::SchemaDefinition;
    use serde_json::json;

    let mut schema = SchemaDefinition::new("expression_test");

    // Add class with expression
    let class_def = linkml_core::ClassDefinition {
        name: "ExpressionTest".to_string(),
        attributes: indexmap::indexmap! {
            "computed_field".to_string() => linkml_core::SlotDefinition {
                name: "computed_field".to_string(),
                equals_expression: Some("base_value * 2 + 10".to_string()),
                ..Default::default()
            }
    Ok(())
}
        },
        ..Default::default()
    };

    schema
        .classes
        .insert("ExpressionTest".to_string(), class_def);

    let engine = ExpressionEngine::new(&schema)?;

    // Test valid expression
    let data = json!({
        "base_value": 5
    });

    let result = engine.evaluate("base_value * 2 + 10", &data)?;
    assert_eq!(result.as_f64().expect("Test operation failed"), 20.0);

    // Test invalid expression (should not panic)
    let invalid_result = engine.evaluate("undefined_var + 1", &data);
    assert!(invalid_result.is_err());

    // Test divide by zero (should not panic)
    let div_zero_result = engine.evaluate("1 / 0", &data);
    assert!(div_zero_result.is_err());

    Ok(())
}

/// Test memory optimization with string interning
#[tokio::test]
async fn test_memory_optimization() -> std::result::Result<(), Box<dyn std::error::Error>> {
    use linkml_core::string_pool::get_pool_stats;

    // Create schema with repeated strings
    let schema_yaml = r#"
id: https://example.org/memory_test
name: MemoryTest
default_prefix: ex
imports:
  - linkml:types
prefixes:
  ex: https://example.org/
  linkml: https://w3id.org/linkml/
classes:
  Class1:
    is_a: NamedThing
    mixins: [HasId, HasName]
    slots: [id, name, status]
  Class2:
    is_a: NamedThing
    mixins: [HasId, HasName]
    slots: [id, name, status]
  Class3:
    is_a: NamedThing
    mixins: [HasId, HasName]
    slots: [id, name, status]
slots:
  id:
    identifier: true
    range: string
  name:
    required: true
    range: string
  status:
    range: string
"#;

    let schema_v1: linkml_core::SchemaDefinition = serde_yaml::from_str(schema_yaml)?;

    // Convert to V2 (with interning)
    let schema_v2: SchemaDefinitionV2 = schema_v1.into();

    // Check that common strings are interned
    let stats = get_pool_stats();
    assert!(stats.total_strings > 0);
    assert!(stats.memory_saved > 0);

    // Verify schema still works correctly
    assert_eq!(schema_v2.name.as_ref(), "MemoryTest");
    assert_eq!(
        schema_v2
            .default_prefix
            .as_ref()
            .expect("Test operation failed")
            .as_ref(),
        "ex"
    );

    // Check that repeated strings share memory
    let class_names: Vec<_> = schema_v2
        .classes
        .values()
        .filter_map(|c| c.is_a.as_ref())
        .collect();

    // All should point to the same "NamedThing" string
    assert!(class_names.len() >= 3);
    for name in &class_names {
        assert_eq!(name.as_ref(), "NamedThing");
    }
    Ok(())
}

    Ok(())
}

/// Test SchemaView API completeness
#[tokio::test]
async fn test_schema_view_api() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r#"
id: https://example.org/schemaview_test
name: SchemaViewTest
prefixes:
  ex: https://example.org/
  schema: http://schema.org/
classes:
  Person:
    class_uri: schema:Person
    is_a: NamedThing
    attributes:
      name:
        slot_uri: schema:name
        required: true
      age:
        range: integer
  NamedThing:
    abstract: true
    attributes:
      id:
        identifier: true
slots:
  id:
    identifier: true
    range: string
  name:
    range: string
"#;

    let schema: linkml_core::SchemaDefinition = serde_yaml::from_str(schema_yaml)?;
    let view = SchemaView::new(schema)?;

    // Test get_element
    let person_element = view.get_element("Person")?;
    assert_eq!(person_element.name(), "Person");

    // Test class hierarchy
    let parents = view.class_parents("Person", true)?;
    assert!(parents.contains(&"NamedThing".to_string());

    let children = view.class_children("NamedThing", false)?;
    assert!(children.contains(&"Person".to_string());

    // Test URI resolution
    let uri = view.get_uri("Person", false)?;
    assert_eq!(uri, "http://schema.org/Person");

    // Test slot methods
    let person_slots = view.class_slots("Person", true)?;
    assert!(person_slots.contains(&"id".to_string());
    assert!(person_slots.contains(&"name".to_string());
    assert!(person_slots.contains(&"age".to_string());

    // Test annotation methods
    let person_class = view.get_class("Person")?;
    let annotations = view.annotation_dict(person_class)?;
    // Annotations would be empty in this test schema

    Ok(())
}
    Ok(())
}

/// Test full LinkML service integration
#[tokio::test]
async fn test_full_service_integration() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;

    // Create configuration
    let config_path = temp_dir.path().join("linkml_config.yaml");
    std::fs::write(
        &config_path,
        r#"
typedb:
  server_address: "localhost:1729"
  default_database: "integration_test"
validation:
  max_depth: 10
  strict_mode: true
performance:
  parallel_evaluation_threshold: 5
"#,
    )?;

    // Create schema
    let schema_path = temp_dir.path().join("test_schema.yaml");
    std::fs::write(
        &schema_path,
        r#"
id: https://example.org/integration
name: IntegrationTest
prefixes:
  ex: https://example.org/
classes:
  Dataset:
    attributes:
      name:
        required: true
        pattern: "^[A-Za-z0-9_]+$"
      size:
        range: integer
        minimum_value: 0
      tags:
        multivalued: true
        range: string
"#,
    )?;

    // Create test data
    let data_path = temp_dir.path().join("test_data.yaml");
    std::fs::write(
        &data_path,
        r#"
name: test_dataset_1
size: 1000
tags:
  - genomics
  - public
"#,
    )?;

    // Initialize service
    let service = LinkMLService::new_with_config(&config_path).await?;

    // Load schema
    let schema = service.load_schema(&schema_path).await?;

    // Validate data
    let validation_result = service.validate_file(&data_path, &schema).await?;
    assert!(validation_result.valid);
    assert_eq!(validation_result.errors.len(), 0);

    // Test invalid data
    let invalid_data_path = temp_dir.path().join("invalid_data.yaml");
    std::fs::write(
        &invalid_data_path,
        r#"
name: "invalid name with spaces"
size: -100
"#,
    )?;

    let invalid_result = service.validate_file(&invalid_data_path, &schema).await?;
    assert!(!invalid_result.valid);
    assert!(invalid_result.errors.len() >= 2); // Pattern mismatch and negative value

    // Generate code
    let python_code = service
        .generate_code(&schema, GeneratorType::Python)
        .await?;
    assert!(python_code.contains("class Dataset"));
    assert!(python_code.contains("name: str"));

    // Test parallel validation with multiple files
    let mut data_files = vec![];
    for i in 0..10 {
        let path = temp_dir.path().join(format!("data_{}
    Ok(())
}.yaml", i));
        std::fs::write(
            &path,
            format!(
                r#"
name: dataset_{}
size: {}
tags: [tag1, tag2]
"#,
                i,
                i * 100
            ),
        )?;
        data_files.push(path);
    }

    let parallel_results = service
        .validate_files_parallel(&data_files, &schema)
        .await?;
    assert_eq!(parallel_results.len(), 10);
    assert!(parallel_results.iter().all(|r| r.valid));

    Ok(())
}

/// Test error handling and recovery
#[tokio::test]
async fn test_error_handling_no_panics() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let fs_adapter = Arc::new(FileSystemAdapter::new());

    // Test with non-existent file
    let parser = SchemaParser::new(fs_adapter.clone());
    let result = parser
        .parse_file(Path::new("/non/existent/file.yaml"))
        .await;
    assert!(result.is_err());
    match result {
        Err(e) => assert!(e.to_string().contains("No such file")),
        Ok(_) => panic!("Should have failed"),
    }
    Ok(())
}

    // Test with malformed YAML
    let malformed_path = temp_dir.path().join("malformed.yaml");
    fs_adapter
        .write_file(&malformed_path, b"{ invalid yaml }}}")
        .await?;

    let result = parser.parse_file(&malformed_path).await;
    assert!(result.is_err());

    // Test with circular imports
    let schema1_path = temp_dir.path().join("schema1.yaml");
    let schema2_path = temp_dir.path().join("schema2.yaml");

    fs_adapter
        .write_file(
            &schema1_path,
            r#"
id: https://example.org/schema1
name: Schema1
imports:
  - schema2.yaml
"#
            .as_bytes(),
        )
        .await?;

    fs_adapter
        .write_file(
            &schema2_path,
            r#"
id: https://example.org/schema2
name: Schema2
imports:
  - schema1.yaml
"#
            .as_bytes(),
        )
        .await?;

    let result = parser.parse_file(&schema1_path).await;
    // Should handle circular imports gracefully
    assert!(result.is_err() || result.is_ok()); // Either way, no panic

    Ok(())
}

/// Test configuration validation
#[test]
fn test_configuration_validation() {
    use linkml_service::config::LinkMLConfig;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

    // Test with invalid values
    let invalid_yaml = r#"
typedb:
  server_address: ""  # Empty not allowed
  default_database: "test"
  batch_size: -1     # Must be positive
  connection_timeout: 0s  # Must be positive
validation:
  max_depth: -5      # Must be positive
  max_errors: 0      # Must be positive
"#;

    let result: Result<LinkMLConfig, _> = serde_yaml::from_str(invalid_yaml);
    // Should fail validation
    assert!(result.is_err());

    // Test with valid values
    let valid_yaml = r#"
typedb:
  server_address: "localhost:1729"
  default_database: "test"
  batch_size: 100
  connection_timeout: 30s
validation:
  max_depth: 10
  max_errors: 100
"#;

    let result: Result<LinkMLConfig, _> = serde_yaml::from_str(valid_yaml);
    assert!(result.is_ok());
}
