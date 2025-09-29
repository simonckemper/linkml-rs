//! Mock services for testing LinkML service
//!
//! These mocks implement the required RootReal service traits
//! for testing purposes only.

use async_trait::async_trait;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::RwLock;

// Import core types and service traits
use cache_core::{CacheError, CacheKey, CacheService, CacheTtl, CacheValue};
use configuration_core::{ConfigurationError, ConfigurationService};
use error_handling_core::{
    ErrorCategory, ErrorContext, ErrorHandlingError, ErrorHandlingService, ErrorPattern,
    RecoveryStrategy, TransientErrorType,
};
use logger_core::{LogEntry, LogLevel, LoggerError, LoggerService};
use monitoring_core::{
    Alert, BottleneckReport, HealthReport, HealthStatus, HealthSummary, MonitoringConfig,
    MonitoringError, MonitoringService, MonitoringSession, MonitoringSessionStatus,
    PerformanceAnalysisSummary, PerformanceMetric, PerformanceReport, SessionConfiguration,
    SystemHealthReport, SystemPerformanceMetrics,
};
use task_management_core::{TaskId, TaskManagementError, TaskManagementService, TaskOptions};
use timestamp_core::{TimestampError, TimestampService};

// Import DBMS and Timeout services
use dbms_core::{
    ConnectionPool, DBMSError, DBMSResult, DBMSService, DatabaseConfig, DatabaseConnection,
    types::{
        DatabaseConfig as TypesConfig, DatabaseEvent, DatabaseInfo, DatabaseMetrics,
        DatabaseStatus, HealthStatus as DBHealthStatus, OptimizationReport, SchemaValidation,
        SchemaVersion,
    },
};
use timeout_core::{
    TimeoutConfig, TimeoutContext, TimeoutError, TimeoutHistory, TimeoutService, TimeoutStatistics,
    TimeoutValue,
};
use random_core::{
    BackendType, BetaParams, BinomialParams, ExponentialParams, GammaParams, NormalParams,
    PoissonParams, RandomConfig, RandomError, RandomResult, RandomService, SeedValue,
};

pub struct MockLoggerService {
    logs: Arc<RwLock<Vec<String>>>,
}

impl MockLoggerService {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn get_logs(&self) -> Vec<String> {
        self.logs.read().await.clone()
    }
}

#[async_trait]
impl LoggerService for MockLoggerService {
    type Error = LoggerError;

    async fn debug(&self, message: &str) -> Result<(), Self::Error> {
        self.logs.write().await.push(format!("[DEBUG] {}", message));
        Ok(())
    }

    async fn info(&self, message: &str) -> Result<(), Self::Error> {
        self.logs.write().await.push(format!("[INFO] {}", message));
        Ok(())
    }

    async fn warn(&self, message: &str) -> Result<(), Self::Error> {
        self.logs.write().await.push(format!("[WARN] {}", message));
        Ok(())
    }

    async fn error(&self, message: &str) -> Result<(), Self::Error> {
        self.logs.write().await.push(format!("[ERROR] {}", message));
        Ok(())
    }

    async fn log(&self, level: LogLevel, message: &str) -> Result<(), Self::Error> {
        let level_str = match level {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        };
        self.logs
            .write()
            .await
            .push(format!("[{}] {}", level_str, message));
        Ok(())
    }

    async fn log_entry(&self, entry: &LogEntry) -> Result<(), Self::Error> {
        self.log(entry.level, &entry.message).await
    }

    async fn set_level(&self, _level: LogLevel) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct MockTimestampService;

#[async_trait]
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
        use chrono::DateTime;
        Ok(DateTime::parse_from_rfc3339(timestamp)
            .map_err(|e| TimestampError::ParseError {
                message: e.to_string().into(),
            })?
            .with_timezone(&chrono::Utc))
    }

    async fn format_iso8601(
        &self,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> Result<String, Self::Error> {
        Ok(timestamp.to_rfc3339())
    }

    async fn duration_since(
        &self,
        timestamp: &chrono::DateTime<chrono::Utc>,
    ) -> Result<chrono::Duration, Self::Error> {
        Ok(chrono::Utc::now() - *timestamp)
    }

    async fn add_duration(
        &self,
        timestamp: &chrono::DateTime<chrono::Utc>,
        duration: chrono::Duration,
    ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
        Ok(*timestamp + duration)
    }

    async fn subtract_duration(
        &self,
        timestamp: &chrono::DateTime<chrono::Utc>,
        duration: chrono::Duration,
    ) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
        Ok(*timestamp - duration)
    }

    async fn duration_between(
        &self,
        from: &chrono::DateTime<chrono::Utc>,
        to: &chrono::DateTime<chrono::Utc>,
    ) -> Result<chrono::Duration, Self::Error> {
        Ok(*to - *from)
    }
}

pub struct MockTaskManagementService;

#[async_trait]
impl TaskManagementService for MockTaskManagementService {
    type Error = TaskManagementError;

