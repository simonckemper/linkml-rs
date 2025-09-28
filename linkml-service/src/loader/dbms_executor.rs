//! DBMS service executor for `TypeDB` integration
//!
//! This module provides a `TypeDB` query executor that uses the DBMS service,
//! allowing `LinkML` to integrate with `TypeDB` without circular dependencies.

use super::typedb_integration::TypeDBQueryExecutor;
use async_trait::async_trait;
use std::sync::Arc;

/// DBMS service executor that integrates with `RootReal`'s DBMS service
pub struct DBMSServiceExecutor<S: dbms_core::DBMSService> {
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
    async fn execute_query(
        &self,
        query: &str,
        database: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // Use the DBMS service's execute_string_query method
        self.service
            .execute_string_query(database, query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn execute_define(
        &self,
        query: &str,
        database: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Schema modifications go through the deploy_schema method
        self.service
            .deploy_schema(database, query)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn execute_insert(
        &self,
        query: &str,
        database: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Data insertions also use execute_string_query
        self.service
            .execute_string_query(database, query)
            .await
            .map(|_| ())
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
