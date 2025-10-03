//! # LinkML Client
//!
//! Client library for interacting with LinkML validation services.
//!
//! This crate provides a client interface for interacting with the LinkML service,
//! allowing remote or local service interaction through a unified API.
//!
//! ## Overview
//!
//! `linkml-client` provides:
//!
//! - **Unified Interface**: Single client for local or remote LinkML services
//! - **Type-Safe**: Leverages Rust's type system for compile-time guarantees
//! - **Async Support**: Full async/await support for non-blocking operations
//! - **Service Delegation**: Transparently delegates to underlying service implementation
//!
//! ## Usage
//!
//! ```rust
//! use linkml_client::LinkMLClient;
//! use linkml_service::create_linkml_service;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create the underlying service
//!     let service = create_linkml_service().await?;
//!
//!     // Wrap it in a client
//!     let client = LinkMLClient::new(Arc::new(service));
//!
//!     // Use the client (same API as service)
//!     let schema = client.load_schema("schema.yaml").await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Design
//!
//! The client is generic over the service implementation because `LinkMLService`
//! is not dyn-compatible (it has generic methods). This allows for:
//!
//! - Zero-cost abstraction when using concrete types
//! - Type-safe service interaction
//! - Flexible deployment models (local, remote, etc.)
//!
//! ## Features
//!
//! - **Service Wrapper**: Wraps any `LinkMLService` implementation
//! - **Trait Delegation**: Implements `LinkMLService` by delegating to wrapped service
//! - **Arc-based Sharing**: Allows sharing client across async tasks
//!
//! ## Example: Remote Client (Future)
//!
//! While the current implementation wraps a local service, the client pattern
//! is designed to support remote services in the future:
//!
//! ```rust,ignore
//! // Future: Remote client over HTTP/gRPC
//! let remote_client = LinkMLClient::connect("http://linkml-service:8080").await?;
//! let schema = remote_client.load_schema("schema.yaml").await?;
//! ```
//!
//! ## License
//!
//! Licensed under CC-BY-NC-4.0. See LICENSE file for details.

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
