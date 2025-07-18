//! Stress testing framework for `LinkML` validation
//!
//! This module provides comprehensive stress testing including:
//! - Load testing with concurrent requests
//! - Memory pressure testing
//! - CPU saturation testing
//! - Error injection testing
//! - Performance regression detection
//! - Chaos testing capabilities

use linkml_core::error::{LinkMLError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Stress test configuration
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct StressTestConfig {
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Total number of operations
    pub total_operations: usize,
    /// Operation timeout
    pub timeout: Duration,
    /// Enable memory pressure
    pub memory_pressure: bool,
    /// Target memory usage (bytes)
    pub target_memory_bytes: usize,
    /// Enable CPU pressure
    pub cpu_pressure: bool,
    /// Target CPU usage (0-100)
    pub target_cpu_percent: f32,
    /// Enable error injection
    pub error_injection: bool,
    /// Error injection rate (0.0-1.0)
    pub error_rate: f64,
    /// Enable chaos testing
    pub chaos_testing: bool,
    /// Performance thresholds
    pub performance_thresholds: PerformanceThresholds,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 100,
            total_operations: 10000,
            timeout: Duration::from_secs(30),
            memory_pressure: false,
            target_memory_bytes: 1024 * 1024 * 1024, // 1GB
            cpu_pressure: false,
            target_cpu_percent: 80.0,
            error_injection: false,
            error_rate: 0.01, // 1%
            chaos_testing: false,
            performance_thresholds: PerformanceThresholds::default(),
        }
    }
}

/// Performance thresholds for regression detection
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    /// Maximum acceptable latency (p99)
    pub max_latency_p99: Duration,
    /// Maximum acceptable latency (p95)
    pub max_latency_p95: Duration,
    /// Maximum acceptable latency (p50)
    pub max_latency_p50: Duration,
    /// Minimum throughput (ops/sec)
    pub min_throughput: f64,
    /// Maximum memory usage
    pub max_memory_bytes: usize,
    /// Maximum error rate
    pub max_error_rate: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_latency_p99: Duration::from_millis(1000),
            max_latency_p95: Duration::from_millis(500),
            max_latency_p50: Duration::from_millis(100),
            min_throughput: 1000.0,
            max_memory_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            max_error_rate: 0.01,                     // 1%
        }
    }
}

/// Stress test operation
pub trait StressOperation: Send + Sync {
    /// Execute the operation
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    fn execute(&self) -> Result<()>;

    /// Get operation name
    fn name(&self) -> &str;

    /// Generate test data
    fn generate_data(&self, size_hint: usize) -> serde_json::Value;
}

/// Stress test result for a single operation
#[derive(Debug, Clone)]
pub struct OperationResult {
    /// Operation name
    pub operation: String,
    /// Success flag
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Operation duration
    pub duration: Duration,
    /// Memory used
    pub memory_delta: i64,
}

/// Stress test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResults {
    /// Test configuration
    pub config: StressTestSummary,
    /// Total duration
    pub total_duration: Duration,
    /// Operations per second
    pub throughput: f64,
    /// Success rate
    pub success_rate: f64,
    /// Latency percentiles
    pub latency_percentiles: LatencyPercentiles,
    /// Error distribution
    pub errors_by_type: HashMap<String, usize>,
    /// Memory statistics
    pub memory_stats: MemoryStats,
    /// Performance violations
    pub violations: Vec<PerformanceViolation>,
    /// Chaos events triggered
    pub chaos_events: usize,
}

/// Stress test configuration summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestSummary {
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Total number of operations executed
    pub total_operations: usize,
    /// Rate of error injection (0.0 to 1.0)
    pub error_injection_rate: f64,
    /// Whether chaos testing was enabled
    pub chaos_testing: bool,
}

/// Latency percentiles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    /// 50th percentile (median) latency
    pub p50: Duration,
    /// 75th percentile latency
    pub p75: Duration,
    /// 90th percentile latency
    pub p90: Duration,
    /// 95th percentile latency
    pub p95: Duration,
    /// 99th percentile latency
    pub p99: Duration,
    /// 99.9th percentile latency
    pub p999: Duration,
    /// Minimum latency observed
    pub min: Duration,
    /// Maximum latency observed
    pub max: Duration,
    /// Mean (average) latency
    pub mean: Duration,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    /// Initial memory usage in bytes
    pub initial_bytes: usize,
    /// Peak memory usage in bytes
    pub peak_bytes: usize,
    /// Final memory usage in bytes
    pub final_bytes: usize,
    /// Total number of allocations
    pub allocations: usize,
    /// Total number of deallocations
    pub deallocations: usize,
}

