//! Enhanced expression engine with JIT compilation and caching
//!
//! This module provides a high-performance expression engine that combines
//! parsing, compilation, caching, and VM execution for optimal performance.

use super::ast::Expression;
use super::cache::{ExpressionKey, GlobalExpressionCache};
use super::compiler::{CompiledExpression, Compiler};
use super::error::{ExpressionError, ParseError};
use super::evaluator::Evaluator;
use super::functions::FunctionRegistry;
use super::parser::Parser;
use super::vm::VirtualMachine;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use timestamp_core::SyncTimestampService;

/// Configuration for the enhanced expression engine
#[derive(Clone)]
pub struct EngineConfig {
    /// Whether to use JIT compilation
    pub use_compilation: bool,
    /// Whether to cache expressions
    pub use_caching: bool,
    /// Cache capacity for parsed expressions
    pub cache_capacity: usize,
    /// Cache capacity for hot expressions
    pub hot_cache_capacity: usize,
    /// Optimization level for compiler (0-3)
    pub optimization_level: u8,
    /// Threshold for using compiled vs interpreted evaluation
    pub compilation_threshold: u64,
    /// Whether to collect performance metrics
    pub collect_metrics: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            use_compilation: true,
            use_caching: true,
            cache_capacity: 1000,
            hot_cache_capacity: 100,
            optimization_level: 2,
            compilation_threshold: 3,
            collect_metrics: false,
        }
    }
}

/// Performance metrics for expression evaluation
#[derive(Clone, Debug, Default)]
pub struct PerformanceMetrics {
    /// Total evaluations
    pub total_evaluations: u64,
    /// Evaluations using interpreter
    pub interpreted_evaluations: u64,
    /// Evaluations using VM
    pub compiled_evaluations: u64,
    /// Total time spent parsing (microseconds)
    pub parse_time_us: u64,
    /// Total time spent compiling (microseconds)
    pub compile_time_us: u64,
    /// Total time spent evaluating (microseconds)
    pub eval_time_us: u64,
    /// Total end-to-end time for all operations (microseconds)
    pub total_time_us: u64,
    /// Average evaluation time (microseconds)
    pub avg_eval_time_us: u64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

/// Enhanced expression engine with compilation and caching
pub struct ExpressionEngineV2 {
    /// Parser instance
    parser: Parser,
    /// Compiler instance
    compiler: Arc<Compiler>,
    /// Interpreter for fallback
    evaluator: Arc<Evaluator>,
    /// Virtual machine for compiled execution
    vm: Arc<VirtualMachine>,
    /// Expression cache
    cache: Arc<GlobalExpressionCache>,
    /// Engine configuration
    config: EngineConfig,
    /// Performance metrics
    metrics: Arc<std::sync::RwLock<PerformanceMetrics>>,
    /// Timestamp service for timing operations
    timestamp_service: Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>>,
}

impl ExpressionEngineV2 {
    /// Create a new enhanced expression engine
    #[must_use]
    pub fn new(config: EngineConfig) -> Self {
        let function_registry = Arc::new(FunctionRegistry::new());
        let timestamp_service = timestamp_service::factory::create_sync_timestamp_service();

        Self {
            parser: Parser::new(),
            compiler: Arc::new(
                Compiler::new(Arc::clone(&function_registry))
                    .with_optimization_level(config.optimization_level),
            ),
            evaluator: Arc::new(Evaluator::new()),
            vm: Arc::new(VirtualMachine::new(Arc::clone(&function_registry))),
            cache: Arc::new(GlobalExpressionCache::new(
                config.cache_capacity,
                config.hot_cache_capacity,
            )),
            config,
            metrics: Arc::new(std::sync::RwLock::new(PerformanceMetrics::default())),
            timestamp_service,
        }
    }

