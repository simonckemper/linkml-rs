//! Abstract Syntax Tree for `LinkML` expressions

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents an expression in the `LinkML` expression language
/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    /// Addition operator (+)
    Add,
    /// Subtraction operator (-)
    Subtract,
    /// Multiplication operator (*)
    Multiply,
    /// Division operator (/)
    Divide,
    /// Modulo operator (%)
    Modulo,
    /// Equality operator (==)
    Equal,
    /// Inequality operator (!=)
    NotEqual,
    /// Less than operator (<)
    Less,
    /// Greater than operator (>)
    Greater,
    /// Less than or equal operator (<=)
    LessOrEqual,
    /// Greater than or equal operator (>=)
    GreaterOrEqual,
    /// Logical AND operator (&&)
    And,
    /// Logical OR operator (||)
    Or,
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    /// Negation operator (-)
    Negate,
    /// Logical NOT operator (!)
    Not,
}

/// Represents an expression in the `LinkML` expression language
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Null literal
    Null,
    /// Boolean literal
    Boolean(bool),
    /// Numeric literal
    Number(f64),
    /// String literal
    String(String),

    /// Variable reference
    Variable(String),

    /// Addition operation
    Add(Box<Expression>, Box<Expression>),
    /// Subtraction operation
    Subtract(Box<Expression>, Box<Expression>),
    /// Multiplication operation
    Multiply(Box<Expression>, Box<Expression>),
    /// Division operation
    Divide(Box<Expression>, Box<Expression>),
    /// Modulo operation
    Modulo(Box<Expression>, Box<Expression>),

    /// Unary negation operation
    Negate(Box<Expression>),

    /// Equality comparison
    Equal(Box<Expression>, Box<Expression>),
    /// Inequality comparison
    NotEqual(Box<Expression>, Box<Expression>),
    /// Less than comparison
    Less(Box<Expression>, Box<Expression>),
    /// Greater than comparison
    Greater(Box<Expression>, Box<Expression>),
    /// Less than or equal comparison
    LessOrEqual(Box<Expression>, Box<Expression>),
    /// Greater than or equal comparison
    GreaterOrEqual(Box<Expression>, Box<Expression>),

    /// Logical AND operation
    And(Box<Expression>, Box<Expression>),
    /// Logical OR operation
    Or(Box<Expression>, Box<Expression>),
    /// Logical NOT operation
    Not(Box<Expression>),

    /// Function call expression
    FunctionCall {
        /// Function name
        name: String,
        /// Function arguments
        args: Vec<Expression>,
    },

    /// Conditional expression (ternary)
    Conditional {
        /// Condition to evaluate
        condition: Box<Expression>,
        /// Expression if condition is true
        then_expr: Box<Expression>,
        /// Expression if condition is false
        else_expr: Box<Expression>,
    },
}

impl Expression {
    /// Create a new variable expression
    pub fn var(name: impl Into<String>) -> Self {
        Expression::Variable(name.into())
    }

    /// Create a new string literal
    pub fn string(value: impl Into<String>) -> Self {
        Expression::String(value.into())
    }

    /// Create a new number literal
    #[must_use]
    pub fn number(value: f64) -> Self {
        Expression::Number(value)
    }

    /// Create a new boolean literal
    #[must_use]
    pub fn boolean(value: bool) -> Self {
        Expression::Boolean(value)
    }

    /// Get the depth of the expression tree
    #[must_use]
    pub fn depth(&self) -> usize {
        match self {
            Expression::Null
            | Expression::Boolean(_)
            | Expression::Number(_)
            | Expression::String(_)
            | Expression::Variable(_) => 1,

            Expression::Negate(expr) | Expression::Not(expr) => 1 + expr.depth(),

            Expression::Add(left, right)
            | Expression::Subtract(left, right)
            | Expression::Multiply(left, right)
            | Expression::Divide(left, right)
            | Expression::Modulo(left, right)
            | Expression::Equal(left, right)
            | Expression::NotEqual(left, right)
            | Expression::Less(left, right)
            | Expression::Greater(left, right)
            | Expression::LessOrEqual(left, right)
            | Expression::GreaterOrEqual(left, right)
            | Expression::And(left, right)
            | Expression::Or(left, right) => 1 + left.depth().max(right.depth()),

            Expression::FunctionCall { args, .. } => {
                1 + args.iter().map(Expression::depth).max().unwrap_or(0)
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                1 + condition
                    .depth()
                    .max(then_expr.depth())
                    .max(else_expr.depth())
            }
        }
    }

