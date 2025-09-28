//! Expression evaluator for `LinkML` expressions

#![allow(missing_docs)]

use super::ast::Expression;
use super::error::EvaluationError;
use super::functions::FunctionRegistry;
use lru::LruCache;
use serde_json::Value;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for the evaluator
#[derive(Debug, Clone)]
pub struct EvaluatorConfig {
    /// Maximum iterations allowed
    pub max_iterations: usize,
    /// Maximum call depth
    pub max_call_depth: usize,
    /// Evaluation timeout
    pub timeout: Duration,
    /// Maximum memory usage (approximate)
    pub max_memory: usize,
    /// Enable expression result caching
    pub enable_cache: bool,
    /// Maximum cache size (number of entries)
    pub cache_size: usize,
}

impl Default for EvaluatorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10_000,
            max_call_depth: 100,
            timeout: Duration::from_secs(1),
            max_memory: 10 * 1024 * 1024, // 10MB
            enable_cache: true,
            cache_size: 1000,
        }
    }
}

/// Cache key for expression results
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
struct CacheKey {
    /// Serialized expression AST
    expression_hash: u64,
    /// Serialized context hash
    context_hash: u64,
}

impl CacheKey {
    fn new(expr: &Expression, context: &HashMap<String, Value>) -> Self {
        let expression_hash = hash_expression(expr);
        let context_hash = hash_context(context);

        Self {
            expression_hash,
            context_hash,
        }
    }
}