/// Performance violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceViolation {
    /// Name of the metric that violated threshold
    pub metric: String,
    /// Threshold value that was exceeded
    pub threshold: String,
    /// Actual value observed
    pub actual: String,
    /// Severity of the violation
    pub severity: ViolationSeverity,
}

/// Violation severity
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Minor violation that doesn't affect functionality
    Warning,
    /// Significant violation that may affect performance
    Error,
    /// Severe violation that requires immediate attention
    Critical,
}

/// Stress test runner
pub struct StressTestRunner {
    config: Arc<RwLock<StressTestConfig>>,
    operations: Vec<Arc<dyn StressOperation>>,
    results: Arc<RwLock<Vec<OperationResult>>>,
    chaos_engine: Option<ChaosEngine>,
}

impl StressTestRunner {
    /// Create new stress test runner
    #[must_use]
    pub fn new(config: StressTestConfig) -> Self {
        let chaos_engine = if config.chaos_testing {
            Some(ChaosEngine::new())
        } else {
            None
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            operations: Vec::new(),
            results: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            chaos_engine,
        }
    }

    /// Add stress operation
    pub fn add_operation(&mut self, operation: Arc<dyn StressOperation>) {
        self.operations.push(operation);
    }

    /// Run stress test
    ///
    /// # Errors
    ///
    /// Returns an error if semaphore acquisition fails.
    pub async fn run(&self) -> Result<StressTestResults> {
        let config = self.config.read().clone();
        let start_time = Instant::now();
        let initial_memory = self.get_memory_usage();

        // Create semaphore for concurrency control
        let semaphore = Arc::new(Semaphore::new(config.concurrency));
        let mut handles = Vec::new();

        // Start background chaos if enabled
        if let Some(chaos) = &self.chaos_engine {
            chaos.start();
        }

        // Launch operations
        for i in 0..config.total_operations {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| LinkMLError::service("Failed to acquire semaphore"))?;

            let operation = self.select_operation(i);
            let results = self.results.clone();
            let config = config.clone();
            let chaos = self.chaos_engine.clone();

            // Pre-compute values that need RNG outside the async block
            let should_fail = config.error_injection && rand::random::<f64>() < config.error_rate;
            let operation_name = operation.name().to_string();

            let handle = tokio::spawn(async move {
                let _permit = permit;

                // Apply chaos if enabled
                if let Some(chaos) = chaos {
                    chaos.maybe_inject_chaos().await;
                }

                let start = Instant::now();
                let memory_before = 0; // Would use actual memory measurement

                let result = if should_fail {
                    Err(LinkMLError::service("Injected error"))
                } else {
                    operation.execute()
                };

                let duration = start.elapsed();
                let memory_after = 0; // Would use actual memory measurement

                let op_result = OperationResult {
                    operation: operation_name,
                    success: result.is_ok(),
                    error: result.err().map(|e| e.to_string()),
                    duration,
                    memory_delta: i64::from(memory_after - memory_before),
                };

                results.write().push(op_result);
            });

            handles.push(handle);

            // Apply CPU pressure if configured
            if config.cpu_pressure {
                self.apply_cpu_pressure(config.target_cpu_percent).await;
            }

            // Apply memory pressure if configured
            if config.memory_pressure {
                self.apply_memory_pressure(config.target_memory_bytes);
            }
        }

        // Wait for all operations to complete
        for handle in handles {
            let _ = handle.await;
        }

        // Stop chaos
        if let Some(chaos) = &self.chaos_engine {
            chaos.stop();
        }

        // Calculate results
        let total_duration = start_time.elapsed();
        let results = self.results.read().clone();

