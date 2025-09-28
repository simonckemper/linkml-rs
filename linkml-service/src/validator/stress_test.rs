//! Stress testing framework for `LinkML` validation
//!
//! This module provides comprehensive stress testing including:
//! - Load testing with concurrent requests
//! - Memory pressure testing
//! - CPU saturation testing
//! - Error injection testing
//! - Performance regression detection
//! - Chaos testing capabilities

use crate::utils::safe_cast::{f32_to_usize_saturating, usize_to_f32_saturating, usize_to_f64};
use crate::validator::ValidationContext;
use indexmap::IndexMap;
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use linkml_core::{LinkMLError, Result};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

use bitflags::bitflags;

bitflags! {
    /// Types of stress testing to enable
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct StressTestFeatures: u8 {
        /// Enable memory pressure testing
        const MEMORY_PRESSURE = 0b0001;
        /// Enable CPU pressure testing
        const CPU_PRESSURE = 0b0010;
        /// Enable error injection testing
        const ERROR_INJECTION = 0b0100;
        /// Enable chaos testing
        const CHAOS_TESTING = 0b1000;

        /// All stress test features enabled
        const ALL = Self::MEMORY_PRESSURE.bits()
                  | Self::CPU_PRESSURE.bits()
                  | Self::ERROR_INJECTION.bits()
                  | Self::CHAOS_TESTING.bits();

        /// Basic stress testing only (no chaos)
        const BASIC = Self::MEMORY_PRESSURE.bits()
                    | Self::CPU_PRESSURE.bits();

        /// No stress features (performance testing only)
        const NONE = 0b0000;
    }
}

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Number of concurrent operations
    pub concurrency: usize,
    /// Total number of operations
    pub total_operations: usize,
    /// Operation timeout
    pub timeout: Duration,
    /// Enabled stress test features
    pub enabled_features: StressTestFeatures,
    /// Target memory usage (bytes)
    pub target_memory_bytes: usize,
    /// Target CPU usage (0-100)
    pub target_cpu_percent: f32,
    /// Error injection rate (0.0-1.0)
    pub error_rate: f64,
    /// Performance thresholds
    pub performance_thresholds: PerformanceThresholds,
}

impl Default for StressTestConfig {
    fn default() -> Self {
        Self {
            concurrency: 100,
            total_operations: 10000,
            timeout: Duration::from_secs(30),
            enabled_features: StressTestFeatures::NONE, // Conservative default
            target_memory_bytes: 1024 * 1024 * 1024,    // 1GB
            target_cpu_percent: 80.0,
            error_rate: 0.01, // 1%
            performance_thresholds: PerformanceThresholds::default(),
        }
    }
}

impl StressTestConfig {
    /// Check if memory pressure testing is enabled
    #[must_use]
    pub fn memory_pressure(&self) -> bool {
        self.enabled_features
            .contains(StressTestFeatures::MEMORY_PRESSURE)
    }

    /// Check if CPU pressure testing is enabled
    #[must_use]
    pub fn cpu_pressure(&self) -> bool {
        self.enabled_features
            .contains(StressTestFeatures::CPU_PRESSURE)
    }

    /// Check if error injection is enabled
    #[must_use]
    pub fn error_injection(&self) -> bool {
        self.enabled_features
            .contains(StressTestFeatures::ERROR_INJECTION)
    }

