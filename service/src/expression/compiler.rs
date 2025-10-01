//! Expression compilation for performance optimization
//!
//! This module provides JIT compilation capabilities for `LinkML` expressions,
//! converting AST nodes into optimized bytecode for faster evaluation.

use super::ast::Expression;
use super::error::{ExpressionError, ParseError};
use super::functions::FunctionRegistry;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Bytecode instruction set for the expression VM
#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    /// Push a constant value onto the stack
    Const(Value),
    /// Load a variable by name
    Load(String),
    /// Store top of stack to variable
    Store(String),
    /// Pop value from stack
    Pop,
    /// Duplicate top of stack
    Dup,
    /// Binary operations
    /// Addition operation
    Add,
    /// Subtraction operation
    Subtract,
    /// Multiplication operation
    Multiply,
    /// Division operation
    Divide,
    /// Modulo operation
    Modulo,
    /// Power/exponentiation operation
    Power,
    /// Equality comparison
    Equal,
    /// Inequality comparison
    NotEqual,
    /// Less than comparison
    Less,
    /// Less than or equal comparison
    LessEqual,
    /// Greater than comparison
    Greater,
    /// Greater than or equal comparison
    GreaterEqual,
    /// Logical AND operation
    And,
    /// Logical OR operation
    Or,
    /// Unary operations
    /// Logical NOT operation
    Not,
    /// Numeric negation operation
    Negate,
    /// Control flow
    /// Unconditional jump to instruction at index
    Jump(usize),
    /// Jump to instruction if top of stack is false
    JumpIfFalse(usize),
    /// Jump to instruction if top of stack is true
    JumpIfTrue(usize),
    /// Function call with argument count
    Call(String, usize),
    /// Return from function
    Return,
    /// Array/object operations
    /// Create array from N stack elements
    MakeArray(usize),
    /// Create object from N key-value pairs on stack
    MakeObject(usize),
    /// Index into array or object
    Index,
    /// Get field from object by name
    GetField(String),
    /// Load variable and get field in one operation
    LoadField(String, String),
}

/// Compiled expression ready for execution
#[derive(Debug, Clone)]
pub struct CompiledExpression {
    /// Bytecode instructions
    pub instructions: Vec<Instruction>,
    /// Constant pool for larger values
    pub constants: Vec<Value>,
    /// Source expression for debugging
    pub source: String,
    /// Metadata about the compilation
    pub metadata: CompilationMetadata,
}

/// Metadata about the compilation process
#[derive(Debug, Clone)]
pub struct CompilationMetadata {
    /// Number of stack slots required
    pub max_stack_size: usize,
    /// Variables accessed by the expression
    pub accessed_variables: Vec<String>,
    /// Functions called by the expression
    pub called_functions: Vec<String>,
    /// Whether the expression is pure (no side effects)
    pub is_pure: bool,
    /// Estimated complexity score
    pub complexity: usize,
}

/// Expression compiler that converts AST to bytecode
pub struct Compiler {
    /// Function registry for validation
    function_registry: Arc<FunctionRegistry>,
    /// Optimization level (0-3)
    optimization_level: u8,
}

impl Compiler {
    /// Create a new compiler with default settings
    #[must_use]
    pub fn new(function_registry: Arc<FunctionRegistry>) -> Self {
        Self {
            function_registry,
            optimization_level: 2,
        }
    }

    /// Set optimization level (0=none, 3=maximum)
    #[must_use]
    pub fn with_optimization_level(mut self, level: u8) -> Self {
        self.optimization_level = level.min(3);
        self
    }

    /// Compile an expression AST into bytecode
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn compile(
        &self,
        expr: &Expression,
        source: &str,
    ) -> Result<CompiledExpression, ExpressionError> {
        let mut ctx = CompilationContext::new();

        // Generate bytecode
        self.compile_expr(expr, &mut ctx)?;

        // Add implicit return if needed
        if !matches!(ctx.instructions.last(), Some(Instruction::Return)) {
            ctx.instructions.push(Instruction::Return);
        }

        // Apply optimizations
        if self.optimization_level > 0 {
            self.optimize(&mut ctx);
        }

        // Calculate metadata
        let metadata = Self::calculate_metadata(&ctx);

        Ok(CompiledExpression {
            instructions: ctx.instructions,
            constants: ctx.constants,
            source: source.to_string(),
            metadata,
        })
    }

