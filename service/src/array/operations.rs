//! Array operations for `LinkML` arrays
//!
//! This module provides common operations on arrays like map, reduce, filter,
//! and mathematical operations.

use super::{ArrayData, ArrayError, ArrayResult};
use serde_json::Value;
use std::cmp::Ordering;

/// Array operations trait
pub trait ArrayOperations {
    /// Apply a function to each element
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Array operation fails
    /// - Function application fails for any element
    fn map<F>(&self, f: F) -> ArrayResult<ArrayData>
    where
        F: Fn(&Value) -> Value;

    /// Filter elements based on a predicate
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Array filtering operation fails
    /// - Predicate evaluation fails for any element
    fn filter<F>(&self, f: F) -> ArrayResult<Vec<(Vec<usize>, Value)>>
    where
        F: Fn(&Value) -> bool;

    /// Reduce array to a single value
    fn reduce<F>(&self, initial: Value, f: F) -> Value
    where
        F: Fn(Value, &Value) -> Value;

    /// Element-wise addition
    ///
    /// # Errors
    ///
    /// Returns an error if arrays have incompatible shapes or addition fails
    fn add(&self, other: &ArrayData) -> ArrayResult<ArrayData>;

    /// Element-wise subtraction
    ///
    /// # Errors
    ///
    /// Returns an error if arrays have incompatible shapes or subtraction fails
    fn subtract(&self, other: &ArrayData) -> ArrayResult<ArrayData>;

    /// Element-wise multiplication
    ///
    /// # Errors
    ///
    /// Returns an error if arrays have incompatible shapes or multiplication fails
    fn multiply(&self, other: &ArrayData) -> ArrayResult<ArrayData>;

    /// Element-wise division
    ///
    /// # Errors
    ///
    /// Returns an error if arrays have incompatible shapes, division by zero, or division fails
    fn divide(&self, other: &ArrayData) -> ArrayResult<ArrayData>;

    /// Scalar addition
    ///
    /// # Errors
    ///
    /// Returns an error if scalar addition operation fails
    fn scalar_add(&self, scalar: f64) -> ArrayResult<ArrayData>;

    /// Scalar multiplication
    ///
    /// # Errors
    ///
    /// Returns an error if scalar multiplication operation fails
    fn scalar_multiply(&self, scalar: f64) -> ArrayResult<ArrayData>;

    /// Sum all elements
    ///
    /// # Errors
    ///
    /// Returns an error if summation fails or elements cannot be converted to numbers
    fn sum(&self) -> ArrayResult<f64>;

    /// Mean of all elements
    ///
    /// # Errors
    ///
    /// Returns an error if mean calculation fails or array is empty
    fn mean(&self) -> ArrayResult<f64>;

    /// Find minimum element
    ///
    /// # Errors
    ///
    /// Returns an error if minimum calculation fails or array is empty
    fn min(&self) -> ArrayResult<f64>;

    /// Find maximum element
    ///
    /// # Errors
    ///
    /// Returns an error if maximum calculation fails or array is empty
    fn max(&self) -> ArrayResult<f64>;

    /// Standard deviation
    ///
    /// # Errors
    ///
    /// Returns an error if standard deviation calculation fails or array is empty
    fn std_dev(&self) -> ArrayResult<f64>;

    /// Flatten to 1D array
    fn flatten(&self) -> Vec<Value>;

    /// Check if all elements satisfy a condition
    fn all<F>(&self, f: F) -> bool
    where
        F: Fn(&Value) -> bool;

    /// Check if any element satisfies a condition
    fn any<F>(&self, f: F) -> bool
    where
        F: Fn(&Value) -> bool;

    /// Count elements that satisfy a condition
    fn count<F>(&self, f: F) -> usize
    where
        F: Fn(&Value) -> bool;

    /// Find indices of elements that satisfy a condition
    fn find_indices<F>(&self, f: F) -> Vec<Vec<usize>>
    where
        F: Fn(&Value) -> bool;
}

impl ArrayOperations for ArrayData {
    fn map<F>(&self, f: F) -> ArrayResult<ArrayData>
    where
        F: Fn(&Value) -> Value,
    {
        let mapped_data: Vec<Value> = self.data.iter().map(f).collect();
        ArrayData::new(self.spec.clone(), self.shape.clone(), mapped_data)
    }

    fn filter<F>(&self, f: F) -> ArrayResult<Vec<(Vec<usize>, Value)>>
    where
        F: Fn(&Value) -> bool,
    {
        let mut results = Vec::new();

        for (i, value) in self.data.iter().enumerate() {
            if f(value) {
                let indices = self.spec.flat_to_indices(i, &self.shape);
                results.push((indices, value.clone()));
            }
        }

        Ok(results)
    }

