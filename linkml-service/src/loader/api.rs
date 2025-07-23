//! API loader and dumper for LinkML
//!
//! This module provides functionality to load data from REST APIs
//! and dump LinkML instances to API endpoints.

use super::traits::{DataLoader, DataDumper, LoaderError, LoaderResult, DumperError, DumperResult, DataInstance};
use linkml_core::prelude::*;
use async_trait::async_trait;
use serde_json::{Value, Map};
use reqwest::{Client, Method, header::{HeaderMap, HeaderName, HeaderValue}};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, info, warn};

/// Options for API loading and dumping
#[derive(Debug, Clone)]
pub struct ApiOptions {
    /// Base URL for the API
    pub base_url: String,
    
    /// Authentication configuration
    pub auth: Option<AuthConfig>,
    
    /// Custom headers to include in requests
    pub headers: HashMap<String, String>,
    
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    
    /// Retry configuration
    pub retry_config: RetryConfig,
    
    /// Pagination configuration
    pub pagination: Option<PaginationConfig>,
    
    /// Endpoint to LinkML class mapping
    pub endpoint_mapping: HashMap<String, EndpointConfig>,
    
    /// Field mapping for responses
    pub field_mapping: HashMap<String, HashMap<String, String>>,
    
    /// Whether to follow redirects
    pub follow_redirects: bool,
    
    /// User agent string
    pub user_agent: String,
    
    /// Rate limiting (requests per second)
    pub rate_limit: Option<f64>,
}

/// Authentication configuration
#[derive(Debug, Clone)]
pub enum AuthConfig {
    /// Bearer token authentication
    Bearer(String),
    
    /// Basic authentication
    Basic { username: String, password: String },
    
    /// API key authentication
    ApiKey { header_name: String, key: String },
    
    /// OAuth2 configuration
    OAuth2 {
        token_url: String,
        client_id: String,
        client_secret: String,
        scopes: Vec<String>,
    },
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retries
    pub max_retries: u32,
    
    /// Initial retry delay in milliseconds
    pub initial_delay_ms: u64,
    
    /// Maximum retry delay in milliseconds
    pub max_delay_ms: u64,
    
    /// Exponential backoff factor
    pub backoff_factor: f64,
    
    /// HTTP status codes to retry on
    pub retry_on_status: Vec<u16>,
}

/// Pagination configuration
#[derive(Debug, Clone)]
pub struct PaginationConfig {
    /// Pagination style
    pub style: PaginationStyle,
    
    /// Parameter name for page number/offset
    pub page_param: String,
    
    /// Parameter name for page size/limit
    pub size_param: String,
    
    /// Default page size
    pub default_size: usize,
    
    /// Maximum page size
    pub max_size: usize,
    
    /// JSON path to data array in response
    pub data_path: String,
    
    /// JSON path to next page token/URL
    pub next_path: Option<String>,
    
    /// JSON path to total count
    pub total_path: Option<String>,
}

/// Pagination style
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PaginationStyle {
    /// Page number based (page=1, page=2, ...)
    PageNumber,
    
    /// Offset based (offset=0, offset=100, ...)
    Offset,
    
    /// Cursor/token based
    Cursor,
    
    /// Link header based (GitHub style)
    LinkHeader,
}

/// Endpoint configuration
#[derive(Debug, Clone)]
pub struct EndpointConfig {
    /// HTTP method
    pub method: Method,
    
    /// Endpoint path (relative to base URL)
    pub path: String,
    
    /// LinkML class name
    pub class_name: String,
    
    /// Query parameters
    pub query_params: HashMap<String, String>,
    
    /// Request body template (for POST/PUT)
    pub body_template: Option<Value>,
    
    /// Response data path (JSON path to array/object)
    pub response_data_path: Option<String>,
    
    /// ID field in response
    pub id_field: String,
}

impl Default for ApiOptions {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            auth: None,
            headers: HashMap::new(),
            timeout_seconds: 30,
            retry_config: RetryConfig::default(),
            pagination: None,
            endpoint_mapping: HashMap::new(),
            field_mapping: HashMap::new(),
            follow_redirects: true,
            user_agent: "LinkML-API-Loader/1.0".to_string(),
            rate_limit: None,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 10000,
            backoff_factor: 2.0,
            retry_on_status: vec![429, 500, 502, 503, 504],
        }
    }
}

/// API loader for LinkML data
pub struct ApiLoader {
    options: ApiOptions,
    client: Client,
    last_request_time: Option<std::time::Instant>,
}

