//! Array support for LinkML
//!
//! This module provides support for N-dimensional arrays in LinkML schemas,
//! similar to NumPy arrays or scientific data formats.

pub mod operations;
pub mod validation;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Error type for array operations
#[derive(Debug, Error)]
pub enum ArrayError {
    /// Invalid shape specification
    #[error("Invalid shape: {0}")]
    InvalidShape(String),

    /// Shape mismatch
    #[error("Shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        /// Expected shape
        expected: Vec<usize>,
        /// Actual shape
        actual: Vec<usize>,
    },

    /// Invalid dimension
    #[error("Invalid dimension: {0}")]
    InvalidDimension(String),

    /// Type mismatch
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch {
        /// Expected type
        expected: String,
        /// Actual type
        actual: String,
    },

    /// Index out of bounds
    #[error("Index out of bounds: {index} >= {size}")]
    IndexOutOfBounds {
        /// Index that was accessed
        index: usize,
        /// Size of the dimension
        size: usize,
    },

    /// Invalid data
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// Result type for array operations
pub type ArrayResult<T> = std::result::Result<T, ArrayError>;

/// Array dimension specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayDimension {
    /// Name of the dimension (e.g., "x", "time", "channel")
    pub name: String,

    /// Size of the dimension (None for dynamic)
    pub size: Option<usize>,

    /// Minimum size (if dynamic)
    pub min_size: Option<usize>,

    /// Maximum size (if dynamic)
    pub max_size: Option<usize>,

    /// Description of the dimension
    pub description: Option<String>,
}

impl ArrayDimension {
    /// Create a fixed-size dimension
    pub fn fixed(name: impl Into<String>, size: usize) -> Self {
        Self {
            name: name.into(),
            size: Some(size),
            min_size: None,
            max_size: None,
            description: None,
        }
    }

