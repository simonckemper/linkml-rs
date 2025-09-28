//! Expression to `TypeQL` translation for rule generation
//!
//! This module translates ``LinkML`` expressions to `TypeQL` rule patterns,
//! supporting arithmetic, comparison, logical operations, and functions.

use crate::expression::ast::Expression;
use std::collections::HashMap;
use std::fmt::Write;
use thiserror::Error;

/// Errors that can occur during expression translation
#[derive(Debug, Error)]
pub enum TranslationError {
    /// Expression type not supported in `TypeQL`
    #[error("Unsupported expression type: {0}")]
    UnsupportedExpression(String),

    /// Invalid variable reference in expression
    #[error("Invalid variable reference: {0}")]
    InvalidVariable(String),

    /// Function not supported in `TypeQL`
    #[error("Function not supported in TypeQL: {0}")]
    UnsupportedFunction(String),

    /// Expression requires decomposition for `TypeQL`
    #[error("Complex expression requires decomposition: {0}")]
    ComplexExpression(String),
}

/// Context for expression translation
#[derive(Debug, Clone)]
pub struct TranslationContext {
    /// Variable bindings (``LinkML`` name -> `TypeQL` variable)
    pub variables: HashMap<String, String>,
    /// Entity variable for the current context
    pub entity_var: String,
    /// Counter for generating unique variables
    pub var_counter: usize,
    /// Whether we're in a negation context
    pub negated: bool,
}

impl TranslationContext {
    /// Create a new translation context
    #[must_use]
    pub fn new(entity_var: String) -> Self {
        Self {
            variables: HashMap::new(),
            entity_var,
            var_counter: 0,
            negated: false,
        }
    }

    /// Generate a new unique variable
    pub fn new_var(&mut self) -> String {
        self.var_counter += 1;
        format!("$v{}", self.var_counter)
    }

    /// Get or create a variable for an attribute
    pub fn get_attribute_var(&mut self, attr_name: &str) -> String {
        if let Some(var) = self.variables.get(attr_name) {
            var.clone()
        } else {
            let var = self.new_var();
            self.variables.insert(attr_name.to_string(), var.clone());
            var
        }
    }
}

/// Result of translating an expression
#[derive(Debug, Clone)]
pub struct TranslatedExpression {
    /// `TypeQL` patterns for the 'when' clause
    pub patterns: Vec<String>,
    /// The resulting TypeQL expression or variable reference
    pub result: String,
    /// Additional variables that need to be bound
    pub bindings: HashMap<String, String>,
}

/// Function handler type
type FunctionHandler = Box<
    dyn Fn(&[Expression], &mut TranslationContext) -> Result<TranslatedExpression, TranslationError>
        + Send
        + Sync,
>;

/// Translates `LinkML` expressions to `TypeQL` patterns.
pub struct ExpressionTranslator {
    /// Supported function mappings
    function_handlers: HashMap<String, FunctionHandler>,
}

impl Default for ExpressionTranslator {
    fn default() -> Self {
        Self::new()
    }
}

impl ExpressionTranslator {
    /// Create a new expression translator
    #[must_use]
    pub fn new() -> Self {
        let mut translator = Self {
            function_handlers: HashMap::new(),
        };

        // Register built-in function handlers
        translator.register_builtin_functions();
        translator
    }

    /// Register built-in function handlers
    fn register_builtin_functions(&mut self) {
        // len() function
        self.function_handlers.insert(
            "len".to_string(),
            Box::new(|args, _ctx| {
                if args.len() != 1 {
                    return Err(TranslationError::UnsupportedExpression(
                        "len() requires exactly 1 argument".to_string(),
                    ));
                }

                // For collections, we need to count in TypeQL
                // This is complex and may require a computed attribute
                Err(TranslationError::ComplexExpression(
                    "len() requires computed attribute in TypeQL".to_string(),
                ))
            }),
        );

        // contains() function - can be translated to pattern matching
        self.function_handlers.insert(
            "contains".to_string(),
            Box::new(|args, ctx| {
                if args.len() != 2 {
                    return Err(TranslationError::UnsupportedExpression(
                        "contains() requires exactly 2 arguments".to_string(),
                    ));
                }

                // Translate to TypeQL pattern matching
                // Example: contains({tags}, "important") -> $entity has tags "important"
                match (&args[0], &args[1]) {
                    (Expression::Variable(var_name), Expression::String(value)) => {
                        let pattern = format!("{} has {} \"{}\"", ctx.entity_var, var_name, value);
                        Ok(TranslatedExpression {
                            patterns: vec![pattern],
                            result: "true".to_string(),
                            bindings: HashMap::new(),
                        })
                    }
                    _ => Err(TranslationError::ComplexExpression(
                        "contains() with complex arguments".to_string(),
                    )),
                }
            }),
        );
    }

