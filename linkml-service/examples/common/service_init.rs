//! Common service initialization for examples
//!
//! Provides real service initialization for LinkML examples
//! instead of using todo!() macros.

use linkml_core::config::LinkMLConfig;
use linkml_service::factory::LinkMLServiceDependencies;
use linkml_service::service::LinkMLServiceImpl;
use std::sync::Arc;

// Mock service implementations for examples
use mock_services::{
    MockCache, MockConfigService, MockDBMS, MockErrorHandler, MockLogger, MockMonitor,
    MockTaskManager, MockTimeout, MockTimestamp,
};

mod mock_services {
    use async_trait::async_trait;
    use parking_lot::RwLock;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Mock Logger
    pub struct MockLogger;

    #[async_trait]
    impl logger_core::LoggerService for MockLogger {
        type Error = logger_core::LoggerError;

        async fn trace(&self, _msg: &str) -> Result<(), Self::Error> {
            Ok(())
        }
        async fn debug(&self, msg: &str) -> Result<(), Self::Error> {
            println!("[DEBUG] {}", msg);
            Ok(())
        }
        async fn info(&self, msg: &str) -> Result<(), Self::Error> {
            println!("[INFO] {}", msg);
            Ok(())
        }
        async fn warn(&self, msg: &str) -> Result<(), Self::Error> {
            println!("[WARN] {}", msg);
            Ok(())
        }
        async fn error(&self, msg: &str) -> Result<(), Self::Error> {
            eprintln!("[ERROR] {}", msg);
            Ok(())
        }
        async fn fatal(&self, msg: &str) -> Result<(), Self::Error> {
            eprintln!("[FATAL] {}", msg);
            Ok(())
        }
        async fn flush(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // Mock Timestamp
    pub struct MockTimestamp;

    #[async_trait]
    impl timestamp_core::TimestampService for MockTimestamp {
        type Error = timestamp_core::TimestampError;

        async fn current_timestamp(&self) -> Result<i64, Self::Error> {
            Ok(chrono::Utc::now().timestamp_nanos())
        }

        async fn format_timestamp(&self, ts: i64) -> Result<String, Self::Error> {
            Ok(format!("{}", ts))
        }
    }

    // Mock Cache
    pub struct MockCache {
        cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    }

    impl MockCache {
        pub fn new() -> Self {
            Self {
                cache: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl cache_core::CacheService for MockCache {
        type Error = cache_core::CacheError;

        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Self::Error> {
            Ok(self.cache.read().get(key).cloned())
        }

        async fn set(
            &self,
            key: &str,
            value: Vec<u8>,
            _ttl: Option<u64>,
        ) -> Result<(), Self::Error> {
            self.cache.write().insert(key.to_string(), value);
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<(), Self::Error> {
            self.cache.write().remove(key);
            Ok(())
        }

        async fn exists(&self, key: &str) -> Result<bool, Self::Error> {
            Ok(self.cache.read().contains_key(key))
        }

        async fn expire(&self, _key: &str, _ttl: u64) -> Result<(), Self::Error> {
            Ok(())
        }

        async fn clear(&self) -> Result<(), Self::Error> {
            self.cache.write().clear();
            Ok(())
        }
    }

    // Mock Monitor
    pub struct MockMonitor;

    #[async_trait]
    impl monitoring_core::MonitoringService for MockMonitor {
        type Error = monitoring_core::MonitoringError;

        async fn record_metric(&self, name: &str, value: f64) -> Result<(), Self::Error> {
            println!("[METRIC] {} = {}", name, value);
            Ok(())
        }

        async fn increment_counter(&self, name: &str) -> Result<(), Self::Error> {
            println!("[COUNTER] {} += 1", name);
            Ok(())
        }

        async fn record_histogram(&self, name: &str, value: f64) -> Result<(), Self::Error> {
            println!("[HISTOGRAM] {} = {}", name, value);
            Ok(())
        }

        async fn check_health(&self) -> Result<monitoring_core::HealthStatus, Self::Error> {
            Ok(monitoring_core::HealthStatus::Healthy)
        }
    }

    // Mock Task Manager
    pub struct MockTaskManager;

    #[async_trait]
    impl task_management_core::TaskManagementService for MockTaskManager {
        type Error = task_management_core::TaskError;

        async fn spawn_task(
            &self,
            task: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>,
            _options: Option<task_management_core::TaskOptions>,
        ) -> Result<task_management_core::TaskId, Self::Error> {
            tokio::spawn(task);
            Ok(task_management_core::TaskId::new())
        }

        async fn cancel_task(
            &self,
            _id: &task_management_core::TaskId,
        ) -> Result<bool, Self::Error> {
            Ok(true)
        }

        async fn get_task_status(
            &self,
            _id: &task_management_core::TaskId,
        ) -> Result<task_management_core::TaskStatus, Self::Error> {
            Ok(task_management_core::TaskStatus::Running)
        }

        async fn list_tasks(&self) -> Result<Vec<task_management_core::TaskInfo>, Self::Error> {
            Ok(vec![])
        }
    }

    // Mock Error Handler
    pub struct MockErrorHandler;

    #[async_trait]
    impl error_handling_core::ErrorHandlingService for MockErrorHandler {
        type Error = error_handling_core::ErrorHandlingError;

        async fn handle_error(
            &self,
            context: &str,
            error: &str,
            _metadata: HashMap<String, String>,
        ) -> Result<(), Self::Error> {
            eprintln!("[ERROR_HANDLER] Context: {}, Error: {}", context, error);
            Ok(())
        }

        async fn get_error_stats(&self) -> Result<error_handling_core::ErrorStats, Self::Error> {
            Ok(error_handling_core::ErrorStats::default())
        }
    }

    // Mock Config Service
    pub struct MockConfigService {
        config: Arc<RwLock<HashMap<String, serde_json::Value>>>,
    }

    impl MockConfigService {
        pub fn new() -> Self {
            Self {
                config: Arc::new(RwLock::new(HashMap::new())),
            }
        }
    }

    #[async_trait]
    impl configuration_core::ConfigurationService for MockConfigService {
        type Error = configuration_core::ConfigurationError;

        async fn get(&self, key: &str) -> Result<serde_json::Value, Self::Error> {
            self.config
                .read()
                .get(key)
                .cloned()
                .ok_or_else(|| configuration_core::ConfigurationError::NotFound(key.to_string()))
        }

        async fn set(&self, key: &str, value: serde_json::Value) -> Result<(), Self::Error> {
            self.config.write().insert(key.to_string(), value);
            Ok(())
        }

        async fn reload(&self) -> Result<(), Self::Error> {
            Ok(())
        }
    }

    // Mock DBMS
    pub struct MockDBMS;

    #[async_trait]
    impl dbms_core::DBMSService for MockDBMS {
        type Error = dbms_core::DBMSError;

        async fn execute_query(&self, _query: &str) -> Result<dbms_core::QueryResult, Self::Error> {
            Ok(dbms_core::QueryResult::default())
        }

        async fn check_health(&self) -> Result<dbms_core::HealthStatus, Self::Error> {
            Ok(dbms_core::HealthStatus::Healthy)
        }
    }

    // Mock Timeout
    pub struct MockTimeout;

    #[async_trait]
    impl timeout_core::TimeoutService for MockTimeout {
        type Error = timeout_core::TimeoutError;

        async fn with_timeout<F, T>(
            &self,
            _duration: std::time::Duration,
            future: F,
        ) -> Result<T, Self::Error>
        where
            F: std::future::Future<Output = T> + Send,
            T: Send,
        {
            Ok(future.await)
        }
    }
}

/// Initialize LinkML service with mock dependencies for examples
///
/// This creates a fully functional LinkML service using mock implementations
/// of RootReal services, suitable for demonstration and testing purposes.
pub async fn initialize_example_service() -> Arc<
    LinkMLServiceImpl<MockTaskManager, MockErrorHandler, MockConfigService, MockDBMS, MockTimeout>,
> {
    // Create mock services
    let logger = Arc::new(MockLogger);
    let timestamp = Arc::new(MockTimestamp);
    let cache = Arc::new(MockCache::new());
    let monitor = Arc::new(MockMonitor);
    let task_manager = Arc::new(MockTaskManager);
    let error_handler = Arc::new(MockErrorHandler);
    let config_service = Arc::new(MockConfigService::new());
    let dbms = Arc::new(MockDBMS);
    let timeout = Arc::new(MockTimeout);

    // Create service dependencies
    let deps = LinkMLServiceDependencies {
        logger,
        timestamp,
        cache,
        monitor,
        config_service,
        task_manager,
        error_handler,
        dbms_service: dbms,
        timeout_service: timeout,
    };

    // Create service with default configuration
    let config = LinkMLConfig::default();
    let service =
        LinkMLServiceImpl::with_config(config, deps).expect("Failed to create LinkML service");

    // Initialize the service
    service
        .initialize()
        .await
        .expect("Failed to initialize LinkML service");

    Arc::new(service)
}
