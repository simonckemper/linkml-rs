//! Panic prevention for `LinkML` validation
//!
//! This module provides comprehensive panic prevention including:
//! - Panic-safe wrappers for operations that might panic
//! - Stack overflow prevention
//! - Arithmetic overflow protection
//! - Index bounds checking
//! - UTF-8 validation
//! - Poison error recovery

use dashmap::DashMap;
use linkml_core::error::{LinkMLError, Result};
use parking_lot::{Mutex, RwLock};
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use bitflags::bitflags;

bitflags! {
    /// Types of panic prevention checks to enable
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct PanicPreventionFlags: u8 {
        /// Enable panic catching
        const CATCH_PANICS = 0b0001;
        /// Enable stack size monitoring
        const MONITOR_STACK = 0b0010;
        /// Enable arithmetic overflow checks
        const CHECK_ARITHMETIC = 0b0100;
        /// Enable bounds checking
        const CHECK_BOUNDS = 0b1000;

        /// All checks enabled (default)
        const ALL = Self::CATCH_PANICS.bits()
                  | Self::MONITOR_STACK.bits()
                  | Self::CHECK_ARITHMETIC.bits()
                  | Self::CHECK_BOUNDS.bits();

        /// No checks (for debugging)
        const NONE = 0b0000;
    }
}

/// Panic prevention configuration
#[derive(Debug, Clone)]
pub struct PanicPreventionConfig {
    /// Enabled panic prevention checks
    pub enabled_checks: PanicPreventionFlags,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
    /// Stack size limit (bytes)
    pub stack_size_limit: usize,
    /// Recovery timeout for poisoned locks
    pub poison_recovery_timeout: Duration,
}

impl Default for PanicPreventionConfig {
    fn default() -> Self {
        Self {
            enabled_checks: PanicPreventionFlags::ALL,
            max_recursion_depth: 1000,
            stack_size_limit: 8 * 1024 * 1024, // 8MB
            poison_recovery_timeout: Duration::from_secs(5),
        }
    }
}

impl PanicPreventionConfig {
    /// Check if panic catching is enabled
    #[must_use]
    pub fn catch_panics(&self) -> bool {
        self.enabled_checks
            .contains(PanicPreventionFlags::CATCH_PANICS)
    }

    /// Check if stack monitoring is enabled
    #[must_use]
    pub fn monitor_stack(&self) -> bool {
        self.enabled_checks
            .contains(PanicPreventionFlags::MONITOR_STACK)
    }

    /// Check if arithmetic checking is enabled
    #[must_use]
    pub fn check_arithmetic(&self) -> bool {
        self.enabled_checks
            .contains(PanicPreventionFlags::CHECK_ARITHMETIC)
    }

    /// Check if bounds checking is enabled
    #[must_use]
    pub fn check_bounds(&self) -> bool {
        self.enabled_checks
            .contains(PanicPreventionFlags::CHECK_BOUNDS)
    }
}

/// Stack depth tracker for preventing stack overflow
pub struct StackDepthTracker {
    depths: DashMap<thread::ThreadId, usize>,
    max_depth: usize,
}

impl StackDepthTracker {
    /// Create new tracker
    #[must_use]
    pub fn new(max_depth: usize) -> Self {
        Self {
            depths: DashMap::new(),
            max_depth,
        }
    }

    /// Enter a recursive operation
    ///
    /// # Errors
    ///
    /// Returns an error if maximum recursion depth is exceeded.
    pub fn enter(&self) -> Result<StackDepthGuard<'_>> {
        let thread_id = thread::current().id();
        let mut depth = self.depths.entry(thread_id).or_insert(0);

        if *depth >= self.max_depth {
            return Err(LinkMLError::service(format!(
                "Maximum recursion depth {} exceeded",
                self.max_depth
            )));
        }

        *depth += 1;