    /// Create a new enhanced expression engine with injected timestamp service (factory pattern compliant)
    pub fn with_timestamp_service<T>(config: EngineConfig, timestamp_service: Arc<T>) -> Self
    where
        T: SyncTimestampService<Error = timestamp_core::TimestampError> + Send + Sync + 'static,
    {
        let function_registry = Arc::new(FunctionRegistry::new());

        Self {
            parser: Parser::new(),
            compiler: Arc::new(
                Compiler::new(Arc::clone(&function_registry))
                    .with_optimization_level(config.optimization_level),
            ),
            evaluator: Arc::new(Evaluator::new()),
            vm: Arc::new(VirtualMachine::new(Arc::clone(&function_registry))),
            cache: Arc::new(GlobalExpressionCache::new(
                config.cache_capacity,
                config.hot_cache_capacity,
            )),
            config,
            metrics: Arc::new(std::sync::RwLock::new(PerformanceMetrics::default())),
            timestamp_service,
        }
    }

    /// Create with custom function registry
    #[must_use]
    pub fn with_function_registry(
        config: EngineConfig,
        function_registry: Arc<FunctionRegistry>,
    ) -> Self {
        let timestamp_service = timestamp_service::wiring::wire_timestamp().into_arc();

        Self {
            parser: Parser::new(),
            compiler: Arc::new(
                Compiler::new(Arc::clone(&function_registry))
                    .with_optimization_level(config.optimization_level),
            ),
            evaluator: Arc::new(Evaluator::with_functions(
                FunctionRegistry::default(), // Create a new instance for the evaluator
            )),
            vm: Arc::new(VirtualMachine::new(function_registry)),
            cache: Arc::new(GlobalExpressionCache::new(
                config.cache_capacity,
                config.hot_cache_capacity,
            )),
            config,
            metrics: Arc::new(std::sync::RwLock::new(PerformanceMetrics::default())),
            timestamp_service,
        }
    }

    /// Create with custom function registry and injected timestamp service (factory pattern compliant)
    pub fn with_function_registry_and_timestamp<T>(
        config: EngineConfig,
        function_registry: Arc<FunctionRegistry>,
        timestamp_service: Arc<T>,
    ) -> Self
    where
        T: SyncTimestampService<Error = timestamp_core::TimestampError> + Send + Sync + 'static,
    {
        Self {
            parser: Parser::new(),
            compiler: Arc::new(
                Compiler::new(Arc::clone(&function_registry))
                    .with_optimization_level(config.optimization_level),
            ),
            evaluator: Arc::new(Evaluator::with_functions(
                FunctionRegistry::default(), // Create a new instance for the evaluator
            )),
            vm: Arc::new(VirtualMachine::new(function_registry)),
            cache: Arc::new(GlobalExpressionCache::new(
                config.cache_capacity,
                config.hot_cache_capacity,
            )),
            config,
            metrics: Arc::new(std::sync::RwLock::new(PerformanceMetrics::default())),
            timestamp_service,
        }
    }

    /// Evaluate an expression with the given context
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> Result<Value, ExpressionError> {
        self.evaluate_with_schema(expression, context, None)
    }

