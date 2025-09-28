//! Array validation for `LinkML` arrays
//!
//! This module provides comprehensive validation for array data
//! including shape, type, uniqueness, and custom constraints.

use super::{ArrayData, ArrayDimension};
use linkml_core::types::{SlotDefinition, TypeDefinition};
use serde_json::Value;
use std::collections::HashSet;

/// Type alias for custom validator functions
type CustomValidator<'a> = Box<dyn Fn(&Value) -> Result<(), String> + 'a>;

/// Array validation context
pub struct ArrayValidationContext<'a> {
    /// The LinkML slot definition that defines constraints for this array
    pub slot: &'a SlotDefinition,

    /// Type definitions for type checking
    pub types: &'a std::collections::HashMap<String, TypeDefinition>,

    /// Whether to allow missing values
    pub allow_missing: bool,

    /// Whether to check uniqueness
    pub check_unique: bool,

    /// Custom validators
    pub custom_validators: Vec<CustomValidator<'a>>,
}

/// Array validation result
#[derive(Debug, Clone)]
pub struct ArrayValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Validation errors
    pub errors: Vec<ArrayValidationError>,

    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Array validation error
#[derive(Debug, Clone)]
pub struct ArrayValidationError {
    /// Error location (indices)
    pub location: Option<Vec<usize>>,

    /// Error message
    pub message: String,

    /// Error type
    pub error_type: ArrayValidationErrorType,
}

/// Types of validation errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayValidationErrorType {
    /// Shape mismatch
    ShapeError,
    /// Type mismatch
    TypeError,
    /// Value out of range
    RangeError,
    /// Pattern mismatch
    PatternError,
    /// Uniqueness violation
    UniquenessError,
    /// Missing required value
    MissingError,
    /// Custom validation failure
    CustomError,
}

/// Enhanced array validator
pub struct ArrayValidatorV2;

impl ArrayValidatorV2 {
    /// Validate array data with full context
    ///
    /// # Errors
    /// Returns validation errors if array data doesn't conform to specification or context requirements.
    #[must_use]
    pub fn validate_with_context(
        data: &ArrayData,
        context: &ArrayValidationContext,
    ) -> ArrayValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Basic shape validation
        if let Err(e) = data.spec.validate_shape(&data.shape) {
            errors.push(ArrayValidationError {
                location: None,
                message: e.to_string(),
                error_type: ArrayValidationErrorType::ShapeError,
            });
        }

        // Validate each element
        for (i, value) in data.data.iter().enumerate() {
            let indices = data.spec.flat_to_indices(i, &data.shape);

            // Type validation
            if let Err(e) =
                Self::validate_element_type(value, &data.spec.element_type, context.types)
            {
                errors.push(ArrayValidationError {
                    location: Some(indices.clone()),
                    message: e,
                    error_type: ArrayValidationErrorType::TypeError,
                });
            }

            // Missing value check
            if !context.allow_missing && value.is_null() {
                errors.push(ArrayValidationError {
                    location: Some(indices.clone()),
                    message: "Missing value not allowed".to_string(),
                    error_type: ArrayValidationErrorType::MissingError,
                });
            }

            // Range validation
            if let Some(min_value) = context.slot.minimum_value.as_ref() {
                let min_str = match min_value {
                    Value::String(s) => s.as_str(),
                    Value::Number(n) => &n.to_string(),
                    _ => "",
                };
                if !min_str.is_empty()
                    && let Err(e) = Self::validate_minimum(value, min_str)
                {
                    errors.push(ArrayValidationError {
                        location: Some(indices.clone()),
                        message: e,
                        error_type: ArrayValidationErrorType::RangeError,
                    });
                }
            }

            if let Some(max_value) = context.slot.maximum_value.as_ref() {
                let max_str = match max_value {
                    Value::String(s) => s.as_str(),
                    Value::Number(n) => &n.to_string(),
                    _ => "",
                };
                if !max_str.is_empty()
                    && let Err(e) = Self::validate_maximum(value, max_str)
                {
                    errors.push(ArrayValidationError {
                        location: Some(indices.clone()),
                        message: e,
                        error_type: ArrayValidationErrorType::RangeError,
                    });
                }
            }

            // Pattern validation
            if let Some(pattern) = &context.slot.pattern
                && let Value::String(s) = value
                && let Err(e) = Self::validate_pattern(s, pattern)
            {
                errors.push(ArrayValidationError {
                    location: Some(indices.clone()),
                    message: e,
                    error_type: ArrayValidationErrorType::PatternError,
                });
            }

            // Custom validators
            for validator in &context.custom_validators {
                if let Err(e) = validator(value) {
                    errors.push(ArrayValidationError {
                        location: Some(indices.clone()),
                        message: e,
                        error_type: ArrayValidationErrorType::CustomError,
                    });
                }
            }
        }

