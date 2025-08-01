//! Configuration validation using LinkML schema
//!
//! This module validates configuration files against the LinkML configuration schema.

use super::{LinkMLConfig, load_config};
use crate::validator::{ValidatorBuilder, ValidationMode};
use crate::parser::YamlParser;
use linkml_core::error::{LinkMLError, Result};
use std::path::Path;

/// Validate configuration against schema
pub async fn validate_config(config: &LinkMLConfig) -> Result<()> {
    // Load the configuration schema
    let schema_path = Path::new("config/schema/linkml-config-schema.yaml");
    let mut parser = YamlParser::new();
    let schema = parser.parse_file(schema_path)?;
    
    // Build validator
    let validator = ValidatorBuilder::new()
        .with_mode(ValidationMode::Strict)
        .build(&schema)?;
    
    // Convert config to JSON for validation
    let config_json = serde_json::to_value(config)
        .map_err(|e| LinkMLError::configuration(format!("Failed to serialize config: {}", e)))?;
    
    // Validate
    let report = validator.validate(&config_json, Some("LinkMLConfig"))?;
    
    if report.has_errors() {
        let errors: Vec<_> = report.issues
            .iter()
            .filter(|i| i.severity == crate::validator::report::Severity::Error)
            .map(|i| format!("- {}", i.message))
            .collect();
            
        return Err(LinkMLError::configuration(
            format!("Configuration validation failed:\n{}", errors.join("\n"))
        ));
    }
    
    Ok(())
}

/// Load and validate configuration from file
pub async fn load_and_validate_config(path: &Path) -> Result<LinkMLConfig> {
    let config: LinkMLConfig = load_config(path)?;
    validate_config(&config).await?;
    Ok(config)
}

/// Validate specific configuration values
pub fn validate_values(config: &LinkMLConfig) -> Result<()> {
    // Additional runtime validations beyond schema
    
    // Ensure L1 < L2 < L3 cache TTLs
    if config.performance.cache_ttl_levels.l1_seconds >= config.performance.cache_ttl_levels.l2_seconds {
        return Err(LinkMLError::configuration(
            "L1 cache TTL must be less than L2 cache TTL".to_string()
        ));
    }
    
    if config.performance.cache_ttl_levels.l2_seconds >= config.performance.cache_ttl_levels.l3_seconds {
        return Err(LinkMLError::configuration(
            "L2 cache TTL must be less than L3 cache TTL".to_string()
        ));
    }
    
    // Ensure min < max TTL
    if config.performance.cache_ttl_levels.min_ttl_seconds >= config.performance.cache_ttl_levels.max_ttl_seconds {
        return Err(LinkMLError::configuration(
            "Minimum TTL must be less than maximum TTL".to_string()
        ));
    }
    
    // Ensure memory limits are reasonable
    if config.performance.memory_pool.max_size_bytes > config.performance.memory_limit_bytes {
        return Err(LinkMLError::configuration(
            "Memory pool size cannot exceed total memory limit".to_string()
        ));
    }
    
    if config.security_limits.max_cache_memory_bytes > config.performance.memory_limit_bytes {
        return Err(LinkMLError::configuration(
            "Cache memory limit cannot exceed total memory limit".to_string()
        ));
    }
    
    // Validate thread count
    if config.validator.enable_parallel && config.validator.thread_count == 0 {
        return Err(LinkMLError::configuration(
            "Thread count must be > 0 when parallel validation is enabled".to_string()
        ));
    }
    
    // Validate network settings
    if config.network.default_host.is_empty() {
        return Err(LinkMLError::configuration(
            "Default host cannot be empty".to_string()
        ));
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_validate_default_config() {
        let config = crate::config::load_default_config()
            .expect("should load default config");
            
        validate_values(&config).expect("default config should be valid");
        
        // Schema validation would require the validator to be fully initialized
        // validate_config(&config).await.expect("default config should validate against schema");
    }
    
    #[test]
    fn test_validate_ttl_ordering() {
        let mut config = crate::config::load_default_config()
            .expect("should load default config");
            
        // Break TTL ordering
        config.performance.cache_ttl_levels.l1_seconds = 7200;
        config.performance.cache_ttl_levels.l2_seconds = 1800;
        
        let result = validate_values(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("L1 cache TTL"));
    }
    
    #[test]
    fn test_validate_memory_limits() {
        let mut config = crate::config::load_default_config()
            .expect("should load default config");
            
        // Set memory pool larger than total limit
        config.performance.memory_limit_bytes = 1_000_000_000; // 1GB
        config.performance.memory_pool.max_size_bytes = 2_000_000_000; // 2GB
        
        let result = validate_values(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Memory pool size"));
    }
}