    /// Compile a single expression
    fn compile_expr(
        &self,
        expr: &Expression,
        ctx: &mut CompilationContext,
    ) -> Result<(), ExpressionError> {
        match expr {
            Expression::Null => {
                ctx.emit(Instruction::Const(Value::Null));
            }
            Expression::Boolean(b) => {
                ctx.emit(Instruction::Const(Value::from(*b)));
            }
            Expression::Number(n) => {
                ctx.emit(Instruction::Const(Value::from(*n)));
            }
            Expression::String(s) => {
                ctx.emit(Instruction::Const(Value::from(s.clone())));
            }

            Expression::Variable(name) => {
                ctx.accessed_variables.insert(name.clone());
                ctx.emit(Instruction::Load(name.clone()));
            }

            // Binary operations
            Expression::Add(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Add);
            }
            Expression::Subtract(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Subtract);
            }
            Expression::Multiply(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Multiply);
            }
            Expression::Divide(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Divide);
            }
            Expression::Modulo(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Modulo);
            }

            // Comparison operations
            Expression::Equal(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Equal);
            }
            Expression::NotEqual(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::NotEqual);
            }
            Expression::Less(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Less);
            }
            Expression::Greater(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::Greater);
            }
            Expression::LessOrEqual(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::LessEqual);
            }
            Expression::GreaterOrEqual(left, right) => {
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                ctx.emit(Instruction::GreaterEqual);
            }

            // Logical operations
            Expression::And(left, right) => {
                if self.optimization_level > 1 {
                    // Short-circuit AND
                    self.compile_expr(left, ctx)?;
                    let jump_idx = ctx.emit_placeholder();
                    ctx.emit(Instruction::Dup);
                    ctx.patch_jump(
                        jump_idx,
                        Instruction::JumpIfFalse(ctx.instructions.len() + 2),
                    );
                    ctx.emit(Instruction::Pop);
                    self.compile_expr(right, ctx)?;
                } else {
                    self.compile_expr(left, ctx)?;
                    self.compile_expr(right, ctx)?;
                    ctx.emit(Instruction::And);
                }
            }
            Expression::Or(left, right) => {
                if self.optimization_level > 1 {
                    // Short-circuit OR
                    self.compile_expr(left, ctx)?;
                    let jump_idx = ctx.emit_placeholder();
                    ctx.emit(Instruction::Dup);
                    ctx.patch_jump(
                        jump_idx,
                        Instruction::JumpIfTrue(ctx.instructions.len() + 2),
                    );
                    ctx.emit(Instruction::Pop);
                    self.compile_expr(right, ctx)?;
                } else {
                    self.compile_expr(left, ctx)?;
                    self.compile_expr(right, ctx)?;
                    ctx.emit(Instruction::Or);
                }
            }

            // Unary operations
            Expression::Negate(operand) => {
                self.compile_expr(operand, ctx)?;
                ctx.emit(Instruction::Negate);
            }
            Expression::Not(operand) => {
                self.compile_expr(operand, ctx)?;
                ctx.emit(Instruction::Not);
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                // Compile condition
                self.compile_expr(condition, ctx)?;

                // Jump if false
                let else_jump = ctx.emit_placeholder();

                // Compile then branch
                self.compile_expr(then_expr, ctx)?;
                let end_jump = ctx.emit_placeholder();

                // Compile else branch
                ctx.patch_jump(else_jump, Instruction::JumpIfFalse(ctx.instructions.len()));
                self.compile_expr(else_expr, ctx)?;

                // Patch end jump
                ctx.patch_jump(end_jump, Instruction::Jump(ctx.instructions.len()));
            }

            Expression::FunctionCall { name, args } => {
                // Validate function exists
                if !self.function_registry.has_function(name) {
                    return Err(ExpressionError::Parse(ParseError::UnknownFunction {
                        name: name.clone(),
                        position: 0,
                    }));
                }

                ctx.called_functions.insert(name.clone());

                // Compile arguments
                for arg in args {
                    self.compile_expr(arg, ctx)?;
                }

                // Emit call
                ctx.emit(Instruction::Call(name.clone(), args.len()));
            }
        }

        Ok(())
    }

    /// Apply bytecode optimizations
    fn optimize(&self, ctx: &mut CompilationContext) {
        if self.optimization_level >= 1 {
            // Constant folding
            Self::fold_constants(ctx);
        }

        if self.optimization_level >= 2 {
            // Dead code elimination
            Self::eliminate_dead_code(ctx);

            // Peephole optimizations
            Self::peephole_optimize(ctx);
        }

        if self.optimization_level >= 3 {
            // Instruction combining
            Self::combine_instructions(ctx);
        }
    }

    /// Fold constant expressions at compile time
    fn fold_constants(ctx: &mut CompilationContext) {
        let mut i = 0;
        while i < ctx.instructions.len() {
            // Look for patterns like: Const, Const, BinaryOp
            if i + 2 < ctx.instructions.len()
                && let (Instruction::Const(a), Instruction::Const(b), op) = (
                    &ctx.instructions[i],
                    &ctx.instructions[i + 1],
                    &ctx.instructions[i + 2],
                )
                && let Some(result) = Self::evaluate_constant_binary_op(a, b, op)
            {
                // Replace with single constant
                ctx.instructions[i] = Instruction::Const(result);
                ctx.instructions.remove(i + 1);
                ctx.instructions.remove(i + 1);
                continue;
            }

            // Look for patterns like: Const, UnaryOp
            if i + 1 < ctx.instructions.len()
                && let (Instruction::Const(val), op) =
                    (&ctx.instructions[i], &ctx.instructions[i + 1])
                && let Some(result) = Self::evaluate_constant_unary_op(val, op)
            {
                ctx.instructions[i] = Instruction::Const(result);
                ctx.instructions.remove(i + 1);
                continue;
            }

            i += 1;
        }
    }

    /// Evaluate constant binary operations
    fn evaluate_constant_binary_op(a: &Value, b: &Value, op: &Instruction) -> Option<Value> {
        match (a, b, op) {
            (Value::Number(n1), Value::Number(n2), Instruction::Add) => {
                let v1 = n1.as_f64()?;
                let v2 = n2.as_f64()?;
                Some(Value::Number(serde_json::Number::from_f64(v1 + v2)?))
            }
            (Value::Number(n1), Value::Number(n2), Instruction::Subtract) => {
                let v1 = n1.as_f64()?;
                let v2 = n2.as_f64()?;
                Some(Value::Number(serde_json::Number::from_f64(v1 - v2)?))
            }
            (Value::Number(n1), Value::Number(n2), Instruction::Multiply) => {
                let v1 = n1.as_f64()?;
                let v2 = n2.as_f64()?;
                Some(Value::Number(serde_json::Number::from_f64(v1 * v2)?))
            }
            (Value::String(s1), Value::String(s2), Instruction::Add) => {
                Some(Value::String(format!("{s1}{s2}")))
            }
            (Value::Bool(b1), Value::Bool(b2), Instruction::And) => Some(Value::Bool(*b1 && *b2)),
            (Value::Bool(b1), Value::Bool(b2), Instruction::Or) => Some(Value::Bool(*b1 || *b2)),
            _ => None,
        }
    }

    /// Evaluate constant unary operations
    fn evaluate_constant_unary_op(val: &Value, op: &Instruction) -> Option<Value> {
        match (val, op) {
            (Value::Bool(b), Instruction::Not) => Some(Value::Bool(!b)),
            (Value::Number(n), Instruction::Negate) => {
                let v = n.as_f64()?;
                Some(Value::Number(serde_json::Number::from_f64(-v)?))
            }
            _ => None,
        }
    }

    /// Remove unreachable code
    fn eliminate_dead_code(ctx: &mut CompilationContext) {
        // Mark reachable instructions
        let mut reachable = vec![false; ctx.instructions.len()];
        let mut work_list = vec![0];

        while let Some(pc) = work_list.pop() {
            if pc >= ctx.instructions.len() || reachable[pc] {
                continue;
            }

            reachable[pc] = true;

            match &ctx.instructions[pc] {
                Instruction::Jump(target) => {
                    work_list.push(*target);
                }
                Instruction::JumpIfTrue(target) | Instruction::JumpIfFalse(target) => {
                    work_list.push(*target);
                    work_list.push(pc + 1);
                }
                Instruction::Return => {
                    // Terminal instruction
                }
                _ => {
                    work_list.push(pc + 1);
                }
            }
        }

        // Remove unreachable instructions and update jumps
        let mut new_instructions = Vec::new();
        let mut old_to_new: HashMap<usize, usize> = HashMap::new();

        for (old_pc, inst) in ctx.instructions.iter().enumerate() {
            if reachable[old_pc] {
                old_to_new.insert(old_pc, new_instructions.len());
                new_instructions.push(inst.clone());
            }
        }

        // Update jump targets
        for inst in &mut new_instructions {
            match inst {
                Instruction::Jump(target)
                | Instruction::JumpIfTrue(target)
                | Instruction::JumpIfFalse(target) => {
                    *target = *old_to_new.get(target).unwrap_or(target);
                }
                _ => {}
            }
        }

        ctx.instructions = new_instructions;
    }

    /// Apply peephole optimizations
    fn peephole_optimize(ctx: &mut CompilationContext) {
        let mut i = 0;
        while i < ctx.instructions.len() {
            // Remove redundant Pop after value-producing operations
            if i + 1 < ctx.instructions.len() {
                match (&ctx.instructions[i], &ctx.instructions[i + 1]) {
                    (_, Instruction::Pop) if Self::produces_value(&ctx.instructions[i]) => {
                        // Replace value-producing operation with no-op equivalent
                        ctx.instructions.remove(i);
                        ctx.instructions.remove(i);
                        continue;
                    }
                    _ => {}
                }
            }

            // Remove double negation
            if i + 1 < ctx.instructions.len()
                && let (Instruction::Not, Instruction::Not) =
                    (&ctx.instructions[i], &ctx.instructions[i + 1])
            {
                ctx.instructions.remove(i);
                ctx.instructions.remove(i);
                continue;
            }

            i += 1;
        }
    }

    /// Combine multiple instructions into more efficient forms
    fn combine_instructions(ctx: &mut CompilationContext) {
        // Look for patterns that can be combined into more efficient forms
        let mut i = 0;
        while i + 1 < ctx.instructions.len() {
            match (&ctx.instructions[i], &ctx.instructions[i + 1]) {
                // Combine consecutive field accesses: Load(x), GetField(a) -> LoadField(x, a)
                (Instruction::Load(var), Instruction::GetField(field)) => {
                    // Check if this is part of a chain of field accesses
                    let mut field_chain = vec![field.clone()];
                    let mut j = i + 2;

                    while j < ctx.instructions.len() {
                        if let Instruction::GetField(next_field) = &ctx.instructions[j] {
                            field_chain.push(next_field.clone());
                            j += 1;
                        } else {
                            break;
                        }
                    }

                    // Optimize single field access to LoadField
                    if field_chain.len() == 1 {
                        ctx.instructions[i] = Instruction::LoadField(var.clone(), field.clone());
                        ctx.instructions.drain(i + 1..i + 2);
                        continue; // Don't increment i, check this position again
                    }
                    // For multiple field accesses, keep as is for now
                    // Future optimization: chain field accesses
                }

                // Combine duplicate loads: Load(x), Load(x) -> Load(x), Dup
                (Instruction::Load(var1), Instruction::Load(var2)) if var1 == var2 => {
                    ctx.instructions[i + 1] = Instruction::Dup;
                }

                // Combine constant operations: Const(a), Const(b), Add -> Const(a+b)
                (Instruction::Const(a), Instruction::Const(b))
                    if i + 2 < ctx.instructions.len() =>
                {
                    if let Some(result) =
                        Self::evaluate_constant_binary_op(a, b, &ctx.instructions[i + 2])
                    {
                        ctx.instructions[i] = Instruction::Const(result);
                        ctx.instructions.drain(i + 1..i + 3);
                        continue; // Don't increment i, check this position again
                    }
                }

                _ => {}
            }
            i += 1;
        }
    }

    /// Check if an instruction produces a value
    fn produces_value(inst: &Instruction) -> bool {
        match inst {
            Instruction::Const(_) | Instruction::Load(_) | Instruction::Dup => true,
            Instruction::Add
            | Instruction::Subtract
            | Instruction::Multiply
            | Instruction::Divide => true,
            Instruction::Modulo | Instruction::Power => true,
            Instruction::Equal | Instruction::NotEqual => true,
            Instruction::Less
            | Instruction::LessEqual
            | Instruction::Greater
            | Instruction::GreaterEqual => true,
            Instruction::And | Instruction::Or | Instruction::Not | Instruction::Negate => true,
            Instruction::Call(_, _) => true,
            Instruction::MakeArray(_) | Instruction::MakeObject(_) => true,
            Instruction::Index | Instruction::GetField(_) => true,
            _ => false,
        }
    }

    /// Calculate compilation metadata
    fn calculate_metadata(ctx: &CompilationContext) -> CompilationMetadata {
        let mut max_stack = 0;
        let mut current_stack: usize = 0;
        let mut is_pure = true;
        let mut complexity = 0;

        for inst in &ctx.instructions {
            // Update stack depth
            match inst {
                Instruction::Const(_) | Instruction::Load(_) | Instruction::Dup => {
                    current_stack += 1;
                }
                Instruction::Store(_)
                | Instruction::Pop
                | Instruction::Add
                | Instruction::Subtract
                | Instruction::Multiply
                | Instruction::Divide
                | Instruction::Modulo
                | Instruction::Power
                | Instruction::Equal
                | Instruction::NotEqual
                | Instruction::Less
                | Instruction::LessEqual
                | Instruction::Greater
                | Instruction::GreaterEqual
                | Instruction::And
                | Instruction::Or
                | Instruction::Index => {
                    current_stack = current_stack.saturating_sub(1usize);
                }
                Instruction::Not | Instruction::Negate | Instruction::GetField(_) => {
                    // Stack neutral
                }
                Instruction::Call(_, args) => {
                    current_stack = current_stack.saturating_sub(*args).saturating_add(1usize);
                    is_pure = false; // Conservative: assume functions have side effects
                }
                Instruction::MakeArray(n) => {
                    current_stack = current_stack.saturating_sub(*n).saturating_add(1usize);
                }
                Instruction::MakeObject(n) => {
                    current_stack = current_stack.saturating_sub(n * 2).saturating_add(1usize);
                }
                _ => {}
            }

            max_stack = max_stack.max(current_stack);

            // Calculate complexity
            match inst {
                Instruction::Call(_, _) => complexity += 10,
                Instruction::Jump(_) | Instruction::JumpIfTrue(_) | Instruction::JumpIfFalse(_) => {
                    complexity += 2;
                }
                _ => complexity += 1,
            }
        }

        CompilationMetadata {
            max_stack_size: max_stack,
            accessed_variables: ctx.accessed_variables.iter().cloned().collect(),
            called_functions: ctx.called_functions.iter().cloned().collect(),
            is_pure,
            complexity,
        }
    }
}