    /// Create a dynamic dimension
    pub fn dynamic(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            size: None,
            min_size: None,
            max_size: None,
            description: None,
        }
    }

    /// Set minimum size
    #[must_use]
    pub fn with_min(mut self, min: usize) -> Self {
        self.min_size = Some(min);
        self
    }

    /// Set maximum size
    #[must_use]
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_size = Some(max);
        self
    }

    /// Set description
    #[must_use]
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Validate a size against this dimension
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::ShapeMismatch` if the size doesn't match a fixed dimension size.
    /// Returns `ArrayError::InvalidDimension` if the size is below the minimum or above the maximum for dynamic dimensions.
    pub fn validate_size(&self, size: usize) -> ArrayResult<()> {
        if let Some(fixed_size) = self.size
            && size != fixed_size
        {
            return Err(ArrayError::ShapeMismatch {
                expected: vec![fixed_size],
                actual: vec![size],
            });
        }

        if let Some(min) = self.min_size
            && size < min
        {
            return Err(ArrayError::InvalidDimension(format!(
                "Dimension '{}' size {} is less than minimum {}",
                self.name, size, min
            )));
        }

        if let Some(max) = self.max_size
            && size > max
        {
            return Err(ArrayError::InvalidDimension(format!(
                "Dimension '{}' size {} exceeds maximum {}",
                self.name, size, max
            )));
        }

        Ok(())
    }
}

/// Array specification for slots
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArraySpec {
    /// Dimensions of the array
    pub dimensions: Vec<ArrayDimension>,

    /// Element type (`LinkML` type)
    pub element_type: String,

    /// Whether to store in row-major (C) or column-major (Fortran) order
    pub row_major: bool,

    /// Whether missing values are allowed
    pub allow_missing: bool,

    /// Value to use for missing elements
    pub missing_value: Option<serde_json::Value>,
}

impl Default for ArraySpec {
    fn default() -> Self {
        Self {
            dimensions: Vec::new(),
            element_type: "float".to_string(),
            row_major: true,
            allow_missing: false,
            missing_value: None,
        }
    }
}

impl ArraySpec {
    /// Create a new array specification
    pub fn new(element_type: impl Into<String>) -> Self {
        Self {
            element_type: element_type.into(),
            ..Default::default()
        }
    }

    /// Add a dimension
    #[must_use]
    pub fn with_dimension(mut self, dim: ArrayDimension) -> Self {
        self.dimensions.push(dim);
        self
    }

    /// Set column-major order
    #[must_use]
    pub fn column_major(mut self) -> Self {
        self.row_major = false;
        self
    }

    /// Allow missing values
    #[must_use]
    pub fn allow_missing(mut self, value: serde_json::Value) -> Self {
        self.allow_missing = true;
        self.missing_value = Some(value);
        self
    }

    /// Get the number of dimensions
    #[must_use]
    pub fn ndim(&self) -> usize {
        self.dimensions.len()
    }

    /// Check if shape is fully specified (no dynamic dimensions)
    #[must_use]
    pub fn is_fixed_shape(&self) -> bool {
        self.dimensions.iter().all(|d| d.size.is_some())
    }

    /// Get fixed shape if all dimensions are fixed
    #[must_use]
    pub fn fixed_shape(&self) -> Option<Vec<usize>> {
        if self.is_fixed_shape() {
            // We've already verified all dimensions have size in is_fixed_shape()
            // Use unwrap_or(0) as defensive programming, though this should never happen
            Some(
                self.dimensions
                    .iter()
                    .map(|d| d.size.unwrap_or(0))
                    .collect(),
            )
        } else {
            None
        }
    }

    /// Validate a shape against this specification
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidShape` if the shape has a different number of dimensions
    /// than expected, or if any dimension fails validation according to the dimension specification.
    pub fn validate_shape(&self, shape: &[usize]) -> ArrayResult<()> {
        if shape.len() != self.dimensions.len() {
            return Err(ArrayError::InvalidShape(format!(
                "Expected {} dimensions, got {}",
                self.dimensions.len(),
                shape.len()
            )));
        }

        for (i, (dim, &size)) in self.dimensions.iter().zip(shape.iter()).enumerate() {
            dim.validate_size(size).map_err(|e| {
                ArrayError::InvalidShape(format!("Dimension {} ({}): {}", i, dim.name, e))
            })?;
        }

        Ok(())
    }

    /// Calculate total number of elements for a shape
    #[must_use]
    pub fn calculate_size(shape: &[usize]) -> usize {
        shape.iter().product()
    }

    /// Convert flat index to multi-dimensional indices
    #[must_use]
    pub fn flat_to_indices(&self, flat_index: usize, shape: &[usize]) -> Vec<usize> {
        let mut indices = vec![0; shape.len()];
        let mut remaining = flat_index;

        if self.row_major {
            // Row-major (C-style): last dimension varies fastest
            for i in (0..shape.len()).rev() {
                indices[i] = remaining % shape[i];
                remaining /= shape[i];
            }
        } else {
            // Column-major (Fortran-style): first dimension varies fastest
            for i in 0..shape.len() {
                indices[i] = remaining % shape[i];
                remaining /= shape[i];
            }
        }

        indices
    }

    /// Convert multi-dimensional indices to flat index
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidShape` if the indices length doesn't match the shape length.
    /// Returns `ArrayError::IndexOutOfBounds` if any index is greater than or equal to the corresponding dimension size.
    pub fn indices_to_flat(&self, indices: &[usize], shape: &[usize]) -> ArrayResult<usize> {
        if indices.len() != shape.len() {
            return Err(ArrayError::InvalidShape(format!(
                "Indices length {} doesn't match shape length {}",
                indices.len(),
                shape.len()
            )));
        }

        // Validate indices
        for (&idx, &dim_size) in indices.iter().zip(shape.iter()) {
            if idx >= dim_size {
                return Err(ArrayError::IndexOutOfBounds {
                    index: idx,
                    size: dim_size,
                });
            }
        }

        let mut flat_index = 0;

        if self.row_major {
            // Row-major
            let mut stride = 1;
            for i in (0..shape.len()).rev() {
                flat_index += indices.get(i).copied().unwrap_or(0) * stride;
                stride *= shape.get(i).copied().unwrap_or(1);
            }
        } else {
            // Column-major
            let mut stride = 1;
            for i in 0..shape.len() {
                flat_index += indices.get(i).copied().unwrap_or(0) * stride;
                stride *= shape.get(i).copied().unwrap_or(1);
            }
        }

        Ok(flat_index)
    }
}

/// Array data container
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArrayData {
    /// Array specification
    pub spec: ArraySpec,

    /// Actual shape of the data
    pub shape: Vec<usize>,

    /// Flattened data
    pub data: Vec<serde_json::Value>,
}

impl ArrayData {
    /// Create new array data
    ///
    /// # Errors
    ///
    /// Returns `ArrayError` if the shape validation fails against the spec, or if the
    /// data length doesn't match the expected number of elements calculated from the shape.
    pub fn new(
        spec: ArraySpec,
        shape: Vec<usize>,
        data: Vec<serde_json::Value>,
    ) -> ArrayResult<Self> {
        // Validate shape
        spec.validate_shape(&shape)?;

        // Validate data size
        let expected_size = ArraySpec::calculate_size(&shape);
        if data.len() != expected_size {
            return Err(ArrayError::InvalidData(format!(
                "Expected {} elements, got {}",
                expected_size,
                data.len()
            )));
        }

        Ok(Self { spec, shape, data })
    }

    /// Create array filled with a value
    ///
    /// # Errors
    ///
    /// Returns `ArrayError` if the shape validation fails against the spec.
    pub fn filled(
        spec: ArraySpec,
        shape: Vec<usize>,
        value: serde_json::Value,
    ) -> ArrayResult<Self> {
        spec.validate_shape(&shape)?;
        let size = ArraySpec::calculate_size(&shape);
        let data = vec![value; size];
        Ok(Self { spec, shape, data })
    }

    /// Get element at indices
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidShape` if indices length doesn't match shape dimensions.
    /// Returns `ArrayError::IndexOutOfBounds` if any index is out of bounds for its dimension,
    /// or if the calculated flat index exceeds the data array length.
    pub fn get(&self, indices: &[usize]) -> ArrayResult<&serde_json::Value> {
        let flat_index = self.spec.indices_to_flat(indices, &self.shape)?;
        self.data
            .get(flat_index)
            .ok_or(ArrayError::IndexOutOfBounds {
                index: flat_index,
                size: self.data.len(),
            })
    }

    /// Set element at indices
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidShape` if indices length doesn't match shape dimensions.
    /// Returns `ArrayError::IndexOutOfBounds` if any index is out of bounds for its dimension,
    /// or if the calculated flat index exceeds the data array length.
    pub fn set(&mut self, indices: &[usize], value: serde_json::Value) -> ArrayResult<()> {
        let flat_index = self.spec.indices_to_flat(indices, &self.shape)?;
        if flat_index >= self.data.len() {
            return Err(ArrayError::IndexOutOfBounds {
                index: flat_index,
                size: self.data.len(),
            });
        }
        self.data[flat_index] = value;
        Ok(())
    }

    /// Get a slice along a dimension
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidDimension` if the dimension index is out of range.
    /// Returns `ArrayError::IndexOutOfBounds` if the slice index is greater than or equal to the dimension size.
    /// Returns `ArrayError` if creating the new `ArrayData` fails during slice extraction.
    pub fn slice(&self, dimension: usize, index: usize) -> ArrayResult<ArrayData> {
        if dimension >= self.shape.len() {
            return Err(ArrayError::InvalidDimension(format!(
                "Dimension {dimension} out of range"
            )));
        }

        let dim_size = self.shape.get(dimension).copied().ok_or_else(|| {
            ArrayError::InvalidDimension(format!("Dimension {dimension} out of range"))
        })?;

        if index >= dim_size {
            return Err(ArrayError::IndexOutOfBounds {
                index,
                size: dim_size,
            });
        }

        // Create new shape without the sliced dimension
        let mut new_shape = self.shape.clone();
        new_shape.remove(dimension);

        // Create new spec without the sliced dimension
        let mut new_spec = self.spec.clone();
        new_spec.dimensions.remove(dimension);

        // Extract slice data
        let mut slice_data = Vec::new();
        let slice_size = ArraySpec::calculate_size(&new_shape);

        for i in 0..slice_size {
            // Convert slice index to full indices
            let mut full_indices = new_spec.flat_to_indices(i, &new_shape);
            full_indices.insert(dimension, index);

            // Get data at full indices
            let value = self.get(&full_indices)?;
            slice_data.push(value.clone());
        }

        ArrayData::new(new_spec, new_shape, slice_data)
    }

    /// Reshape the array
    ///
    /// # Errors
    ///
    /// Returns `ArrayError::InvalidShape` if the new shape has a different total number
    /// of elements than the current shape. Also returns errors from shape validation
    /// against the updated specification.
    pub fn reshape(&self, new_shape: Vec<usize>) -> ArrayResult<ArrayData> {
        let new_size = ArraySpec::calculate_size(&new_shape);
        let current_size = ArraySpec::calculate_size(&self.shape);

        if new_size != current_size {
            return Err(ArrayError::InvalidShape(format!(
                "Cannot reshape from {:?} to {:?}: different sizes",
                self.shape, new_shape
            )));
        }

        // Update spec dimensions
        let mut new_spec = self.spec.clone();
        new_spec.dimensions = new_shape
            .iter()
            .enumerate()
            .map(|(i, &size)| ArrayDimension::fixed(format!("dim_{i}"), size))
            .collect();

        // Validate new shape
        new_spec.validate_shape(&new_shape)?;

        Ok(ArrayData {
            spec: new_spec,
            shape: new_shape,
            data: self.data.clone(),
        })
    }

    /// Transpose the array (reverse dimensions)
    ///
    /// # Errors
    ///
    /// Returns an error if the transposition fails due to invalid array structure
    /// or if index calculations during data reordering are invalid.
    ///
    /// # Panics
    ///
    /// Panics if transposed indices cannot be converted to flat index (should never happen)
    pub fn transpose(&self) -> Result<ArrayData, Box<dyn std::error::Error>> {
        let mut new_spec = self.spec.clone();
        new_spec.dimensions.reverse();

        let mut new_shape = self.shape.clone();
        new_shape.reverse();

        // Reorder data
        let mut new_data = vec![serde_json::Value::Null; self.data.len()];

        for i in 0..self.data.len() {
            let indices = self.spec.flat_to_indices(i, &self.shape);
            let mut transposed_indices = indices.clone();
            transposed_indices.reverse();

            let new_flat = new_spec
                .indices_to_flat(&transposed_indices, &new_shape)
                .expect("transposed indices should always be valid: {}");
            new_data[new_flat] = self.data[i].clone();
        }

        Ok(ArrayData {
            spec: new_spec,
            shape: new_shape,
            data: new_data,
        })
    }
}

