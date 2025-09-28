//! TTL (Time-To-Live) management for `LinkML` validation cache
//!
//! This module provides sophisticated TTL management with:
//! - Dynamic TTL based on access patterns
//! - Hierarchical TTL inheritance
//! - TTL prediction using access frequency
//! - Efficient expiration tracking

use dashmap::DashMap;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::sync::Arc;
use std::time::{Duration, Instant};
use rootreal_core_foundation_timestamp_core::{TimestampError, TimestampService};

/// TTL configuration for different cache levels
#[derive(Debug, Clone)]
pub struct TtlConfig {
    /// Base TTL for L1 cache
    pub l1_base_ttl: Duration,
    /// Base TTL for L2 cache
    pub l2_base_ttl: Duration,
    /// Base TTL for L3 cache
    pub l3_base_ttl: Duration,
    /// Minimum TTL allowed
    pub min_ttl: Duration,
    /// Maximum TTL allowed
    pub max_ttl: Duration,
    /// TTL extension on hit
    pub ttl_extension_factor: f64,
    /// Enable adaptive TTL
    pub adaptive_ttl: bool,
    /// Access count threshold for promotion
    pub promotion_threshold: u32,
}

impl Default for TtlConfig {
    fn default() -> Self {
        Self {
            l1_base_ttl: Duration::from_secs(300),    // 5 minutes
            l2_base_ttl: Duration::from_secs(3_600),  // 1 hour
            l3_base_ttl: Duration::from_secs(86_400), // 24 hours
            min_ttl: Duration::from_secs(60),         // 1 minute
            max_ttl: Duration::from_secs(604_800),    // 7 days
            ttl_extension_factor: 1.5,
            adaptive_ttl: true,
            promotion_threshold: 5,
        }
    }
}

impl TtlConfig {
    /// Create TTL config from `LinkML` service configuration
    #[must_use]
    pub fn from_service_config(config: &linkml_core::configuration_v2::PerformanceConfig) -> Self {
        Self {
            l1_base_ttl: Duration::from_secs(config.cache_ttl_levels.l1_seconds),
            l2_base_ttl: Duration::from_secs(config.cache_ttl_levels.l2_seconds),
            l3_base_ttl: Duration::from_secs(config.cache_ttl_levels.l3_seconds),
            min_ttl: Duration::from_secs(config.cache_ttl_levels.min_ttl_seconds),
            max_ttl: Duration::from_secs(config.cache_ttl_levels.max_ttl_seconds),
            ttl_extension_factor: 1.5, // Could be added to config
            adaptive_ttl: true,        // Could be added to config
            promotion_threshold: 5,    // Could be added to config
        }
    }
}

/// Access pattern tracking for adaptive TTL
#[derive(Debug, Clone)]
struct AccessPattern {
    /// Number of accesses
    access_count: u32,
    /// Last access time
    last_access: std::time::Instant,
    /// Average time between accesses
    avg_access_interval: Duration,
    /// Access history (limited size)
    access_history: SmallVec<[std::time::Instant; 8]>,
}

impl Default for AccessPattern {
    fn default() -> Self {
        Self {
            access_count: 0,
            last_access: std::time::Instant::now(),
            avg_access_interval: Duration::from_secs(0),
            access_history: SmallVec::new(),
        }
    }
}

impl AccessPattern {
    /// Record a new access
    fn record_access(&mut self) {
        let now = std::time::Instant::now();
        self.access_count += 1;

        // Update average interval
        if self.access_count > 1 {
            let interval = now.duration_since(self.last_access);
            let weight = 1.0 / f64::from(self.access_count);
            let new_avg = self.avg_access_interval.as_secs_f64() * (1.0 - weight)
                + interval.as_secs_f64() * weight;
            self.avg_access_interval = Duration::from_secs_f64(new_avg);
        }

        self.last_access = now;

        // Keep limited history
        if self.access_history.len() >= 8 {
            self.access_history.remove(0);
        }
        self.access_history.push(now);
    }

    /// Calculate predicted next access time
    fn predict_next_access(&self) -> Option<Instant> {
        if self.access_count < 2 {
            return None;
        }

        Some(self.last_access + self.avg_access_interval)
    }