impl ApiLoader {
    /// Create a new API loader
    pub fn new(options: ApiOptions) -> Self {
        let mut headers = HeaderMap::new();
        
        // Add custom headers
        for (name, value) in &options.headers {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                }
            }
        }
        
        // Build client
        let client = Client::builder()
            .default_headers(headers)
            .user_agent(&options.user_agent)
            .timeout(Duration::from_secs(options.timeout_seconds))
            .redirect(if options.follow_redirects {
                reqwest::redirect::Policy::default()
            } else {
                reqwest::redirect::Policy::none()
            })
            .build()
            .unwrap_or_default();
        
        Self {
            options,
            client,
            last_request_time: None,
        }
    }
    
    /// Apply rate limiting
    async fn apply_rate_limit(&mut self) {
        if let Some(rate_limit) = self.options.rate_limit {
            if let Some(last_time) = self.last_request_time {
                let elapsed = last_time.elapsed().as_secs_f64();
                let min_interval = 1.0 / rate_limit;
                
                if elapsed < min_interval {
                    let sleep_duration = Duration::from_secs_f64(min_interval - elapsed);
                    tokio::time::sleep(sleep_duration).await;
                }
            }
            
            self.last_request_time = Some(std::time::Instant::now());
        }
    }
    
    /// Build request with authentication
    fn build_request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.request(method, url);
        
        // Apply authentication
        if let Some(auth) = &self.options.auth {
            match auth {
                AuthConfig::Bearer(token) => {
                    request = request.bearer_auth(token);
                }
                AuthConfig::Basic { username, password } => {
                    request = request.basic_auth(username, Some(password));
                }
                AuthConfig::ApiKey { header_name, key } => {
                    request = request.header(header_name, key);
                }
                AuthConfig::OAuth2 { .. } => {
                    // OAuth2 would require token refresh logic
                    warn!("OAuth2 authentication not fully implemented");
                }
            }
        }
        
        request
    }
    
    /// Execute request with retry logic
    async fn execute_with_retry(&self, request: reqwest::RequestBuilder) -> LoaderResult<reqwest::Response> {
        let mut retries = 0;
        let mut delay = self.options.retry_config.initial_delay_ms;
        
        loop {
            let req = request.try_clone()
                .ok_or_else(|| LoaderError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to clone request"
                )))?;
            
            match req.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    
                    if response.status().is_success() {
                        return Ok(response);
                    }
                    
                    if self.options.retry_config.retry_on_status.contains(&status) &&
                       retries < self.options.retry_config.max_retries {
                        warn!("Request failed with status {}, retrying ({}/{})", 
                              status, retries + 1, self.options.retry_config.max_retries);
                        
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        
                        retries += 1;
                        delay = (delay as f64 * self.options.retry_config.backoff_factor) as u64;
                        delay = delay.min(self.options.retry_config.max_delay_ms);
                        
                        continue;
                    }
                    
                    return Err(LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Request failed with status: {}", status)
                    )));
                }
                Err(e) => {
                    if retries < self.options.retry_config.max_retries {
                        warn!("Request failed: {}, retrying ({}/{})", 
                              e, retries + 1, self.options.retry_config.max_retries);
                        
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        
                        retries += 1;
                        delay = (delay as f64 * self.options.retry_config.backoff_factor) as u64;
                        delay = delay.min(self.options.retry_config.max_delay_ms);
                        
                        continue;
                    }
                    
                    return Err(LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Request failed: {}", e)
                    )));
                }
            }
        }
    }
    
    /// Load data from a single endpoint
    async fn load_endpoint(&mut self, endpoint_name: &str, endpoint_config: &EndpointConfig, 
                          schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let mut all_instances = Vec::new();
        
        // Build initial URL
        let mut url = format!("{}{}", self.options.base_url, endpoint_config.path);
        
        // Add query parameters
        if !endpoint_config.query_params.is_empty() {
            let query = serde_urlencoded::to_string(&endpoint_config.query_params)
                .map_err(|e| LoaderError::ParseError(format!("Failed to encode query params: {}", e)))?;
            url.push('?');
            url.push_str(&query);
        }
        
        // Handle pagination
        if let Some(pagination) = &self.options.pagination {
            match pagination.style {
                PaginationStyle::PageNumber => {
                    let mut page = 1;
                    loop {
                        let page_url = format!("{}&{}={}&{}={}", 
                            url, 
                            pagination.page_param, page,
                            pagination.size_param, pagination.default_size
                        );
                        
                        let instances = self.fetch_page(&page_url, endpoint_config, schema).await?;
                        let instance_count = instances.len();
                        all_instances.extend(instances);
                        
                        if instance_count < pagination.default_size {
                            break;
                        }
                        
                        page += 1;
                    }
                }
                PaginationStyle::Offset => {
                    let mut offset = 0;
                    loop {
                        let page_url = format!("{}&{}={}&{}={}", 
                            url,
                            pagination.page_param, offset,
                            pagination.size_param, pagination.default_size
                        );
                        
                        let instances = self.fetch_page(&page_url, endpoint_config, schema).await?;
                        let instance_count = instances.len();
                        all_instances.extend(instances);
                        
                        if instance_count < pagination.default_size {
                            break;
                        }
                        
                        offset += pagination.default_size;
                    }
                }
                PaginationStyle::Cursor | PaginationStyle::LinkHeader => {
                    // These would require more complex implementation
                    warn!("Cursor and LinkHeader pagination not fully implemented");
                    let instances = self.fetch_page(&url, endpoint_config, schema).await?;
                    all_instances.extend(instances);
                }
            }
        } else {
            // No pagination, single request
            let instances = self.fetch_page(&url, endpoint_config, schema).await?;
            all_instances.extend(instances);
        }
        
        Ok(all_instances)
    }
    
    /// Fetch a single page of data
    async fn fetch_page(&mut self, url: &str, endpoint_config: &EndpointConfig, 
                       schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        // Apply rate limiting
        self.apply_rate_limit().await;
        
        // Build and execute request
        let request = self.build_request(endpoint_config.method.clone(), url);
        let response = self.execute_with_retry(request).await?;
        
        // Parse response
        let json: Value = response.json().await
            .map_err(|e| LoaderError::ParseError(format!("Failed to parse JSON response: {}", e)))?;
        
        // Extract data based on response path
        let data = if let Some(path) = &endpoint_config.response_data_path {
            self.extract_by_path(&json, path)?
        } else {
            json
        };
        
        // Convert to instances
        self.json_to_instances(data, &endpoint_config.class_name, endpoint_config, schema)
    }
    
    /// Extract data by JSON path
    fn extract_by_path(&self, json: &Value, path: &str) -> LoaderResult<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;
        
        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)
                        .ok_or_else(|| LoaderError::ParseError(
                            format!("Path '{}' not found in response", part)
                        ))?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)
                            .ok_or_else(|| LoaderError::ParseError(
                                format!("Array index {} out of bounds", index)
                            ))?;
                    } else {
                        return Err(LoaderError::ParseError(
                            format!("Invalid array index: {}", part)
                        ));
                    }
                }
                _ => {
                    return Err(LoaderError::ParseError(
                        format!("Cannot navigate path '{}' in non-object/array", part)
                    ));
                }
            }
        }
        
        Ok(current.clone())
    }
    
    /// Convert JSON to LinkML instances
    fn json_to_instances(&self, json: Value, class_name: &str, endpoint_config: &EndpointConfig,
                        schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let mut instances = Vec::new();
        
        match json {
            Value::Array(items) => {
                for item in items {
                    if let Value::Object(obj) = item {
                        let instance = self.object_to_instance(obj, class_name, endpoint_config)?;
                        instances.push(instance);
                    }
                }
            }
            Value::Object(obj) => {
                let instance = self.object_to_instance(obj, class_name, endpoint_config)?;
                instances.push(instance);
            }
            _ => {
                return Err(LoaderError::ParseError(
                    "Expected array or object in API response".to_string()
                ));
            }
        }
        
        Ok(instances)
    }
    
    /// Convert JSON object to instance
    fn object_to_instance(&self, mut obj: Map<String, Value>, class_name: &str, 
                         endpoint_config: &EndpointConfig) -> LoaderResult<DataInstance> {
        // Apply field mapping if configured
        if let Some(mapping) = self.options.field_mapping.get(class_name) {
            let mut mapped_obj = Map::new();
            
            for (api_field, linkml_field) in mapping {
                if let Some(value) = obj.remove(api_field) {
                    mapped_obj.insert(linkml_field.clone(), value);
                }
            }
            
            // Keep unmapped fields
            for (key, value) in obj {
                if !mapped_obj.contains_key(&key) {
                    mapped_obj.insert(key, value);
                }
            }
            
            obj = mapped_obj;
        }
        
        Ok(DataInstance {
            class_name: class_name.to_string(),
            data: obj,
        })
    }
}