/// Extension trait for `SlotDefinition` to support arrays
pub trait ArraySlotExt {
    /// Get array specification if this slot is an array
    fn array_spec(&self) -> Option<&ArraySpec>;

    /// Set array specification
    fn set_array_spec(&mut self, spec: ArraySpec);

    /// Check if this slot is an array
    fn is_array(&self) -> bool {
        self.array_spec().is_some()
    }
}

// Note: In a real implementation, we would extend SlotDefinition
// For now, we'll use a separate storage mechanism

/// Array validator
pub struct ArrayValidator;

impl ArrayValidator {
    /// Validate array data against specification
    ///
    /// # Errors
    ///
    /// Returns `ArrayError` if the array shape doesn't match the specification,
    /// or if any element has a type that doesn't match the expected element type
    /// defined in the specification.
    pub fn validate(data: &ArrayData, spec: &ArraySpec) -> ArrayResult<()> {
        // Validate shape
        spec.validate_shape(&data.shape)?;

        // Validate element types
        for (i, value) in data.data.iter().enumerate() {
            Self::validate_element(value, &spec.element_type, i)?;
        }

        Ok(())
    }

    /// Validate a single element
    fn validate_element(
        value: &serde_json::Value,
        expected_type: &str,
        _index: usize,
    ) -> ArrayResult<()> {
        let actual_type = match value {
            serde_json::Value::Null => "null",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(n) => {
                if n.is_f64() {
                    "float"
                } else {
                    "integer"
                }
            }
            serde_json::Value::String(_) => "string",
            serde_json::Value::Array(_) => "array",
            serde_json::Value::Object(_) => "object",
        };

        // Type checking (simplified)
        let valid = match expected_type {
            "float" | "double" | "decimal" => {
                matches!(actual_type, "float" | "integer")
            }
            "integer" => actual_type == "integer",
            "string" => actual_type == "string",
            "boolean" => actual_type == "boolean",
            _ => actual_type == expected_type,
        };

        if !valid {
            return Err(ArrayError::TypeMismatch {
                expected: expected_type.to_string(),
                actual: actual_type.to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_array_dimension() {
        let dim = ArrayDimension::fixed("x", 10);
        assert_eq!(dim.size, Some(10));
        assert!(dim.validate_size(10).is_ok());
        assert!(dim.validate_size(5).is_err());

        let dynamic_dim = ArrayDimension::dynamic("time").with_min(1).with_max(100);
        assert!(dynamic_dim.validate_size(50).is_ok());
        assert!(dynamic_dim.validate_size(0).is_err());
        assert!(dynamic_dim.validate_size(101).is_err());
    }

    #[test]
    fn test_array_spec() {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 3))
            .with_dimension(ArrayDimension::fixed("y", 4));

        assert_eq!(spec.ndim(), 2);
        assert!(spec.is_fixed_shape());
        assert_eq!(spec.fixed_shape(), Some(vec![3, 4]));

        assert!(spec.validate_shape(&[3, 4]).is_ok());
        assert!(spec.validate_shape(&[3, 5]).is_err());
        assert!(spec.validate_shape(&[3]).is_err());
    }

    #[test]
    fn test_array_indexing() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 3))
            .with_dimension(ArrayDimension::fixed("y", 4));

