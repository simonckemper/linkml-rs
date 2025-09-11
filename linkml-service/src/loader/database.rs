//! Database loader and dumper for `LinkML`
//!
//! This module provides functionality to load data from SQL databases
//! and dump `LinkML` instances back to databases.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumperError, DumperResult, LoaderError, LoaderResult,
    DumpOptions, LoadOptions};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{json, Value};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Column, Row};
use sqlx::postgres::PgRow;
use sqlx::mysql::MySqlRow;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::{debug, info};

/// Database pool enum to handle different database types without Any
#[derive(Debug)]
enum DatabasePool {
    PostgreSQL(PgPool),
    MySQL(MySqlPool)}

/// Options for database loading and dumping
#[derive(Debug, Clone)]
pub struct DatabaseOptions {
    /// Database connection string
    pub connection_string: String,

    /// Schema name to use (if applicable)
    pub schema_name: Option<String>,

    /// Table name to class name mapping
    pub table_mapping: HashMap<String, String>,

    /// Column name to slot name mapping (per table)
    pub column_mapping: HashMap<String, HashMap<String, String>>,

    /// Tables to exclude from loading
    pub exclude_tables: HashSet<String>,

    /// Include only these tables
    pub include_tables: Option<HashSet<String>>,

    /// Primary key column names (if not standard)
    pub primary_key_columns: HashMap<String, String>,

    /// Foreign key relationships
    pub foreign_keys: HashMap<String, Vec<ForeignKeyRelation>>,

    /// Batch size for loading/dumping
    pub batch_size: usize,

    /// Whether to infer types from database schema
    pub infer_types: bool,

    /// Whether to create tables if they don't exist
    pub create_if_not_exists: bool,

    /// Whether to use transactions
    pub use_transactions: bool,

    /// Maximum number of connections in the pool
    pub max_connections: u32}

/// Foreign key relationship definition
#[derive(Debug, Clone)]
pub struct ForeignKeyRelation {
    /// Column in the source table
    pub column: String,

    /// Referenced table
    pub referenced_table: String,

    /// Referenced column
    pub referenced_column: String,

    /// Slot name in `LinkML` schema
    pub slot_name: String}

impl Default for DatabaseOptions {
    fn default() -> Self {
        Self {
            connection_string: String::new(),
            schema_name: None,
            table_mapping: HashMap::new(),
            column_mapping: HashMap::new(),
            exclude_tables: HashSet::new(),
            include_tables: None,
            primary_key_columns: HashMap::new(),
            foreign_keys: HashMap::new(),
            batch_size: 1000,
            infer_types: true,
            create_if_not_exists: false,
            use_transactions: true,
            max_connections: 5}
    }
}

/// Database loader for `LinkML` data
pub struct DatabaseLoader {
    options: DatabaseOptions,
    pool: Option<DatabasePool>}