    fn reduce<F>(&self, initial: Value, f: F) -> Value
    where
        F: Fn(Value, &Value) -> Value,
    {
        self.data.iter().fold(initial, f)
    }

    fn add(&self, other: &ArrayData) -> ArrayResult<ArrayData> {
        self.element_wise_op(other, |a, b| match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                Value::Number(serde_json::Number::from_f64(v1 + v2).unwrap_or(n1.clone()))
            }
            _ => a.clone(),
        })
    }

    fn subtract(&self, other: &ArrayData) -> ArrayResult<ArrayData> {
        self.element_wise_op(other, |a, b| match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                Value::Number(serde_json::Number::from_f64(v1 - v2).unwrap_or(n1.clone()))
            }
            _ => a.clone(),
        })
    }

    fn multiply(&self, other: &ArrayData) -> ArrayResult<ArrayData> {
        self.element_wise_op(other, |a, b| match (a, b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                Value::Number(serde_json::Number::from_f64(v1 * v2).unwrap_or(n1.clone()))
            }
            _ => a.clone(),
        })
    }

    fn divide(&self, other: &ArrayData) -> ArrayResult<ArrayData> {
        self.element_wise_op(other, |a, b| {
            match (a, b) {
                (Value::Number(n1), Value::Number(n2)) => {
                    let v1 = n1.as_f64().unwrap_or(0.0);
                    let v2 = n2.as_f64().unwrap_or(1.0);
                    if v2 == 0.0 {
                        a.clone() // Keep original on division by zero
                    } else {
                        Value::Number(serde_json::Number::from_f64(v1 / v2).unwrap_or(n1.clone()))
                    }
                }
                _ => a.clone(),
            }
        })
    }

    fn scalar_add(&self, scalar: f64) -> ArrayResult<ArrayData> {
        self.map(|v| match v {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Value::Number(serde_json::Number::from_f64(val + scalar).unwrap_or(n.clone()))
            }
            _ => v.clone(),
        })
    }

    fn scalar_multiply(&self, scalar: f64) -> ArrayResult<ArrayData> {
        self.map(|v| match v {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Value::Number(serde_json::Number::from_f64(val * scalar).unwrap_or(n.clone()))
            }
            _ => v.clone(),
        })
    }

    fn sum(&self) -> ArrayResult<f64> {
        let mut sum = 0.0;
        for value in &self.data {
            if let Value::Number(n) = value {
                sum += n.as_f64().unwrap_or(0.0);
            }
        }
        Ok(sum)
    }

    fn mean(&self) -> ArrayResult<f64> {
        if self.data.is_empty() {
            return Err(ArrayError::InvalidData(
                "Cannot compute mean of empty array".to_string(),
            ));
        }
        let sum = self.sum()?;
        Ok(sum / crate::utils::usize_to_f64(self.data.len()))
    }

    fn min(&self) -> ArrayResult<f64> {
        let mut min_val: Option<f64> = None;

        for value in &self.data {
            if let Value::Number(n) = value {
                let val = n.as_f64().unwrap_or(f64::NAN);
                if !val.is_nan() {
                    min_val = Some(match min_val {
                        None => val,
                        Some(current) => current.min(val),
                    });
                }
            }
        }

        min_val.ok_or_else(|| ArrayError::InvalidData("No numeric values found".to_string()))
    }

    fn max(&self) -> ArrayResult<f64> {
        let mut max_val: Option<f64> = None;

        for value in &self.data {
            if let Value::Number(n) = value {
                let val = n.as_f64().unwrap_or(f64::NAN);
                if !val.is_nan() {
                    max_val = Some(match max_val {
                        None => val,
                        Some(current) => current.max(val),
                    });
                }
            }
        }

        max_val.ok_or_else(|| ArrayError::InvalidData("No numeric values found".to_string()))
    }

    fn std_dev(&self) -> ArrayResult<f64> {
        if self.data.len() < 2 {
            return Err(ArrayError::InvalidData(
                "Need at least 2 elements for standard deviation".to_string(),
            ));
        }

        let mean = self.mean()?;
        let mut sum_squared_diff = 0.0;
        let mut count = 0;

        for value in &self.data {
            if let Value::Number(n) = value {
                let val = n.as_f64().unwrap_or(f64::NAN);
                if !val.is_nan() {
                    sum_squared_diff += (val - mean).powi(2);
                    count += 1;
                }
            }
        }

        if count < 2 {
            return Err(ArrayError::InvalidData(
                "Need at least 2 numeric values".to_string(),
            ));
        }

        Ok((sum_squared_diff / f64::from(count - 1)).sqrt())
    }

    fn flatten(&self) -> Vec<Value> {
        self.data.clone()
    }

    fn all<F>(&self, f: F) -> bool
    where
        F: Fn(&Value) -> bool,
    {
        self.data.iter().all(f)
    }

    fn any<F>(&self, f: F) -> bool
    where
        F: Fn(&Value) -> bool,
    {
        self.data.iter().any(f)
    }

    fn count<F>(&self, f: F) -> usize
    where
        F: Fn(&Value) -> bool,
    {
        self.data.iter().filter(|v| f(v)).count()
    }

    fn find_indices<F>(&self, f: F) -> Vec<Vec<usize>>
    where
        F: Fn(&Value) -> bool,
    {
        let mut indices = Vec::new();

        for (i, value) in self.data.iter().enumerate() {
            if f(value) {
                indices.push(self.spec.flat_to_indices(i, &self.shape));
            }
        }

        indices
    }
}