#[async_trait]
impl DataLoader for ApiLoader {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        let mut all_instances = Vec::new();
        
        // Load from each configured endpoint
        for (endpoint_name, endpoint_config) in &self.options.endpoint_mapping.clone() {
            info!("Loading data from endpoint: {}", endpoint_name);
            
            let instances = self.load_endpoint(endpoint_name, endpoint_config, schema).await?;
            info!("Loaded {} instances from {}", instances.len(), endpoint_name);
            
            all_instances.extend(instances);
        }
        
        Ok(all_instances)
    }
}

/// API dumper for LinkML data
pub struct ApiDumper {
    options: ApiOptions,
    client: Client,
    last_request_time: Option<std::time::Instant>,
}

impl ApiDumper {
    /// Create a new API dumper
    pub fn new(options: ApiOptions) -> Self {
        let mut headers = HeaderMap::new();
        
        // Add custom headers
        for (name, value) in &options.headers {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes()) {
                if let Ok(header_value) = HeaderValue::from_str(value) {
                    headers.insert(header_name, header_value);
                }
            }
        }
        
        // Build client
        let client = Client::builder()
            .default_headers(headers)
            .user_agent(&options.user_agent)
            .timeout(Duration::from_secs(options.timeout_seconds))
            .build()
            .unwrap_or_default();
        
