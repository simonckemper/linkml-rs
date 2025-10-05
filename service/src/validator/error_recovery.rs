//! Comprehensive error recovery for `LinkML` validation
//!
//! This module provides robust error recovery mechanisms including:
//! - Graceful degradation for partial failures
//! - Circuit breaker pattern for external dependencies
//! - Retry strategies with exponential backoff
//! - Error context preservation
//! - Recovery suggestions

use dashmap::DashMap;
use linkml_core::{LinkMLError, Result};
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Error recovery configuration
#[derive(Debug, Clone)]
pub struct ErrorRecoveryConfig {
    /// Enable circuit breaker
    pub circuit_breaker_enabled: bool,
    /// Circuit breaker failure threshold
    pub failure_threshold: u32,
    /// Circuit breaker recovery timeout
    pub recovery_timeout: Duration,
    /// Enable retry on recoverable errors
    pub retry_enabled: bool,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Initial retry delay
    pub initial_retry_delay: Duration,
    /// Retry backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum retry delay
    pub max_retry_delay: Duration,
    /// Enable partial results
    pub allow_partial_results: bool,
}

impl Default for ErrorRecoveryConfig {
    fn default() -> Self {
        Self {
            circuit_breaker_enabled: true,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            retry_enabled: true,
            max_retries: 3,
            initial_retry_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_retry_delay: Duration::from_secs(10),
            allow_partial_results: true,
        }
    }
}

/// Recoverable error types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecoverableErrorType {
    /// Network timeout
    NetworkTimeout,
    /// Cache unavailable
    CacheUnavailable,
    /// Resource temporarily unavailable
    ResourceBusy,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Partial data available
    PartialData,
    /// Schema not cached
    SchemaNotCached,
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
enum CircuitState {
    /// Circuit is closed (normal operation)
    Closed,
    /// Circuit is open (failing fast)
    Open,
    /// Circuit is half-open (testing recovery)
    HalfOpen,
}

/// Circuit breaker for a specific service
struct CircuitBreaker {
    state: CircuitState,
    failure_count: u32,
    last_failure: Option<Instant>,
    success_count: u32,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: None,
            success_count: 0,
        }
    }
}

/// Error context with recovery information
#[derive(Debug)]
pub struct ErrorContext {
    /// Original error
    pub error: LinkMLError,
    /// Error type if recoverable
    pub error_type: Option<RecoverableErrorType>,
    /// Retry count
    pub retry_count: u32,
    /// Recovery suggestions
    pub suggestions: SmallVec<[String; 4]>,
    /// Partial results if available
    pub partial_results: Option<serde_json::Value>,
    /// Timestamp
    pub timestamp: Instant,
}

impl ErrorContext {
    /// Create new error context
    #[must_use]
    pub fn new(error: LinkMLError) -> Self {
        Self {
            error,
            error_type: None,
            retry_count: 0,
            suggestions: SmallVec::new(),
            partial_results: None,
            timestamp: Instant::now(),
        }
    }

    /// Add a recovery suggestion
    pub fn add_suggestion(&mut self, suggestion: impl Into<String>) {
        self.suggestions.push(suggestion.into());
    }

    /// Check if error is recoverable
    #[must_use]
    pub fn is_recoverable(&self) -> bool {
        self.error_type.is_some()
    }
}

/// Error recovery manager
pub struct ErrorRecoveryManager {
    config: Arc<RwLock<ErrorRecoveryConfig>>,
    circuit_breakers: DashMap<String, RwLock<CircuitBreaker>>,
    error_history: Arc<RwLock<Vec<ErrorContext>>>,
    recovery_strategies: DashMap<RecoverableErrorType, Box<dyn RecoveryStrategy>>,
}

/// Recovery strategy trait
pub trait RecoveryStrategy: Send + Sync {
    /// Attempt to recover from error
    ///
    /// # Errors
    ///
    /// Returns an error if recovery fails.
    fn recover(&self, context: &ErrorContext) -> Result<RecoveryAction>;

    /// Check if recovery is possible
    fn can_recover(&self, context: &ErrorContext) -> bool;
}