    /// Translate a ``LinkML`` expression to `TypeQL` patterns
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn translate(
        &self,
        expr: &Expression,
        ctx: &mut TranslationContext,
    ) -> Result<TranslatedExpression, TranslationError> {
        match expr {
            Expression::Null => self.translate_literal_null(),
            Expression::Boolean(b) => Ok(self.translate_literal_bool(*b)),
            Expression::Number(n) => Ok(self.translate_literal_number(*n)),
            Expression::String(s) => Ok(self.translate_literal_string(s)),
            Expression::Variable(name) => Ok(self.translate_variable(name, ctx)),
            Expression::Add(left, right) => self.translate_binary_op(left, "Add", right, ctx),
            Expression::Subtract(left, right) => {
                self.translate_binary_op(left, "Subtract", right, ctx)
            }
            Expression::Multiply(left, right) => {
                self.translate_binary_op(left, "Multiply", right, ctx)
            }
            Expression::Divide(left, right) => self.translate_binary_op(left, "Divide", right, ctx),
            Expression::Equal(left, right) => self.translate_binary_op(left, "Equal", right, ctx),
            Expression::NotEqual(left, right) => {
                self.translate_binary_op(left, "NotEqual", right, ctx)
            }
            Expression::Less(left, right) => self.translate_binary_op(left, "Less", right, ctx),
            Expression::LessOrEqual(left, right) => {
                self.translate_binary_op(left, "LessEqual", right, ctx)
            }
            Expression::Greater(left, right) => {
                self.translate_binary_op(left, "Greater", right, ctx)
            }
            Expression::GreaterOrEqual(left, right) => {
                self.translate_binary_op(left, "GreaterEqual", right, ctx)
            }
            Expression::And(left, right) => self.translate_binary_op(left, "And", right, ctx),
            Expression::Or(left, right) => self.translate_binary_op(left, "Or", right, ctx),
            Expression::Not(operand) => self.translate_unary_op("Not", operand, ctx),
            Expression::FunctionCall { name, args } => self.translate_function(name, args, ctx),
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => self.translate_conditional(condition, then_expr, else_expr, ctx),
            Expression::Modulo(left, right) => self.translate_binary_op(left, "Modulo", right, ctx),
            Expression::Negate(operand) => self.translate_unary_op("Negate", operand, ctx),
        }
    }

    /// Translate a null literal
    fn translate_literal_null(&self) -> Result<TranslatedExpression, TranslationError> {
        Err(TranslationError::UnsupportedExpression(
            "null values in TypeQL rules".to_string(),
        ))
    }

    /// Translate a boolean literal
    fn translate_literal_bool(&self, b: bool) -> TranslatedExpression {
        TranslatedExpression {
            patterns: vec![],
            result: b.to_string(),
            bindings: HashMap::new(),
        }
    }

    /// Translate a number literal
    fn translate_literal_number(&self, n: f64) -> TranslatedExpression {
        TranslatedExpression {
            patterns: vec![],
            result: n.to_string(),
            bindings: HashMap::new(),
        }
    }

    /// Translate a string literal
    fn translate_literal_string(&self, s: &str) -> TranslatedExpression {
        TranslatedExpression {
            patterns: vec![],
            result: format!("\"{s}\""),
            bindings: HashMap::new(),
        }
    }

    /// Translate a variable reference
    fn translate_variable(&self, name: &str, ctx: &mut TranslationContext) -> TranslatedExpression {
        // Generate pattern to bind the variable
        let var = ctx.get_attribute_var(name);
        let pattern = format!("{} has {} {}", ctx.entity_var, name, var);

        TranslatedExpression {
            patterns: vec![pattern],
            result: var,
            bindings: HashMap::new(),
        }
    }

    /// Translate a binary operation
    fn translate_binary_op(
        &self,
        left: &Expression,
        op: &str,
        right: &Expression,
        ctx: &mut TranslationContext,
    ) -> Result<TranslatedExpression, TranslationError> {
        let left_trans = self.translate(left, ctx)?;
        let right_trans = self.translate(right, ctx)?;

        let mut patterns = left_trans.patterns;
        patterns.extend(right_trans.patterns);

        match op {
            // Comparison operators translate directly
            "Equal" => {
                patterns.push(format!("{} = {}", left_trans.result, right_trans.result));
            }
            "NotEqual" => {
                patterns.push(format!(
                    "not {{ {} = {}; }}",
                    left_trans.result, right_trans.result
                ));
            }
            "Less" => {
                patterns.push(format!("{} < {}", left_trans.result, right_trans.result));
            }
            "LessEqual" => {
                patterns.push(format!("{} <= {}", left_trans.result, right_trans.result));
            }
            "Greater" => {
                patterns.push(format!("{} > {}", left_trans.result, right_trans.result));
            }
            "GreaterEqual" => {
                patterns.push(format!("{} >= {}", left_trans.result, right_trans.result));
            }

            // Arithmetic operations need computed attributes
            "Add" | "Subtract" | "Multiply" | "Divide" | "Modulo" => {
                return Err(TranslationError::ComplexExpression(format!(
                    "Arithmetic operation {op} requires computed attribute"
                )));
            }

            // Logical operators
            "And" => {
                // Patterns are already combined (implicit AND in TypeQL)
            }
            "Or" => {
                // OR requires separate rules in TypeQL
                return Err(TranslationError::ComplexExpression(
                    "OR requires multiple rules".to_string(),
                ));
            }

            _ => {
                return Err(TranslationError::UnsupportedExpression(format!(
                    "Unknown operator: {op}"
                )));
            }
        }

        Ok(TranslatedExpression {
            patterns,
            result: "true".to_string(), // Comparison result
            bindings: HashMap::new(),
        })
    }