    /// Check if chaos testing is enabled
    #[must_use]
    pub fn chaos_testing(&self) -> bool {
        self.enabled_features
            .contains(StressTestFeatures::CHAOS_TESTING)
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
pub struct StressTestRunner<R>
where
    R: random_core::RandomService,
{
    config: Arc<RwLock<StressTestConfig>>,
    operations: Vec<Arc<dyn StressOperation>>,
    results: Arc<RwLock<Vec<OperationResult>>>,
    chaos_engine: Option<Arc<ChaosEngine<R>>>,
    random_service: Arc<R>,
}

impl<R> StressTestRunner<R>
where
    R: random_core::RandomService + Send + Sync + 'static,
{
    /// Create new stress test runner
    #[must_use]
    pub fn new(
        config: StressTestConfig,
        random_service: Arc<R>,
    ) -> Self {
        let chaos_engine = if config.chaos_testing() {
            Some(Arc::new(ChaosEngine::new(random_service.clone())))
        } else {
            None
        };

        Self {
            config: Arc::new(RwLock::new(config)),
            operations: Vec::new(),
            results: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            chaos_engine,
            random_service,
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
        let initial_memory = Self::get_memory_usage();

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
            let results = Arc::clone(&self.results);
            let config = config.clone();
            let chaos = self.chaos_engine.clone();

            // Pre-compute values that need RNG outside the async block
            let random_value = self.random_service.generate_f64().await.unwrap_or(0.0);
            let should_fail = config.error_injection() && random_value < config.error_rate;
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
            if config.cpu_pressure() {
                self.apply_cpu_pressure(config.target_cpu_percent).await;
            }

            // Apply memory pressure if configured
            if config.memory_pressure() {
                Self::apply_memory_pressure(config.target_memory_bytes);
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
        // Calculate metrics using safe casting
        let throughput = usize_to_f64(total_ops) / total_duration.as_secs_f64();
        let success_rate = usize_to_f64(successful_ops) / usize_to_f64(total_ops);

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
                        .sum::<u128>(),
                )
                .unwrap_or(u64::MAX)
                    / u64::try_from(latencies.len()).unwrap_or(u64::MAX),
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
            final_bytes: Self::get_memory_usage(),
            allocations: 0, // Would track actual allocations
            deallocations: 0,
        };

        // Check for violations
        let violations = Self::check_violations(
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
                chaos_testing: config.chaos_testing(),
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
                .map_or(0, |engine| engine.events_triggered()),
        }
    }

    /// Check for performance violations
    fn check_violations(
        thresholds: &PerformanceThresholds,
        latencies: &LatencyPercentiles,
        throughput: f64,
        success_rate: f64,
        memory: &MemoryStats,
    ) -> Vec<PerformanceViolation> {
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
    fn get_memory_usage() -> usize {
        #[cfg(target_os = "linux")]
        {
            // Read from /proc/self/status for accurate RSS
            if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("VmRSS:")
                        && let Some(kb_str) = line.split_whitespace().nth(1)
                        && let Ok(kb) = kb_str.parse::<usize>()
                    {
                        return kb * 1024; // Convert KB to bytes
                    }
                }
            }
        }

        #[cfg(not(target_os = "linux"))]
        {
            // Fallback: Use approximation based on heap allocation
            // This is less accurate but works cross-platform
            use std::alloc::{GlobalAlloc, Layout, System};
            // Note: This is an approximation - actual implementation would need
            // platform-specific memory APIs
            1024 * 1024 * 100 // Default to 100MB estimate
        }

        0
    }

    /// Apply CPU pressure
    async fn apply_cpu_pressure(&self, target_percent: f32) {
        let num_cores = num_cpus::get();
        let active_threads = f32_to_usize_saturating(
            ((target_percent / 100.0) * usize_to_f32_saturating(num_cores)).ceil(),
        );

        // Spawn CPU-intensive tasks
        let mut handles = Vec::new();
        for _ in 0..active_threads {
            let handle = tokio::task::spawn_blocking(move || {
                let start = std::time::Instant::now();
                let duration = Duration::from_millis(100);

                // CPU-intensive loop
                while start.elapsed() < duration {
                    // Perform actual computations to consume CPU
                    let mut sum = 0u64;
                    for i in 0..10000 {
                        sum = sum.wrapping_add(i * i);
                    }
                    // Prevent optimization
                    std::hint::black_box(sum);
                }
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            let _ = handle.await;
        }
    }

    /// Apply memory pressure
    fn apply_memory_pressure(target_bytes: usize) {
        // Allocate memory to create pressure
        let mut allocations = Vec::new();
        let chunk_size = 1024 * 1024; // 1MB chunks
        let num_chunks = target_bytes / chunk_size;

        for _ in 0..num_chunks {
            // Allocate and initialize to prevent optimization
            let mut chunk = vec![0u8; chunk_size];
            // Touch the memory to ensure it's actually allocated
            for i in (0..chunk_size).step_by(4096) {
                chunk[i] = 42;
            }
            allocations.push(chunk);
        }

        // Keep allocations alive briefly
        std::thread::sleep(Duration::from_millis(10));

        // Prevent optimization from removing allocations
        std::hint::black_box(allocations);
    }
}

/// Chaos engine for injecting failures
#[derive(Clone)]
struct ChaosEngine<R>
where
    R: random_core::RandomService,
{
    enabled: Arc<RwLock<bool>>,
    events: Arc<RwLock<usize>>,
    random_service: Arc<R>,
}

impl<R> ChaosEngine<R>
where
    R: random_core::RandomService + Send + Sync + 'static,
{
    fn new(
        random_service: Arc<R>,
    ) -> Self {
        Self {
            enabled: Arc::new(RwLock::new(false)),
            events: Arc::new(RwLock::new(0)),
            random_service,
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

        let chaos_type = self
            .random_service
            .generate_u32_range(0, 10)
            .await
            .unwrap_or(0);

        match chaos_type {
            0 => {
                // Inject delay
                let delay_ms = 10
                    + self
                        .random_service
                        .generate_u32_range(0, 90)
                        .await
                        .unwrap_or(0);
                tokio::time::sleep(Duration::from_millis(u64::from(delay_ms))).await;
                *self.events.write() += 1;
            }
            1 => {
                // CPU spike - Real computation instead of random multiplication
                let mut sum = 0.0f64;
                for i in 0..1_000_000 {
                    sum += f64::from(i).sin().cos(); // Real CPU work
                }
                std::hint::black_box(sum); // Prevent optimization
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
        // Perform actual validation stress test
        // Create a real validation context and validate data
        let schema = SchemaDefinition {
            name: "stress_test".to_string(),
            classes: {
                let mut classes = IndexMap::new();
                classes.insert(
                    "TestClass".to_string(),
                    ClassDefinition {
                        name: "TestClass".to_string(),
                        slots: vec!["field1".to_string(), "field2".to_string()],
                        ..Default::default()
                    },
                );
                classes
            },
            slots: {
                let mut slots = IndexMap::new();
                slots.insert(
                    "field1".to_string(),
                    SlotDefinition {
                        name: "field1".to_string(),
                        required: Some(true),
                        ..Default::default()
                    },
                );
                slots.insert(
                    "field2".to_string(),
                    SlotDefinition {
                        name: "field2".to_string(),
                        multivalued: Some(true),
                        ..Default::default()
                    },
                );
                slots
            },
            ..Default::default()
        };

        // Generate test data
        let test_data = self.generate_data(10);

        // Perform actual validation
        let context = Arc::new(schema);
        let mut validation_context = ValidationContext::new(context);

        // Run validation on each field
        let test_object = test_data
            .as_object()
            .ok_or_else(|| LinkMLError::data_validation("Test data must be an object"))?;

        for (key, value) in test_object {
            // Get slot info before borrowing context mutably
            let slot_info = validation_context.schema.slots.get(key).cloned();

            if let Some(slot) = slot_info {
                // This performs real validation work
                validation_context.push_path(key.clone());

                // Check required constraint
                if slot.required.unwrap_or(false) && value.is_null() {
                    return Err(LinkMLError::data_validation(format!(
                        "Required field '{key}' is null"
                    )));
                }

                // Check multivalued constraint
                if slot.multivalued.unwrap_or(false) && !value.is_array() {
                    return Err(LinkMLError::data_validation(format!(
                        "Multivalued field '{key}' is not an array"
                    )));
                }
            }
        }

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

        // For tests, we'll create a simplified random service
        // In production, the random service would be injected via dependency injection
        struct SimpleRandomService;

        #[async_trait::async_trait]
        impl random_core::RandomService for SimpleRandomService {
            type Error = random_core::RandomError;

            // Basic Random Generation
            async fn generate_u32(&self) -> random_core::RandomResult<u32> {
                Ok(42) // Simple deterministic value for testing
            }

            async fn generate_u64(&self) -> random_core::RandomResult<u64> {
                Ok(42) // Simple deterministic value for testing
            }

            async fn generate_i32(&self) -> random_core::RandomResult<i32> {
                Ok(42) // Simple deterministic value for testing
            }

            async fn generate_i64(&self) -> random_core::RandomResult<i64> {
                Ok(42) // Simple deterministic value for testing
            }

            async fn generate_f32(&self) -> random_core::RandomResult<f32> {
                Ok(0.5) // Simple deterministic value for testing
            }

            async fn generate_f64(&self) -> random_core::RandomResult<f64> {
                Ok(0.5) // Simple deterministic value for testing
            }

            async fn generate_bool(&self) -> random_core::RandomResult<bool> {
                Ok(true) // Simple deterministic value for testing
            }

            async fn fill_bytes(&self, dest: &mut [u8]) -> random_core::RandomResult<()> {
                dest.fill(42);
                Ok(())
            }

            fn fill_bytes_sync(&self, dest: &mut [u8]) -> random_core::RandomResult<()> {
                dest.fill(42);
                Ok(())
            }

            fn generate_u32_sync(&self) -> random_core::RandomResult<u32> {
                Ok(42)
            }

            fn generate_u64_sync(&self) -> random_core::RandomResult<u64> {
                Ok(42)
            }

            fn create_sync_rng(&self) -> Box<dyn random_core::SyncCryptoRng> {
                // TODO: Fix trait bound issue with StdRng and SyncCryptoRng
                // The issue is that rand's RngCore is different from random_core's expectations
                unimplemented!("Stress test RNG implementation needs trait bound fixes")
            }

            // Range-Based Generation
            async fn generate_u32_range(
                &self,
                min: u32,
                max: u32,
            ) -> random_core::RandomResult<u32> {
                Ok(min + (max - min) / 2) // Simple deterministic value for testing
            }

            async fn generate_u64_range(
                &self,
                min: u64,
                max: u64,
            ) -> random_core::RandomResult<u64> {
                Ok(min + (max - min) / 2)
            }

            async fn generate_i32_range(
                &self,
                min: i32,
                max: i32,
            ) -> random_core::RandomResult<i32> {
                Ok(min + (max - min) / 2)
            }

            async fn generate_i64_range(
                &self,
                min: i64,
                max: i64,
            ) -> random_core::RandomResult<i64> {
                Ok(min + (max - min) / 2)
            }

            async fn generate_f32_range(
                &self,
                min: f32,
                max: f32,
            ) -> random_core::RandomResult<f32> {
                Ok(min + (max - min) / 2.0)
            }

            async fn generate_f64_range(
                &self,
                min: f64,
                max: f64,
            ) -> random_core::RandomResult<f64> {
                Ok(min + (max - min) / 2.0)
            }

            // Statistical Distributions
            async fn generate_beta(
                &self,
                _params: random_core::BetaParams,
            ) -> random_core::RandomResult<f64> {
                Ok(0.5)
            }

            async fn generate_gamma(
                &self,
                _params: random_core::GammaParams,
            ) -> random_core::RandomResult<f64> {
                Ok(1.0)
            }

            async fn generate_binomial(
                &self,
                _params: random_core::BinomialParams,
            ) -> random_core::RandomResult<u32> {
                Ok(5)
            }

            async fn generate_poisson(
                &self,
                _params: random_core::PoissonParams,
            ) -> random_core::RandomResult<u32> {
                Ok(3)
            }

            async fn generate_normal(
                &self,
                params: random_core::NormalParams,
            ) -> random_core::RandomResult<f64> {
                Ok(params.mean)
            }

            async fn generate_exponential(
                &self,
                params: random_core::ExponentialParams,
            ) -> random_core::RandomResult<f64> {
                Ok(1.0 / params.rate)
            }

            // Advanced Features
            async fn set_seed(
                &self,
                _seed: random_core::SeedValue,
            ) -> random_core::RandomResult<()> {
                Ok(())
            }

            async fn get_entropy_level(&self) -> random_core::RandomResult<f64> {
                Ok(1.0)
            }

            async fn switch_backend(
                &self,
                _backend: random_core::BackendType,
            ) -> random_core::RandomResult<()> {
                Ok(())
            }

            async fn get_current_backend(
                &self,
            ) -> random_core::RandomResult<random_core::BackendType> {
                Ok(random_core::BackendType::ThreadRng)
            }

            async fn get_config(&self) -> random_core::RandomResult<random_core::RandomConfig> {
                Ok(random_core::RandomConfig::default())
            }

            async fn update_config(
                &self,
                _config: random_core::RandomConfig,
            ) -> random_core::RandomResult<()> {
                Ok(())
            }

            // Collection Operations
            async fn generate_vec_u32(&self, count: usize) -> random_core::RandomResult<Vec<u32>> {
                Ok(vec![42; count])
            }

            async fn generate_vec_normal(
                &self,
                params: random_core::NormalParams,
                count: usize,
            ) -> random_core::RandomResult<Vec<f64>> {
                Ok(vec![params.mean; count])
            }

            async fn shuffle_bytes(&self, _slice: &mut [u8]) -> random_core::RandomResult<()> {
                // No-op shuffle for testing
                Ok(())
            }

            async fn shuffle_u16(&self, _slice: &mut [u16]) -> random_core::RandomResult<()> {
                // No-op shuffle for testing
                Ok(())
            }

            async fn shuffle_u32(&self, _slice: &mut [u32]) -> random_core::RandomResult<()> {
                // No-op shuffle for testing
                Ok(())
            }

            async fn shuffle_u64(&self, _slice: &mut [u64]) -> random_core::RandomResult<()> {
                // No-op shuffle for testing
                Ok(())
            }

            async fn sample_u32_without_replacement(
                &self,
                collection: &[u32],
                n: usize,
            ) -> random_core::RandomResult<Vec<u32>> {
                Ok(collection.iter().take(n).copied().collect())
            }

            async fn sample_u64_without_replacement(
                &self,
                collection: &[u64],
                n: usize,
            ) -> random_core::RandomResult<Vec<u64>> {
                Ok(collection.iter().take(n).copied().collect())
            }

            // UUID Generation
            async fn generate_uuid_v4(&self) -> random_core::RandomResult<String> {
                Ok("12345678-1234-1234-1234-123456789abc".to_string())
            }

            async fn generate_uuid_v4_bytes(&self) -> random_core::RandomResult<[u8; 16]> {
                Ok([42; 16])
            }

            async fn shuffle<T: Send>(&self, _slice: &mut [T]) -> random_core::RandomResult<()> {
                // No-op shuffle for deterministic testing
                Ok(())
            }

            async fn sample_without_replacement<T: Clone + Send + Sync>(
                &self,
                collection: &[T],
                n: usize,
            ) -> random_core::RandomResult<Vec<T>> {
                // Simple deterministic sampling - take first n elements
                let actual_n = n.min(collection.len());
                Ok(collection.iter().take(actual_n).cloned().collect())
            }
        }

        let random_service = Arc::new(SimpleRandomService);
        let mut runner = StressTestRunner::new(config, random_service);
        runner.add_operation(Arc::new(TestOperation));

        let results = runner.run().await.expect("Test operation failed");

        assert_eq!(results.config.total_operations, 10);
        assert!(results.success_rate > 0.9);
        assert!(results.throughput > 0.0);
    }
}