/// Recovery actions
#[derive(Debug, Clone)]
pub enum RecoveryAction {
    /// Retry the operation
    Retry {
        /// Delay before retry
        delay: Duration,
    },
    /// Use fallback value
    Fallback {
        /// Fallback value to use
        value: serde_json::Value,
    },
    /// Skip and continue
    Skip,
    /// Use cached value
    UseCache {
        /// Cache key to use
        key: String,
    },
    /// Degrade to partial functionality
    Degrade {
        /// Feature to degrade
        feature: String,
    },
    /// Fail permanently
    Fail,
}

impl ErrorRecoveryManager {
    /// Create new error recovery manager
    #[must_use]
    pub fn new(config: ErrorRecoveryConfig) -> Self {
        let mut manager = Self {
            config: Arc::new(RwLock::new(config)),
            circuit_breakers: DashMap::new(),
            error_history: Arc::new(RwLock::new(Vec::with_capacity(1000))),
            recovery_strategies: DashMap::new(),
        };

        // Register default recovery strategies
        manager.register_default_strategies();

        manager
    }

    /// Register default recovery strategies
    fn register_default_strategies(&mut self) {
        // Network timeout strategy
        self.recovery_strategies.insert(
            RecoverableErrorType::NetworkTimeout,
            Box::new(NetworkTimeoutStrategy),
        );

        // Cache unavailable strategy
        self.recovery_strategies.insert(
            RecoverableErrorType::CacheUnavailable,
            Box::new(CacheUnavailableStrategy),
        );

        // Rate limit strategy
        self.recovery_strategies.insert(
            RecoverableErrorType::RateLimitExceeded,
            Box::new(RateLimitStrategy),
        );
    }

    /// Handle an error with recovery
    ///
    /// # Errors
    ///
    /// Returns an error if recovery attempts fail.
    pub async fn handle_error<F, T>(
        &self,
        service_name: &str,
        error: LinkMLError,
        operation: F,
    ) -> Result<T>
    where
        F: Fn() -> Result<T> + Clone + Send,
        T: Send,
    {
        let mut context = Self::analyze_error(error);

        // Check circuit breaker
        if self.config.read().circuit_breaker_enabled
            && let Some(breaker) = self.circuit_breakers.get(service_name)
        {
            let mut breaker = breaker.write();
            if breaker.state == CircuitState::Open {
                // Check if we should transition to half-open
                if let Some(last_failure) = breaker.last_failure {
                    if last_failure.elapsed() > self.config.read().recovery_timeout {
                        breaker.state = CircuitState::HalfOpen;
                        breaker.success_count = 0;
                    } else {
                        return Err(LinkMLError::service(format!(
                            "Circuit breaker open for {service_name}"
                        )));
                    }
                }
            }
        }

        // Try recovery strategies
        if context.is_recoverable()
            && self.config.read().retry_enabled
            && let Some(error_type) = &context.error_type
            && let Some(strategy) = self.recovery_strategies.get(error_type)
        {
            match strategy.recover(&context)? {
                RecoveryAction::Retry { delay } => {
                    // Implement retry with backoff
                    let max_retries = self.config.read().max_retries;
                    while context.retry_count < max_retries {
                        tokio::time::sleep(delay).await;

                        match operation() {
                            Ok(result) => {
                                self.record_success(service_name);
                                return Ok(result);
                            }
                            Err(e) => {
                                context.retry_count += 1;
                                context.error = e;

                                if context.retry_count >= max_retries {
                                    break;
                                }
                            }
                        }
                    }
                }
                RecoveryAction::Fallback { value: _ } => {
                    // Use fallback value (requires type conversion)
                    self.record_success(service_name);
                    return Err(LinkMLError::service(
                        "Fallback values not yet implemented for this type",
                    ));
                }
                RecoveryAction::Skip => {
                    // Skip this operation
                    return Err(context.error);
                }
                RecoveryAction::UseCache { key } => {
                    // Try to use cached value
                    return Err(LinkMLError::service(format!(
                        "Cache fallback not available for key: {key}"
                    )));
                }
                RecoveryAction::Degrade { feature } => {
                    // Degrade functionality
                    tracing::warn!("Degrading functionality: {}", feature);
                    return Err(context.error);
                }
                RecoveryAction::Fail => {
                    // Fail permanently
                    self.record_failure(service_name);
                    return Err(context.error);
                }
            }
        }

        // Record failure and return error
        self.record_failure(service_name);
        self.record_error_context(context);
        Err(LinkMLError::service(
            "Operation failed after recovery attempts",
        ))
    }

