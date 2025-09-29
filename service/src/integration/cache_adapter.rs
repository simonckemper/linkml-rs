//! Cache adapter for `RootReal` `CacheService` integration

use async_trait::async_trait;
use cache_core::CacheKey;
use std::sync::Arc;

/// Adapter to convert `RootReal`'s `CacheService` to our internal trait
pub struct CacheServiceAdapter {
    cache: Arc<dyn cache_core::CacheService<Error = cache_core::CacheError>>,
}

impl CacheServiceAdapter {
    /// Create a new cache service adapter
    pub fn new(cache: Arc<dyn cache_core::CacheService<Error = cache_core::CacheError>>) -> Self {
        Self { cache }
    }
}

#[async_trait]
impl crate::validator::cache::CacheService for CacheServiceAdapter {
    async fn get(
        &self,
        key: &str,
    ) -> std::result::Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        use cache_core::CacheKey;

        let cache_key = CacheKey::new(format!("linkml:{key}"))?;

        match self.cache.get(&cache_key).await {
            Ok(Some(value)) => {
                // Extract bytes from CacheValue
                match value.to_bytes() {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
                }
            }
            Ok(None) => Ok(None),
            Err(e) => Err(Box::new(e) as Box<dyn std::error::Error>),
        }
    }

    async fn set(
        &self,
        key: &str,
        value: Vec<u8>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        use cache_core::{CacheKey, CacheTtl, CacheValue};

        let cache_key = CacheKey::new(format!("linkml:{key}"))?;
        let cache_value = CacheValue::from_bytes(value);
        let ttl = Some(CacheTtl::Seconds(3600)); // 1 hour default TTL

        self.cache
            .set(&cache_key, &cache_value, ttl)
            .await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }

    async fn delete(&self, key: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let cache_key = CacheKey::new(format!("linkml:{key}"))?;

        self.cache
            .delete(&cache_key)
            .await
            .map(|_| ()) // Convert bool to ()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
}