/// Hash an expression securely without string formatting
fn hash_expression(expr: &Expression) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Hash based on expression type
    match expr {
        Expression::Null => 0u8.hash(&mut hasher),
        Expression::Boolean(b) => {
            1u8.hash(&mut hasher);
            b.hash(&mut hasher);
        }
        Expression::Number(n) => {
            2u8.hash(&mut hasher);
            n.to_bits().hash(&mut hasher);
        }
        Expression::String(s) => {
            3u8.hash(&mut hasher);
            s.hash(&mut hasher);
        }
        Expression::Variable(name) => {
            4u8.hash(&mut hasher);
            name.hash(&mut hasher);
        }
        Expression::Add(l, r) => {
            5u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Subtract(l, r) => {
            6u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Multiply(l, r) => {
            7u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Divide(l, r) => {
            8u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Modulo(l, r) => {
            9u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Negate(e) => {
            10u8.hash(&mut hasher);
            hash_expression(e).hash(&mut hasher);
        }
        Expression::Equal(l, r) => {
            11u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::NotEqual(l, r) => {
            12u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Less(l, r) => {
            13u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Greater(l, r) => {
            14u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::LessOrEqual(l, r) => {
            15u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::GreaterOrEqual(l, r) => {
            16u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::And(l, r) => {
            17u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Or(l, r) => {
            18u8.hash(&mut hasher);
            hash_expression(l).hash(&mut hasher);
            hash_expression(r).hash(&mut hasher);
        }
        Expression::Not(e) => {
            19u8.hash(&mut hasher);
            hash_expression(e).hash(&mut hasher);
        }
        Expression::FunctionCall { name, args } => {
            20u8.hash(&mut hasher);
            name.hash(&mut hasher);
            for arg in args {
                hash_expression(arg).hash(&mut hasher);
            }
        }
        Expression::Conditional {
            condition,
            then_expr,
            else_expr,
        } => {
            21u8.hash(&mut hasher);
            hash_expression(condition).hash(&mut hasher);
            hash_expression(then_expr).hash(&mut hasher);
            hash_expression(else_expr).hash(&mut hasher);
        }
    }

    hasher.finish()
}

/// Hash a context securely without string formatting
fn hash_context(context: &HashMap<String, Value>) -> u64 {
    let mut hasher = DefaultHasher::new();

    // Sort keys for consistent hashing
    let mut keys: Vec<_> = context.keys().collect();
    keys.sort();

    for key in keys {
        key.hash(&mut hasher);
        if let Some(value) = context.get(key) {
            hash_value(value, &mut hasher);
        }
    }

    hasher.finish()
}

/// Hash a `JSON` value securely
fn hash_value<H: Hasher>(value: &Value, hasher: &mut H) {
    match value {
        Value::Null => 0u8.hash(hasher),
        Value::Bool(b) => {
            1u8.hash(hasher);
            b.hash(hasher);
        }
        Value::Number(n) => {
            2u8.hash(hasher);
            if let Some(i) = n.as_i64() {
                i.hash(hasher);
            } else if let Some(f) = n.as_f64() {
                f.to_bits().hash(hasher);
            }
        }
        Value::String(s) => {
            3u8.hash(hasher);
            s.hash(hasher);
        }
        Value::Array(arr) => {
            4u8.hash(hasher);
            arr.len().hash(hasher);
            for v in arr {
                hash_value(v, hasher);
            }
        }
        Value::Object(map) => {
            5u8.hash(hasher);
            map.len().hash(hasher);
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                key.hash(hasher);
                if let Some(v) = map.get(key) {
                    hash_value(v, hasher);
                }
            }
        }
    }
}

/// Expression evaluator with safety limits
pub struct Evaluator {
    config: EvaluatorConfig,
    function_registry: FunctionRegistry,
    cache: Option<Arc<Mutex<LruCache<CacheKey, Value>>>>,
}

impl Evaluator {
    /// Create a new evaluator with default configuration.
    ///
    /// # Panics
    ///
    /// Panics if the configured cache size cannot be converted into a non-zero
    /// value.
    #[must_use]
    pub fn new() -> Self {
        let config = EvaluatorConfig::default();
        let cache = if config.enable_cache {
            let cache_size = NonZeroUsize::new(config.cache_size)
                .or_else(|| NonZeroUsize::new(1000))
                .expect("Cache size of 1000 should always be valid");
            Some(Arc::new(Mutex::new(LruCache::new(cache_size))))
        } else {
            None
        };

        Self {
            config,
            function_registry: FunctionRegistry::new(),
            cache,
        }
    }

    /// Create a new evaluator with a custom function registry.
    ///
    /// # Panics
    ///
    /// Panics if the configured cache size cannot be converted into a non-zero
    /// value.
    #[must_use]
    pub fn with_functions(function_registry: FunctionRegistry) -> Self {
        let config = EvaluatorConfig::default();
        let cache = if config.enable_cache {
            let cache_size = NonZeroUsize::new(config.cache_size)
                .or_else(|| NonZeroUsize::new(1000))
                .expect("Cache size of 1000 should always be valid");
            Some(Arc::new(Mutex::new(LruCache::new(cache_size))))
        } else {
            None
        };

        Self {
            config,
            function_registry,
            cache,
        }
    }

    /// Get mutable reference to function registry for custom function registration
    pub fn function_registry_mut(&mut self) -> &mut FunctionRegistry {
        &mut self.function_registry
    }

    /// Clear the expression cache
    pub fn clear_cache(&self) {
        if let Some(cache) = &self.cache
            && let Ok(mut cache) = cache.lock()
        {
            cache.clear();
        }
    }

    /// Get cache statistics
    #[must_use]
    pub fn cache_stats(&self) -> Option<(usize, usize)> {
        if let Some(cache) = &self.cache
            && let Ok(cache) = cache.lock()
        {
            return Some((cache.len(), cache.cap().into()));
        }
        None
    }

    /// Create an evaluator with custom configuration.
    ///
    /// # Panics
    ///
    /// Panics if the configured cache size cannot be converted into a non-zero
    /// value.
    #[must_use]
    pub fn with_config(config: EvaluatorConfig) -> Self {
        let cache = if config.enable_cache {
            let cache_size = NonZeroUsize::new(config.cache_size)
                .or_else(|| NonZeroUsize::new(1000))
                .expect("Cache size of 1000 should always be valid");
            Some(Arc::new(Mutex::new(LruCache::new(cache_size))))
        } else {
            None
        };

        Self {
            config,
            function_registry: FunctionRegistry::new(),
            cache,
        }
    }

    /// Evaluate an expression with the given variable context
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate(
        &self,
        expr: &Expression,
        context: &HashMap<String, Value>,
    ) -> Result<Value, EvaluationError> {
        // Check cache first if enabled
        if let Some(cache) = &self.cache {
            let cache_key = CacheKey::new(expr, context);

            // Try to get from cache
            if let Ok(mut cache) = cache.lock()
                && let Some(cached_value) = cache.get(&cache_key)
            {
                return Ok(cached_value.clone());
            }

            // Evaluate the expression
            let mut eval_context = EvalContext {
                variables: context,
                iterations: 0,
                call_depth: 0,
                start_time: Instant::now(),
                memory_used: 0,
                config: &self.config,
                functions: &self.function_registry,
            };

            let result = eval_context.evaluate_expr(expr)?;

            // Cache the result
            if let Ok(mut cache) = cache.lock() {
                cache.put(cache_key, result.clone());
            }

            Ok(result)
        } else {
            // No caching, evaluate directly
            let mut eval_context = EvalContext {
                variables: context,
                iterations: 0,
                call_depth: 0,
                start_time: Instant::now(),
                memory_used: 0,
                config: &self.config,
                functions: &self.function_registry,
            };

            eval_context.evaluate_expr(expr)
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal evaluation context
struct EvalContext<'a> {
    variables: &'a HashMap<String, Value>,
    iterations: usize,
    call_depth: usize,
    start_time: Instant,
    memory_used: usize,
    config: &'a EvaluatorConfig,
    functions: &'a FunctionRegistry,
}

impl EvalContext<'_> {
    fn check_limits(&mut self) -> Result<(), EvaluationError> {
        // Check iteration limit
        self.iterations += 1;
        if self.iterations > self.config.max_iterations {
            return Err(EvaluationError::TooManyIterations {
                max: self.config.max_iterations,
            });
        }

        // Check timeout
        let elapsed = self.start_time.elapsed();
        if elapsed > self.config.timeout {
            return Err(EvaluationError::Timeout {
                seconds: elapsed.as_secs_f64(),
            });
        }

        // Check call depth
        if self.call_depth > self.config.max_call_depth {
            return Err(EvaluationError::CallStackTooDeep {
                max: self.config.max_call_depth,
            });
        }

        // Check memory (approximate)
        if self.memory_used > self.config.max_memory {
            return Err(EvaluationError::MemoryLimitExceeded {
                limit: self.config.max_memory,
            });
        }

        Ok(())
    }

    fn evaluate_expr(&mut self, expr: &Expression) -> Result<Value, EvaluationError> {
        self.check_limits()?;
        self.call_depth += 1;

        let result = match expr {
            Expression::Null => Ok(Value::Null),
            Expression::Boolean(b) => Ok(Value::Bool(*b)),
            Expression::Number(n) => Ok(Value::Number(
                serde_json::Number::from_f64(*n).ok_or(EvaluationError::NumericOverflow)?,
            )),
            Expression::String(s) => {
                self.memory_used += s.len();
                Ok(Value::String(s.clone()))
            }
            Expression::Variable(name) => {
                // Handle dot notation for nested access
                if name.contains('.') {
                    let parts: Vec<&str> = name.split('.').collect();
                    let mut current = self.variables.get(parts[0]).ok_or_else(|| {
                        EvaluationError::UndefinedVariable {
                            name: parts[0].to_string(),
                        }
                    })?;

                    for part in &parts[1..] {
                        match current {
                            Value::Object(map) => {
                                current = map.get(*part).ok_or_else(|| {
                                    EvaluationError::UndefinedVariable {
                                        name: format!("{part} (in {name})"),
                                    }
                                })?;
                            }
                            _ => {
                                return Err(EvaluationError::TypeError {
                                    message: format!(
                                        "Cannot access property '{part}' on non-object"
                                    ),
                                });
                            }
                        }
                    }
                    Ok(current.clone())
                } else {
                    self.variables
                        .get(name)
                        .cloned()
                        .ok_or_else(|| EvaluationError::UndefinedVariable { name: name.clone() })
                }
            }

            Expression::Add(left, right) => self.evaluate_add(left, right),
            Expression::Subtract(left, right) => self.evaluate_subtract(left, right),
            Expression::Multiply(left, right) => self.evaluate_multiply(left, right),
            Expression::Divide(left, right) => self.evaluate_divide(left, right),
            Expression::Modulo(left, right) => self.evaluate_modulo(left, right),

            Expression::Negate(expr) => self.evaluate_negate(expr),

            Expression::Equal(left, right) => self.evaluate_equal(left, right),
            Expression::NotEqual(left, right) => self.evaluate_not_equal(left, right),
            Expression::Less(left, right) => self.evaluate_less(left, right),
            Expression::Greater(left, right) => self.evaluate_greater(left, right),
            Expression::LessOrEqual(left, right) => self.evaluate_less_or_equal(left, right),
            Expression::GreaterOrEqual(left, right) => self.evaluate_greater_or_equal(left, right),

            Expression::And(left, right) => self.evaluate_and(left, right),
            Expression::Or(left, right) => self.evaluate_or(left, right),
            Expression::Not(expr) => self.evaluate_not(expr),

            Expression::FunctionCall { name, args } => self.evaluate_function_call(name, args),

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => self.evaluate_conditional(condition, then_expr, else_expr),
        };

        self.call_depth -= 1;
        result
    }

    // Arithmetic operations

    fn evaluate_add(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                let result = left_num + right_num;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            (Value::String(l), Value::String(r)) => {
                let result = format!("{l}{r}");
                self.memory_used += result.len();
                Ok(Value::String(result))
            }
            _ => Err(EvaluationError::binary_type_error(
                "add",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_subtract(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                let result = left_num - right_num;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            _ => Err(EvaluationError::binary_type_error(
                "subtract",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_multiply(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                let result = left_num * right_num;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            _ => Err(EvaluationError::binary_type_error(
                "multiply",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_divide(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let divisor = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                if divisor == 0.0 {
                    return Err(EvaluationError::DivisionByZero);
                }
                let dividend = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let result = dividend / divisor;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            _ => Err(EvaluationError::binary_type_error(
                "divide",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_modulo(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let divisor = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                if divisor == 0.0 {
                    return Err(EvaluationError::DivisionByZero);
                }
                let dividend = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let result = dividend % divisor;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            _ => Err(EvaluationError::binary_type_error(
                "modulo",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_negate(&mut self, expr: &Expression) -> Result<Value, EvaluationError> {
        let val = self.evaluate_expr(expr)?;

        match val {
            Value::Number(n) => {
                let num = n.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Operand is not a valid number".to_string(),
                })?;
                let result = -num;
                Ok(Value::Number(
                    serde_json::Number::from_f64(result).ok_or(EvaluationError::NumericOverflow)?,
                ))
            }
            _ => Err(EvaluationError::unary_type_error(
                "negate",
                &value_type_name(&val),
            )),
        }
    }

    // Comparison operations

    fn evaluate_equal(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;
        Ok(Value::Bool(values_equal(&left_val, &right_val)))
    }

    fn evaluate_not_equal(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;
        Ok(Value::Bool(!values_equal(&left_val, &right_val)))
    }

    fn evaluate_less(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                Ok(Value::Bool(left_num < right_num))
            }
            (Value::String(l), Value::String(r)) => Ok(Value::Bool(l < r)),
            _ => Err(EvaluationError::binary_type_error(
                "compare",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_greater(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                Ok(Value::Bool(left_num > right_num))
            }
            (Value::String(l), Value::String(r)) => Ok(Value::Bool(l > r)),
            _ => Err(EvaluationError::binary_type_error(
                "compare",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_less_or_equal(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                Ok(Value::Bool(left_num <= right_num))
            }
            (Value::String(l), Value::String(r)) => Ok(Value::Bool(l <= r)),
            _ => Err(EvaluationError::binary_type_error(
                "compare",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    fn evaluate_greater_or_equal(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;
        let right_val = self.evaluate_expr(right)?;

        match (&left_val, &right_val) {
            (Value::Number(l), Value::Number(r)) => {
                let left_num = l.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Left operand is not a valid number".to_string(),
                })?;
                let right_num = r.as_f64().ok_or_else(|| EvaluationError::TypeError {
                    message: "Right operand is not a valid number".to_string(),
                })?;
                Ok(Value::Bool(left_num >= right_num))
            }
            (Value::String(l), Value::String(r)) => Ok(Value::Bool(l >= r)),
            _ => Err(EvaluationError::binary_type_error(
                "compare",
                &value_type_name(&left_val),
                &value_type_name(&right_val),
            )),
        }
    }

    // Logical operations

    fn evaluate_and(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;

        // Short-circuit evaluation
        if !is_truthy(&left_val) {
            return Ok(Value::Bool(false));
        }

        let right_val = self.evaluate_expr(right)?;
        Ok(Value::Bool(is_truthy(&right_val)))
    }

    fn evaluate_or(
        &mut self,
        left: &Expression,
        right: &Expression,
    ) -> Result<Value, EvaluationError> {
        let left_val = self.evaluate_expr(left)?;

        // Short-circuit evaluation
        if is_truthy(&left_val) {
            return Ok(Value::Bool(true));
        }

        let right_val = self.evaluate_expr(right)?;
        Ok(Value::Bool(is_truthy(&right_val)))
    }

    fn evaluate_not(&mut self, expr: &Expression) -> Result<Value, EvaluationError> {
        let val = self.evaluate_expr(expr)?;
        Ok(Value::Bool(!is_truthy(&val)))
    }

    // Function calls

    fn evaluate_function_call(
        &mut self,
        name: &str,
        args: &[Expression],
    ) -> Result<Value, EvaluationError> {
        // Evaluate arguments
        let mut arg_values = Vec::with_capacity(args.len());
        for arg in args {
            arg_values.push(self.evaluate_expr(arg)?);
        }

        // Call function
        self.functions
            .call(name, arg_values)
            .map_err(|e| EvaluationError::FunctionError {
                name: name.to_string(),
                message: e.to_string(),
            })
    }

    // Conditional

    fn evaluate_conditional(
        &mut self,
        condition: &Expression,
        then_expr: &Expression,
        else_expr: &Expression,
    ) -> Result<Value, EvaluationError> {
        let condition_val = self.evaluate_expr(condition)?;

        if is_truthy(&condition_val) {
            self.evaluate_expr(then_expr)
        } else {
            self.evaluate_expr(else_expr)
        }
    }
}

// Helper functions

fn value_type_name(value: &Value) -> String {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
    .to_string()
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(l), Value::Bool(r)) => l == r,
        (Value::Number(l), Value::Number(r)) => l.as_f64() == r.as_f64(),
        (Value::String(l), Value::String(r)) => l == r,
        _ => false,
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::ast::Expression;
    use serde_json::json;

    #[test]
    fn test_evaluate_literals() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let context = HashMap::new();

        assert_eq!(
            evaluator
                .evaluate(&Expression::Null, &context)
                .expect("Should evaluate null: {}"),
            Value::Null
        );
        assert_eq!(
            evaluator
                .evaluate(&Expression::Boolean(true), &context)
                .expect("Should evaluate boolean: {}"),
            Value::Bool(true)
        );
        assert_eq!(
            evaluator
                .evaluate(&Expression::Number(42.0), &context)
                .expect("Should evaluate number: {}"),
            json!(42.0)
        );
        assert_eq!(
            evaluator
                .evaluate(&Expression::String("hello".to_string()), &context)
                .expect("Should evaluate string: {}"),
            json!("hello")
        );
        Ok(())
    }

    #[test]
    fn test_evaluate_variables() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let mut context = HashMap::new();
        context.insert("x".to_string(), json!(10));
        context.insert("name".to_string(), json!("Alice"));

        assert_eq!(
            evaluator
                .evaluate(&Expression::Variable("x".to_string()), &context)
                .expect("Should evaluate variable x: {}"),
            json!(10)
        );
        assert_eq!(
            evaluator
                .evaluate(&Expression::Variable("name".to_string()), &context)
                .expect("Should evaluate variable name: {}"),
            json!("Alice")
        );

        // Undefined variable
        assert!(matches!(
            evaluator.evaluate(&Expression::Variable("undefined".to_string()), &context),
            Err(EvaluationError::UndefinedVariable { .. })
        ));
        Ok(())
    }

    #[test]
    fn test_evaluate_arithmetic() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let context = HashMap::new();

        // 2 + 3 = 5
        let expr = Expression::Add(
            Box::new(Expression::Number(2.0)),
            Box::new(Expression::Number(3.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(5.0)
        );

        // 10 - 4 = 6
        let expr = Expression::Subtract(
            Box::new(Expression::Number(10.0)),
            Box::new(Expression::Number(4.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(6.0)
        );

        // 3 * 4 = 12
        let expr = Expression::Multiply(
            Box::new(Expression::Number(3.0)),
            Box::new(Expression::Number(4.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(12.0)
        );

        // 15 / 3 = 5
        let expr = Expression::Divide(
            Box::new(Expression::Number(15.0)),
            Box::new(Expression::Number(3.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(5.0)
        );

        // Division by zero
        let expr = Expression::Divide(
            Box::new(Expression::Number(10.0)),
            Box::new(Expression::Number(0.0)),
        );
        assert!(matches!(
            evaluator.evaluate(&expr, &context),
            Err(EvaluationError::DivisionByZero)
        ));
        Ok(())
    }

    #[test]
    fn test_evaluate_comparison() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let context = HashMap::new();

        // 5 > 3 = true
        let expr = Expression::Greater(
            Box::new(Expression::Number(5.0)),
            Box::new(Expression::Number(3.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(true)
        );

        // 2 < 2 = false
        let expr = Expression::Less(
            Box::new(Expression::Number(2.0)),
            Box::new(Expression::Number(2.0)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(false)
        );

        // "abc" == "abc" = true
        let expr = Expression::Equal(
            Box::new(Expression::String("abc".to_string())),
            Box::new(Expression::String("abc".to_string())),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(true)
        );
        Ok(())
    }

    #[test]
    fn test_evaluate_logical() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let context = HashMap::new();

        // true and false = false
        let expr = Expression::And(
            Box::new(Expression::Boolean(true)),
            Box::new(Expression::Boolean(false)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(false)
        );

        // true or false = true
        let expr = Expression::Or(
            Box::new(Expression::Boolean(true)),
            Box::new(Expression::Boolean(false)),
        );
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(true)
        );

        // not true = false
        let expr = Expression::Not(Box::new(Expression::Boolean(true)));
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!(false)
        );
        Ok(())
    }

    #[test]
    fn test_evaluate_conditional() -> Result<(), anyhow::Error> {
        let evaluator = Evaluator::new();
        let mut context = HashMap::new();
        context.insert("x".to_string(), json!(10));

        // "big" if x > 5 else "small"
        let expr = Expression::Conditional {
            condition: Box::new(Expression::Greater(
                Box::new(Expression::Variable("x".to_string())),
                Box::new(Expression::Number(5.0)),
            )),
            then_expr: Box::new(Expression::String("big".to_string())),
            else_expr: Box::new(Expression::String("small".to_string())),
        };

        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!("big")
        );

        // Change x to 3
        context.insert("x".to_string(), json!(3));
        assert_eq!(
            evaluator
                .evaluate(&expr, &context)
                .expect("Test evaluation should succeed: {}"),
            json!("small")
        );
        Ok(())
    }
}
