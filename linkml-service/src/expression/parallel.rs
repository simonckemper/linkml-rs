//! Parallel evaluation support for `LinkML` expressions
//!
//! This module provides functionality to evaluate multiple expressions
//! concurrently for improved performance.

use super::{EvaluationError, Expression, ExpressionEngine};
use futures::future::join_all;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::task::JoinHandle;

/// Result of parallel expression evaluation
#[derive(Debug, Clone)]
pub struct ParallelResult {
    /// Successfully evaluated expressions (key -> result)
    pub successful: HashMap<String, Value>,

    /// Failed expressions (key -> error)
    pub failed: HashMap<String, String>,

    /// Total evaluation time in milliseconds
    pub total_time_ms: u64,
}

/// Options for parallel evaluation
#[derive(Debug, Clone)]
pub struct ParallelOptions {
    /// Maximum number of concurrent evaluations
    pub max_concurrency: usize,

    /// Whether to fail fast on first error
    pub fail_fast: bool,

    /// Timeout for each expression in milliseconds (0 = use default)
    pub timeout_ms: u64,
}

impl Default for ParallelOptions {
    fn default() -> Self {
        Self {
            max_concurrency: num_cpus::get(),
            fail_fast: false,
            timeout_ms: 0,
        }
    }
}

/// Extension trait for parallel evaluation
pub trait ParallelEvaluator {
    /// Evaluate multiple expressions in parallel
    fn evaluate_parallel(
        &self,
        expressions: HashMap<String, String>,
        context: &HashMap<String, Value>,
        options: ParallelOptions,
    ) -> impl std::future::Future<Output = ParallelResult> + Send;

    /// Evaluate multiple pre-parsed expressions in parallel
    fn evaluate_ast_parallel(
        &self,
        expressions: HashMap<String, Expression>,
        context: &HashMap<String, Value>,
        options: ParallelOptions,
    ) -> impl std::future::Future<Output = ParallelResult> + Send;

    /// Evaluate the same expression with multiple different contexts
    fn evaluate_with_contexts(
        &self,
        expression: &str,
        contexts: Vec<HashMap<String, Value>>,
        options: ParallelOptions,
    ) -> impl std::future::Future<Output = Vec<Result<Value, EvaluationError>>> + Send;
}

// Task handle manager removed - not needed for properly awaited tasks

impl ParallelEvaluator for ExpressionEngine {
    async fn evaluate_parallel(
        &self,
        expressions: HashMap<String, String>,
        context: &HashMap<String, Value>,
        options: ParallelOptions,
    ) -> ParallelResult {
        let timestamp_service = self.timestamp_service();
        let start_time = timestamp_service
            .system_time()
            .unwrap_or_else(|_| SystemTime::now());
        let engine = Arc::new(self.clone());
        let context = Arc::new(context.clone());

        // Create semaphore for concurrency control
        let semaphore = Arc::new(tokio::sync::Semaphore::new(options.max_concurrency));

        let mut tasks: Vec<JoinHandle<(String, Result<Value, String>)>> = Vec::new();

        for (key, expr) in expressions {
            let engine = Arc::clone(&engine);
            let context = Arc::clone(&context);
            let semaphore = Arc::clone(&semaphore);
            let key_clone = key.clone();

            let task = tokio::spawn(async move {
                // Acquire permit for concurrency control
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        return (
                            key_clone,
                            Err(format!("semaphore should not be closed: {e}")),
                        );
                    }
                };

                // Parse and evaluate
                let result = match engine.parse(&expr) {
                    Ok(ast) => match engine.evaluate_ast(&ast, &context) {
                        Ok(value) => Ok(value),
                        Err(e) => Err(e.to_string()),
                    },
                    Err(e) => Err(e.to_string()),
                };

                (key_clone, result)
            });

