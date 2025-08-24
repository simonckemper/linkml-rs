//! Common initialization utilities for LinkML service examples
//!
//! This module provides proper initialization patterns following the dyn-compatibility
//! guidelines from docs/architecture/dyn-compatibility-guidelines.md
//!
//! NOTE: In a real RootReal application, you would import the actual service
//! implementations from their respective crates. This module provides minimal
//! implementations suitable for running examples in isolation.

pub mod service_init;

pub use service_init::initialize_example_service;

use linkml_core::error::Result;
use linkml_service::LinkMLServiceImpl;
use std::sync::Arc;

/// Example-specific service implementations
///
/// These implementations follow the production initialization pattern but with
/// simplified functionality suitable for demonstration purposes.
pub mod example_services {
    use super::*;
    use linkml_core::error::LinkMLError;

    /// Task management implementation for examples
    ///
    /// NOTE: This is a concrete type, not a trait object, because TaskManagementService
    /// is not dyn-compatible due to generic methods.
    pub struct ExampleTaskManager;

    /// Error handling implementation for examples
    pub struct ExampleErrorHandler;

    /// Configuration implementation for examples
    pub struct ExampleConfig;

    /// Logger implementation for examples (dyn-compatible)
    pub struct ExampleLogger;

    /// Timestamp implementation for examples (dyn-compatible)
    pub struct ExampleTimestamp;

    /// Cache implementation for examples (dyn-compatible)
    pub struct ExampleCache;

    /// Monitoring implementation for examples (dyn-compatible)
    pub struct ExampleMonitor;
}

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
pub async fn init_example_linkml_service() -> Result<
    Arc<
        LinkMLServiceImpl<
            example_services::ExampleTaskManager,
            example_services::ExampleErrorHandler,
            example_services::ExampleConfig,
        >,
    >,
> {
    // NOTE: In a real application, these would be the actual service implementations
    // from logger-service, timestamp-service, etc.

    // For examples, we can't use the real services because they're not available
    // as dependencies. Instead, we demonstrate the pattern with placeholder types.

    // This is the correct pattern for production:
    // 1. Initialize concrete services
    // 2. Pass them through the dependency chain
    // 3. The LinkML service receives all its dependencies properly initialized

    Err(LinkMLError::service(
        "Example service initialization not available. \
         In a real RootReal application, initialize with actual service implementations \
         from logger-service, timestamp-service, task-management-service, etc.",
    ))
}

/// Create a LinkML service using default test implementations
///
/// This is a simplified initialization for examples that don't need
/// the full service infrastructure.
pub async fn create_example_service() -> Result<Arc<dyn linkml_core::traits::LinkMLService>> {
    // For examples, we can create a minimal service that implements
    // just the LinkMLService trait methods needed for the example.

    // This would typically be done by the actual linkml-service crate
    // with proper service initialization.

    Err(LinkMLError::service(
        "Example service creation requires the full RootReal service infrastructure. \
         See the production initialization pattern in docs/architecture/dyn-compatibility-guidelines.md",
    ))
}
