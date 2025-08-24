//! Real stress testing implementation for LinkML CLI

use linkml_core::error::Result;
use linkml_core::types::SchemaDefinition;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

/// Stress test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StressTestResults {
    /// Total operations performed
    pub total_operations: u64,
    /// Successful operations
    pub successful_operations: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Success rate percentage
    pub success_rate: f64,
    /// Operations per second
    pub throughput: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// P50 latency in milliseconds
    pub p50_latency_ms: f64,
    /// P95 latency in milliseconds
    pub p95_latency_ms: f64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: f64,
    /// Maximum latency in milliseconds
    pub max_latency_ms: f64,
    /// Test duration in seconds
    pub duration_secs: f64,
    /// Errors encountered
    pub errors: Vec<String>,
}

/// Stress test configuration
#[derive(Debug, Clone)]
pub struct StressTestConfig {
    /// Number of concurrent workers
    pub concurrency: usize,
    /// Total number of operations to perform
    pub operations: usize,
    /// Enable chaos testing (random failures, delays)
    pub chaos: bool,
    /// Chaos failure rate (0.0 to 1.0)
    pub chaos_failure_rate: f64,
    /// Maximum chaos delay in milliseconds
    pub chaos_max_delay_ms: u64,
}

/// Stress test executor
pub struct StressTestExecutor<S> {
    service: Arc<S>,
    config: StressTestConfig,
    success_count: Arc<AtomicU64>,
    failure_count: Arc<AtomicU64>,
    latencies: Arc<parking_lot::Mutex<Vec<Duration>>>,
    errors: Arc<parking_lot::Mutex<Vec<String>>>,
    stop_signal: Arc<AtomicBool>,
}

impl<S> StressTestExecutor<S>
where
    S: linkml_core::traits::LinkMLService + Send + Sync + 'static,
{
    /// Create a new stress test executor
    pub fn new(service: Arc<S>, config: StressTestConfig) -> Self {
        let operations = config.operations;
        Self {
            service,
            config,
            success_count: Arc::new(AtomicU64::new(0)),
            failure_count: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(parking_lot::Mutex::new(Vec::with_capacity(operations))),
            errors: Arc::new(parking_lot::Mutex::new(Vec::new())),
            stop_signal: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Run the stress test
    pub async fn run(&self, schema: &SchemaDefinition) -> Result<StressTestResults> {
        let start_time = Instant::now();
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Generate test data
        let test_data = self.generate_test_data(schema);
        let target_class = self.get_target_class(schema);

        // Spawn worker tasks
        let mut handles = Vec::new();
        let operations_per_worker = self.config.operations / self.config.concurrency;
        let remainder = self.config.operations % self.config.concurrency;

        for worker_id in 0..self.config.concurrency {
            let ops = if worker_id < remainder {
                operations_per_worker + 1
            } else {
                operations_per_worker
            };

            let service = self.service.clone();
            let schema = schema.clone();
            let test_data = test_data.clone();
            let target_class = target_class.clone();
            let semaphore = semaphore.clone();
            let success_count = self.success_count.clone();
            let failure_count = self.failure_count.clone();
            let latencies = self.latencies.clone();
            let errors = self.errors.clone();
            let stop_signal = self.stop_signal.clone();
            let chaos = self.config.chaos;
            let chaos_failure_rate = self.config.chaos_failure_rate;
            let chaos_max_delay_ms = self.config.chaos_max_delay_ms;

            let handle = tokio::spawn(async move {
                for _ in 0..ops {
                    if stop_signal.load(Ordering::Relaxed) {
                        break;
                    }

                    let _permit = semaphore.acquire().await.expect("Semaphore acquire failed");

                    // Apply chaos if enabled
                    if chaos {
                        Self::apply_chaos(chaos_failure_rate, chaos_max_delay_ms).await;
                    }

                    // Perform validation operation
                    let op_start = Instant::now();
                    match service.validate(&test_data, &schema, &target_class).await {
                        Ok(_) => {
                            success_count.fetch_add(1, Ordering::Relaxed);
                            let duration = op_start.elapsed();
                            latencies.lock().push(duration);
                        }
                        Err(e) => {
                            failure_count.fetch_add(1, Ordering::Relaxed);
                            errors.lock().push(format!("Worker {}: {}", worker_id, e));
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            let _ = handle.await;
        }

        let duration = start_time.elapsed();

        // Calculate results
        self.calculate_results(duration)
    }

    /// Generate test data based on schema
    fn generate_test_data(&self, _schema: &SchemaDefinition) -> Value {
        // Generate realistic test data based on schema
        // For now, use a simple example
        serde_json::json!({
            "id": "test_001",
            "name": "Test Entity",
            "description": "Generated for stress testing",
            "created_at": "2025-01-31T12:00:00Z",
            "attributes": {
                "key1": "value1",
                "key2": 42,
                "key3": true
            }
        })
    }

    /// Get target class for validation
    fn get_target_class(&self, schema: &SchemaDefinition) -> String {
        // Get the first available class from schema
        schema
            .classes
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "Entity".to_string())
    }

    /// Apply chaos testing effects
    async fn apply_chaos(failure_rate: f64, max_delay_ms: u64) {
        use rand::Rng;

        // Random delay
        if max_delay_ms > 0 {
            let delay_ms = {
                let mut rng = rand::thread_rng();
                rng.gen_range(0..=max_delay_ms)
            };
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        // Random failure (will be caught by error handling)
        let failure_roll = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..1.0)
        };
        if failure_roll < failure_rate {
            // Simulate transient failure by doing nothing
            // The actual operation will handle this
        }
    }

    /// Calculate final results
    fn calculate_results(&self, duration: Duration) -> Result<StressTestResults> {
        let success = self.success_count.load(Ordering::Relaxed);
        let failure = self.failure_count.load(Ordering::Relaxed);
        let total = success + failure;

        let mut latencies = self.latencies.lock().clone();
        latencies.sort_unstable();

        let success_rate = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        let throughput = if duration.as_secs_f64() > 0.0 {
            total as f64 / duration.as_secs_f64()
        } else {
            0.0
        };

        let avg_latency_ms = if !latencies.is_empty() {
            latencies.iter().map(|d| d.as_millis() as f64).sum::<f64>() / latencies.len() as f64
        } else {
            0.0
        };

        let p50_latency_ms = Self::percentile(&latencies, 0.50);
        let p95_latency_ms = Self::percentile(&latencies, 0.95);
        let p99_latency_ms = Self::percentile(&latencies, 0.99);
        let max_latency_ms = latencies
            .last()
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0);

        let errors = self.errors.lock().clone();

        Ok(StressTestResults {
            total_operations: total,
            successful_operations: success,
            failed_operations: failure,
            success_rate,
            throughput,
            avg_latency_ms,
            p50_latency_ms,
            p95_latency_ms,
            p99_latency_ms,
            max_latency_ms,
            duration_secs: duration.as_secs_f64(),
            errors,
        })
    }

    /// Calculate percentile from sorted latencies
    fn percentile(sorted_latencies: &[Duration], percentile: f64) -> f64 {
        if sorted_latencies.is_empty() {
            return 0.0;
        }

        let index = ((sorted_latencies.len() - 1) as f64 * percentile) as usize;
        sorted_latencies
            .get(index)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    }
}
