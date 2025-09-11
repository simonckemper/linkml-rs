//! Database loader implementation

use super::column_info::ColumnInfo;
use super::converters::{MySqlConverter, PostgresConverter};
use super::options::{DatabaseOptions, ForeignKeyRelation};
use super::pool::DatabasePool;
use crate::loader::traits::{DataInstance, DataLoader, LoaderError, LoaderResult, LoadOptions};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{json, Value};
use sqlx::mysql::{MySqlPoolOptions, MySqlRow};
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Column, Row};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

/// Database loader for `LinkML` data
pub struct DatabaseLoader {
    options: DatabaseOptions,
    pool: Option<DatabasePool>,
}

impl DatabaseLoader {
    /// Create a new database loader
    #[must_use]
    pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None,
        }
    }

    /// Connect to the database
    async fn connect(&mut self) -> LoaderResult<()> {
        if self.pool.is_none() {
            let pool = if self.options.connection_string.starts_with("postgres://") {
                let pg_pool = PgPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| {
                        LoaderError::Configuration(format!("Failed to connect to PostgreSQL: {e}"))
                    })?;
                DatabasePool::PostgreSQL(pg_pool)
            } else if self.options.connection_string.starts_with("mysql://") {
                let mysql_pool = MySqlPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| {
                        LoaderError::Configuration(format!("Failed to connect to MySQL: {e}"))
                    })?;
                DatabasePool::MySQL(mysql_pool)
            } else {
                return Err(LoaderError::Configuration(
                    "Unsupported database type. Only PostgreSQL and MySQL are supported."
                        .to_string(),
                ));
            };

            self.pool = Some(pool);
        }
        Ok(())
    }

    /// Get table names from the database
    async fn get_table_names(&self) -> LoaderResult<Vec<String>> {
        let pool = self.get_pool()?;

        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let query = if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables 
                         WHERE table_schema = '{}' AND table_type = 'BASE TABLE'",
                        schema
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables 
                     WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
                        .to_string()
                };

                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!(
                        "Failed to get table names: {e}"
                    )))
                })?;

                let mut tables = Vec::new();
                for row in rows {
                    let table_name: String = row.try_get(0).map_err(|e| {
                        LoaderError::Io(std::io::Error::other(format!(
                            "Failed to get table name: {e}"
                        )))
                    })?;
                    
                    // Apply include/exclude filters
                    if self.should_include_table(&table_name) {
                        tables.push(table_name);
                    }
                }
                Ok(tables)
            }
            DatabasePool::MySQL(mysql_pool) => {
                let database = self
                    .options
                    .schema_name
                    .as_deref()
                    .unwrap_or("information_schema");
                
                let query = format!(
                    "SELECT table_name FROM information_schema.tables 
                     WHERE table_schema = '{}' AND table_type = 'BASE TABLE'",
                    database
                );

                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!(
                        "Failed to get table names: {e}"
                    )))
                })?;

                let mut tables = Vec::new();
                for row in rows {
                    let table_name: String = row.try_get(0).map_err(|e| {
                        LoaderError::Io(std::io::Error::other(format!(
                            "Failed to get table name: {e}"
                        )))
                    })?;
                    
                    // Apply include/exclude filters
                    if self.should_include_table(&table_name) {
                        tables.push(table_name);
                    }
                }
                Ok(tables)
            }
        }
    }

    /// Check if a table should be included
    fn should_include_table(&self, table_name: &str) -> bool {
        // Check exclude list
        if self.options.exclude_tables.contains(table_name) {
            return false;
        }

        // Check include list if specified
        if let Some(include) = &self.options.include_tables {
            return include.contains(table_name);
        }

        true
    }

    /// Get column information for a table
    async fn get_columns(&self, table_name: &str) -> LoaderResult<Vec<ColumnInfo>> {
        let pool = self.get_pool()?;

        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let query = format!(
                    "SELECT column_name, data_type, is_nullable, column_default,
                            character_maximum_length, numeric_precision, numeric_scale
                     FROM information_schema.columns
                     WHERE table_name = '{}'
                     ORDER BY ordinal_position",
                    table_name
                );

                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!("Failed to get columns: {e}")))
                })?;

                let mut columns = Vec::new();
                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get(0).unwrap_or_default(),
                        data_type: row.try_get(1).unwrap_or_default(),
                        is_nullable: row.try_get::<String, _>(2).unwrap_or_default() == "YES",
                        is_primary_key: false, // Would need additional query
                        default_value: row.try_get(3).ok(),
                        max_length: row.try_get(4).ok(),
                        numeric_precision: row.try_get(5).ok(),
                        numeric_scale: row.try_get(6).ok(),
                    });
                }
                Ok(columns)
            }
            DatabasePool::MySQL(mysql_pool) => {
                let database = self.options.schema_name.as_deref().unwrap_or("mysql");
                let query = format!(
                    "SELECT column_name, data_type, is_nullable, column_default,
                            character_maximum_length, numeric_precision, numeric_scale
                     FROM information_schema.columns
                     WHERE table_schema = '{}' AND table_name = '{}'
                     ORDER BY ordinal_position",
                    database, table_name
                );

                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!("Failed to get columns: {e}")))
                })?;

                let mut columns = Vec::new();
                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get(0).unwrap_or_default(),
                        data_type: row.try_get(1).unwrap_or_default(),
                        is_nullable: row.try_get::<String, _>(2).unwrap_or_default() == "YES",
                        is_primary_key: false, // Would need additional query
                        default_value: row.try_get(3).ok(),
                        max_length: row.try_get(4).ok(),
                        numeric_precision: row.try_get(5).ok(),
                        numeric_scale: row.try_get(6).ok(),
                    });
                }
                Ok(columns)
            }
        }
    }

    /// Load data from a table
    async fn load_table_data(
        &self,
        table_name: &str,
        columns: &[ColumnInfo],
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let pool = self.get_pool()?;
        let mut instances = Vec::new();

        // Build query
        let query = format!("SELECT * FROM {table_name} LIMIT {}", self.options.batch_size);

        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                use futures::TryStreamExt;
                
                let mut rows = sqlx::query(&query).fetch(pg_pool);
                let mut batch = Vec::new();
                
                while let Some(row) = rows.try_next().await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!("Failed to fetch row: {e}")))
                })? {
                    let instance = PostgresConverter::row_to_instance(
                        &row,
                        table_name,
                        columns,
                        &self.options.table_mapping,
                        &self.options.column_mapping,
                    )?;
                    batch.push(instance);
                    
                    if batch.len() >= self.options.batch_size {
                        instances.append(&mut batch);
                    }
                }
                instances.extend(batch);
            }
            DatabasePool::MySQL(mysql_pool) => {
                use futures::TryStreamExt;
                
                let mut rows = sqlx::query(&query).fetch(mysql_pool);
                let mut batch = Vec::new();
                
                while let Some(row) = rows.try_next().await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(format!("Failed to fetch row: {e}")))
                })? {
                    let instance = MySqlConverter::row_to_instance(
                        &row,
                        table_name,
                        columns,
                        &self.options.table_mapping,
                        &self.options.column_mapping,
                    )?;
                    batch.push(instance);
                    
                    if batch.len() >= self.options.batch_size {
                        instances.append(&mut batch);
                    }
                }
                instances.extend(batch);
            }
        }

        // Apply foreign key relationships
        self.apply_foreign_keys(&mut instances, table_name);

        Ok(instances)
    }

    /// Apply foreign key relationships to instances
    fn apply_foreign_keys(&self, instances: &mut [DataInstance], table_name: &str) {
        if let Some(fk_relations) = self.options.foreign_keys.get(table_name) {
            for instance in instances {
                for fk in fk_relations {
                    if let Some(Value::String(ref_id)) = instance.data.get(&fk.column) {
                        // Create a reference to the related object
                        let ref_instance = json!({
                            "@type": self.options.table_mapping.get(&fk.referenced_table)
                                .cloned()
                                .unwrap_or_else(|| to_pascal_case(&fk.referenced_table)),
                            "id": ref_id
                        });
                        instance.data.insert(fk.slot_name.clone(), ref_instance);
                    }
                }
            }
        }
    }

    /// Get the database pool
    fn get_pool(&self) -> LoaderResult<&DatabasePool> {
        self.pool
            .as_ref()
            .ok_or_else(|| LoaderError::Configuration("Database not connected".to_string()))
    }

    /// Load data from database
    pub async fn load_from_database(
        &mut self,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Connect to database
        self.connect().await?;
        info!("Database loader connected successfully");

        // Get all tables
        let tables = self.get_table_names().await?;
        info!("Found {} tables to load", tables.len());

        let mut all_instances = Vec::new();

        // Load data from each table
        for table_name in tables {
            debug!("Loading data from table: {}", table_name);

            // Get column information
            let columns = self.get_columns(&table_name).await?;

            // Load table data
            let instances = self.load_table_data(&table_name, &columns, schema).await?;
            info!(
                "Loaded {} instances from table {}",
                instances.len(),
                table_name
            );

            all_instances.extend(instances);
        }

        info!("Total loaded {} instances from database", all_instances.len());
        Ok(all_instances)
    }
}

// Helper function
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}