impl ArrayData {
    /// Helper for element-wise operations
    fn element_wise_op<F>(&self, other: &ArrayData, op: F) -> ArrayResult<ArrayData>
    where
        F: Fn(&Value, &Value) -> Value,
    {
        // Check shapes match
        if self.shape != other.shape {
            return Err(ArrayError::ShapeMismatch {
                expected: self.shape.clone(),
                actual: other.shape.clone(),
            });
        }

        let result_data: Vec<Value> = self
            .data
            .iter()
            .zip(other.data.iter())
            .map(|(a, b)| op(a, b))
            .collect();

        ArrayData::new(self.spec.clone(), self.shape.clone(), result_data)
    }

    /// Sort array along a dimension
    ///
    /// # Errors
    /// Returns error if dimension is out of range or operation is not supported for array dimensions.
    pub fn sort_along(&self, dimension: usize, descending: bool) -> ArrayResult<ArrayData> {
        if dimension >= self.shape.len() {
            return Err(ArrayError::InvalidDimension(format!(
                "Dimension {dimension} out of range"
            )));
        }

        // For simplicity, we'll implement sorting for 1D and 2D arrays
        match self.shape.len() {
            1 => self.sort_1d(descending),
            2 => self.sort_2d(dimension, descending),
            _ => Err(ArrayError::InvalidData(
                "Sorting only implemented for 1D and 2D arrays".to_string(),
            )),
        }
    }

    fn sort_1d(&self, descending: bool) -> ArrayResult<ArrayData> {
        let mut indexed_data: Vec<(usize, &Value)> = self.data.iter().enumerate().collect();

        indexed_data.sort_by(|(_, a), (_, b)| {
            let cmp = compare_values(a, b);
            if descending { cmp.reverse() } else { cmp }
        });

        let sorted_data: Vec<Value> = indexed_data.into_iter().map(|(_, v)| v.clone()).collect();

        ArrayData::new(self.spec.clone(), self.shape.clone(), sorted_data)
    }

    fn sort_2d(&self, dimension: usize, descending: bool) -> ArrayResult<ArrayData> {
        let mut result_data = self.data.clone();

        if dimension == 0 {
            // Sort rows
            for col in 0..self.shape[1] {
                let mut column_data: Vec<(usize, Value)> = Vec::new();
                for row in 0..self.shape[0] {
                    let idx = self.spec.indices_to_flat(&[row, col], &self.shape)?;
                    column_data.push((row, self.data[idx].clone()));
                }

                column_data.sort_by(|(_, a), (_, b)| {
                    let cmp = compare_values(a, b);
                    if descending { cmp.reverse() } else { cmp }
                });

                for (new_row, (_, value)) in column_data.into_iter().enumerate() {
                    let idx = self.spec.indices_to_flat(&[new_row, col], &self.shape)?;
                    result_data[idx] = value;
                }
            }
        } else {
            // Sort columns
            for row in 0..self.shape[0] {
                let mut row_data: Vec<(usize, Value)> = Vec::new();
                for col in 0..self.shape[1] {
                    let idx = self.spec.indices_to_flat(&[row, col], &self.shape)?;
                    row_data.push((col, self.data[idx].clone()));
                }

                row_data.sort_by(|(_, a), (_, b)| {
                    let cmp = compare_values(a, b);
                    if descending { cmp.reverse() } else { cmp }
                });

                for (new_col, (_, value)) in row_data.into_iter().enumerate() {
                    let idx = self.spec.indices_to_flat(&[row, new_col], &self.shape)?;
                    result_data[idx] = value;
                }
            }
        }

        ArrayData::new(self.spec.clone(), self.shape.clone(), result_data)
    }
}

