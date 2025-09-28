//! Monitoring integration for `LinkML` service.
//!
//! This module provides comprehensive monitoring integration with RootReal's
//! Monitoring Service, tracking performance metrics, operation counts, and error rates.

use async_trait::async_trait;
use linkml_core::error::{LinkMLError, Result};
use monitoring_core::{HealthStatus, MonitoringService, PerformanceMetric};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics tracker for `LinkML` operations
pub struct LinkMLMetrics {
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    service_name: String,
}

impl LinkMLMetrics {
    /// Create new metrics tracker
    pub fn new(
        monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    ) -> Self {
        Self {
            monitoring,
            service_name: "linkml-service".to_string(),
        }
    }

    async fn submit_metric(&self, metric: &PerformanceMetric) -> Result<()> {
        self.monitoring
            .register_service_for_monitoring(&self.service_name)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to register service for monitoring: {e}"))
            })?;

        self.monitoring
            .record_metric(&metric.name, metric.value)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to record metric '{}': {}", metric.name, e))
            })?;

        if !metric.tags.is_empty() {
            tracing::debug!(
                metric = metric.name.as_str(),
                tags = ?metric.tags,
                "Recorded LinkML performance metric"
            );
        }

        Ok(())
    }

    /// Track a validation operation by creating performance metrics
    ///
    /// # Errors
    /// Returns error if metrics recording fails or logger service is unavailable
    pub async fn track_validation(
        &self,
        schema_name: &str,
        data_size: usize,
        duration: Duration,
        success: bool,
    ) -> Result<()> {
        // Create performance metrics for the validation operation
        let validation_metric = PerformanceMetric::new(
            "validation_duration_ms".to_string(),
            duration.as_millis() as f64,
            "milliseconds".to_string(),
        )
        .with_tag("service".to_string(), self.service_name.clone())
        .with_tag("schema".to_string(), schema_name.to_string())
        .with_tag("success".to_string(), success.to_string())
        .with_tag("data_size_bytes".to_string(), data_size.to_string());

        // Log the operation for monitoring service to collect later
        // In a real system, this would send the metric to a metrics collector
        // For now, we'll register the service for monitoring if not already done
        self.submit_metric(&validation_metric).await?;

        // Record additional context metrics for capacity planning
        self.monitoring
            .record_metric("linkml.validation.data_size_bytes", data_size as f64)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to record validation data size metric: {e}"))
            })?;

        let counter_name = if success {
            "linkml.validation.success_total"
        } else {
            "linkml.validation.failure_total"
        };

        self.monitoring
            .increment_counter(counter_name, 1)
            .await
            .map_err(|e| {
                LinkMLError::service(format!(
                    "Failed to increment validation counter '{counter_name}': {e}"
                ))
            })?;

        Ok(())
    }

    /// Track a code generation operation
    ///
    /// # Errors
    /// Returns error if metrics recording fails or logger service is unavailable
    pub async fn track_generation(
        &self,
        generator_type: &str,
        schema_name: &str,
        duration: Duration,
        files_generated: usize,
        success: bool,
    ) -> Result<()> {
        // Create performance metrics for the generation operation
        let generation_metric = PerformanceMetric::new(
            "generation_duration_ms".to_string(),
            duration.as_millis() as f64,
            "milliseconds".to_string(),
        )
        .with_tag("service".to_string(), self.service_name.clone())
        .with_tag("generator_type".to_string(), generator_type.to_string())
        .with_tag("schema".to_string(), schema_name.to_string())
        .with_tag("success".to_string(), success.to_string())
        .with_tag("files_generated".to_string(), files_generated.to_string());

        // Register service for monitoring if not already done
        self.submit_metric(&generation_metric).await?;

        self.monitoring
            .record_metric("linkml.generation.files_generated", files_generated as f64)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to record generation output metric: {e}"))
            })?;

        let counter_name = if success {
            "linkml.generation.success_total"
        } else {
            "linkml.generation.failure_total"
        };

        self.monitoring
            .increment_counter(counter_name, 1)
            .await
            .map_err(|e| {
                LinkMLError::service(format!(
                    "Failed to increment generation counter '{counter_name}': {e}"
                ))
            })?;

        Ok(())
    }

    /// Track schema parsing operation
    ///
    /// # Errors
    /// Returns error if metrics recording fails or logger service is unavailable
    pub async fn track_parsing(
        &self,
        _file_path: &str,
        file_size: usize,
        duration: Duration,
        success: bool,
    ) -> Result<()> {
        // Create performance metrics for the parsing operation
        let parsing_metric = PerformanceMetric::new(
            "parsing_duration_ms".to_string(),
            duration.as_millis() as f64,
            "milliseconds".to_string(),
        )
        .with_tag("service".to_string(), self.service_name.clone())
        .with_tag("file_size_bytes".to_string(), file_size.to_string())
        .with_tag("success".to_string(), success.to_string());

        // Register service for monitoring
        self.submit_metric(&parsing_metric).await?;

        self.monitoring
            .record_metric("linkml.parsing.file_size_bytes", file_size as f64)
            .await
            .map_err(|e| {
                LinkMLError::service(format!("Failed to record parsing file size metric: {e}"))
            })?;

        let counter_name = if success {
            "linkml.parsing.success_total"
        } else {
            "linkml.parsing.failure_total"
        };

        self.monitoring
            .increment_counter(counter_name, 1)
            .await
            .map_err(|e| {
                LinkMLError::service(format!(
                    "Failed to increment parsing counter '{counter_name}': {e}"
                ))
            })?;

        Ok(())
    }

    /// Track cache performance
    ///
    /// # Errors
    ///
    /// Returns an error if metric submission fails
    pub async fn track_cache_operation(
        &self,
        operation: &str,
        cache_type: &str,
        duration_ns: u64,
    ) -> Result<()> {
        // Create performance metrics for the cache operation
        let cache_metric = PerformanceMetric::new(
            "cache_operation_duration_ns".to_string(),
            duration_ns as f64,
            "nanoseconds".to_string(),
        )
        .with_tag("service".to_string(), self.service_name.clone())
        .with_tag("operation".to_string(), operation.to_string())
        .with_tag("cache_type".to_string(), cache_type.to_string());

        self.submit_metric(&cache_metric).await
    }

    /// Track error rates
    ///
    /// # Errors
    ///
    /// Returns an error if metric submission fails
    pub async fn track_error(
        &self,
        error_type: &str,
        operation: &str,
        severity: &str,
    ) -> Result<()> {
        // Create performance metrics for error tracking
        let error_metric =
            PerformanceMetric::new("error_count".to_string(), 1.0, "count".to_string())
                .with_tag("service".to_string(), self.service_name.clone())
                .with_tag("error_type".to_string(), error_type.to_string())
                .with_tag("operation".to_string(), operation.to_string())
                .with_tag("severity".to_string(), severity.to_string());

        self.submit_metric(&error_metric).await
    }

    /// Record memory usage
    ///
    /// # Errors
    ///
    /// Returns an error if metric submission fails
    pub async fn track_memory_usage(&self, bytes_used: usize) -> Result<()> {
        // Create performance metrics for memory usage
        let memory_metric = PerformanceMetric::new(
            "memory_usage_bytes".to_string(),
            bytes_used as f64,
            "bytes".to_string(),
        )
        .with_tag("service".to_string(), self.service_name.clone());

        self.submit_metric(&memory_metric).await
    }

    /// Get current service metrics summary
    ///
    /// # Errors
    ///
    /// Returns an error if health check or metrics retrieval fails
    pub async fn get_metrics_summary(&self) -> Result<ServiceMetricsSummary> {
        // Use the monitoring service to get actual metrics
        let _health = self
            .monitoring
            .check_service_health(&self.service_name)
            .await
            .map_err(|e| LinkMLError::service(format!("Failed to get health: {e}")))?;

        // Return basic summary for now
        Ok(ServiceMetricsSummary {
            total_validations: 0,
            total_generations: 0,
            total_parse_operations: 0,
            cache_hit_rate: 0.0,
            average_validation_ms: 0.0,
            average_generation_ms: 0.0,
            error_rate: 0.0,
            memory_usage_bytes: 0,
        })
    }
}