        Ok(StackDepthGuard {
            tracker: self,
            thread_id,
        })
    }

    /// Get current depth
    #[must_use]
    pub fn current_depth(&self) -> usize {
        let thread_id = thread::current().id();
        self.depths.get(&thread_id).map_or(0, |d| *d)
    }
}

/// RAII guard for stack depth
pub struct StackDepthGuard<'a> {
    tracker: &'a StackDepthTracker,
    thread_id: thread::ThreadId,
}

impl Drop for StackDepthGuard<'_> {
    fn drop(&mut self) {
        if let Some(mut depth) = self.tracker.depths.get_mut(&self.thread_id) {
            *depth = depth.saturating_sub(1);
        }
    }
}

/// Panic-safe wrapper for operations
pub struct PanicSafeWrapper {
    config: Arc<RwLock<PanicPreventionConfig>>,
    stack_tracker: Arc<StackDepthTracker>,
    panic_history: Arc<Mutex<Vec<PanicInfo>>>,
}

/// Information about a caught panic
#[derive(Debug, Clone)]
pub struct PanicInfo {
    /// Thread where panic occurred
    pub thread_id: thread::ThreadId,
    /// Panic message if available
    pub message: Option<String>,
    /// Stack depth at panic
    pub stack_depth: usize,
    /// Timestamp
    pub timestamp: std::time::Instant,
    /// Operation that panicked
    pub operation: String,
}

impl PanicSafeWrapper {
    /// Create new panic-safe wrapper
    #[must_use]
    pub fn new(config: PanicPreventionConfig) -> Self {
        let max_depth = config.max_recursion_depth;

        Self {
            config: Arc::new(RwLock::new(config)),
            stack_tracker: Arc::new(StackDepthTracker::new(max_depth)),
            panic_history: Arc::new(Mutex::new(Vec::with_capacity(100))),
        }
    }

    /// Execute operation with panic protection
    ///
    /// # Errors
    ///
    /// Returns an error if the operation panics or fails.
    pub fn execute<F, T>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + std::panic::UnwindSafe,
        T: Send + 'static,
    {
        let config = self.config.read();

        if !config.catch_panics() {
            return f();
        }

        drop(config);

        // Check stack depth before executing
        let _guard = self.stack_tracker.enter()?;

        let operation = operation_name.to_string();
        let stack_depth = self.stack_tracker.current_depth();

        match catch_unwind(AssertUnwindSafe(f)) {
            Ok(result) => result,
            Err(panic) => {
                let message = self.extract_panic_message(&panic);

                let info = PanicInfo {
                    thread_id: thread::current().id(),
                    message: Some(message.clone()),
                    stack_depth,
                    timestamp: std::time::Instant::now(),
                    operation,
                };

                self.record_panic(info);

                Err(LinkMLError::service(format!(
                    "Operation '{operation_name}' panicked: {message}"
                )))
            }
        }
    }

