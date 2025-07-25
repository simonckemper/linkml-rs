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
pub mod string_functions;
pub mod date_functions;
pub mod math_functions;
pub mod aggregation_functions;

// Performance optimization modules
pub mod compiler;
pub mod vm;
pub mod cache;
pub mod parallel;

use linkml_core::error::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

pub use ast::Expression;
pub use error::{ExpressionError, ParseError, EvaluationError};
pub use evaluator::{Evaluator, EvaluatorConfig};
pub use functions::{CustomFunction, FunctionError, FunctionRegistry};
pub use parser::Parser;
pub use parallel::{ParallelEvaluator, ParallelOptions, ParallelResult, BatchEvaluator};

/// Main expression engine that combines parsing and evaluation
#[derive(Clone)]
pub struct ExpressionEngine {
    parser: Parser,
    evaluator: Arc<Evaluator>,
}

impl ExpressionEngine {
    /// Create a new expression engine with default settings
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            evaluator: Arc::new(Evaluator::new()),
        }
    }
    
    /// Create an expression engine with custom evaluator
    pub fn with_evaluator(evaluator: Arc<Evaluator>) -> Self {
        Self {
            parser: Parser::new(),
            evaluator,
        }
    }
    
    /// Parse an expression string into an AST
    pub fn parse(&self, expression: &str) -> Result<Expression> {
        self.parser.parse(expression)
            .map_err(|e| linkml_core::error::LinkMLError::other(format!("Expression parse error: {}", e)))
    }
    
    /// Evaluate an expression with the given variable context
    pub fn evaluate(
        &self,
        expression: &str,
        context: &HashMap<String, Value>,
    ) -> Result<Value> {
        let ast = self.parse(expression)?;
        self.evaluator.evaluate(&ast, context)
            .map_err(|e| linkml_core::error::LinkMLError::other(format!("Expression evaluation error: {}", e)))
    }
    
    /// Evaluate a pre-parsed expression
    pub fn evaluate_ast(
        &self,
        ast: &Expression,
        context: &HashMap<String, Value>,
    ) -> Result<Value> {
        self.evaluator.evaluate(ast, context)
            .map_err(|e| linkml_core::error::LinkMLError::other(format!("Expression evaluation error: {}", e)))
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
    fn test_expression_engine_creation() {
        let engine = ExpressionEngine::new();
        // Test that engine can parse and evaluate
        let expr = engine.parse("1 + 2").expect("should parse simple expression");
        let result = engine.evaluate_ast(&expr, &HashMap::new()).expect("should evaluate simple expression");
        assert_eq!(result, serde_json::json!(3.0));
    }
}