    /// Evaluate an expression with schema context for better caching
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate_with_schema(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
        schema_id: Option<&str>,
    ) -> Result<Value, ExpressionError> {
        let start_time = if self.config.collect_metrics {
            Some(self.timestamp_service.system_time().map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Failed to get system time: {e}"),
                })
            })?)
        } else {
            None
        };

        // Create cache key
        let key = ExpressionKey {
            source: expression.to_string(),
            schema_id: schema_id.map(std::string::ToString::to_string),
        };

        // Try to get from cache
        let (ast, compiled) = if self.config.use_caching {
            if let Some(cached) = self.cache.get(&key) {
                if let Some(start) = start_time {
                    let mut metrics = self
                        .metrics
                        .write()
                        .expect("metrics lock should not be poisoned: {}");
                    metrics.cache_hit_rate = self.cache.overall_hit_rate();
                    // Track cache hit timing - this is very fast
                    if let Ok(now) = self.timestamp_service.system_time()
                        && let Ok(duration) = now.duration_since(start)
                    {
                        // Cache hits are typically sub-microsecond
                        metrics.total_time_us += duration.as_micros() as u64;
                    }
                }
                (cached.ast, cached.compiled)
            } else {
                // Parse and optionally compile
                let (ast, compiled) = self.parse_and_compile(expression, start_time)?;

                // Cache the result
                self.cache.insert(key, ast.clone(), compiled.clone());

                (ast, compiled)
            }
        } else {
            // No caching - parse and compile every time
            self.parse_and_compile(expression, start_time)?
        };

        // Decide whether to use compiled or interpreted evaluation
        let result = if self.should_use_compiled(&compiled) {
            let compiled_expr = compiled.as_ref().ok_or_else(|| {
                ExpressionError::Other(
                    "should have compiled expression when use_compiled is true".to_string(),
                )
            })?;
            self.evaluate_compiled(compiled_expr, context, start_time)?
        } else {
            self.evaluate_interpreted(&ast, context, start_time)?
        };

        // Update metrics including total operation time
        if self.config.collect_metrics {
            let mut metrics = self
                .metrics
                .write()
                .expect("metrics lock should not be poisoned: {}");
            metrics.total_evaluations += 1;

            // Calculate and record total operation time if we have a start time
            if let Some(start) = start_time {
                let end = self
                    .timestamp_service
                    .system_time()
                    .map_err(|e| ExpressionError::Other(format!("Failed to get end time: {e}")))?;
                if let Ok(duration) = end.duration_since(start) {
                    // Track total evaluation time including cache lookups, parsing, compilation, and execution
                    metrics.total_time_us += duration.as_micros() as u64;

                    // Track average evaluation time
                    let avg_time_us = metrics.total_time_us / metrics.total_evaluations.max(1);
                    metrics.avg_eval_time_us = avg_time_us;
                }
            }
        }

        Ok(result)
    }

    /// Parse and optionally compile an expression
    fn parse_and_compile(
        &self,
        expression: &str,
        start_time: Option<SystemTime>,
    ) -> Result<(Expression, Option<Arc<CompiledExpression>>), ExpressionError> {
        // Parse
        let parse_start = self.timestamp_service.system_time().map_err(|e| {
            ExpressionError::Parse(ParseError::SystemError {
                message: format!("Failed to get system time: {e}"),
            })
        })?;
        let ast = self.parser.parse(expression)?;

        if let Some(_start) = start_time {
            let parse_end = self.timestamp_service.system_time().map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Failed to get system time: {e}"),
                })
            })?;
            let parse_duration = parse_end.duration_since(parse_start).map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Time calculation error: {e}"),
                })
            })?;
            let mut metrics = self.metrics.write().map_err(|e| {
                ExpressionError::Other(format!("metrics lock should not be poisoned: {e}"))
            })?;
            metrics.parse_time_us += parse_duration.as_micros() as u64;
        }

        // Compile if enabled
        let compiled = if self.config.use_compilation {
            let compile_start = self.timestamp_service.system_time().map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Failed to get system time: {e}"),
                })
            })?;
            let compiled = self.compiler.compile(&ast, expression)?;

            if let Some(_start) = start_time {
                let compile_end = self.timestamp_service.system_time().map_err(|e| {
                    ExpressionError::Parse(ParseError::SystemError {
                        message: format!("Failed to get system time: {e}"),
                    })
                })?;
                let compile_duration = compile_end.duration_since(compile_start).map_err(|e| {
                    ExpressionError::Parse(ParseError::SystemError {
                        message: format!("Time calculation error: {e}"),
                    })
                })?;
                let mut metrics = self.metrics.write().map_err(|e| {
                    ExpressionError::Other(format!("metrics lock should not be poisoned: {e}"))
                })?;
                metrics.compile_time_us += compile_duration.as_micros() as u64;
            }

            Some(Arc::new(compiled))
        } else {
            None
        };

        Ok((ast, compiled))
    }

    /// Decide whether to use compiled evaluation
    fn should_use_compiled(&self, compiled: &Option<Arc<CompiledExpression>>) -> bool {
        if let Some(compiled) = compiled {
            // Use compiled if complexity is above threshold
            compiled.metadata.complexity as u64 >= self.config.compilation_threshold
        } else {
            false
        }
    }

    /// Evaluate using the VM
    fn evaluate_compiled(
        &self,
        compiled: &CompiledExpression,
        context: &HashMap<String, Value>,
        start_time: Option<SystemTime>,
    ) -> Result<Value, ExpressionError> {
        let eval_start = self.timestamp_service.system_time().map_err(|e| {
            ExpressionError::Parse(ParseError::SystemError {
                message: format!("Failed to get system time: {e}"),
            })
        })?;
        let result = self.vm.execute(compiled, context)?;

        if let Some(_start) = start_time {
            let eval_end = self.timestamp_service.system_time().map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Failed to get system time: {e}"),
                })
            })?;
            let eval_duration = eval_end.duration_since(eval_start).map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Time calculation error: {e}"),
                })
            })?;
            let mut metrics = self.metrics.write().map_err(|e| {
                ExpressionError::Other(format!("metrics lock should not be poisoned: {e}"))
            })?;
            metrics.compiled_evaluations += 1;
            metrics.eval_time_us += eval_duration.as_micros() as u64;
        }

        Ok(result)
    }

    /// Evaluate using the interpreter
    fn evaluate_interpreted(
        &self,
        ast: &Expression,
        context: &HashMap<String, Value>,
        start_time: Option<SystemTime>,
    ) -> Result<Value, ExpressionError> {
        let eval_start = self.timestamp_service.system_time().map_err(|e| {
            ExpressionError::Parse(ParseError::SystemError {
                message: format!("Failed to get system time: {e}"),
            })
        })?;
        let result = self
            .evaluator
            .evaluate(ast, context)
            .map_err(ExpressionError::Evaluation)?;

        if let Some(_start) = start_time {
            let eval_end = self.timestamp_service.system_time().map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Failed to get system time: {e}"),
                })
            })?;
            let eval_duration = eval_end.duration_since(eval_start).map_err(|e| {
                ExpressionError::Parse(ParseError::SystemError {
                    message: format!("Time calculation error: {e}"),
                })
            })?;
            let mut metrics = self.metrics.write().map_err(|e| {
                ExpressionError::Other(format!("metrics lock should not be poisoned: {e}"))
            })?;
            metrics.interpreted_evaluations += 1;
            metrics.eval_time_us += eval_duration.as_micros() as u64;
        }

        Ok(result)
    }

    /// Get performance metrics
    #[must_use]
    pub fn metrics(&self) -> PerformanceMetrics {
        self.metrics
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Clear the expression cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Prune old cache entries
    pub fn prune_cache(&self) {
        self.cache.prune_old_entries();
    }

    /// Pre-compile an expression for later use
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn precompile(
        &self,
        expression: &str,
        schema_id: Option<&str>,
    ) -> Result<(), ExpressionError> {
        let key = ExpressionKey {
            source: expression.to_string(),
            schema_id: schema_id.map(std::string::ToString::to_string),
        };

        // Check if already cached
        if self.cache.get(&key).is_some() {
            return Ok(());
        }

        // Parse and compile
        let (ast, compiled) = self.parse_and_compile(expression, None)?;

        // Cache the result
        self.cache.insert(key, ast, compiled);

        Ok(())
    }

    /// Batch evaluate multiple expressions
    #[must_use]
    pub fn batch_evaluate(
        &self,
        expressions: &[(String, HashMap<String, Value>)],
    ) -> Vec<Result<Value, ExpressionError>> {
        expressions
            .iter()
            .map(|(expr, ctx)| self.evaluate(expr, ctx))
            .collect()
    }
}

