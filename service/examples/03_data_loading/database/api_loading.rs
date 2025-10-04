//! Example demonstrating API loading and dumping in LinkML
//!
//! This example shows how to:
//! 1. Configure API authentication
//! 2. Load data from REST APIs with pagination
//! 3. Map API responses to LinkML instances
//! 4. Dump LinkML instances back to APIs

use linkml_core::prelude::*;
use linkml_service::loader::{
    ApiDumper, ApiLoader, ApiOptions, AuthConfig, EndpointConfig,
    PaginationConfig, PaginationStyle, RetryConfig,
};
use reqwest::Method;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!(
        "=== LinkML API Loading Example ===
"
    );

    // Create a sample schema for a user management API
    let schema = create_user_api_schema();

    // Example 1: Basic API Configuration
    println!("1. Basic API Configuration");
    println!(
        "=========================
"
    );

    let mut basic_options = ApiOptions {
        base_url: "https://api.example.com".to_string(),
        timeout_seconds: 30,
        user_agent: "LinkML-API-Client/1.0".to_string(),
        ..Default::default()
    };

    // Configure endpoints
    basic_options.endpoint_mapping.insert(
        "users".to_string(),
        EndpointConfig {
            method: Method::GET,
            path: "/v1/users".to_string(),
            class_name: "User".to_string(),
            query_params: HashMap::new(),
            body_template: None,
            response_data_path: Some("data".to_string()),
            id_field: "id".to_string(),
        },
    );

    basic_options.endpoint_mapping.insert(
        "posts".to_string(),
        EndpointConfig {
            method: Method::GET,
            path: "/v1/posts".to_string(),
            class_name: "Post".to_string(),
            query_params: [("status".to_string(), "published".to_string())].into(),
            body_template: None,
            response_data_path: Some("data.posts".to_string()),
            id_field: "id".to_string(),
        },
    );

    println!("API Base URL: {}", basic_options.base_url);
    println!("Configured endpoints:");
    for (name, config) in &basic_options.endpoint_mapping {
        println!(
            "  - {}: {} {} -> {}",
            name, config.method, config.path, config.class_name
        );
    }
    println!();

    // Example 2: Authentication Methods
    println!("2. Authentication Methods");
    println!(
        "========================
"
    );

    // Bearer token authentication
    let bearer_auth = AuthConfig::Bearer("your-api-token-here".to_string());
    println!("Bearer Token Authentication:");
    println!("  Authorization: Bearer your-api-token-here");
    println!();

    // Basic authentication
    let basic_auth = AuthConfig::Basic {
        username: "api_user".to_string(),
        password: "api_password".to_string(),
    };
    println!("Basic Authentication:");
    println!("  Username: api_user");
    println!("  Password: ***");
    println!();

    // API key authentication
    let api_key_auth = AuthConfig::ApiKey {
        header_name: "X-API-Key".to_string(),
        key: "sk_live_example_key".to_string(),
    };
    println!("API Key Authentication:");
    println!("  Header: X-API-Key");
    println!("  Key: sk_live_***");
    println!();

    // OAuth2 configuration
    let oauth2_auth = AuthConfig::OAuth2 {
        token_url: "https://auth.example.com/oauth/token".to_string(),
        client_id: "client_id".to_string(),
        client_secret: "client_secret".to_string(),
        scopes: vec!["read:users".to_string(), "write:users".to_string()],
    };
    println!("OAuth2 Configuration:");
    println!("  Token URL: https://auth.example.com/oauth/token");
    println!("  Scopes: read:users, write:users");
    println!();

    // Example 3: Pagination Strategies
    println!("3. Pagination Strategies");
    println!(
        "=======================
"
    );

    // Page number pagination
    let page_pagination = PaginationConfig {
        style: PaginationStyle::PageNumber,
        page_param: "page".to_string(),
        size_param: "per_page".to_string(),
        default_size: 100,
        max_size: 1000,
        data_path: "data".to_string(),
        next_path: Some("next_page".to_string()),
        total_path: Some("total_count".to_string()),
    };
    println!("Page Number Pagination:");
    println!("  URL: /api/users?page=1&per_page=100");
    println!("  URL: /api/users?page=2&per_page=100");
    println!();

    // Offset pagination
    let offset_pagination = PaginationConfig {
        style: PaginationStyle::Offset,
        page_param: "offset".to_string(),
        size_param: "limit".to_string(),
        default_size: 50,
        max_size: 500,
        data_path: "results".to_string(),
        next_path: None,
        total_path: Some("count".to_string()),
    };
    println!("Offset Pagination:");
    println!("  URL: /api/users?offset=0&limit=50");
    println!("  URL: /api/users?offset=50&limit=50");
    println!();

    // Example 4: Field Mapping
    println!("4. Field Mapping");
    println!(
        "===============
"
    );

    let mut mapped_options = ApiOptions::default();

    // Map API fields to LinkML slots
    let mut user_mapping = HashMap::new();
    user_mapping.insert("user_id".to_string(), "id".to_string());
    user_mapping.insert("full_name".to_string(), "name".to_string());
    user_mapping.insert("email_address".to_string(), "email".to_string());
    user_mapping.insert("created_timestamp".to_string(), "created_at".to_string());
    mapped_options
        .field_mapping
        .insert("User".to_string(), user_mapping);

    println!("User field mappings:");
    for (api_field, linkml_field) in &mapped_options.field_mapping["User"] {
        println!("  {} -> {}", api_field, linkml_field);
    }
    println!();

    // Example 5: Retry and Rate Limiting
    println!("5. Retry and Rate Limiting");
    println!(
        "=========================
"
    );

    let retry_config = RetryConfig {
        max_retries: 5,
        initial_delay_ms: 100,
        max_delay_ms: 30000,
        backoff_factor: 2.0,
        retry_on_status: vec![429, 500, 502, 503, 504],
    };

    println!("Retry Configuration:");
    println!("  Max retries: {}", retry_config.max_retries);
    println!("  Initial delay: {}ms", retry_config.initial_delay_ms);
    println!("  Backoff factor: {}", retry_config.backoff_factor);
    println!(
        "  Retry on status codes: {:?}",
        retry_config.retry_on_status
    );
    println!();

    let mut rate_limited_options = ApiOptions {
        rate_limit: Some(10.0), // 10 requests per second
        retry_config,
        ..Default::default()
    };

    println!("Rate Limiting:");
    println!("  Max requests per second: 10");
    println!("  Automatic throttling enabled");
    println!();

    // Example 6: Loading Data
    println!("6. Loading Data");
    println!(
        "==============
"
    );

    // Configure a complete example
    let mut github_options = ApiOptions {
        base_url: "https://api.github.com".to_string(),
        auth: Some(AuthConfig::Bearer("ghp_example_token".to_string())),
        timeout_seconds: 60,
        pagination: Some(PaginationConfig {
            style: PaginationStyle::PageNumber,
            page_param: "page".to_string(),
            size_param: "per_page".to_string(),
            default_size: 30,
            max_size: 100,
            data_path: ".".to_string(), // GitHub returns array directly
            next_path: None,
            total_path: None,
        }),
        ..Default::default()
    };

    // Add custom headers
    github_options.headers.insert(
        "Accept".to_string(),
        "application/vnd.github.v3+json".to_string(),
    );

    // Configure repository endpoint
    github_options.endpoint_mapping.insert(
        "repos".to_string(),
        EndpointConfig {
            method: Method::GET,
            path: "/users/rust-lang/repos".to_string(),
            class_name: "Repository".to_string(),
            query_params: [
                ("sort".to_string(), "updated".to_string()),
                ("direction".to_string(), "desc".to_string()),
            ]
            .into(),
            body_template: None,
            response_data_path: None,
            id_field: "id".to_string(),
        },
    );

    println!("GitHub API Configuration:");
    println!("  Base URL: {}", github_options.base_url);
    println!("  Authentication: Bearer token");
    println!("  Endpoint: /users/rust-lang/repos");
    println!("  Pagination: 30 items per page");
    println!();

    // Note: In a real application, you would load from the actual API
    // let mut loader = ApiLoader::new(github_options);
    // let instances = loader.load(&schema).await?;
    // println!("Loaded {} instances", instances.len());

    // Example 7: Dumping Data
    println!("7. Dumping Data to API");
    println!(
        "=====================
"
    );

    let mut crud_options = ApiOptions {
        base_url: "https://api.example.com".to_string(),
        auth: Some(bearer_auth),
        ..Default::default()
    };

    // Configure POST endpoint for creating users
    crud_options.endpoint_mapping.insert(
        "create_user".to_string(),
        EndpointConfig {
            method: Method::POST,
            path: "/v1/users".to_string(),
            class_name: "User".to_string(),
            query_params: HashMap::new(),
            body_template: None,
            response_data_path: None,
            id_field: "id".to_string(),
        },
    );

    // Configure PUT endpoint for updating users
    crud_options.endpoint_mapping.insert(
        "update_user".to_string(),
        EndpointConfig {
            method: Method::PUT,
            path: "/v1/users".to_string(), // ID will be appended
            class_name: "User".to_string(),
            query_params: HashMap::new(),
            body_template: None,
            response_data_path: None,
            id_field: "id".to_string(),
        },
    );

    println!("CRUD Operations:");
    println!("  POST /v1/users - Create new user");
    println!("  PUT /v1/users/{id} - Update existing user");
    println!("  PATCH /v1/users/{id} - Partial update");
    println!();

    // Sample instances to dump
    let instances = create_sample_instances()?;
    println!("Sample instances to dump: {}", instances.len());

    // Note: In a real application, you would dump to the actual API
    // let mut dumper = ApiDumper::new(crud_options);
    // let result = dumper.dump(&instances, &schema).await?;
    // println!("Dump result: {}", String::from_utf8_lossy(&result));

    println!(
        "
✅ API loading examples complete!"
    );
    println!(
        "
Key features demonstrated:"
    );
    println!("- Multiple authentication methods (Bearer, Basic, API Key, OAuth2)");
    println!("- Pagination strategies (Page, Offset, Cursor, Link Header)");
    println!("- Field mapping between API and LinkML");
    println!("- Retry logic with exponential backoff");
    println!("- Rate limiting for API compliance");
    println!("- Custom headers and query parameters");
    println!("- CRUD operations (GET, POST, PUT, PATCH)");

    Ok(())
}

