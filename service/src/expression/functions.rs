//! Built-in functions for the `LinkML` expression language

#![allow(missing_docs)]

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

/// Error type for function calls
#[derive(Debug)]
pub struct FunctionError {
    pub message: String,
}

impl fmt::Display for FunctionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for FunctionError {}

impl FunctionError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn wrong_arity(name: &str, expected: &str, actual: usize) -> Self {
        Self {
            message: format!("Function '{name}' expects {expected} arguments, got {actual}"),
        }
    }

    pub fn invalid_argument(name: &str, message: impl Into<String>) -> Self {
        Self {
            message: format!(
                "Invalid argument for function '{}': {}",
                name,
                message.into()
            ),
        }
    }

    pub fn invalid_result(name: &str, message: impl Into<String>) -> Self {
        Self {
            message: format!(
                "Invalid result from function '{}': {}",
                name,
                message.into()
            ),
        }
    }
}

/// Convert f64 to `serde_json::Number`, returning error for non-finite values
fn f64_to_number(val: f64, function_name: &str) -> Result<serde_json::Number, FunctionError> {
    serde_json::Number::from_f64(val).ok_or_else(|| {
        FunctionError::invalid_result(
            function_name,
            "result is not a finite number (NaN or infinity)",
        )
    })
}

/// Function signature trait
pub trait BuiltinFunction: Send + Sync {
    /// Function name
    fn name(&self) -> &str;

    /// Validate argument count
    ///
    /// # Errors
    ///
    /// Returns an error if the number of arguments is invalid for this function
    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError>;

    /// Execute the function
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Function execution fails
    /// - Arguments are invalid or incompatible
    /// - Runtime errors occur during function call
    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError>;
}

/// Custom function implementation wrapper
pub struct CustomFunction {
    name: String,
    min_args: usize,
    max_args: Option<usize>,
    handler: Box<dyn Fn(Vec<Value>) -> Result<Value, FunctionError> + Send + Sync>,
}

impl CustomFunction {
    /// Create a new custom function
    ///
    /// This function cannot fail as it only constructs the function wrapper.
    pub fn new(
        name: impl Into<String>,
        min_args: usize,
        max_args: Option<usize>,
        handler: impl Fn(Vec<Value>) -> Result<Value, FunctionError> + Send + Sync + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            min_args,
            max_args,
            handler: Box::new(handler),
        }
    }
}