/// Builder for creating configured expression engines
pub struct EngineBuilder {
    config: EngineConfig,
    function_registry: Option<Arc<FunctionRegistry>>,
}

impl EngineBuilder {
    /// Create a new builder with default config
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: EngineConfig::default(),
            function_registry: None,
        }
    }

    /// Set whether to use compilation
    #[must_use]
    pub fn use_compilation(mut self, enabled: bool) -> Self {
        self.config.use_compilation = enabled;
        self
    }

    /// Set whether to use caching
    #[must_use]
    pub fn use_caching(mut self, enabled: bool) -> Self {
        self.config.use_caching = enabled;
        self
    }

    /// Set cache capacity
    #[must_use]
    pub fn cache_capacity(mut self, capacity: usize) -> Self {
        self.config.cache_capacity = capacity;
        self
    }

    /// Set optimization level (0-3)
    #[must_use]
    pub fn optimization_level(mut self, level: u8) -> Self {
        self.config.optimization_level = level.min(3);
        self
    }

    /// Set compilation threshold
    #[must_use]
    pub fn compilation_threshold(mut self, threshold: u64) -> Self {
        self.config.compilation_threshold = threshold;
        self
    }

    /// Set custom function registry
    #[must_use]
    pub fn with_function_registry(mut self, registry: Arc<FunctionRegistry>) -> Self {
        self.function_registry = Some(registry);
        self
    }

    /// Enable metrics collection
    #[must_use]
    pub fn collect_metrics(mut self, enabled: bool) -> Self {
        self.config.collect_metrics = enabled;
        self
    }

    /// Build the engine
    #[must_use]
    pub fn build(self) -> ExpressionEngineV2 {
        if let Some(registry) = self.function_registry {
            ExpressionEngineV2::with_function_registry(self.config, registry)
        } else {
            ExpressionEngineV2::new(self.config)
        }
    }
}