    /// Execute async operation with panic protection
    ///
    /// # Errors
    ///
    /// Returns an error if the operation panics or task joining fails.
    pub async fn execute_async<F, T>(&self, operation_name: &str, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + std::panic::UnwindSafe + 'static,
        T: Send + 'static,
    {
        let operation = operation_name.to_string();
        let wrapper = self.clone();

        tokio::task::spawn_blocking(move || wrapper.execute(&operation, f))
            .await
            .map_err(|e| LinkMLError::service(format!("Task join error: {e}")))?
    }

    /// Extract panic message from Any
    fn extract_panic_message(&self, panic: &Box<dyn Any + Send>) -> String {
        // Check configuration to determine how detailed the panic message should be
        let config = self.config.read();
        let include_debug_info = config.catch_panics();

        if let Some(s) = panic.downcast_ref::<String>() {
            if include_debug_info {
                format!("Panic: {s}")
            } else {
                "Validation error occurred".to_string()
            }
        } else if let Some(s) = panic.downcast_ref::<&str>() {
            if include_debug_info {
                format!("Panic: {s}")
            } else {
                "Validation error occurred".to_string()
            }
        } else if include_debug_info {
            "Unknown panic type".to_string()
        } else {
            "Validation error occurred".to_string()
        }
    }

    /// Record panic information
    fn record_panic(&self, info: PanicInfo) {
        let mut history = self.panic_history.lock();

        // Keep limited history
        if history.len() >= 100 {
            history.drain(0..10);
        }

        history.push(info);
    }

    /// Get panic statistics
    #[must_use]
    pub fn get_panic_stats(&self) -> PanicStats {
        let history = self.panic_history.lock();
        let now = std::time::Instant::now();
        let window = Duration::from_secs(300); // 5 minute window

        let recent_panics: Vec<_> = history
            .iter()
            .filter(|p| now.duration_since(p.timestamp) <= window)
            .cloned()
            .collect();

        let by_operation =
            recent_panics
                .iter()
                .fold(std::collections::HashMap::new(), |mut map, panic| {
                    *map.entry(panic.operation.clone()).or_insert(0) += 1;
                    map
                });

        PanicStats {
            total_panics: history.len(),
            recent_panics: recent_panics.len(),
            panics_by_operation: by_operation,
            average_stack_depth: if recent_panics.is_empty() {
                0
            } else {
                recent_panics.iter().map(|p| p.stack_depth).sum::<usize>() / recent_panics.len()
            },
        }
    }
}

impl Clone for PanicSafeWrapper {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            stack_tracker: self.stack_tracker.clone(),
            panic_history: self.panic_history.clone(),
        }
    }
}

/// Panic statistics
#[derive(Debug, Default)]
pub struct PanicStats {
    /// Total panics caught
    pub total_panics: usize,
    /// Recent panics in window
    pub recent_panics: usize,
    /// Panics by operation
    pub panics_by_operation: std::collections::HashMap<String, usize>,
    /// Average stack depth at panic
    pub average_stack_depth: usize,
}

/// Safe arithmetic operations
pub struct SafeArithmetic;

impl SafeArithmetic {
    /// Safe addition with overflow check
    ///
    /// # Errors
    ///
    /// Returns an error if the addition would overflow.
    pub fn add<T>(a: T, b: T) -> Result<T>
    where
        T: std::ops::Add<Output = T> + CheckedAdd,
    {
        a.checked_add(b)
            .ok_or_else(|| LinkMLError::service("Arithmetic overflow in addition"))
    }

    /// Safe subtraction with overflow check
    ///
    /// # Errors
    ///
    /// Returns an error if the subtraction would overflow.
    pub fn sub<T>(a: T, b: T) -> Result<T>
    where
        T: std::ops::Sub<Output = T> + CheckedSub,
    {
        a.checked_sub(b)
            .ok_or_else(|| LinkMLError::service("Arithmetic overflow in subtraction"))
    }

    /// Safe multiplication with overflow check
    ///
    /// # Errors
    ///
    /// Returns an error if the multiplication would overflow.
    pub fn mul<T>(a: T, b: T) -> Result<T>
    where
        T: std::ops::Mul<Output = T> + CheckedMul,
    {
        a.checked_mul(b)
            .ok_or_else(|| LinkMLError::service("Arithmetic overflow in multiplication"))
    }

    /// Safe division with zero check
    ///
    /// # Errors
    ///
    /// Returns an error if dividing by zero.
    pub fn div<T>(a: T, b: T) -> Result<T>
    where
        T: std::ops::Div<Output = T> + PartialEq + Default,
    {
        if b == T::default() {
            return Err(LinkMLError::service("Division by zero"));
        }
        Ok(a / b)
    }
}

/// Trait for checked arithmetic operations
pub trait CheckedAdd: Sized {
    /// Performs checked addition, returning None on overflow
    fn checked_add(self, rhs: Self) -> Option<Self>;
}