impl BuiltinFunction for CustomFunction {
    fn name(&self) -> &str {
        &self.name
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() < self.min_args {
            return Err(FunctionError::wrong_arity(
                &self.name,
                &format!("at least {}", self.min_args),
                args.len(),
            ));
        }
        if let Some(max) = self.max_args
            && args.len() > max
        {
            return Err(FunctionError::wrong_arity(
                &self.name,
                &format!("at most {max}"),
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        (self.handler)(args)
    }
}

/// Registry of built-in functions
pub struct FunctionRegistry {
    functions: HashMap<String, Box<dyn BuiltinFunction>>,
    /// Whether the registry is locked to prevent further registrations
    locked: bool,
}

impl FunctionRegistry {
    /// Create a new function registry with all built-in functions
    #[must_use]
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
            locked: false,
        };

        // Register all built-in functions
        registry.register(Box::new(LenFunction));
        registry.register(Box::new(MaxFunction));
        registry.register(Box::new(MinFunction));
        registry.register(Box::new(CaseFunction));
        registry.register(Box::new(MatchesFunction));
        registry.register(Box::new(ContainsFunction));

        // Register string functions
        registry.register(Box::new(crate::expression::string_functions::UpperFunction));
        registry.register(Box::new(crate::expression::string_functions::LowerFunction));
        registry.register(Box::new(crate::expression::string_functions::TrimFunction));
        registry.register(Box::new(
            crate::expression::string_functions::StartsWithFunction,
        ));
        registry.register(Box::new(
            crate::expression::string_functions::EndsWithFunction,
        ));
        registry.register(Box::new(
            crate::expression::string_functions::ReplaceFunction,
        ));
        registry.register(Box::new(crate::expression::string_functions::SplitFunction));
        registry.register(Box::new(crate::expression::string_functions::JoinFunction));
        registry.register(Box::new(
            crate::expression::string_functions::SubstringFunction,
        ));

        // Register date functions
        // NOTE: NowFunction and TodayFunction require TimestampService dependency
        // They should be registered when the FunctionRegistry is created with service dependencies
        // registry.register(Box::new(crate::expression::date_functions::NowFunction::new(timestamp_service)));
        // registry.register(Box::new(crate::expression::date_functions::TodayFunction::new(timestamp_service)));
        registry.register(Box::new(
            crate::expression::date_functions::DateParseFunction,
        ));
        registry.register(Box::new(
            crate::expression::date_functions::DateFormatFunction,
        ));
        registry.register(Box::new(crate::expression::date_functions::DateAddFunction));
        registry.register(Box::new(
            crate::expression::date_functions::DateDiffFunction,
        ));
        registry.register(Box::new(crate::expression::date_functions::YearFunction));
        registry.register(Box::new(crate::expression::date_functions::MonthFunction));
        registry.register(Box::new(crate::expression::date_functions::DayFunction));

        // Register math functions
        registry.register(Box::new(crate::expression::math_functions::AbsFunction));
        registry.register(Box::new(crate::expression::math_functions::SqrtFunction));
        registry.register(Box::new(crate::expression::math_functions::PowFunction));
        registry.register(Box::new(crate::expression::math_functions::SinFunction));
        registry.register(Box::new(crate::expression::math_functions::CosFunction));
        registry.register(Box::new(crate::expression::math_functions::TanFunction));
        registry.register(Box::new(crate::expression::math_functions::LogFunction));
        registry.register(Box::new(crate::expression::math_functions::ExpFunction));
        registry.register(Box::new(crate::expression::math_functions::FloorFunction));
        registry.register(Box::new(crate::expression::math_functions::CeilFunction));
        registry.register(Box::new(crate::expression::math_functions::RoundFunction));
        registry.register(Box::new(crate::expression::math_functions::ModFunction));

        // Register aggregation functions
        registry.register(Box::new(
            crate::expression::aggregation_functions::SumFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::AvgFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::CountFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::MedianFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::ModeFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::StdDevFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::VarianceFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::UniqueFunction,
        ));
        registry.register(Box::new(
            crate::expression::aggregation_functions::GroupByFunction,
        ));

        registry
    }

    /// Create a registry with security restrictions (no custom functions allowed)
    #[must_use]
    pub fn new_restricted() -> Self {
        let mut registry = Self::new();
        registry.locked = true;
        registry
    }

    /// Lock the registry to prevent further registrations
    pub fn lock(&mut self) {
        self.locked = true;
    }

    /// Check if the registry is locked
    #[must_use]
    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Register a function
    fn register(&mut self, function: Box<dyn BuiltinFunction>) {
        if self.locked {
            // Silently ignore registration attempts when locked
            return;
        }
        self.functions.insert(function.name().to_string(), function);
    }

    /// Register a custom function
    ///
    /// Returns an error if the registry is locked.
    ///
    /// # Example
    ///
    /// ```
    /// use linkml_service::expression::functions::{FunctionRegistry, CustomFunction, FunctionError};
    /// use serde_json::{json, Value};
    ///
    /// let mut registry = FunctionRegistry::new();
    ///
    /// // Register a custom uppercase function
    /// registry.register_custom(CustomFunction::new(
    ///     "uppercase",
    ///     1,
    ///     Some(1),
    ///     |args| {
    ///         match &args[0] {
    ///             Value::String(s) => Ok(Value::String(s.to_uppercase())),
    ///             _ => Err(FunctionError::new("Expected string argument")),
    ///         }
    ///     }
    /// )).expect("should register custom function");
    ///
    /// let result = registry.call("uppercase", vec![json!("hello")]).expect("should call custom function");
    /// assert_eq!(result, json!("HELLO"));
    /// ```
    /// Register a custom function
    ///
    /// # Errors
    ///
    /// Returns an error if the function registry is locked
    pub fn register_custom(&mut self, function: CustomFunction) -> Result<(), FunctionError> {
        if self.locked {
            return Err(FunctionError::new("Function registry is locked"));
        }
        self.register(Box::new(function));
        Ok(())
    }

    /// Call a function by name
    ///
    /// # Errors
    ///
    /// Returns an error if the function is not found, argument count is incorrect,
    /// or the function execution fails
    pub fn call(&self, name: &str, args: Vec<Value>) -> Result<Value, FunctionError> {
        match self.functions.get(name) {
            Some(function) => {
                function.validate_arity(&args)?;
                function.call(args)
            }
            None => Err(FunctionError::new(format!("Unknown function: {name}"))),
        }
    }

    /// Check if a function exists
    #[must_use]
    pub fn has_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
    }

