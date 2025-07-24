//! DBMS service executor for TypeDB integration
//!
//! This module provides a TypeDB query executor that uses the DBMS service,
//! allowing LinkML to integrate with TypeDB without circular dependencies.

use super::typedb_integration::TypeDBQueryExecutor;
use async_trait::async_trait;
use std::sync::Arc;

/// DBMS service executor that integrates with RootReal's DBMS service
pub struct DBMSServiceExecutor<S: dbms_core::DBMSService> {
    /// The DBMS service instance
    service: Arc<S>,
}

impl<S: dbms_core::DBMSService> DBMSServiceExecutor<S> {
    /// Create a new DBMS service executor
    pub fn new(service: Arc<S>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<S> TypeDBQueryExecutor for DBMSServiceExecutor<S>
where
    S: dbms_core::DBMSService + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    async fn execute_query(&self, query: &str, database: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Use the DBMS service's execute_string_query method
        self.service
            .execute_string_query(database, query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    
    async fn execute_define(&self, query: &str, database: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Schema modifications go through the deploy_schema method
        self.service
            .deploy_schema(database, query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    
    async fn execute_insert(&self, query: &str, database: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Data insertions also use execute_string_query
        self.service
            .execute_string_query(database, query)
            .await
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}

/// Direct TypeDB driver executor that bypasses DBMS service
/// 
/// IMPORTANT: This is a placeholder for cases where the DBMS service is not available.
/// In production, always use DBMSServiceExecutor instead.
pub struct DirectTypeDBExecutor {
    server_address: String,
}

impl DirectTypeDBExecutor {
    /// Create a new direct TypeDB executor
    pub fn new(server_address: impl Into<String>) -> Self {
        Self {
            server_address: server_address.into(),
        }
    }
}

#[async_trait]
impl TypeDBQueryExecutor for DirectTypeDBExecutor {
    async fn execute_query(&self, _query: &str, _database: &str) -> Result<String, Box<dyn std::error::Error>> {
        // Return an error indicating this should not be used in production
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "DirectTypeDBExecutor is not implemented. Use DBMSServiceExecutor with the DBMS service instead."
        )))
    }
    
    async fn execute_define(&self, _query: &str, _database: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Return an error indicating this should not be used in production
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "DirectTypeDBExecutor is not implemented. Use DBMSServiceExecutor with the DBMS service instead."
        )))
    }
    
    async fn execute_insert(&self, _query: &str, _database: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Return an error indicating this should not be used in production
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "DirectTypeDBExecutor is not implemented. Use DBMSServiceExecutor with the DBMS service instead."
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_direct_executor_creation() {
        let executor = DirectTypeDBExecutor::new("localhost:1729");
        assert_eq!(executor.server_address, "localhost:1729");
    }
    
    #[tokio::test]
    async fn test_direct_executor_returns_error() {
        let executor = DirectTypeDBExecutor::new("localhost:1729");
        
        // All methods should return an error
        assert!(executor.execute_query("match $x isa thing;", "test").await.is_err());
        assert!(executor.execute_define("define person sub entity;", "test").await.is_err());
        assert!(executor.execute_insert("insert $x isa person;", "test").await.is_err());
    }
}