    /// Translate a unary operation
    fn translate_unary_op(
        &self,
        op: &str,
        operand: &Expression,
        ctx: &mut TranslationContext,
    ) -> Result<TranslatedExpression, TranslationError> {
        match op {
            "Not" => {
                ctx.negated = !ctx.negated;
                let trans = self.translate(operand, ctx)?;
                ctx.negated = !ctx.negated;

                // Wrap patterns in negation
                let negated_pattern = format!("not {{ {} }}", trans.patterns.join("; "));

                Ok(TranslatedExpression {
                    patterns: vec![negated_pattern],
                    result: "true".to_string(),
                    bindings: trans.bindings,
                })
            }
            "Negate" => {
                // Numeric negation requires computed attributes
                Err(TranslationError::ComplexExpression(
                    "Numeric negation requires computed attribute".to_string(),
                ))
            }
            _ => Err(TranslationError::UnsupportedExpression(format!(
                "Unary operator {op} not supported"
            ))),
        }
    }

    /// Translate a function call
    fn translate_function(
        &self,
        name: &str,
        args: &[Expression],
        ctx: &mut TranslationContext,
    ) -> Result<TranslatedExpression, TranslationError> {
        if let Some(handler) = self.function_handlers.get(name) {
            handler(args, ctx)
        } else if name == "case" {
            // Special handling for case expressions - need to generate multiple rules
            Err(TranslationError::ComplexExpression(
                "case() expressions generate multiple rules".to_string(),
            ))
        } else {
            Err(TranslationError::UnsupportedFunction(name.to_string()))
        }
    }

    /// Translate a conditional expression
    fn translate_conditional(
        &self,
        _condition: &Expression,
        _then_expr: &Expression,
        _else_expr: &Expression,
        _ctx: &mut TranslationContext,
    ) -> Result<TranslatedExpression, TranslationError> {
        // Conditionals typically need to be split into multiple rules
        Err(TranslationError::ComplexExpression(
            "Conditional expressions require multiple rules".to_string(),
        ))
    }

    /// Generate a simple equality rule
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn generate_equality_rule(
        &self,
        entity_type: &str,
        attribute: &str,
        expression: &Expression,
    ) -> Result<String, TranslationError> {
        let entity_var = "$x".to_string();
        let mut ctx = TranslationContext::new(entity_var.clone());

        let trans = self.translate(expression, &mut ctx)?;

        let mut rule = String::new();
        writeln!(rule, "rule compute-{entity_type}-{attribute}:")
            .expect("writeln! to String should never fail");
        rule.push_str(
            "when {
",
        );
        writeln!(rule, "    {entity_var} isa {entity_type};")
            .expect("writeln! to String should never fail");

        for pattern in &trans.patterns {
            writeln!(rule, "    {pattern};").expect("writeln! to String should never fail");
        }

        rule.push_str(
            "} then {
",
        );
        writeln!(
            rule,
            "    {} has {} {};",
            entity_var, attribute, trans.result
        )
        .expect("LinkML operation should succeed");
        rule.push_str(
            "};
",
        );

        Ok(rule)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable_translation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let translator = ExpressionTranslator::new();
        let mut ctx = TranslationContext::new("$p".to_string());

        let expr = Expression::Variable("age".to_string());
        let result = translator
            .translate(&expr, &mut ctx)
            .expect("should translate simple variable: {}");

        assert_eq!(result.patterns.len(), 1);
        assert!(result.patterns[0].contains("$p has age $v1"));
        assert_eq!(result.result, "$v1");
        Ok(())
    }

    #[test]
    fn test_comparison_translation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let translator = ExpressionTranslator::new();
        let mut ctx = TranslationContext::new("$p".to_string());

        let expr = Expression::GreaterOrEqual(
            Box::new(Expression::Variable("age".to_string())),
            Box::new(Expression::Number(18.0)),
        );

        let result = translator
            .translate(&expr, &mut ctx)
            .expect("should translate comparison: {}");

        assert!(result.patterns.iter().any(|p| p.contains("$p has age $v1")));
        assert!(result.patterns.iter().any(|p| p.contains("$v1 >= 18")));
        Ok(())
    }

    #[test]
    fn test_contains_function() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let translator = ExpressionTranslator::new();
        let mut ctx = TranslationContext::new("$doc".to_string());

        let expr = Expression::FunctionCall {
            name: "contains".to_string(),
            args: vec![
                Expression::Variable("tags".to_string()),
                Expression::String("important".to_string()),
            ],
        };

        let result = translator
            .translate(&expr, &mut ctx)
            .expect("should translate contains function: {}");

        assert_eq!(result.patterns.len(), 1);
        assert!(result.patterns[0].contains("$doc has tags \"important\""));
        Ok(())
    }
}