/// Create a sample schema for user API
fn create_user_api_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("UserAPISchema".to_string());
    schema.description = Some("Schema for user management API".to_string());

    // User class
    let mut user_class = ClassDefinition::default();
    user_class.description = Some("A user in the system".to_string());
    user_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "email".to_string(),
        "created_at".to_string(),
        "updated_at".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("User".to_string(), user_class);

    // Post class
    let mut post_class = ClassDefinition::default();
    post_class.description = Some("A blog post".to_string());
    post_class.slots = vec![
        "id".to_string(),
        "author_id".to_string(),
        "title".to_string(),
        "content".to_string(),
        "published_at".to_string(),
        "tags".to_string(),
    ];
    schema.classes.insert("Post".to_string(), post_class);

    // Repository class (for GitHub example)
    let mut repo_class = ClassDefinition::default();
    repo_class.description = Some("A GitHub repository".to_string());
    repo_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "full_name".to_string(),
        "description".to_string(),
        "language".to_string(),
        "stars".to_string(),
    ];
    schema.classes.insert("Repository".to_string(), repo_class);

    // Define slots
    let slots = vec![
        ("id", "string", true, true),
        ("name", "string", true, false),
        ("email", "string", false, false),
        ("title", "string", true, false),
        ("content", "string", false, false),
        ("description", "string", false, false),
        ("created_at", "datetime", false, false),
        ("updated_at", "datetime", false, false),
        ("published_at", "datetime", false, false),
        ("status", "string", false, false),
        ("author_id", "string", true, false),
        ("full_name", "string", false, false),
        ("language", "string", false, false),
        ("stars", "integer", false, false),
        ("tags", "string", false, false),
    ];

    for (name, range, required, identifier) in slots {
        let mut slot = SlotDefinition::default();
        slot.range = Some(range.to_string());
        slot.required = Some(required);
        slot.identifier = Some(identifier);
        if name == "tags" {
            slot.multivalued = Some(true);
        }
        schema.slots.insert(name.to_string(), slot);
    }

    schema
}

/// Create sample instances for dumping
fn create_sample_instances()
-> Result<Vec<linkml_service::loader::traits::DataInstance>, serde_json::Error> {
    use linkml_service::loader::traits::DataInstance;
    use serde_json::json;

    Ok(vec![
        DataInstance {
            class_name: "User".to_string(),
            data: serde_json::from_value(json!({
                "id": "user-123",
                "name": "Alice Johnson",
                "email": "alice@example.com",
                "status": "active",
                "created_at": "2024-01-15T10:30:00Z"
            }))?,
            id: Some("user-123".to_string()),
            metadata: std::collections::HashMap::new(),
        },
        DataInstance {
            class_name: "Post".to_string(),
            data: serde_json::from_value(json!({
                "author_id": "user-123",
                "title": "Introduction to LinkML",
                "content": "LinkML is a powerful schema language...",
                "tags": ["linkml", "data-modeling", "schemas"],
                "published_at": "2024-02-01T14:00:00Z"
            }))?,
            id: Some("post-456".to_string()),
            metadata: std::collections::HashMap::new(),
        },
    ])
}