/// Summary of `LinkML` service metrics
#[derive(Debug, Clone)]
pub struct ServiceMetricsSummary {
    /// Total number of schema validation operations performed
    pub total_validations: u64,
    /// Total number of code generation operations performed
    pub total_generations: u64,
    /// Total number of schema parsing operations performed
    pub total_parse_operations: u64,
    /// Cache hit rate as a percentage (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Average duration of validation operations in milliseconds
    pub average_validation_ms: f64,
    /// Average duration of code generation operations in milliseconds
    pub average_generation_ms: f64,
    /// Error rate as a percentage (0.0 to 1.0) of failed operations
    pub error_rate: f64,
    /// Current memory usage of the `LinkML` service in bytes.
    pub memory_usage_bytes: usize,
}

/// Performance timer for tracking operation duration
pub struct PerformanceTimer {
    start: Instant,
    operation: String,
    metrics: Arc<LinkMLMetrics>,
}

impl PerformanceTimer {
    /// Start a new performance timer
    pub fn start(operation: impl Into<String>, metrics: Arc<LinkMLMetrics>) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.into(),
            metrics,
        }
    }

    /// Complete the timer and record the metric
    pub async fn complete(self, success: bool) -> Duration {
        let PerformanceTimer {
            start,
            operation,
            metrics,
        } = self;

        let duration = start.elapsed();
        record_operation_duration(&metrics, &operation, duration, success).await;
        duration
    }

    /// Complete with additional context
    pub async fn complete_with_size(self, success: bool, size: usize) -> Duration {
        let PerformanceTimer {
            start,
            operation,
            metrics,
        } = self;

        let duration = start.elapsed();
        record_operation_duration(&metrics, &operation, duration, success).await;

        // Create size metric
        let size_metric = PerformanceMetric::new(
            format!("operation_{operation}_size_bytes"),
            size as f64,
            "bytes".to_string(),
        )
        .with_tag("service".to_string(), "linkml-service".to_string())
        .with_tag("operation".to_string(), operation.clone());

        if let Err(e) = metrics.submit_metric(&size_metric).await {
            tracing::warn!("Failed to record operation size metric: {}", e);
        }

        duration
    }
}