    async fn spawn_task<F, T>(
        &self,
        future: F,
        _options: Option<TaskOptions>,
    ) -> Result<TaskId, Self::Error>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // For mock, just run the future and return a dummy task ID
        tokio::spawn(future);
        Ok(TaskId::new())
    }

    async fn cancel_task(&self, _task_id: &TaskId) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn wait_for_task<T>(&self, _task_id: &TaskId) -> Result<T, Self::Error>
    where
        T: Send + 'static,
    {
        Err(TaskManagementError::TaskNotFound {
            task_id: "test-task".to_string(),
        })
    }

    async fn is_task_running(&self, _task_id: &TaskId) -> Result<bool, Self::Error> {
        Ok(false)
    }

    async fn active_task_count(&self) -> usize {
        0
    }

    async fn spawn_blocking<F, T>(
        &self,
        func: F,
        _options: Option<TaskOptions>,
    ) -> Result<TaskId, Self::Error>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        tokio::task::spawn_blocking(func);
        Ok(TaskId::new())
    }
}

pub struct MockErrorHandlerService;

#[async_trait]
impl ErrorHandlingService for MockErrorHandlerService {
    type Error = ErrorHandlingError;

    async fn categorize_error<E: std::error::Error + Send + Sync + 'static>(
        &self,
        _error: &E,
        _context: Option<ErrorContext>,
    ) -> Result<ErrorCategory, Self::Error> {
        Ok(ErrorCategory::Transient {
            subcategory: TransientErrorType::NetworkTimeout,
            estimated_recovery_time_ms: 1000,
            retry_safe: true,
        })
    }

    async fn get_error_statistics(
        &self,
    ) -> Result<error_handling_core::traits::error_handling::ErrorStatistics, Self::Error> {
        Ok(
            error_handling_core::traits::error_handling::ErrorStatistics {
                total_count: 0,
                is_empty: true,
                capacity: Some(1000),
                is_sharded: false,
                recent_hour_count: 0,
                recent_day_count: 0,
            },
        )
    }

    async fn get_recent_errors(
        &self,
        _limit: u32,
    ) -> Result<Vec<error_handling_core::ErrorReport>, Self::Error> {
        Ok(Vec::new())
    }

    async fn get_service_errors(
        &self,
        _service_name: &str,
    ) -> Result<Vec<error_handling_core::ErrorReport>, Self::Error> {
        Ok(Vec::new())
    }

    async fn clear_all_errors(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    #[cfg(debug_assertions)]
    async fn get_raw_reports(&self) -> Result<Vec<error_handling_core::ErrorReport>, Self::Error> {
        Ok(Vec::new())
    }

    async fn determine_recovery_strategy(
        &self,
        _category: &ErrorCategory,
        _attempt_count: u32,
        _service_context: &str,
    ) -> Result<RecoveryStrategy, Self::Error> {
        Ok(RecoveryStrategy::ImmediateRetry {
            max_attempts: 3,
            backoff_ms: Some(100),
        })
    }

    async fn enrich_error_context<E: std::error::Error + Send + Sync + 'static>(
        &self,
        error: &E,
        service_context: &str,
        operation: &str,
        timestamp: Option<chrono::DateTime<chrono::Utc>>,
        correlation_id: Option<String>,
    ) -> Result<ErrorContext, Self::Error> {
        Ok(ErrorContext {
            timestamp: timestamp.unwrap_or_else(|| chrono::Utc::now()),
            service_name: service_context.to_string(),
            operation: operation.to_string(),
            correlation_id: correlation_id.or_else(|| Some("test-correlation-id".to_string())),
            user_context: None,
            system_metrics: HashMap::new(),
            error_chain: vec![error.to_string()],
            additional_data: HashMap::new(),
        })
    }

    async fn analyze_error_patterns(
        &self,
        _time_window_minutes: u32,
        _service_filter: Option<&str>,
    ) -> Result<Vec<ErrorPattern>, Self::Error> {
        Ok(Vec::new())
    }

    async fn is_retryable<E: std::error::Error + Send + Sync + 'static>(
        &self,
        _error: &E,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn extract_error_metadata<E: std::error::Error + Send + Sync + 'static>(
        &self,
        _error: &E,
    ) -> Result<Vec<(String, String)>, Self::Error> {
        Ok(Vec::new())
    }
}

pub struct MockConfigurationService {
    config: HashMap<String, String>,
}

impl MockConfigurationService {
    pub fn new() -> Self {
        let mut config = HashMap::new();
        config.insert("linkml.cache_enabled".to_string(), "true".to_string());
        config.insert("linkml.cache_size".to_string(), "1000".to_string());
        config.insert("linkml.validation_timeout".to_string(), "30".to_string());
        config.insert("linkml.max_validation_depth".to_string(), "10".to_string());
        config.insert("linkml.parallel_validation".to_string(), "true".to_string());

        Self { config }
    }

    pub fn set(&self, key: &str, value: &str) {
        // Since config is not mutable, this is a no-op for the mock
        // In a real implementation, you'd need Arc<RwLock<HashMap>> or similar
        let _ = (key, value);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.config.get(key).cloned()
    }
}