    /// Analyze error to determine if it's recoverable
    fn analyze_error(error: LinkMLError) -> ErrorContext {
        // Analyze error message for patterns
        let error_string = error.to_string();
        let mut context = ErrorContext::new(error);

        if error_string.contains("timeout") || error_string.contains("timed out") {
            context.error_type = Some(RecoverableErrorType::NetworkTimeout);
            context.add_suggestion("Increase timeout duration");
            context.add_suggestion("Check network connectivity");
        } else if error_string.contains("cache") && error_string.contains("unavailable") {
            context.error_type = Some(RecoverableErrorType::CacheUnavailable);
            context.add_suggestion("Check cache service status");
            context.add_suggestion("Use direct validation without cache");
        } else if error_string.contains("rate limit") {
            context.error_type = Some(RecoverableErrorType::RateLimitExceeded);
            context.add_suggestion("Implement request throttling");
            context.add_suggestion("Use batch operations");
        } else if error_string.contains("resource busy") || error_string.contains("locked") {
            context.error_type = Some(RecoverableErrorType::ResourceBusy);
            context.add_suggestion("Retry after short delay");
            context.add_suggestion("Check for deadlocks");
        }

        context
    }

    /// Record successful operation
    fn record_success(&self, service_name: &str) {
        if let Some(breaker) = self.circuit_breakers.get(service_name) {
            let mut breaker = breaker.write();
            breaker.failure_count = 0;
            breaker.success_count += 1;

            if breaker.state == CircuitState::HalfOpen {
                // Transition back to closed after enough successes
                if breaker.success_count >= 3 {
                    breaker.state = CircuitState::Closed;
                    tracing::info!("Circuit breaker closed for {}", service_name);
                }
            }
        }
    }

    /// Record failed operation
    fn record_failure(&self, service_name: &str) {
        let breaker = self
            .circuit_breakers
            .entry(service_name.to_string())
            .or_insert_with(|| RwLock::new(CircuitBreaker::default()));

        let mut breaker = breaker.write();
        breaker.failure_count += 1;
        breaker.last_failure = Some(Instant::now());

        let threshold = self.config.read().failure_threshold;

        match breaker.state {
            CircuitState::Closed | CircuitState::HalfOpen => {
                if breaker.failure_count >= threshold {
                    breaker.state = CircuitState::Open;
                    tracing::warn!("Circuit breaker opened for {}", service_name);
                }
            }
            CircuitState::Open => {}
        }
    }

    /// Record error context for analysis
    fn record_error_context(&self, context: ErrorContext) {
        let mut history = self.error_history.write();

        // Keep limited history
        if history.len() >= 1000 {
            history.drain(0..100);
        }

        history.push(context);
    }

    /// Get error statistics
    #[must_use]
    pub fn get_error_stats(&self) -> ErrorStats {
        let history = self.error_history.read();
        let mut stats = ErrorStats::default();

        let now = Instant::now();
        let window = Duration::from_secs(300); // 5 minute window

        for context in history.iter() {
            if now.duration_since(context.timestamp) <= window {
                stats.total_errors += 1;

                if context.is_recoverable() {
                    stats.recoverable_errors += 1;
                }

                if let Some(error_type) = &context.error_type {
                    *stats.errors_by_type.entry(error_type.clone()).or_insert(0) += 1;
                }
            }
        }

        // Circuit breaker stats
        for entry in &self.circuit_breakers {
            let (service, breaker) = entry.pair();
            let breaker = breaker.read();

            match breaker.state {
                CircuitState::Open => stats.open_circuits.push(service.clone()),
                CircuitState::HalfOpen => stats.half_open_circuits.push(service.clone()),
                CircuitState::Closed => {}
            }
        }

        stats
    }
}

/// Error statistics
#[derive(Debug, Default)]
pub struct ErrorStats {
    /// Total errors in window
    pub total_errors: u32,
    /// Recoverable errors
    pub recoverable_errors: u32,
    /// Errors by type
    pub errors_by_type: std::collections::HashMap<RecoverableErrorType, u32>,
    /// Open circuit breakers
    pub open_circuits: Vec<String>,
    /// Half-open circuit breakers
    pub half_open_circuits: Vec<String>,
}

