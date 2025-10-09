//! # LinkML Service Handle
//!
//! Zero-cost newtype wrapper for LinkML service following Rust-idiomatic DI pattern.
//!
//! ## Purpose
//!
//! The Handle pattern provides:
//!
//! 1. Clear API boundaries for LinkML schema operations
//! 2. Future optimization potential without breaking changes
//! 3. Zero-cost abstraction via `#[repr(transparent)]`
//! 4. Flexible ownership - extract as Arc or owned type
//!
//! ## Design
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │   LinkMLHandle (newtype)                │
//! │  #[repr(transparent)]                   │
//! ├─────────────────────────────────────────┤
//! │  Arc<LinkMLServiceImpl<T,E,C,O,R>>      │
//! ├─────────────────────────────────────────┤
//! │  Full LinkML service with dependencies  │
//! └─────────────────────────────────────────┘
//! ```

use std::sync::Arc;

/// Zero-cost newtype wrapper around LinkML service
///
/// This handle provides a clean abstraction over the LinkML service,
/// enabling dependency injection without exposing internal Arc details.
///
/// # Performance
///
/// Uses `#[repr(transparent)]` to guarantee zero overhead - the handle has the
/// exact same memory layout and ABI as the wrapped `Arc<T>`.
///
/// # Type Parameters
///
/// * `T` - Concrete LinkML service implementation (typically `LinkMLServiceImpl`)
///
/// # Examples
///
/// ```rust,no_run
/// use linkml_service::handle::LinkMLHandle;
/// use linkml_service::service::LinkMLServiceImpl;
/// use std::sync::Arc;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create service with dependencies (shown elsewhere)
/// # let service = todo!();
/// let handle: LinkMLHandle<LinkMLServiceImpl<_, _, _, _, _>> =
///     LinkMLHandle::new(Arc::new(service));
///
/// // Use handle to validate schemas
/// let is_valid = handle.validate_schema("path/to/schema.yaml").await?;
/// # Ok(())
/// # }
/// ```
#[repr(transparent)]
#[derive(Clone)]
pub struct LinkMLHandle<T> {
    inner: Arc<T>,
}

impl<T> LinkMLHandle<T> {
    /// Create a new LinkML handle from a concrete service implementation
    ///
    /// # Arguments
    ///
    /// * `inner` - LinkML service implementation wrapped in Arc
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use linkml_service::handle::LinkMLHandle;
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let service = todo!();
    /// let handle = LinkMLHandle::new(Arc::new(service));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }

    /// Extract the inner Arc<T>
    ///
    /// This consumes the handle and returns the wrapped Arc, useful when you need
    /// to share the service across multiple owners.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use linkml_service::handle::LinkMLHandle;
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let service = todo!();
    /// let handle = LinkMLHandle::new(Arc::new(service));
    ///
    /// // Extract Arc for sharing
    /// let service_arc = handle.into_arc();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn into_arc(self) -> Arc<T> {
        self.inner
    }

    /// Extract the owned service implementation
    ///
    /// This attempts to unwrap the Arc and return the owned service. If the Arc
    /// has other references, this will clone the service instead.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use linkml_service::handle::LinkMLHandle;
    /// use std::sync::Arc;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let service = todo!();
    /// let handle = LinkMLHandle::new(Arc::new(service));
    ///
    /// // Extract owned service
    /// let service = handle.into_inner();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn into_inner(self) -> T
    where
        T: Clone,
    {
        Arc::try_unwrap(self.inner).unwrap_or_else(|arc| (*arc).clone())
    }

    /// Get a reference to the inner service
    ///
    /// This provides access to the service without consuming the handle.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use linkml_service::handle::LinkMLHandle;
    /// use std::sync::Arc;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let service = todo!();
    /// let handle = LinkMLHandle::new(Arc::new(service));
    ///
    /// // Access service through reference
    /// let service_ref = handle.as_ref();
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn as_service_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> AsRef<T> for LinkMLHandle<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

impl<T> std::ops::Deref for LinkMLHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLinkMLService;

    #[test]
    fn test_handle_creation() {
        let service = Arc::new(MockLinkMLService);
        let handle = LinkMLHandle::new(service);
        assert!(std::ptr::eq(
            handle.as_ref() as *const _,
            &MockLinkMLService as *const _
        ));
    }

    #[test]
    fn test_into_arc() {
        let service = Arc::new(MockLinkMLService);
        let handle = LinkMLHandle::new(service.clone());
        let extracted = handle.into_arc();
        assert_eq!(Arc::strong_count(&extracted), 2); // Original + extracted
    }

    #[test]
    fn test_deref() {
        let service = Arc::new(MockLinkMLService);
        let handle = LinkMLHandle::new(service);
        let _ref: &MockLinkMLService = &handle;
    }
}
