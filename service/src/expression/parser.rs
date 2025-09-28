//! Expression parser for `LinkML` expression language

#![allow(missing_docs)]
#![allow(dead_code)]

use super::ast::Expression;
use super::error::ParseError;
use std::iter::Peekable;
use std::str::Chars;

/// Token types for the expression parser
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Literals
    Null,
    Boolean(bool),
    Number(f64),
    String(String),

    // Identifiers and variables
    Identifier(String),
    Variable(String), // {name}

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Logical
    And,
    Or,
    Not,

    // Delimiters
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,

    // Keywords
    If,
    Else,

    // End of input
    Eof,
}

/// Tokenizer for breaking input into tokens
struct Tokenizer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    position: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            position: 0,
        }
    }

    fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();

        if let Some(&ch) = self.chars.peek() {
            match ch {
                '+' => {
                    self.advance();
                    Ok(Token::Plus)
                }
                '-' => {
                    self.advance();
                    Ok(Token::Minus)
                }
                '*' => {
                    self.advance();
                    Ok(Token::Star)
                }
                '/' => {
                    self.advance();
                    Ok(Token::Slash)
                }
                '%' => {
                    self.advance();
                    Ok(Token::Percent)
                }
                '(' => {
                    self.advance();
                    Ok(Token::LeftParen)
                }
                ')' => {
                    self.advance();
                    Ok(Token::RightParen)
                }
                '{' => {
                    self.advance();
                    self.read_variable()
                }
                ',' => {
                    self.advance();
                    Ok(Token::Comma)
                }
                '"' => self.read_string(),
                '0'..='9' => self.read_number(),
                'a'..='z' | 'A'..='Z' | '_' => Ok(self.read_identifier()),
                '=' => {
                    self.advance();
                    if self.chars.peek() == Some(&'=') {
                        self.advance();
                        Ok(Token::Equal)
                    } else {
                        Err(ParseError::UnexpectedToken {
                            token: "=".to_string(),
                            position: self.position - 1,
                        })
                    }
                }
                '!' => {
                    self.advance();
                    if self.chars.peek() == Some(&'=') {
                        self.advance();
                        Ok(Token::NotEqual)
                    } else {
                        Ok(Token::Not)
                    }
                }
                '&' => {
                    self.advance();
                    if self.chars.peek() == Some(&'&') {
                        self.advance();
                        Ok(Token::And)
                    } else {
                        Err(ParseError::UnexpectedToken {
                            token: "&".to_string(),
                            position: self.position - 1,
                        })
                    }
                }
                '|' => {
                    self.advance();
                    if self.chars.peek() == Some(&'|') {
                        self.advance();
                        Ok(Token::Or)
                    } else {
                        Err(ParseError::UnexpectedToken {
                            token: "|".to_string(),
                            position: self.position - 1,
                        })
                    }
                }
                '<' => {
                    self.advance();
                    if self.chars.peek() == Some(&'=') {
                        self.advance();
                        Ok(Token::LessEqual)
                    } else {
                        Ok(Token::Less)
                    }
                }
                '>' => {
                    self.advance();
                    if self.chars.peek() == Some(&'=') {
                        self.advance();
                        Ok(Token::GreaterEqual)
                    } else {
                        Ok(Token::Greater)
                    }
                }
                _ => Err(ParseError::UnexpectedToken {
                    token: ch.to_string(),
                    position: self.position,
                }),
            }
        } else {
            Ok(Token::Eof)
        }
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(ch) = self.chars.next() {
            self.position += ch.len_utf8();
            Some(ch)
        } else {
            None
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(&ch) = self.chars.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_number(&mut self) -> Result<Token, ParseError> {
        let start = self.position;
        let mut num_str = String::new();

        // Read integer part
        while let Some(&ch) = self.chars.peek() {
            if ch.is_numeric() {
                num_str.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Read decimal part if present
        if self.chars.peek() == Some(&'.') {
            num_str.push('.');
            self.advance();

            while let Some(&ch) = self.chars.peek() {
                if ch.is_numeric() {
                    num_str.push(ch);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Parse the number
        match num_str.parse::<f64>() {
            Ok(num) => Ok(Token::Number(num)),
            Err(_) => Err(ParseError::InvalidNumber {
                value: num_str,
                position: start,
            }),
        }
    }

    fn read_string(&mut self) -> Result<Token, ParseError> {
        let start = self.position;
        self.advance(); // Skip opening quote

        let mut string = String::new();
        let mut escaped = false;

        loop {
            match self.chars.peek() {
                Some(&'"') if !escaped => {
                    self.advance();
                    return Ok(Token::String(string));
                }
                Some(&'\\') if !escaped => {
                    escaped = true;
                    self.advance();
                }
                Some(&ch) => {
                    if escaped {
                        let escaped_char = match ch {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '"' => '"',
                            _ => {
                                return Err(ParseError::InvalidString {
                                    position: self.position,
                                    reason: format!("Invalid escape sequence \\{ch}"),
                                });
                            }
                        };
                        string.push(escaped_char);
                        escaped = false;
                    } else {
                        string.push(ch);
                    }
                    self.advance();
                }
                None => {
                    return Err(ParseError::InvalidString {
                        position: start,
                        reason: "Unterminated string".to_string(),
                    });
                }
            }
        }
    }

    fn read_identifier(&mut self) -> Token {
        let mut ident = String::new();

        while let Some(&ch) = self.chars.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        // Check for keywords

        match ident.as_str() {
            "null" => Token::Null,
            "true" => Token::Boolean(true),
            "false" => Token::Boolean(false),
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            "if" => Token::If,
            "else" => Token::Else,
            _ => Token::Identifier(ident),
        }
    }

    fn read_variable(&mut self) -> Result<Token, ParseError> {
        let start = self.position - 1;
        let mut var_name = String::new();

        // First character must be alphabetic or underscore
        if let Some(&ch) = self.chars.peek() {
            if ch.is_alphabetic() || ch == '_' {
                var_name.push(ch);
                self.advance();
            } else if ch == '}' {
                return Err(ParseError::InvalidVariable {
                    name: var_name,
                    position: start,
                });
            } else {
                // Consume invalid characters until we hit }
                while let Some(&ch) = self.chars.peek() {
                    if ch == '}' {
                        self.advance();
                        break;
                    }
                    var_name.push(ch);
                    self.advance();
                }
                return Err(ParseError::InvalidVariable {
                    name: var_name,
                    position: start,
                });
            }
        }

        while let Some(&ch) = self.chars.peek() {
            if ch == '}' {
                self.advance();
                return Ok(Token::Variable(var_name));
            } else if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                var_name.push(ch);
                self.advance();
            } else {
                return Err(ParseError::InvalidVariable {
                    name: var_name,
                    position: start,
                });
            }
        }

        Err(ParseError::MissingDelimiter {
            delimiter: '}',
            position: start,
        })
    }
}

/// Expression parser
#[derive(Clone)]
pub struct Parser {
    max_depth: usize,
    max_length: usize,
}

impl Parser {
    /// Create a new parser with default settings
    #[must_use]
    pub fn new() -> Self {
        Self {
            max_depth: 100,
            max_length: 10_000,
        }
    }

    /// Create a parser with custom limits
    #[must_use]
    pub fn with_limits(max_depth: usize, max_length: usize) -> Self {
        Self {
            max_depth,
            max_length,
        }
    }

    /// Parse an expression string
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn parse(&self, input: &str) -> Result<Expression, ParseError> {
        if input.len() > self.max_length {
            return Err(ParseError::TooLong {
                length: input.len(),
                max: self.max_length,
            });
        }

        let tokenizer = Tokenizer::new(input);
        let mut parser = ParserState {
            tokenizer,
            current: Token::Eof,
            depth: 0,
            max_depth: self.max_depth,
        };

        parser.advance()?;
        let expr = parser.parse_expression()?;

        if parser.current != Token::Eof {
            return Err(ParseError::TrailingInput {
                input: input[parser.tokenizer.position..].to_string(),
            });
        }

        Ok(expr)
    }

    /// Parse an expression string (alias for parse method)
    ///
    /// This method provides compatibility with code expecting a `parse_str` method.
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn parse_str(&self, input: &str) -> Result<Expression, ParseError> {
        self.parse(input)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal parser state
struct ParserState<'a> {
    tokenizer: Tokenizer<'a>,
    current: Token,
    depth: usize,
    max_depth: usize,
}

impl ParserState<'_> {
    fn advance(&mut self) -> Result<(), ParseError> {
        self.current = self.tokenizer.next_token()?;
        Ok(())
    }

    fn check_depth(&mut self) -> Result<(), ParseError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(ParseError::TooDeep {
                depth: self.depth,
                max: self.max_depth,
            });
        }
        Ok(())
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_ternary()
    }

    fn parse_ternary(&mut self) -> Result<Expression, ParseError> {
        let expr = self.parse_or()?;

        if self.current == Token::If {
            self.check_depth()?;
            self.advance()?;

            let condition = self.parse_or()?;

            if self.current != Token::Else {
                return Err(ParseError::UnexpectedToken {
                    token: format!("{:?}", self.current),
                    position: self.tokenizer.position,
                });
            }
            self.advance()?;

            let else_expr = self.parse_ternary()?;
            self.depth -= 1;

            Ok(Expression::Conditional {
                condition: Box::new(condition),
                then_expr: Box::new(expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_and()?;

        while self.current == Token::Or {
            self.check_depth()?;
            self.advance()?;
            let right = self.parse_and()?;
            left = Expression::Or(Box::new(left), Box::new(right));
            self.depth -= 1;
        }

        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_comparison()?;

        while self.current == Token::And {
            self.check_depth()?;
            self.advance()?;
            let right = self.parse_comparison()?;
            left = Expression::And(Box::new(left), Box::new(right));
            self.depth -= 1;
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_addition()?;

        loop {
            let op = match self.current {
                Token::Equal => Expression::Equal,
                Token::NotEqual => Expression::NotEqual,
                Token::Less => Expression::Less,
                Token::Greater => Expression::Greater,
                Token::LessEqual => Expression::LessOrEqual,
                Token::GreaterEqual => Expression::GreaterOrEqual,
                _ => break,
            };

            self.check_depth()?;
            self.advance()?;
            let right = self.parse_addition()?;
            left = op(Box::new(left), Box::new(right));
            self.depth -= 1;
        }

        Ok(left)
    }

    fn parse_addition(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_multiplication()?;

        loop {
            let op = match self.current {
                Token::Plus => Expression::Add,
                Token::Minus => Expression::Subtract,
                _ => break,
            };

            self.check_depth()?;
            self.advance()?;
            let right = self.parse_multiplication()?;
            left = op(Box::new(left), Box::new(right));
            self.depth -= 1;
        }

        Ok(left)
    }

    fn parse_multiplication(&mut self) -> Result<Expression, ParseError> {
        let mut left = self.parse_unary()?;

        loop {
            let op = match self.current {
                Token::Star => Expression::Multiply,
                Token::Slash => Expression::Divide,
                Token::Percent => Expression::Modulo,
                _ => break,
            };

            self.check_depth()?;
            self.advance()?;
            let right = self.parse_unary()?;
            left = op(Box::new(left), Box::new(right));
            self.depth -= 1;
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        match &self.current {
            Token::Not => {
                self.check_depth()?;
                self.advance()?;
                let expr = self.parse_unary()?;
                self.depth -= 1;
                Ok(Expression::Not(Box::new(expr)))
            }
            Token::Minus => {
                self.check_depth()?;
                self.advance()?;
                let expr = self.parse_unary()?;
                self.depth -= 1;
                Ok(Expression::Negate(Box::new(expr)))
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match &self.current.clone() {
            Token::Null => {
                self.advance()?;
                Ok(Expression::Null)
            }
            Token::Boolean(b) => {
                let value = *b;
                self.advance()?;
                Ok(Expression::Boolean(value))
            }
            Token::Number(n) => {
                let value = *n;
                self.advance()?;
                Ok(Expression::Number(value))
            }
            Token::String(s) => {
                let value = s.clone();
                self.advance()?;
                Ok(Expression::String(value))
            }
            Token::Variable(name) => {
                let var_name = name.clone();
                self.advance()?;
                Ok(Expression::Variable(var_name))
            }
            Token::Identifier(name) => {
                let ident = name.clone();
                self.advance()?;

                if self.current == Token::LeftParen {
                    self.parse_function_call(ident)
                } else {
                    // Treat standalone identifiers as variable references
                    Ok(Expression::Variable(ident))
                }
            }
            Token::LeftParen => {
                self.check_depth()?;
                self.advance()?;
                let expr = self.parse_expression()?;

                if self.current != Token::RightParen {
                    return Err(ParseError::MissingDelimiter {
                        delimiter: ')',
                        position: self.tokenizer.position,
                    });
                }
                self.advance()?;
                self.depth -= 1;

                Ok(expr)
            }
            _ => Err(ParseError::UnexpectedToken {
                token: format!("{:?}", self.current),
                position: self.tokenizer.position,
            }),
        }
    }

    fn parse_function_call(&mut self, name: String) -> Result<Expression, ParseError> {
        self.check_depth()?;
        self.advance()?; // Skip '('

        let mut args = Vec::new();

        if self.current != Token::RightParen {
            loop {
                args.push(self.parse_expression()?);

                match self.current {
                    Token::Comma => {
                        self.advance()?;
                    }
                    Token::RightParen => break,
                    _ => {
                        return Err(ParseError::UnexpectedToken {
                            token: format!("{:?}", self.current),
                            position: self.tokenizer.position,
                        });
                    }
                }
            }
        }

        if self.current != Token::RightParen {
            return Err(ParseError::MissingDelimiter {
                delimiter: ')',
                position: self.tokenizer.position,
            });
        }
        self.advance()?;
        self.depth -= 1;

        Ok(Expression::FunctionCall { name, args })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literals() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        assert_eq!(
            parser.parse("null").expect("should parse null: {}"),
            Expression::Null
        );
        assert_eq!(
            parser.parse("true").expect("should parse true: {}"),
            Expression::Boolean(true)
        );
        assert_eq!(
            parser.parse("false").expect("should parse false: {}"),
            Expression::Boolean(false)
        );
        assert_eq!(
            parser.parse("42").expect("should parse integer: {}"),
            Expression::Number(42.0)
        );
        assert_eq!(
            parser.parse("3.14").expect("should parse float: {}"),
            Expression::Number(3.14)
        );
        assert_eq!(
            parser.parse("\"hello\"").expect("should parse string: {}"),
            Expression::String("hello".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_parse_variables() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        assert_eq!(
            parser.parse("{x}").expect("should parse variable: {}"),
            Expression::Variable("x".to_string())
        );
        assert_eq!(
            parser
                .parse("{user_name}")
                .expect("should parse variable with underscore: {}"),
            Expression::Variable("user_name".to_string())
        );
        Ok(())
    }

    #[test]
    fn test_parse_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        let expr = parser
            .parse("1 + 2")
            .expect("should parse binary expression: {}");
        assert_eq!(
            expr,
            Expression::Add(
                Box::new(Expression::Number(1.0)),
                Box::new(Expression::Number(2.0))
            )
        );

        let expr = parser
            .parse("3 * 4 + 5")
            .expect("should parse expression with precedence: {}");
        assert_eq!(
            expr,
            Expression::Add(
                Box::new(Expression::Multiply(
                    Box::new(Expression::Number(3.0)),
                    Box::new(Expression::Number(4.0))
                )),
                Box::new(Expression::Number(5.0))
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_comparison() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        let expr = parser
            .parse("{x} > 5")
            .expect("should parse comparison: {}");
        assert_eq!(
            expr,
            Expression::Greater(
                Box::new(Expression::Variable("x".to_string())),
                Box::new(Expression::Number(5.0))
            )
        );

        let expr = parser
            .parse("{age} >= 18 and {age} < 65")
            .expect("should parse logical expression: {}");
        assert_eq!(
            expr,
            Expression::And(
                Box::new(Expression::GreaterOrEqual(
                    Box::new(Expression::Variable("age".to_string())),
                    Box::new(Expression::Number(18.0))
                )),
                Box::new(Expression::Less(
                    Box::new(Expression::Variable("age".to_string())),
                    Box::new(Expression::Number(65.0))
                ))
            )
        );
        Ok(())
    }

    #[test]
    fn test_parse_function_call() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        let expr = parser
            .parse("len({items})")
            .expect("should parse function call: {}");
        assert_eq!(
            expr,
            Expression::FunctionCall {
                name: "len".to_string(),
                args: vec![Expression::Variable("items".to_string())]
            }
        );

        let expr = parser
            .parse("max(1, 2, 3)")
            .expect("should parse function with multiple args: {}");
        assert_eq!(
            expr,
            Expression::FunctionCall {
                name: "max".to_string(),
                args: vec![
                    Expression::Number(1.0),
                    Expression::Number(2.0),
                    Expression::Number(3.0),
                ]
            }
        );
        Ok(())
    }

    #[test]
    fn test_parse_errors() -> Result<(), Box<dyn std::error::Error>> {
        let parser = Parser::new();

        assert!(matches!(
            parser.parse("{"),
            Err(ParseError::MissingDelimiter { delimiter: '}', .. })
        ));

        assert!(matches!(
            parser.parse("1 +"),
            Err(ParseError::UnexpectedToken { .. })
        ));

        assert!(matches!(
            parser.parse("1 + 2 extra"),
            Err(ParseError::TrailingInput { .. })
        ));
        Ok(())
    }
}