        // Uniqueness check
        if context.check_unique
            && let Some(duplicate_indices) = Self::find_duplicates(data)
        {
            for indices in duplicate_indices {
                errors.push(ArrayValidationError {
                    location: Some(indices),
                    message: "Duplicate value found".to_string(),
                    error_type: ArrayValidationErrorType::UniquenessError,
                });
            }
        }

        // Dimension-specific validation
        for (i, dim) in data.spec.dimensions.iter().enumerate() {
            if let Some(actual_size) = data.shape.get(i)
                && let Err(e) = dim.validate_size(*actual_size)
            {
                warnings.push(format!("Dimension {} ({}): {}", i, dim.name, e));
            }
        }

        ArrayValidationResult {
            valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate element type
    fn validate_element_type(
        value: &Value,
        expected_type: &str,
        type_defs: &std::collections::HashMap<String, TypeDefinition>,
    ) -> Result<(), String> {
        // Handle null values
        if value.is_null() {
            return Ok(()); // Null handling is done separately
        }

        // Check against built-in types
        let valid = match expected_type {
            "string" | "uri" | "uriorcurie" => value.is_string(),
            "integer" => value.is_i64() || value.is_u64(),
            "float" | "double" | "decimal" => value.is_number(),
            "boolean" => value.is_boolean(),
            _ => {
                // Check custom types
                if let Some(type_def) = type_defs.get(expected_type)
                    && let Some(base_type) = &type_def.base_type
                {
                    return Self::validate_element_type(value, base_type, type_defs);
                }
                true // Unknown types pass by default
            }
        };

        if valid {
            Ok(())
        } else {
            Err(format!(
                "Expected type '{}', got '{}'",
                expected_type,
                match value {
                    Value::Null => "null",
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    Value::Object(_) => "object",
                }
            ))
        }
    }

    /// Validate minimum value
    fn validate_minimum(value: &Value, minimum: &str) -> Result<(), String> {
        if let Value::Number(n) = value
            && let Ok(min_val) = minimum.parse::<f64>()
        {
            let val = n.as_f64().unwrap_or(f64::NAN);
            if !val.is_nan() && val < min_val {
                return Err(format!("Value {val} is less than minimum {min_val}"));
            }
        }
        Ok(())
    }

    /// Validate maximum value
    fn validate_maximum(value: &Value, maximum: &str) -> Result<(), String> {
        if let Value::Number(n) = value
            && let Ok(max_val) = maximum.parse::<f64>()
        {
            let val = n.as_f64().unwrap_or(f64::NAN);
            if !val.is_nan() && val > max_val {
                return Err(format!("Value {val} exceeds maximum {max_val}"));
            }
        }
        Ok(())
    }

    /// Validate against pattern
    fn validate_pattern(value: &str, pattern: &str) -> Result<(), String> {
        // Simple pattern matching - in production, use regex
        if pattern.starts_with('^') && pattern.ends_with('$') {
            // Full match pattern
            let pattern_content = &pattern[1..pattern.len() - 1];
            if !value.contains(pattern_content) {
                return Err(format!("Value '{value}' doesn't match pattern '{pattern}'"));
            }
        }
        Ok(())
    }

    /// Find duplicate values and their indices
    fn find_duplicates(data: &ArrayData) -> Option<Vec<Vec<usize>>> {
        let mut seen = HashSet::new();
        let mut duplicates = Vec::new();

        for (i, value) in data.data.iter().enumerate() {
            // Convert to comparable string representation
            let key = serde_json::to_string(value).unwrap_or_default();

            if !seen.insert(key) {
                let indices = data.spec.flat_to_indices(i, &data.shape);
                duplicates.push(indices);
            }
        }

        if duplicates.is_empty() {
            None
        } else {
            Some(duplicates)
        }
    }
}

/// Dimension validator
pub struct DimensionValidator;

impl DimensionValidator {
    /// Validate array dimensions against constraints
    #[must_use]
    pub fn validate_dimensions(dimensions: &[ArrayDimension], shape: &[usize]) -> Vec<String> {
        let mut errors = Vec::new();

        if dimensions.len() != shape.len() {
            errors.push(format!(
                "Dimension count mismatch: expected {}, got {}",
                dimensions.len(),
                shape.len()
            ));
            return errors;
        }

        for (i, (dim, &size)) in dimensions.iter().zip(shape.iter()).enumerate() {
            // Check fixed size
            if let Some(expected) = dim.size
                && size != expected
            {
                errors.push(format!(
                    "Dimension {} ({}): expected size {}, got {}",
                    i, dim.name, expected, size
                ));
            }

            // Check minimum
            if let Some(min) = dim.min_size
                && size < min
            {
                errors.push(format!(
                    "Dimension {} ({}): size {} is less than minimum {}",
                    i, dim.name, size, min
                ));
            }

            // Check maximum
            if let Some(max) = dim.max_size
                && size > max
            {
                errors.push(format!(
                    "Dimension {} ({}): size {} exceeds maximum {}",
                    i, dim.name, size, max
                ));
            }
        }

        errors
    }
}

/// Builder for array validation context
pub struct ArrayValidationContextBuilder<'a> {
    slot: Option<&'a SlotDefinition>,
    types: Option<&'a std::collections::HashMap<String, TypeDefinition>>,
    allow_missing: bool,
    check_unique: bool,
    custom_validators: Vec<Box<dyn Fn(&Value) -> Result<(), String> + 'a>>,
}