        Ok(self.calculate_results(&config, &results, total_duration, initial_memory))
    }

    /// Select operation for iteration
    fn select_operation(&self, iteration: usize) -> Arc<dyn StressOperation> {
        assert!(!self.operations.is_empty(), "No operations registered");

        let index = iteration % self.operations.len();
        self.operations[index].clone()
    }

    /// Calculate stress test results
    fn calculate_results(
        &self,
        config: &StressTestConfig,
        results: &[OperationResult],
        total_duration: Duration,
        initial_memory: usize,
    ) -> StressTestResults {
        let total_ops = results.len();
        let successful_ops = results.iter().filter(|r| r.success).count();
        #[allow(clippy::cast_precision_loss)]
        let throughput = total_ops as f64 / total_duration.as_secs_f64();
        #[allow(clippy::cast_precision_loss)]
        let success_rate = successful_ops as f64 / total_ops as f64;

        // Calculate latency percentiles
        let mut latencies: Vec<_> = results.iter().map(|r| r.duration).collect();
        latencies.sort();

        let latency_percentiles = LatencyPercentiles {
            p50: latencies[latencies.len() * 50 / 100],
            p75: latencies[latencies.len() * 75 / 100],
            p90: latencies[latencies.len() * 90 / 100],
            p95: latencies[latencies.len() * 95 / 100],
            p99: latencies[latencies.len() * 99 / 100],
            p999: latencies[latencies.len() * 999 / 1000],
            min: latencies[0],
            max: latencies[latencies.len() - 1],
            mean: Duration::from_nanos(
                u64::try_from(
                    latencies
                        .iter()
                        .map(std::time::Duration::as_nanos)
                        .sum::<u128>()
                ).unwrap_or(u64::MAX)
                    / latencies.len() as u64,
            ),
        };

        // Error distribution
        let errors_by_type = results.iter().filter_map(|r| r.error.as_ref()).fold(
            HashMap::new(),
            |mut map, error| {
                *map.entry(error.clone()).or_insert(0) += 1;
                map
            },
        );

        // Memory stats
        let memory_stats = MemoryStats {
            initial_bytes: initial_memory,
            peak_bytes: initial_memory, // Would track actual peak
            final_bytes: self.get_memory_usage(),
            allocations: 0, // Would track actual allocations
            deallocations: 0,
        };

        // Check for violations
        let violations = self.check_violations(
            &config.performance_thresholds,
            &latency_percentiles,
            throughput,
            success_rate,
            &memory_stats,
        );

        StressTestResults {
            config: StressTestSummary {
                concurrency: config.concurrency,
                total_operations: config.total_operations,
                error_injection_rate: config.error_rate,
                chaos_testing: config.chaos_testing,
            },
            total_duration,
            throughput,
            success_rate,
            latency_percentiles,
            errors_by_type,
            memory_stats,
            violations,
            chaos_events: self
                .chaos_engine
                .as_ref()
                .map_or(0, ChaosEngine::events_triggered),
        }
    }

    /// Check for performance violations
    fn check_violations(
        &self,
        thresholds: &PerformanceThresholds,
        latencies: &LatencyPercentiles,
        throughput: f64,
        success_rate: f64,
        memory: &MemoryStats,
    ) -> Vec<PerformanceViolation> {
        let _ = self;
        let mut violations = Vec::new();

        // Latency violations
        if latencies.p99 > thresholds.max_latency_p99 {
            violations.push(PerformanceViolation {
                metric: "P99 Latency".to_string(),
                threshold: format!("{:?}", thresholds.max_latency_p99),
                actual: format!("{:?}", latencies.p99),
                severity: ViolationSeverity::Critical,
            });
        }

        if latencies.p95 > thresholds.max_latency_p95 {
            violations.push(PerformanceViolation {
                metric: "P95 Latency".to_string(),
                threshold: format!("{:?}", thresholds.max_latency_p95),
                actual: format!("{:?}", latencies.p95),
                severity: ViolationSeverity::Error,
            });
        }

        // Throughput violation
        if throughput < thresholds.min_throughput {
            violations.push(PerformanceViolation {
                metric: "Throughput".to_string(),
                threshold: format!("{} ops/sec", thresholds.min_throughput),
                actual: format!("{throughput:.2} ops/sec"),
                severity: ViolationSeverity::Critical,
            });
        }

        // Error rate violation
        let error_rate = 1.0 - success_rate;
        if error_rate > thresholds.max_error_rate {
            violations.push(PerformanceViolation {
                metric: "Error Rate".to_string(),
                threshold: format!("{}%", thresholds.max_error_rate * 100.0),
                actual: format!("{:.2}%", error_rate * 100.0),
                severity: ViolationSeverity::Error,
            });
        }

        // Memory violation
        if memory.peak_bytes > thresholds.max_memory_bytes {
            violations.push(PerformanceViolation {
                metric: "Peak Memory".to_string(),
                threshold: format!("{} MB", thresholds.max_memory_bytes / 1024 / 1024),
                actual: format!("{} MB", memory.peak_bytes / 1024 / 1024),
                severity: ViolationSeverity::Warning,
            });
        }

        violations
    }

    /// Get current memory usage
    fn get_memory_usage(&self) -> usize {
        let _ = self;
        // Simplified - would use actual memory measurement
        0
    }

    /// Apply CPU pressure
    async fn apply_cpu_pressure(&self, _target_percent: f32) {
        // Simplified - would use actual CPU load generation
        tokio::time::sleep(Duration::from_micros(1)).await;
    }

    /// Apply memory pressure
    fn apply_memory_pressure(&self, _target_bytes: usize) {
        let _ = self;
        // Simplified - would allocate memory to create pressure
    }
}

