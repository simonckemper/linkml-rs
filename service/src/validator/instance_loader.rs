//! Instance-based validation support
//!
//! Loads permissible values from external data sources

use linkml_core::error::{LinkMLError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use rootreal_core_foundation_timestamp_core::TimestampService;
use rootreal_core_foundation_timestamp::factory;

/// Instance data for permissible values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceData {
    /// Map of keys to permissible values
    pub values: HashMap<String, Vec<String>>,
    /// Source of the instance data
    pub source: String,
    /// Timestamp when loaded
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

/// Configuration for instance-based validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceConfig {
    /// Key field in the data (e.g., "code", "id")
    pub key_field: String,
    /// Value field in the data (e.g., "name", "label")
    pub value_field: Option<String>,
    /// Filter expression (future enhancement)
    pub filter: Option<String>,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            key_field: "id".to_string(),
            value_field: None,
            filter: None,
        }
    }
}

impl InstanceConfig {
    /// Check if the configuration is valid
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.key_field.is_empty()
    }
}

/// Loads instance data from various sources
pub struct InstanceLoader {
    /// Cache of loaded instance data
    cache: dashmap::DashMap<String, Arc<InstanceData>>,
    /// Timestamp service for `loaded_at` timestamps
    timestamp_service: Arc<dyn TimestampService<Error = rootreal_core_foundation_timestamp_core::TimestampError>>,
}

impl InstanceLoader {
    /// Create a new instance loader
    #[must_use]
    pub fn new(
        timestamp_service: Arc<dyn TimestampService<Error = rootreal_core_foundation_timestamp_core::TimestampError>>,
    ) -> Self {
        Self {
            cache: dashmap::DashMap::new(),
            timestamp_service,
        }
    }

    /// Load instance data from a `JSON` file
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub async fn load_json_file(
        &self,
        path: impl AsRef<Path>,
        config: &InstanceConfig,
    ) -> linkml_core::error::Result<Arc<InstanceData>> {
        let path = path.as_ref();
        let cache_key = format!("file:{}", path.display());

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(Arc::clone(&cached));
        }

        // Read and parse file
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(LinkMLError::from)?;

        let json: Value = serde_json::from_str(&content)
            .map_err(|e| LinkMLError::parse(format!("Invalid JSON in instance file: {e}")))?;

        // Extract values based on config
        let values = Self::extract_values_from_json(&json, config)?;

        let loaded_at =
            self.timestamp_service.now_utc().await.map_err(|e| {
                LinkMLError::service(format!("Failed to get current timestamp: {e}"))
            })?;

        let instance_data = Arc::new(InstanceData {
            values,
            source: cache_key.clone(),
            loaded_at,
        });

