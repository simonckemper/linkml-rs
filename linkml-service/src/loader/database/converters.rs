//! Database value converters for PostgreSQL and MySQL

use super::column_info::ColumnInfo;
use crate::loader::traits::{DataInstance, LoaderResult};
use serde_json::Value;
use sqlx::Row;
use sqlx::mysql::MySqlRow;
use sqlx::postgres::PgRow;
use std::collections::HashMap;

/// PostgreSQL row to DataInstance converter
pub struct PostgresConverter;

impl PostgresConverter {
    /// Convert a PostgreSQL row to a `DataInstance`
    pub fn row_to_instance(
        row: &PgRow,
        table_name: &str,
        columns: &[ColumnInfo],
        table_mapping: &HashMap<String, String>,
        column_mapping: &HashMap<String, HashMap<String, String>>,
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();

        // Get class name for this table
        let class_name = table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Get column mapping for this table
        let col_mapping = column_mapping.get(table_name);

        for (i, column) in columns.iter().enumerate() {
            let column_name = &column.name;
            let mapped_name = col_mapping
                .and_then(|mapping| mapping.get(column_name))
                .cloned()
                .unwrap_or_else(|| to_snake_case(column_name));

            // Extract value with proper type handling
            let value = Self::get_column_value(row, i, &column.data_type)?;

            if !value.is_null() {
                data.insert(mapped_name, value);
            }
        }

        Ok(DataInstance {
            class_name,
            data,
            id: None,
            metadata: HashMap::new(),
        })
    }

    /// Get column value with proper type conversion
    pub fn get_column_value(row: &PgRow, idx: usize, db_type: &str) -> LoaderResult<Value> {
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
            "json" | "jsonb" => {
                // For PostgreSQL, try to get as JSON first, fallback to string parsing
                if let Ok(val) = row.try_get::<Option<sqlx::types::Json<serde_json::Value>>, _>(idx)
                {
                    Ok(val.map_or(Value::Null, |json| json.0))
                } else if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    match val {
                        Some(json_str) => {
                            match serde_json::from_str::<serde_json::Value>(&json_str) {
                                Ok(parsed) => Ok(parsed),
                                Err(_) => Ok(Value::String(json_str)), // Fallback to string if parse fails
                            }
                        }
                        None => Ok(Value::Null),
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "uuid" => {
                // Use String type for UUID instead of direct uuid::Uuid
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::String))
                } else {
                    Ok(Value::Null)
                }
            }
            "date" => {
                if let Ok(val) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
                    Ok(val.map_or(Value::Null, |d| Value::String(d.to_string())))
                } else {
                    Ok(Value::Null)
                }
            }
            "timestamp" | "timestamptz" => {
                if let Ok(val) = row.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(idx) {
                    Ok(val.map_or(Value::Null, |dt| Value::String(dt.to_rfc3339())))
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
}

/// MySQL row to DataInstance converter
pub struct MySqlConverter;

impl MySqlConverter {
    /// Convert a MySQL row to a `DataInstance`
    pub fn row_to_instance(
        row: &MySqlRow,
        table_name: &str,
        columns: &[ColumnInfo],
        table_mapping: &HashMap<String, String>,
        column_mapping: &HashMap<String, HashMap<String, String>>,
    ) -> LoaderResult<DataInstance> {
        let mut data = HashMap::new();

        // Get class name for this table
        let class_name = table_mapping
            .get(table_name)
            .cloned()
            .unwrap_or_else(|| to_pascal_case(table_name));

        // Get column mapping for this table
        let col_mapping = column_mapping.get(table_name);

        for (i, column) in columns.iter().enumerate() {
            let column_name = &column.name;
            let mapped_name = col_mapping
                .and_then(|mapping| mapping.get(column_name))
                .cloned()
                .unwrap_or_else(|| to_snake_case(column_name));

            // Extract value with proper type handling
            let value = Self::get_column_value(row, i, &column.data_type)?;

            if !value.is_null() {
                data.insert(mapped_name, value);
            }
        }

        Ok(DataInstance {
            class_name,
            data,
            id: None,
            metadata: HashMap::new(),
        })
    }

    /// Get column value with proper type conversion
    pub fn get_column_value(row: &MySqlRow, idx: usize, db_type: &str) -> LoaderResult<Value> {
        // Try to decode based on the database type
        match db_type.to_lowercase().as_str() {
            "tinyint" | "smallint" | "mediumint" | "int" | "integer" => {
                if let Ok(val) = row.try_get::<Option<i32>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "bigint" => {
                if let Ok(val) = row.try_get::<Option<i64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "float" => {
                if let Ok(val) = row.try_get::<Option<f32>, _>(idx) {
                    Ok(val.map_or(Value::Null, |v| Value::from(f64::from(v))))
                } else {
                    Ok(Value::Null)
                }
            }
            "double" | "decimal" | "numeric" => {
                if let Ok(val) = row.try_get::<Option<f64>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "bit" | "boolean" | "bool" => {
                if let Ok(val) = row.try_get::<Option<bool>, _>(idx) {
                    Ok(val.map_or(Value::Null, Value::from))
                } else {
                    Ok(Value::Null)
                }
            }
            "json" => {
                // For MySQL, JSON is stored as text and needs to be parsed
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
                    match val {
                        Some(json_str) => {
                            match serde_json::from_str::<serde_json::Value>(&json_str) {
                                Ok(parsed) => Ok(parsed),
                                Err(_) => Ok(Value::String(json_str)), // Fallback to string if parse fails
                            }
                        }
                        None => Ok(Value::Null),
                    }
                } else {
                    Ok(Value::Null)
                }
            }
            "date" => {
                if let Ok(val) = row.try_get::<Option<chrono::NaiveDate>, _>(idx) {
                    Ok(val.map_or(Value::Null, |d| Value::String(d.to_string())))
                } else {
                    Ok(Value::Null)
                }
            }
            "datetime" | "timestamp" => {
                if let Ok(val) = row.try_get::<Option<chrono::NaiveDateTime>, _>(idx) {
                    Ok(val.map_or(Value::Null, |dt| Value::String(dt.to_string())))
                } else {
                    Ok(Value::Null)
                }
            }
            "char" | "varchar" | "text" | "mediumtext" | "longtext" => {
                if let Ok(val) = row.try_get::<Option<String>, _>(idx) {
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
}

// Helper functions
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

fn to_snake_case(s: &str) -> String {
    s.to_lowercase().replace('-', "_")
}