        Self {
            options,
            client,
            last_request_time: None,
        }
    }
    
    /// Apply rate limiting
    async fn apply_rate_limit(&mut self) {
        if let Some(rate_limit) = self.options.rate_limit {
            if let Some(last_time) = self.last_request_time {
                let elapsed = last_time.elapsed().as_secs_f64();
                let min_interval = 1.0 / rate_limit;
                
                if elapsed < min_interval {
                    let sleep_duration = Duration::from_secs_f64(min_interval - elapsed);
                    tokio::time::sleep(sleep_duration).await;
                }
            }
            
            self.last_request_time = Some(std::time::Instant::now());
        }
    }
    
    /// Build request with authentication
    fn build_request(&self, method: Method, url: &str) -> reqwest::RequestBuilder {
        let mut request = self.client.request(method, url);
        
        // Apply authentication
        if let Some(auth) = &self.options.auth {
            match auth {
                AuthConfig::Bearer(token) => {
                    request = request.bearer_auth(token);
                }
                AuthConfig::Basic { username, password } => {
                    request = request.basic_auth(username, Some(password));
                }
                AuthConfig::ApiKey { header_name, key } => {
                    request = request.header(header_name, key);
                }
                AuthConfig::OAuth2 { .. } => {
                    warn!("OAuth2 authentication not fully implemented");
                }
            }
        }
        
        request
    }
    
    /// Dump a single instance
    async fn dump_instance(&mut self, instance: &DataInstance, endpoint_config: &EndpointConfig) 
        -> DumperResult<()> {
        // Build URL
        let url = format!("{}{}", self.options.base_url, endpoint_config.path);
        
        // Prepare data
        let mut data = instance.data.clone();
        
        // Apply reverse field mapping if configured
        if let Some(mapping) = self.options.field_mapping.get(&instance.class_name) {
            let mut unmapped_data = Map::new();
            
            // Create reverse mapping
            let reverse_mapping: HashMap<&str, &str> = mapping.iter()
                .map(|(k, v)| (v.as_str(), k.as_str()))
                .collect();
            
            for (linkml_field, value) in data {
                if let Some(api_field) = reverse_mapping.get(linkml_field.as_str()) {
                    unmapped_data.insert(api_field.to_string(), value);
                } else {
                    unmapped_data.insert(linkml_field, value);
                }
            }
            
            data = unmapped_data;
        }
        
        // Apply rate limiting
        self.apply_rate_limit().await;
        
        // Send request based on method
        let request = match endpoint_config.method {
            Method::POST => {
                self.build_request(Method::POST, &url)
                    .json(&data)
            }
            Method::PUT => {
                // For PUT, we might need to include ID in URL
                if let Some(id_value) = data.get(&endpoint_config.id_field) {
                    let url_with_id = format!("{}/{}", url, id_value);
                    self.build_request(Method::PUT, &url_with_id)
                        .json(&data)
                } else {
                    self.build_request(Method::PUT, &url)
                        .json(&data)
                }
            }
            Method::PATCH => {
                // Similar to PUT
                if let Some(id_value) = data.get(&endpoint_config.id_field) {
                    let url_with_id = format!("{}/{}", url, id_value);
                    self.build_request(Method::PATCH, &url_with_id)
                        .json(&data)
                } else {
                    self.build_request(Method::PATCH, &url)
                        .json(&data)
                }
            }
            _ => {
                return Err(DumperError::UnsupportedFormat(
                    format!("Unsupported HTTP method for dumping: {}", endpoint_config.method)
                ));
            }
        };
        
        let response = request.send().await
            .map_err(|e| DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to send request: {}", e)
            )))?;
        
        if !response.status().is_success() {
            return Err(DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Request failed with status: {}", response.status())
            )));
        }
        
        Ok(())
    }
}