impl Default for ArrayValidationContextBuilder<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ArrayValidationContextBuilder<'a> {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            slot: None,
            types: None,
            allow_missing: false,
            check_unique: false,
            custom_validators: Vec::new(),
        }
    }

    /// Set the slot definition
    #[must_use]
    pub fn slot(mut self, slot: &'a SlotDefinition) -> Self {
        self.slot = Some(slot);
        self
    }

    /// Set type definitions
    #[must_use]
    pub fn types(mut self, types: &'a std::collections::HashMap<String, TypeDefinition>) -> Self {
        self.types = Some(types);
        self
    }

    /// Allow missing values
    #[must_use]
    pub fn allow_missing(mut self, allow: bool) -> Self {
        self.allow_missing = allow;
        self
    }

    /// Enable uniqueness checking
    #[must_use]
    pub fn check_unique(mut self, check: bool) -> Self {
        self.check_unique = check;
        self
    }

    /// Add a custom validator
    #[must_use]
    pub fn add_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&Value) -> Result<(), String> + 'a,
    {
        self.custom_validators.push(Box::new(validator));
        self
    }

    /// Build the validation context
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn build(self) -> Result<ArrayValidationContext<'a>, &'static str> {
        let slot = self.slot.ok_or("Slot definition is required")?;
        let types = self.types.ok_or("Type definitions are required")?;

        Ok(ArrayValidationContext {
            slot,
            types,
            allow_missing: self.allow_missing,
            check_unique: self.check_unique,
            custom_validators: self.custom_validators,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::ArraySpec;
    use serde_json::json;

    fn create_test_slot() -> SlotDefinition {
        SlotDefinition {
            name: "test_array".to_string(),
            range: Some("float".to_string()),
            minimum_value: Some(linkml_core::Value::Number(serde_json::Number::from(0))),
            maximum_value: Some(linkml_core::Value::Number(serde_json::Number::from(100))),
            ..Default::default()
        }
    }

    fn create_test_types() -> std::collections::HashMap<String, TypeDefinition> {
        let mut types = std::collections::HashMap::new();
        types.insert(
            "float".to_string(),
            TypeDefinition {
                name: "float".to_string(),
                base_type: Some("float".to_string()),
                ..Default::default()
            },
        );
        types
    }

    #[test]
    fn test_basic_validation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float").with_dimension(ArrayDimension::fixed("x", 3));

        let data = vec![json!(1.0), json!(2.0), json!(3.0)];
        let array = ArrayData::new(spec, vec![3], data)
            .expect("test data should create valid array - basic validation: {}");

        let slot = create_test_slot();
        let types = create_test_types();

        let context = ArrayValidationContextBuilder::new()
            .slot(&slot)
            .types(&types)
            .build()
            .expect("validation context should build with valid inputs - basic: {}");

        let result = ArrayValidatorV2::validate_with_context(&array, &context);
        assert!(result.valid);
        assert!(result.errors.is_empty());
        Ok(())
    }

    #[test]
    fn test_range_validation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float").with_dimension(ArrayDimension::fixed("x", 3));

        let data = vec![json!(50.0), json!(150.0), json!(-10.0)];
        let array = ArrayData::new(spec, vec![3], data)
            .expect("test data should create valid array - range validation: {}");

        let slot = create_test_slot();
        let types = create_test_types();

        let context = ArrayValidationContextBuilder::new()
            .slot(&slot)
            .types(&types)
            .build()
            .expect("validation context should build with valid inputs - range: {}");

        let result = ArrayValidatorV2::validate_with_context(&array, &context);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 2); // One too high, one too low
        Ok(())
    }

    #[test]
    fn test_uniqueness_validation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 4));

        let data = vec![json!(1), json!(2), json!(2), json!(3)];
        let array = ArrayData::new(spec, vec![4], data)
            .expect("test data should create valid array - uniqueness: {}");

        let slot = create_test_slot();
        let types = create_test_types();

        let context = ArrayValidationContextBuilder::new()
            .slot(&slot)
            .types(&types)
            .check_unique(true)
            .build()
            .expect("validation context should build with valid inputs - unique: {}");

        let result = ArrayValidatorV2::validate_with_context(&array, &context);
        assert!(!result.valid);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.error_type == ArrayValidationErrorType::UniquenessError)
        );
        Ok(())
    }

    #[test]
    fn test_custom_validator() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 3));

        let data = vec![json!(2), json!(4), json!(5)];
        let array = ArrayData::new(spec, vec![3], data)
            .expect("test data should create valid array - custom validator: {}");

        let slot = create_test_slot();
        let types = create_test_types();

        // Custom validator that only allows even numbers
        let context = ArrayValidationContextBuilder::new()
            .slot(&slot)
            .types(&types)
            .add_validator(|v| {
                if let Value::Number(n) = v {
                    if n.as_i64().unwrap_or(0) % 2 == 0 {
                        Ok(())
                    } else {
                        Err("Value must be even".to_string())
                    }
                } else {
                    Ok(())
                }
            })
            .build()
            .expect("validation context should build with valid inputs - custom: {}");

        let result = ArrayValidatorV2::validate_with_context(&array, &context);
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1); // Only 5 is odd
        assert_eq!(result.errors[0].location, Some(vec![2]));
        Ok(())
    }
}
