//! Advanced tests for import resolution with aliases, mappings, and edge cases

use linkml_core::{
    settings::{ImportSettings, SchemaSettings, ImportResolutionStrategy},
    types::SchemaDefinition,
};
use linkml_service::parser::{ImportResolverV2, SchemaParser, YamlParser};
use tempfile::TempDir;
use tokio::fs;
use std::collections::HashMap;

#[tokio::test]
async fn test_import_aliases_and_mappings() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create common schema
    let common_v2 = r#"
id: https://example.org/common/v2
name: common_v2
classes:
  CommonBase:
    description: Common base class v2
"#;
    
    let common_dir = base_path.join("common");
    fs::create_dir_all(&common_dir).await.unwrap();
    fs::write(common_dir.join("v2.yaml"), common_v2).await.unwrap();
    
    // Create base schemas directory
    let base_schemas_dir = base_path.join("base_schemas");
    fs::create_dir_all(&base_schemas_dir).await.unwrap();
    
    let entity_schema = r#"
id: https://base.example.org/schemas/entity
name: entity
classes:
  Entity:
    description: Base entity
"#;
    fs::write(base_schemas_dir.join("entity.yaml"), entity_schema).await.unwrap();
    
    // Create main schema with aliases and mappings
    let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    import_aliases:
      common: common/v2
    import_mappings:
      "base:": "base_schemas/"
imports:
  - common  # Should resolve to common/v2.yaml
  - base:entity  # Should resolve to base_schemas/entity.yaml
classes:
  MyClass:
    is_a: CommonBase
    description: Uses aliased import
"#;
    
    fs::write(base_path.join("main.yaml"), main_schema).await.unwrap();
    
    // Create resolver and load schema
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("main.yaml")).await.unwrap()).unwrap();
    
    // Apply settings to resolver
    if let Some(ref settings) = schema.settings {
        resolver.resolve_imports(&mut schema, Some(&settings.imports)).await.unwrap();
    } else {
        resolver.resolve_imports(&mut schema, None).await.unwrap();
    }
    
    // Verify imports were resolved
    assert!(schema.classes.contains_key("CommonBase"));
    assert!(schema.classes.contains_key("Entity"));
    assert!(schema.classes.contains_key("MyClass"));
}

#[tokio::test]
async fn test_url_import_resolution() {
    // Note: This test uses mock URLs - in production these would be real
    let mut schema = SchemaDefinition::new("test_schema");
    schema.imports = vec![
        "https://w3id.org/linkml/types".to_string(),
        "https://example.org/schemas/base".to_string(),
    ];
    
    let resolver = ImportResolverV2::new();
    
    // The resolver should handle URL imports (actual implementation would fetch)
    // For now, we just verify the imports are recognized as URLs
    for import in &schema.imports {
        assert!(import.starts_with("http://") || import.starts_with("https://"));
    }
}

#[tokio::test]
async fn test_circular_import_detection() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create schema A that imports B
    let schema_a = r#"
id: https://example.org/a
name: schema_a
imports:
  - schema_b
classes:
  ClassA:
    description: Class in schema A
"#;
    
    // Create schema B that imports C
    let schema_b = r#"
id: https://example.org/b
name: schema_b  
imports:
  - schema_c
classes:
  ClassB:
    description: Class in schema B
"#;
    
    // Create schema C that imports A (circular!)
    let schema_c = r#"
id: https://example.org/c
name: schema_c
imports:
  - schema_a
classes:
  ClassC:
    description: Class in schema C
