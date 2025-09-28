//! Tests for API loading and dumping functionality

use linkml_core::prelude::*;
use linkml_service::loader::{
    ApiDumper, ApiLoader, ApiOptions, AuthConfig, DataDumper, DataLoader, EndpointConfig,
    PaginationConfig, PaginationStyle, RetryConfig, traits::DataInstance,
};
use reqwest::Method;
use serde_json::json;
use std::collections::HashMap;
use linkml_core::types::{SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, TypeDefinition, SubsetDefinition, Element};

/// Create a test schema
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("TestAPISchema".to_string());

    // User class
    let mut user_class = ClassDefinition::default();
    user_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "email".to_string(),
        "active".to_string(),
    ];
    schema.classes.insert("User".to_string(), user_class);

    // Product class
    let mut product_class = ClassDefinition::default();
    product_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "price".to_string(),
        "category".to_string(),
    ];
    schema.classes.insert("Product".to_string(), product_class);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut active_slot = SlotDefinition::default();
    active_slot.range = Some("boolean".to_string());
    schema.slots.insert("active".to_string(), active_slot);

    let mut price_slot = SlotDefinition::default();
    price_slot.range = Some("float".to_string());
    schema.slots.insert("price".to_string(), price_slot);

    let mut category_slot = SlotDefinition::default();
    category_slot.range = Some("string".to_string());
    schema.slots.insert("category".to_string(), category_slot);

    schema
}

#[test]
fn test_api_options_default() {
    let options = ApiOptions::default();

    assert_eq!(options.timeout_seconds, 30);
    assert!(options.follow_redirects);
    assert_eq!(options.user_agent, "LinkML-API-Loader/1.0");
    assert!(options.auth.is_none());
    assert!(options.pagination.is_none());
    assert!(options.rate_limit.is_none());
}

#[test]
fn test_auth_configurations() {
    // Bearer token
    let bearer = AuthConfig::Bearer("test-token".to_string());
    match bearer {
        AuthConfig::Bearer(token) => assert_eq!(token, "test-token"),
        _ => panic!("Wrong auth type"),
    }

    // Basic auth
    let basic = AuthConfig::Basic {
        username: "testuser".to_string(),
        password: "testpass".to_string(),
    };
    match basic {
        AuthConfig::Basic { username, password } => {
            assert_eq!(username, "testuser");
            assert_eq!(password, "testpass");
        }
        _ => panic!("Wrong auth type"),
    }

    // API key
    let api_key = AuthConfig::ApiKey {
        header_name: "X-API-Key".to_string(),
        key: "sk_test_123".to_string(),
    };
    match api_key {
        AuthConfig::ApiKey { header_name, key } => {
            assert_eq!(header_name, "X-API-Key");
            assert_eq!(key, "sk_test_123");
        }
        _ => panic!("Wrong auth type"),
    }

    // OAuth2
    let oauth2 = AuthConfig::OAuth2 {
        token_url: "https://auth.example.com/token".to_string(),
        client_id: "client123".to_string(),
        client_secret: "secret456".to_string(),
        scopes: vec!["read".to_string(), "write".to_string()],
    };
    match oauth2 {
        AuthConfig::OAuth2 {
            token_url,
            client_id,
            scopes,
            ..
        } => {
            assert_eq!(token_url, "https://auth.example.com/token");
            assert_eq!(client_id, "client123");
            assert_eq!(scopes.len(), 2);
        }
        _ => panic!("Wrong auth type"),
    }
}

#[test]
fn test_retry_config() {
    let config = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 200,
        max_delay_ms: 60000,
        backoff_factor: 3.0,
        retry_on_status: vec![429, 503],
    };

    assert_eq!(config.max_retries, 5);
    assert_eq!(config.initial_delay_ms, 200);
    assert_eq!(config.max_delay_ms, 60000);
    assert_eq!(config.backoff_factor, 3.0);
    assert_eq!(config.retry_on_status, vec![429, 503]);
}