/// Compilation context for tracking state during compilation
struct CompilationContext {
    instructions: Vec<Instruction>,
    constants: Vec<Value>,
    accessed_variables: std::collections::HashSet<String>,
    called_functions: std::collections::HashSet<String>,
}

impl CompilationContext {
    fn new() -> Self {
        Self {
            instructions: Vec::new(),
            constants: Vec::new(),
            accessed_variables: std::collections::HashSet::new(),
            called_functions: std::collections::HashSet::new(),
        }
    }

    fn emit(&mut self, inst: Instruction) {
        self.instructions.push(inst);
    }

    fn emit_placeholder(&mut self) -> usize {
        let idx = self.instructions.len();
        self.instructions.push(Instruction::Return); // Placeholder
        idx
    }

    fn patch_jump(&mut self, idx: usize, inst: Instruction) {
        self.instructions[idx] = inst;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{FunctionRegistry, Parser};

    #[test]
    fn test_simple_compilation() -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry);
        let parser = Parser::new();

        // Test arithmetic
        let expr = parser
            .parse("1 + 2 * 3")
            .expect("should parse expression: {}");
        let compiled_expr = compiler
            .compile(&expr, "1 + 2 * 3")
            .expect("should compile expression: {}");

        // The compiler optimizes 2 * 3 to 6 at compile time
        // So we get: 1, 6, Add
        assert!(compiled_expr.instructions.iter().any(
            |inst| matches!(inst, Instruction::Const(Value::Number(n)) if n.as_f64() == Some(1.0))
        ));
        assert!(compiled_expr.instructions.iter().any(
            |inst| matches!(inst, Instruction::Const(Value::Number(n)) if n.as_f64() == Some(6.0))
        ));
        assert!(compiled_expr.instructions.contains(&Instruction::Add));
        Ok(())
    }

    #[test]
    fn test_constant_folding() -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry).with_optimization_level(1);
        let parser = Parser::new();

        // Constants should be folded
        let expr = parser
            .parse("2 + 3")
            .expect("should parse constant expression: {}");
        let compiled_expr = compiler
            .compile(&expr, "2 + 3")
            .expect("should compile constant expression: {}");

        // Should be optimized to a single constant
        assert_eq!(compiled_expr.instructions.len(), 2); // Const(5), Return
        assert!(matches!(&compiled_expr.instructions[0],
                Instruction::Const(Value::Number(n)) if n.as_f64() == Some(5.0)));
        Ok(())
    }

    #[test]
    fn test_short_circuit() {
        use crate::expression::functions::FunctionRegistry;
        use crate::expression::parser::Parser;

        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry).with_optimization_level(2);
        let parser = Parser::new();

        // Test short-circuit AND
        let expr = parser
            .parse("false && true")
            .expect("should parse short-circuit expression");
        let compiled_expr = compiler
            .compile(&expr, "false && true")
            .expect("should compile short-circuit expression");

        // Should have jump instruction for short-circuit
        assert!(
            compiled_expr
                .instructions
                .iter()
                .any(|inst| matches!(inst, Instruction::JumpIfFalse(_))),
            "Should have short-circuit jump for AND"
        );
    }
}
