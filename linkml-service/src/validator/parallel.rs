//! Parallel validation for improved performance
//!
//! This module provides parallel validation capabilities using Rayon
//! to validate multiple values concurrently while maintaining thread safety.

use super::{
    ValidationEngine, ValidationIssue, ValidationOptions, ValidationReport,
    buffer_pool::ValidationBufferPools, context::ValidationContext,
};
use crate::utils::safe_cast::{u64_to_f64_lossy, usize_to_f64};
use rayon::prelude::*;
use serde_json::Value;
use std::sync::{Arc, Mutex};

/// Parallel validation engine for bulk validation
pub struct ParallelValidationEngine {
    engine: Arc<ValidationEngine>,
    /// Thread pool for parallel execution
    thread_pool: rayon::ThreadPool,
    /// Shared buffer pools
    buffer_pools: Arc<ValidationBufferPools>,
}

impl ParallelValidationEngine {
    /// Create a new parallel validation engine
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if thread pool creation fails
    pub fn new(engine: ValidationEngine) -> linkml_core::error::Result<Self> {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_cpus::get())
            .build()
            .map_err(|e| {
                linkml_core::error::LinkMLError::service(format!(
                    "Failed to create thread pool: {e}"
                ))
            })?;

        Ok(Self {
            engine: Arc::new(engine),
            thread_pool,
            buffer_pools: Arc::new(ValidationBufferPools::new()),
        })
    }

    /// Create with custom thread pool configuration
    ///
    /// # Errors
    ///
    /// Returns a `LinkMLError` if thread pool creation fails
    pub fn with_thread_count(
        engine: ValidationEngine,
        threads: usize,
    ) -> linkml_core::error::Result<Self> {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| {
                linkml_core::error::LinkMLError::service(format!(
                    "Failed to create thread pool: {e}"
                ))
            })?;

        Ok(Self {
            engine: Arc::new(engine),
            thread_pool,
            buffer_pools: Arc::new(ValidationBufferPools::new()),
        })
    }

    /// Validate multiple values in parallel
    #[must_use]
    pub fn validate_batch(
        &self,
        values: &[Value],
        class_name: &str,
        options: Option<ValidationOptions>,
    ) -> Vec<ValidationReport> {
        let engine = Arc::clone(&self.engine);
        let class_name = class_name.to_string();
        let options = options.unwrap_or_default();

        // Use thread pool to parallelize validation
        self.thread_pool.install(|| {
            values
                .par_iter()
                .map(|value| {
                    // Each thread gets its own context with shared buffer pools
                    let _context = ValidationContext::with_buffer_pools(
                        engine.schema.clone(),
                        self.buffer_pools.clone(),
                    );

                    // Validate synchronously within the thread
                    futures::executor::block_on(engine.validate_as_class(
                        value,
                        &class_name,
                        Some(options.clone()),
                    ))
                    .unwrap_or_else(|e| {
                        // Create error report if validation fails
                        let mut report = ValidationReport::new(&engine.schema.id);
                        report.add_issue(ValidationIssue::error(
                            format!("Validation error: {e}"),
                            "$",
                            "parallel_validator",
                        ));
                        report
                    })
                })
                .collect()
        })
    }

    /// Validate values in parallel with result aggregation
    #[must_use]
    pub fn validate_batch_aggregated(
        &self,
        values: &[(String, Value)], // (id, value) pairs
        class_name: &str,
        options: Option<ValidationOptions>,
    ) -> AggregatedValidationReport {
        let reports = self.validate_batch(
            &values.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>(),
            class_name,
            options,
        );

        // Aggregate results
        let mut aggregated = AggregatedValidationReport::new(&self.engine.schema.id);

        for ((id, _), report) in values.iter().zip(reports.iter()) {
            aggregated.add_report(id.clone(), report.clone());
        }

        aggregated
    }

    /// Validate a stream of values with parallel processing
    ///
    /// # Panics
    ///
    /// This function may panic if the mutex lock is poisoned.
    pub fn validate_stream<I>(
        &self,
        values: I,
        class_name: &str,
        options: Option<ValidationOptions>,
        chunk_size: usize,
    ) -> StreamValidationResult
    where
        I: Iterator<Item = Value> + Send,
    {
        let engine = Arc::clone(&self.engine);
        let class_name = class_name.to_string();
        let options = options.unwrap_or_default();
        let results = Arc::new(Mutex::new(StreamValidationResult::new()));

        // Process in chunks for better parallelism
        let chunks: Vec<Vec<Value>> = values
            .collect::<Vec<_>>()
            .chunks(chunk_size)
            .map(<[linkml_core::Value]>::to_vec)
            .collect();

        self.thread_pool.install(|| {
            chunks.par_iter().for_each(|chunk| {
                for value in chunk {
                    let report = futures::executor::block_on(engine.validate_as_class(
                        value,
                        &class_name,
                        Some(options.clone()),
                    ))
                    .unwrap_or_else(|e| {
                        let mut report = ValidationReport::new(&engine.schema.id);
                        report.add_issue(ValidationIssue::error(
                            format!("Validation error: {e}"),
                            "$",
                            "parallel_validator",
                        ));
                        report
                    });

                    // Handle mutex poisoning gracefully
                    match results.lock() {
                        Ok(mut results_guard) => {
                            results_guard.add_report(report);
                        }
                        Err(poisoned) => {
                            // Mutex was poisoned, but we can still recover the data
                            let mut results_guard = poisoned.into_inner();
                            results_guard.add_report(report);
                        }
                    }
                }
            });
        });

        // Finalize and extract the result
        let results_clone = results.clone();
        let mut final_result = match Arc::try_unwrap(results) {
            Ok(mutex) => match mutex.into_inner() {
                Ok(result) => result,
                Err(poisoned) => {
                    // Recover from poisoned mutex
                    poisoned.into_inner()
                }
            },
            Err(_) => {
                // If we can't unwrap the Arc, clone the inner value
                match results_clone.lock() {
                    Ok(guard) => guard.clone(),
                    Err(poisoned) => {
                        // Recover from poisoned mutex
                        poisoned.into_inner().clone()
                    }
                }
            }
        };

        final_result.finalize();
        final_result
    }
}

