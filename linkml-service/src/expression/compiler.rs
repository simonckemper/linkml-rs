//! Expression compilation for performance optimization
//!
//! This module provides JIT compilation capabilities for LinkML expressions,
//! converting AST nodes into optimized bytecode for faster evaluation.

use super::ast::{Expression, BinaryOp, UnaryOp};
use super::error::ExpressionError;
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
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Power,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
    /// Unary operations
    Not,
    Negate,
    /// Control flow
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    /// Function call with argument count
    Call(String, usize),
    /// Return from function
    Return,
    /// Array/object operations
    MakeArray(usize),
    MakeObject(usize),
    Index,
    GetField(String),
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
    pub fn new(function_registry: Arc<FunctionRegistry>) -> Self {
        Self {
            function_registry,
            optimization_level: 2,
        }
    }
    
    /// Set optimization level (0=none, 3=maximum)
    pub fn with_optimization_level(mut self, level: u8) -> Self {
        self.optimization_level = level.min(3);
        self
    }
    
    /// Compile an expression AST into bytecode
    pub fn compile(&self, expr: &Expression, source: &str) -> Result<CompiledExpression, ExpressionError> {
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
        let metadata = self.calculate_metadata(&ctx);
        
        Ok(CompiledExpression {
            instructions: ctx.instructions,
            constants: ctx.constants,
            source: source.to_string(),
            metadata,
        })
    }
    
    /// Compile a single expression
    fn compile_expr(&self, expr: &Expression, ctx: &mut CompilationContext) -> Result<(), ExpressionError> {
        match expr {
            Expression::Literal(val) => {
                ctx.emit(Instruction::Const(val.clone()));
            }
            
            Expression::Variable(name) => {
                ctx.accessed_variables.insert(name.clone());
                ctx.emit(Instruction::Load(name.clone()));
            }
            
            Expression::Binary { op, left, right } => {
                // Compile operands
                self.compile_expr(left, ctx)?;
                self.compile_expr(right, ctx)?;
                
                // Emit operation
                match op {
                    BinaryOp::Add => ctx.emit(Instruction::Add),
                    BinaryOp::Subtract => ctx.emit(Instruction::Subtract),
                    BinaryOp::Multiply => ctx.emit(Instruction::Multiply),
                    BinaryOp::Divide => ctx.emit(Instruction::Divide),
                    BinaryOp::Modulo => ctx.emit(Instruction::Modulo),
                    BinaryOp::Power => ctx.emit(Instruction::Power),
                    BinaryOp::Equal => ctx.emit(Instruction::Equal),
                    BinaryOp::NotEqual => ctx.emit(Instruction::NotEqual),
                    BinaryOp::Less => ctx.emit(Instruction::Less),
                    BinaryOp::LessEqual => ctx.emit(Instruction::LessEqual),
                    BinaryOp::Greater => ctx.emit(Instruction::Greater),
                    BinaryOp::GreaterEqual => ctx.emit(Instruction::GreaterEqual),
                    BinaryOp::And => {
                        // Short-circuit AND
                        if self.optimization_level > 1 {
                            let jump_idx = ctx.emit_placeholder();
                            ctx.emit(Instruction::Dup);
                            ctx.patch_jump(jump_idx, Instruction::JumpIfFalse(ctx.instructions.len() + 1));
                            ctx.emit(Instruction::Pop);
                            self.compile_expr(right, ctx)?;
                            return Ok(());
                        }
                        ctx.emit(Instruction::And)
                    }
                    BinaryOp::Or => {
                        // Short-circuit OR
                        if self.optimization_level > 1 {
                            let jump_idx = ctx.emit_placeholder();
                            ctx.emit(Instruction::Dup);
                            ctx.patch_jump(jump_idx, Instruction::JumpIfTrue(ctx.instructions.len() + 1));
                            ctx.emit(Instruction::Pop);
                            self.compile_expr(right, ctx)?;
                            return Ok(());
                        }
                        ctx.emit(Instruction::Or)
                    }
                }
            }
            
            Expression::Unary { op, operand } => {
                self.compile_expr(operand, ctx)?;
                match op {
                    UnaryOp::Not => ctx.emit(Instruction::Not),
                    UnaryOp::Negate => ctx.emit(Instruction::Negate),
                }
            }
            
            Expression::Conditional { condition, then_expr, else_expr } => {
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
                    return Err(ExpressionError::Parse(format!("Unknown function: {}", name)));
                }
                
                ctx.called_functions.insert(name.clone());
                
                // Compile arguments
                for arg in args {
                    self.compile_expr(arg, ctx)?;
                }
                
                // Emit call
                ctx.emit(Instruction::Call(name.clone(), args.len()));
            }
            
            Expression::Array(elements) => {
                // Compile elements
                for elem in elements {
                    self.compile_expr(elem, ctx)?;
                }
                ctx.emit(Instruction::MakeArray(elements.len()));
            }
            
            Expression::Object(fields) => {
                // Compile field values
                for (key, value) in fields {
                    ctx.emit(Instruction::Const(Value::String(key.clone())));
                    self.compile_expr(value, ctx)?;
                }
                ctx.emit(Instruction::MakeObject(fields.len()));
            }
            
            Expression::Index { object, index } => {
                self.compile_expr(object, ctx)?;
                self.compile_expr(index, ctx)?;
                ctx.emit(Instruction::Index);
            }
            
            Expression::FieldAccess { object, field } => {
                self.compile_expr(object, ctx)?;
                ctx.emit(Instruction::GetField(field.clone()));
            }
        }
        
        Ok(())
    }
    
    /// Apply bytecode optimizations
    fn optimize(&self, ctx: &mut CompilationContext) {
        if self.optimization_level >= 1 {
            // Constant folding
            self.fold_constants(ctx);
        }
        
        if self.optimization_level >= 2 {
            // Dead code elimination
            self.eliminate_dead_code(ctx);
            
            // Peephole optimizations
            self.peephole_optimize(ctx);
        }
        
        if self.optimization_level >= 3 {
            // Instruction combining
            self.combine_instructions(ctx);
        }
    }
    
    /// Fold constant expressions at compile time
    fn fold_constants(&self, ctx: &mut CompilationContext) {
        let mut i = 0;
        while i < ctx.instructions.len() {
            // Look for patterns like: Const, Const, BinaryOp
            if i + 2 < ctx.instructions.len() {
                if let (
                    Instruction::Const(a),
                    Instruction::Const(b),
                    op
                ) = (&ctx.instructions[i], &ctx.instructions[i + 1], &ctx.instructions[i + 2]) {
                    if let Some(result) = self.evaluate_constant_binary_op(a, b, op) {
                        // Replace with single constant
                        ctx.instructions[i] = Instruction::Const(result);
                        ctx.instructions.remove(i + 1);
                        ctx.instructions.remove(i + 1);
                        continue;
                    }
                }
            }
            
            // Look for patterns like: Const, UnaryOp
            if i + 1 < ctx.instructions.len() {
                if let (Instruction::Const(val), op) = (&ctx.instructions[i], &ctx.instructions[i + 1]) {
                    if let Some(result) = self.evaluate_constant_unary_op(val, op) {
                        ctx.instructions[i] = Instruction::Const(result);
                        ctx.instructions.remove(i + 1);
                        continue;
                    }
                }
            }
            
            i += 1;
        }
    }
    
    /// Evaluate constant binary operations
    fn evaluate_constant_binary_op(&self, a: &Value, b: &Value, op: &Instruction) -> Option<Value> {
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
                Some(Value::String(format!("{}{}", s1, s2)))
            }
            (Value::Bool(b1), Value::Bool(b2), Instruction::And) => {
                Some(Value::Bool(*b1 && *b2))
            }
            (Value::Bool(b1), Value::Bool(b2), Instruction::Or) => {
                Some(Value::Bool(*b1 || *b2))
            }
            _ => None,
        }
    }
    
    /// Evaluate constant unary operations
    fn evaluate_constant_unary_op(&self, val: &Value, op: &Instruction) -> Option<Value> {
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
    fn eliminate_dead_code(&self, ctx: &mut CompilationContext) {
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
                Instruction::Jump(target) => {
                    *target = *old_to_new.get(target).unwrap_or(target);
                }
                Instruction::JumpIfTrue(target) | Instruction::JumpIfFalse(target) => {
                    *target = *old_to_new.get(target).unwrap_or(target);
                }
                _ => {}
            }
        }
        
        ctx.instructions = new_instructions;
    }
    
    /// Apply peephole optimizations
    fn peephole_optimize(&self, ctx: &mut CompilationContext) {
        let mut i = 0;
        while i < ctx.instructions.len() {
            // Remove redundant Pop after value-producing operations
            if i + 1 < ctx.instructions.len() {
                match (&ctx.instructions[i], &ctx.instructions[i + 1]) {
                    (_, Instruction::Pop) if self.produces_value(&ctx.instructions[i]) => {
                        // Replace value-producing operation with no-op equivalent
                        ctx.instructions.remove(i);
                        ctx.instructions.remove(i);
                        continue;
                    }
                    _ => {}
                }
            }
            
            // Remove double negation
            if i + 1 < ctx.instructions.len() {
                if let (Instruction::Not, Instruction::Not) = (&ctx.instructions[i], &ctx.instructions[i + 1]) {
                    ctx.instructions.remove(i);
                    ctx.instructions.remove(i);
                    continue;
                }
            }
            
            i += 1;
        }
    }
    
    /// Combine multiple instructions into more efficient forms
    fn combine_instructions(&self, ctx: &mut CompilationContext) {
        // This is a placeholder for more advanced optimizations
        // such as combining multiple field accesses or array operations
    }
    
    /// Check if an instruction produces a value
    fn produces_value(&self, inst: &Instruction) -> bool {
        match inst {
            Instruction::Const(_) | Instruction::Load(_) | Instruction::Dup => true,
            Instruction::Add | Instruction::Subtract | Instruction::Multiply | Instruction::Divide => true,
            Instruction::Modulo | Instruction::Power => true,
            Instruction::Equal | Instruction::NotEqual => true,
            Instruction::Less | Instruction::LessEqual | Instruction::Greater | Instruction::GreaterEqual => true,
            Instruction::And | Instruction::Or | Instruction::Not | Instruction::Negate => true,
            Instruction::Call(_, _) => true,
            Instruction::MakeArray(_) | Instruction::MakeObject(_) => true,
            Instruction::Index | Instruction::GetField(_) => true,
            _ => false,
        }
    }
    
    /// Calculate compilation metadata
    fn calculate_metadata(&self, ctx: &CompilationContext) -> CompilationMetadata {
        let mut max_stack = 0;
        let mut current_stack = 0;
        let mut is_pure = true;
        let mut complexity = 0;
        
        for inst in &ctx.instructions {
            // Update stack depth
            match inst {
                Instruction::Const(_) | Instruction::Load(_) | Instruction::Dup => {
                    current_stack += 1;
                }
                Instruction::Store(_) | Instruction::Pop => {
                    current_stack = current_stack.saturating_sub(1);
                }
                Instruction::Add | Instruction::Subtract | Instruction::Multiply | Instruction::Divide |
                Instruction::Modulo | Instruction::Power | Instruction::Equal | Instruction::NotEqual |
                Instruction::Less | Instruction::LessEqual | Instruction::Greater | Instruction::GreaterEqual |
                Instruction::And | Instruction::Or | Instruction::Index => {
                    current_stack = current_stack.saturating_sub(1);
                }
                Instruction::Not | Instruction::Negate | Instruction::GetField(_) => {
                    // Stack neutral
                }
                Instruction::Call(_, args) => {
                    current_stack = current_stack.saturating_sub(*args).saturating_add(1);
                    is_pure = false; // Conservative: assume functions have side effects
                }
                Instruction::MakeArray(n) => {
                    current_stack = current_stack.saturating_sub(*n).saturating_add(1);
                }
                Instruction::MakeObject(n) => {
                    current_stack = current_stack.saturating_sub(n * 2).saturating_add(1);
                }
                _ => {}
            }
            
            max_stack = max_stack.max(current_stack);
            
            // Calculate complexity
            match inst {
                Instruction::Call(_, _) => complexity += 10,
                Instruction::Jump(_) | Instruction::JumpIfTrue(_) | Instruction::JumpIfFalse(_) => complexity += 2,
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
    use crate::expression::{Parser, FunctionRegistry};
    
    #[test]
    fn test_simple_compilation() {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry);
        let parser = Parser::new();
        
        // Test arithmetic
        let expr = parser.parse("1 + 2 * 3").unwrap();
        let compiled = compiler.compile(&expr, "1 + 2 * 3").unwrap();
        
        assert!(compiled.instructions.contains(&Instruction::Const(Value::Number(serde_json::Number::from(1)))));
        assert!(compiled.instructions.contains(&Instruction::Add));
        assert!(compiled.instructions.contains(&Instruction::Multiply));
    }
    
    #[test]
    fn test_constant_folding() {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry).with_optimization_level(1);
        let parser = Parser::new();
        
        // Constants should be folded
        let expr = parser.parse("2 + 3").unwrap();
        let compiled = compiler.compile(&expr, "2 + 3").unwrap();
        
        // Should be optimized to a single constant
        assert_eq!(compiled.instructions.len(), 2); // Const(5), Return
        assert_eq!(compiled.instructions[0], Instruction::Const(Value::Number(serde_json::Number::from(5))));
    }
    
    #[test]
    fn test_short_circuit() {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(registry).with_optimization_level(2);
        let parser = Parser::new();
        
        // Test short-circuit AND
        let expr = parser.parse("false && expensive_func()").unwrap();
        let compiled = compiler.compile(&expr, "false && expensive_func()").unwrap();
        
        // Should have jump instruction for short-circuit
        assert!(compiled.instructions.iter().any(|inst| matches!(inst, Instruction::JumpIfFalse(_))));
    }
}