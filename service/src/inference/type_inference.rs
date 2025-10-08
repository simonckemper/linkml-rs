//! Type inference from sample values
//!
//! This module provides functionality for inferring LinkML types from string samples
//! using pattern matching and validation. The type inference follows a priority order
//! from most specific to most general types.

use crate::inference::traits::{InferredType, TypeInferencer};
use chrono::NaiveDateTime;
use regex::Regex;
use std::sync::Arc;

/// Standard type inferencer implementation
///
/// This implementation uses pattern matching and parsing to infer the most specific
/// type that matches all provided samples. It follows this priority order:
/// 1. Boolean (true/false)
/// 2. Integer (i64)
/// 3. Float (f64)
/// 4. DateTime (ISO 8601)
/// 5. Date (ISO 8601)
/// 6. Time (ISO 8601)
/// 7. Uri (http://, https://, ftp://)
/// 8. Email (contains @ with basic validation)
/// 9. String (fallback)
pub struct StandardTypeInferencer {
    email_regex: Regex,
    uri_regex: Regex,
    date_regex: Regex,
    time_regex: Regex,
    datetime_regex: Regex,
}

impl StandardTypeInferencer {
    /// Create a new StandardTypeInferencer with compiled regexes
    pub fn new() -> Self {
        Self {
            email_regex: Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
                .expect("Valid email regex"),
            uri_regex: Regex::new(r"^(https?|ftp)://[^\s/$.?#].[^\s]*$").expect("Valid URI regex"),
            date_regex: Regex::new(r"^\d{4}-\d{2}-\d{2}$").expect("Valid date regex"),
            time_regex: Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?$").expect("Valid time regex"),
            datetime_regex: Regex::new(
                r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$",
            )
            .expect("Valid datetime regex"),
        }
    }

    /// Infer type from a single value
    fn infer_single_value(&self, value: &str) -> InferredType {
        let trimmed = value.trim();

        // Empty string
        if trimmed.is_empty() {
            return InferredType::Unknown;
        }

        // Boolean
        if is_boolean(trimmed) {
            return InferredType::Boolean;
        }

        // Integer
        if trimmed.parse::<i64>().is_ok() {
            return InferredType::Integer;
        }

        // Float
        if trimmed.parse::<f64>().is_ok() {
            return InferredType::Float;
        }

        // DateTime
        if self.datetime_regex.is_match(trimmed) && is_valid_datetime(trimmed) {
            return InferredType::DateTime;
        }

        // Date
        if self.date_regex.is_match(trimmed) {
            return InferredType::Date;
        }

        // Time
        if self.time_regex.is_match(trimmed) {
            return InferredType::Time;
        }

        // URI
        if self.uri_regex.is_match(trimmed) {
            return InferredType::Uri;
        }

        // Email
        if self.email_regex.is_match(trimmed) {
            return InferredType::Email;
        }

        // Default to String
        InferredType::String
    }

    /// Find the most general type that matches all samples
    fn find_common_type(&self, samples: &[String]) -> InferredType {
        if samples.is_empty() {
            return InferredType::Unknown;
        }

        // Infer type for first sample
        let mut common_type = self.infer_single_value(&samples[0]);

        // Check if all other samples match the same type
        for sample in &samples[1..] {
            let sample_type = self.infer_single_value(sample);

            // If types don't match, try to find a common parent type
            if sample_type != common_type {
                common_type = find_common_parent_type(common_type, sample_type);
            }
        }

        common_type
    }

    /// Calculate confidence based on sample agreement
    fn calculate_confidence(&self, samples: &[String], inferred_type: &InferredType) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let matching_count = samples
            .iter()
            .filter(|sample| {
                let sample_type = self.infer_single_value(sample);
                &sample_type == inferred_type || is_compatible_type(&sample_type, inferred_type)
            })
            .count();

        matching_count as f32 / samples.len() as f32
    }
}

impl Default for StandardTypeInferencer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeInferencer for StandardTypeInferencer {
    fn infer_from_samples(&self, samples: &[String]) -> InferredType {
        self.find_common_type(samples)
    }

    fn infer_with_confidence(&self, samples: &[String]) -> (InferredType, f32) {
        let inferred = self.find_common_type(samples);
        let confidence = self.calculate_confidence(samples, &inferred);
        (inferred, confidence)
    }
}

/// Check if a string represents a boolean value
fn is_boolean(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "1" | "0"
    )
}

/// Validate datetime string by attempting to parse it
fn is_valid_datetime(s: &str) -> bool {
    // Try parsing with chrono
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .is_ok()
}

/// Find a common parent type for two different types
fn find_common_parent_type(type1: InferredType, type2: InferredType) -> InferredType {
    match (type1, type2) {
        // Same types
        (t1, t2) if t1 == t2 => t1,

        // Integer and Float can both be represented as Float
        (InferredType::Integer, InferredType::Float)
        | (InferredType::Float, InferredType::Integer) => InferredType::Float,

        // DateTime is more general than Date or Time
        (InferredType::DateTime, InferredType::Date | InferredType::Time)
        | (InferredType::Date | InferredType::Time, InferredType::DateTime) => {
            InferredType::DateTime
        }

        // Date and Time have no common parent except String
        (InferredType::Date, InferredType::Time) | (InferredType::Time, InferredType::Date) => {
            InferredType::String
        }

        // Any type combined with String becomes String
        (InferredType::String, _) | (_, InferredType::String) => InferredType::String,

        // Uri and Email are both strings
        (InferredType::Uri, InferredType::Email) | (InferredType::Email, InferredType::Uri) => {
            InferredType::String
        }

        // Unknown combined with anything becomes that type
        (InferredType::Unknown, t) | (t, InferredType::Unknown) => t,

        // All other combinations fall back to String
        _ => InferredType::String,
    }
}