impl Default for EngineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_evaluation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let engine = EngineBuilder::new().build();
        let context = HashMap::new();

        let result = engine
            .evaluate("1 + 2 * 3", &context)
            .expect("should evaluate simple expression: {}");
        assert_eq!(result, Value::Number(serde_json::Number::from(7)));
        Ok(())
    }

    #[test]
    fn test_caching_performance() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let engine = EngineBuilder::new().collect_metrics(true).build();

        let context = HashMap::new();
        let expr = "1 + 2 + 3 + 4 + 5";

        // First evaluation - cache miss
        engine
            .evaluate(expr, &context)
            .expect("should evaluate expression on first try: {}");

        // Subsequent evaluations - cache hits
        for _ in 0..10 {
            engine
                .evaluate(expr, &context)
                .expect("should evaluate cached expression: {}");
        }

        let metrics = engine.metrics();
        assert_eq!(metrics.total_evaluations, 11);
        assert!(metrics.cache_hit_rate > 0.9);
        Ok(())
    }

    #[test]
    fn test_compilation_threshold() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let engine = EngineBuilder::new()
            .collect_metrics(true)
            .compilation_threshold(10)
            .build();

        let context = HashMap::new();

        // Simple expression - should use interpreter
        engine
            .evaluate("1 + 2", &context)
            .expect("should evaluate simple expression with interpreter: {}");

        // Complex expression - should use VM
        let complex = "1 + 2 * 3 - 4 / 5 + 6 * 7 - 8 / 9 + 10";
        engine
            .evaluate(complex, &context)
            .expect("should evaluate complex expression with VM: {}");

        let metrics = engine.metrics();
        assert_eq!(metrics.interpreted_evaluations, 1);
        assert_eq!(metrics.compiled_evaluations, 1);
        Ok(())
    }
}