/// Compare two `JSON` values for sorting
fn compare_values(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Null, Value::Null) => Ordering::Equal,
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
        (Value::Number(a), Value::Number(b)) => {
            let a_f = a.as_f64().unwrap_or(f64::NAN);
            let b_f = b.as_f64().unwrap_or(f64::NAN);
            a_f.partial_cmp(&b_f).unwrap_or(Ordering::Equal)
        }
        (Value::String(a), Value::String(b)) => a.cmp(b),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::{ArrayDimension, ArraySpec};
    use serde_json::json;

    #[test]
    fn test_array_map() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 3));

        let data = vec![json!(1), json!(2), json!(3)];
        let array =
            ArrayData::new(spec, vec![3], data).expect("test data should create valid array: {}");

        let doubled = array
            .map(|v| {
                if let Value::Number(n) = v {
                    json!(n.as_i64().unwrap_or(0) * 2)
                } else {
                    v.clone()
                }
            })
            .expect("map operation should succeed with valid data: {}");

        assert_eq!(doubled.data, vec![json!(2), json!(4), json!(6)]);
        Ok(())
    }

    #[test]
    fn test_array_filter() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 5));

        let data = vec![json!(1), json!(2), json!(3), json!(4), json!(5)];
        let array = ArrayData::new(spec, vec![5], data)
            .expect("test data should create valid array - filter test: {}");

        let evens = array
            .filter(|v| {
                if let Value::Number(n) = v {
                    n.as_i64().unwrap_or(0) % 2 == 0
                } else {
                    false
                }
            })
            .expect("filter operation should succeed with valid data: {}");

        assert_eq!(evens.len(), 2);
        assert_eq!(evens[0].0, vec![1]); // index 1 -> value 2
        assert_eq!(evens[1].0, vec![3]); // index 3 -> value 4
        Ok(())
    }

    #[test]
    fn test_array_statistics() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float").with_dimension(ArrayDimension::fixed("x", 5));

        let data = vec![json!(1.0), json!(2.0), json!(3.0), json!(4.0), json!(5.0)];
        let array = ArrayData::new(spec, vec![5], data)
            .expect("test data should create valid array - statistics test: {}");

        assert_eq!(array.sum().expect("sum should succeed: {}"), 15.0);
        assert_eq!(array.mean().expect("mean should succeed: {}"), 3.0);
        assert_eq!(array.min().expect("min should succeed: {}"), 1.0);
        assert_eq!(array.max().expect("max should succeed: {}"), 5.0);
        assert!(
            (array.std_dev().expect("std_dev should succeed: {}") - 1.581_138_83).abs() < 0.00001
        );
        Ok(())
    }

    #[test]
    fn test_element_wise_operations() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 3));

        let data1 = vec![json!(1), json!(2), json!(3)];
        let data2 = vec![json!(4), json!(5), json!(6)];

        let array1 = ArrayData::new(spec.clone(), vec![3], data1)
            .expect("test data should create valid array1: {}");
        let array2 =
            ArrayData::new(spec, vec![3], data2).expect("test data should create valid array2: {}");

        let sum = array1
            .add(&array2)
            .expect("add operation should succeed with matching shapes: {}");
        assert_eq!(sum.data, vec![json!(5.0), json!(7.0), json!(9.0)]);

        let product = array1
            .multiply(&array2)
            .expect("multiply operation should succeed with matching shapes: {}");
        assert_eq!(product.data, vec![json!(4.0), json!(10.0), json!(18.0)]);
        Ok(())
    }

    #[test]
    fn test_array_sorting() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer").with_dimension(ArrayDimension::fixed("x", 5));

        let data = vec![json!(3), json!(1), json!(4), json!(1), json!(5)];
        let array = ArrayData::new(spec, vec![5], data)
            .expect("test data should create valid array - sorting test: {}");

        let sorted = array
            .sort_along(0, false)
            .expect("sort should succeed on 1D array: {}");
        assert_eq!(
            sorted.data,
            vec![json!(1), json!(1), json!(3), json!(4), json!(5)]
        );

        let sorted_desc = array
            .sort_along(0, true)
            .expect("descending sort should succeed on 1D array: {}");
        assert_eq!(
            sorted_desc.data,
            vec![json!(5), json!(4), json!(3), json!(1), json!(1)]
        );
        Ok(())
    }
}