    /// Calculate access frequency (accesses per hour)
    fn access_frequency(&self) -> f64 {
        if self.access_history.is_empty() {
            return 0.0;
        }

        let duration = self.last_access.duration_since(self.access_history[0]);
        if duration.as_secs() == 0 {
            return 0.0;
        }

        f64::from(self.access_count) / (duration.as_secs_f64() / 3600.0)
    }
}

/// TTL entry with metadata
#[derive(Debug, Clone)]
pub struct TtlEntry {
    /// Expiration time
    pub expires_at: std::time::Instant,
    /// Current TTL duration
    pub ttl_duration: Duration,
    /// Cache level (1, 2, or 3)
    pub cache_level: u8,
    /// Access pattern for this entry
    access_pattern: AccessPattern,
}

impl TtlEntry {
    /// Create a new TTL entry
    #[must_use]
    pub fn new(ttl: Duration, cache_level: u8) -> Self {
        Self {
            expires_at: std::time::Instant::now() + ttl,
            ttl_duration: ttl,
            cache_level,
            access_pattern: AccessPattern::default(),
        }
    }

    /// Check if expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() > self.expires_at
    }

    /// Time until expiration
    #[must_use]
    pub fn time_until_expiry(&self) -> Option<Duration> {
        self.expires_at
            .checked_duration_since(std::time::Instant::now())
    }

    /// Record access and potentially extend TTL
    pub fn record_access(&mut self, config: &TtlConfig) {
        self.access_pattern.record_access();

        if config.adaptive_ttl {
            // Extend TTL based on access frequency
            let frequency = self.access_pattern.access_frequency();
            let extension_factor = 1.0 + (frequency / 10.0).min(config.ttl_extension_factor - 1.0);

            let new_ttl =
                Duration::from_secs_f64(self.ttl_duration.as_secs_f64() * extension_factor);

            // Clamp to min/max
            let new_ttl = new_ttl.max(config.min_ttl).min(config.max_ttl);

            self.ttl_duration = new_ttl;
            self.expires_at = std::time::Instant::now() + new_ttl;
        }
    }
}

/// Hierarchical TTL rules
#[derive(Debug, Clone)]
pub struct TtlRule {
    /// Pattern to match cache keys
    pub pattern: String,
    /// TTL override for this pattern
    pub ttl_override: Option<Duration>,
    /// TTL multiplier for this pattern
    pub ttl_multiplier: Option<f64>,
    /// Priority (higher wins)
    pub priority: i32,
}

/// TTL manager for sophisticated cache expiration
pub struct TtlManager {
    /// Configuration
    config: Arc<RwLock<TtlConfig>>,
    /// TTL entries by key
    entries: DashMap<String, TtlEntry>,
    /// Hierarchical TTL rules
    rules: Arc<RwLock<Vec<TtlRule>>>,
    /// Global access patterns
    global_patterns: Arc<RwLock<AccessPattern>>,
    /// Timestamp service
    _timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
}

