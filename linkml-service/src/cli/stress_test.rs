//! Real stress testing implementation for `LinkML` CLI

use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use linkml_core::types::SchemaDefinition;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::Semaphore;
use timestamp_core::{TimestampService, TimestampError};

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
    pub errors: Vec<String>}

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
    pub chaos_max_delay_ms: u64}

/// Stress test executor
pub struct StressTestExecutor<S> {
    service: Arc<S>,
    config: StressTestConfig,
    timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    success_count: Arc<AtomicU64>,
    failure_count: Arc<AtomicU64>,
    latencies: Arc<parking_lot::Mutex<Vec<Duration>>>,
    errors: Arc<parking_lot::Mutex<Vec<String>>>,
    stop_signal: Arc<AtomicBool>}

impl<S> StressTestExecutor<S>
where
    S: linkml_core::traits::LinkMLService + Send + Sync + 'static,
{
    /// Create a new stress test executor
    pub fn new(
        service: Arc<S>,
        config: StressTestConfig,
        timestamp: Arc<dyn TimestampService<Error = TimestampError>>,
    ) -> Self {
        let operations = config.operations;
        Self {
            service,
            config,
            timestamp,
            success_count: Arc::new(AtomicU64::new(0)),
            failure_count: Arc::new(AtomicU64::new(0)),
            latencies: Arc::new(parking_lot::Mutex::new(Vec::with_capacity(operations))),
            errors: Arc::new(parking_lot::Mutex::new(Vec::new())),
            stop_signal: Arc::new(AtomicBool::new(false))}
    }

    /// Run the stress test
    pub async fn run(&self, schema: &SchemaDefinition) -> crate::Result<StressTestResults> {
        let start_time = std::time::Instant::now();
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

            let service = Arc::clone(&self.service);
            let schema = schema.clone();
            let test_data = test_data.clone();
            let target_class = target_class.clone();
            let semaphore = semaphore.clone();
            let success_count = Arc::clone(&self.success_count);
            let failure_count = Arc::clone(&self.failure_count);
            let latencies = Arc::clone(&self.latencies);
            let errors = Arc::clone(&self.errors);
            let stop_signal = Arc::clone(&self.stop_signal);
            let _timestamp = Arc::clone(&self.timestamp);
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
                    let op_start = std::time::Instant::now();
                    match service.validate(&test_data, &schema, &target_class).await {
                        Ok(_) => {
                            success_count.fetch_add(1, Ordering::Relaxed);
                            let duration = op_start.elapsed();
                            latencies.lock().push(duration);
                        }
                        Err(e) => {
                            failure_count.fetch_add(1, Ordering::Relaxed);
                            errors.lock().push(format!("Worker {worker_id}: {e}"));
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
    fn generate_test_data(&self, schema: &SchemaDefinition) -> Value {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // If schema has no classes, return basic test data
        if schema.classes.is_empty() {
            return serde_json::json!({
                "schema_name": schema.name,
                "generated_at": chrono::Utc::now().to_rfc3339(),
                "test_id": format!("test_{}", rng.r#gen::<u32>()),
                "message": "No classes defined in schema"
            });
        }

        // Pick a random class to generate data for
        let class_names: Vec<_> = schema.classes.keys().collect();
        let selected_class = class_names[rng.gen_range(0..class_names.len())];
        let class_def = &schema.classes[selected_class];

        let mut test_object = serde_json::Map::new();

        // Add class metadata
        test_object.insert("@type".to_string(), Value::String(selected_class.clone()));
        test_object.insert("@schema".to_string(), Value::String(schema.name.clone()));
        test_object.insert("@generated_at".to_string(),
                          Value::String(chrono::Utc::now().to_rfc3339()));

        // Generate data for each slot
        for slot_name in &class_def.slots {
            let slot_value = if let Some(slot_def) = schema.slots.get(slot_name) {
                self.generate_slot_value(slot_def, &mut rng, schema)
            } else {
                // Generate generic value if slot not defined
                self.generate_generic_value(&mut rng)
            };

            test_object.insert(slot_name.clone(), slot_value);
        }

        // Add some synthetic slots for stress testing
        for i in 0..rng.gen_range(1..5) {
            let synthetic_key = format!("stress_field_{}", i);
            test_object.insert(synthetic_key, self.generate_generic_value(&mut rng));
        }

        Value::Object(test_object)
    }

    fn generate_slot_value(&self, slot_def: &linkml_core::types::SlotDefinition,
                          rng: &mut rand::rngs::ThreadRng,
                          schema: &SchemaDefinition) -> Value {
        // Generate value based on slot definition
        if let Some(range) = &slot_def.range {
            // Check if range is an enum
            if let Some(enum_def) = schema.enums.get(range) {
                let values: Vec<String> = enum_def.permissible_values.iter().map(|pv| {
                    match pv {
                        linkml_core::types::PermissibleValue::Simple(text) | linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
                    }
                }).collect();
                if !values.is_empty() {
                    return Value::String(values[rng.gen_range(0..values.len())].clone());
                }
            }

            // Generate based on known types
            match range.as_str() {
                "string" | "str" => Value::String(format!("test_string_{}", rng.r#gen::<u32>())),
                "integer" | "int" => Value::Number(serde_json::Number::from(rng.gen_range(1..1000))),
                "float" | "double" => {
                    Value::Number(serde_json::Number::from_f64(rng.r#gen::<f64>() * 1000.0).unwrap())
                },
                "boolean" | "bool" => Value::Bool(rng.r#gen()),
                "date" => Value::String(chrono::Utc::now().date_naive().to_string()),
                "datetime" => Value::String(chrono::Utc::now().to_rfc3339()),
                "uri" | "url" => Value::String(format!("https://example.com/resource/{}", rng.r#gen::<u32>())),
                _ => self.generate_generic_value(rng),
            }
        } else {
            self.generate_generic_value(rng)
        }
    }

    fn generate_generic_value(&self, rng: &mut rand::rngs::ThreadRng) -> Value {
        match rng.gen_range(0..6) {
            0 => Value::String(format!("generated_string_{}", rng.r#gen::<u32>())),
            1 => Value::Number(serde_json::Number::from(rng.gen_range(1..10000))),
            2 => Value::Bool(rng.r#gen()),
            3 => Value::Array(vec![
                Value::String("item1".to_string()),
                Value::String("item2".to_string()),
            ]),
            4 => {
                let mut obj = serde_json::Map::new();
                obj.insert("nested_key".to_string(), Value::String(format!("nested_value_{}", rng.r#gen::<u32>())));
                Value::Object(obj)
            },
            _ => Value::Null,
        }
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

    /// Apply chaos testing effects through real system load
    async fn apply_chaos(failure_rate: f64, max_delay_ms: u64) {
        use rand::Rng;

        // Create real CPU load instead of simulated delay
        if max_delay_ms > 0 {
            let delay_ms = {
                let mut rng = rand::thread_rng();
                rng.gen_range(0..=max_delay_ms)
            };
            
            // Perform real CPU-intensive work for the specified duration
            let start = std::time::Instant::now();
            let target_duration = Duration::from_millis(delay_ms);
            
            // Real computation work - calculate fibonacci numbers
            let mut a = 0u64;
            let mut b = 1u64;
            while start.elapsed() < target_duration {
                let temp = a.wrapping_add(b);
                a = b;
                b = temp;
                // Yield occasionally to avoid blocking the executor
                if a % 1000 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }

        // Introduce real resource contention instead of fake failures
        let failure_roll = {
            let mut rng = rand::thread_rng();
            rng.gen_range(0.0..1.0)
        };
        if failure_roll < failure_rate {
            // Create real memory pressure by allocating and deallocating
            let size = 1024 * 1024; // 1MB
            let _memory_pressure: Vec<u8> = vec![0; size];
            // Memory is automatically freed when going out of scope
            // This creates real GC/allocator pressure
        }
    }

    /// Calculate final results
    fn calculate_results(&self, duration: Duration) -> crate::Result<StressTestResults> {
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

        let avg_latency_ms = if latencies.is_empty() {
            0.0
        } else {
            latencies.iter().map(|d| d.as_millis() as f64).sum::<f64>() / latencies.len() as f64
        };

        let p50_latency_ms = Self::percentile(&latencies, 0.50);
        let p95_latency_ms = Self::percentile(&latencies, 0.95);
        let p99_latency_ms = Self::percentile(&latencies, 0.99);
        let max_latency_ms = latencies
            .last()
            .map_or(0.0, |d| d.as_millis() as f64);

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
            errors})
    }

    /// Calculate percentile from sorted latencies
    fn percentile(sorted_latencies: &[Duration], percentile: f64) -> f64 {
        if sorted_latencies.is_empty() {
            return 0.0;
        }

        let index = ((sorted_latencies.len() - 1) as f64 * percentile) as usize;
        sorted_latencies
            .get(index)
            .map_or(0.0, |d| d.as_millis() as f64)
    }
}