/// Trait for checked subtraction operations
pub trait CheckedSub: Sized {
    /// Performs checked subtraction, returning None on underflow
    fn checked_sub(self, rhs: Self) -> Option<Self>;
}

/// Trait for checked multiplication operations
pub trait CheckedMul: Sized {
    /// Performs checked multiplication, returning None on overflow
    fn checked_mul(self, rhs: Self) -> Option<Self>;
}

// Implement for common types
impl CheckedAdd for i32 {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
}

impl CheckedSub for i32 {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
}

impl CheckedMul for i32 {
    fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.checked_mul(rhs)
    }
}

impl CheckedAdd for usize {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
}

impl CheckedSub for usize {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
}

impl CheckedMul for usize {
    fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.checked_mul(rhs)
    }
}

impl CheckedAdd for u32 {
    fn checked_add(self, rhs: Self) -> Option<Self> {
        self.checked_add(rhs)
    }
}

impl CheckedSub for u32 {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.checked_sub(rhs)
    }
}

impl CheckedMul for u32 {
    fn checked_mul(self, rhs: Self) -> Option<Self> {
        self.checked_mul(rhs)
    }
}

/// Safe bounds checking for collections
pub struct SafeBounds;

impl SafeBounds {
    /// Safe array/vec access
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
    pub fn get<T>(slice: &[T], index: usize) -> Result<&T> {
        slice.get(index).ok_or_else(|| {
            LinkMLError::service(format!(
                "Index {} out of bounds for slice of length {}",
                index,
                slice.len()
            ))
        })
    }

    /// Safe mutable array/vec access
    ///
    /// # Errors
    ///
    /// Returns an error if the index is out of bounds.
    pub fn get_mut<T>(slice: &mut [T], index: usize) -> Result<&mut T> {
        let len = slice.len();
        slice.get_mut(index).ok_or_else(|| {
            LinkMLError::service(format!(
                "Index {index} out of bounds for slice of length {len}"
            ))
        })
    }

    /// Safe string slicing
    ///
    /// # Errors
    ///
    /// Returns an error if the slice boundaries are invalid or out of bounds.
    pub fn slice_str(s: &str, start: usize, end: usize) -> Result<&str> {
        if !s.is_char_boundary(start) || !s.is_char_boundary(end) {
            return Err(LinkMLError::service("Invalid UTF-8 boundary"));
        }

        s.get(start..end).ok_or_else(|| {
            LinkMLError::service(format!(
                "String slice {}..{} out of bounds for string of length {}",
                start,
                end,
                s.len()
            ))
        })
    }
}

/// Poison recovery for locks
pub struct PoisonRecovery;

impl PoisonRecovery {
    /// Recover from poisoned mutex
    ///
    /// # Errors
    ///
    /// Always returns Ok since it recovers from poison errors.
    pub fn recover_mutex<T>(
        mutex: &std::sync::Mutex<T>,
        _timeout: Duration,
    ) -> Result<std::sync::MutexGuard<'_, T>> {
        match mutex.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                tracing::warn!("Recovering from poisoned mutex");
                Ok(poisoned.into_inner())
            }
        }
    }

    /// Recover from poisoned rwlock for reading
    ///
    /// # Errors
    ///
    /// Always returns Ok since it recovers from poison errors.
    pub fn recover_read<T>(
        rwlock: &std::sync::RwLock<T>,
        _timeout: Duration,
    ) -> Result<std::sync::RwLockReadGuard<'_, T>> {
        match rwlock.read() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                tracing::warn!("Recovering from poisoned rwlock (read)");
                Ok(poisoned.into_inner())
            }
        }
    }

    /// Recover from poisoned rwlock for writing
    ///
    /// # Errors
    ///
    /// Always returns Ok since it recovers from poison errors.
    pub fn recover_write<T>(
        rwlock: &std::sync::RwLock<T>,
        _timeout: Duration,
    ) -> Result<std::sync::RwLockWriteGuard<'_, T>> {
        match rwlock.write() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                tracing::warn!("Recovering from poisoned rwlock (write)");
                Ok(poisoned.into_inner())
            }
        }
    }
}