            tasks.push(task);
        }

        // Wait for all tasks or fail fast

        if options.fail_fast {
            // For fail-fast, check results as they complete
            let mut successful = HashMap::new();
            let mut failed = HashMap::new();

            for task in tasks {
                match task.await {
                    Ok((key, Ok(value))) => {
                        successful.insert(key, value);
                    }
                    Ok((key, Err(error))) => {
                        failed.insert(key, error);
                        // Cancel remaining tasks on first error
                        break;
                    }
                    Err(e) => {
                        failed.insert("_join_error".to_string(), e.to_string());
                        break;
                    }
                }
            }

            ParallelResult {
                successful,
                failed,
                total_time_ms: {
                    let end_time = timestamp_service
                        .system_time()
                        .unwrap_or_else(|_| SystemTime::now());
                    let duration = end_time
                        .duration_since(start_time)
                        .unwrap_or_else(|_| std::time::Duration::from_millis(0));
                    duration.as_millis() as u64
                },
            }
        } else {
            // Collect all results
            let all_results = join_all(tasks).await;
            let mut successful = HashMap::new();
            let mut failed = HashMap::new();

            for result in all_results {
                match result {
                    Ok((key, Ok(value))) => {
                        successful.insert(key, value);
                    }
                    Ok((key, Err(error))) => {
                        failed.insert(key, error);
                    }
                    Err(e) => {
                        failed.insert("_join_error".to_string(), e.to_string());
                    }
                }
            }

            ParallelResult {
                successful,
                failed,
                total_time_ms: {
                    let end_time = timestamp_service
                        .system_time()
                        .unwrap_or_else(|_| SystemTime::now());
                    let duration = end_time
                        .duration_since(start_time)
                        .unwrap_or_else(|_| std::time::Duration::from_millis(0));
                    duration.as_millis() as u64
                },
            }
        }
    }

    async fn evaluate_ast_parallel(
        &self,
        expressions: HashMap<String, Expression>,
        context: &HashMap<String, Value>,
        options: ParallelOptions,
    ) -> ParallelResult {
        let timestamp_service = self.timestamp_service();
        let start_time = timestamp_service
            .system_time()
            .unwrap_or_else(|_| SystemTime::now());
        let engine = Arc::new(self.clone());
        let context = Arc::new(context.clone());

        let semaphore = Arc::new(tokio::sync::Semaphore::new(options.max_concurrency));
        let mut tasks: Vec<JoinHandle<(String, Result<Value, String>)>> = Vec::new();

        for (key, ast) in expressions {
            let engine = Arc::clone(&engine);
            let context = Arc::clone(&context);
            let semaphore = Arc::clone(&semaphore);
            let key_clone = key.clone();

            let task = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        return (
                            key_clone,
                            Err(format!("semaphore should not be closed: {e}")),
                        );
                    }
                };

                let result = match engine.evaluate_ast(&ast, &context) {
                    Ok(value) => Ok(value),
                    Err(e) => Err(e.to_string()),
                };

                (key_clone, result)
            });

            tasks.push(task);
        }

        let all_results = join_all(tasks).await;
        let mut successful = HashMap::new();
        let mut failed = HashMap::new();

        for result in all_results {
            match result {
                Ok((key, Ok(value))) => {
                    successful.insert(key, value);
                }
                Ok((key, Err(error))) => {
                    failed.insert(key, error);
                }
                Err(e) => {
                    failed.insert("_join_error".to_string(), e.to_string());
                }
            }
        }

        let end_time = timestamp_service
            .system_time()
            .unwrap_or_else(|_| SystemTime::now());
        let duration = end_time
            .duration_since(start_time)
            .unwrap_or_else(|_| std::time::Duration::from_millis(0));

        ParallelResult {
            successful,
            failed,
            total_time_ms: duration.as_millis() as u64,
        }
    }

    async fn evaluate_with_contexts(
        &self,
        expression: &str,
        contexts: Vec<HashMap<String, Value>>,
        options: ParallelOptions,
    ) -> Vec<Result<Value, EvaluationError>> {
        let engine = Arc::new(self.clone());

        // Parse expression once
        let ast = match engine.parse(expression) {
            Ok(ast) => Arc::new(ast),
            Err(e) => {
                // Return error for all contexts
                let error = Err(EvaluationError::TypeError {
                    message: format!("Parse error: {e}"),
                });
                return (0..contexts.len()).map(|_| error.clone()).collect();
            }
        };

        let semaphore = Arc::new(tokio::sync::Semaphore::new(options.max_concurrency));
        let mut tasks = Vec::new();

        for (idx, context) in contexts.into_iter().enumerate() {
            let engine = Arc::clone(&engine);
            let ast = Arc::clone(&ast);
            let semaphore = Arc::clone(&semaphore);

            let task = tokio::spawn(async move {
                let _permit = match semaphore.acquire().await {
                    Ok(permit) => permit,
                    Err(e) => {
                        return (
                            idx,
                            Err(EvaluationError::TypeError {
                                message: format!("semaphore should not be closed: {e}"),
                            }),
                        );
                    }
                };
                let result =
                    engine
                        .evaluate_ast(&ast, &context)
                        .map_err(|e| EvaluationError::TypeError {
                            message: e.to_string(),
                        });
                (idx, result)
            });

            tasks.push(task);
        }

        // Collect results preserving order
        let mut results: Vec<Result<Value, EvaluationError>> = (0..tasks.len())
            .map(|_| {
                Err(EvaluationError::TypeError {
                    message: "Unprocessed".to_string(),
                })
            })
            .collect();

        for (idx, task) in join_all(tasks).await.into_iter().enumerate() {
            match task {
                Ok((task_idx, result)) => results[task_idx] = result,
                Err(e) => {
                    results[idx] = Err(EvaluationError::TypeError {
                        message: format!("Task join error: {e}"),
                    });
                }
            }
        }

        results
    }
}

/// Batch evaluation helper for common patterns
pub struct BatchEvaluator {
    engine: Arc<ExpressionEngine>,
    options: ParallelOptions,
}

