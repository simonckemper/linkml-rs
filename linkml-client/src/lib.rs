//! `LinkML` Client Library
//!
//! This crate provides a client interface for interacting with the `LinkML` service.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use async_trait::async_trait;
use linkml_core::{
    error::Result,
    traits::{LinkMLService, LinkMLServiceExt},
};
use std::sync::Arc;

/// Client for remote `LinkML` service
///
/// Generic over the concrete `LinkML` service implementation
/// since `LinkMLService` is not dyn-compatible (has generic methods)
pub struct LinkMLClient<S> {
    service: Arc<S>,
}

impl<S> LinkMLClient<S>
where
    S: LinkMLService + Send + Sync + 'static,
{
    /// Create a new client with a service instance
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }

    /// Get reference to the underlying service
    #[must_use]
    pub fn service(&self) -> &Arc<S> {
        &self.service
    }
}

// Delegate trait implementation to service
#[async_trait]
impl<S> LinkMLService for LinkMLClient<S>
where
    S: LinkMLService + Send + Sync + 'static,
{
    async fn load_schema(
        &self,
        path: &std::path::Path,
    ) -> Result<linkml_core::types::SchemaDefinition> {
        self.service.load_schema(path).await
    }

    async fn load_schema_str(
        &self,
        content: &str,
        format: linkml_core::traits::SchemaFormat,
    ) -> Result<linkml_core::types::SchemaDefinition> {
        self.service.load_schema_str(content, format).await
    }

    async fn validate(
        &self,
        data: &serde_json::Value,
        schema: &linkml_core::types::SchemaDefinition,
        target_class: &str,
    ) -> Result<linkml_core::types::ValidationReport> {
        self.service.validate(data, schema, target_class).await
    }
}

#[async_trait]
impl<S> LinkMLServiceExt for LinkMLClient<S>
where
    S: LinkMLServiceExt + Send + Sync + 'static,
{
    async fn validate_typed<T>(
        &self,
        data: &serde_json::Value,
        schema: &linkml_core::types::SchemaDefinition,
        target_class: &str,
    ) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.service
            .validate_typed(data, schema, target_class)
            .await
    }
}