#[test]
fn test_pagination_configs() {
    // Page number pagination
    let page_config = PaginationConfig {
        style: PaginationStyle::PageNumber,
        page_param: "page".to_string(),
        size_param: "limit".to_string(),
        default_size: 50,
        max_size: 200,
        data_path: "items".to_string(),
        next_path: Some("next_page".to_string()),
        total_path: Some("total".to_string()),
    };

    assert_eq!(page_config.style, PaginationStyle::PageNumber);
    assert_eq!(page_config.page_param, "page");
    assert_eq!(page_config.default_size, 50);

    // Offset pagination
    let offset_config = PaginationConfig {
        style: PaginationStyle::Offset,
        page_param: "skip".to_string(),
        size_param: "take".to_string(),
        default_size: 100,
        max_size: 1000,
        data_path: "data".to_string(),
        next_path: None,
        total_path: Some("count".to_string()),
    };

    assert_eq!(offset_config.style, PaginationStyle::Offset);
    assert_eq!(offset_config.page_param, "skip");
    assert_eq!(offset_config.size_param, "take");
}

#[test]
fn test_endpoint_config() {
    let mut query_params = HashMap::new();
    query_params.insert("status".to_string(), "active".to_string());
    query_params.insert("sort".to_string(), "name".to_string());

    let config = EndpointConfig {
        method: Method::GET,
        path: "/api/v2/users".to_string(),
        class_name: "User".to_string(),
        query_params,
        body_template: None,
        response_data_path: Some("data.users".to_string()),
        id_field: "user_id".to_string(),
    };

    assert_eq!(config.method, Method::GET);
    assert_eq!(config.path, "/api/v2/users");
    assert_eq!(config.class_name, "User");
    assert_eq!(config.query_params.len(), 2);
    assert_eq!(
        config.query_params.get("status"),
        Some(&"active".to_string())
    );
    assert_eq!(config.id_field, "user_id");
}

