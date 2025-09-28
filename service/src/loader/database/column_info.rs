//! Database column information structures

/// Column information from database schema
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    /// Column name
    pub name: String,

    /// Column data type
    pub data_type: String,

    /// Whether the column is nullable
    pub is_nullable: bool,

    /// Whether the column is a primary key
    pub is_primary_key: bool,

    /// Default value if any
    pub default_value: Option<String>,

    /// Maximum length for string types
    pub max_length: Option<i32>,

    /// Precision for numeric types
    pub numeric_precision: Option<i32>,

    /// Scale for numeric types
    pub numeric_scale: Option<i32>,
}
