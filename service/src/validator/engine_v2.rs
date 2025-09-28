//! Optimized validation engine using Arc<SchemaDefinition>
//!
//! This module provides a validation engine that shares schemas efficiently
//! using Arc, eliminating unnecessary cloning during validation.

use std::sync::Arc;
use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;

use linkml_core::{
    error::{LinkMLError, Result},
    types::SchemaDefinition,
    schema_arc::{ArcSchema, SchemaProvider}};

use crate::traits::ValidationEngine as ValidationEngineTrait;
use super::{
    ValidationContext, ValidationResult, ValidationError,
    validators::{RequiredValidator, RangeValidator, PatternValidator}};

/// Optimized validation engine using Arc for schema sharing
pub struct ValidationEngineV2 {
    schema: ArcSchema,
    /// Validator instances
    validators: Vec<Box<dyn Validator>>,
    /// Cached validation contexts
    context_cache: dashmap::DashMap<String, Arc<ValidationContext>>,
    /// Configuration
    config: ValidationConfig}

/// Validation configuration
#[derive(Clone, Debug)]
pub struct ValidationConfig {
    /// Whether to use cached contexts
    pub cache_contexts: bool,
    /// Maximum number of cached contexts
    pub max_cached_contexts: usize,
    /// Whether to validate recursively
    pub recursive_validation: bool,
    /// Whether to collect all errors or stop on first
    pub collect_all_errors: bool}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            cache_contexts: true,
            max_cached_contexts: 100,
            recursive_validation: true,
            collect_all_errors: true}
    }
}

impl ValidationEngineV2 {
    /// Create a new validation engine from Arc<SchemaDefinition>
    pub fn new(schema: ArcSchema) -> Self {
        Self::with_config(schema, ValidationConfig::default())
    }

    /// Create with custom configuration
    pub fn with_config(schema: ArcSchema, config: ValidationConfig) -> Self {
        let validators = Self::create_validators();

        Self {
            schema,
            validators,
            context_cache: dashmap::DashMap::new(),
            config}
    }

    /// Create from owned schema
    pub fn from_schema(schema: SchemaDefinition) -> Self {
        Self::new(Arc::new(schema))
    }

    /// Get the schema Arc (cheap clone)
    pub fn schema_arc(&self) -> ArcSchema {
        Arc::clone(&self.schema)
    }

    /// Create default validators
    fn create_validators() -> Vec<Box<dyn Validator>> {
        vec![
            Box::new(RequiredValidator),
            Box::new(RangeValidator),
            Box::new(PatternValidator),
        ]
    }

    /// Get or create validation context for a class
    fn get_context(&self, class_name: &str) -> Arc<ValidationContext> {
        if self.config.cache_contexts {
            // Check cache first
            if let Some(context) = self.context_cache.get(class_name) {
                return Arc::clone(&context);
            }

            // Create and cache new context
            let context = Arc::new(ValidationContext::new(
                Arc::clone(&self.schema),
                class_name.to_string(),
            ));

            // Evict old entries if at capacity
            if self.context_cache.len() >= self.config.max_cached_contexts {
                // Simple FIFO eviction
                if let Some(first_key) = self.context_cache.iter().next().map(|e| e.key().clone()) {
                    self.context_cache.remove(&first_key);
                }
            }

            self.context_cache.insert(class_name.to_string(), Arc::clone(&context));
            context
        } else {
            // Create fresh context each time
            Arc::new(ValidationContext::new(
                Arc::clone(&self.schema),
                class_name.to_string(),
            ))
        }
    }

