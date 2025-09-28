//! Database dumper implementation

use super::options::DatabaseOptions;
use super::pool::DatabasePool;
use crate::loader::traits::{DataDumper, DataInstance, DumpOptions, DumperError, DumperResult};
use async_trait::async_trait;
use linkml_core::prelude::*;
use serde_json::Value;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Column, Row};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use tracing::info;

/// Database dumper for `LinkML` data
pub struct DatabaseDumper {
    options: DatabaseOptions,
    pool: Option<DatabasePool>,
}

impl DatabaseDumper {
    /// Create a new database dumper
    #[must_use]
    pub fn new(options: DatabaseOptions) -> Self {
        Self {
            options,
            pool: None,
        }
    }

    /// Connect to the database
    async fn connect(&self) -> DumperResult<DatabasePool> {
        if let Some(ref pool) = self.pool {
            Ok(pool.clone())
        } else {
            let pool = if self.options.connection_string.starts_with("postgres://") {
                let pg_pool = PgPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| {
                        DumperError::Configuration(format!("Failed to connect to PostgreSQL: {e}"))
                    })?;
                DatabasePool::PostgreSQL(pg_pool)
            } else if self.options.connection_string.starts_with("mysql://") {
                let mysql_pool = MySqlPoolOptions::new()
                    .max_connections(self.options.max_connections)
                    .connect(&self.options.connection_string)
                    .await
                    .map_err(|e| {
                        DumperError::Configuration(format!("Failed to connect to MySQL: {e}"))
                    })?;
                DatabasePool::MySQL(mysql_pool)
            } else {
                return Err(DumperError::Configuration(
                    "Unsupported database type. Only PostgreSQL and MySQL are supported."
                        .to_string(),
                ));
            };

            Ok(pool)
        }
    }

    /// Execute a query and return results as a vector of string maps
    async fn execute_query_as_maps(
        &self,
        query: &str,
        pool: &DatabasePool,
    ) -> DumperResult<Vec<HashMap<String, String>>> {
        match pool {
            DatabasePool::PostgreSQL(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "PostgreSQL query failed: {e}"
                    )))
                })?;

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
            DatabasePool::MySQL(pool) => {
                let rows = sqlx::query(query).fetch_all(pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!("MySQL query failed: {e}")))
                })?;

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
        }
    }

    /// Execute a DDL/DML statement
    async fn execute_statement(&self, statement: &str, pool: &DatabasePool) -> DumperResult<()> {
        match pool {
            DatabasePool::PostgreSQL(pool) => {
                sqlx::query(statement).execute(pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "PostgreSQL statement failed: {e}"
                    )))
                })?;
                Ok(())
            }
            DatabasePool::MySQL(pool) => {
                sqlx::query(statement).execute(pool).await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "MySQL statement failed: {e}"
                    )))
                })?;
                Ok(())
            }
        }
    }

    /// Get database type
    fn get_database_type(&self) -> DumperResult<DatabaseType> {
        if self.options.connection_string.starts_with("postgresql://")
            || self.options.connection_string.starts_with("postgres://")
        {
            Ok(DatabaseType::PostgreSQL)
        } else if self.options.connection_string.starts_with("mysql://") {
            Ok(DatabaseType::MySQL)
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
            .find(|(_, cn)| cn == &class_name)
            .map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

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
        };

        let pool = self.pool.as_ref().ok_or_else(|| {
            DumperError::Configuration("No database connection available".to_string())
        })?;
        let rows = self.execute_query_as_maps(&exists_query, pool).await?;
        let table_exists = if let Some(row) = rows.first() {
            match self.get_database_type()? {
                DatabaseType::PostgreSQL => {
                    row.get("exists").is_some_and(|v| v == "t" || v == "true")
                }
                DatabaseType::MySQL => {
                    row.get("count")
                        .and_then(|v| v.parse::<i32>().ok())
                        .unwrap_or(0)
                        > 0
                }
            }
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
        let pool = self.pool.as_ref().ok_or_else(|| {
            DumperError::Configuration("No database connection available".to_string())
        })?;
        self.execute_statement(&create_sql, pool).await?;

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
            schema
                .slots
                .get(slot_name)
                .is_some_and(|slot| slot.identifier.unwrap_or(false))
        });

        if !has_id_slot {
            let id_type = match self.get_database_type()? {
                DatabaseType::PostgreSQL => "SERIAL PRIMARY KEY",
                DatabaseType::MySQL => "INT AUTO_INCREMENT PRIMARY KEY",
            };
            columns.push(format!("id {id_type}"));
        }

        // Process each slot
        for slot_name in &class_def.slots {
            if let Some(slot_def) = schema.slots.get(slot_name) {
                let column_name = self
                    .options
                    .column_mapping
                    .get(table_name)
                    .and_then(|mapping| mapping.get(slot_name))
                    .cloned()
                    .unwrap_or_else(|| to_snake_case(slot_name));

                let db_type = self.linkml_range_to_db_type(
                    &slot_def
                        .range
                        .clone()
                        .unwrap_or_else(|| "string".to_string()),
                )?;

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
        let mut sql = format!(
            "CREATE TABLE {table_name} (
  "
        );
        sql.push_str(&columns.join(
            ",
  ",
        ));

        if !primary_keys.is_empty() && has_id_slot {
            sql.push_str(&format!(
                ",
  PRIMARY KEY ({})",
                primary_keys.join(", ")
            ));
        }

        sql.push_str(
            "
)",
        );

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
        if instances.is_empty() {
            return Ok(());
        }

        // Get table name
        let table_name = self
            .options
            .table_mapping
            .iter()
            .find(|(_, cn)| cn == &class_name)
            .map_or_else(|| to_snake_case(class_name), |(tn, _)| tn.clone());

        // Verify class exists in schema
        let _class_def = schema.classes.get(class_name).ok_or_else(|| {
            DumperError::Configuration(format!("Class {class_name} not found in schema"))
        })?;

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
            DatabaseType::PostgreSQL => (1..=columns.len())
                .map(|i| format!("${i}"))
                .collect::<Vec<_>>()
                .join(", "),
            DatabaseType::MySQL => vec!["?"; columns.len()].join(", "),
        };

        let insert_sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders
        );

        info!(
            "Inserting {} instances into table {} with SQL: {}",
            instances.len(),
            table_name,
            insert_sql
        );

        // Use transactions for batch insertion
        if self.options.use_transactions {
            self.insert_with_transaction(&insert_sql, &columns, instances)
                .await?;
        } else {
            self.insert_without_transaction(&insert_sql, &columns, instances)
                .await?;
        }

        info!(
            "Successfully inserted {} instances into table {}",
            instances.len(),
            table_name
        );
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
                    DumperError::Io(std::io::Error::other(format!(
                        "Failed to begin PostgreSQL transaction: {e}"
                    )))
                })?;

                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance
                            .data
                            .get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(&mut *tx).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert PostgreSQL row: {e}"
                        )))
                    })?;
                }

                tx.commit().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "Failed to commit PostgreSQL transaction: {e}"
                    )))
                })?;
            }
            Some(DatabasePool::MySQL(pool)) => {
                let mut tx = pool.begin().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "Failed to begin MySQL transaction: {e}"
                    )))
                })?;

                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance
                            .data
                            .get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(&mut *tx).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert MySQL row: {e}"
                        )))
                    })?;
                }

                tx.commit().await.map_err(|e| {
                    DumperError::Io(std::io::Error::other(format!(
                        "Failed to commit MySQL transaction: {e}"
                    )))
                })?;
            }
            None => {
                return Err(DumperError::Configuration(
                    "No database connection available".to_string(),
                ));
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
                        let value = instance
                            .data
                            .get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(pool).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert PostgreSQL row: {e}"
                        )))
                    })?;
                }
            }
            Some(DatabasePool::MySQL(pool)) => {
                for instance in instances {
                    let mut query = sqlx::query(insert_sql);

                    for column in columns {
                        let value = instance
                            .data
                            .get(column)
                            .map(|v| self.json_value_to_string(v))
                            .unwrap_or_default();
                        query = query.bind(value);
                    }

                    query.execute(pool).await.map_err(|e| {
                        DumperError::Io(std::io::Error::other(format!(
                            "Failed to insert MySQL row: {e}"
                        )))
                    })?;
                }
            }
            None => {
                return Err(DumperError::Configuration(
                    "No database connection available".to_string(),
                ));
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
            _ => value.to_string(),
        }
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
            "Database dumper does not support file dumping. Use dump_string.".to_string(),
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

/// Database type enumeration
#[derive(Debug, Clone, Copy)]
enum DatabaseType {
    PostgreSQL,
    MySQL,
}

// Helper function
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        result.push(
            ch.to_lowercase()
                .next()
                .expect("to_lowercase() should produce at least one char"),
        );
        prev_upper = ch.is_uppercase();
    }

    result
}