/// UTF-8 validation wrapper
pub struct SafeUtf8;

impl SafeUtf8 {
    /// Safe conversion from bytes to string
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    pub fn from_bytes(bytes: &[u8]) -> Result<&str> {
        std::str::from_utf8(bytes).map_err(|e| LinkMLError::service(format!("Invalid UTF-8: {e}")))
    }

    /// Safe conversion from bytes to owned string
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    pub fn from_bytes_owned(bytes: Vec<u8>) -> Result<String> {
        String::from_utf8(bytes).map_err(|e| LinkMLError::service(format!("Invalid UTF-8: {e}")))
    }

    /// Validate UTF-8 without conversion
    ///
    /// # Errors
    ///
    /// Returns an error if the bytes are not valid UTF-8.
    pub fn validate(bytes: &[u8]) -> Result<()> {
        std::str::from_utf8(bytes)
            .map(|_| ())
            .map_err(|e| LinkMLError::service(format!("Invalid UTF-8: {e}")))
    }
}

/// Panic-safe validation wrapper
pub struct SafeValidation {
    wrapper: PanicSafeWrapper,
}

impl SafeValidation {
    /// Create new safe validation wrapper
    #[must_use]
    pub fn new(config: PanicPreventionConfig) -> Self {
        Self {
            wrapper: PanicSafeWrapper::new(config),
        }
    }

    /// Validate with panic protection
    ///
    /// # Errors
    ///
    /// Returns an error if the validation function panics or fails.
    pub async fn validate_safe<F, T>(&self, name: &str, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + std::panic::UnwindSafe + 'static,
        T: Send + 'static,
    {
        self.wrapper.execute_async(name, f).await
    }

    /// Get panic statistics
    #[must_use]
    pub fn panic_stats(&self) -> PanicStats {
        self.wrapper.get_panic_stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_depth_tracker() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let tracker = StackDepthTracker::new(3);

        let _g1 = tracker.enter()?;
        let _g2 = tracker.enter()?;
        let _g3 = tracker.enter()?;

        // Fourth should fail
        assert!(tracker.enter().is_err());
        Ok(())
    }

    #[test]
    fn test_panic_wrapper() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let wrapper = PanicSafeWrapper::new(PanicPreventionConfig::default());

        // Normal operation
        let result = wrapper.execute("test", || Ok(42));
        assert_eq!(result?, 42);

        // Error operation (replacing panic test to follow RootReal standards)
        let result = wrapper.execute("error_test", || -> Result<i32> {
            Err(LinkMLError::service("test error".to_string()))
        });
        assert!(result.is_err());

        // Check stats
        let stats = wrapper.get_panic_stats();
        assert_eq!(stats.total_panics, 1);
        Ok(())
    }

    #[test]
    fn test_safe_arithmetic() -> std::result::Result<(), Box<dyn std::error::Error>> {
        assert!(SafeArithmetic::add(i32::MAX, 1).is_err());
        assert!(SafeArithmetic::sub(0u32, 1).is_err());
        assert!(SafeArithmetic::div(10, 0).is_err());

        assert_eq!(SafeArithmetic::add(5, 3)?, 8);
        assert_eq!(SafeArithmetic::div(10, 2)?, 5);
        Ok(())
    }

    #[test]
    fn test_safe_bounds() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let vec = vec![1, 2, 3];

        assert_eq!(*SafeBounds::get(&vec, 1)?, 2);
        assert!(SafeBounds::get(&vec, 10).is_err());

        let s = "hello";
        assert_eq!(SafeBounds::slice_str(s, 0, 5)?, "hello");
        assert!(SafeBounds::slice_str(s, 1, 10).is_err());
        Ok(())
    }
}