impl TtlManager {
    /// Create a new TTL manager
    #[must_use]
    pub fn new(
        config: TtlConfig,
        _timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            entries: DashMap::new(),
            rules: Arc::new(RwLock::new(Vec::new())),
            global_patterns: Arc::new(RwLock::new(AccessPattern::default())),
            _timestamp,
        }
    }

    /// Set TTL for a key
    #[must_use]
    pub fn set_ttl(&self, key: String, cache_level: u8) -> Duration {
        let config = self.config.read();

        // Get base TTL for cache level
        let base_ttl = match cache_level {
            1 => config.l1_base_ttl,
            3 => config.l3_base_ttl,
            _ => config.l2_base_ttl, // Use L2 as default for level 2 and any other level
        };

        // Apply rules
        let ttl = self.apply_rules(&key, base_ttl);

        // Create entry
        let entry = TtlEntry::new(ttl, cache_level);
        self.entries.insert(key, entry);

        ttl
    }

    /// Get TTL for a key (and record access)
    #[must_use]
    pub fn get_ttl(&self, key: &str) -> Option<Duration> {
        if let Some(mut entry) = self.entries.get_mut(key) {
            let config = self.config.read();
            entry.record_access(&config);

            // Record global access
            self.global_patterns.write().record_access();

            Some(entry.ttl_duration)
        } else {
            None
        }
    }

    /// Check if a key is expired
    #[must_use]
    pub fn is_expired(&self, key: &str) -> bool {
        self.entries.get(key).is_none_or(|entry| entry.is_expired())
    }

    /// Remove expired entries
    #[must_use]
    pub fn remove_expired(&self) -> Vec<String> {
        let mut expired_keys = Vec::new();

        self.entries.retain(|key, entry| {
            if entry.is_expired() {
                expired_keys.push(key.clone());
                false
            } else {
                true
            }
        });

        expired_keys
    }

    /// Get entries expiring soon (within duration)
    #[must_use]
    pub fn get_expiring_soon(&self, within: Duration) -> Vec<(String, Duration)> {
        let threshold = std::time::Instant::now() + within;

        self.entries
            .iter()
            .filter_map(|entry| {
                let (key, ttl_entry) = entry.pair();
                if ttl_entry.expires_at <= threshold {
                    ttl_entry
                        .time_until_expiry()
                        .map(|remaining| (key.clone(), remaining))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Add a TTL rule
    pub fn add_rule(&self, rule: TtlRule) {
        let mut rules = self.rules.write();
        rules.push(rule);
        rules.sort_by_key(|r| -r.priority);
    }

    /// Apply rules to determine TTL
    fn apply_rules(&self, key: &str, base_ttl: Duration) -> Duration {
        let rules = self.rules.read();

        for rule in rules.iter() {
            if key.contains(&rule.pattern) {
                if let Some(override_ttl) = rule.ttl_override {
                    return override_ttl;
                }

                if let Some(multiplier) = rule.ttl_multiplier {
                    return Duration::from_secs_f64(base_ttl.as_secs_f64() * multiplier);
                }
            }
        }

        base_ttl
    }

    /// Get cache statistics
    #[must_use]
    pub fn get_stats(&self) -> TtlStats {
        let total_entries = self.entries.len();
        let mut expired_count = 0;
        let mut by_level = [0usize; 4];
        let mut avg_ttl = Duration::from_secs(0);

        for entry in &self.entries {
            if entry.is_expired() {
                expired_count += 1;
            }
            by_level[entry.cache_level as usize] += 1;
            avg_ttl += entry.ttl_duration;
        }

        if total_entries > 0 {
            avg_ttl /= u32::try_from(total_entries).unwrap_or(u32::MAX);
        }

        TtlStats {
            total_entries,
            expired_count,
            entries_by_level: by_level,
            average_ttl: avg_ttl,
            global_access_frequency: self.global_patterns.read().access_frequency(),
        }
    }

    /// Predict optimal TTL based on access patterns
    #[must_use]
    pub fn predict_optimal_ttl(&self, key: &str) -> Option<Duration> {
        if let Some(entry) = self.entries.get(key) {
            let pattern = &entry.access_pattern;

            // If we have enough data, predict based on access pattern
            if pattern.access_count >= 3 {
                // Predict next access
                if let Some(next_access) = pattern.predict_next_access() {
                    let now = std::time::Instant::now();
                    if next_access > now {
                        // Set TTL to predicted time + buffer
                        let predicted_ttl = next_access.duration_since(now);
                        let buffer = Duration::from_secs_f64(predicted_ttl.as_secs_f64() * 0.2);
                        return Some(predicted_ttl + buffer);
                    }
                }
            }
        }

        None
    }
}

/// TTL statistics
#[derive(Debug, Clone)]
pub struct TtlStats {
    /// Total number of entries
    pub total_entries: usize,
    /// Number of expired entries
    pub expired_count: usize,
    /// Entries by cache level
    pub entries_by_level: [usize; 4],
    /// Average TTL duration
    pub average_ttl: Duration,
    /// Global access frequency
    pub global_access_frequency: f64,
}

/// Background TTL maintenance worker
pub struct TtlMaintenanceWorker {
    manager: Arc<TtlManager>,
    interval: Duration,
}

impl TtlMaintenanceWorker {
    /// Create a new maintenance worker
    #[must_use]
    pub fn new(manager: Arc<TtlManager>, interval: Duration) -> Self {
        Self { manager, interval }
    }

    /// Run the maintenance loop
    pub async fn run(&self) {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            interval.tick().await;

            // Remove expired entries
            let expired = self.manager.remove_expired();

            if !expired.is_empty() {
                tracing::debug!("Removed {} expired cache entries", expired.len());
            }

            // Log statistics periodically
            let stats = self.manager.get_stats();
            tracing::debug!(
                "TTL stats: {} entries, {} expired, avg TTL: {:?}",
                stats.total_entries,
                stats.expired_count,
                stats.average_ttl
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ttl_entry() {
        let mut entry = TtlEntry::new(Duration::from_secs(300), 1);
        assert!(!entry.is_expired());

        let config = TtlConfig::default();
        entry.record_access(&config);

        assert!(entry.time_until_expiry().is_some());
    }

    #[test]
    fn test_access_pattern() {
        let mut pattern = AccessPattern::default();

        for _ in 0..5 {
            pattern.record_access();
            std::thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(pattern.access_count, 5);
        assert!(pattern.avg_access_interval > Duration::from_secs(0));
    }

    #[test]
    fn test_ttl_rules() {
        // Create a mock timestamp service
        struct MockTimestampService;

        #[async_trait::async_trait]
        impl TimestampService for MockTimestampService {
            type Error = TimestampError;

            async fn now_utc(&self) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
                Ok(chrono::Utc::now())
            }

            async fn now_local(&self) -> Result<chrono::DateTime<chrono::Local>, Self::Error> {
                Ok(chrono::Local::now())
            }

            async fn system_time(&self) -> Result<std::time::SystemTime, Self::Error> {
                Ok(std::time::SystemTime::now())
            }

            async fn parse_iso8601(
                &self,
                timestamp: &str,
            ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
                timestamp
                    .parse()
                    .map_err(|e| TimestampError::parse_error(format!("Parse error: {e}")))
            }

            async fn format_iso8601(
                &self,
                timestamp: &chrono::DateTime<chrono::Utc>,
            ) -> Result<String, Self::Error> {
                Ok(timestamp.to_rfc3339())
            }

            async fn duration_since(
                &self,
                earlier: &chrono::DateTime<chrono::Utc>,
            ) -> Result<chrono::TimeDelta, Self::Error> {
                Ok(chrono::Utc::now() - *earlier)
            }

            async fn unix_timestamp_to_datetime(
                &self,
                seconds: i64,
            ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
                chrono::DateTime::from_timestamp(seconds, 0).ok_or_else(|| {
                    TimestampError::parse_error("Invalid Unix timestamp".to_string())
                })
            }

            async fn add_duration(
                &self,
                timestamp: &chrono::DateTime<chrono::Utc>,
                duration: chrono::TimeDelta,
            ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
                Ok(*timestamp + duration)
            }

            async fn subtract_duration(
                &self,
                timestamp: &chrono::DateTime<chrono::Utc>,
                duration: chrono::TimeDelta,
            ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
                Ok(*timestamp - duration)
            }

            async fn duration_between(
                &self,
                from: &chrono::DateTime<chrono::Utc>,
                to: &chrono::DateTime<chrono::Utc>,
            ) -> Result<chrono::TimeDelta, Self::Error> {
                Ok(*to - *from)
            }
        }

        let timestamp_service = Arc::new(MockTimestampService);
        let manager = TtlManager::new(TtlConfig::default(), timestamp_service);

        // Add a rule for schema patterns
        manager.add_rule(TtlRule {
            pattern: "schema:".to_string(),
            ttl_override: Some(Duration::from_secs(7200)),
            ttl_multiplier: None,
            priority: 10,
        });

        let ttl = manager.set_ttl("linkml:schema:test".to_string(), 1);
        assert_eq!(ttl, Duration::from_secs(7200));
    }
}