        // Cache the result
        self.cache.insert(cache_key, Arc::clone(&instance_data));
        Ok(instance_data)
    }

    /// Load instance data from a CSV file
    ///
    /// # Errors
    ///
    /// Returns an error if the CSV file cannot be read or parsed.
    pub async fn load_csv_file(
        &self,
        path: impl AsRef<Path>,
        config: &InstanceConfig,
    ) -> linkml_core::error::Result<Arc<InstanceData>> {
        let path = path.as_ref();
        let cache_key = format!("file:{}", path.display());

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(Arc::clone(&cached));
        }

        // Read CSV file
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(LinkMLError::from)?;

        let mut reader = csv::Reader::from_reader(content.as_bytes());
        let headers = reader
            .headers()
            .map_err(|e| LinkMLError::parse(format!("Failed to read CSV headers: {e}")))?
            .clone();

        // Find column indices
        let key_idx = headers
            .iter()
            .position(|h| h == config.key_field.as_str())
            .ok_or_else(|| {
                LinkMLError::data_validation(format!(
                    "Key field '{}' not found in CSV",
                    config.key_field
                ))
            })?;

        let value_idx = if let Some(value_field) = &config.value_field {
            Some(
                headers
                    .iter()
                    .position(|h| h == value_field)
                    .ok_or_else(|| {
                        LinkMLError::data_validation(format!(
                            "Value field '{value_field}' not found in CSV"
                        ))
                    })?,
            )
        } else {
            None
        };

        // Extract values
        let mut values: HashMap<String, Vec<String>> = HashMap::new();

        for result in reader.records() {
            let record = result
                .map_err(|e| LinkMLError::parse(format!("Failed to read CSV record: {e}")))?;

            let key = record
                .get(key_idx)
                .ok_or_else(|| LinkMLError::parse("Missing key field in CSV record"))?
                .to_string();

            let value = if let Some(idx) = value_idx {
                record
                    .get(idx)
                    .ok_or_else(|| LinkMLError::parse("Missing value field in CSV record"))?
                    .to_string()
            } else {
                key.clone()
            };

            values.entry(key).or_default().push(value);
        }

        let loaded_at =
            self.timestamp_service.now_utc().await.map_err(|e| {
                LinkMLError::service(format!("Failed to get current timestamp: {e}"))
            })?;

        let instance_data = Arc::new(InstanceData {
            values,
            source: cache_key.clone(),
            loaded_at,
        });

        // Cache the result
        self.cache.insert(cache_key, Arc::clone(&instance_data));
        Ok(instance_data)
    }

    /// Extract values from `JSON` based on configuration
    ///
    /// # Errors
    ///
    /// Returns error if JSON path extraction fails, if the JSON structure doesn't
    /// match the configuration requirements, or if type conversion fails.
    fn extract_values_from_json(
        json: &Value,
        config: &InstanceConfig,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut values: HashMap<String, Vec<String>> = HashMap::new();

        // Handle array of objects
        if let Some(array) = json.as_array() {
            for item in array {
                Self::extract_from_object(item, config, &mut values)?;
            }
        }
        // Handle single object with nested data
        else if let Some(obj) = json.as_object() {
            // Look for common data keys
            for (_, value) in obj {
                if let Some(array) = value.as_array() {
                    for item in array {
                        Self::extract_from_object(item, config, &mut values)?;
                    }
                }
            }
        }
        // Handle direct object
        else {
            Self::extract_from_object(json, config, &mut values)?;
        }

        Ok(values)
    }

    /// Extract key-value pair from a `JSON` object
    fn extract_from_object(
        obj: &Value,
        config: &InstanceConfig,
        values: &mut HashMap<String, Vec<String>>,
    ) -> linkml_core::error::Result<()> {
        if let Some(obj_map) = obj.as_object() {
            // Get key
            let key = obj_map
                .get(&config.key_field)
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    LinkMLError::data_validation(format!(
                        "Key field '{}' not found or not a string",
                        config.key_field
                    ))
                })?
                .to_string();

            // Get value
            let value = if let Some(value_field) = &config.value_field {
                obj_map
                    .get(value_field)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        LinkMLError::data_validation(format!(
                            "Value field '{value_field}' not found or not a string"
                        ))
                    })?
                    .to_string()
            } else {
                key.clone()
            };

            values.entry(key).or_default().push(value);
        }

        Ok(())
    }

    /// Load from a GraphQL endpoint (future enhancement)
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn load_graphql(
        &self,
        _endpoint: &str,
        _query: &str,
        _config: &InstanceConfig,
    ) -> linkml_core::error::Result<Arc<InstanceData>> {
        Err(LinkMLError::not_implemented("GraphQL instance loading"))
    }

    /// Load from a `SQL` database (future enhancement)
    ///
    /// # Errors
    ///
    /// Returns an error as this is not yet implemented.
    pub fn load_sql(
        &self,
        _connection: &str,
        _query: &str,
        _config: &InstanceConfig,
    ) -> linkml_core::error::Result<Arc<InstanceData>> {
        Err(LinkMLError::not_implemented("SQL instance loading"))
    }

    /// Load from a SPARQL endpoint (future enhancement)
    ///
    /// # Errors
    ///
    /// Returns an error as this is not yet implemented.
    pub fn load_sparql(
        &self,
        _endpoint: &str,
        _query: &str,
        _config: &InstanceConfig,
    ) -> linkml_core::error::Result<Arc<InstanceData>> {
        Err(LinkMLError::not_implemented("SPARQL instance loading"))
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    #[must_use]
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            entries: self.cache.len(),
            sources: self
                .cache
                .iter()
                .map(|entry| entry.source.clone())
                .collect(),
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries
    pub entries: usize,
    /// List of cached sources
    pub sources: Vec<String>,
}

