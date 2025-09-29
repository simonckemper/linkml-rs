//! Timestamp utilities using `RootReal`'s `TimestampService`
//!
//! This module provides convenient timestamp functions that use the `TimestampService`
//! instead of direct chrono usage, ensuring architectural compliance and testability.

use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, TimeZone, Utc};
use linkml_core::error::{LinkMLError, Result};
use std::sync::Arc;
use timestamp_core::TimestampService;

/// Timestamp utilities that wrap `TimestampService` functionality
pub struct TimestampUtils {
    service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
}

impl TimestampUtils {
    /// Create new timestamp utilities with a `TimestampService`
    pub fn new(service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>) -> Self {
        Self { service }
    }

    /// Get current UTC timestamp
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails to provide current time.
    pub async fn now(&self) -> Result<DateTime<Utc>> {
        self.service
            .now_utc()
            .await
            .map_err(|e| LinkMLError::service(format!("Timestamp error: {e}")))
    }

    /// Get current timestamp as RFC3339 string
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails or formatting fails.
    pub async fn now_rfc3339(&self) -> Result<String> {
        let now = self.now().await?;
        Ok(now.to_rfc3339())
    }

    /// Get current timestamp as ISO8601 string
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails or formatting fails.
    pub async fn now_iso8601(&self) -> Result<String> {
        let now = self.now().await?;
        Ok(now.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string())
    }

    /// Get current date (without time)
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails.
    pub async fn today(&self) -> Result<NaiveDate> {
        let now = self.now().await?;
        Ok(now.naive_utc().date())
    }

    /// Get current date as string (YYYY-MM-DD)
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails or date formatting fails.
    pub async fn today_string(&self) -> Result<String> {
        let today = self.today().await?;
        Ok(today.format("%Y-%m-%d").to_string())
    }

    /// Parse a date string.
    ///
    /// # Errors
    ///
    /// Returns an error when the input string cannot be parsed with any of the
    /// supported date formats.
    pub fn parse_date(date_str: &str) -> Result<NaiveDate> {
        NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%Y/%m/%d"))
            .or_else(|_| NaiveDate::parse_from_str(date_str, "%m/%d/%Y"))
            .map_err(|e| LinkMLError::other(format!("Failed to parse date '{date_str}': {e}")))
    }

    /// Parse a datetime string.
    ///
    /// # Errors
    ///
    /// Returns an error when the input string does not match any supported
    /// datetime representation.
    pub fn parse_datetime(dt_str: &str) -> Result<DateTime<Utc>> {
        // Try RFC3339 first
        DateTime::parse_from_rfc3339(dt_str)
            .map(|dt| dt.with_timezone(&Utc))
            .or_else(|_| {
                // Try ISO8601-like format
                NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%dT%H:%M:%S")
                    .or_else(|_| NaiveDateTime::parse_from_str(dt_str, "%Y-%m-%d %H:%M:%S"))
                    .map(|ndt| Utc.from_utc_datetime(&ndt))
            })
            .map_err(|e| LinkMLError::other(format!("Failed to parse datetime '{dt_str}': {e}")))
    }

    /// Format a timestamp for display
    #[must_use]
    pub fn format_timestamp(dt: &DateTime<Utc>, format: &str) -> String {
        dt.format(format).to_string()
    }

    /// Get Unix timestamp (seconds since epoch)
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails.
    pub async fn unix_timestamp(&self) -> Result<i64> {
        let now = self.now().await?;
        Ok(now.timestamp())
    }

    /// Get Unix timestamp in milliseconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the timestamp service fails.
    pub async fn unix_timestamp_millis(&self) -> Result<i64> {
        let now = self.now().await?;
        Ok(now.timestamp_millis())
    }

    /// Check if a date string represents today
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails or date string cannot be parsed.
    pub async fn is_today(&self, date_str: &str) -> Result<bool> {
        let today = self.today().await?;
        let date = Self::parse_date(date_str)?;
        Ok(date == today)
    }