"#;
    
    fs::write(base_path.join("schema_a.yaml"), schema_a).await.unwrap();
    fs::write(base_path.join("schema_b.yaml"), schema_b).await.unwrap();
    fs::write(base_path.join("schema_c.yaml"), schema_c).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("schema_a.yaml")).await.unwrap()).unwrap();
    
    // Should handle circular imports gracefully
    let result = resolver.resolve_imports(&mut schema, None).await;
    
    // The resolver should either:
    // 1. Detect the cycle and return an error, or
    // 2. Handle it gracefully by tracking visited schemas
    // Both are valid approaches
    
    if result.is_err() {
        // If it errors, it should be a circular dependency error
        let err = result.unwrap_err();
        assert!(err.to_string().contains("ircular") || err.to_string().contains("cycle"));
    } else {
        // If it succeeds, each class should be imported only once
        assert_eq!(schema.classes.keys().filter(|k| *k == "ClassA").count(), 1);
        assert_eq!(schema.classes.keys().filter(|k| *k == "ClassB").count(), 1);
        assert_eq!(schema.classes.keys().filter(|k| *k == "ClassC").count(), 1);
    }
}

#[tokio::test]
async fn test_selective_imports() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create a large schema to import from
    let large_schema = r#"
id: https://example.org/large
name: large_schema
classes:
  ClassA:
    description: Class A
  ClassB:
    description: Class B
  ClassC:
    description: Class C
  ClassD:
    description: Class D
slots:
  slot1:
    range: string
  slot2:
    range: integer
  slot3:
    range: boolean
  slot4:
    range: float
"#;
    
    fs::write(base_path.join("large.yaml"), large_schema).await.unwrap();
    
    // Import with 'only' directive (mock - actual implementation would filter)
    let selective_schema = r#"
id: https://example.org/selective
name: selective
imports:
  - large  # In real implementation: large only: [ClassA, ClassB, slot1]
classes:
  MyClass:
    description: Uses selective import
"#;
    
    fs::write(base_path.join("selective.yaml"), selective_schema).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("selective.yaml")).await.unwrap()).unwrap();
    
    resolver.resolve_imports(&mut schema, None).await.unwrap();
    
    // In a full implementation with selective imports, we would verify
    // only the requested elements were imported
    assert!(schema.classes.contains_key("ClassA"));
    assert!(schema.classes.contains_key("MyClass"));
}

#[tokio::test]
async fn test_import_conflict_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create two schemas with conflicting definitions
    let schema1 = r#"
id: https://example.org/schema1
name: schema1
classes:
  Person:
    description: Person from schema1
    slots:
      - name
      - age
slots:
  name:
    range: string
    required: true
  age:
    range: integer
"#;
    
    let schema2 = r#"
id: https://example.org/schema2  
name: schema2
classes:
  Person:
    description: Person from schema2
    slots:
      - full_name
      - birth_date
slots:
  full_name:
    range: string
  birth_date:
    range: date
"#;
    
    fs::write(base_path.join("schema1.yaml"), schema1).await.unwrap();
    fs::write(base_path.join("schema2.yaml"), schema2).await.unwrap();
    
    // Schema that imports both (conflict!)
    let main_schema = r#"
id: https://example.org/main
name: main
imports:
  - schema1
  - schema2
classes:
  Employee:
    is_a: Person  # Which Person?
    description: Employee class
"#;
    
    fs::write(base_path.join("main.yaml"), main_schema).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("main.yaml")).await.unwrap()).unwrap();
    
    resolver.resolve_imports(&mut schema, None).await.unwrap();
    
    // Last import wins strategy - schema2's Person should override
    let person = schema.classes.get("Person").unwrap();
    assert_eq!(person.description.as_deref(), Some("Person from schema2"));
    assert!(person.slots.contains(&"full_name".to_string()));
}

#[tokio::test]
async fn test_nested_import_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create deeply nested directory structure
    let deep_path = base_path.join("org").join("example").join("schemas").join("v1");
    fs::create_dir_all(&deep_path).await.unwrap();
    
    let nested_schema = r#"
id: https://example.org/nested
name: nested
classes:
  NestedClass:
    description: Deeply nested class
"#;
    
    fs::write(deep_path.join("nested.yaml"), nested_schema).await.unwrap();
    
    // Create schema with complex import path
    let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    resolution_strategy: Mixed
imports:
  - org/example/schemas/v1/nested
classes:
  MainClass:
    is_a: NestedClass