    /// Get list of registered function names
    #[must_use]
    pub fn function_names(&self) -> Vec<&str> {
        self.functions
            .keys()
            .map(std::string::String::as_str)
            .collect()
    }
}

impl Default for FunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in function implementations

/// `len()` - Returns the length of a string, array, or object
struct LenFunction;

impl BuiltinFunction for LenFunction {
    fn name(&self) -> &'static str {
        "len"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let len = match &args[0] {
            Value::String(s) => s.len(),
            Value::Array(arr) => arr.len(),
            Value::Object(obj) => obj.len(),
            Value::Null => 0,
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "expected string, array, or object",
                ));
            }
        };

        Ok(Value::Number(serde_json::Number::from(len as u64)))
    }
}

/// `max()` - Returns the maximum value from arguments
struct MaxFunction;

impl BuiltinFunction for MaxFunction {
    fn name(&self) -> &'static str {
        "max"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.is_empty() {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "at least 1",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let mut max_val: Option<f64> = None;

        for arg in &args {
            match arg {
                Value::Number(n) => {
                    let val = n.as_f64().unwrap_or(0.0);
                    max_val = Some(max_val.map_or(val, |m| m.max(val)));
                }
                _ => {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "all arguments must be numbers",
                    ));
                }
            }
        }

        match max_val {
            Some(val) => Ok(Value::Number(f64_to_number(val, self.name())?)),
            None => Ok(Value::Null),
        }
    }
}

/// `min()` - Returns the minimum value from arguments
struct MinFunction;

impl BuiltinFunction for MinFunction {
    fn name(&self) -> &'static str {
        "min"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.is_empty() {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "at least 1",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let mut min_val: Option<f64> = None;

        for arg in &args {
            match arg {
                Value::Number(n) => {
                    let val = n.as_f64().unwrap_or(0.0);
                    min_val = Some(min_val.map_or(val, |m| m.min(val)));
                }
                _ => {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "all arguments must be numbers",
                    ));
                }
            }
        }

        match min_val {
            Some(val) => Ok(Value::Number(f64_to_number(val, self.name())?)),
            None => Ok(Value::Null),
        }
    }
}

/// `case()` - Multi-way conditional (like a switch statement)
/// case(condition1, value1, condition2, value2, ..., default)
struct CaseFunction;

impl BuiltinFunction for CaseFunction {
    fn name(&self) -> &'static str {
        "case"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() < 3 || args.len().is_multiple_of(2) {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "odd number of arguments (at least 3)",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        // Process pairs of condition, value
        for i in (0..args.len() - 1).step_by(2) {
            if is_truthy(&args[i]) {
                return Ok(args[i + 1].clone());
            }
        }

        // Return default (last argument)
        Ok(args[args.len() - 1].clone())
    }
}

/// `matches()` - Test if a string matches a regex pattern
struct MatchesFunction;

impl BuiltinFunction for MatchesFunction {
    fn name(&self) -> &'static str {
        "matches"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(text) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let Value::String(pattern) = &args[1] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a string pattern",
            ));
        };

        // For now, return a placeholder result
        // In a real implementation, we would compile and match the regex
        Ok(Value::Bool(text.contains(pattern)))
    }
}

/// `contains()` - Test if a value contains another value
struct ContainsFunction;

