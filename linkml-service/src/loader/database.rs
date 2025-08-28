//! Database loader and dumper for LinkML
//!
//! This module provides functionality to load data from SQL databases
//! and dump LinkML instances back to databases.

use super::traits::{
    DataDumper, DataInstance, DataLoader, DumperError, DumperResult, LoaderError, LoaderResult,
};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::{Map, Value};
use sqlx::mysql::{MySqlPool, MySqlPoolOptions, MySqlRow};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::{Column, Database, Executor, Pool, Row, TypeInfo, query::Query};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

/// Database pool enum to handle different database types without Any
#[derive(Debug)]
enum DatabasePool {
    PostgreSQL(PgPool),
    MySQL(MySqlPool),
}

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
    pub max_connections: u32,
}

/// Foreign key relationship definition
#[derive(Debug, Clone)]
pub struct ForeignKeyRelation {
    /// Column in the source table
    pub column: String,

    /// Referenced table
    pub referenced_table: String,

    /// Referenced column
    pub referenced_column: String,

    /// Slot name in LinkML schema
    pub slot_name: String,
}

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
            max_connections: 5,
        }
    }
}

/// Database loader for LinkML data
pub struct DatabaseLoader {
    options: DatabaseOptions,
    pool: Option<DatabasePool>,
}