impl DatabaseLoader {
    /// Create a new database loader
    #[must_use] pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None}
    }

    /// Execute a query and return results as a vector of string maps
    async fn execute_query_as_maps(&self, query: &str) -> LoaderResult<Vec<HashMap<String, String>>> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => {
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| LoaderError::Io(std::io::Error::other(
                        format!("PostgreSQL query failed: {e}"),
                    )))?;

                let mut results = Vec::new();
                for row in rows {
                    let mut map = HashMap::new();
                    for (i, column) in row.columns().iter().enumerate() {
                        let value: Option<String> = row.try_get(i).unwrap_or(None);
                        map.insert(column.name().to_string(), value.unwrap_or_default());
                    }
                    results.push(map);
                }
                Ok(results)
            }
            Some(DatabasePool::MySQL(pool)) => {
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| LoaderError::Io(std::io::Error::other(
                        format!("MySQL query failed: {e}"),
                    )))?;

                let mut results = Vec::new();
                for row in rows {
                    let mut map = HashMap::new();
                    for (i, column) in row.columns().iter().enumerate() {
                        let value: Option<String> = row.try_get(i).unwrap_or(None);
                        map.insert(column.name().to_string(), value.unwrap_or_default());
                    }
                    results.push(map);
                }
                Ok(results)
            }
            None => Err(LoaderError::Configuration("No database connection available".to_string()))}
    }

    /// Get the Postgre`SQL` pool if available
    fn get_pg_pool(&self) -> LoaderResult<&PgPool> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => Ok(pool),
            _ => Err(LoaderError::Configuration("PostgreSQL pool not available".to_string()))}
    }

    /// Get the My`SQL` pool if available
    fn get_mysql_pool(&self) -> LoaderResult<&MySqlPool> {
        match self.pool.as_ref() {
            Some(DatabasePool::MySQL(pool)) => Ok(pool),
            _ => Err(LoaderError::Configuration("MySQL pool not available".to_string()))}
    }

    /// Connect to the database
    async fn connect(&mut self) -> LoaderResult<()> {
        if self.pool.is_none() {
            let pool = if self.options.connection_string.starts_with("postgres://") {
                let pg_pool = PgPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| LoaderError::Configuration(format!("Failed to connect to PostgreSQL: {e}")))?;
                DatabasePool::PostgreSQL(pg_pool)
            } else if self.options.connection_string.starts_with("mysql://") {
                let mysql_pool = MySqlPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| LoaderError::Configuration(format!("Failed to connect to MySQL: {e}")))?;
                DatabasePool::MySQL(mysql_pool)
            } else {
                return Err(LoaderError::Configuration(
                    "Unsupported database type. Only PostgreSQL and MySQL are supported.".to_string()
                ));
            };

            self.pool = Some(pool);
        }
        Ok(())
    }

    /// Get the connection pool
    fn get_pool(&self) -> LoaderResult<&DatabasePool> {
        self.pool.as_ref().ok_or_else(|| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Database not connected",
            ))
        })
    }

    /// Get table names from the database
    async fn get_table_names(&self) -> LoaderResult<Vec<String>> {
        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'
                         ORDER BY table_name"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'public' AND table_type = 'BASE TABLE'
                     ORDER BY table_name".to_string()
                }
            }
            DatabaseType::MySQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'
                         ORDER BY table_name"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'
                     ORDER BY table_name".to_string()
                }
            }
            DatabaseType::SQLite => {
                return Err(LoaderError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        let rows = self.execute_query_as_maps(&query).await?;

        let mut tables = Vec::new();
        for row in rows {
            if let Some(table_name) = row.get("table_name") {
                // Apply filters
                if self.options.exclude_tables.contains(table_name) {
                    continue;
                }

                if let Some(ref include_tables) = self.options.include_tables
                    && !include_tables.contains(table_name) {
                        continue;
                    }

                tables.push(table_name.clone());
            }
        }

        info!("Found {} tables: {:?}", tables.len(), tables);
        Ok(tables)
    }

    /// Get table names from the database (original implementation - now restored)
    async fn _get_table_names_original(&self) -> LoaderResult<Vec<String>> {
        let pool = self.get_pool()?;

        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
                        .to_string()
                }
            }
            DatabaseType::MySQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'"
                        .to_string()
                }
            }
            DatabaseType::SQLite => "SELECT name FROM sqlite_master WHERE type='table'
                 AND name NOT LIKE 'sqlite_%'"
                .to_string(),
        };

        let mut tables = Vec::new();
        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query tables: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        // Apply filtering
                        if self.options.exclude_tables.contains(&table_name) {
                            continue;
                        }

                        if let Some(include) = &self.options.include_tables
                            && !include.contains(&table_name) {
                                continue;
                            }

                        tables.push(table_name);
                    }
                }
            }
            DatabasePool::MySQL(mysql_pool) => {
                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query tables: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        // Apply filtering
                        if self.options.exclude_tables.contains(&table_name) {
                            continue;
                        }

                        if let Some(include) = &self.options.include_tables
                            && !include.contains(&table_name) {
                                continue;
                            }

                        tables.push(table_name);
                    }
                }
            }
        }

        Ok(tables)
    }

    /// Get table names from the database (complex implementation - now restored)
    async fn _get_table_names_complex(&self) -> LoaderResult<Vec<String>> {
        let pool = self.get_pool()?;

        // This query works for PostgreSQL, MySQL, and SQLite
        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = 'public' AND table_type = 'BASE TABLE'"
                        .to_string()
                }
            }
            DatabaseType::MySQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{schema}' AND table_type = 'BASE TABLE'"
                    )
                } else {
                    "SELECT table_name FROM information_schema.tables
                     WHERE table_schema = DATABASE() AND table_type = 'BASE TABLE'"
                        .to_string()
                }
            }
            DatabaseType::SQLite => "SELECT name FROM sqlite_master WHERE type='table'
                 AND name NOT LIKE 'sqlite_%'"
                .to_string(),
        };

        let mut tables = Vec::new();
        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query tables: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        // Apply filtering
                        if self.options.exclude_tables.contains(&table_name) {
                            continue;
                        }

                        if let Some(include) = &self.options.include_tables
                            && !include.contains(&table_name) {
                                continue;
                            }

                        tables.push(table_name);
                    }
                }
            }
            DatabasePool::MySQL(mysql_pool) => {
                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query tables: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(table_name) = row.try_get::<String, _>(0) {
                        // Apply filtering
                        if self.options.exclude_tables.contains(&table_name) {
                            continue;
                        }

                        if let Some(include) = &self.options.include_tables
                            && !include.contains(&table_name) {
                                continue;
                            }

                        tables.push(table_name);
                    }
                }
            }
        }

        Ok(tables)
    }

    /// Get database type from connection string
    fn get_database_type(&self) -> LoaderResult<DatabaseType> {
        if self.options.connection_string.starts_with("postgresql://")
            || self.options.connection_string.starts_with("postgres://")
        {
            Ok(DatabaseType::PostgreSQL)
        } else if self.options.connection_string.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
        } else if self.options.connection_string.starts_with("sqlite://") {
            Err(LoaderError::Configuration(
                "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
            ))
        } else {
            Err(LoaderError::Configuration(
                "Unsupported database type in connection string".to_string(),
            ))
        }
    }

    /// Get column information for a table
    async fn get_columns(&self, table_name: &str) -> LoaderResult<Vec<ColumnInfo>> {
        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                let schema = self.options.schema_name.as_deref().unwrap_or("public");
                format!(
                    "SELECT
                        column_name,
                        data_type,
                        is_nullable,
                        column_default
                     FROM information_schema.columns
                     WHERE table_schema = '{schema}' AND table_name = '{table_name}'
                     ORDER BY ordinal_position"
                )
            }
            DatabaseType::MySQL => {
                let schema = if let Some(ref schema_name) = self.options.schema_name {
                    schema_name.clone()
                } else {
                    "DATABASE()".to_string()
                };

                if schema == "DATABASE()" {
                    format!(
                        "SELECT
                            column_name,
                            data_type,
                            is_nullable,
                            column_default
                         FROM information_schema.columns
                         WHERE table_schema = DATABASE() AND table_name = '{table_name}'
                         ORDER BY ordinal_position"
                    )
                } else {
                    format!(
                        "SELECT
                            column_name,
                            data_type,
                            is_nullable,
                            column_default
                         FROM information_schema.columns
                         WHERE table_schema = '{schema}' AND table_name = '{table_name}'
                         ORDER BY ordinal_position"
                    )
                }
            }
            DatabaseType::SQLite => {
                return Err(LoaderError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        let rows = self.execute_query_as_maps(&query).await?;

        let mut columns = Vec::new();
        for row in rows {
            if let (Some(column_name), Some(data_type), Some(is_nullable)) = (
                row.get("column_name"),
                row.get("data_type"),
                row.get("is_nullable"),
            ) {
                columns.push(ColumnInfo {
                    name: column_name.clone(),
                    data_type: data_type.clone(),
                    is_nullable: is_nullable.to_lowercase() == "yes",
                    is_primary_key: false, // Will be determined separately
                });
            }
        }

        // Get primary key information
        self.update_primary_keys(&mut columns, table_name).await?;

        debug!("Found {} columns for table {}: {:?}", columns.len(), table_name,
               columns.iter().map(|c| &c.name).collect::<Vec<_>>());

        Ok(columns)
    }

    /// Get column information for a table (original implementation - commented out)
    async fn _get_columns_original(&self, table_name: &str) -> LoaderResult<Vec<ColumnInfo>> {
        let pool = self.get_pool()?;

        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                format!(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_name = '{}' {}
                     ORDER BY ordinal_position",
                    table_name,
                    if let Some(schema) = &self.options.schema_name {
                        format!("AND table_schema = '{schema}'")
                    } else {
                        "AND table_schema = 'public'".to_string()
                    }
                )
            }
            DatabaseType::MySQL => {
                format!(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_name = '{}' {}
                     ORDER BY ordinal_position",
                    table_name,
                    if let Some(schema) = &self.options.schema_name {
                        format!("AND table_schema = '{schema}'")
                    } else {
                        "AND table_schema = DATABASE()".to_string()
                    }
                )
            }
            DatabaseType::SQLite => {
                format!("PRAGMA table_info({table_name})")
            }
        };

        let mut columns = Vec::new();

        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query columns: {e}"),
                    ))
                })?;

                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get::<String, _>(0).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get column name: {e}"))
                        ))?,
                        data_type: row.try_get::<String, _>(1).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get data type: {e}"))
                        ))?,
                        is_nullable: row.try_get::<String, _>(2).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get nullable: {e}"))
                        ))? == "YES",
                        is_primary_key: false, // Will be determined separately
                    });
                }

                // Get primary key information
                self.update_primary_keys(&mut columns, table_name).await?;
            }
            DatabasePool::MySQL(mysql_pool) => {
                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query columns: {e}"),
                    ))
                })?;

                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get::<String, _>(0).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get column name: {e}"))
                        ))?,
                        data_type: row.try_get::<String, _>(1).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get data type: {e}"))
                        ))?,
                        is_nullable: row.try_get::<String, _>(2).map_err(|e| LoaderError::Io(
                            std::io::Error::other(format!("Failed to get nullable: {e}"))
                        ))? == "YES",
                        is_primary_key: false, // Will be determined separately
                    });
                }

                // Get primary key information
                self.update_primary_keys(&mut columns, table_name).await?;
            }
        }

        Ok(columns)
    }

    /// Update primary key information for columns
    async fn update_primary_keys(
        &self,
        columns: &mut [ColumnInfo],
        table_name: &str,
    ) -> LoaderResult<()> {
        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                let schema = self.options.schema_name.as_deref().unwrap_or("public");
                format!(
                    "SELECT column_name
                     FROM information_schema.key_column_usage
                     WHERE table_schema = '{schema}' AND table_name = '{table_name}'
                       AND constraint_name IN (
                           SELECT constraint_name
                           FROM information_schema.table_constraints
                           WHERE table_schema = '{schema}' AND table_name = '{table_name}'
                             AND constraint_type = 'PRIMARY KEY'
                       )"
                )
            }
            DatabaseType::MySQL => {
                let schema = if let Some(ref schema_name) = self.options.schema_name {
                    schema_name.clone()
                } else {
                    "DATABASE()".to_string()
                };

                if schema == "DATABASE()" {
                    format!(
                        "SELECT column_name
                         FROM information_schema.key_column_usage
                         WHERE table_schema = DATABASE() AND table_name = '{table_name}'
                           AND constraint_name = 'PRIMARY'"
                    )
                } else {
                    format!(
                        "SELECT column_name
                         FROM information_schema.key_column_usage
                         WHERE table_schema = '{schema}' AND table_name = '{table_name}'
                           AND constraint_name = 'PRIMARY'"
                    )
                }
            }
            DatabaseType::SQLite => {
                return Err(LoaderError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        let rows = self.execute_query_as_maps(&query).await?;

        let mut pk_columns = HashSet::new();
        for row in rows {
            if let Some(column_name) = row.get("column_name") {
                pk_columns.insert(column_name.clone());
            }
        }

        for column in columns {
            if pk_columns.contains(&column.name) {
                column.is_primary_key = true;
            }
        }

        debug!("Primary key columns for table {}: {:?}", table_name, pk_columns);
        Ok(())
    }

    /// Update primary key information for columns (original implementation - commented out)
    async fn _update_primary_keys_original(
        &self,
        columns: &mut [ColumnInfo],
        table_name: &str,
    ) -> LoaderResult<()> {
        let pool = self.get_pool()?;

        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                format!(
                    "SELECT kcu.column_name
                     FROM information_schema.table_constraints tc
                     JOIN information_schema.key_column_usage kcu
                     ON tc.constraint_name = kcu.constraint_name
                     WHERE tc.table_name = '{}'
                     AND tc.constraint_type = 'PRIMARY KEY' {}",
                    table_name,
                    if let Some(schema) = &self.options.schema_name {
                        format!("AND tc.table_schema = '{schema}'")
                    } else {
                        "AND tc.table_schema = 'public'".to_string()
                    }
                )
            }
            DatabaseType::MySQL => {
                format!(
                    "SELECT column_name
                     FROM information_schema.key_column_usage
                     WHERE table_name = '{}'
                     AND constraint_name = 'PRIMARY' {}",
                    table_name,
                    if let Some(schema) = &self.options.schema_name {
                        format!("AND table_schema = '{schema}'")
                    } else {
                        "AND table_schema = DATABASE()".to_string()
                    }
                )
            }
            _ => return Ok(()), // SQLite handled differently
        };

        let mut pk_columns = HashSet::new();
        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let rows = sqlx::query(&query).fetch_all(pg_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query primary keys: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(col_name) = row.try_get::<String, _>(0) {
                        pk_columns.insert(col_name);
                    }
                }
            }
            DatabasePool::MySQL(mysql_pool) => {
                let rows = sqlx::query(&query).fetch_all(mysql_pool).await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to query primary keys: {e}"),
                    ))
                })?;

                for row in rows {
                    if let Ok(col_name) = row.try_get::<String, _>(0) {
                        pk_columns.insert(col_name);
                    }
                }
            }
        }

        for column in columns {
            if pk_columns.contains(&column.name) {
                column.is_primary_key = true;
            }
        }

        Ok(())
    }

    /// Convert database type to `LinkML` range
    fn db_type_to_linkml_range(&self, db_type: &str) -> String {
        match db_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "int8" | "bigint" | "smallint" => "integer".to_string(),
            "real" | "double" | "float" | "float4" | "float8" | "decimal" | "numeric" => "float".to_string(),
            "boolean" | "bool" => "boolean".to_string(),
            "text" | "varchar" | "char" | "character varying" => "string".to_string(),
            "date" => "date".to_string(),
            "timestamp"
            | "datetime"
            | "timestamp with time zone"
            | "timestamp without time zone" => "datetime".to_string(),
            "time" | "time with time zone" | "time without time zone" => "time".to_string(),
            _ => "string".to_string(), // Default to string for unknown types
        }
    }

    /// Load data from a table
    async fn load_table_data(
        &self,
        table_name: &str,
        columns: &[ColumnInfo],
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let query = format!("SELECT * FROM {} LIMIT {}", table_name, self.options.batch_size);
        let rows = self.execute_query_as_maps(&query).await?;

        let mut instances = Vec::new();

        // Get class name for this table
        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Get column mapping for this table
        let column_mapping = self.options.column_mapping.get(table_name);

        for row in rows {
            let mut data = HashMap::new();

            for column in columns {
                let column_name = &column.name;
                let mapped_name = column_mapping
                    .and_then(|mapping| mapping.get(column_name))
                    .cloned()
                    .unwrap_or_else(|| to_snake_case(column_name));

                if let Some(raw_value) = row.get(column_name) {
                    // Convert the string value to appropriate JSON value based on column type
                    let value = self.convert_db_value_to_json(raw_value, &column.data_type)?;

                    if !value.is_null() {
                        data.insert(mapped_name, value);
                    }
                }
            }

            // Handle foreign key references
            if let Some(fk_relations) = self.options.foreign_keys.get(table_name) {
                for fk_relation in fk_relations {
                    if let Some(Value::String(ref_id)) = data.get(&fk_relation.column) {
                        // Create a reference to the related object
                        let ref_instance = json!({
                            "@type": fk_relation.referenced_table.clone(),
                            "id": ref_id
                        });
                        data.insert(fk_relation.slot_name.clone(), ref_instance);
                    }
                }
            }

            let instance = DataInstance {
                class_name: class_name.clone(),
                data,
                id: None,
                metadata: HashMap::new()};

            instances.push(instance);
        }

        info!("Loaded {} instances from table {}", instances.len(), table_name);
        Ok(instances)
    }

    /// Convert database string value to appropriate `JSON` value based on column type
    fn convert_db_value_to_json(&self, raw_value: &str, data_type: &str) -> LoaderResult<Value> {
        if raw_value.is_empty() {
            return Ok(Value::Null);
        }

        match data_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "smallint" | "bigint" | "int8" => {
                raw_value.parse::<i64>()
                    .map(Value::from)
                    .or_else(|_| raw_value.parse::<i32>().map(Value::from))
                    .map_err(|e| LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse integer '{raw_value}': {e}"),
                    )))
            }
            "real" | "double" | "float" | "float4" | "float8" | "decimal" | "numeric" => {
                raw_value.parse::<f64>()
                    .map(Value::from)
                    .map_err(|e| LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse float '{raw_value}': {e}"),
                    )))
            }
            "boolean" | "bool" => {
                match raw_value.to_lowercase().as_str() {
                    "true" | "t" | "1" | "yes" | "y" => Ok(Value::Bool(true)),
                    "false" | "f" | "0" | "no" | "n" => Ok(Value::Bool(false)),
                    _ => Err(LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Failed to parse boolean '{raw_value}'"),
                    )))
                }
            }
            "date" | "timestamp" | "datetime" | "time" => {
                // Keep date/time values as strings for now
                // Could be enhanced to parse into proper date objects
                Ok(Value::String(raw_value.to_string()))
            }
            _ => {
                // Default to string for all other types
                Ok(Value::String(raw_value.to_string()))
            }
        }
    }

    /// Load data from a table (original implementation - commented out)
    async fn _load_table_data_original(
        &self,
        table_name: &str,
        columns: &[ColumnInfo],
        _schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let pool = self.get_pool()?;
        let mut instances = Vec::new();

        // Build query
        let query = format!("SELECT * FROM {table_name}");
        
        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                let mut rows = sqlx::query(&query).fetch(pg_pool);
                let mut batch = Vec::new();

                while let Some(row) = rows.try_next().await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to fetch row: {e}"),
                    ))
                })? {
                    let instance = self.pg_row_to_instance(&row, table_name, columns)?;
                    batch.push(instance);

                    if batch.len() >= self.options.batch_size {
                        instances.append(&mut batch);
                    }
                }
                instances.extend(batch);
            }
            DatabasePool::MySQL(mysql_pool) => {
                let mut rows = sqlx::query(&query).fetch(mysql_pool);
                let mut batch = Vec::new();

                while let Some(row) = rows.try_next().await.map_err(|e| {
                    LoaderError::Io(std::io::Error::other(
                        format!("Failed to fetch row: {e}"),
                    ))
                })? {
                    let instance = self.mysql_row_to_instance(&row, table_name, columns)?;
                    batch.push(instance);

                    if batch.len() >= self.options.batch_size {
                        instances.append(&mut batch);
                    }
                }
                instances.extend(batch);
            }
        }

        // Apply foreign key relationships
        if let Some(fk_relations) = self.options.foreign_keys.get(table_name) {
            for instance in &mut instances {
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

        Ok(instances)
    }

    /// Convert a database row to a `DataInstance` (simplified version)
    fn create_instance_from_data(
        &self,
        table_name: &str,
        data: HashMap<String, Value>,
    ) -> LoaderResult<DataInstance> {
        // Determine class name
        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        Ok(DataInstance {
            class_name,
            data,
            id: None,
            metadata: HashMap::new()})
    }

    /// Convert a database row to a `DataInstance` (`PostgreSQL` version)
    fn pg_row_to_instance(
        &self,
        row: &PgRow,
        table_name: &str,
        columns: &[ColumnInfo],
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();
        
        // Get class name for this table
        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Get column mapping for this table
        let column_mapping = self.options.column_mapping.get(table_name);

        for (i, column) in columns.iter().enumerate() {
            let column_name = &column.name;
            let mapped_name = column_mapping
                .and_then(|mapping| mapping.get(column_name))
                .cloned()
                .unwrap_or_else(|| to_snake_case(column_name));

            // Extract value with proper type handling
            let value = self.get_pg_column_value(row, i, &column.data_type)?;
            
            if !value.is_null() {
                data.insert(mapped_name, value);
            }
        }

        Ok(DataInstance {
            class_name,
            data,
            id: None,
            metadata: HashMap::new()})
    }

    /// Convert a database row to a `DataInstance` (`MySQL` version)
    fn mysql_row_to_instance(
        &self,
        row: &MySqlRow,
        table_name: &str,
        columns: &[ColumnInfo],
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();
        
        // Get class name for this table
        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Get column mapping for this table
        let column_mapping = self.options.column_mapping.get(table_name);

        for (i, column) in columns.iter().enumerate() {
            let column_name = &column.name;
            let mapped_name = column_mapping
                .and_then(|mapping| mapping.get(column_name))
                .cloned()
                .unwrap_or_else(|| to_snake_case(column_name));

            // Extract value with proper type handling
            let value = self.get_mysql_column_value(row, i, &column.data_type)?;
            
            if !value.is_null() {
                data.insert(mapped_name, value);
            }
        }

        Ok(DataInstance {
            class_name,
            data,
            id: None,
            metadata: HashMap::new()})
    }

    /// Get column value with proper type conversion (`PostgreSQL`)
    fn get_pg_column_value(&self, row: &PgRow, idx: usize, db_type: &str) -> LoaderResult<Value> {
        use sqlx::Row;
        
        // Try to decode based on the database type
        match db_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "smallint" => {
                if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "bigint" | "int8" => {
                if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "real" | "float" | "float4" => {
                if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
                    Ok(val.map_or(Value::Null, |v| Value::from(f64::from(v))))
                } else {
                    Ok(Value::Null)
                }
            }
            "double" | "float8" | "decimal" | "numeric" => {
                if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "boolean" | "bool" => {
                if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => {
                // Default to string
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    /// Get column value with proper type conversion (`MySQL`)
    fn get_mysql_column_value(&self, row: &MySqlRow, idx: usize, db_type: &str) -> LoaderResult<Value> {
        use sqlx::Row;
        
        // Try to decode based on the database type
        match db_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "smallint" => {
                if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "bigint" | "int8" => {
                if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "real" | "float" | "float4" => {
                if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
                    Ok(val.map_or(Value::Null, |v| Value::from(f64::from(v))))
                } else {
                    Ok(Value::Null)
                }
            }
            "double" | "float8" | "decimal" | "numeric" => {
                if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "boolean" | "bool" => {
                if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => {
                // Default to string
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }

    // TODO: Implement database-specific row parsing
    // This function needs to be rewritten to work with specific database row types
    /*
    /// Get column value with proper type conversion
    fn get_column_value(&self, row: &AnyRow, idx: usize, db_type: &str) -> LoaderResult<Value> {
        // Try to decode based on the database type
        match db_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "smallint" => {
                if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "bigint" | "int8" => {
                if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "real" | "float" | "float4" => {
                if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
                    Ok(val.map(|v| Value::from(v as f64)).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "double" | "float8" | "decimal" | "numeric" => {
                if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            "boolean" | "bool" => {
                if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
            _ => {
                // Default to string
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    Ok(val.map(Value::from).unwrap_or(Value::Null))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }
    */
    /// Helper method for database loading
    async fn load_from_database(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
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

#[async_trait]
impl DataLoader for DatabaseLoader {
    fn name(&self) -> &'static str {
        "Database Loader"
    }

    fn description(&self) -> &'static str {
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
            "Database loader does not support file loading. Use load_string with connection string.".to_string()
        ))
    }

    async fn load_string(
        &self,
        content: &str,
        schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        // For database loader, the "content" is treated as a connection string
        let mut loader = DatabaseLoader::new(DatabaseOptions {
            connection_string: content.to_string(),
            schema_name: None,
            table_mapping: HashMap::new(),
            column_mapping: HashMap::new(),
            exclude_tables: HashSet::new(),
            include_tables: None,
            primary_key_columns: HashMap::new(),
            foreign_keys: HashMap::new(),
            max_connections: 10,
            batch_size: 1000,
            infer_types: true,
            create_if_not_exists: false,
            use_transactions: true});
        loader.load_from_database(schema).await
    }

    async fn load_bytes(
        &self,
        _data: &[u8],
        _schema: &SchemaDefinition,
        _options: &LoadOptions,
    ) -> LoaderResult<Vec<DataInstance>> {
        Err(LoaderError::Configuration(
            "Database loader does not support byte loading.".to_string()
        ))
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> LoaderResult<()> {
        // Basic validation - could be enhanced
        Ok(())
    }


}

/// Database dumper for `LinkML` data
pub struct DatabaseDumper {
    options: DatabaseOptions,
    pool: Option<DatabasePool>}

impl DatabaseDumper {
    /// Create a new database dumper
    #[must_use] pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None}
    }

    /// Execute a query and return results as a vector of string maps
    async fn execute_query_as_maps(&self, query: &str) -> DumperResult<Vec<HashMap<String, String>>> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => {
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| DumperError::Io(std::io::Error::other(
                        format!("PostgreSQL query failed: {e}"),
                    )))?;

                let mut results = Vec::new();
                for row in rows {
                    let mut map = HashMap::new();
                    for (i, column) in row.columns().iter().enumerate() {
                        let value: Option<String> = row.try_get(i).unwrap_or(None);
                        map.insert(column.name().to_string(), value.unwrap_or_default());
                    }
                    results.push(map);
                }
                Ok(results)
            }
            Some(DatabasePool::MySQL(pool)) => {
                let rows = sqlx::query(query)
                    .fetch_all(pool)
                    .await
                    .map_err(|e| DumperError::Io(std::io::Error::other(
                        format!("MySQL query failed: {e}"),
                    )))?;

                let mut results = Vec::new();
                for row in rows {
                    let mut map = HashMap::new();
                    for (i, column) in row.columns().iter().enumerate() {
                        let value: Option<String> = row.try_get(i).unwrap_or(None);
                        map.insert(column.name().to_string(), value.unwrap_or_default());
                    }
                    results.push(map);
                }
                Ok(results)
            }
            None => Err(DumperError::Configuration("No database connection available".to_string()))}
    }

    /// Execute a DDL/DML statement
    async fn execute_statement(&self, statement: &str) -> DumperResult<()> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => {
                sqlx::query(statement)
                    .execute(pool)
                    .await
                    .map_err(|e| DumperError::Io(std::io::Error::other(
                        format!("PostgreSQL statement failed: {e}"),
                    )))?;
                Ok(())
            }
            Some(DatabasePool::MySQL(pool)) => {
                sqlx::query(statement)
                    .execute(pool)
                    .await
                    .map_err(|e| DumperError::Io(std::io::Error::other(
                        format!("MySQL statement failed: {e}"),
                    )))?;
                Ok(())
            }
            None => Err(DumperError::Configuration("No database connection available".to_string()))}
    }

    /// Get the Postgre`SQL` pool if available
    fn get_pg_pool(&self) -> DumperResult<&PgPool> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => Ok(pool),
            _ => Err(DumperError::Configuration("PostgreSQL pool not available".to_string()))}
    }

    /// Get the My`SQL` pool if available
    fn get_mysql_pool(&self) -> DumperResult<&MySqlPool> {
        match self.pool.as_ref() {
            Some(DatabasePool::MySQL(pool)) => Ok(pool),
            _ => Err(DumperError::Configuration("MySQL pool not available".to_string()))}
    }

    /// Connect to the database
    async fn connect(&mut self) -> DumperResult<()> {
        if self.pool.is_none() {
            let pool = if self.options.connection_string.starts_with("postgres://") {
                let pg_pool = PgPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| DumperError::Configuration(format!("Failed to connect to PostgreSQL: {e}")))?;
                DatabasePool::PostgreSQL(pg_pool)
            } else if self.options.connection_string.starts_with("mysql://") {
                let mysql_pool = MySqlPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| DumperError::Configuration(format!("Failed to connect to MySQL: {e}")))?;
                DatabasePool::MySQL(mysql_pool)
            } else {
                return Err(DumperError::Configuration(
                    "Unsupported database type. Only PostgreSQL and MySQL are supported.".to_string()
                ));
            };

            self.pool = Some(pool);
        }
        Ok(())
    }

    /// Get the connection pool
    fn get_pool(&self) -> DumperResult<&DatabasePool> {
        self.pool.as_ref().ok_or_else(|| {
            DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Database not connected",
            ))
        })
    }

    /// Get database type
    fn get_database_type(&self) -> DumperResult<DatabaseType> {
        if self.options.connection_string.starts_with("postgresql://")
            || self.options.connection_string.starts_with("postgres://")
        {
            Ok(DatabaseType::PostgreSQL)
        } else if self.options.connection_string.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
        } else if self.options.connection_string.starts_with("sqlite://") {
            Err(DumperError::Configuration(
                "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
            ))
        } else {
            Err(DumperError::Configuration(
                "Unsupported database type in connection string".to_string(),
            ))
        }
    }

    /// Create table if it doesn't exist
    async fn create_table_if_needed(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        if !self.options.create_if_not_exists {
            return Ok(());
        }

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name).map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Check if table already exists
        let exists_query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                let schema_name = self.options.schema_name.as_deref().unwrap_or("public");
                format!(
                    "SELECT EXISTS (
                        SELECT FROM information_schema.tables
                        WHERE table_schema = '{schema_name}' AND table_name = '{table_name}'
                    )"
                )
            }
            DatabaseType::MySQL => {
                let schema_name = if let Some(ref schema_name) = self.options.schema_name {
                    schema_name.clone()
                } else {
                    "DATABASE()".to_string()
                };

                if schema_name == "DATABASE()" {
                    format!(
                        "SELECT COUNT(*) as count FROM information_schema.tables
                         WHERE table_schema = DATABASE() AND table_name = '{table_name}'"
                    )
                } else {
                    format!(
                        "SELECT COUNT(*) as count FROM information_schema.tables
                         WHERE table_schema = '{schema_name}' AND table_name = '{table_name}'"
                    )
                }
            }
            DatabaseType::SQLite => {
                return Err(DumperError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        let rows = self.execute_query_as_maps(&exists_query).await?;
        let table_exists = if let Some(row) = rows.first() {
            match self.get_database_type()? {
                DatabaseType::PostgreSQL => {
                    row.get("exists").is_some_and(|v| v == "t" || v == "true")
                }
                DatabaseType::MySQL => {
                    row.get("count").and_then(|v| v.parse::<i32>().ok()).unwrap_or(0) > 0
                }
                _ => false}
        } else {
            false
        };

        if table_exists {
            info!("Table {} already exists, skipping creation", table_name);
            return Ok(());
        }

        // Generate CREATE TABLE statement
        let create_sql = self.generate_create_table_sql(&table_name, class_def, schema)?;

        info!("Creating table {} with SQL: {}", table_name, create_sql);
        self.execute_statement(&create_sql).await?;

        info!("Successfully created table {}", table_name);
        Ok(())
    }

    /// Generate CREATE TABLE `SQL` statement
    fn generate_create_table_sql(
        &self,
        table_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> DumperResult<String> {
        let mut columns = Vec::new();
        let mut primary_keys = Vec::new();

        // Add ID column if not explicitly defined
        let has_id_slot = class_def.slots.iter().any(|slot_name| {
            schema.slots.get(slot_name)
                .is_some_and(|slot| slot.identifier.unwrap_or(false))
        });

        if !has_id_slot {
            let id_type = match self.get_database_type()? {
                DatabaseType::PostgreSQL => "SERIAL PRIMARY KEY",
                DatabaseType::MySQL => "INT AUTO_INCREMENT PRIMARY KEY",
                _ => "INTEGER PRIMARY KEY"};
            columns.push(format!("id {id_type}"));
        }

        // Process each slot
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let column_name = self.options.column_mapping
                    .get(table_name)
                    .and_then(|mapping| mapping.get(slot_name))
                    .cloned()
                    .unwrap_or_else(|| to_snake_case(slot_name));

                let db_type = self.linkml_range_to_db_type(&slot_def.range.clone().unwrap_or_else(|| "string".to_string()))?;

                let mut column_def = format!("{column_name} {db_type}");

                // Add constraints
                if slot_def.required.unwrap_or(false) {
                    column_def.push_str(" NOT NULL");
                }

                if slot_def.identifier.unwrap_or(false) {
                    primary_keys.push(column_name.clone());
                }

                columns.push(column_def);
            }
        }

        // Build the CREATE TABLE statement
        let mut sql = format!("CREATE TABLE {table_name} (\n");
        sql.push_str(&columns.join(",\n  "));

        if !primary_keys.is_empty() && has_id_slot {
            sql.push_str(&format!(",\n  PRIMARY KEY ({})", primary_keys.join(", ")));
        }

        sql.push_str("\n)");

        Ok(sql)
    }

    /// Convert `LinkML` range to database type
    fn linkml_range_to_db_type(&self, range: &str) -> DumperResult<String> {
        let db_type = match self.get_database_type()? {
            DatabaseType::PostgreSQL => match range {
                "string" => "TEXT",
                "integer" => "INTEGER",
                "float" => "DOUBLE PRECISION",
                "boolean" => "BOOLEAN",
                "date" => "DATE",
                "datetime" => "TIMESTAMP",
                "time" => "TIME",
                _ => "TEXT"},
            DatabaseType::MySQL => match range {
                "string" => "TEXT",
                "integer" => "INT",
                "float" => "DOUBLE",
                "boolean" => "BOOLEAN",
                "date" => "DATE",
                "datetime" => "DATETIME",
                "time" => "TIME",
                _ => "TEXT"},
            DatabaseType::SQLite => {
                return Err(DumperError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        Ok(db_type.to_string())
    }

    /// Create table if it doesn't exist (original implementation - commented out)
    async fn _create_table_if_needed_original(
        &self,
        class_name: &str,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        if !self.options.create_if_not_exists {
            return Ok(());
        }

        let pool = self.get_pool()?;

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name).map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Build CREATE TABLE statement
        let mut columns = Vec::new();

        // Add ID column (assuming all classes have an identifier)
        columns.push("id VARCHAR(255) PRIMARY KEY".to_string());

        // Add columns for each slot
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let column_name = to_snake_case(slot_name);
                let column_type =
                    self.linkml_range_to_db_type(slot_def.range.as_deref().unwrap_or("string"))?;

                let nullable = if slot_def.required == Some(true) {
                    "NOT NULL"
                } else {
                    ""
                };

                columns.push(
                    format!("{column_name} {column_type} {nullable}")
                        .trim()
                        .to_string(),
                );
            }
        }

        let create_sql = format!(
            "CREATE TABLE IF NOT EXISTS {} ({})",
            table_name,
            columns.join(", ")
        );

        match pool {
            DatabasePool::PostgreSQL(pg_pool) => {
                sqlx::query(&create_sql).execute(pg_pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to create table: {e}"),
                    ))
                })?;
            }
            DatabasePool::MySQL(mysql_pool) => {
                sqlx::query(&create_sql).execute(mysql_pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to create table: {e}"),
                    ))
                })?;
            }
        }

        Ok(())
    }


    /// Insert instances for a class
    async fn insert_instances(
        &self,
        class_name: &str,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        if instances.is_empty() {
            return Ok(());
        }

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name).map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Verify class exists in schema
        let _class_def = schema.classes.get(class_name)
            .ok_or_else(|| DumperError::Configuration(format!("Class {class_name} not found in schema")))?;

        // Collect all unique columns from instances
        let mut all_columns = HashSet::new();
        for instance in instances {
            for key in instance.data.keys() {
                all_columns.insert(key.clone());
            }
        }
        let columns: Vec<String> = all_columns.into_iter().collect();

        if columns.is_empty() {
            info!("No data to insert for class {}", class_name);
            return Ok(());
        }

        // Generate INSERT statement
        let placeholders = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                (1..=columns.len()).map(|i| format!("${i}")).collect::<Vec<_>>().join(", ")
            }
            DatabaseType::MySQL => {
                vec!["?"; columns.len()].join(", ")
            }
            DatabaseType::SQLite => {
                return Err(DumperError::Configuration(
                    "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
                ));
            }
        };

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders
        );

        info!("Inserting {} instances into table {} with SQL: {}", instances.len(), table_name, insert_sql);

        // Use transactions for batch insertion
        if self.options.use_transactions {
            self.insert_with_transaction(&insert_sql, &columns, instances).await?;
        } else {
            self.insert_without_transaction(&insert_sql, &columns, instances).await?;
        }

        info!("Successfully inserted {} instances into table {}", instances.len(), table_name);
        Ok(())
    }

    /// Insert instances using transactions
    async fn insert_with_transaction(
        &self,
        insert_sql: &str,
        columns: &[String],
        instances: &[DataInstance],
    ) -> DumperResult<()> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => {
                let mut tx = pool.begin().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to begin PostgreSQL transaction: {e}"),
                    ))
                })?;

                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance.data.get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(&mut *tx).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to insert PostgreSQL row: {e}"),
                        ))
                    })?;
                }

                tx.commit().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to commit PostgreSQL transaction: {e}"),
                    ))
                })?;
            }
            Some(DatabasePool::MySQL(pool)) => {
                let mut tx = pool.begin().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to begin MySQL transaction: {e}"),
                    ))
                })?;

                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance.data.get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(&mut *tx).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to insert MySQL row: {e}"),
                        ))
                    })?;
                }

                tx.commit().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(
                        format!("Failed to commit MySQL transaction: {e}"),
                    ))
                })?;
            }
            None => {
                return Err(DumperError::Configuration("No database connection available".to_string()));
            }
        }
        Ok(())
    }

    /// Insert instances without transactions
    async fn insert_without_transaction(
        &self,
        insert_sql: &str,
        columns: &[String],
        instances: &[DataInstance],
    ) -> DumperResult<()> {
        match self.pool.as_ref() {
            Some(DatabasePool::PostgreSQL(pool)) => {
                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance.data.get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(pool).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to insert PostgreSQL row: {e}"),
                        ))
                    })?;
                }
            }
            Some(DatabasePool::MySQL(pool)) => {
                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance.data.get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(pool).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to insert MySQL row: {e}"),
                        ))
                    })?;
                }
            }
            None => {
                return Err(DumperError::Configuration("No database connection available".to_string()));
            }
        }
        Ok(())
    }

    /// Convert `JSON` value to string for database insertion
    fn json_value_to_string(&self, value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Null => String::new(),
            _ => value.to_string()}
    }

    /// Insert instances for a class (original implementation - commented out)
    async fn _insert_instances_original(
        &self,
        class_name: &str,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<()> {
        let pool = self.get_pool()?;

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name).map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Get class definition
        let class_def = schema.classes.get(class_name).ok_or_else(|| {
            DumperError::SchemaValidation(format!("Class {class_name} not found in schema"))
        })?;

        // Get column mapping
        let column_mapping = self.options.column_mapping.get(&table_name);

        // Process in batches
        for batch in instances.chunks(self.options.batch_size) {
            // Process instances based on pool type
            match pool {
                DatabasePool::PostgreSQL(pg_pool) => {
                    let mut tx = pg_pool.begin().await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to begin PostgreSQL transaction: {e}"),
                        ))
                    })?;

                    for instance in batch {
                        // Build INSERT statement
                        let mut columns = Vec::new();
                        let mut values = Vec::new();
                        let mut placeholders = Vec::new();

                        for (slot_name, value) in &instance.data {
                            let column_name = if let Some(mapping) = column_mapping {
                                mapping
                                    .get(slot_name)
                                    .cloned()
                                    .unwrap_or_else(|| to_snake_case(slot_name))
                            } else {
                                to_snake_case(slot_name)
                            };

                            columns.push(column_name);
                            values.push(value.clone());
                            placeholders.push(format!("${}", placeholders.len() + 1));
                        }

                        let insert_sql = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            table_name,
                            columns.join(", "),
                            placeholders.join(", ")
                        );

                        // Build query with values
                        let mut query = sqlx::query(&insert_sql);
                        for value in values {
                            query = match value {
                                Value::String(s) => query.bind(s),
                                Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        query.bind(i)
                                    } else if let Some(f) = n.as_f64() {
                                        query.bind(f)
                                    } else {
                                        query.bind(n.to_string())
                                    }
                                }
                                Value::Bool(b) => query.bind(b),
                                Value::Null => query.bind(None::<String>),
                                _ => query.bind(value.to_string())};
                        }

                        query.execute(&mut *tx).await.map_err(|e| {
                            DumperError::Io(std::io::Error::other(
                                format!("Failed to insert row: {e}"),
                            ))
                        })?;
                    }

                    tx.commit().await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to commit transaction: {e}"),
                        ))
                    })?;
                }
                DatabasePool::MySQL(mysql_pool) => {
                    let mut tx = mysql_pool.begin().await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to begin MySQL transaction: {e}"),
                        ))
                    })?;

                    for instance in batch {
                        // Build INSERT statement
                        let mut columns = Vec::new();
                        let mut values = Vec::new();
                        let mut placeholders = Vec::new();

                        for (slot_name, value) in &instance.data {
                            let column_name = if let Some(mapping) = column_mapping {
                                mapping
                                    .get(slot_name)
                                    .cloned()
                                    .unwrap_or_else(|| to_snake_case(slot_name))
                            } else {
                                to_snake_case(slot_name)
                            };

                            columns.push(column_name);
                            values.push(value.clone());
                            placeholders.push("?".to_string()); // MySQL uses ? placeholders
                        }

                        let insert_sql = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            table_name,
                            columns.join(", "),
                            placeholders.join(", ")
                        );

                        // Build query with values
                        let mut query = sqlx::query(&insert_sql);
                        for value in values {
                            query = match value {
                                Value::String(s) => query.bind(s),
                                Value::Number(n) => {
                                    if let Some(i) = n.as_i64() {
                                        query.bind(i)
                                    } else if let Some(f) = n.as_f64() {
                                        query.bind(f)
                                    } else {
                                        query.bind(n.to_string())
                                    }
                                }
                                Value::Bool(b) => query.bind(b),
                                Value::Null => query.bind(None::<String>),
                                _ => query.bind(value.to_string())};
                        }

                        query.execute(&mut *tx).await.map_err(|e| {
                            DumperError::Io(std::io::Error::other(
                                format!("Failed to insert row: {e}"),
                            ))
                        })?;
                    }

                    tx.commit().await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(
                            format!("Failed to commit transaction: {e}"),
                        ))
                    })?;
                }
            }
        }

        Ok(())
    }

    /// Helper method for database dumping
    async fn dump_to_database(
        &mut self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<Vec<u8>> {
        // Connect to database
        self.connect().await?;
        info!("Database dumper connected successfully");

        // Group instances by class
        let mut instances_by_class: HashMap<String, Vec<&DataInstance>> = HashMap::new();
        for instance in instances {
            instances_by_class
                .entry(instance.class_name.clone())
                .or_default()
                .push(instance);
        }

        // Create tables if needed and insert data
        for (class_name, class_instances) in instances_by_class {
            if let Some(class_def) = schema.classes.get(&class_name) {
                // Create table if needed
                self.create_table_if_needed(&class_name, class_def, schema)
                    .await?;

                // Convert references to owned instances
                let owned_instances: Vec<DataInstance> =
                    class_instances.into_iter().cloned().collect();

                // Insert instances
                self.insert_instances(&class_name, &owned_instances, schema)
                    .await?;

                info!(
                    "Dumped {} instances of class {}",
                    owned_instances.len(),
                    class_name
                );
            }
        }

        // Return summary as bytes
        let summary = format!(
            "Successfully dumped {} instances to database",
            instances.len()
        );
        Ok(summary.into_bytes())
    }
}

