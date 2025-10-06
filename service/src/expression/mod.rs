//! Expression language for LinkML
//!
//! This module implements a safe, sandboxed expression language for computed fields
//! and dynamic validation in LinkML schemas.

pub mod ast;
pub mod error;
pub mod evaluator;
pub mod functions;
pub mod parser;

// Additional function modules
pub mod aggregation_functions;
pub mod date_functions;
pub mod math_functions;
pub mod string_functions;

// Performance optimization modules
pub mod cache;
pub mod cache_v2;
pub mod compiler;
pub mod engine_v2;
pub mod parallel;
pub mod vm;

use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use timestamp_core::SyncTimestampService;

pub use ast::Expression;
pub use error::{EvaluationError, ExpressionError, ParseError};
pub use evaluator::{Evaluator, EvaluatorConfig};
pub use functions::{CustomFunction, FunctionError, FunctionRegistry};
pub use parallel::{BatchEvaluator, ParallelEvaluator, ParallelOptions, ParallelResult};
pub use parser::Parser;

/// Main expression engine that combines parsing and evaluation
#[derive(Clone)]
pub struct ExpressionEngine {
    parser: Parser,
    evaluator: Arc<Evaluator>,
    timestamp_service: Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>>,
}

impl ExpressionEngine {
    /// Create a new expression engine with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            evaluator: Arc::new(Evaluator::new()),
            timestamp_service: timestamp_service::wiring::wire_sync_timestamp(),
        }
    }

    /// Create an expression engine with custom evaluator
    #[must_use]
    pub fn with_evaluator(evaluator: Arc<Evaluator>) -> Self {
        Self {
            parser: Parser::new(),
            evaluator,
            timestamp_service: timestamp_service::wiring::wire_sync_timestamp(),
        }
    }

    /// Create an expression engine with injected dependencies (factory pattern compliant)
    pub fn with_dependencies<T>(evaluator: Arc<Evaluator>, timestamp_service: Arc<T>) -> Self
    where
        T: SyncTimestampService<Error = timestamp_core::TimestampError> + Send + Sync + 'static,
    {
        Self {
            parser: Parser::new(),
            evaluator,
            timestamp_service,
        }
    }

    /// Parse an expression string into an AST
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn parse(&self, expression: &str) -> linkml_core::error::Result<Expression> {
        self.parser.parse(expression).map_err(|e| {
            linkml_core::error::LinkMLError::other(format!("Expression parse error: {e}"))
        })
    }

    /// Evaluate an expression with the given variable context
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> linkml_core::error::Result<Value> {
        let ast = self.parse(expression)?;
        self.evaluator.evaluate(&ast, context).map_err(|e| {
            linkml_core::error::LinkMLError::other(format!("Expression evaluation error: {e}"))
        })
    }

    /// Evaluate a pre-parsed expression
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn evaluate_ast(
        &self,
        ast: &Expression,
        context: &HashMap<String, Value>,
    ) -> linkml_core::error::Result<Value> {
        self.evaluator.evaluate(ast, context).map_err(|e| {
            linkml_core::error::LinkMLError::other(format!("Expression evaluation error: {e}"))
        })
    }

    /// Get the timestamp service (internal use)
    pub(crate) fn timestamp_service(
        &self,
    ) -> &Arc<dyn SyncTimestampService<Error = timestamp_core::TimestampError>> {
        &self.timestamp_service
    }
}

impl Default for ExpressionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_engine_creation() -> std::result::Result<(), anyhow::Error> {
        let engine = ExpressionEngine::new();
        // Test that engine can parse and evaluate
        let expr = engine
            .parse("1 + 2")
            .expect("should parse simple expression: {}");
        let result = engine
            .evaluate_ast(&expr, &HashMap::new())
            .expect("should evaluate simple expression: {}");
        assert_eq!(result, serde_json::json!(3.0));
        Ok(())
    }
}
