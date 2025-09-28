//! Mathematical functions for `LinkML` expressions
//!
//! This module provides mathematical functions like trigonometric, logarithmic, and arithmetic operations.

use super::functions::{BuiltinFunction, FunctionError};
use serde_json::Value;

/// Convert f64 to `serde_json::Number`, returning error for non-finite values
fn f64_to_number(val: f64, function_name: &str) -> Result<serde_json::Number, FunctionError> {
    serde_json::Number::from_f64(val).ok_or_else(|| {
        FunctionError::invalid_result(
            function_name,
            "result is not a finite number (NaN or infinity)",
        )
    })
}

/// `abs()` - Absolute value
pub struct AbsFunction;

impl BuiltinFunction for AbsFunction {
    fn name(&self) -> &'static str {
        "abs"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(f64_to_number(val.abs(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `sqrt()` - Square root
pub struct SqrtFunction;

impl BuiltinFunction for SqrtFunction {
    fn name(&self) -> &'static str {
        "sqrt"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                if val < 0.0 {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "cannot take square root of negative number",
                    ));
                }
                Ok(Value::Number(f64_to_number(val.sqrt(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `pow()` - Power function
pub struct PowFunction;

impl BuiltinFunction for PowFunction {
    fn name(&self) -> &'static str {
        "pow"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let base = match &args[0] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "first argument must be a number",
                ));
            }
        };

        let exponent = match &args[1] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a number",
                ));
            }
        };

        Ok(Value::Number(f64_to_number(
            base.powf(exponent),
            self.name(),
        )?))
    }
}

/// `sin()` - Sine function (radians)
pub struct SinFunction;

impl BuiltinFunction for SinFunction {
    fn name(&self) -> &'static str {
        "sin"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(f64_to_number(val.sin(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `cos()` - Cosine function (radians)
pub struct CosFunction;

impl BuiltinFunction for CosFunction {
    fn name(&self) -> &'static str {
        "cos"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(f64_to_number(val.cos(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `tan()` - Tangent function (radians)
pub struct TanFunction;

impl BuiltinFunction for TanFunction {
    fn name(&self) -> &'static str {
        "tan"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(f64_to_number(val.tan(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `log()` - Natural logarithm
pub struct LogFunction;

impl BuiltinFunction for LogFunction {
    fn name(&self) -> &'static str {
        "log"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 && args.len() != 2 {
            return Err(FunctionError::wrong_arity(
                self.name(),
                "1 or 2",
                args.len(),
            ));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let val = match &args[0] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "first argument must be a number",
                ));
            }
        };

        if val <= 0.0 {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "logarithm of non-positive number is undefined",
            ));
        }

        let result = if args.len() == 2 {
            // log with custom base
            let base = match &args[1] {
                Value::Number(n) => n.as_f64().unwrap_or(0.0),
                _ => {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "second argument must be a number",
                    ));
                }
            };

            if base <= 0.0 || base == 1.0 {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "base must be positive and not equal to 1",
                ));
            }

            val.log(base)
        } else {
            // natural logarithm
            val.ln()
        };

        Ok(Value::Number(f64_to_number(result, self.name())?))
    }
}

/// `exp()` - Exponential function (e^x)
pub struct ExpFunction;

impl BuiltinFunction for ExpFunction {
    fn name(&self) -> &'static str {
        "exp"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(f64_to_number(val.exp(), self.name())?))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `floor()` - Round down to nearest integer
pub struct FloorFunction;

impl BuiltinFunction for FloorFunction {
    fn name(&self) -> &'static str {
        "floor"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(serde_json::Number::from(val.floor() as i64)))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `ceil()` - Round up to nearest integer
pub struct CeilFunction;

impl BuiltinFunction for CeilFunction {
    fn name(&self) -> &'static str {
        "ceil"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 1 {
            return Err(FunctionError::wrong_arity(self.name(), "1", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        match &args[0] {
            Value::Number(n) => {
                let val = n.as_f64().unwrap_or(0.0);
                Ok(Value::Number(serde_json::Number::from(val.ceil() as i64)))
            }
            _ => Err(FunctionError::invalid_argument(
                self.name(),
                "expected numeric argument",
            )),
        }
    }
}

/// `round()` - Round to nearest integer or decimal places
pub struct RoundFunction;

impl BuiltinFunction for RoundFunction {
    fn name(&self) -> &'static str {
        "round"
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
        let val = match &args[0] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "first argument must be a number",
                ));
            }
        };

        let result = if args.len() == 2 {
            // Round to specific decimal places
            let places = match &args[1] {
                Value::Number(n) => n.as_i64().unwrap_or(0),
                _ => {
                    return Err(FunctionError::invalid_argument(
                        self.name(),
                        "second argument must be an integer",
                    ));
                }
            };

            #[allow(clippy::cast_possible_truncation)] // places is validated as small integer
            let factor = 10_f64.powi(places as i32);
            (val * factor).round() / factor
        } else {
            // Round to nearest integer
            val.round()
        };

        Ok(Value::Number(f64_to_number(result, self.name())?))
    }
}

/// `mod()` - Modulo operation
pub struct ModFunction;