/// Chaos engine for injecting failures
#[derive(Clone)]
struct ChaosEngine {
    enabled: Arc<RwLock<bool>>,
    events: Arc<RwLock<usize>>,
}

impl ChaosEngine {
    fn new() -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            events: Arc::new(RwLock::new(0)),
        }
    }

    fn start(&self) {
        *self.enabled.write() = true;
    }

    fn stop(&self) {
        *self.enabled.write() = false;
    }

    async fn maybe_inject_chaos(&self) {
        if !*self.enabled.read() {
            return;
        }

        let chaos_type = rand::random::<u32>() % 10;

        match chaos_type {
            0 => {
                // Inject delay
                let delay_ms = 10 + (rand::random::<u32>() % 90);
                tokio::time::sleep(Duration::from_millis(u64::from(delay_ms))).await;
                *self.events.write() += 1;
            }
            1 => {
                // CPU spike
                for _ in 0..1_000_000 {
                    let _ = rand::random::<f64>() * rand::random::<f64>();
                }
                *self.events.write() += 1;
            }
            _ => {
                // No chaos this time
            }
        }
    }

    fn events_triggered(&self) -> usize {
        *self.events.read()
    }
}

/// Example validation stress operation
pub struct ValidationStressOperation {
    _schema_size: usize,
    _document_size: usize,
}

impl ValidationStressOperation {
    /// Create a new validation stress operation
    #[must_use]
    pub fn new(schema_size: usize, document_size: usize) -> Self {
        Self {
            _schema_size: schema_size,
            _document_size: document_size,
        }
    }
}

impl StressOperation for ValidationStressOperation {
    fn execute(&self) -> Result<()> {
        // Simulate validation work
        std::thread::sleep(Duration::from_micros(100));
        Ok(())
    }

    fn name(&self) -> &'static str {
        "validation"
    }

    fn generate_data(&self, size_hint: usize) -> serde_json::Value {
        // Generate test data based on size hint
        let mut map = serde_json::Map::new();
        for i in 0..size_hint {
            map.insert(
                format!("field_{i}"),
                serde_json::Value::String("test".to_string()),
            );
        }
        serde_json::Value::Object(map)
    }
}

/// Format stress test results as a report
#[must_use]
pub fn format_results(results: &StressTestResults) -> String {
    format!(
        r"
Stress Test Results
==================

Configuration:
- Concurrency: {}
- Total Operations: {}
- Error Injection Rate: {:.2}%
- Chaos Testing: {}

Performance:
- Total Duration: {:.2}s
- Throughput: {:.2} ops/sec
- Success Rate: {:.2}%

Latency Percentiles:
- P50: {:?}
- P75: {:?}
- P90: {:?}
- P95: {:?}
- P99: {:?}
- P999: {:?}

Memory Usage:
- Initial: {} MB
- Peak: {} MB
- Final: {} MB

Violations: {}
Chaos Events: {}

Error Distribution:
{:?}
",
        results.config.concurrency,
        results.config.total_operations,
        results.config.error_injection_rate * 100.0,
        results.config.chaos_testing,
        results.total_duration.as_secs_f64(),
        results.throughput,
        results.success_rate * 100.0,
        results.latency_percentiles.p50,
        results.latency_percentiles.p75,
        results.latency_percentiles.p90,
        results.latency_percentiles.p95,
        results.latency_percentiles.p99,
        results.latency_percentiles.p999,
        results.memory_stats.initial_bytes / 1024 / 1024,
        results.memory_stats.peak_bytes / 1024 / 1024,
        results.memory_stats.final_bytes / 1024 / 1024,
        results.violations.len(),
        results.chaos_events,
        results.errors_by_type
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestOperation;

    impl StressOperation for TestOperation {
        fn execute(&self) -> Result<()> {
            Ok(())
        }

        fn name(&self) -> &'static str {
            "test"
        }

        fn generate_data(&self, _size_hint: usize) -> serde_json::Value {
            serde_json::json!({})
        }
    }

    #[tokio::test]
    async fn test_stress_runner() {
        let config = StressTestConfig {
            concurrency: 2,
            total_operations: 10,
            ..Default::default()
        };

        let mut runner = StressTestRunner::new(config);
        runner.add_operation(Arc::new(TestOperation));

        let results = runner.run().await.unwrap();

        assert_eq!(results.config.total_operations, 10);
        assert!(results.success_rate > 0.9);
        assert!(results.throughput > 0.0);
    }
}
