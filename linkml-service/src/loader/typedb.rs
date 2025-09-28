//! `TypeDB` loader and dumper for `LinkML`
//!
//! This module provides `TypeDB` integration for `LinkML` using the DBMS service.
//! All `TypeDB` operations must go through the DBMS service as the single source of truth.

use super::dbms_executor::DBMSServiceExecutor;
use super::traits::{
    DataDumper, DataInstance, DataLoader, DumpOptions, DumperResult, LoadOptions, LoaderResult,
};
use super::typedb_integration::{
    TypeDBIntegrationDumper, TypeDBIntegrationLoader, TypeDBIntegrationOptions,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

/// Options for `TypeDB` loading and dumping
#[derive(Debug, Clone)]
pub struct TypeDBOptions {
    /// Database name
    pub database_name: String,

    /// `TypeQL` type to `LinkML` class mapping
    pub type_mapping: HashMap<String, String>,

    /// `TypeQL` attribute to `LinkML` slot mapping (per type)
    pub attribute_mapping: HashMap<String, HashMap<String, String>>,

    /// Batch size for loading/dumping
    pub batch_size: usize,

    /// Whether to infer types from `TypeDB` schema
    pub infer_types: bool,

    /// Whether to create types if they don't exist
    pub create_if_not_exists: bool,

    /// Include inferred attributes
    pub include_inferred: bool,
}

impl Default for TypeDBOptions {
    fn default() -> Self {
        // Load configuration
        let config = crate::config::get_config();

        Self {
            database_name: config.typedb.default_database.clone(),
            type_mapping: HashMap::new(),
            attribute_mapping: HashMap::new(),
            batch_size: config.typedb.batch_size,
            infer_types: true,
            create_if_not_exists: false,
            include_inferred: config.typedb.include_inferred,
        }
    }
}

/// `TypeDB` loader for `LinkML` data
///
/// This loader requires a DBMS service instance to perform all `TypeDB` operations.
pub struct TypeDBLoader<S: dbms_core::DBMSService + 'static> {
    inner: TypeDBIntegrationLoader<DBMSServiceExecutor<S>>,
}

impl<S> TypeDBLoader<S>
where
    S: dbms_core::DBMSService + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    /// Create a new `TypeDB` loader with a DBMS service
    pub fn new(options: TypeDBOptions, dbms_service: Arc<S>) -> Self {
        // Convert options to integration options
        let integration_options = TypeDBIntegrationOptions {
            database_name: options.database_name,
            type_mapping: options.type_mapping,
            attribute_mapping: options.attribute_mapping,
            batch_size: options.batch_size,
            infer_types: options.infer_types,
            include_inferred: options.include_inferred,
            query_timeout_ms: 30000, // Default timeout
        };

        // Create executor using DBMS service
        let executor = DBMSServiceExecutor::new(dbms_service);

        Self {
            inner: TypeDBIntegrationLoader::new(integration_options, executor),
        }
    }
}

#[async_trait]
impl<S> DataLoader for TypeDBLoader<S>
where
    S: dbms_core::DBMSService + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn name(&self) -> &'static str {
        "typedb"
    }

    fn description(&self) -> &'static str {
        "Load data from TypeDB using DBMS service"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        self.inner.supported_extensions()
    }

    async fn load_file(
        &self,
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        self.inner.load_file(path, schema, options).await
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        self.inner.load_string(content, schema, options).await
    }

    async fn load_bytes(
        &self,
        data: &[u8],
        schema: &SchemaDefinition,
        options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        self.inner.load_bytes(data, schema, options).await
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        self.inner.validate_schema(schema)
    }
}

/// `TypeDB` dumper for `LinkML` data
///
/// This dumper requires a DBMS service instance to perform all `TypeDB` operations.
pub struct TypeDBDumper<S: dbms_core::DBMSService + 'static> {
    inner: TypeDBIntegrationDumper<DBMSServiceExecutor<S>>,
}

impl<S> TypeDBDumper<S>
where
    S: dbms_core::DBMSService + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    /// Create a new `TypeDB` dumper with a DBMS service
    pub fn new(options: TypeDBOptions, dbms_service: Arc<S>) -> Self {
        // Convert options to integration options
        let integration_options = TypeDBIntegrationOptions {
            database_name: options.database_name,
            type_mapping: options.type_mapping,
            attribute_mapping: options.attribute_mapping,
            batch_size: options.batch_size,
            infer_types: options.infer_types,
            include_inferred: options.include_inferred,
            query_timeout_ms: 30000, // Default timeout
        };

        // Create executor using DBMS service
        let executor = DBMSServiceExecutor::new(dbms_service);

        Self {
            inner: TypeDBIntegrationDumper::new(integration_options, executor),
        }
    }
}

#[async_trait]
impl<S> DataDumper for TypeDBDumper<S>
where
    S: dbms_core::DBMSService + Send + Sync + 'static,
    S::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    fn name(&self) -> &'static str {
        "typedb"
    }

    fn description(&self) -> &'static str {
        "Dump data to TypeDB using DBMS service"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        self.inner.supported_extensions()
    }

    async fn dump_file(
        &self,
        instances: &[DataInstance],
        path: &std::path::Path,
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<()> {
        self.inner.dump_file(instances, path, schema, options).await
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<String> {
        self.inner.dump_string(instances, schema, options).await
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        self.inner.dump_bytes(instances, schema, options).await
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> DumperResult<()> {
        self.inner.validate_schema(schema)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typedb_options_default() {
        let options = TypeDBOptions::default();

        assert_eq!(options.batch_size, 1000);
        assert!(options.infer_types);
        assert!(!options.create_if_not_exists);
        assert!(!options.include_inferred);
    }
}
