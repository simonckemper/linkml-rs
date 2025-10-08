//! Common service initialization for examples
//!
//! Provides simplified service initialization for LinkML examples.
//! For production use, see the full service factory patterns in the main crate.

use linkml_core::error::Result;
use linkml_core::traits::LinkMLService;
use linkml_service::service::MinimalLinkMLServiceImpl;
use std::sync::Arc;

/// Initialize a simplified LinkML service for examples
///
/// This creates a basic service instance suitable for examples and testing.
/// For production use, use the full factory functions with proper dependencies.
pub async fn init_linkml_service_with_real_deps() -> Result<Arc<dyn LinkMLService>> {
    let service = MinimalLinkMLServiceImpl::new()?;
    Ok(Arc::new(service))
}