impl BuiltinFunction for ContainsFunction {
    fn name(&self) -> &'static str {
        "contains"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match (&args[0], &args[1]) {
            (Value::String(haystack), Value::String(needle)) => {
                Ok(Value::Bool(haystack.contains(needle)))
            }
            (Value::Array(arr), item) => Ok(Value::Bool(arr.iter().any(|v| values_equal(v, item)))),
            (Value::Object(obj), Value::String(key)) => Ok(Value::Bool(obj.contains_key(key))),
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "invalid argument types for contains",
            )),
        }
    }
}

// Helper functions

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        (Value::Object(a), Value::Object(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_len_function() -> Result<(), Box<dyn std::error::Error>> {
        let registry = FunctionRegistry::new();

        // String length
        assert_eq!(
            registry
                .call("len", vec![json!("hello")])
                .expect("should calculate string length: {}"),
            json!(5)
        );

        // Array length
        assert_eq!(
            registry
                .call("len", vec![json!([1, 2, 3])])
                .expect("should calculate array length: {}"),
            json!(3)
        );

        // Object length
        assert_eq!(
            registry
                .call("len", vec![json!({"a": 1, "b": 2})])
                .expect("should calculate object length: {}"),
            json!(2)
        );

        // Null length
        assert_eq!(
            registry
                .call("len", vec![json!(null)])
                .expect("should handle null length: {}"),
            json!(0)
        );
        Ok(())
    }

    #[test]
    fn test_max_min_functions() -> Result<(), Box<dyn std::error::Error>> {
        let registry = FunctionRegistry::new();

        // max
        assert_eq!(
            registry
                .call("max", vec![json!(1), json!(5), json!(3)])
                .expect("should find maximum value: {}"),
            json!(5.0)
        );

        // min
        assert_eq!(
            registry
                .call("min", vec![json!(1), json!(5), json!(3)])
                .expect("should find minimum value: {}"),
            json!(1.0)
        );

        // Single value
        assert_eq!(
            registry
                .call("max", vec![json!(42)])
                .expect("should handle single value max: {}"),
            json!(42.0)
        );
        Ok(())
    }

    #[test]
    fn test_case_function() -> Result<(), Box<dyn std::error::Error>> {
        let registry = FunctionRegistry::new();

        // First condition true
        assert_eq!(
            registry
                .call(
                    "case",
                    vec![
                        json!(true),
                        json!("first"),
                        json!(false),
                        json!("second"),
                        json!("default")
                    ]
                )
                .expect("should evaluate case with first condition true: {}"),
            json!("first")
        );

        // Second condition true
        assert_eq!(
            registry
                .call(
                    "case",
                    vec![
                        json!(false),
                        json!("first"),
                        json!(true),
                        json!("second"),
                        json!("default")
                    ]
                )
                .expect("should evaluate case with second condition true: {}"),
            json!("second")
        );

        // Default case
        assert_eq!(
            registry
                .call(
                    "case",
                    vec![
                        json!(false),
                        json!("first"),
                        json!(false),
                        json!("second"),
                        json!("default")
                    ]
                )
                .expect("should evaluate case with default: {}"),
            json!("default")
        );
        Ok(())
    }

    #[test]
    fn test_contains_function() -> Result<(), Box<dyn std::error::Error>> {
        let registry = FunctionRegistry::new();

        // String contains
        assert_eq!(
            registry
                .call("contains", vec![json!("hello world"), json!("world")])
                .expect("should check string contains: {}"),
            json!(true)
        );

        // Array contains
        assert_eq!(
            registry
                .call("contains", vec![json!([1, 2, 3]), json!(2)])
                .expect("should check array contains: {}"),
            json!(true)
        );

        // Object contains key
        assert_eq!(
            registry
                .call("contains", vec![json!({"a": 1, "b": 2}), json!("a")])
                .expect("should check object contains key: {}"),
            json!(true)
        );
        Ok(())
    }

    #[test]
    fn test_function_errors() -> Result<(), Box<dyn std::error::Error>> {
        let registry = FunctionRegistry::new();

        // Wrong arity
        assert!(registry.call("len", vec![]).is_err());
        assert!(registry.call("len", vec![json!(1), json!(2)]).is_err());

        // Unknown function
        assert!(registry.call("unknown", vec![json!(1)]).is_err());

        // Invalid argument type
        assert!(registry.call("max", vec![json!("not a number")]).is_err());
        Ok(())
    }
}