/// Check if a type is compatible with another type
fn is_compatible_type(sample_type: &InferredType, target_type: &InferredType) -> bool {
    match (sample_type, target_type) {
        // Same types are always compatible
        (t1, t2) if t1 == t2 => true,

        // Integer is compatible with Float
        (InferredType::Integer, InferredType::Float) => true,

        // Date and Time are compatible with DateTime
        (InferredType::Date | InferredType::Time, InferredType::DateTime) => true,

        // Everything is compatible with String
        (_, InferredType::String) => true,

        // Unknown is compatible with everything
        (InferredType::Unknown, _) | (_, InferredType::Unknown) => true,

        // All other combinations are incompatible
        _ => false,
    }
}

/// Public helper function for inferring type from a single value
///
/// This is a convenience function used by TypeVotes and other modules
/// that need to infer a type from a single string value.
pub fn infer_type_from_value(value: &str) -> InferredType {
    let inferencer = StandardTypeInferencer::new();
    inferencer.infer_single_value(value)
}

/// Create a standard type inferencer instance
///
/// # Returns
///
/// * `Arc<StandardTypeInferencer>` - Thread-safe type inferencer
///
/// # Example
///
/// ```rust
/// use linkml_service::inference::create_type_inferencer;
/// use linkml_service::inference::TypeInferencer;
///
/// let inferencer = create_type_inferencer();
/// let samples = vec!["10".to_string(), "20".to_string(), "30".to_string()];
/// let inferred_type = inferencer.infer_from_samples(&samples);
/// println!("Inferred type: {}", inferred_type);
/// ```
pub fn create_type_inferencer() -> Arc<StandardTypeInferencer> {
    Arc::new(StandardTypeInferencer::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_boolean() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec!["true".to_string(), "false".to_string()]),
            InferredType::Boolean
        );
        assert_eq!(
            inferencer.infer_from_samples(&vec!["yes".to_string(), "no".to_string()]),
            InferredType::Boolean
        );
        assert_eq!(
            inferencer.infer_from_samples(&vec!["1".to_string(), "0".to_string()]),
            InferredType::Boolean
        );
    }

    #[test]
    fn test_infer_integer() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec![
                "10".to_string(),
                "20".to_string(),
                "-5".to_string()
            ]),
            InferredType::Integer
        );
    }

    #[test]
    fn test_infer_float() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec![
                "10.5".to_string(),
                "20.3".to_string(),
                "-5.7".to_string()
            ]),
            InferredType::Float
        );
    }

    #[test]
    fn test_infer_mixed_numeric() {
        let inferencer = StandardTypeInferencer::new();
        // Integer and Float should become Float
        assert_eq!(
            inferencer.infer_from_samples(&vec!["10".to_string(), "20.5".to_string()]),
            InferredType::Float
        );
    }

    #[test]
    fn test_infer_uri() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec![
                "https://example.com".to_string(),
                "http://test.org".to_string()
            ]),
            InferredType::Uri
        );
    }

    #[test]
    fn test_infer_email() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec![
                "test@example.com".to_string(),
                "user@domain.org".to_string()
            ]),
            InferredType::Email
        );
    }

    #[test]
    fn test_infer_string() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec!["hello".to_string(), "world".to_string()]),
            InferredType::String
        );
    }

    #[test]
    fn test_infer_with_confidence() {
        let inferencer = StandardTypeInferencer::new();

        // All integers - 100% confidence
        let (inferred, confidence) = inferencer.infer_with_confidence(&vec![
            "10".to_string(),
            "20".to_string(),
            "30".to_string(),
        ]);
        assert_eq!(inferred, InferredType::Integer);
        assert_eq!(confidence, 1.0);

        // Mixed types - lower confidence
        let (inferred, confidence) = inferencer.infer_with_confidence(&vec![
            "10".to_string(),
            "hello".to_string(),
            "30".to_string(),
        ]);
        assert_eq!(inferred, InferredType::String);
        assert!(confidence < 1.0);
    }

    #[test]
    fn test_empty_samples() {
        let inferencer = StandardTypeInferencer::new();
        assert_eq!(
            inferencer.infer_from_samples(&vec![]),
            InferredType::Unknown
        );
    }

    #[test]
    fn test_create_type_inferencer() {
        let inferencer = create_type_inferencer();
        let result = inferencer.infer_from_samples(&vec!["42".to_string()]);
        assert_eq!(result, InferredType::Integer);
    }

    #[test]
    fn test_infer_type_from_value() {
        assert_eq!(infer_type_from_value("42"), InferredType::Integer);
        assert_eq!(infer_type_from_value("3.14"), InferredType::Float);
        assert_eq!(infer_type_from_value("true"), InferredType::Boolean);
        assert_eq!(infer_type_from_value("hello"), InferredType::String);
    }
}
