//! String manipulation functions for `LinkML` expressions
//!
//! This module provides string manipulation functions like upper, lower, trim, etc.

use super::functions::{BuiltinFunction, FunctionError};
use serde_json::Value;

/// `upper()` - Convert string to uppercase
pub struct UpperFunction;

impl BuiltinFunction for UpperFunction {
    fn name(&self) -> &'static str {
        "upper"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::String(s) => Ok(Value::String(s.to_uppercase())),
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected string argument",
            )),
        }
    }
}

/// `lower()` - Convert string to lowercase
pub struct LowerFunction;

impl BuiltinFunction for LowerFunction {
    fn name(&self) -> &'static str {
        "lower"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::String(s) => Ok(Value::String(s.to_lowercase())),
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected string argument",
            )),
        }
    }
}

/// `trim()` - Remove leading and trailing whitespace
pub struct TrimFunction;

impl BuiltinFunction for TrimFunction {
    fn name(&self) -> &'static str {
        "trim"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::String(s) => Ok(Value::String(s.trim().to_string())),
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected string argument",
            )),
        }
    }
}

/// `starts_with()` - Check if string starts with prefix
pub struct StartsWithFunction;

impl BuiltinFunction for StartsWithFunction {
    fn name(&self) -> &'static str {
        "starts_with"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(string) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let Value::String(prefix) = &args[1] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a string",
            ));
        };

        Ok(Value::Bool(string.starts_with(prefix)))
    }
}

/// `ends_with()` - Check if string ends with suffix
pub struct EndsWithFunction;

impl BuiltinFunction for EndsWithFunction {
    fn name(&self) -> &'static str {
        "ends_with"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(string) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let Value::String(suffix) = &args[1] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a string",
            ));
        };

        Ok(Value::Bool(string.ends_with(suffix)))
    }
}

/// `replace()` - Replace all occurrences of a substring
pub struct ReplaceFunction;

impl BuiltinFunction for ReplaceFunction {
    fn name(&self) -> &'static str {
        "replace"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 3 {
            return Err(FunctionError::wrong_arity(self.name(), "3", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(string) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let Value::String(from) = &args[1] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "second argument must be a string",
            ));
        };

        let Value::String(to) = &args[2] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "third argument must be a string",
            ));
        };

        Ok(Value::String(string.replace(from, to)))
    }
}

/// `split()` - Split string into array
pub struct SplitFunction;

impl BuiltinFunction for SplitFunction {
    fn name(&self) -> &'static str {
        "split"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(string) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let delimiter = match &args[1] {
            Value::String(s) => s,
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a string",
                ));
            }
        };

        let parts: Vec<Value> = string
            .split(delimiter)
            .map(|s| Value::String(s.to_string()))
            .collect();

        Ok(Value::Array(parts))
    }
}

/// `join()` - Join array elements into string
pub struct JoinFunction;

impl BuiltinFunction for JoinFunction {
    fn name(&self) -> &'static str {
        "join"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let array = match &args[0] {
            Value::Array(arr) => arr,
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "first argument must be an array",
                ));
            }
        };

        let delimiter = match &args[1] {
            Value::String(s) => s,
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a string",
                ));
            }
        };

        let strings: Result<Vec<String>, _> = array
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                _ => Err(FunctionError::invalid_argument(
                    self.name(),
                    "array must contain only strings",
                )),
            })
            .collect();

        match strings {
            Ok(strs) => Ok(Value::String(strs.join(delimiter))),
            Err(e) => Err(e),
        }
    }
}

/// `substring()` - Extract substring
pub struct SubstringFunction;

impl BuiltinFunction for SubstringFunction {
    fn name(&self) -> &'static str {
        "substring"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() < 2 || args.len() > 3 {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "2 or 3",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let Value::String(string) = &args[0] else {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "first argument must be a string",
            ));
        };

        let start = match &args[1] {
            Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a number",
                ));
            }
        };

        let result = if args.len() == 3 {
            let length = match &args[2] {
                Value::Number(n) => n.as_u64().unwrap_or(0) as usize,
                _ => {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "third argument must be a number",
                    ));
                }
            };

            string.chars().skip(start).take(length).collect()
        } else {
            string.chars().skip(start).collect()
        };

        Ok(Value::String(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_upper_lower_trim() -> Result<(), Box<dyn std::error::Error>> {
        let upper = UpperFunction;
        assert_eq!(
            upper
                .call(vec![json!("hello")])
                .expect("should convert to uppercase: {}"),
            json!("HELLO")
        );

        let lower = LowerFunction;
        assert_eq!(
            lower
                .call(vec![json!("HELLO")])
                .expect("should convert to lowercase: {}"),
            json!("hello")
        );

        let trim = TrimFunction;
        assert_eq!(
            trim.call(vec![json!("  hello  ")])
                .expect("should trim whitespace: {}"),
            json!("hello")
        );
        Ok(())
    }

    #[test]
    fn test_starts_ends_with() -> Result<(), Box<dyn std::error::Error>> {
        let starts_with = StartsWithFunction;
        assert_eq!(
            starts_with
                .call(vec![json!("hello world"), json!("hello")])
                .expect("should check starts_with: {}"),
            json!(true)
        );
        assert_eq!(
            starts_with
                .call(vec![json!("hello world"), json!("world")])
                .expect("should check starts_with: {}"),
            json!(false)
        );

        let ends_with = EndsWithFunction;
        assert_eq!(
            ends_with
                .call(vec![json!("hello world"), json!("world")])
                .expect("should check ends_with: {}"),
            json!(true)
        );
        assert_eq!(
            ends_with
                .call(vec![json!("hello world"), json!("hello")])
                .expect("should check ends_with: {}"),
            json!(false)
        );
        Ok(())
    }

    #[test]
    fn test_replace() -> Result<(), Box<dyn std::error::Error>> {
        let replace = ReplaceFunction;
        assert_eq!(
            replace
                .call(vec![json!("hello world"), json!("world"), json!("rust")])
                .expect("should replace substring: {}"),
            json!("hello rust")
        );
        Ok(())
    }

    #[test]
    fn test_split_join() -> Result<(), Box<dyn std::error::Error>> {
        let split = SplitFunction;
        assert_eq!(
            split
                .call(vec![json!("a,b,c"), json!(",")])
                .expect("should split string: {}"),
            json!(["a", "b", "c"])
        );

        let join = JoinFunction;
        assert_eq!(
            join.call(vec![json!(["a", "b", "c"]), json!("-")])
                .expect("should join array: {}"),
            json!("a-b-c")
        );
        Ok(())
    }

    #[test]
    fn test_substring() -> Result<(), Box<dyn std::error::Error>> {
        let substring = SubstringFunction;

        // With length
        assert_eq!(
            substring
                .call(vec![json!("hello world"), json!(6), json!(5)])
                .expect("should extract substring: {}"),
            json!("world")
        );

        // Without length
        assert_eq!(
            substring
                .call(vec![json!("hello world"), json!(6)])
                .expect("should extract substring to end: {}"),
            json!("world")
        );
        Ok(())
    }
}
