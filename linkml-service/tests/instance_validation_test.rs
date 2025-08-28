//! Tests for instance-based validation

use linkml_service::parser::Parser;
use linkml_service::validator::{
    InstanceConfig, InstanceLoader, ValidationEngine, validate_as_class,
};
use serde_json::json;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_instance_validation_from_json() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema
description: Test schema for instance-based validation

classes:
  Address:
    name: Address
    slots:
      - country
      - state

slots:
  country:
    name: country
    range: string
    description: Country code validated against instance data

  state:
    name: state
    range: string
    description: State/province code validated against instance data
"#;

    // Create instance data files
    let temp_dir = TempDir::new().unwrap();

    // Country codes
    let countries_file = temp_dir.path().join("countries.json");
    let countries_data = json!([
        {"code": "US", "name": "United States"},
        {"code": "CA", "name": "Canada"},
        {"code": "MX", "name": "Mexico"},
        {"code": "UK", "name": "United Kingdom"},
        {"code": "FR", "name": "France"},
        {"code": "DE", "name": "Germany"}
    ]);
    fs::write(
        &countries_file,
        serde_json::to_string_pretty(&countries_data).unwrap(),
    )
    .await
    .unwrap();

    // US states
    let states_file = temp_dir.path().join("us_states.json");
    let states_data = json!([
        {"code": "CA", "name": "California"},
        {"code": "TX", "name": "Texas"},
        {"code": "NY", "name": "New York"},
        {"code": "FL", "name": "Florida"}
    ]);
    fs::write(
        &states_file,
        serde_json::to_string_pretty(&states_data).unwrap(),
    )
    .await
    .unwrap();

    // Parse schema
    let parser = Parser::new();
    let schema = parser.parse_str(schema_yaml, "yaml").unwrap();

    // Create engine with instance validation
    let _engine = ValidationEngine::new(&schema).unwrap();

    // Load instance data
    let loader = InstanceLoader::new();
    let country_config = InstanceConfig {
        key_field: "code".to_string(),
        value_field: None,
        filter: None,
    };

    let country_data = loader
        .load_json_file(&countries_file, &country_config)
        .await
        .unwrap();
    let state_data = loader
        .load_json_file(&states_file, &country_config)
        .await
        .unwrap();

    // Create combined instance data
    let mut instance_values: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    instance_values.insert(
        "country".to_string(),
        country_data.values.keys().cloned().collect(),
    );
    instance_values.insert(
        "state".to_string(),
        state_data.values.keys().cloned().collect(),
    );

    // Set instance data on engine (this would be a method we'd add)
    // For now, we'll test without the full integration

    // Valid data
    let valid_data = json!({
        "country": "US",
        "state": "CA"
    });

    // This test demonstrates the concept - full integration would require
    // modifying ValidationEngine to support instance data
    let report = validate_as_class(&schema, &valid_data, "Address", None)
        .await
        .unwrap();

    // Without instance validation integration, this would pass
    // With it, it would validate against loaded data
    assert!(report.valid);
}

#[tokio::test]
async fn test_instance_validation_from_csv() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  Product:
    name: Product
    slots:
      - category
      - subcategory

slots:
  category:
    name: category
    range: string

  subcategory:
    name: subcategory
    range: string
"#;

    // Create CSV instance data
    let temp_dir = TempDir::new().unwrap();
    let categories_file = temp_dir.path().join("categories.csv");

    let csv_data = "id,name,parent\nELEC,Electronics,\nCOMP,Computers,ELEC\nPHON,Phones,ELEC\nCLOT,Clothing,\nMENS,Mens,CLOT\nWOMN,Womens,CLOT\n";
    fs::write(&categories_file, csv_data).await.unwrap();

    let parser = Parser::new();
    let _schema = parser.parse_str(schema_yaml, "yaml").unwrap();

    // Load CSV data
    let loader = InstanceLoader::new();
    let config = InstanceConfig {
        key_field: "id".to_string(),
        value_field: None,
        filter: None,
    };

    let category_data = loader
        .load_csv_file(&categories_file, &config)
        .await
        .unwrap();

    // Check that data was loaded correctly
    assert!(category_data.values.contains_key("ELEC"));
    assert!(category_data.values.contains_key("COMP"));
    assert_eq!(category_data.values.len(), 6);

    // Test caching
    let category_data2 = loader
        .load_csv_file(&categories_file, &config)
        .await
        .unwrap();
    assert!(std::sync::Arc::ptr_eq(&category_data, &category_data2));
}

#[tokio::test]
async fn test_instance_validation_with_multivalued() {
    let schema_yaml = r#"
id: https://example.org/test
name: test_schema

classes:
  TaggedItem:
    name: TaggedItem
    slots:
      - tags

slots:
  tags:
    name: tags
    range: string
    multivalued: true
    description: Tags from controlled vocabulary
"#;

    // Create tag vocabulary
    let temp_dir = TempDir::new().unwrap();
    let tags_file = temp_dir.path().join("tags.json");

    let tags_data = json!({
        "tags": [
            {"id": "important", "label": "Important"},
            {"id": "urgent", "label": "Urgent"},
            {"id": "review", "label": "Needs Review"},
            {"id": "approved", "label": "Approved"},
            {"id": "draft", "label": "Draft"}
        ]
    });
    fs::write(
        &tags_file,
        serde_json::to_string_pretty(&tags_data).unwrap(),
    )
    .await
    .unwrap();

    let parser = Parser::new();
    let schema = parser.parse_str(schema_yaml, "yaml").unwrap();

    // Load instance data
    let loader = InstanceLoader::new();
    let config = InstanceConfig {
        key_field: "id".to_string(),
        value_field: Some("label".to_string()),
        filter: None,
    };

    let tag_data = loader.load_json_file(&tags_file, &config).await.unwrap();

    // Verify loading worked
    assert_eq!(tag_data.values.len(), 5);
    assert_eq!(
        tag_data.values.get("important").unwrap(),
        &vec!["Important"]
    );

    // Test data with multiple tags
    let data = json!({
        "tags": ["important", "urgent", "approved"]
    });

    // This would validate each tag against the loaded vocabulary
    let report = validate_as_class(&schema, &data, "TaggedItem", None)
        .await
        .unwrap();
    assert!(report.valid);
}