#[test]
fn test_field_mapping() {
    let mut options = ApiOptions::default();

    // Add field mappings for User class
    let mut user_mapping = HashMap::new();
    user_mapping.insert("userId".to_string(), "id".to_string());
    user_mapping.insert("userName".to_string(), "name".to_string());
    user_mapping.insert("emailAddress".to_string(), "email".to_string());
    user_mapping.insert("isActive".to_string(), "active".to_string());

    options
        .field_mapping
        .insert("User".to_string(), user_mapping);

    assert_eq!(options.field_mapping.len(), 1);
    let mapping = &options.field_mapping["User"];
    assert_eq!(mapping.get("userId"), Some(&"id".to_string());
    assert_eq!(mapping.get("userName"), Some(&"name".to_string());
    assert_eq!(mapping.get("emailAddress"), Some(&"email".to_string());
    assert_eq!(mapping.get("isActive"), Some(&"active".to_string());
}

#[test]
fn test_complete_configuration() {
    let mut options = ApiOptions {
        base_url: "https://api.example.com".to_string(),
        auth: Some(AuthConfig::Bearer("token123".to_string())),
        timeout_seconds: 60,
        follow_redirects: false,
        user_agent: "MyApp/2.0".to_string(),
        rate_limit: Some(5.0), // 5 requests per second
        ..Default::default()
    };

    // Add custom headers
    options
        .headers
        .insert("Accept".to_string(), "application/json".to_string());
    options
        .headers
        .insert("X-Custom-Header".to_string(), "custom-value".to_string());

    // Configure pagination
    options.pagination = Some(PaginationConfig {
        style: PaginationStyle::PageNumber,
        page_param: "p".to_string(),
        size_param: "s".to_string(),
        default_size: 25,
        max_size: 100,
        data_path: "results".to_string(),
        next_path: None,
        total_path: Some("totalCount".to_string()),
    });

    // Add endpoint
    options.endpoint_mapping.insert(
        "users".to_string(),
        EndpointConfig {
            method: Method::GET,
            path: "/users".to_string(),
            class_name: "User".to_string(),
            query_params: HashMap::new(),
            body_template: None,
            response_data_path: Some("data".to_string()),
            id_field: "id".to_string(),
        },
    );

    assert_eq!(options.base_url, "https://api.example.com");
    assert!(matches!(options.auth, Some(AuthConfig::Bearer(_)));
    assert_eq!(options.timeout_seconds, 60);
    assert!(!options.follow_redirects);
    assert_eq!(options.rate_limit, Some(5.0));
    assert_eq!(options.headers.len(), 2);
    assert!(options.pagination.is_some());
    assert_eq!(options.endpoint_mapping.len(), 1);
}

#[tokio::test]
async fn test_loader_creation() {
    let options = ApiOptions {
        base_url: "https://jsonplaceholder.typicode.com".to_string(),
        ..Default::default()
    };

    let loader = ApiLoader::new(options);
    // Loader should be created successfully
    assert!(loader.last_request_time.is_none());
}

#[tokio::test]
async fn test_dumper_creation() {
    let options = ApiOptions {
        base_url: "https://jsonplaceholder.typicode.com".to_string(),
        ..Default::default()
    };

    let dumper = ApiDumper::new(options);
    // Dumper should be created successfully
    assert!(dumper.last_request_time.is_none());
}

#[test]
fn test_json_path_extraction() {
    let loader = ApiLoader::new(ApiOptions::default());

    // Test nested path extraction
    let json = json!({
        "status": "success",
        "data": {
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"}
            ],
            "total": 2
        }
    });

    // Extract users array
    let users = loader
        .extract_by_path(&json, "data.users")
        .expect("Test operation failed");
    assert!(users.is_array());
    assert_eq!(users.as_array().expect("Test operation failed").len(), 2);

    // Extract total count
    let total = loader
        .extract_by_path(&json, "data.total")
        .expect("Test operation failed");
    assert_eq!(total.as_u64(), Some(2));

    // Extract first user's name
    let first_user_name = loader
        .extract_by_path(&json, "data.users.0.name")
        .expect("Test operation failed");
    assert_eq!(first_user_name.as_str(), Some("Alice"));
}

#[test]
fn test_data_instance_conversion() {
    let loader = ApiLoader::new(ApiOptions::default());

    let json = json!({
        "id": "123",
        "name": "Test User",
        "email": "test@example.com",
        "active": true
    });

    let obj = json.as_object().expect("Test operation failed").clone();
    let endpoint_config = EndpointConfig {
        method: Method::GET,
        path: "/users".to_string(),
        class_name: "User".to_string(),
        query_params: HashMap::new(),
        body_template: None,
        response_data_path: None,
        id_field: "id".to_string(),
    };

    let instance = loader
        .object_to_instance(obj, "User", &endpoint_config)
        .expect("Test operation failed");

    assert_eq!(instance.class_name, "User");
    assert_eq!(instance.data.get("id"), Some(&json!("123"));
    assert_eq!(instance.data.get("name"), Some(&json!("Test User"));
    assert_eq!(instance.data.get("email"), Some(&json!("test@example.com"));
    assert_eq!(instance.data.get("active"), Some(&json!(true));
}

#[test]
fn test_instance_with_field_mapping() {
    let mut options = ApiOptions::default();

    // Add field mapping
    let mut mapping = HashMap::new();
    mapping.insert("user_id".to_string(), "id".to_string());
    mapping.insert("full_name".to_string(), "name".to_string());
    options.field_mapping.insert("User".to_string(), mapping);

    let loader = ApiLoader::new(options);

    let json = json!({
        "user_id": "456",
        "full_name": "John Doe",
        "email": "john@example.com", // Unmapped field
        "extra_field": "extra_value"  // Another unmapped field
    });

    let obj = json.as_object().expect("Test operation failed").clone();
    let endpoint_config = EndpointConfig {
        method: Method::GET,
        path: "/users".to_string(),
        class_name: "User".to_string(),
        query_params: HashMap::new(),
        body_template: None,
        response_data_path: None,
        id_field: "id".to_string(),
    };

    let instance = loader
        .object_to_instance(obj, "User", &endpoint_config)
        .expect("Test operation failed");

    // Check mapped fields
    assert_eq!(instance.data.get("id"), Some(&json!("456"));
    assert_eq!(instance.data.get("name"), Some(&json!("John Doe"));

    // Check unmapped fields are preserved
    assert_eq!(instance.data.get("email"), Some(&json!("john@example.com"));
    assert_eq!(
        instance.data.get("extra_field"),
        Some(&json!("extra_value"))
    );
}
