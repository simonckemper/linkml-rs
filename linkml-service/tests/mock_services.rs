//! Mock services for testing LinkML service
//!
//! These mocks implement the required RootReal service traits
//! for testing purposes only.

use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use async_trait::async_trait;
use std::future::Future;

// Import core types and service traits
use logger_core::{LogLevel, LogEntry, LoggerError, LoggerService};
use timestamp_core::{TimestampError, TimestampService};
use task_management_core::{TaskManagementError, TaskManagementService, TaskId, TaskOptions};
use error_handling_core::{ErrorHandlingError, ErrorHandlingService, ErrorCategory, ErrorContext, ErrorPattern, RecoveryStrategy, TransientErrorType};
use configuration_core::{ConfigurationError, ConfigurationService};
use cache_core::{CacheError, CacheService, CacheKey, CacheValue, CacheTtl};
use monitoring_core::{
    MonitoringError, MonitoringService, MonitoringConfig,
    HealthReport, HealthStatus, SystemHealthReport, PerformanceReport, 
    PerformanceMetric, BottleneckReport, MonitoringSession, MonitoringSessionStatus, Alert,
    HealthSummary, SystemPerformanceMetrics,
    PerformanceAnalysisSummary, SessionConfiguration
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
    
    #[allow(dead_code)]
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
        self.logs.write().await.push(format!("[{}] {}", level_str, message));
        Ok(())
    }
    
    async fn log_entry(&self, entry: &LogEntry) -> Result<(), Self::Error> {
        self.log(entry.level, &entry.message).await
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
    
    async fn parse_iso8601(&self, timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
        use chrono::DateTime;
        Ok(DateTime::parse_from_rfc3339(timestamp)
            .map_err(|e| TimestampError::ParseError { message: e.to_string() })?
            .with_timezone(&chrono::Utc))
    }
    
    async fn format_iso8601(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> Result<String, Self::Error> {
        Ok(timestamp.to_rfc3339())
    }
    
    async fn duration_since(&self, timestamp: &chrono::DateTime<chrono::Utc>) -> Result<chrono::Duration, Self::Error> {
        Ok(chrono::Utc::now() - *timestamp)
    }
    
    async fn add_duration(&self, timestamp: &chrono::DateTime<chrono::Utc>, duration: chrono::Duration) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
        Ok(*timestamp + duration)
    }
    
    async fn subtract_duration(&self, timestamp: &chrono::DateTime<chrono::Utc>, duration: chrono::Duration) -> Result<chrono::DateTime<chrono::Utc>, Self::Error> {
        Ok(*timestamp - duration)
    }
    
    async fn duration_between(&self, from: &chrono::DateTime<chrono::Utc>, to: &chrono::DateTime<chrono::Utc>) -> Result<chrono::Duration, Self::Error> {
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
        Err(TaskManagementError::TaskNotFound { task_id: "test-task".to_string() })
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
    
    #[allow(dead_code)]
    pub fn set(&self, key: &str, value: &str) {
        // Since config is not mutable, this is a no-op for the mock
        // In a real implementation, you'd need Arc<RwLock<HashMap>> or similar
        let _ = (key, value);
    }
    
    #[allow(dead_code)]
    pub fn get(&self, key: &str) -> Option<String> {
        self.config.get(key).cloned()
    }
}

#[async_trait]
impl ConfigurationService for MockConfigurationService {
    type Error = ConfigurationError;
    
    async fn load_configuration<T>(&self) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de> + configuration_core::Validate + Clone + Send + Sync + 'static,
    {
        Err(ConfigurationError::LoadingError { message: "Mock load not implemented".to_string() })
    }
    
    async fn load_configuration_from_source<T>(&self, _source: &str) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de> + configuration_core::Validate + Clone + Send + Sync + 'static,
    {
        Err(ConfigurationError::LoadingError { message: "Mock load not implemented".to_string() })
    }
    
    async fn get_configuration<T>(&self, key: &str) -> Result<T, Self::Error>
    where
        T: for<'de> serde::Deserialize<'de> + serde::Serialize + Clone + Send + Sync + 'static,
    {
        let value = self.config.get(key)
            .ok_or_else(|| ConfigurationError::ValidationError { message: format!("Key not found: {}", key) })?;
        serde_json::from_str(value).map_err(|_| ConfigurationError::ParsingError { message: "Mock parse error".to_string() })
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
    
    async fn get_configuration_metadata(&self, _source: &str) -> Result<configuration_core::ConfigurationMetadata, Self::Error> {
        Err(ConfigurationError::SourceError { message: "Mock metadata not available".to_string() })
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
    
    #[allow(dead_code)]
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
    
    async fn set(&self, key: &CacheKey, value: &CacheValue, _ttl: Option<CacheTtl>) -> Result<(), Self::Error> {
        self.cache.write().await.insert(key.clone(), value.clone());
        Ok(())
    }
    
    async fn delete(&self, key: &CacheKey) -> Result<bool, Self::Error> {
        Ok(self.cache.write().await.remove(key).is_some())
    }
    
    async fn exists(&self, key: &CacheKey) -> Result<bool, Self::Error> {
        Ok(self.cache.read().await.contains_key(key))
    }
    
    async fn get_many(&self, keys: &[CacheKey]) -> Result<HashMap<CacheKey, CacheValue>, Self::Error> {
        let cache = self.cache.read().await;
        let mut result = HashMap::new();
        for key in keys {
            if let Some(value) = cache.get(key) {
                result.insert(key.clone(), value.clone());
            }
        }
        Ok(result)
    }
    
    async fn set_many(&self, entries: &[(CacheKey, CacheValue, Option<CacheTtl>)]) -> Result<(), Self::Error> {
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
    
    async fn scan_keys(&self, _pattern: &str, limit: Option<u64>) -> Result<Vec<CacheKey>, Self::Error> {
        let cache = self.cache.read().await;
        let keys: Vec<CacheKey> = cache.keys()
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
    _metrics: Arc<RwLock<HashMap<String, f64>>>,
}

impl MockMonitoringService {
    pub fn new() -> Self {
        Self {
            _metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn _record_metric(&self, name: &str, value: f64) {
        let mut metrics = self._metrics.write().await;
        // Increment the metric value if it already exists
        let current = metrics.get(name).copied().unwrap_or(0.0);
        metrics.insert(name.to_string(), current + value);
    }
    
    pub async fn _get_all_metrics(&self) -> HashMap<String, f64> {
        self._metrics.read().await.clone()
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
    
    async fn register_service_for_monitoring(&self, _service_name: &str) -> Result<(), Self::Error> {
        Ok(())
    }
    
    async fn unregister_service_from_monitoring(&self, _service_name: &str) -> Result<(), Self::Error> {
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
    
    async fn collect_service_performance_metrics(&self, _service_name: &str) -> Result<Vec<PerformanceMetric>, Self::Error> {
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
    
    async fn get_monitoring_session_status(&self, _session_id: &str) -> Result<MonitoringSessionStatus, Self::Error> {
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

#[allow(dead_code)]
pub struct MockHealthCheckService {
    checks: Arc<RwLock<HashMap<String, bool>>>,
}

#[allow(dead_code)]
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