#[async_trait]
impl DataDumper for ApiDumper {
    async fn dump(&mut self, instances: &[DataInstance], schema: &SchemaDefinition) -> DumperResult<Vec<u8>> {
        let mut success_count = 0;
        let mut error_count = 0;
        
        // Group instances by class
        let mut instances_by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in instances {
            instances_by_class.entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }
        
        // Dump instances for each configured endpoint
        for (class_name, class_instances) in instances_by_class {
            // Find endpoint configuration for this class
            let endpoint_config = self.options.endpoint_mapping.values()
                .find(|ec| ec.class_name == class_name);
            
            if let Some(config) = endpoint_config {
                for instance in class_instances {
                    match self.dump_instance(instance, config).await {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            error!("Failed to dump instance: {}", e);
                            error_count += 1;
                        }
                    }
                }
            } else {
                warn!("No endpoint configured for class: {}", class_name);
                error_count += class_instances.len();
            }
        }
        
        let summary = format!(
            "API dump complete: {} successful, {} failed",
            success_count, error_count
        );
        
        if error_count > 0 {
            Err(DumperError::ValidationFailed(summary))
        } else {
            Ok(summary.into_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_api_options_default() {
        let options = ApiOptions::default();
        
        assert_eq!(options.timeout_seconds, 30);
        assert!(options.follow_redirects);
        assert_eq!(options.user_agent, "LinkML-API-Loader/1.0");
        assert!(options.auth.is_none());
    }
    
    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_delay_ms, 100);
        assert_eq!(config.backoff_factor, 2.0);
        assert!(config.retry_on_status.contains(&429));
        assert!(config.retry_on_status.contains(&503));
    }
    
    #[test]
    fn test_auth_config() {
        let bearer = AuthConfig::Bearer("token123".to_string());
        let basic = AuthConfig::Basic {
            username: "user".to_string(),
            password: "pass".to_string(),
        };
        let api_key = AuthConfig::ApiKey {
            header_name: "X-API-Key".to_string(),
            key: "key123".to_string(),
        };
        
        match bearer {
            AuthConfig::Bearer(token) => assert_eq!(token, "token123"),
            _ => panic!("Wrong auth type"),
        }
        
        match basic {
            AuthConfig::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, "pass");
            }
            _ => panic!("Wrong auth type"),
        }
        
        match api_key {
            AuthConfig::ApiKey { header_name, key } => {
                assert_eq!(header_name, "X-API-Key");
                assert_eq!(key, "key123");
            }
            _ => panic!("Wrong auth type"),
        }
    }
    
    #[test]
    fn test_pagination_config() {
        let config = PaginationConfig {
            style: PaginationStyle::PageNumber,
            page_param: "page".to_string(),
            size_param: "per_page".to_string(),
            default_size: 100,
            max_size: 1000,
            data_path: "data".to_string(),
            next_path: Some("next".to_string()),
            total_path: Some("total".to_string()),
        };
        
        assert_eq!(config.style, PaginationStyle::PageNumber);
        assert_eq!(config.page_param, "page");
        assert_eq!(config.default_size, 100);
    }
    
    #[test]
    fn test_endpoint_config() {
        let config = EndpointConfig {
            method: Method::GET,
            path: "/api/v1/users".to_string(),
            class_name: "User".to_string(),
            query_params: [("active".to_string(), "true".to_string())].into(),
            body_template: None,
            response_data_path: Some("data.users".to_string()),
            id_field: "id".to_string(),
        };
        
        assert_eq!(config.method, Method::GET);
        assert_eq!(config.path, "/api/v1/users");
        assert_eq!(config.class_name, "User");
        assert_eq!(config.query_params.get("active"), Some(&"true".to_string()));
    }
    
    #[tokio::test]
    async fn test_loader_creation() {
        let mut options = ApiOptions::default();
        options.base_url = "https://api.example.com".to_string();
        options.headers.insert("Accept".to_string(), "application/json".to_string());
        
        let loader = ApiLoader::new(options);
        assert!(loader.last_request_time.is_none());
    }
    
    #[tokio::test]
    async fn test_json_path_extraction() {
        let options = ApiOptions::default();
        let loader = ApiLoader::new(options);
        
        let json = serde_json::json!({
            "data": {
                "users": [
                    {"id": 1, "name": "Alice"},
                    {"id": 2, "name": "Bob"}
                ]
            }
        });
        
        let extracted = loader.extract_by_path(&json, "data.users").unwrap();
        assert!(extracted.is_array());
        assert_eq!(extracted.as_array().unwrap().len(), 2);
    }
}