impl DatabaseLoader {
    /// Create a new database loader
    pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None,
        }
    }

    /// Connect to the database
    async fn connect(&mut self) -> LoaderResult<()> {
        if self.pool.is_none() {
            let pool = AnyPoolOptions::new()
                .max_connections(self.options.max_connections)
                .connect(&self.options.connection_string)
                .await
                .map_err(|e| {
                    LoaderError::Io(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        format!("Failed to connect to database: {}", e),
                    ))
                })?;

            self.pool = Some(pool);
        }
        Ok(())
    }

    /// Get the connection pool
    fn get_pool(&self) -> LoaderResult<&AnyPool> {
        self.pool.as_ref().ok_or_else(|| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::NotConnected,
                "Database not connected",
            ))
        })
    }

    /// Get table names from the database
    async fn get_table_names(&self) -> LoaderResult<Vec<String>> {
        let pool = self.get_pool()?;

        // This query works for PostgreSQL, MySQL, and SQLite
        let query = match self.get_database_type()? {
            DatabaseType::PostgreSQL => {
                if let Some(schema) = &self.options.schema_name {
                    format!(
                        "SELECT table_name FROM information_schema.tables
                         WHERE table_schema = '{}' AND table_type = 'BASE TABLE'",
                        schema
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
                         WHERE table_schema = '{}' AND table_type = 'BASE TABLE'",
                        schema
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
            _ => {
                return Err(LoaderError::UnsupportedFormat(
                    "Unsupported database type".to_string(),
                ));
            }
        };

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to query tables: {}", e),
            ))
        })?;

        let mut tables = Vec::new();
        for row in rows {
            if let Ok(table_name) = row.try_get::<String, _>(0) {
                // Apply filtering
                if self.options.exclude_tables.contains(&table_name) {
                    continue;
                }

                if let Some(include) = &self.options.include_tables {
                    if !include.contains(&table_name) {
                        continue;
                    }
                }

                tables.push(table_name);
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
            Err(LoaderError::UnsupportedFormat(
                "SQLite support disabled to resolve dependency conflicts. Use PostgreSQL or MySQL instead.".to_string()
            ))
        } else {
            Err(LoaderError::UnsupportedFormat(
                "Unsupported database type in connection string".to_string(),
            ))
        }
    }

    /// Get column information for a table
    async fn get_columns(&self, table_name: &str) -> LoaderResult<Vec<ColumnInfo>> {
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
                        format!("AND table_schema = '{}'", schema)
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
                        format!("AND table_schema = '{}'", schema)
                    } else {
                        "AND table_schema = DATABASE()".to_string()
                    }
                )
            }
            DatabaseType::SQLite => {
                format!("PRAGMA table_info({})", table_name)
            }
            _ => {
                return Err(LoaderError::UnsupportedFormat(
                    "Unsupported database type".to_string(),
                ));
            }
        };

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to query columns: {}", e),
            ))
        })?;

        let mut columns = Vec::new();

        match self.get_database_type()? {
            DatabaseType::SQLite => {
                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get::<String, _>(1)?,
                        data_type: row.try_get::<String, _>(2)?,
                        is_nullable: row.try_get::<i32, _>(3)? == 0,
                        is_primary_key: row.try_get::<i32, _>(5)? == 1,
                    });
                }
            }
            _ => {
                for row in rows {
                    columns.push(ColumnInfo {
                        name: row.try_get::<String, _>(0)?,
                        data_type: row.try_get::<String, _>(1)?,
                        is_nullable: row.try_get::<String, _>(2)? == "YES",
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
                        format!("AND tc.table_schema = '{}'", schema)
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
                        format!("AND table_schema = '{}'", schema)
                    } else {
                        "AND table_schema = DATABASE()".to_string()
                    }
                )
            }
            _ => return Ok(()), // SQLite handled differently
        };

        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to query primary keys: {}", e),
            ))
        })?;

        let mut pk_columns = HashSet::new();
        for row in rows {
            if let Ok(col_name) = row.try_get::<String, _>(0) {
                pk_columns.insert(col_name);
            }
        }

        for column in columns {
            if pk_columns.contains(&column.name) {
                column.is_primary_key = true;
            }
        }

        Ok(())
    }

    /// Convert database type to LinkML range
    fn db_type_to_linkml_range(&self, db_type: &str) -> String {
        match db_type.to_lowercase().as_str() {
            "integer" | "int" | "int4" | "int8" | "bigint" | "smallint" => "integer",
            "real" | "double" | "float" | "float4" | "float8" | "decimal" | "numeric" => "float",
            "boolean" | "bool" => "boolean",
            "text" | "varchar" | "char" | "character varying" => "string",
            "date" => "date",
            "timestamp"
            | "datetime"
            | "timestamp with time zone"
            | "timestamp without time zone" => "datetime",
            "time" | "time with time zone" | "time without time zone" => "time",
            _ => "string", // Default to string for unknown types
        }
    }

    /// Load data from a table
    async fn load_table_data(
        &self,
        table_name: &str,
        columns: &[ColumnInfo],
        schema: &SchemaDefinition,
    ) -> LoaderResult<Vec<DataInstance>> {
        let pool = self.get_pool()?;
        let mut instances = Vec::new();

        // Get the class name for this table
        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Build query
        let query = format!("SELECT * FROM {}", table_name);
        let mut rows = sqlx::query(&query).fetch(pool);

        let mut batch = Vec::new();

        while let Some(row) = rows.try_next().await.map_err(|e| {
            LoaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to fetch row: {}", e),
            ))
        })? {
            let instance = self.row_to_instance(&row, table_name, columns)?;
            batch.push(instance);

            if batch.len() >= self.options.batch_size {
                instances.extend(batch.drain(..));
            }
        }

        instances.extend(batch);

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

    /// Convert a database row to a DataInstance
    fn row_to_instance(
        &self,
        row: &AnyRow,
        table_name: &str,
        columns: &[ColumnInfo],
    ) -> LoaderResult<DataInstance> {
        let mut data = Map::new();

        // Get column mapping for this table
        let column_mapping = self.options.column_mapping.get(table_name);

        for (idx, column) in columns.iter().enumerate() {
            let slot_name = if let Some(mapping) = column_mapping {
                mapping
                    .get(&column.name)
                    .cloned()
                    .unwrap_or_else(|| to_snake_case(&column.name))
            } else {
                to_snake_case(&column.name)
            };

            // Get value based on type
            let value = self.get_column_value(row, idx, &column.data_type)?;

            if !value.is_null() {
                data.insert(slot_name, value);
            }
        }

        let class_name = self
            .options
            .table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        Ok(DataInstance { class_name, data })
    }

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
}

#[async_trait]
impl DataLoader for DatabaseLoader {
    async fn load(&mut self, schema: &SchemaDefinition) -> LoaderResult<Vec<DataInstance>> {
        // Connect to database
        self.connect().await?;

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

        Ok(all_instances)
    }
}

