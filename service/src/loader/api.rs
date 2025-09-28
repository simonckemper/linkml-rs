//! API loader and dumper for `LinkML`
//!
//! This module provides functionality to load data from REST APIs
//! and dump `LinkML` instances to API endpoints.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumperError, DumperResult, LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use regex::Regex;
use reqwest::{
    Client, Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{error, info, warn};

/// Options for `API` loading and dumping
#[derive(Debug, Clone)]
pub struct ApiOptions {
    /// Base `URL` for the `API`
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

    /// Endpoint to `LinkML` class mapping
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
    Basic {
        /// Username for authentication
        username: String,
        /// Password for authentication
        password: String,
    },

    /// `API` key authentication
    ApiKey {
        /// `HTTP` header name for API key
        header_name: String,
        /// `API` key value
        key: String,
    },

    /// `OAuth2` configuration
    OAuth2 {
        /// Token endpoint `URL`
        token_url: String,
        /// `OAuth2` client ID
        client_id: String,
        /// `OAuth2` client secret
        client_secret: String,
        /// Required scopes
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

    /// `HTTP` status codes to retry on
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

    /// `JSON` path to data array in response
    pub data_path: String,

    /// `JSON` path to next page token/URL
    pub next_path: Option<String>,

    /// `JSON` path to total count
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
    /// `HTTP` method
    pub method: Method,

    /// Endpoint path (relative to base `URL`)
    pub path: String,

    /// `LinkML` class name
    pub class_name: String,

    /// Query parameters
    pub query_params: HashMap<String, String>,

    /// Request body template (for POST/PUT)
    pub body_template: Option<Value>,

    /// Response data path (`JSON` path to array/object)
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

/// `API` loader for `LinkML` data
pub struct ApiLoader {
    options: ApiOptions,
    client: Client,
    last_request_time: std::sync::Mutex<Option<std::time::Instant>>,
}

impl ApiLoader {
    /// Create a new `API` loader
    #[must_use]
    pub fn new(options: ApiOptions) -> Self {
        let mut headers = HeaderMap::new();

        // Add custom headers
        for (name, value) in &options.headers {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes())
                && let Ok(header_value) = HeaderValue::from_str(value)
            {
                headers.insert(header_name, header_value);
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
            last_request_time: std::sync::Mutex::new(None),
        }
    }

    /// Apply rate limiting
    async fn apply_rate_limit(&self) {
        if let Some(rate_limit) = self.options.rate_limit {
            // Calculate sleep duration if needed
            let sleep_duration = {
                let last_time = self
                    .last_request_time
                    .lock()
                    .map_err(|_| "Mutex poisoned")
                    .expect("Mutex lock for rate limiting");

                if let Some(last) = *last_time {
                    let elapsed = last.elapsed().as_secs_f64();
                    let min_interval = 1.0 / rate_limit;

                    if elapsed < min_interval {
                        Some(Duration::from_secs_f64(min_interval - elapsed))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }; // MutexGuard is dropped here

            // Sleep if needed
            if let Some(duration) = sleep_duration {
                tokio::time::sleep(duration).await;
            }

            // Update last request time
            let mut last_time = self
                .last_request_time
                .lock()
                .map_err(|_| "Mutex poisoned")
                .expect("Mutex lock for rate limiting");
            *last_time = Some(std::time::Instant::now());
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
    async fn execute_with_retry(
        &self,
        request: reqwest::RequestBuilder,
    ) -> LoaderResult<reqwest::Response> {
        let mut retries = 0;
        let mut delay = self.options.retry_config.initial_delay_ms;

        loop {
            let req = request
                .try_clone()
                .ok_or_else(|| LoaderError::Io(std::io::Error::other("Failed to clone request")))?;

            match req.send().await {
                Ok(response) => {
                    let status = response.status().as_u16();

                    if response.status().is_success() {
                        return Ok(response);
                    }

                    if self.options.retry_config.retry_on_status.contains(&status)
                        && retries < self.options.retry_config.max_retries
                    {
                        warn!(
                            "Request failed with status {}, retrying ({}/{})",
                            status,
                            retries + 1,
                            self.options.retry_config.max_retries
                        );

                        tokio::time::sleep(Duration::from_millis(delay)).await;

                        retries += 1;
                        delay = (delay as f64 * self.options.retry_config.backoff_factor) as u64;
                        delay = delay.min(self.options.retry_config.max_delay_ms);

                        continue;
                    }

                    return Err(LoaderError::Io(std::io::Error::other(format!(
                        "Request failed with status: {status}"
                    ))));
                }
                Err(e) => {
                    if retries < self.options.retry_config.max_retries {
                        warn!(
                            "Request failed: {}, retrying ({}/{})",
                            e,
                            retries + 1,
                            self.options.retry_config.max_retries
                        );

                        tokio::time::sleep(Duration::from_millis(delay)).await;

                        retries += 1;
                        delay = (delay as f64 * self.options.retry_config.backoff_factor) as u64;
                        delay = delay.min(self.options.retry_config.max_delay_ms);

                        continue;
                    }

                    return Err(LoaderError::Io(std::io::Error::other(format!(
                        "Request failed: {e}"
                    ))));
                }
            }
        }
    }

    /// Load data from a single endpoint
    async fn load_endpoint(
        &self,
        endpoint_name: &str,
        endpoint_config: &EndpointConfig,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        tracing::debug!("Loading data from endpoint: {}", endpoint_name);
        let mut all_instances = Vec::new();

        // Build initial URL
        let mut url = format!("{}{}", self.options.base_url, endpoint_config.path);

        // Add query parameters
        if !endpoint_config.query_params.is_empty() {
            let query = serde_urlencoded::to_string(&endpoint_config.query_params)
                .map_err(|e| LoaderError::Parse(format!("Failed to encode query params: {e}")))?;
            url.push('?');
            url.push_str(&query);
        }

        // Handle pagination
        if let Some(pagination) = &self.options.pagination {
            match pagination.style {
                PaginationStyle::PageNumber => {
                    let mut page = 1;
                    loop {
                        let page_url = format!(
                            "{}&{}={}&{}={}",
                            url,
                            pagination.page_param,
                            page,
                            pagination.size_param,
                            pagination.default_size
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
                        let page_url = format!(
                            "{}&{}={}&{}={}",
                            url,
                            pagination.page_param,
                            offset,
                            pagination.size_param,
                            pagination.default_size
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
    async fn fetch_page(
        &self,
        url: &str,
        endpoint_config: &EndpointConfig,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Apply rate limiting
        self.apply_rate_limit().await;

        // Build and execute request
        let request = self.build_request(endpoint_config.method.clone(), url);
        let response = self.execute_with_retry(request).await?;

        // Parse response
        let json: Value = response
            .json()
            .await
            .map_err(|e| LoaderError::Parse(format!("Failed to parse JSON response: {e}")))?;

        // Extract data based on response path
        let data = if let Some(path) = &endpoint_config.response_data_path {
            self.extract_by_path(&json, path)?
        } else {
            json
        };

        // Convert to instances
        self.json_to_instances(data, &endpoint_config.class_name, endpoint_config, schema)
    }

    /// Extract data by `JSON` path
    fn extract_by_path(&self, json: &Value, path: &str) -> LoaderResult<Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part).ok_or_else(|| {
                        LoaderError::Parse(format!("Path '{part}' not found in response"))
                    })?;
                }
                Value::Array(arr) => {
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index).ok_or_else(|| {
                            LoaderError::Parse(format!("Array index {index} out of bounds"))
                        })?;
                    } else {
                        return Err(LoaderError::Parse(format!("Invalid array index: {part}")));
                    }
                }
                _ => {
                    return Err(LoaderError::Parse(format!(
                        "Cannot navigate path '{part}' in non-object/array"
                    )));
                }
            }
        }

        Ok(current.clone())
    }

    /// Convert `JSON` to `LinkML` instances
    fn json_to_instances(
        &self,
        json: Value,
        class_name: &str,
        endpoint_config: &EndpointConfig,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let mut instances = Vec::new();

        // Check if the class exists in the schema
        if !schema.classes.contains_key(class_name) {
            return Err(LoaderError::Parse(format!(
                "Class '{class_name}' not found in schema"
            )));
        }

        match json {
            Value::Array(items) => {
                for (index, item) in items.into_iter().enumerate() {
                    if let Value::Object(obj) = item {
                        let instance = self.object_to_instance(obj, class_name, endpoint_config)?;

                        // Validate the instance against the schema
                        if let Err(e) = self.validate_instance(&instance, class_name, schema) {
                            return Err(LoaderError::Parse(format!(
                                "Instance at index {index} failed validation: {e}"
                            )));
                        }

                        instances.push(instance);
                    }
                }
            }
            Value::Object(obj) => {
                let instance = self.object_to_instance(obj, class_name, endpoint_config)?;

                // Validate the instance against the schema
                if let Err(e) = self.validate_instance(&instance, class_name, schema) {
                    return Err(LoaderError::Parse(format!(
                        "Instance failed validation: {e}"
                    )));
                }

                instances.push(instance);
            }
            _ => {
                return Err(LoaderError::Parse(
                    "Expected array or object in API response".to_string(),
                ));
            }
        }

        Ok(instances)
    }

    /// Validate an instance against the schema
    fn validate_instance(
        &self,
        instance: &DataInstance,
        class_name: &str,
        schema: &SchemaDefinition,
    ) -> LoaderResult<()> {
        // Get the class definition
        let class_def = schema.classes.get(class_name).ok_or_else(|| {
            LoaderError::Parse(format!("Class '{class_name}' not found in schema"))
        })?;

        // Validate required fields
        for (slot_name, slot_def) in &class_def.attributes {
            if slot_def.required.unwrap_or(false) && !instance.data.contains_key(slot_name) {
                return Err(LoaderError::Parse(format!(
                    "Required field '{slot_name}' missing in class '{class_name}'"
                )));
            }
        }

        // Validate slot definitions if referenced
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name)
                && slot_def.required.unwrap_or(false)
                && !instance.data.contains_key(slot_name)
            {
                return Err(LoaderError::Parse(format!(
                    "Required slot '{slot_name}' missing in class '{class_name}'"
                )));
            }
        }

        // Validate data types and constraints for each field
        for (field_name, field_value) in &instance.data {
            // Check if field is defined as attribute
            if let Some(slot_def) = class_def.attributes.get(field_name) {
                self.validate_field(field_name, field_value, slot_def)?;
            }
            // Check if field is defined as a slot reference
            else if class_def.slots.contains(field_name)
                && let Some(slot_def) = schema.slots.get(field_name)
            {
                self.validate_field(field_name, field_value, slot_def)?;
            }
            // Field not defined in schema - this might be allowed depending on schema settings
            // For now, we'll allow extra fields but could make this configurable
        }

        Ok(())
    }

    /// Validate a field value against its slot definition
    fn validate_field(
        &self,
        field_name: &str,
        value: &Value,
        slot_def: &SlotDefinition,
    ) -> LoaderResult<()> {
        // Check pattern if defined
        if let Some(pattern) = &slot_def.pattern
            && let Value::String(s) = value
        {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(s) {
                        return Err(LoaderError::Parse(format!(
                            "Field '{field_name}' value '{s}' does not match pattern '{pattern}'"
                        )));
                    }
                }
                Err(e) => {
                    warn!(
                        "Invalid regex pattern '{}' in slot '{}': {}",
                        pattern, field_name, e
                    );
                    // Continue validation without pattern check
                }
            }
        }

        // Check minimum value
        if let Some(min_val) = &slot_def.minimum_value {
            if let (Value::Number(n), Value::Number(min_n)) = (value, min_val) {
                if let (Some(v), Some(min_v)) = (n.as_f64(), min_n.as_f64())
                    && v < min_v
                {
                    return Err(LoaderError::Parse(format!(
                        "Field '{field_name}' value {v} is less than minimum {min_v}"
                    )));
                }
            } else {
                // Skip validation if types don't match
            }
        }

        // Check maximum value
        if let Some(max_val) = &slot_def.maximum_value {
            if let (Value::Number(n), Value::Number(max_n)) = (value, max_val) {
                if let (Some(v), Some(max_v)) = (n.as_f64(), max_n.as_f64())
                    && v > max_v
                {
                    return Err(LoaderError::Parse(format!(
                        "Field '{field_name}' value {v} is greater than maximum {max_v}"
                    )));
                }
            } else {
                // Skip validation if types don't match
            }
        }

        // Check multivalued constraint
        if let Some(multivalued) = slot_def.multivalued {
            match (value, multivalued) {
                (Value::Array(_), false) => {
                    return Err(LoaderError::Parse(format!(
                        "Field '{field_name}' should not be multivalued but is an array"
                    )));
                }
                (Value::Array(arr), true)
                    if arr.is_empty() && slot_def.required.unwrap_or(false) =>
                {
                    return Err(LoaderError::Parse(format!(
                        "Field '{field_name}' is required and multivalued but is empty"
                    )));
                }
                (v, true)
                    if !matches!(v, Value::Array(_)) && slot_def.required.unwrap_or(false) =>
                {
                    return Err(LoaderError::Parse(format!(
                        "Field '{field_name}' should be multivalued (array) but is not"
                    )));
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Convert `JSON` object to instance
    fn object_to_instance(
        &self,
        mut obj: Map<String, Value>,
        class_name: &str,
        _endpoint_config: &EndpointConfig,
    ) -> LoaderResult<DataInstance> {
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
            data: obj.into_iter().collect(),
            id: None,
            metadata: HashMap::new(),
        })
    }
}

#[async_trait]
impl DataLoader for ApiLoader {
    fn name(&self) -> &'static str {
        "api"
    }

    fn description(&self) -> &'static str {
        "Load data from REST APIs"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // API loader doesn't use file extensions
    }

    async fn load_file(
        &self,
        _path: &std::path::Path,
        _schema: &SchemaDefinition,
        _options: &super::traits::LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::InvalidFormat(
            "API loader does not support file loading".to_string(),
        ))
    }

    async fn load_string(
        &self,
        _content: &str,
        schema: &SchemaDefinition,
        _options: &super::traits::LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Load from all configured endpoints
        let mut all_instances = Vec::new();

        for (endpoint_name, endpoint_config) in &self.options.endpoint_mapping {
            info!("Loading data from endpoint: {}", endpoint_name);

            let instances = self
                .load_endpoint(endpoint_name, endpoint_config, schema)
                .await?;
            info!(
                "Loaded {} instances from {}",
                instances.len(),
                endpoint_name
            );

            all_instances.extend(instances);
        }

        Ok(all_instances)
    }

    async fn load_bytes(
        &self,
        _data: &[u8],
        _schema: &SchemaDefinition,
        _options: &super::traits::LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::InvalidFormat(
            "API loader does not support raw bytes loading".to_string(),
        ))
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> LoaderResult<()> {
        // Could validate that schema classes match endpoint configurations
        Ok(())
    }
}

/// `API` dumper for `LinkML` data
pub struct ApiDumper {
    options: ApiOptions,
    client: Client,
    last_request_time: std::sync::Mutex<Option<std::time::Instant>>,
}

impl ApiDumper {
    /// Create a new `API` dumper
    #[must_use]
    pub fn new(options: ApiOptions) -> Self {
        let mut headers = HeaderMap::new();

        // Add custom headers
        for (name, value) in &options.headers {
            if let Ok(header_name) = HeaderName::from_bytes(name.as_bytes())
                && let Ok(header_value) = HeaderValue::from_str(value)
            {
                headers.insert(header_name, header_value);
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
            last_request_time: std::sync::Mutex::new(None),
        }
    }

    /// Apply rate limiting
    async fn apply_rate_limit(&self) {
        if let Some(rate_limit) = self.options.rate_limit {
            // Calculate sleep duration if needed
            let sleep_duration = {
                let last_time = self
                    .last_request_time
                    .lock()
                    .map_err(|_| "Mutex poisoned")
                    .expect("Mutex lock for rate limiting");

                if let Some(last) = *last_time {
                    let elapsed = last.elapsed().as_secs_f64();
                    let min_interval = 1.0 / rate_limit;

                    if elapsed < min_interval {
                        Some(Duration::from_secs_f64(min_interval - elapsed))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }; // MutexGuard is dropped here

            // Sleep if needed
            if let Some(duration) = sleep_duration {
                tokio::time::sleep(duration).await;
            }

            // Update last request time
            let mut last_time = self
                .last_request_time
                .lock()
                .map_err(|_| "Mutex poisoned")
                .expect("Mutex lock for rate limiting");
            *last_time = Some(std::time::Instant::now());
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
    async fn dump_instance(
        &self,
        instance: &DataInstance,
        endpoint_config: &EndpointConfig,
    ) -> DumperResult<()> {
        // Build URL
        let url = format!("{}{}", self.options.base_url, endpoint_config.path);

        // Prepare data
        let mut data = instance.data.clone();

        // Apply reverse field mapping if configured
        if let Some(mapping) = self.options.field_mapping.get(&instance.class_name) {
            let mut unmapped_data = Map::new();

            // Create reverse mapping
            let reverse_mapping: HashMap<&str, &str> = mapping
                .iter()
                .map(|(k, v)| (v.as_str(), k.as_str()))
                .collect();

            for (linkml_field, value) in data {
                if let Some(api_field) = reverse_mapping.get(linkml_field.as_str()) {
                    unmapped_data.insert((*api_field).to_string(), value);
                } else {
                    unmapped_data.insert(linkml_field, value);
                }
            }

            data = unmapped_data.into_iter().collect();
        }

        // Apply rate limiting
        self.apply_rate_limit().await;

        // Send request based on method
        let request = match endpoint_config.method {
            Method::POST => self.build_request(Method::POST, &url).json(&data),
            Method::PUT => {
                // For PUT, we might need to include ID in URL
                if let Some(id_value) = data.get(&endpoint_config.id_field) {
                    let url_with_id = format!("{url}/{id_value}");
                    self.build_request(Method::PUT, &url_with_id).json(&data)
                } else {
                    self.build_request(Method::PUT, &url).json(&data)
                }
            }
            Method::PATCH => {
                // Similar to PUT
                if let Some(id_value) = data.get(&endpoint_config.id_field) {
                    let url_with_id = format!("{url}/{id_value}");
                    self.build_request(Method::PATCH, &url_with_id).json(&data)
                } else {
                    self.build_request(Method::PATCH, &url).json(&data)
                }
            }
            _ => {
                return Err(DumperError::Configuration(format!(
                    "Unsupported HTTP method for dumping: {}",
                    endpoint_config.method
                )));
            }
        };

        let response = request.send().await.map_err(|e| {
            DumperError::Io(std::io::Error::other(format!(
                "Failed to send request: {e}"
            )))
        })?;

        if !response.status().is_success() {
            return Err(DumperError::Io(std::io::Error::other(format!(
                "Request failed with status: {}",
                response.status()
            ))));
        }

        Ok(())
    }
}

#[async_trait]
impl DataDumper for ApiDumper {
    fn name(&self) -> &'static str {
        "api"
    }

    fn description(&self) -> &'static str {
        "Dump data to REST APIs"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // API dumper doesn't use file extensions
    }

    async fn dump_file(
        &self,
        _instances: &[DataInstance],
        _path: &std::path::Path,
        _schema: &SchemaDefinition,
        _options: &super::traits::DumpOptions,
    ) -> DumperResult<()> {
        Err(DumperError::Configuration(
            "API dumper does not support file dumping".to_string(),
        ))
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &super::traits::DumpOptions,
    ) -> DumperResult<String> {
        let mut success_count = 0;
        let mut error_count = 0;

        // Validate schema compatibility first
        self.validate_schema(schema)?;

        // Apply options for filtering and formatting
        let filtered_instances: Vec<&DataInstance> = instances
            .iter()
            .filter(|instance| {
                // Use options to filter instances if specified
                if let Some(ref class_filter) = options.include_classes {
                    class_filter.contains(&instance.class_name)
                } else {
                    true
                }
            })
            .collect();

        // Group instances by class
        let mut instances_by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in filtered_instances {
            instances_by_class
                .entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }

        // Dump instances for each configured endpoint
        for (class_name, class_instances) in instances_by_class {
            // Find endpoint configuration for this class
            let endpoint_config = self
                .options
                .endpoint_mapping
                .values()
                .find(|ec| ec.class_name == class_name);

            if let Some(config) = endpoint_config {
                for instance in class_instances {
                    match self.dump_instance(instance, config).await {
                        Ok(()) => success_count += 1,
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

        let summary =
            format!("API dump complete: {success_count} successful, {error_count} failed");

        if error_count > 0 {
            Err(DumperError::Serialization(summary))
        } else {
            Ok(summary)
        }
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &super::traits::DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let result = self.dump_string(instances, schema, options).await?;
        Ok(result.into_bytes())
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        // Validate that schema classes match endpoint configurations
        for endpoint_config in self.options.endpoint_mapping.values() {
            if !schema.classes.contains_key(&endpoint_config.class_name) {
                return Err(DumperError::Configuration(format!(
                    "Class '{}' referenced in endpoint configuration not found in schema",
                    endpoint_config.class_name
                )));
            }
        }
        Ok(())
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

        if let AuthConfig::Bearer(token) = bearer {
            assert_eq!(token, "token123");
        } else {
            assert!(false, "Expected Bearer variant");
        }

        if let AuthConfig::Basic { username, password } = basic {
            assert_eq!(username, "user");
            assert_eq!(password, "pass");
        } else {
            assert!(false, "Expected Basic variant");
        }

        if let AuthConfig::ApiKey { header_name, key } = api_key {
            assert_eq!(header_name, "X-API-Key");
            assert_eq!(key, "key123");
        } else {
            assert!(false, "Expected ApiKey variant");
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
        options
            .headers
            .insert("Accept".to_string(), "application/json".to_string());

        let loader = ApiLoader::new(options);
        assert!(
            loader
                .last_request_time
                .lock()
                .expect("Should lock mutex")
                .is_none()
        );
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

        let extracted = loader
            .extract_by_path(&json, "data.users")
            .expect("should extract data.users");
        assert!(extracted.is_array());
        assert_eq!(extracted.as_array().expect("should be array").len(), 2);
    }

    #[test]
    fn test_validate_instance() {
        use indexmap::IndexMap;
        use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

        // Create test schema
        let mut schema = SchemaDefinition::default();

        // Create Person class with required fields
        let mut person_class = ClassDefinition::default();
        let mut attributes = IndexMap::new();

        // Add required name field
        let mut name_slot = SlotDefinition::default();
        name_slot.required = Some(true);
        name_slot.range = Some("string".to_string());
        attributes.insert("name".to_string(), name_slot);

        // Add age field with min/max constraints
        let mut age_slot = SlotDefinition::default();
        age_slot.minimum_value = Some(serde_json::json!(0));
        age_slot.maximum_value = Some(serde_json::json!(150));
        age_slot.range = Some("integer".to_string());
        attributes.insert("age".to_string(), age_slot);

        // Add email field with pattern
        let mut email_slot = SlotDefinition::default();
        email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2}$".to_string());
        email_slot.range = Some("string".to_string());
        attributes.insert("email".to_string(), email_slot);

        person_class.attributes = attributes;
        schema.classes.insert("Person".to_string(), person_class);

        let options = ApiOptions::default();
        let loader = ApiLoader::new(options);

        // Test valid instance
        let valid_instance = DataInstance {
            class_name: "Person".to_string(),
            data: vec![
                ("name".to_string(), serde_json::json!("Alice")),
                ("age".to_string(), serde_json::json!(30)),
                ("email".to_string(), serde_json::json!("alice@example.com")),
            ]
            .into_iter()
            .collect(),
            id: None,
            metadata: HashMap::new(),
        };

        assert!(
            loader
                .validate_instance(&valid_instance, "Person", &schema)
                .is_ok()
        );

        // Test missing required field
        let missing_name = DataInstance {
            class_name: "Person".to_string(),
            data: vec![("age".to_string(), serde_json::json!(30))]
                .into_iter()
                .collect(),
            id: None,
            metadata: HashMap::new(),
        };

        let result = loader.validate_instance(&missing_name, "Person", &schema);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Required field 'name' missing")
        );

        // Test invalid age
        let invalid_age = DataInstance {
            class_name: "Person".to_string(),
            data: vec![
                ("name".to_string(), serde_json::json!("Bob")),
                ("age".to_string(), serde_json::json!(200)),
            ]
            .into_iter()
            .collect(),
            id: None,
            metadata: HashMap::new(),
        };

        let result = loader.validate_instance(&invalid_age, "Person", &schema);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("greater than maximum")
        );

        // Test invalid email pattern
        let invalid_email = DataInstance {
            class_name: "Person".to_string(),
            data: vec![
                ("name".to_string(), serde_json::json!("Charlie")),
                ("email".to_string(), serde_json::json!("not-an-email")),
            ]
            .into_iter()
            .collect(),
            id: None,
            metadata: HashMap::new(),
        };

        let result = loader.validate_instance(&invalid_email, "Person", &schema);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("does not match pattern")
        );
    }
}