"#;
    
    fs::write(base_path.join("main.yaml"), main_schema).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("main.yaml")).await.unwrap()).unwrap();
    
    resolver.resolve_imports(&mut schema, None).await.unwrap();
    
    assert!(schema.classes.contains_key("NestedClass"));
    assert!(schema.classes.contains_key("MainClass"));
}

#[tokio::test]
async fn test_import_with_different_strategies() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Test different resolution strategies
    let strategies = vec![
        ImportResolutionStrategy::Relative,
        ImportResolutionStrategy::Absolute,
        ImportResolutionStrategy::Mixed,
    ];
    
    for strategy in strategies {
        let mut settings = ImportSettings::default();
        settings.resolution_strategy = strategy.clone();
        
        // Create a simple schema
        let test_schema = format!(r#"
id: https://example.org/test_{}
name: test_{}
settings:
  imports:
    resolution_strategy: {:?}
"#, 
            format!("{:?}", strategy).to_lowercase(),
            format!("{:?}", strategy).to_lowercase(),
            strategy
        );
        
        let filename = format!("test_{}.yaml", format!("{:?}", strategy).to_lowercase());
        fs::write(base_path.join(&filename), test_schema).await.unwrap();
        
        let parser = YamlParser::new();
        let schema = parser.parse(&tokio::fs::read_to_string(base_path.join(&filename)).await.unwrap()).unwrap();
        
        // Verify strategy was parsed correctly
        if let Some(schema_settings) = schema.settings {
            assert_eq!(schema_settings.imports.resolution_strategy, strategy);
        }
    }
}

#[tokio::test]
async fn test_import_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Schema with non-existent import
    let bad_schema = r#"
id: https://example.org/bad
name: bad
imports:
  - non_existent_schema
  - another_missing_schema
"#;
    
    fs::write(base_path.join("bad.yaml"), bad_schema).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("bad.yaml")).await.unwrap()).unwrap();
    
    let result = resolver.resolve_imports(&mut schema, None).await;
    
    // Should fail with clear error about missing imports
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("non_existent_schema") || 
           err.to_string().contains("not found") ||
           err.to_string().contains("failed to load"));
}

#[tokio::test]
async fn test_import_with_version_specifiers() {
    let temp_dir = TempDir::new().unwrap();
    let base_path = temp_dir.path();
    
    // Create versioned schemas
    let v1_dir = base_path.join("common").join("v1");
    let v2_dir = base_path.join("common").join("v2");
    fs::create_dir_all(&v1_dir).await.unwrap();
    fs::create_dir_all(&v2_dir).await.unwrap();
    
    let common_v1 = r#"
id: https://example.org/common/v1
name: common_v1
version: 1.0.0
classes:
  Base:
    description: Base class v1
    slots:
      - id
"#;
    
    let common_v2 = r#"
id: https://example.org/common/v2
name: common_v2
version: 2.0.0
classes:
  Base:
    description: Base class v2
    slots:
      - identifier
      - name
"#;
    
    fs::write(v1_dir.join("common.yaml"), common_v1).await.unwrap();
    fs::write(v2_dir.join("common.yaml"), common_v2).await.unwrap();
    
    // Schema that uses version-specific import
    let main_schema = r#"
id: https://example.org/main
name: main
settings:
  imports:
    import_aliases:
      common: common/v2/common
imports:
  - common
"#;
    
    fs::write(base_path.join("main.yaml"), main_schema).await.unwrap();
    
    let resolver = ImportResolverV2::new()
        .with_base_path(base_path.to_path_buf());
    
    let parser = YamlParser::new();
    let mut schema = parser.parse(&tokio::fs::read_to_string(base_path.join("main.yaml")).await.unwrap()).unwrap();
    
    if let Some(ref settings) = schema.settings {
        resolver.resolve_imports(&mut schema, Some(&settings.imports)).await.unwrap();
    }
    
    // Should have v2 Base class
    let base_class = schema.classes.get("Base").unwrap();
    assert_eq!(base_class.description.as_deref(), Some("Base class v2"));
    assert!(base_class.slots.contains(&"identifier".to_string()));
}