        // Row-major indexing
        assert_eq!(
            spec.indices_to_flat(&[0, 0], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            0
        );
        assert_eq!(
            spec.indices_to_flat(&[0, 1], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            1
        );
        assert_eq!(
            spec.indices_to_flat(&[1, 0], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            4
        );
        assert_eq!(
            spec.indices_to_flat(&[2, 3], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            11
        );

        assert_eq!(spec.flat_to_indices(0, &[3, 4]), vec![0, 0]);
        assert_eq!(spec.flat_to_indices(1, &[3, 4]), vec![0, 1]);
        assert_eq!(spec.flat_to_indices(4, &[3, 4]), vec![1, 0]);
        assert_eq!(spec.flat_to_indices(11, &[3, 4]), vec![2, 3]);

        // Column-major indexing
        let col_spec = spec.column_major();
        assert_eq!(
            col_spec
                .indices_to_flat(&[0, 0], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            0
        );
        assert_eq!(
            col_spec
                .indices_to_flat(&[1, 0], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            1
        );
        assert_eq!(
            col_spec
                .indices_to_flat(&[0, 1], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            3
        );
        assert_eq!(
            col_spec
                .indices_to_flat(&[2, 3], &[3, 4])
                .expect("valid indices should convert to flat index: {}"),
            11
        );
        Ok(())
    }

    #[test]
    fn test_array_data() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer")
            .with_dimension(ArrayDimension::fixed("x", 2))
            .with_dimension(ArrayDimension::fixed("y", 3));

        let data = vec![json!(1), json!(2), json!(3), json!(4), json!(5), json!(6)];

        let array = ArrayData::new(spec, vec![2, 3], data)
            .expect("test data should create valid array: {}");

        assert_eq!(
            array
                .get(&[0, 0])
                .expect("valid indices should return value: {}"),
            &json!(1)
        );
        assert_eq!(
            array
                .get(&[0, 2])
                .expect("valid indices should return value: {}"),
            &json!(3)
        );
        assert_eq!(
            array
                .get(&[1, 1])
                .expect("valid indices should return value: {}"),
            &json!(5)
        );

        // Test slicing
        let slice = array
            .slice(0, 1)
            .expect("valid slice operation should succeed: {}");
        assert_eq!(slice.shape, vec![3]);
        assert_eq!(slice.data, vec![json!(4), json!(5), json!(6)]);

        // Test reshape
        let reshaped = array
            .reshape(vec![3, 2])
            .expect("reshape with same total size should succeed: {}");
        assert_eq!(reshaped.shape, vec![3, 2]);
        assert_eq!(
            reshaped
                .get(&[0, 0])
                .expect("valid indices should return value: {}"),
            &json!(1)
        );
        assert_eq!(
            reshaped
                .get(&[0, 1])
                .expect("valid indices should return value: {}"),
            &json!(2)
        );
        assert_eq!(
            reshaped
                .get(&[1, 0])
                .expect("valid indices should return value: {}"),
            &json!(3)
        );
        Ok(())
    }

    #[test]
    fn test_array_transpose() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer")
            .with_dimension(ArrayDimension::fixed("x", 2))
            .with_dimension(ArrayDimension::fixed("y", 3));

        let data = vec![json!(1), json!(2), json!(3), json!(4), json!(5), json!(6)];

        let array = ArrayData::new(spec, vec![2, 3], data)
            .expect("test data should create valid array - transpose test: {}");
        let transposed = array.transpose()?;

        assert_eq!(transposed.shape, vec![3, 2]);
        assert_eq!(
            transposed
                .get(&[0, 0])
                .expect("valid indices should return value: {}"),
            &json!(1)
        );
        assert_eq!(
            transposed
                .get(&[1, 0])
                .expect("valid indices should return value: {}"),
            &json!(2)
        );
        assert_eq!(
            transposed
                .get(&[0, 1])
                .expect("valid indices should return value: {}"),
            &json!(4)
        );
        assert_eq!(
            transposed
                .get(&[2, 1])
                .expect("valid indices should return value: {}"),
            &json!(6)
        );
        Ok(())
    }
}