// Recovery strategy implementations

struct NetworkTimeoutStrategy;

impl RecoveryStrategy for NetworkTimeoutStrategy {
    fn recover(&self, context: &ErrorContext) -> Result<RecoveryAction> {
        // Calculate retry delay with exponential backoff
        let base_delay = Duration::from_millis(100);
        let delay = base_delay * 2u32.pow(context.retry_count.min(5));

        Ok(RecoveryAction::Retry { delay })
    }

    fn can_recover(&self, context: &ErrorContext) -> bool {
        context.retry_count < 3
    }
}

struct CacheUnavailableStrategy;

impl RecoveryStrategy for CacheUnavailableStrategy {
    fn recover(&self, _context: &ErrorContext) -> Result<RecoveryAction> {
        // Degrade to direct validation without cache
        Ok(RecoveryAction::Degrade {
            feature: "cache-acceleration".to_string(),
        })
    }

    fn can_recover(&self, _context: &ErrorContext) -> bool {
        true
    }
}

struct RateLimitStrategy;

impl RecoveryStrategy for RateLimitStrategy {
    fn recover(&self, context: &ErrorContext) -> Result<RecoveryAction> {
        // Calculate delay based on rate limit headers or exponential backoff
        let delay = Duration::from_secs(2u64.pow(context.retry_count.min(5)));

        Ok(RecoveryAction::Retry { delay })
    }

    fn can_recover(&self, context: &ErrorContext) -> bool {
        context.retry_count < 5
    }
}

/// Error recovery wrapper for validation operations
pub struct RecoverableValidation<T> {
    operation: Arc<dyn Fn() -> Result<T> + Send + Sync>,
    recovery_manager: Arc<ErrorRecoveryManager>,
    service_name: String,
}

impl<T> RecoverableValidation<T>
where
    T: Send + 'static,
{
    /// Create new recoverable validation
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn new(
        operation: impl Fn() -> Result<T> + Send + Sync + 'static,
        recovery_manager: Arc<ErrorRecoveryManager>,
        service_name: impl Into<String>,
    ) -> Self {
        Self {
            operation: Arc::new(operation),
            recovery_manager,
            service_name: service_name.into(),
        }
    }

    /// Execute with recovery
    ///
    /// # Errors
    ///
    /// Returns an error if operation and recovery attempts fail.
    pub async fn execute(self) -> Result<T> {
        let op = Arc::clone(&self.operation);
        match op() {
            Ok(result) => {
                self.recovery_manager.record_success(&self.service_name);
                Ok(result)
            }
            Err(error) => {
                let op_clone = Arc::clone(&self.operation);
                self.recovery_manager
                    .handle_error(&self.service_name, error, move || op_clone())
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let error = LinkMLError::service("Connection timeout");
        let mut context = ErrorContext::new(error);

        context.error_type = Some(RecoverableErrorType::NetworkTimeout);
        context.add_suggestion("Retry with longer timeout");

        assert!(context.is_recoverable());
        assert_eq!(context.suggestions.len(), 1);
    }

    #[test]
    fn test_circuit_breaker() -> anyhow::Result<()> {
        let config = ErrorRecoveryConfig {
            failure_threshold: 2,
            ..Default::default()
        };

        let manager = ErrorRecoveryManager::new(config);

        // Record failures
        manager.record_failure("test_service");
        manager.record_failure("test_service");

        // Check circuit state
        let breaker = manager
            .circuit_breakers
            .get("test_service")
            .ok_or_else(|| anyhow::anyhow!("Circuit breaker not found for test_service"))?;
        assert_eq!(breaker.read().state, CircuitState::Open);
        Ok(())
    }

    #[tokio::test]
    async fn test_retry_strategy() -> anyhow::Result<()> {
        let strategy = NetworkTimeoutStrategy;
        let mut context = ErrorContext::new(LinkMLError::service("timeout"));
        context.error_type = Some(RecoverableErrorType::NetworkTimeout);

        match strategy.recover(&context)? {
            RecoveryAction::Retry { delay } => {
                assert!(delay.as_millis() >= 100);
            }
            _ => panic!("Expected retry action"),
        }
        Ok(())
    }
}