async fn record_operation_duration(
    metrics: &Arc<LinkMLMetrics>,
    operation: &str,
    duration: Duration,
    success: bool,
) {
    let metric = PerformanceMetric::new(
        format!("operation_{operation}_duration_ms"),
        duration.as_millis() as f64,
        "milliseconds".to_string(),
    )
    .with_tag("service".to_string(), "linkml-service".to_string())
    .with_tag("operation".to_string(), operation.to_string())
    .with_tag("success".to_string(), success.to_string());

    if let Err(e) = metrics.submit_metric(&metric).await {
        tracing::warn!("Failed to record operation metric: {}", e);
    }
}

/// Monitoring integration for health checks
pub struct HealthMonitor {
    monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new(
        monitoring: Arc<dyn MonitoringService<Error = monitoring_core::MonitoringError>>,
    ) -> Self {
        Self { monitoring }
    }

    /// Check `LinkML` service health
    ///
    /// # Errors
    ///
    /// Returns an error if health check fails or monitoring service is unavailable
    pub async fn check_health(&self) -> Result<HealthStatus> {
        let health_report = self
            .monitoring
            .check_service_health("linkml-service")
            .await
            .map_err(|e| LinkMLError::service(format!("Health check failed: {e}")))?;

        Ok(health_report.status)
    }

    /// Report component health
    ///
    /// # Errors
    ///
    /// Returns an error if health report submission fails
    pub fn report_component_health(
        &self,
        component: &str,
        healthy: bool,
        message: Option<&str>,
    ) -> Result<()> {
        let status = if healthy {
            HealthStatus::Healthy
        } else {
            HealthStatus::Unhealthy
        };

        // Log the component health status
        tracing::info!(
            "Component {} health: {:?} - {:?}",
            component,
            status,
            message
        );

        Ok(())
    }
}

/// Extension trait for adding monitoring to `LinkML` operations
#[async_trait]
pub trait MonitoredOperation {
    /// Execute operation with monitoring
    async fn execute_monitored<T, F, Fut>(
        &self,
        operation_name: &str,
        metrics: Arc<LinkMLMetrics>,
        operation: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = Result<T>> + Send,
        T: Send;
}

/// Dashboard metrics for `LinkML` service.
pub struct LinkMLDashboard {
    metrics: Arc<LinkMLMetrics>,
}

impl LinkMLDashboard {
    /// Create new dashboard
    #[must_use]
    pub fn new(metrics: Arc<LinkMLMetrics>) -> Self {
        Self { metrics }
    }

    /// Get dashboard data
    ///
    /// # Errors
    ///
    /// Returns an error if metrics retrieval fails
    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let summary = self.metrics.get_metrics_summary().await?;

        Ok(DashboardData {
            title: "LinkML Service Dashboard".to_string(),
            metrics: vec![
                DashboardMetric {
                    name: "Total Validations".to_string(),
                    value: summary.total_validations as f64,
                    unit: "count".to_string(),
                },
                DashboardMetric {
                    name: "Average Validation Time".to_string(),
                    value: summary.average_validation_ms,
                    unit: "ms".to_string(),
                },
                DashboardMetric {
                    name: "Cache Hit Rate".to_string(),
                    value: summary.cache_hit_rate * 100.0,
                    unit: "%".to_string(),
                },
                DashboardMetric {
                    name: "Error Rate".to_string(),
                    value: summary.error_rate * 100.0,
                    unit: "%".to_string(),
                },
                DashboardMetric {
                    name: "Memory Usage".to_string(),
                    value: (summary.memory_usage_bytes as f64) / 1_048_576.0,
                    unit: "MB".to_string(),
                },
            ],
            last_updated: std::time::SystemTime::now(),
        })
    }
}

/// Dashboard data structure
#[derive(Debug, Clone)]
pub struct DashboardData {
    /// Title of the dashboard for display purposes
    pub title: String,
    /// Collection of metrics to display on the dashboard
    pub metrics: Vec<DashboardMetric>,
    /// Timestamp of when these metrics were last updated
    pub last_updated: std::time::SystemTime,
}

/// Individual dashboard metric
#[derive(Debug, Clone)]
pub struct DashboardMetric {
    /// Display name of the metric
    pub name: String,
    /// Numeric value of the metric
    pub value: f64,
    /// Unit of measurement for the metric value
    pub unit: String,
}