/// Aggregated validation report for batch validation
#[derive(Debug, Clone)]
pub struct AggregatedValidationReport {
    /// Schema ID
    pub schema_id: String,
    /// Individual reports by ID
    pub reports: std::collections::HashMap<String, ValidationReport>,
    /// Total number of valid items
    pub total_valid: usize,
    /// Total number of invalid items
    pub total_invalid: usize,
    /// Total number of validation errors across all items
    pub total_errors: usize,
    /// Total number of validation warnings across all items
    pub total_warnings: usize,
}

impl AggregatedValidationReport {
    /// Create a new aggregated report
    #[must_use]
    pub fn new(schema_id: &str) -> Self {
        Self {
            schema_id: schema_id.to_string(),
            reports: std::collections::HashMap::new(),
            total_valid: 0,
            total_invalid: 0,
            total_errors: 0,
            total_warnings: 0,
        }
    }

    /// Add a report for an ID
    pub fn add_report(&mut self, id: String, report: ValidationReport) {
        if report.valid {
            self.total_valid += 1;
        } else {
            self.total_invalid += 1;
        }

        self.total_errors += report.stats.error_count;
        self.total_warnings += report.stats.warning_count;

        self.reports.insert(id, report);
    }

    /// Get overall validity
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.total_invalid == 0
    }

    /// Get summary
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Validated {} items: {} valid, {} invalid ({} errors, {} warnings)",
            self.reports.len(),
            self.total_valid,
            self.total_invalid,
            self.total_errors,
            self.total_warnings
        )
    }
}

/// Result of streaming validation
#[derive(Debug, Clone)]
pub struct StreamValidationResult {
    /// Number of items processed
    pub items_processed: usize,
    /// Number of valid items
    pub valid_count: usize,
    /// Number of invalid items
    pub invalid_count: usize,
    /// All validation issues
    pub all_issues: Vec<ValidationIssue>,
    /// Processing time in milliseconds
    pub duration_ms: u64,
    /// Start time
    start_time: std::time::Instant,
}

impl StreamValidationResult {
    /// Create a new stream result
    #[must_use]
    pub fn new() -> Self {
        Self {
            items_processed: 0,
            valid_count: 0,
            invalid_count: 0,
            all_issues: Vec::new(),
            duration_ms: 0,
            start_time: std::time::Instant::now(),
        }
    }

    /// Add a validation report
    pub fn add_report(&mut self, report: ValidationReport) {
        self.items_processed += 1;

        if report.valid {
            self.valid_count += 1;
        } else {
            self.invalid_count += 1;
        }

        self.all_issues.extend(report.issues);
    }

    /// Finalize the result
    pub fn finalize(&mut self) {
        self.duration_ms = u64::try_from(self.start_time.elapsed().as_millis()).unwrap_or(u64::MAX);
    }

    /// Get throughput in items per second
    #[must_use]
    pub fn throughput(&self) -> f64 {
        if self.duration_ms == 0 {
            0.0
        } else {
            // Calculate throughput using safe casting
            (usize_to_f64(self.items_processed) * 1000.0) / u64_to_f64_lossy(self.duration_ms)
        }
    }
}

impl Default for StreamValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Parallel validation configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of threads to use
    pub thread_count: usize,
    /// Chunk size for streaming
    pub chunk_size: usize,
    /// Whether to fail fast on first error
    pub fail_fast: bool,
    /// Maximum memory per thread
    pub max_memory_per_thread: usize,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            thread_count: num_cpus::get(),
            chunk_size: 100,
            fail_fast: false,
            max_memory_per_thread: 100 * 1024 * 1024, // 100MB
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SchemaDefinition;
    use serde_json::json;

    #[tokio::test]
    async fn test_parallel_batch_validation() -> anyhow::Result<()> {
        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        let engine = ValidationEngine::new(&schema).expect("should create validation engine: {}");
        let parallel_engine =
            ParallelValidationEngine::new(engine).expect("should create parallel engine: {}");

        let values = vec![
            json!({"name": "test1"}),
            json!({"name": "test2"}),
            json!({"name": "test3"}),
        ];

        let reports = parallel_engine.validate_batch(&values, "TestClass", None);

        assert_eq!(reports.len(), 3);
        Ok(())
    }

    #[tokio::test]
    async fn test_aggregated_validation() -> anyhow::Result<()> {
        let schema = SchemaDefinition {
            id: "test-schema".to_string(),
            name: "TestSchema".to_string(),
            ..Default::default()
        };

        let engine = ValidationEngine::new(&schema).expect("should create validation engine: {}");
        let parallel_engine =
            ParallelValidationEngine::new(engine).expect("should create parallel engine: {}");

        let values = vec![
            ("id1".to_string(), json!({"name": "test1"})),
            ("id2".to_string(), json!({"name": "test2"})),
        ];

        let aggregated = parallel_engine.validate_batch_aggregated(&values, "TestClass", None);

        assert_eq!(aggregated.reports.len(), 2);
        assert!(aggregated.reports.contains_key("id1"));
        assert!(aggregated.reports.contains_key("id2"));
        Ok(())
    }
}