    /// Count the total number of nodes in the expression tree
    #[must_use]
    pub fn node_count(&self) -> usize {
        match self {
            Expression::Null
            | Expression::Boolean(_)
            | Expression::Number(_)
            | Expression::String(_)
            | Expression::Variable(_) => 1,

            Expression::Negate(expr) | Expression::Not(expr) => 1 + expr.node_count(),

            Expression::Add(left, right)
            | Expression::Subtract(left, right)
            | Expression::Multiply(left, right)
            | Expression::Divide(left, right)
            | Expression::Modulo(left, right)
            | Expression::Equal(left, right)
            | Expression::NotEqual(left, right)
            | Expression::Less(left, right)
            | Expression::Greater(left, right)
            | Expression::LessOrEqual(left, right)
            | Expression::GreaterOrEqual(left, right)
            | Expression::And(left, right)
            | Expression::Or(left, right) => 1 + left.node_count() + right.node_count(),

            Expression::FunctionCall { args, .. } => {
                1 + args.iter().map(Expression::node_count).sum::<usize>()
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => 1 + condition.node_count() + then_expr.node_count() + else_expr.node_count(),
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Null => write!(f, "null"),
            Expression::Boolean(b) => write!(f, "{b}"),
            Expression::Number(n) => write!(f, "{n}"),
            Expression::String(s) => write!(f, "\"{s}\""),
            Expression::Variable(name) => write!(f, "{{{name}}}"),

            Expression::Add(left, right) => write!(f, "({left} + {right})"),
            Expression::Subtract(left, right) => write!(f, "({left} - {right})"),
            Expression::Multiply(left, right) => write!(f, "({left} * {right})"),
            Expression::Divide(left, right) => write!(f, "({left} / {right})"),
            Expression::Modulo(left, right) => write!(f, "({left} % {right})"),

            Expression::Negate(expr) => write!(f, "-{expr}"),

            Expression::Equal(left, right) => write!(f, "({left} == {right})"),
            Expression::NotEqual(left, right) => write!(f, "({left} != {right})"),
            Expression::Less(left, right) => write!(f, "({left} < {right})"),
            Expression::Greater(left, right) => write!(f, "({left} > {right})"),
            Expression::LessOrEqual(left, right) => write!(f, "({left} <= {right})"),
            Expression::GreaterOrEqual(left, right) => write!(f, "({left} >= {right})"),

            Expression::And(left, right) => write!(f, "({left} and {right})"),
            Expression::Or(left, right) => write!(f, "({left} or {right})"),
            Expression::Not(expr) => write!(f, "not {expr}"),

            Expression::FunctionCall { name, args } => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => write!(f, "({then_expr} if {condition} else {else_expr})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_builders() {
        let var = Expression::var("x");
        assert_eq!(var, Expression::Variable("x".to_string()));

        let num = Expression::number(42.0);
        assert_eq!(num, Expression::Number(42.0));

        let string = Expression::string("hello");
        assert_eq!(string, Expression::String("hello".to_string()));

        let boolean = Expression::boolean(true);
        assert_eq!(boolean, Expression::Boolean(true));
    }

    #[test]
    fn test_expression_depth() {
        let simple = Expression::number(42.0);
        assert_eq!(simple.depth(), 1);

        let binary = Expression::Add(
            Box::new(Expression::number(1.0)),
            Box::new(Expression::number(2.0)),
        );
        assert_eq!(binary.depth(), 2);

        let nested = Expression::Add(
            Box::new(Expression::Multiply(
                Box::new(Expression::number(2.0)),
                Box::new(Expression::number(3.0)),
            )),
            Box::new(Expression::number(4.0)),
        );
        assert_eq!(nested.depth(), 3);
    }

    #[test]
    fn test_expression_display() {
        let expr = Expression::Add(
            Box::new(Expression::Variable("x".to_string())),
            Box::new(Expression::Number(5.0)),
        );
        assert_eq!(expr.to_string(), "({x} + 5)");

        let func = Expression::FunctionCall {
            name: "len".to_string(),
            args: vec![Expression::Variable("items".to_string())],
        };
        assert_eq!(func.to_string(), "len({items})");
    }
}
