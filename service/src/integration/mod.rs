//! RootReal service integration module

pub mod cache_adapter;
pub mod iceberg_integration;
pub mod typedb_integration;

pub use cache_adapter::CacheServiceAdapter;