#[async_trait]
impl DataDumper for DatabaseDumper {
    fn name(&self) -> &'static str {
        "Database Dumper"
    }

    fn description(&self) -> &'static str {
        "Dumps data to SQL databases (PostgreSQL and MySQL)"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec![] // Database dumper doesn't work with file extensions
    }

    async fn dump_file(
        &self,
        _instances: &[DataInstance],
        _path: &Path,
        _schema: &SchemaDefinition,
        _options: &DumpOptions,
    ) -> DumperResult<()> {
        Err(DumperError::Configuration(
            "Database dumper does not support file dumping. Use dump_string.".to_string()
        ))
    }

    async fn dump_string(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        _options: &DumpOptions,
    ) -> DumperResult<String> {
        // For database dumper, return a summary of what was dumped
        let mut dumper = DatabaseDumper::new(self.options.clone());
        let result = dumper.dump_to_database(instances, schema).await?;
        Ok(String::from_utf8(result).unwrap_or_else(|_| "Database dump completed".to_string()))
    }

    async fn dump_bytes(
        &self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
        _options: &DumpOptions,
    ) -> DumperResult<Vec<u8>> {
        let mut dumper = DatabaseDumper::new(self.options.clone());
        dumper.dump_to_database(instances, schema).await
    }

    fn validate_schema(&self, _schema: &SchemaDefinition) -> DumperResult<()> {
        // Basic validation - could be enhanced
        Ok(())
    }


}

/// Column information
#[derive(Debug, Clone)]
struct ColumnInfo {
    name: String,
    data_type: String,
    is_nullable: bool,
    is_primary_key: bool}

/// Database type enumeration
#[derive(Debug, Clone, Copy)]
enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite}

/// Convert string to pascal case
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str()}
        })
        .collect()
}

/// Convert string to snake case
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        // to_lowercase() for ASCII characters always produces at least one char
        result.push(
            ch.to_lowercase()
                .next()
                .expect("to_lowercase() should produce at least one char"),
        );
        prev_upper = ch.is_uppercase();
    }

    result
}

// Helper trait imports for async operations
use futures::{TryStreamExt, StreamExt};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pascal_case_conversion() {
        assert_eq!(to_pascal_case("user_account"), "UserAccount");
        assert_eq!(to_pascal_case("order_items"), "OrderItems");
        assert_eq!(to_pascal_case("product"), "Product");
    }

    #[test]
    fn test_snake_case_conversion() {
        assert_eq!(to_snake_case("UserAccount"), "user_account");
        assert_eq!(to_snake_case("OrderItems"), "order_items");
        assert_eq!(to_snake_case("product"), "product");
        assert_eq!(to_snake_case("XMLParser"), "xmlparser");
    }
}