#[async_trait]
impl ConfigurationService for MockConfigurationService {
    type Error = ConfigurationError;

    async fn load_configuration<T>(&self) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de>
            + configuration_core::Validate
            + Clone
            + Send
            + Sync
            + 'static,
    {
        Err(ConfigurationError::LoadingError {
            message: "Mock load not implemented".to_string(),
        })
    }

    async fn load_configuration_from_source<T>(&self, _source: &str) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de>
            + configuration_core::Validate
            + Clone
            + Send
            + Sync
            + 'static,
    {
        Err(ConfigurationError::LoadingError {
            message: "Mock load not implemented".to_string(),
        })
    }

    async fn get_configuration<T>(&self, key: &str) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone + Send + Sync + 'static,
    {
        let value = self
            .config
            .get(key)
            .ok_or_else(|| ConfigurationError::ValidationError {
                message: format!("Key not found: {}", key),
            })?;
        serde_json::from_str(value).map_err(|_| ConfigurationError::ParsingError {
            message: "Mock parse error".to_string(),
        })
    }

    async fn reload_configuration(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn validate_configuration(&self, _source: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn set_configuration<T>(&self, _key: &str, _value: &T) -> Result<(), Self::Error>
    where
        T: serde::Serialize + configuration_core::Validate + Clone + Send + Sync + 'static,
    {
        Ok(())
    }

    async fn delete_configuration(&self, _key: &str) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn get_configuration_metadata(
        &self,
        _source: &str,
    ) -> Result<configuration_core::ConfigurationMetadata, Self::Error> {
        Err(ConfigurationError::SourceError {
            message: "Mock metadata not available".to_string(),
        })
    }
}

pub struct MockCacheService {
    cache: Arc<RwLock<HashMap<CacheKey, CacheValue>>>,
}

impl MockCacheService {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn stats(&self) -> (usize, usize, usize) {
        let cache = self.cache.read().await;
        let size = cache.len();
        // For the mock, we'll just return simple stats
        (size, 0, 0) // (size, hits, misses)
    }
}

#[async_trait]
impl CacheService for MockCacheService {
    type Error = CacheError;

    async fn get(&self, key: &CacheKey) -> Result<Option<CacheValue>, Self::Error> {
        Ok(self.cache.read().await.get(key).cloned())
    }

    async fn set(
        &self,
        key: &CacheKey,
        value: &CacheValue,
        _ttl: Option<CacheTtl>,
    ) -> Result<(), Self::Error> {
        self.cache.write().await.insert(key.clone(), value.clone());
        Ok(())
    }

    async fn delete(&self, key: &CacheKey) -> Result<bool, Self::Error> {
        Ok(self.cache.write().await.remove(key).is_some())
    }

    async fn exists(&self, key: &CacheKey) -> Result<bool, Self::Error> {
        Ok(self.cache.read().await.contains_key(key))
    }

    async fn get_many(
        &self,
        keys: &[CacheKey],
    ) -> Result<HashMap<CacheKey, CacheValue>, Self::Error> {
        let cache = self.cache.read().await;
        let mut result = HashMap::new();
        for key in keys {
            if let Some(value) = cache.get(key) {
                result.insert(key.clone(), value.clone());
            }
        }
        Ok(result)
    }

    async fn set_many(
        &self,
        entries: &[(CacheKey, CacheValue, Option<CacheTtl>)],
    ) -> Result<(), Self::Error> {
        let mut cache = self.cache.write().await;
        for (key, value, _ttl) in entries {
            cache.insert(key.clone(), value.clone());
        }
        Ok(())
    }

    async fn delete_many(&self, keys: &[CacheKey]) -> Result<u64, Self::Error> {
        let mut cache = self.cache.write().await;
        let mut count = 0;
        for key in keys {
            if cache.remove(key).is_some() {
                count += 1;
            }
        }
        Ok(count)
    }

    async fn delete_by_pattern(&self, _pattern: &str) -> Result<u64, Self::Error> {
        Ok(0) // Mock implementation doesn't support patterns
    }

    async fn scan_keys(
        &self,
        _pattern: &str,
        limit: Option<u64>,
    ) -> Result<Vec<CacheKey>, Self::Error> {
        let cache = self.cache.read().await;
        let keys: Vec<CacheKey> = cache
            .keys()
            .take(limit.unwrap_or(u64::MAX) as usize)
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn clear(&self) -> Result<(), Self::Error> {
        self.cache.write().await.clear();
        Ok(())
    }

    async fn flush(&self) -> Result<(), Self::Error> {
        Ok(()) // No-op for mock
    }

    async fn execute_lua_script(
        &self,
        _script: &str,
        _keys: Vec<String>,
        _args: Vec<String>,
    ) -> Result<CacheValue, Self::Error> {
        Ok(CacheValue::String("null".to_string()))
    }
}

pub struct MockMonitoringService {
    metrics: Arc<RwLock<HashMap<String, f64>>>,
}

impl MockMonitoringService {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn record_metric(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.write().await;
        // Increment the metric value if it already exists
        let current = metrics.get(name).copied().unwrap_or(0.0);
        metrics.insert(name.to_string(), current + value);
    }

    pub async fn get_all_metrics(&self) -> HashMap<String, f64> {
        self.metrics.read().await.clone()
    }
}

#[async_trait]
impl MonitoringService for MockMonitoringService {
    type Error = MonitoringError;

    async fn initialize(&self, _config: &MonitoringConfig) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn check_service_health(&self, _service_name: &str) -> Result<HealthReport, Self::Error> {
        Ok(HealthReport {
            service_name: "test".to_string(),
            status: HealthStatus::Healthy,
            score: 100.0,
            timestamp: chrono::Utc::now(),
            details: "Service is healthy".to_string(),
            metrics: Vec::new(),
        })
    }

    async fn check_all_services_health(&self) -> Result<SystemHealthReport, Self::Error> {
        Ok(SystemHealthReport {
            overall_status: HealthStatus::Healthy,
            overall_score: 100.0,
            timestamp: chrono::Utc::now(),
            service_reports: Vec::new(),
            summary: HealthSummary {
                total_services: 0,
                healthy_services: 0,
                degraded_services: 0,
                unhealthy_services: 0,
                critical_services: 0,
                health_percentage: 100.0,
            },
        })
    }

    async fn register_service_for_monitoring(
        &self,
        _service_name: &str,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn unregister_service_from_monitoring(
        &self,
        _service_name: &str,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn record_metric(&self, name: &str, value: f64) -> Result<(), Self::Error> {
        let mut metrics = self.metrics.write().await;
        let current = metrics.get(name).copied().unwrap_or(0.0);
        metrics.insert(name.to_string(), current + value);
        Ok(())
    }

    async fn increment_counter(&self, name: &str, value: u64) -> Result<(), Self::Error> {
        let mut metrics = self.metrics.write().await;
        let current = metrics.get(name).copied().unwrap_or(0.0);
        metrics.insert(name.to_string(), current + value as f64);
        Ok(())
    }

    async fn collect_performance_metrics(&self) -> Result<PerformanceReport, Self::Error> {
        Ok(PerformanceReport {
            timestamp: chrono::Utc::now(),
            service_metrics: Vec::new(),
            system_metrics: SystemPerformanceMetrics {
                total_services: 0,
                average_response_time_ms: 0.0,
                total_cpu_usage_percent: 0.0,
                total_memory_usage_mb: 0,
                system_health_score: 100.0,
            },
            analysis: PerformanceAnalysisSummary {
                bottlenecks_detected: 0,
                services_with_issues: Vec::new(),
                recommendations: Vec::new(),
                overall_performance_score: 100.0,
            },
        })
    }

    async fn collect_service_performance_metrics(
        &self,
        _service_name: &str,
    ) -> Result<Vec<PerformanceMetric>, Self::Error> {
        Ok(Vec::new())
    }

    async fn detect_bottlenecks(&self) -> Result<Vec<BottleneckReport>, Self::Error> {
        Ok(Vec::new())
    }

    async fn start_real_time_monitoring(&self) -> Result<MonitoringSession, Self::Error> {
        Ok(MonitoringSession {
            id: "test-session".to_string(),
            name: "Test Session".to_string(),
            status: MonitoringSessionStatus::Running,
            start_time: chrono::Utc::now(),
            end_time: None,
            monitored_services: Vec::new(),
            configuration: SessionConfiguration {
                collection_interval_seconds: 60,
                alert_thresholds_enabled: true,
                telemetry_integration_enabled: true,
                max_duration_minutes: None,
            },
            metrics_collected: 0,
            alerts_generated: 0,
        })
    }

    async fn stop_real_time_monitoring(&self, _session_id: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn get_monitoring_session_status(
        &self,
        _session_id: &str,
    ) -> Result<MonitoringSessionStatus, Self::Error> {
        Ok(MonitoringSessionStatus::Running)
    }

    async fn process_alert(&self, _alert: Alert) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn get_active_alerts(&self) -> Result<Vec<Alert>, Self::Error> {
        Ok(Vec::new())
    }

    async fn health_check(&self) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct MockHealthCheckService {
    checks: Arc<RwLock<HashMap<String, bool>>>,
}

impl MockHealthCheckService {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_check(&self, name: &str) {
        self.checks.write().await.insert(name.to_string(), true);
    }

    pub async fn set_health(&self, name: &str, healthy: bool) {
        self.checks.write().await.insert(name.to_string(), healthy);
    }

    pub async fn is_healthy(&self, name: &str) -> bool {
        self.checks.read().await.get(name).copied().unwrap_or(false)
    }

    pub async fn overall_health(&self) -> bool {
        self.checks.read().await.values().all(|&h| h)
    }
}

// Mock DBMS Service
pub struct MockDBMSService;

#[async_trait]
impl DBMSService for MockDBMSService {
    type Error = DBMSError;

    // Database Lifecycle Management
    async fn create_database(
        &self,
        name: &str,
        config: DatabaseConfig,
    ) -> DBMSResult<DatabaseInfo> {
        use std::collections::HashSet;
        use uuid::Uuid;
        // Convert config::DatabaseConfig to types::DatabaseConfig
        let types_config = TypesConfig {
            enabled_features: HashSet::new(), // Default to no features enabled
            max_connections: config.connection_limits.max_connections,
            connection_timeout_secs: config.connection_limits.connection_timeout_secs,
            query_timeout_secs: config.query_limits.max_execution_time_secs,
            transaction_timeout_secs: 600, // Default transaction timeout
            backup_interval_hours: 24,     // Default backup interval
            max_backup_files: 7,           // Default backup file retention
            database_settings: config.custom_settings,
        };

        Ok(DatabaseInfo {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: Some("Mock database".to_string()),
            created_at: chrono::Utc::now(),
            modified_at: chrono::Utc::now(),
            owner: "mock-owner".to_string(),
            config: types_config,
            status: DatabaseStatus::Active,
            size_bytes: 0,
            entity_count: 0,
            relation_count: 0,
            attribute_count: 0,
            schema_version: None,
            tags: Vec::new(),
            metadata: std::collections::HashMap::new(),
        })
    }

    async fn delete_database(&self, _name: &str) -> DBMSResult<()> {
        Ok(())
    }

    async fn list_databases(&self) -> DBMSResult<Vec<DatabaseInfo>> {
        Ok(Vec::new())
    }

    async fn get_database_status(&self, name: &str) -> DBMSResult<DatabaseStatus> {
        let _ = name;
        Ok(DatabaseStatus::Active)
    }

    // Connection Management
    async fn get_connection(
        &self,
        _database: &str,
    ) -> DBMSResult<Arc<dyn DatabaseConnection<Error = Self::Error>>> {
        Err(DBMSError::Configuration {
            message: "Mock connection not implemented".to_string(),
            field: None,
            value: None,
        })
    }

    async fn get_connection_pool(
        &self,
        _database: &str,
    ) -> DBMSResult<Arc<dyn ConnectionPool<Error = Self::Error>>> {
        Err(DBMSError::Configuration {
            message: "Mock connection pool not implemented".to_string(),
            field: None,
            value: None,
        })
    }

    async fn health_check(&self, _database: &str) -> DBMSResult<DBHealthStatus> {
        Ok(DBHealthStatus {
            status: dbms_core::types::HealthState::Healthy,
            timestamp: chrono::Utc::now(),
            database: "mock-db".to_string(),
            components: HashMap::new(),
            details: Some("Mock health check successful".to_string()),
            check_duration_ms: 10,
        })
    }

    // Schema Management
    async fn deploy_schema(&self, _database: &str, _schema: &str) -> DBMSResult<()> {
        Ok(())
    }

    async fn validate_schema(&self, _schema: &str) -> DBMSResult<SchemaValidation> {
        Ok(SchemaValidation {
            is_valid: true,
            schema_version: "1.0.0".to_string(),
            validated_at: chrono::Utc::now(),
            errors: Vec::new(),
            warnings: Vec::new(),
            validation_duration_ms: 50,
            elements_validated: 10,
        })
    }

    async fn get_schema_version(&self, _database: &str) -> DBMSResult<SchemaVersion> {
        Ok(SchemaVersion {
            version: "1.0.0".to_string(),
            description: Some("Mock schema version".to_string()),
            deployed_at: chrono::Utc::now(),
            deployed_by: "mock-user".to_string(),
            content_hash: "mock-hash".to_string(),
            migrations: Vec::new(),
            status: dbms_core::types::SchemaVersionStatus::Active,
            previous_version: None,
            tags: Vec::new(),
        })
    }

    // Query Execution
    async fn execute_string_query(&self, _database: &str, _query: &str) -> DBMSResult<String> {
        Ok("{}".to_string())
    }

    // Performance Monitoring
    async fn get_database_metrics(&self, _database: &str) -> DBMSResult<DatabaseMetrics> {
        Ok(DatabaseMetrics {
            timestamp: chrono::Utc::now(),
            database: "mock-db".to_string(),
            connection_pool: dbms_core::types::ConnectionPoolMetrics {
                total_connections: 5,
                active_connections: 1,
                idle_connections: 4,
                waiting_connections: 0,
                avg_acquisition_time_ms: 1.0,
                max_acquisition_time_ms: 5.0,
                utilization_percent: 20.0,
                connection_timeouts: 0,
                connection_errors: 0,
            },
            query_performance: dbms_core::types::QueryPerformanceMetrics {
                total_queries: 100,
                avg_query_time_ms: 10.0,
                max_query_time_ms: 50.0,
                p95_query_time_ms: 25.0,
                slow_queries: 0,
                query_timeouts: 0,
                cache_hit_rate: 0.8,
            },
            resource_usage: dbms_core::types::ResourceUsageMetrics {
                disk_usage_bytes: 1024 * 1024,
                memory_usage_bytes: 100 * 1024 * 1024,
                cpu_usage_percent: 10.0,
                open_file_descriptors: 50,
                network_bytes_sent: 1024,
                network_bytes_received: 2048,
            },
            transaction_metrics: dbms_core::types::TransactionMetrics {
                total_transactions: 50,
                committed_transactions: 48,
                rolled_back_transactions: 2,
                avg_transaction_duration_ms: 15.0,
                max_transaction_duration_ms: 100.0,
                deadlocks_detected: 0,
            },
            schema_metrics: dbms_core::types::SchemaMetrics {
                entity_types: 10,
                relation_types: 5,
                attribute_types: 20,
                roles: 15,
                rules: 3,
                complexity_score: 25.0,
            },
            error_metrics: dbms_core::types::ErrorMetrics {
                total_errors: 2,
                connection_errors: 1,
                query_errors: 1,
                transaction_errors: 0,
                auth_errors: 0,
                error_rate_per_minute: 0.1,
            },
        })
    }

    async fn optimize_database(&self, database: &str) -> DBMSResult<OptimizationReport> {
        let mock_metrics = DatabaseMetrics {
            timestamp: chrono::Utc::now(),
            database: database.to_string(),
            connection_pool: dbms_core::types::ConnectionPoolMetrics {
                total_connections: 5,
                active_connections: 1,
                idle_connections: 4,
                waiting_connections: 0,
                avg_acquisition_time_ms: 1.0,
                max_acquisition_time_ms: 5.0,
                utilization_percent: 20.0,
                connection_timeouts: 0,
                connection_errors: 0,
            },
            query_performance: dbms_core::types::QueryPerformanceMetrics {
                total_queries: 100,
                avg_query_time_ms: 10.0,
                max_query_time_ms: 50.0,
                p95_query_time_ms: 25.0,
                slow_queries: 0,
                query_timeouts: 0,
                cache_hit_rate: 0.8,
            },
            resource_usage: dbms_core::types::ResourceUsageMetrics {
                disk_usage_bytes: 1024 * 1024,
                memory_usage_bytes: 100 * 1024 * 1024,
                cpu_usage_percent: 10.0,
                open_file_descriptors: 50,
                network_bytes_sent: 1024,
                network_bytes_received: 2048,
            },
            transaction_metrics: dbms_core::types::TransactionMetrics {
                total_transactions: 50,
                committed_transactions: 48,
                rolled_back_transactions: 2,
                avg_transaction_duration_ms: 15.0,
                max_transaction_duration_ms: 100.0,
                deadlocks_detected: 0,
            },
            schema_metrics: dbms_core::types::SchemaMetrics {
                entity_types: 10,
                relation_types: 5,
                attribute_types: 20,
                roles: 15,
                rules: 3,
                complexity_score: 25.0,
            },
            error_metrics: dbms_core::types::ErrorMetrics {
                total_errors: 2,
                connection_errors: 1,
                query_errors: 1,
                transaction_errors: 0,
                auth_errors: 0,
                error_rate_per_minute: 0.1,
            },
        };

        Ok(OptimizationReport {
            generated_at: chrono::Utc::now(),
            database: database.to_string(),
            optimization_score: 85.0,
            recommendations: Vec::new(),
            before_metrics: mock_metrics.clone(),
            projected_metrics: Some(mock_metrics),
            estimated_improvement: 5.0,
        })
    }

    // Event Integration
    async fn get_events(
        &self,
        _database: Option<&str>,
        _since: chrono::DateTime<chrono::Utc>,
        _limit: u32,
    ) -> DBMSResult<Vec<DatabaseEvent>> {
        Ok(Vec::new())
    }

    // Database Export/Import Operations
    async fn export_database(&self, _database: &str, _include_data: bool) -> DBMSResult<String> {
        Ok("define
  entity sub entity;".to_string())
    }

    async fn import_database(
        &self,
        _database: &str,
        _content: &str,
        _create_if_not_exists: bool,
    ) -> DBMSResult<u64> {
        Ok(0)
    }
}

// Mock Timeout Service
use std::time::Duration;
// Removed linkml_core::error::Result to use proper error types

pub struct MockTimeoutService;

#[async_trait]
impl TimeoutService for MockTimeoutService {
    type Error = TimeoutError;

    async fn calculate_timeout(
        &self,
        _operation: &str,
        _context: Option<&TimeoutContext>,
    ) -> Result<TimeoutValue, Self::Error> {
        Ok(TimeoutValue {
            duration: Duration::from_secs(30),
            algorithm: timeout_core::TimeoutAlgorithm::Static,
            confidence: 0.8,
            jitter: None,
            calculated_at: chrono::Utc::now(),
        })
    }

    async fn record_duration(
        &self,
        _operation: &str,
        _duration: Duration,
        _success: bool,
        _context: Option<&TimeoutContext>,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn get_history(
        &self,
        _operation: &str,
        _limit: Option<usize>,
    ) -> Result<TimeoutHistory, Self::Error> {
        Ok(TimeoutHistory {
            operation: "test".to_string(),
            records: Vec::new(),
            current_timeout: None,
            statistics: None,
        })
    }

    async fn get_statistics(&self, _operation: &str) -> Result<TimeoutStatistics, Self::Error> {
        Ok(TimeoutStatistics {
            count: 0,
            mean: Duration::from_secs(1),
            median: Duration::from_secs(1),
            std_dev: Duration::from_millis(100),
            min: Duration::from_secs(1),
            max: Duration::from_secs(1),
            percentiles: timeout_core::PercentileData {
                p50: Duration::from_secs(1),
                p75: Duration::from_secs(1),
                p90: Duration::from_secs(1),
                p95: Duration::from_secs(1),
                p99: Duration::from_secs(1),
            },
            success_rate: 1.0,
            timeout_violation_rate: 0.0,
        })
    }

    async fn clear_history(&self, _operation: &str) -> Result<(), Self::Error> {
        Ok(())
    }

    async fn update_config(&self, _config: TimeoutConfig) -> Result<(), Self::Error> {
        Ok(())
    }
}

pub struct MockRandomService {
    counter: Arc<RwLock<u64>>,
}

impl MockRandomService {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(RwLock::new(0)),
        }
    }

    async fn next_counter(&self) -> u64 {
        let mut counter = self.counter.write().await;
        *counter = counter.wrapping_add(1);
        *counter
    }
}

#[async_trait]
impl RandomService for MockRandomService {
    type Error = RandomError;

    async fn generate_u32(&self) -> RandomResult<u32> {
        Ok(self.next_counter().await as u32)
    }

    async fn generate_u64(&self) -> RandomResult<u64> {
        Ok(self.next_counter().await)
    }

    async fn generate_i32(&self) -> RandomResult<i32> {
        Ok(self.next_counter().await as i32)
    }

    async fn generate_i64(&self) -> RandomResult<i64> {
        Ok(self.next_counter().await as i64)
    }

    async fn generate_f32(&self) -> RandomResult<f32> {
        let counter = self.next_counter().await;
        Ok((counter % 1000) as f32 / 1000.0) // Deterministic value between 0.0 and 1.0
    }

    async fn generate_f64(&self) -> RandomResult<f64> {
        let counter = self.next_counter().await;
        Ok((counter % 1000) as f64 / 1000.0) // Deterministic value between 0.0 and 1.0
    }

    async fn generate_bool(&self) -> RandomResult<bool> {
        Ok(self.next_counter().await % 2 == 0)
    }

    async fn fill_bytes(&self, dest: &mut [u8]) -> RandomResult<()> {
        for byte in dest.iter_mut() {
            *byte = (self.next_counter().await % 256) as u8;
        }
        Ok(())
    }

    fn fill_bytes_sync(&self, dest: &mut [u8]) -> RandomResult<()> {
        // For test purposes, use a simple deterministic pattern
        for (i, byte) in dest.iter_mut().enumerate() {
            *byte = (i % 256) as u8;
        }
        Ok(())
    }

    fn generate_u32_sync(&self) -> RandomResult<u32> {
        // For test purposes, return a fixed value
        Ok(42)
    }

    fn generate_u64_sync(&self) -> RandomResult<u64> {
        // For test purposes, return a fixed value
        Ok(42)
    }

    fn create_sync_rng(&self) -> Box<dyn random_core::SyncCryptoRng> {
        use rand::{RngCore, CryptoRng};

        // Simple deterministic RNG for tests
        struct TestRng(u64);

        impl RngCore for TestRng {
            fn next_u32(&mut self) -> u32 {
                self.0 = self.0.wrapping_add(1);
                self.0 as u32
            }

            fn next_u64(&mut self) -> u64 {
                self.0 = self.0.wrapping_add(1);
                self.0
            }

            fn fill_bytes(&mut self, dest: &mut [u8]) {
                for (i, byte) in dest.iter_mut().enumerate() {
                    *byte = (i % 256) as u8;
                }
            }

            fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), rand::Error> {
                self.fill_bytes(dest);
                Ok(())
            }
        }

        impl CryptoRng for TestRng {}

        Box::new(TestRng(0))
    }

    async fn generate_u32_range(&self, min: u32, max: u32) -> RandomResult<u32> {
        if min >= max {
            return Ok(min);
        }
        let range = max - min;
        let value = self.next_counter().await as u32;
        Ok(min + (value % range))
    }

    async fn generate_u64_range(&self, min: u64, max: u64) -> RandomResult<u64> {
        if min >= max {
            return Ok(min);
        }
        let range = max - min;
        let value = self.next_counter().await;
        Ok(min + (value % range))
    }

    async fn generate_i32_range(&self, min: i32, max: i32) -> RandomResult<i32> {
        if min >= max {
            return Ok(min);
        }
        let range = (max - min) as u32;
        let value = self.next_counter().await as u32;
        Ok(min + (value % range) as i32)
    }

    async fn generate_i64_range(&self, min: i64, max: i64) -> RandomResult<i64> {
        if min >= max {
            return Ok(min);
        }
        let range = (max - min) as u64;
        let value = self.next_counter().await;
        Ok(min + (value % range) as i64)
    }

    async fn generate_f32_range(&self, min: f32, max: f32) -> RandomResult<f32> {
        let value = self.generate_f32().await?;
        Ok(min + value * (max - min))
    }

    async fn generate_f64_range(&self, min: f64, max: f64) -> RandomResult<f64> {
        let value = self.generate_f64().await?;
        Ok(min + value * (max - min))
    }

    async fn generate_beta(&self, _params: BetaParams) -> RandomResult<f64> {
        self.generate_f64().await // Simplified for tests
    }

    async fn generate_gamma(&self, _params: GammaParams) -> RandomResult<f64> {
        self.generate_f64().await // Simplified for tests
    }

    async fn generate_binomial(&self, _params: BinomialParams) -> RandomResult<u32> {
        self.generate_u32().await // Simplified for tests
    }

    async fn generate_poisson(&self, _params: PoissonParams) -> RandomResult<u32> {
        self.generate_u32().await // Simplified for tests
    }

    async fn generate_normal(&self, _params: NormalParams) -> RandomResult<f64> {
        self.generate_f64().await // Simplified for tests
    }

    async fn generate_exponential(&self, _params: ExponentialParams) -> RandomResult<f64> {
        self.generate_f64().await // Simplified for tests
    }

    async fn set_seed(&self, _seed: SeedValue) -> RandomResult<()> {
        Ok(()) // Mock implementation ignores seed
    }

    async fn get_entropy_level(&self) -> RandomResult<f64> {
        Ok(1.0) // Mock always reports perfect entropy
    }

    async fn switch_backend(&self, _backend: BackendType) -> RandomResult<()> {
        Ok(()) // Mock implementation ignores backend switching
    }

    async fn get_current_backend(&self) -> RandomResult<BackendType> {
        Ok(BackendType::ChaCha20) // Mock always reports ChaCha20
    }

    async fn get_config(&self) -> RandomResult<RandomConfig> {
        Ok(RandomConfig::default()) // Return default config
    }

    async fn update_config(&self, _config: RandomConfig) -> RandomResult<()> {
        Ok(()) // Mock implementation ignores config updates
    }

    async fn generate_vec_u32(&self, count: usize) -> RandomResult<Vec<u32>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.generate_u32().await?);
        }
        Ok(result)
    }

    async fn generate_vec_normal(&self, _params: NormalParams, count: usize) -> RandomResult<Vec<f64>> {
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(self.generate_f64().await?);
        }
        Ok(result)
    }

    async fn shuffle_bytes(&self, slice: &mut [u8]) -> RandomResult<()> {
        // Simple reverse for deterministic testing
        slice.reverse();
        Ok(())
    }

    async fn shuffle_u16(&self, slice: &mut [u16]) -> RandomResult<()> {
        // Simple reverse for deterministic testing
        slice.reverse();
        Ok(())
    }

    async fn shuffle_u32(&self, slice: &mut [u32]) -> RandomResult<()> {
        // Simple reverse for deterministic testing
        slice.reverse();
        Ok(())
    }

    async fn shuffle_u64(&self, slice: &mut [u64]) -> RandomResult<()> {
        // Simple reverse for deterministic testing
        slice.reverse();
        Ok(())
    }

    async fn sample_u32_without_replacement(&self, collection: &[u32], n: usize) -> RandomResult<Vec<u32>> {
        let sample_size = n.min(collection.len());
        Ok(collection.iter().take(sample_size).copied().collect())
    }

    async fn sample_u64_without_replacement(&self, collection: &[u64], n: usize) -> RandomResult<Vec<u64>> {
        let sample_size = n.min(collection.len());
        Ok(collection.iter().take(sample_size).copied().collect())
    }

    async fn generate_uuid_v4(&self) -> RandomResult<String> {
        let counter = self.next_counter().await;
        Ok(format!("00000000-0000-4000-8000-{:012x}", counter))
    }

    async fn generate_uuid_v4_bytes(&self) -> RandomResult<[u8; 16]> {
        let counter = self.next_counter().await;
        let mut bytes = [0u8; 16];
        bytes[8..16].copy_from_slice(&counter.to_be_bytes());
        bytes[6] = 0x40; // Version 4
        bytes[8] = 0x80; // Variant bits
        Ok(bytes)
    }

    async fn shuffle<T: Send>(&self, _slice: &mut [T]) -> RandomResult<()> {
        Ok(())
    }
    async fn sample_without_replacement<T: Clone + Send + Sync>(
        &self,
        collection: &[T],
        n: usize,
    ) -> RandomResult<Vec<T>> {
        // For test implementations, just return first n elements
        if n > collection.len() {
            return Ok(collection.to_vec());
        }
        Ok(collection[..n].to_vec())
    }
}

pub fn create_mock_random_service() -> Arc<dyn RandomService<Error = RandomError>> {
    Arc::new(MockRandomService::new())
}