/// Database dumper for LinkML data
pub struct DatabaseDumper {
    options: DatabaseOptions,
    pool: Option<AnyPool>,
}

impl DatabaseDumper {
    /// Create a new database dumper
    pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None,
        }
    }

    /// Connect to the database
    async fn connect(&mut self) -> DumperResult<()> {
        if self.pool.is_none() {
            let pool = AnyPoolOptions::new()
                .max_connections(self.options.max_connections)
                .connect(&self.options.connection_string)
                .await
                .map_err(|e| {
                    DumperError::Io(std::io::Error::new(
                        std::io::ErrorKind::ConnectionRefused,
                        format!("Failed to connect to database: {}", e),
                    ))
                })?;

            self.pool = Some(pool);
        }
        Ok(())
    }

    /// Get the connection pool
    fn get_pool(&self) -> DumperResult<&AnyPool> {
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

        let pool = self.get_pool()?;

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name)
            .map(|(tn, _)| tn.clone())
            .unwrap_or_else(|| to_snake_case(class_name));

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
                    format!("{} {} {}", column_name, column_type, nullable)
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

        sqlx::query(&create_sql).execute(pool).await.map_err(|e| {
            DumperError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create table: {}", e),
            ))
        })?;

        Ok(())
    }

    /// Convert LinkML range to database type
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
                _ => "TEXT",
            },
            DatabaseType::MySQL => match range {
                "string" => "TEXT",
                "integer" => "INT",
                "float" => "DOUBLE",
                "boolean" => "BOOLEAN",
                "date" => "DATE",
                "datetime" => "DATETIME",
                "time" => "TIME",
                _ => "TEXT",
            },
            DatabaseType::SQLite => match range {
                "string" => "TEXT",
                "integer" => "INTEGER",
                "float" => "REAL",
                "boolean" => "INTEGER",
                "date" => "TEXT",
                "datetime" => "TEXT",
                "time" => "TEXT",
                _ => "TEXT",
            },
            _ => {
                return Err(DumperError::UnsupportedFormat(
                    "Unsupported database type".to_string(),
                ));
            }
        };

        Ok(db_type.to_string())
    }

    /// Insert instances for a class
    async fn insert_instances(
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
            .find(|(_, cn)| cn == &class_name)
            .map(|(tn, _)| tn.clone())
            .unwrap_or_else(|| to_snake_case(class_name));

        // Get class definition
        let class_def = schema.classes.get(class_name).ok_or_else(|| {
            DumperError::SchemaValidation(format!("Class {} not found in schema", class_name))
        })?;

        // Get column mapping
        let column_mapping = self.options.column_mapping.get(&table_name);

        // Process in batches
        for batch in instances.chunks(self.options.batch_size) {
            let mut tx = pool.begin().await.map_err(|e| {
                DumperError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to begin transaction: {}", e),
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
                        _ => query.bind(value.to_string()),
                    };
                }

                query.execute(&mut *tx).await.map_err(|e| {
                    DumperError::Io(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to insert row: {}", e),
                    ))
                })?;
            }

            tx.commit().await.map_err(|e| {
                DumperError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to commit transaction: {}", e),
                ))
            })?;
        }

        Ok(())
    }
}

#[async_trait]
impl DataDumper for DatabaseDumper {
    async fn dump(
        &mut self,
        instances: &[DataInstance],
        schema: &SchemaDefinition,
    ) -> DumperResult<Vec<u8>> {
        // Connect to database
        self.connect().await?;

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

/// Column information
#[derive(Debug, Clone)]
struct ColumnInfo {
    name: String,
    data_type: String,
    is_nullable: bool,
    is_primary_key: bool,
}

/// Database type enumeration
#[derive(Debug, Clone, Copy)]
enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
}

/// Convert string to pascal case
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
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
                .map_err(|e| anyhow::anyhow!("Error: {}", e))? should produce at least one char"),
        );
        prev_upper = ch.is_uppercase();
    }

    result
}

// Helper trait imports for async operations
use futures::TryStreamExt;

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