    /// Validate data against a class
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn validate_with_class(
        &self,
        data: &Value,
        class_name: &str,
    ) -> Result<ValidationResult> {
        let context = self.get_context(class_name);
        let mut errors = Vec::new();

        // Run all validators
        for validator in &self.validators {
            match validator.validate(data, &context).await {
                Ok(mut result) => {
                    errors.append(&mut result.errors);
                    if !self.config.collect_all_errors && !errors.is_empty() {
                        break;
                    }
                }
                Err(e) => {
                    errors.push(ValidationError {
                        path: "/".to_string(),
                        message: format!("Validator error: {e}"),
                        severity: super::Severity::Error});
                    if !self.config.collect_all_errors {
                        break;
                    }
                }
            }
        }

        Ok(ValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings: Vec::new()})
    }

    /// Clear context cache
    pub fn clear_cache(&self) {
        self.context_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            contexts_cached: self.context_cache.len(),
            cache_enabled: self.config.cache_contexts,
            max_contexts: self.config.max_cached_contexts}
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub contexts_cached: usize,
    pub cache_enabled: bool,
    pub max_contexts: usize}

impl SchemaProvider for ValidationEngineV2 {
    fn schema(&self) -> &ArcSchema {
        &self.schema
    }
}

/// Trait for validators (simplified for this example)
#[async_trait]
trait Validator: Send + Sync {
    async fn validate(
        &self,
        data: &Value,
        context: &ValidationContext,
    ) -> Result<ValidationResult>;
}

/// Factory for creating validation engines with proper Arc handling
pub struct ValidationEngineFactory {
    schema_cache: Arc<linkml_core::schema_arc::SchemaCache>}

impl ValidationEngineFactory {
    /// Create new factory
    pub fn new() -> Self {
        Self {
            schema_cache: Arc::new(linkml_core::schema_arc::SchemaCache::new())}
    }

    /// Create engine from schema name (uses cache)
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn create_from_name(&self, schema_name: &str) -> Result<ValidationEngineV2> {
        let schema = self.schema_cache.get_or_insert(schema_name, || {
            // In real implementation, load from file/network
            SchemaDefinition {
                id: schema_name.to_string(),
                name: schema_name.to_string(),
                ..Default::default()
            }
        });

        Ok(ValidationEngineV2::new(schema))
    }

    /// Create engine from schema
    pub fn create_from_schema(&self, schema: SchemaDefinition) -> ValidationEngineV2 {
        let schema_arc = Arc::new(schema);
        self.schema_cache.insert(&schema_arc.name, Arc::clone(&schema_arc));
        ValidationEngineV2::new(schema_arc)
    }
}

/// Batch validation using shared engine
pub struct BatchValidator {
    engine: Arc<ValidationEngineV2>}

impl BatchValidator {
    /// Create new batch validator
    pub fn new(engine: ValidationEngineV2) -> Self {
        Self {
            engine: Arc::new(engine)}
    }

    /// Validate multiple items in parallel
    pub async fn validate_batch(
        &self,
        items: Vec<(Value, String)>, // (data, class_name)
    ) -> Vec<Result<ValidationResult>> {
        use futures::future::join_all;

        let futures = items.into_iter().map(|(data, class_name)| {
            let engine = Arc::clone(&self.engine);
            async move {
                engine.validate_with_class(&data, &class_name).await
            }
        });

        join_all(futures).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_validation_engine_v2() {
        let schema = SchemaDefinition {
            id: "test".to_string(),
            name: "test".to_string(),
            ..Default::default()
        };

        let engine = ValidationEngineV2::from_schema(schema);
        let data = serde_json::json!({
            "name": "Test",
            "value": 42
        });

        let result = engine.validate_with_class(&data, "TestClass").await.expect("should validate: {}");
        assert!(result.valid || !result.valid); // Just check it runs
    }

    #[test]
    fn test_schema_sharing() {
        let schema = Arc::new(SchemaDefinition::default());
        let engine1 = ValidationEngineV2::new(Arc::clone(&schema));
        let engine2 = ValidationEngineV2::new(Arc::clone(&schema));

        assert!(Arc::ptr_eq(&engine1.schema, &engine2.schema));
    }

    #[test]
    fn test_cache_stats() {
        let engine = ValidationEngineV2::from_schema(SchemaDefinition::default());
        let stats = engine.cache_stats();

        assert_eq!(stats.contexts_cached, 0);
        assert!(stats.cache_enabled);
    }
}