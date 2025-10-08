//! Common initialization utilities for LinkML service examples
//!
//! This module provides proper initialization patterns following the dyn-compatibility
//! guidelines from docs/architecture/dyn-compatibility-guidelines.md
//!
//! NOTE: In a real RootReal application, you would import the actual service
//! implementations from their respective crates. This module provides minimal
//! implementations suitable for running examples in isolation.

pub mod service_init;

pub use service_init::init_linkml_service_with_real_deps;

use linkml_core::error::Result;
use linkml_core::traits::LinkMLService;
use linkml_service::service::MinimalLinkMLServiceImpl;
use std::sync::Arc;

/// Initialize LinkML service for examples
///
/// This function demonstrates the proper initialization pattern from the
/// dyn-compatibility guidelines. In production:
///
/// 1. Create concrete instances of non-dyn-compatible services
/// 2. Pass them to services that depend on them
/// 3. Convert to trait objects only for dyn-compatible services
///
/// For examples, we use simplified implementations that allow the examples
/// to run without the full RootReal service infrastructure.
pub async fn init_example_linkml_service() -> Result<Arc<dyn LinkMLService>> {
    let service = MinimalLinkMLServiceImpl::new()?;
    Ok(Arc::new(service))
}

/// Create a LinkML service using default test implementations
///
/// This is a simplified initialization for examples that don't need
/// the full service infrastructure.
///
/// Returns an `Arc<dyn LinkMLService>` so examples can share the service across tasks.
pub async fn create_example_service() -> Result<Arc<dyn LinkMLService>> {
    init_example_linkml_service().await
}
