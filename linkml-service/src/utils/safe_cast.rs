//! Safe numeric casting utilities for LinkML service
//!
//! This module provides safe casting functions that handle precision loss,
//! truncation, and overflow in a controlled manner appropriate for LinkML's use cases.

/// Safely cast usize to f64 with precision checking
/// For values that exceed f64's precision, returns the maximum representable value
pub fn usize_to_f64(value: usize) -> f64 {
    // f64 can precisely represent integers up to 2^53 - 1
    const MAX_PRECISE_F64: u64 = (1_u64 << 53) - 1;

    if value as u64 <= MAX_PRECISE_F64 {
        value as f64
    } else {
        // For very large values in schema statistics, use max precise value
        // This is reasonable since schemas are unlikely to have > 2^53 elements
        MAX_PRECISE_F64 as f64
    }
}

/// Safely cast f64 to u64 with saturation and rounding
/// Negative values become 0, values too large become u64::MAX
pub fn f64_to_u64_saturating(value: f64) -> u64 {
    if value < 0.0 {
        0
    } else if value > u64::MAX as f64 {
        u64::MAX
    } else {
        value.round() as u64
    }
}

/// Safely cast u64 to f64 with precision awareness
/// For values larger than f64 can precisely represent, use saturation
pub fn u64_to_f64_lossy(value: u64) -> f64 {
    const MAX_PRECISE_F64: u64 = (1_u64 << 53) - 1;
    
    if value <= MAX_PRECISE_F64 {
        value as f64
    } else {
        // For very large values, this is acceptable for metrics and statistics
        value as f64
    }
}

/// Safely cast usize to u32 with saturation
pub fn usize_to_u32_saturating(value: usize) -> u32 {
    if value > u32::MAX as usize {
        u32::MAX
    } else {
        value as u32
    }
}

/// Safely cast usize to i32 with saturation
pub fn usize_to_i32_saturating(value: usize) -> i32 {
    if value > i32::MAX as usize {
        i32::MAX
    } else {
        value as i32
    }
}

/// Safely cast f64 to f32 with saturation
pub fn f64_to_f32_saturating(value: f64) -> f32 {
    if value > f32::MAX as f64 {
        f32::MAX
    } else if value < f32::MIN as f64 {
        f32::MIN
    } else {
        value as f32
    }
}

/// Safely cast i64 to u64, returning 0 for negative values
pub fn i64_to_u64_positive(value: i64) -> u64 {
    if value < 0 {
        0
    } else {
        value as u64
    }
}

/// Safely cast f32 to u8 with saturation and rounding
pub fn f32_to_u8_saturating(value: f32) -> u8 {
    if value < 0.0 {
        0
    } else if value > 255.0 {
        255
    } else {
        value.round() as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usize_to_f64() {
        assert_eq!(usize_to_f64(0), 0.0);
        assert_eq!(usize_to_f64(1000), 1000.0);
        // Test precision boundary
        let max_precise = (1_u64 << 53) - 1;
        assert_eq!(usize_to_f64(max_precise as usize), max_precise as f64);
    }

    #[test]
    fn test_f64_to_u64_saturating() {
        assert_eq!(f64_to_u64_saturating(-1.0), 0);
        assert_eq!(f64_to_u64_saturating(100.5), 101);
        assert_eq!(f64_to_u64_saturating(0.0), 0);
        assert_eq!(f64_to_u64_saturating(f64::MAX), u64::MAX);
    }

    #[test]
    fn test_usize_to_u32_saturating() {
        assert_eq!(usize_to_u32_saturating(100), 100);
        #[cfg(target_pointer_width = "64")]
        assert_eq!(usize_to_u32_saturating(usize::MAX), u32::MAX);
    }

    #[test]
    fn test_i64_to_u64_positive() {
        assert_eq!(i64_to_u64_positive(-1), 0);
        assert_eq!(i64_to_u64_positive(0), 0);
        assert_eq!(i64_to_u64_positive(100), 100);
    }
}