impl BuiltinFunction for ModFunction {
    fn name(&self) -> &'static str {
        "mod"
    }

    fn validate_arity(&self, args: &[Value]) -> Result<(), FunctionError> {
        if args.len() != 2 {
            return Err(FunctionError::wrong_arity(self.name(), "2", args.len()));
        }
        Ok(())
    }

    fn call(&self, args: Vec<Value>) -> Result<Value, FunctionError> {
        let dividend = match &args[0] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "first argument must be a number",
                ));
            }
        };

        let divisor = match &args[1] {
            Value::Number(n) => n.as_f64().unwrap_or(0.0),
            _ => {
                return Err(FunctionError::invalid_argument(
                    self.name(),
                    "second argument must be a number",
                ));
            }
        };

        if divisor == 0.0 {
            return Err(FunctionError::invalid_argument(
                self.name(),
                "division by zero",
            ));
        }

        Ok(Value::Number(f64_to_number(
            dividend % divisor,
            self.name(),
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_abs_function() -> Result<(), Box<dyn std::error::Error>> {
        let abs_fn = AbsFunction;
        assert_eq!(abs_fn.call(vec![json!(-5)])?, json!(5.0));
        assert_eq!(abs_fn.call(vec![json!(5)])?, json!(5.0));
        assert_eq!(abs_fn.call(vec![json!(-3.14)])?, json!(3.14));
        Ok(())
    }

    #[test]
    fn test_sqrt_function() -> Result<(), Box<dyn std::error::Error>> {
        let sqrt_fn = SqrtFunction;
        assert_eq!(sqrt_fn.call(vec![json!(4)])?, json!(2.0));
        assert_eq!(sqrt_fn.call(vec![json!(9)])?, json!(3.0));
        assert_eq!(
            sqrt_fn.call(vec![json!(2)])?,
            json!(1.414_213_562_373_095_1)
        );

        // Negative number should error
        assert!(sqrt_fn.call(vec![json!(-1)]).is_err());
        Ok(())
    }

    #[test]
    fn test_pow_function() -> Result<(), Box<dyn std::error::Error>> {
        let pow_fn = PowFunction;
        assert_eq!(pow_fn.call(vec![json!(2), json!(3)])?, json!(8.0));
        assert_eq!(pow_fn.call(vec![json!(5), json!(2)])?, json!(25.0));
        assert_eq!(pow_fn.call(vec![json!(10), json!(0)])?, json!(1.0));
        Ok(())
    }

    #[test]
    fn test_trig_functions() -> Result<(), Box<dyn std::error::Error>> {
        let sin_fn = SinFunction;
        let cos_fn = CosFunction;
        let tan_fn = TanFunction;

        // sin(0) = 0
        assert_eq!(sin_fn.call(vec![json!(0)])?, json!(0.0));

        // cos(0) = 1
        assert_eq!(cos_fn.call(vec![json!(0)])?, json!(1.0));

        // tan(0) = 0
        assert_eq!(tan_fn.call(vec![json!(0)])?, json!(0.0));

        // sin(π/2) ≈ 1
        let pi_2 = std::f64::consts::PI / 2.0;
        let sin_result = sin_fn.call(vec![json!(pi_2)])?;
        if let Value::Number(n) = sin_result {
            assert!((n.as_f64().ok_or("Failed to convert to f64")? - 1.0).abs() < 1e-10);
        }
        Ok(())
    }

    #[test]
    fn test_log_function() -> Result<(), Box<dyn std::error::Error>> {
        let log_fn = LogFunction;

        // Natural log
        let e = std::f64::consts::E;
        assert_eq!(log_fn.call(vec![json!(e)])?, json!(1.0));

        // Log base 10
        assert_eq!(log_fn.call(vec![json!(100), json!(10)])?, json!(2.0));

        // Log of non-positive should error
        assert!(log_fn.call(vec![json!(0)]).is_err());
        assert!(log_fn.call(vec![json!(-1)]).is_err());
        Ok(())
    }

    #[test]
    fn test_exp_function() -> Result<(), Box<dyn std::error::Error>> {
        let exp_fn = ExpFunction;
        assert_eq!(exp_fn.call(vec![json!(0)])?, json!(1.0));
        assert_eq!(exp_fn.call(vec![json!(1)])?, json!(std::f64::consts::E));
        Ok(())
    }

    #[test]
    fn test_rounding_functions() -> Result<(), Box<dyn std::error::Error>> {
        let floor_fn = FloorFunction;
        let ceil_fn = CeilFunction;
        let round_fn = RoundFunction;

        // Floor
        assert_eq!(floor_fn.call(vec![json!(3.7)])?, json!(3));
        assert_eq!(floor_fn.call(vec![json!(-3.7)])?, json!(-4));

        // Ceil
        assert_eq!(ceil_fn.call(vec![json!(3.2)])?, json!(4));
        assert_eq!(ceil_fn.call(vec![json!(-3.2)])?, json!(-3));

        // Round
        assert_eq!(round_fn.call(vec![json!(3.5)])?, json!(4.0));
        assert_eq!(round_fn.call(vec![json!(3.2)])?, json!(3.0));
        assert_eq!(round_fn.call(vec![json!(3.14159), json!(2)])?, json!(3.14));
        Ok(())
    }

    #[test]
    fn test_mod_function() -> Result<(), Box<dyn std::error::Error>> {
        let mod_fn = ModFunction;
        assert_eq!(mod_fn.call(vec![json!(10), json!(3)])?, json!(1.0));
        assert_eq!(mod_fn.call(vec![json!(7), json!(4)])?, json!(3.0));

        // Division by zero should error
        assert!(mod_fn.call(vec![json!(10), json!(0)]).is_err());
        Ok(())
    }
}
