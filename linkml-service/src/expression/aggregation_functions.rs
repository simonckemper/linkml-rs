//! Aggregation functions for `LinkML` expressions
//!
//! This module provides functions for aggregating values from arrays or collections.

use super::functions::{BuiltinFunction, FunctionError};
use serde_json::Value;
use std::collections::HashMap;

/// Convert f64 to `serde_json::Number`, returning error for non-finite values
fn f64_to_number(val: f64, function_name: &str) -> Result<serde_json::Number, FunctionError> {
    serde_json::Number::from_f64(val).ok_or_else(|| {
        FunctionError::invalid_result(
            function_name,
            "result is not a finite number (NaN or infinity)",
        )
    })
}

/// `sum()` - Sum of numeric values
pub struct SumFunction;

impl BuiltinFunction for SumFunction {
    fn name(&self) -> &'static str {
        "sum"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                let mut sum = 0.0;
                for val in arr {
                    match val {
                        Value::Number(n) => sum += n.as_f64().unwrap_or(0.0),
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "array must contain only numbers",
                            ));
                        }
                    }
                }
                Ok(Value::Number(f64_to_number(sum, self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `avg()` - Average of numeric values
pub struct AvgFunction;

impl BuiltinFunction for AvgFunction {
    fn name(&self) -> &'static str {
        "avg"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Value::Null);
                }

                let mut sum = 0.0;
                for val in arr {
                    match val {
                        Value::Number(n) => sum += n.as_f64().unwrap_or(0.0),
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "array must contain only numbers",
                            ));
                        }
                    }
                }

                let avg = sum / arr.len() as f64;
                Ok(Value::Number(f64_to_number(avg, self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `count()` - Count of values (with optional condition)
pub struct CountFunction;

impl BuiltinFunction for CountFunction {
    fn name(&self) -> &'static str {
        "count"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.is_empty() || args.len() > 2 {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "1 or 2",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                let count = if args.len() == 2 {
                    // Count with condition
                    match &args[1] {
                        Value::String(condition) => {
                            // Simple condition support: "non-null", "non-empty", etc.
                            match condition.as_str() {
                                "non-null" => {
                                    arr.iter().filter(|v| !matches!(v, Value::Null)).count()
                                }
                                "non-empty" => arr
                                    .iter()
                                    .filter(|v| match v {
                                        Value::String(s) => !s.is_empty(),
                                        Value::Array(a) => !a.is_empty(),
                                        Value::Object(o) => !o.is_empty(),
                                        Value::Null => false,
                                        _ => true,
                                    })
                                    .count(),
                                _ => {
                                    return Err(FunctionError::invalid_argument(
                                        self.name(),
                                        "unsupported condition",
                                    ));
                                }
                            }
                        }
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "condition must be a string",
                            ));
                        }
                    }
                } else {
                    // Simple count
                    arr.len()
                };

                Ok(Value::Number(serde_json::Number::from(count)))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `median()` - Median of numeric values
pub struct MedianFunction;

impl BuiltinFunction for MedianFunction {
    fn name(&self) -> &'static str {
        "median"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Value::Null);
                }

                // Extract and sort numeric values
                let mut numbers: Vec<f64> = Vec::new();
                for val in arr {
                    match val {
                        Value::Number(n) => numbers.push(n.as_f64().unwrap_or(0.0)),
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "array must contain only numbers",
                            ));
                        }
                    }
                }

                numbers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

                let median = if numbers.len().is_multiple_of(2) {
                    // Even number of elements
                    let mid = numbers.len() / 2;
                    f64::midpoint(numbers[mid - 1], numbers[mid])
                } else {
                    // Odd number of elements
                    numbers[numbers.len() / 2]
                };

                Ok(Value::Number(f64_to_number(median, self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `mode()` - Most frequent value(s)
pub struct ModeFunction;

impl BuiltinFunction for ModeFunction {
    fn name(&self) -> &'static str {
        "mode"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                if arr.is_empty() {
                    return Ok(Value::Null);
                }

                // Count occurrences
                use std::collections::HashMap;
                let mut counts: HashMap<String, (usize, &Value)> = HashMap::new();

                for val in arr {
                    let key = serde_json::to_string(val).unwrap_or_default();
                    counts
                        .entry(key)
                        .and_modify(|(count, _)| *count += 1)
                        .or_insert((1, val));
                }

                // Find maximum count
                let max_count = counts.values().map(|(count, _)| *count).max().unwrap_or(0);

                // Collect all values with max count
                let modes: Vec<Value> = counts
                    .values()
                    .filter(|(count, _)| *count == max_count)
                    .map(|(_, val)| (*val).clone())
                    .collect();

                // Return single value if only one mode, otherwise array
                match modes.len() {
                    0 => Ok(Value::Null),
                    1 => Ok(modes[0].clone()),
                    _ => Ok(Value::Array(modes)),
                }
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `stddev()` - Standard deviation
pub struct StdDevFunction;

impl BuiltinFunction for StdDevFunction {
    fn name(&self) -> &'static str {
        "stddev"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                if arr.len() < 2 {
                    return Ok(Value::Null);
                }

                // Calculate mean
                let mut sum = 0.0;
                let mut numbers = Vec::new();

                for val in arr {
                    match val {
                        Value::Number(n) => {
                            let num = n.as_f64().unwrap_or(0.0);
                            sum += num;
                            numbers.push(num);
                        }
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "array must contain only numbers",
                            ));
                        }
                    }
                }

                let mean = sum / numbers.len() as f64;

                // Calculate variance
                let variance = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / (numbers.len() - 1) as f64;

                // Standard deviation is square root of variance
                let stddev = variance.sqrt();

                Ok(Value::Number(f64_to_number(stddev, self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `variance()` - Variance of numeric values
pub struct VarianceFunction;

impl BuiltinFunction for VarianceFunction {
    fn name(&self) -> &'static str {
        "variance"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                if arr.len() < 2 {
                    return Ok(Value::Null);
                }

                // Calculate mean
                let mut sum = 0.0;
                let mut numbers = Vec::new();

                for val in arr {
                    match val {
                        Value::Number(n) => {
                            let num = n.as_f64().unwrap_or(0.0);
                            sum += num;
                            numbers.push(num);
                        }
                        _ => {
                            return Err(FunctionError::invalid_argument(
                                self.name(),
                                "array must contain only numbers",
                            ));
                        }
                    }
                }

                let mean = sum / numbers.len() as f64;

                // Calculate variance
                let variance = numbers.iter().map(|x| (x - mean).powi(2)).sum::<f64>()
                    / (numbers.len() - 1) as f64;

                Ok(Value::Number(f64_to_number(variance, self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `unique()` - Unique values from array
pub struct UniqueFunction;

impl BuiltinFunction for UniqueFunction {
    fn name(&self) -> &'static str {
        "unique"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Array(arr) => {
                use std::collections::HashSet;
                let mut seen = HashSet::new();
                let mut unique_values = Vec::new();

                for val in arr {
                    let key = serde_json::to_string(val).unwrap_or_default();
                    if seen.insert(key) {
                        unique_values.push(val.clone());
                    }
                }

                Ok(Value::Array(unique_values))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array argument",
            )),
        }
    }
}

/// `group_by()` - Group array elements by a key
pub struct GroupByFunction;

impl BuiltinFunction for GroupByFunction {
    fn name(&self) -> &'static str {
        "group_by"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match (&args[0], &args[1]) {
            (Value::Array(arr), Value::String(key)) => {
                let mut groups: HashMap<String, Vec<Value>> = HashMap::new();

                for val in arr {
                    if let Value::Object(obj) = val {
                        let group_key = match obj.get(key) {
                            Some(v) => {
                                serde_json::to_string(v).unwrap_or_else(|_| "null".to_string())
                            }
                            None => "null".to_string(),
                        };

                        groups.entry(group_key).or_default().push(val.clone());
                    } else {
                        return Err(FunctionError::invalid_argument(
                            self.name(),
                            "array must contain objects",
                        ));
                    }
                }

                // Convert to object with group keys
                let mut result = serde_json::Map::new();
                for (k, v) in groups {
                    result.insert(k, Value::Array(v));
                }

                Ok(Value::Object(result))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected array and string key",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sum_avg() -> Result<(), anyhow::Error> {
        let sum_fn = SumFunction;
        let avg_fn = AvgFunction;

        let numbers = json!([1, 2, 3, 4, 5]);

        assert_eq!(
            sum_fn
                .call(vec![numbers.clone()])
                .expect("should calculate sum: {}"),
            json!(15.0)
        );
        assert_eq!(
            avg_fn
                .call(vec![numbers])
                .expect("should calculate average: {}"),
            json!(3.0)
        );

        // Empty array
        assert_eq!(
            sum_fn
                .call(vec![json!([])])
                .expect("should handle empty array sum: {}"),
            json!(0.0)
        );
        assert_eq!(
            avg_fn
                .call(vec![json!([])])
                .expect("should handle empty array average: {}"),
            json!(null)
        );
        Ok(())
    }

    #[test]
    fn test_count() -> Result<(), anyhow::Error> {
        let count_fn = CountFunction;

        // Simple count
        assert_eq!(
            count_fn
                .call(vec![json!([1, 2, 3, 4, 5])])
                .expect("should count array elements: {}"),
            json!(5)
        );

        // Count non-null
        assert_eq!(
            count_fn
                .call(vec![json!([1, null, 3, null, 5]), json!("non-null")])
                .expect("should count non-null elements: {}"),
            json!(3)
        );

        // Count non-empty
        assert_eq!(
            count_fn
                .call(vec![json!(["a", "", "c", ""]), json!("non-empty")])
                .expect("should count non-empty elements: {}"),
            json!(2)
        );
        Ok(())
    }

    #[test]
    fn test_median() -> Result<(), anyhow::Error> {
        let median_fn = MedianFunction;

        // Odd number of elements
        assert_eq!(
            median_fn
                .call(vec![json!([1, 3, 5, 7, 9])])
                .expect("should calculate median of odd elements: {}"),
            json!(5.0)
        );

        // Even number of elements
        assert_eq!(
            median_fn
                .call(vec![json!([1, 2, 3, 4])])
                .expect("should calculate median of even elements: {}"),
            json!(2.5)
        );

        // Unsorted array
        assert_eq!(
            median_fn
                .call(vec![json!([5, 1, 3, 9, 7])])
                .expect("should calculate median of unsorted array: {}"),
            json!(5.0)
        );
        Ok(())
    }

    #[test]
    fn test_mode() -> Result<(), anyhow::Error> {
        let mode_fn = ModeFunction;

        // Single mode
        assert_eq!(
            mode_fn
                .call(vec![json!([1, 2, 2, 3, 2, 4])])
                .expect("should find single mode: {}"),
            json!(2)
        );

        // Multiple modes
        let result = mode_fn
            .call(vec![json!([1, 1, 2, 2, 3])])
            .expect("should find multiple modes: {}");
        match result {
            Value::Array(arr) => {
                assert_eq!(arr.len(), 2);
                assert!(arr.contains(&json!(1)));
                assert!(arr.contains(&json!(2)));
            }
            _ => assert!(false, "Expected array of modes"),
        }
        Ok(())
    }

    #[test]
    fn test_stddev_variance() -> Result<(), anyhow::Error> {
        let stddev_fn = StdDevFunction;
        let variance_fn = VarianceFunction;

        let data = json!([2, 4, 4, 4, 5, 5, 7, 9]);

        // Sample variance (using n-1) should be 32/7 ≈ 4.571428571428571
        let variance_result = variance_fn
            .call(vec![data.clone()])
            .expect("should calculate variance: {}");
        assert!(
            matches!(variance_result, Value::Number(n) if (n.as_f64().ok_or_else(|| anyhow::anyhow!("should be number"))? - 4.571_428_571_428_571).abs() < 0.0001)
        );

        // Sample standard deviation should be sqrt(32/7) ≈ 2.1380899352993947
        let stddev_result = stddev_fn
            .call(vec![data])
            .expect("should calculate standard deviation: {}");
        assert!(
            matches!(stddev_result, Value::Number(n) if (n.as_f64().ok_or_else(|| anyhow::anyhow!("should be number"))? - 2.138_089_935_299_394_7).abs() < 0.0001)
        );
        Ok(())
    }

    #[test]
    fn test_unique() -> Result<(), Box<dyn std::error::Error>> {
        let unique_fn = UniqueFunction;

        let result = unique_fn
            .call(vec![json!([1, 2, 2, 3, 1, 4, 3])])
            .expect("should find unique numbers: {}");
        assert_eq!(result, json!([1, 2, 3, 4]));

        // With strings
        let result = unique_fn
            .call(vec![json!(["a", "b", "a", "c", "b"])])
            .expect("should find unique strings: {}");
        assert_eq!(result, json!(["a", "b", "c"]));
        Ok(())
    }

    #[test]
    fn test_group_by() -> Result<(), Box<dyn std::error::Error>> {
        let group_by_fn = GroupByFunction;

        let data = json!([
            {"type": "fruit", "name": "apple"},
            {"type": "vegetable", "name": "carrot"},
            {"type": "fruit", "name": "banana"},
            {"type": "vegetable", "name": "lettuce"}
        ]);

        let result = group_by_fn
            .call(vec![data, json!("type")])
            .expect("should group by type field: {}");

        match result {
            Value::Object(groups) => {
                assert_eq!(groups.len(), 2);
                assert!(groups.contains_key("\"fruit\""));
                assert!(groups.contains_key("\"vegetable\""));

                if let Some(Value::Array(fruits)) = groups.get("\"fruit\"") {
                    assert_eq!(fruits.len(), 2);
                }
                if let Some(Value::Array(veggies)) = groups.get("\"vegetable\"") {
                    assert_eq!(veggies.len(), 2);
                }
            }
            _ => assert!(false, "Expected object result"),
        }
        Ok(())
    }
}
