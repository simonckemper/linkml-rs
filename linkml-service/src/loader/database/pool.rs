//! Database connection pool management

use sqlx::mysql::MySqlPool;
use sqlx::postgres::PgPool;

/// Database pool enum to handle different database types without Any
#[derive(Debug)]
pub enum DatabasePool {
    /// PostgreSQL connection pool
    PostgreSQL(PgPool),
    /// MySQL connection pool
    MySQL(MySqlPool),
}