    /// Get age in years from a birth date
    ///
    /// # Errors
    ///
    /// Returns error if timestamp service fails or birthdate string cannot be parsed.
    pub async fn age_from_birthdate(&self, birthdate_str: &str) -> Result<i32> {
        let birthdate = Self::parse_date(birthdate_str)?;
        let today = self.today().await?;

        let age = today.year() - birthdate.year();

        // Adjust if birthday hasn't occurred this year
        if today.month() < birthdate.month()
            || (today.month() == birthdate.month() && today.day() < birthdate.day())
        {
            Ok(age - 1)
        } else {
            Ok(age)
        }
    }

    /// Add days to the current date.
    ///
    /// # Errors
    ///
    /// Returns an error if the timestamp service fails while retrieving the
    /// current time.
    pub async fn add_days(&self, days: i64) -> Result<DateTime<Utc>> {
        let now = self.now().await?;
        Ok(now + chrono::Duration::days(days))
    }

    /// Add hours to the current time.
    ///
    /// # Errors
    ///
    /// Returns an error if the timestamp service fails while retrieving the
    /// current time.
    pub async fn add_hours(&self, hours: i64) -> Result<DateTime<Utc>> {
        let now = self.now().await?;
        Ok(now + chrono::Duration::hours(hours))
    }

    /// Calculate duration between two timestamps
    #[must_use]
    pub fn duration_between(start: &DateTime<Utc>, end: &DateTime<Utc>) -> chrono::Duration {
        *end - *start
    }
}

/// Synchronous timestamp utilities for non-async contexts
///
/// This is a convenience wrapper that blocks on async operations.
/// Use sparingly and prefer the async version when possible.
pub struct SyncTimestampUtils {
    utils: Arc<TimestampUtils>,
}

impl SyncTimestampUtils {
    /// Create new sync timestamp utilities.
    pub fn new(service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>) -> Self {
        Self {
            utils: Arc::new(TimestampUtils::new(service)),
        }
    }

    /// Get current UTC timestamp (blocking)
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying timestamp service fails.
    pub fn now(&self) -> Result<DateTime<Utc>> {
        tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(self.utils.now()))
    }

    /// Get current timestamp as RFC3339 string (blocking)
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying timestamp service fails.
    pub fn now_rfc3339(&self) -> Result<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.utils.now_rfc3339())
        })
    }

    /// Get current date (blocking)
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying timestamp service fails.
    pub fn today(&self) -> Result<NaiveDate> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.utils.today())
        })
    }

    /// Get current date as string (blocking)
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying timestamp service fails.
    pub fn today_string(&self) -> Result<String> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.utils.today_string())
        })
    }

    /// Parse a date string
    ///
    /// # Errors
    ///
    /// Returns an error when the input string cannot be parsed with any of the
    /// supported date formats.
    pub fn parse_date(&self, date_str: &str) -> Result<NaiveDate> {
        TimestampUtils::parse_date(date_str)
    }

    /// Parse a datetime string
    ///
    /// # Errors
    ///
    /// Returns an error when the input string does not match any supported
    /// datetime representation.
    pub fn parse_datetime(&self, dt_str: &str) -> Result<DateTime<Utc>> {
        TimestampUtils::parse_datetime(dt_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Datelike, Timelike};
    use rootreal_core_foundation_timestamp::factory::create_timestamp_service;

    #[tokio::test]
    async fn test_timestamp_utils() -> Result<()> {
        let ts_service = create_timestamp_service();
        let utils = TimestampUtils::new(ts_service);

        // Test getting current time
        let now = utils.now().await?;
        assert!(now.timestamp() > 0);

        // Test RFC3339 formatting
        let rfc3339 = utils.now_rfc3339().await?;
        assert!(rfc3339.contains("T"));
        assert!(rfc3339.contains("Z"));

        // Test date parsing
        let date = TimestampUtils::parse_date("2024-01-15")?;
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 15);

        // Test datetime parsing
        let dt = TimestampUtils::parse_datetime("2024-01-15T10:30:00Z")?;
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.hour(), 10);
        assert_eq!(dt.minute(), 30);

        Ok(())
    }

    #[tokio::test]
    async fn test_sync_utils() -> Result<()> {
        let ts_service = create_timestamp_service();
        let utils = SyncTimestampUtils::new(ts_service);

        // Test blocking operations
        let now = utils.now()?;
        assert!(now.timestamp() > 0);

        let today_str = utils.today_string()?;
        assert!(today_str.len() == 10); // YYYY-MM-DD format

        Ok(())
    }
}
