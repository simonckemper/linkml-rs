//! Small vector optimization for common collection sizes
//!
//! This module provides optimized collection types for the common case
//! where collections have only a few elements.

use smallvec::SmallVec;

/// Type alias for a small vector optimized for validation issues
/// Most slots produce 0-2 issues, so we optimize for that case
pub type IssueVec<T> = SmallVec<[T; 2]>;

/// Type alias for a small vector optimized for slot names
/// Most classes have fewer than 8 direct slots
pub type SlotVec<T> = SmallVec<[T; 8]>;

/// Type alias for a small vector optimized for validator lists
/// Most slots have 3-5 validators
pub type ValidatorVec<T> = SmallVec<[T; 4]>;

/// Type alias for a small vector optimized for path segments
/// Validation paths are typically 2-4 segments deep
pub type PathVec<T> = SmallVec<[T; 4]>;

/// Create an empty issue vector
#[inline]
#[must_use]
pub fn issue_vec<T>() -> IssueVec<T> {
    SmallVec::new()
}

/// Create an empty slot vector
#[inline]
#[must_use]
pub fn slot_vec<T>() -> SlotVec<T> {
    SmallVec::new()
}

/// Create an empty validator vector
#[inline]
#[must_use]
pub fn validator_vec<T>() -> ValidatorVec<T> {
    SmallVec::new()
}

/// Create an empty path vector
#[inline]
#[must_use]
pub fn path_vec<T>() -> PathVec<T> {
    SmallVec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_vec_optimization() {
        // Test that small collections don't allocate
        let mut issues: IssueVec<String> = issue_vec();
        issues.push("error1".to_string());
        issues.push("error2".to_string());

        // Should still be inline (no heap allocation)
        assert!(!issues.spilled());

        // Adding more causes spill to heap
        issues.push("error3".to_string());
        assert!(issues.spilled());
    }
}
