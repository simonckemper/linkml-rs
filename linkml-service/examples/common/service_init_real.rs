//! Example using REAL RootReal services (not mocks)
//!
//! This example demonstrates proper service integration using factory functions.
//! NO MOCKS are used - all services are real implementations.

// Import factory functions for real services
use logger_service::factory::create_logger_service;
use timestamp_service::factory::create_timestamp_service;
use task_management_service::factory::create_task_management_service;
use error_handling_service::factory::create_error_handling_service;
use error_handling_service::service::components::{ErrorCategorizer, RecoveryStrategist, PatternAnalyzer};
use cache_service::factory::create_cache_service;
use configuration_service::factory::create_configuration_service;
use validation_core::ValidationRegistry;

use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Example using REAL RootReal Services ===\n");
    
    // Create real services using factory functions
    let timestamp = create_timestamp_service()?;
    let logger = create_logger_service(timestamp.clone())?;
    let task_management = create_task_management_service(logger.clone())?;
    
    let error_handler = create_error_handling_service(
        logger.clone(),
        ErrorCategorizer::new(),
        RecoveryStrategist::default(),
        PatternAnalyzer::new(),
    )?;
    
    // Add your example logic here using the real services
    logger.info("Example running with real services").await?;
    
    // Your code here...
    
    println!("\n✅ Example completed successfully!");
    println!("This example used REAL services, not mocks!");
    
    Ok(())
}