impl Default for InstanceLoader {
    fn default() -> Self {
        let timestamp_service = factory::create_timestamp_service();
        Self::new(timestamp_service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_load_json_file() -> anyhow::Result<(), LinkMLError> {
        let temp_dir = TempDir::new().expect("should create temporary directory: {}");
        let file_path = temp_dir.path().join("instances.json");

        let json_data = r#"[
            {"code": "US", "name": "United States"},
            {"code": "UK", "name": "United Kingdom"},
            {"code": "CA", "name": "Canada"}
        ]"#;

        fs::write(&file_path, json_data)
            .await
            .expect("should write test JSON file: {}");

        let timestamp_service = Arc::new(factory::create_timestamp_service());
        let loader = InstanceLoader::new(timestamp_service);
        let config = InstanceConfig {
            key_field: "code".to_string(),
            value_field: Some("name".to_string()),
            filter: None,
        };

        let instance_data = loader
            .load_json_file(&file_path, &config)
            .await
            .expect("should load JSON instance data: {}");

        assert_eq!(instance_data.values.len(), 3);
        assert_eq!(
            instance_data
                .values
                .get("US")
                .ok_or_else(|| anyhow::anyhow!("should have US entry"))?,
            &vec!["United States"]
        );
        assert_eq!(
            instance_data
                .values
                .get("UK")
                .ok_or_else(|| anyhow::anyhow!("should have UK entry"))?,
            &vec!["United Kingdom"]
        );
        assert_eq!(
            instance_data
                .values
                .get("CA")
                .ok_or_else(|| anyhow::anyhow!("should have CA entry"))?,
            &vec!["Canada"]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_load_csv_file() -> anyhow::Result<(), LinkMLError> {
        let temp_dir = TempDir::new().expect("should create temporary directory: {}");
        let file_path = temp_dir.path().join("instances.csv");

        let csv_data = "code,name
US,United States
UK,United Kingdom
CA,Canada
";
        fs::write(&file_path, csv_data)
            .await
            .expect("should write test CSV file: {}");

        let timestamp_service = Arc::new(factory::create_timestamp_service());
        let loader = InstanceLoader::new(timestamp_service);
        let config = InstanceConfig {
            key_field: "code".to_string(),
            value_field: Some("name".to_string()),
            filter: None,
        };

        let instance_data = loader
            .load_csv_file(&file_path, &config)
            .await
            .expect("should load CSV instance data: {}");

        assert_eq!(instance_data.values.len(), 3);
        assert_eq!(
            instance_data
                .values
                .get("US")
                .ok_or_else(|| anyhow::anyhow!("should have US entry in CSV data"))?,
            &vec!["United States"]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_caching() -> anyhow::Result<(), LinkMLError> {
        let temp_dir = TempDir::new().expect("should create temporary directory: {}");
        let file_path = temp_dir.path().join("instances.json");

        let json_data = r#"[{"id": "1", "value": "test"}]"#;
        fs::write(&file_path, json_data)
            .await
            .expect("should write test JSON file for caching: {}");

        let timestamp_service = Arc::new(factory::create_timestamp_service());
        let loader = InstanceLoader::new(timestamp_service);
        let config = InstanceConfig::default();

        // First load
        let data1 = loader
            .load_json_file(&file_path, &config)
            .await
            .expect("should load JSON data first time: {}");

        // Second load should be from cache
        let data2 = loader
            .load_json_file(&file_path, &config)
            .await
            .expect("should load JSON data from cache: {}");

        // Should be the same Arc
        assert!(Arc::ptr_eq(&data1, &data2));

        // Check cache stats
        let stats = loader.cache_stats();
        assert_eq!(stats.entries, 1);
        Ok(())
    }
}
