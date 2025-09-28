//! Database loader implementation

use super::column_info::ColumnInfo;
use super::converters::{MySqlConverter, PostgresConverter};
use super::options::DatabaseOptions;
use super::pool::DatabasePool;
use crate::loader::traits::{DataInstance, LoaderError, LoaderResult, DataLoader, LoadOptions};
use linkml_core::prelude::*;
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use tracing::{debug, info};
use async_trait::async_trait;
use std::path::Path;

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
    async fn connect(&self) -> LoaderResult<DatabasePool> {
        if let Some(ref pool) = self.pool {
            Ok(pool.clone())
        } else {
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

            Ok(pool)
        }
    }

    /// Get table names from the database
    async fn get_table_names(&self, pool: &DatabasePool) -> LoaderResult<Vec<String>> {

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

                let rows = sqlx::query(&query)
                    .fetch_all(mysql_pool)
                    .await
                    .map_err(|e| {
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
    async fn get_columns(&self, table_name: &str, pool: &DatabasePool) -> LoaderResult<Vec<ColumnInfo>> {

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

                let rows = sqlx::query(&query)
                    .fetch_all(mysql_pool)
                    .await
                    .map_err(|e| {
                        LoaderError::Io(std::io::Error::other(format!(
                            "Failed to get columns: {e}"
                        )))
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
        pool: &DatabasePool,
    ) -> LoaderResult<Vec<DataInstance>> {
        let mut instances = Vec::new();

        // Build query
        let query = format!(
            "SELECT * FROM {table_name} LIMIT {}",
            self.options.batch_size
        );

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


    /// Load data from database
    pub async fn load_from_database(
        &self,
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        // Connect to database
        let pool = self.connect().await?;
        info!("Database loader connected successfully");

        // Get all tables
        let tables = self.get_table_names(&pool).await?;
        info!("Found {} tables to load", tables.len());

        let mut all_instances = Vec::new();

        // Load data from each table
        for table_name in tables {
            debug!("Loading data from table: {}", table_name);

            // Get column information
            let columns = self.get_columns(&table_name, &pool).await?;

            // Load table data
            let instances = self.load_table_data(&table_name, &columns, schema, &pool).await?;
            info!(
                "Loaded {} instances from table {}",
                instances.len(),
                table_name
            );

            all_instances.extend(instances);
        }

        info!(
            "Total loaded {} instances from database",
            all_instances.len()
        );
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

/// Implementation of `DataLoader` trait for `DatabaseLoader`
#[async_trait]
impl DataLoader for DatabaseLoader {
    fn name(&self) -> &str {
        "Database"
    }

    fn description(&self) -> &str {
        "Loads data from SQL databases (PostgreSQL and MySQL)"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // Database loader doesn't work with file extensions
    }

    async fn load_file(
        &self,
        _path: &Path,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::Configuration(
            "Database loader does not support loading from files. Use load_from_database() instead.".to_string()
        ))
    }

    async fn load_string(
        &self,
        _content: &str,
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::Configuration(
            "Database loader does not support loading from strings. Use load_from_database() instead.".to_string()
        ))
    }

    async fn load_bytes(
        &self,
        _data: &[u8],
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::Configuration(
            "Database loader does not support loading from bytes. Use load_from_database() instead.".to_string()
        ))
    }

    fn validate_schema(&self, schema: &SchemaDefinition) -> LoaderResult<()> {
        // Validate that the schema has classes that can be mapped to database tables
        if schema.classes.is_empty() {
            return Err(LoaderError::SchemaValidation(
                "Schema must have at least one class for database loading".to_string()
            ));
        }

        // Additional validations can be added here
        for (class_name, class_def) in &schema.classes {
            if class_def.slots.is_empty() {
                return Err(LoaderError::SchemaValidation(
                    format!("Class '{}' must have at least one slot for database loading", class_name)
                ));
            }
        }

        Ok(())
    }
}
