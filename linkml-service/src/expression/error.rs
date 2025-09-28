//! Error types for the expression language
//!
//! This module provides comprehensive error handling for the `LinkML` expression language,
//! including parse-time errors and runtime evaluation errors.

use thiserror::Error;

/// Main error type for expression operations
#[derive(Debug, Error)]
pub enum ExpressionError {
    /// Error during parsing
    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    /// Error during evaluation
    #[error("Evaluation error: {0}")]
    Evaluation(#[from] EvaluationError),

    /// Other errors
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for ExpressionError {
    fn from(err: anyhow::Error) -> Self {
        ExpressionError::Other(err.to_string())
    }
}

/// Errors that can occur during parsing
#[derive(Debug, Error)]
pub enum ParseError {
    /// Unexpected end of input
    #[error("Unexpected end of input at position {position}")]
    UnexpectedEof {
        /// Position in the input where parsing failed
        position: usize,
    },

    /// Unexpected token
    #[error("Unexpected token '{token}' at position {position}")]
    UnexpectedToken {
        /// The unexpected token that was encountered
        token: String,
        /// Position in the input where the token was found
        position: usize,
    },

    /// Invalid number format
    #[error("Invalid number '{value}' at position {position}")]
    InvalidNumber {
        /// The invalid numeric string that couldn't be parsed
        value: String,
        /// Position in the input where the number was found
        position: usize,
    },

    /// Invalid string literal
    #[error("Invalid string literal at position {position}: {reason}")]
    InvalidString {
        /// Position in the input where the invalid string was found
        position: usize,
        /// Reason why the string is invalid
        reason: String,
    },

    /// Invalid variable name
    #[error("Invalid variable name '{name}' at position {position}")]
    InvalidVariable {
        /// The invalid variable name that was encountered
        name: String,
        /// Position in the input where the variable was found
        position: usize,
    },

    /// Missing closing delimiter
    #[error("Missing closing '{delimiter}' at position {position}")]
    MissingDelimiter {
        /// The delimiter character that was expected but not found
        delimiter: char,
        /// Position in the input where the delimiter was expected
        position: usize,
    },

    /// Expression too deep
    #[error("Expression nesting depth {depth} exceeds maximum of {max}")]
    TooDeep {
        /// Current nesting depth
        depth: usize,
        /// Maximum allowed nesting depth
        max: usize,
    },

    /// Expression too long
    #[error("Expression length {length} exceeds maximum of {max}")]
    TooLong {
        /// Current expression length
        length: usize,
        /// Maximum allowed expression length
        max: usize,
    },

    /// Unknown function
    #[error("Unknown function '{name}' at position {position}")]
    UnknownFunction {
        /// Name of the function that was not recognized
        name: String,
        /// Position in the input where the function was found
        position: usize,
    },

    /// Wrong number of arguments
    #[error("Function '{name}' expects {expected} arguments, got {actual}")]
    WrongArity {
        /// Name of the function with incorrect argument count
        name: String,
        /// Expected number of arguments (as string for flexibility)
        expected: String,
        /// Actual number of arguments provided
        actual: usize,
    },

    /// Trailing input after expression
    #[error("Unexpected input after expression: '{input}'")]
    TrailingInput {
        /// The unexpected input that remained after parsing
        input: String,
    },

    /// System error (e.g., time operations)
    #[error("System error: {message}")]
    SystemError {
        /// Description of the system error
        message: String,
    },
}

/// Errors that can occur during evaluation
#[derive(Debug, Clone, Error)]
pub enum EvaluationError {
    /// Variable not found in context
    #[error("Undefined variable '{name}'")]
    UndefinedVariable {
        /// Name of the variable that was not found in the evaluation context
        name: String,
    },

    /// Type mismatch in operation
    #[error("Type error: {message}")]
    TypeError {
        /// Description of the type error
        message: String,
    },

    /// Division by zero
    #[error("Division by zero")]
    DivisionByZero,

    /// Invalid operation on null
    #[error("Cannot perform operation on null value")]
    NullOperation,

    /// Function evaluation error
    #[error("Function '{name}' error: {message}")]
    FunctionError {
        /// Name of the function that encountered an error
        name: String,
        /// Error message from the function
        message: String,
    },

    /// Invalid argument for function
    #[error("Invalid argument for function '{function}': {message}")]
    InvalidArgument {
        /// Name of the function that received an invalid argument
        function: String,
        /// Description of why the argument is invalid
        message: String,
    },

    /// Evaluation timeout
    #[error("Expression evaluation timed out after {seconds} seconds")]
    Timeout {
        /// Number of seconds after which evaluation timed out
        seconds: f64,
    },

    /// Too many iterations
    #[error("Expression evaluation exceeded maximum iterations ({max})")]
    TooManyIterations {
        /// Maximum number of iterations allowed
        max: usize,
    },

    /// Call stack too deep
    #[error("Expression evaluation exceeded maximum call depth ({max})")]
    CallStackTooDeep {
        /// Maximum call depth allowed
        max: usize,
    },

    /// Memory limit exceeded
    #[error("Expression evaluation exceeded memory limit ({limit} bytes)")]
    MemoryLimitExceeded {
        /// Memory limit in bytes
        limit: usize,
    },

    /// Overflow in numeric operation
    #[error("Numeric overflow in operation")]
    NumericOverflow,

    /// Invalid regex pattern
    #[error("Invalid regex pattern: {pattern}")]
    InvalidRegex {
        /// The regex pattern string that was invalid
        pattern: String,
    },
}

impl EvaluationError {
    /// Create a type error for binary operations
    #[must_use]
    pub fn binary_type_error(op: &str, left: &str, right: &str) -> Self {
        Self::TypeError {
            message: format!("Cannot {op} values of type {left} and {right}"),
        }
    }

    /// Create a type error for unary operations
    #[must_use]
    pub fn unary_type_error(op: &str, value: &str) -> Self {
        Self::TypeError {
            message: format!("Cannot {op} value of type {value}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ParseError::UnexpectedToken {
            token: "+".to_string(),
            position: 5,
        };
        assert_eq!(err.to_string(), "Unexpected token '+' at position 5");

        let err = EvaluationError::UndefinedVariable {
            name: "foo".to_string(),
        };
        assert_eq!(err.to_string(), "Undefined variable 'foo'");
    }

    #[test]
    fn test_type_error_helpers() {
        let err = EvaluationError::binary_type_error("add", "string", "number");
        assert_eq!(
            err.to_string(),
            "Type error: Cannot add values of type string and number"
        );

        let err = EvaluationError::unary_type_error("negate", "string");
        assert_eq!(
            err.to_string(),
            "Type error: Cannot negate value of type string"
        );
    }
}
