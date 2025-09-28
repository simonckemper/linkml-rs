//! Database loader and dumper options

use std::collections::{HashMap, HashSet};

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

    /// Slot name in `LinkML` schema
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