impl BatchEvaluator {
    /// Create a new batch evaluator
    #[must_use]
    pub fn new(engine: ExpressionEngine) -> Self {
        Self {
            engine: Arc::new(engine),
            options: ParallelOptions::default(),
        }
    }

    /// Set parallel options
    #[must_use]
    pub fn with_options(mut self, options: ParallelOptions) -> Self {
        self.options = options;
        self
    }

    /// Evaluate a collection of items with an expression template
    ///
    /// Example:
    /// ```
    /// let items = vec![
    ///     json!({"price": 10, "quantity": 5}),
    ///     json!({"price": 20, "quantity": 3}),
    /// ];
    /// let results = batch.evaluate_collection(
    ///     "{price} * {quantity}",
    ///     items,
    ///     &base_context
    /// ).await;
    /// ```
    pub async fn evaluate_collection(
        &self,
        expression_template: &str,
        items: Vec<Value>,
        base_context: &HashMap<String, Value>,
    ) -> Vec<Result<Value, EvaluationError>> {
        // Create contexts for each item
        let contexts: Vec<HashMap<String, Value>> = items
            .into_iter()
            .map(|item| {
                let mut ctx = base_context.clone();

                // Add item fields to context
                if let Value::Object(obj) = item {
                    for (key, value) in obj {
                        ctx.insert(key, value);
                    }
                } else {
                    ctx.insert("item".to_string(), item);
                }

                ctx
            })
            .collect();

        self.engine
            .evaluate_with_contexts(expression_template, contexts, self.options.clone())
            .await
    }

    /// Map-reduce style evaluation
    ///
    /// Map phase: evaluate expression for each item
    /// Reduce phase: aggregate results with reducer expression
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub async fn map_reduce(
        &self,
        map_expression: &str,
        reduce_expression: &str,
        items: Vec<Value>,
        base_context: &HashMap<String, Value>,
    ) -> Result<Value, EvaluationError> {
        // Map phase
        let map_results = self
            .evaluate_collection(map_expression, items, base_context)
            .await;

        // Collect successful results
        let values: Vec<Value> = map_results
            .into_iter()
            .filter_map(std::result::Result::ok)
            .collect();

        // Reduce phase
        let mut reduce_context = base_context.clone();
        reduce_context.insert("values".to_string(), Value::Array(values));

        self.engine
            .evaluate(reduce_expression, &reduce_context)
            .map_err(|e| EvaluationError::TypeError {
                message: e.to_string(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_parallel_evaluation() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExpressionEngine::new();
        let context = HashMap::from([("x".to_string(), json!(10)), ("y".to_string(), json!(5))]);

        let expressions = HashMap::from([
            ("sum".to_string(), "{x} + {y}".to_string()),
            ("diff".to_string(), "{x} - {y}".to_string()),
            ("product".to_string(), "{x} * {y}".to_string()),
        ]);

        let result = engine
            .evaluate_parallel(expressions, &context, ParallelOptions::default())
            .await;

        assert_eq!(result.successful.len(), 3);
        assert_eq!(result.successful.get("sum"), Some(&json!(15.0)));
        assert_eq!(result.successful.get("diff"), Some(&json!(5.0)));
        assert_eq!(result.successful.get("product"), Some(&json!(50.0)));
        assert_eq!(result.failed.len(), 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_evaluate_with_contexts() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExpressionEngine::new();

        let contexts = vec![
            HashMap::from([("x".to_string(), json!(1))]),
            HashMap::from([("x".to_string(), json!(2))]),
            HashMap::from([("x".to_string(), json!(3))]),
        ];

        let results = engine
            .evaluate_with_contexts("{x} * 2", contexts, ParallelOptions::default())
            .await;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().expect("context 0 value"), &json!(2.0));
        assert_eq!(results[1].as_ref().expect("context 1 value"), &json!(4.0));
        assert_eq!(results[2].as_ref().expect("context 2 value"), &json!(6.0));
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_evaluator() -> Result<(), Box<dyn std::error::Error>> {
        let engine = ExpressionEngine::new();
        let batch = BatchEvaluator::new(engine);

        let items = vec![
            json!({"price": 10, "quantity": 5}),
            json!({"price": 20, "quantity": 3}),
            json!({"price": 15, "quantity": 4}),
        ];

        let results = batch
            .evaluate_collection("{price} * {quantity}", items.clone(), &HashMap::new())
            .await;

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].as_ref().expect("item 0 value"), &json!(50.0));
        assert_eq!(results[1].as_ref().expect("item 1 value"), &json!(60.0));
        assert_eq!(results[2].as_ref().expect("item 2 value"), &json!(60.0));

        // Test map-reduce
        let total = batch
            .map_reduce(
                "{price} * {quantity}",
                "sum({values})",
                items,
                &HashMap::new(),
            )
            .await?;

        assert_eq!(total, json!(170.0));
        Ok(())
    }
}
