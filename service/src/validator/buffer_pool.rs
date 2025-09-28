//! Buffer pooling for efficient memory reuse during validation
//!
//! This module provides object pools for commonly allocated types during
//! validation to reduce allocation overhead and improve performance.

use parking_lot::Mutex;
use std::collections::VecDeque;
use std::sync::Arc;

/// Pool for reusable String buffers
pub struct StringPool {
    pool: Arc<Mutex<VecDeque<String>>>,
    max_size: usize,
    max_buffer_capacity: usize,
}

impl StringPool {
    /// Create a new string pool
    #[must_use]
    pub fn new(max_size: usize, max_buffer_capacity: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            max_buffer_capacity,
        }
    }

    /// Get a string buffer from the pool
    #[must_use]
    pub fn get(&self) -> StringBuffer {
        let mut pool = self.pool.lock();
        let buffer = pool.pop_front().unwrap_or_default();
        StringBuffer {
            buffer,
            pool: Arc::clone(&self.pool),
            max_capacity: self.max_buffer_capacity,
            max_pool_size: self.max_size,
        }
    }

    /// Get current pool size
    #[must_use]
    pub fn size(&self) -> usize {
        self.pool.lock().len()
    }
}

/// A string buffer that returns to the pool when dropped
pub struct StringBuffer {
    buffer: String,
    pool: Arc<Mutex<VecDeque<String>>>,
    max_capacity: usize,
    max_pool_size: usize,
}

impl StringBuffer {
    /// Get a mutable reference to the string
    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.buffer
    }

    /// Get an immutable reference to the string
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Clear the buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Drop for StringBuffer {
    fn drop(&mut self) {
        // Clear the buffer before returning to pool
        self.buffer.clear();

        // Only return to pool if capacity is reasonable
        if self.buffer.capacity() <= self.max_capacity {
            let mut pool = self.pool.lock();
            // Only add back if pool isn't full
            if pool.len() < self.max_pool_size {
                pool.push_back(std::mem::take(&mut self.buffer));
            }
        }
    }
}

impl std::ops::Deref for StringBuffer {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl std::ops::DerefMut for StringBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// Pool for reusable Vec<T> buffers
pub struct VecPool<T> {
    pool: Arc<Mutex<VecDeque<Vec<T>>>>,
    max_size: usize,
    max_buffer_capacity: usize,
}

impl<T> VecPool<T> {
    /// Create a new vector pool
    #[must_use]
    pub fn new(max_size: usize, max_buffer_capacity: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(VecDeque::with_capacity(max_size))),
            max_size,
            max_buffer_capacity,
        }
    }

    /// Get a vector buffer from the pool
    #[must_use]
    pub fn get(&self) -> VecBuffer<T> {
        let mut pool = self.pool.lock();
        let buffer = pool.pop_front().unwrap_or_default();
        VecBuffer {
            buffer,
            pool: Arc::clone(&self.pool),
            max_capacity: self.max_buffer_capacity,
            max_pool_size: self.max_size,
        }
    }

    /// Get current pool size
    #[must_use]
    pub fn size(&self) -> usize {
        self.pool.lock().len()
    }
}

/// A vector buffer that returns to the pool when dropped
pub struct VecBuffer<T> {
    buffer: Vec<T>,
    pool: Arc<Mutex<VecDeque<Vec<T>>>>,
    max_capacity: usize,
    max_pool_size: usize,
}

impl<T> VecBuffer<T> {
    /// Clear the buffer for reuse
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl<T> Drop for VecBuffer<T> {
    fn drop(&mut self) {
        // Clear the buffer before returning to pool
        self.buffer.clear();

        // Only return to pool if capacity is reasonable
        if self.buffer.capacity() <= self.max_capacity {
            let mut pool = self.pool.lock();
            // Only add back if pool isn't full
            if pool.len() < self.max_pool_size {
                pool.push_back(std::mem::take(&mut self.buffer));
            }
        }
    }
}

impl<T> std::ops::Deref for VecBuffer<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.buffer
    }
}

impl<T> std::ops::DerefMut for VecBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.buffer
    }
}

/// Global buffer pools for the validation system
pub struct ValidationBufferPools {
    /// Pool for `JSON` path strings
    pub path_strings: StringPool,
    /// Pool for error message strings
    pub error_messages: StringPool,
    /// Pool for temporary string buffers
    pub temp_strings: StringPool,
    /// Pool for validation issue vectors
    pub issue_vecs: VecPool<super::report::ValidationIssue>,
}

impl ValidationBufferPools {
    /// Create a new set of buffer pools with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            path_strings: StringPool::new(100, 256),
            error_messages: StringPool::new(50, 512),
            temp_strings: StringPool::new(200, 1024),
            issue_vecs: VecPool::new(50, 100),
        }
    }

    /// Create pools with custom settings
    #[must_use]
    pub fn with_config(
        path_pool_size: usize,
        error_pool_size: usize,
        temp_pool_size: usize,
        issue_pool_size: usize,
    ) -> Self {
        Self {
            path_strings: StringPool::new(path_pool_size, 256),
            error_messages: StringPool::new(error_pool_size, 512),
            temp_strings: StringPool::new(temp_pool_size, 1024),
            issue_vecs: VecPool::new(issue_pool_size, 100),
        }
    }

    /// Get statistics about pool usage
    #[must_use]
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            path_strings_pooled: self.path_strings.size(),
            error_messages_pooled: self.error_messages.size(),
            temp_strings_pooled: self.temp_strings.size(),
            issue_vecs_pooled: self.issue_vecs.size(),
        }
    }
}

impl Default for ValidationBufferPools {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about buffer pool usage
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Number of path strings in pool
    pub path_strings_pooled: usize,
    /// Number of error messages in pool
    pub error_messages_pooled: usize,
    /// Number of temp strings in pool
    pub temp_strings_pooled: usize,
    /// Number of issue vectors in pool
    pub issue_vecs_pooled: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_pool() {
        let pool = StringPool::new(10, 1024);

        // Get a buffer
        let mut buffer1 = pool.get();
        buffer1.push_str("Hello, World!");
        assert_eq!(buffer1.as_str(), "Hello, World!");

        // Drop returns to pool
        drop(buffer1);
        assert_eq!(pool.size(), 1);

        // Reuse the buffer
        let buffer2 = pool.get();
        assert_eq!(buffer2.as_str(), ""); // Should be cleared
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_vec_pool() {
        let pool = VecPool::<i32>::new(10, 1024);

        // Get a buffer
        let mut buffer1 = pool.get();
        buffer1.extend(&[1, 2, 3, 4, 5]);
        assert_eq!(buffer1.len(), 5);

        // Drop returns to pool
        drop(buffer1);
        assert_eq!(pool.size(), 1);

        // Reuse the buffer
        let buffer2 = pool.get();
        assert_eq!(buffer2.len(), 0); // Should be cleared
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_pool_capacity_limit() {
        let pool = StringPool::new(2, 100);

        // Fill the pool
        {
            let _b1 = pool.get();
            let _b2 = pool.get();
            let _b3 = pool.get();
        } // All dropped

        // Pool should only contain 2 (max_size)
        assert_eq!(pool.size(), 2);
    }

    #[test]
    fn test_buffer_capacity_limit() {
        let pool = StringPool::new(10, 10);

        {
            let mut buffer = pool.get();
            // Exceed max capacity
            for _ in 0..20 {
                buffer.push('x');
            }
        } // Dropped

        // Should not return to pool due to capacity
        assert_eq!(pool.size(), 0);
    }
}
