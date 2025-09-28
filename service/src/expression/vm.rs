//! Virtual machine for executing compiled expressions
//!
//! This module provides a stack-based VM that executes bytecode generated
//! by the expression compiler for optimal performance.

use super::compiler::{CompiledExpression, Instruction};
use super::error::{EvaluationError, ExpressionError};
use super::functions::FunctionRegistry;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Convert f64 to `serde_json::Number`, returning error for non-finite values
fn f64_to_number(val: f64) -> Result<serde_json::Number, EvaluationError> {
    serde_json::Number::from_f64(val).ok_or_else(|| EvaluationError::TypeError {
        message: "Number must be finite (not NaN or infinity)".to_string(),
    })
}

/// Stack-based virtual machine for expression evaluation
pub struct VirtualMachine {
    /// Function registry for function calls
    function_registry: Arc<FunctionRegistry>,
    /// Maximum stack depth to prevent overflow
    max_stack_depth: usize,
    /// Maximum iterations to prevent infinite loops
    max_iterations: usize,
}

impl VirtualMachine {
    /// Create a new virtual machine
    #[must_use]
    pub fn new(function_registry: Arc<FunctionRegistry>) -> Self {
        Self {
            function_registry,
            max_stack_depth: 1024,
            max_iterations: 10_000,
        }
    }

    /// Set maximum stack depth
    #[must_use]
    pub fn with_max_stack_depth(mut self, depth: usize) -> Self {
        self.max_stack_depth = depth;
        self
    }

    /// Set maximum iterations for loop prevention
    #[must_use]
    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    /// Execute compiled expression with given context
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Stack underflow occurs during execution
    /// - Division by zero is attempted
    /// - Type mismatch occurs in operations
    /// - Maximum iteration limit is exceeded
    /// - Unknown function is called
    /// - Stack depth limit is exceeded
    pub fn execute(
        &self,
        compiled: &CompiledExpression,
        context: &HashMap<String, Value>,
    ) -> Result<Value, ExpressionError> {
        let mut state = VMState::new(context, self.max_stack_depth);

        self.execute_instructions(&compiled.instructions, &mut state)?;

        // Result should be on top of stack
        state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Empty stack after execution".to_string(),
            })
        })
    }

    /// Execute a sequence of instructions
    fn execute_instructions(
        &self,
        instructions: &[Instruction],
        state: &mut VMState,
    ) -> Result<(), ExpressionError> {
        let mut iteration_count = 0;

        while state.pc < instructions.len() {
            // Check iteration limit to prevent infinite loops
            iteration_count += 1;
            if iteration_count > self.max_iterations {
                return Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: format!(
                        "Maximum iteration limit ({}) exceeded - possible infinite loop",
                        self.max_iterations
                    ),
                }));
            }

            let inst = &instructions[state.pc];
            state.pc += 1;

            // Check if we should continue execution
            if !self.execute_single_instruction(inst, state)? {
                break;
            }
        }

        Ok(())
    }

    /// Execute a single instruction
    /// Returns true if execution should continue, false if it should stop
    fn execute_single_instruction(
        &self,
        inst: &Instruction,
        state: &mut VMState,
    ) -> Result<bool, ExpressionError> {
        match inst {
            Instruction::Const(val) => {
                state.push(val.clone())?;
            }

            Instruction::Load(name) => {
                let val = state.context.get(name).cloned().unwrap_or(Value::Null);
                state.push(val)?;
            }

            Instruction::Store(_name) => {
                // This is for future use with variable assignment
                let _val = state.pop().ok_or_else(|| {
                    ExpressionError::Evaluation(EvaluationError::TypeError {
                        message: "Stack underflow".to_string(),
                    })
                })?;
                // For now, we don't support variable assignment
                return Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Variable assignment not supported".to_string(),
                }));
            }

            Instruction::Pop => {
                state.pop().ok_or_else(|| {
                    ExpressionError::Evaluation(EvaluationError::TypeError {
                        message: "Stack underflow".to_string(),
                    })
                })?;
            }

            Instruction::Dup => {
                let val = state
                    .peek()
                    .ok_or_else(|| {
                        ExpressionError::Evaluation(EvaluationError::TypeError {
                            message: "Stack underflow".to_string(),
                        })
                    })?
                    .clone();
                state.push(val)?;
            }

            // Binary operations
            Instruction::Add => Self::execute_add(state)?,
            Instruction::Subtract => Self::execute_subtract(state)?,
            Instruction::Multiply => Self::execute_multiply(state)?,
            Instruction::Divide => Self::execute_divide(state)?,
            Instruction::Modulo => Self::execute_modulo(state)?,
            Instruction::Power => Self::execute_power(state)?,
            Instruction::Equal => Self::execute_equal(state)?,
            Instruction::NotEqual => Self::execute_not_equal(state)?,
            Instruction::Less => Self::execute_less(state)?,
            Instruction::LessEqual => Self::execute_less_equal(state)?,
            Instruction::Greater => Self::execute_greater(state)?,
            Instruction::GreaterEqual => Self::execute_greater_equal(state)?,
            Instruction::And => Self::execute_and(state)?,
            Instruction::Or => Self::execute_or(state)?,

            // Unary operations
            Instruction::Not => Self::execute_not(state)?,
            Instruction::Negate => Self::execute_negate(state)?,

            // Control flow
            Instruction::Jump(target) => {
                state.pc = *target;
            }

            Instruction::JumpIfFalse(target) => {
                let condition = state.pop().ok_or_else(|| {
                    ExpressionError::Evaluation(EvaluationError::TypeError {
                        message: "Stack underflow".to_string(),
                    })
                })?;
                if !is_truthy(&condition) {
                    state.pc = *target;
                }
            }

            Instruction::JumpIfTrue(target) => {
                let condition = state.pop().ok_or_else(|| {
                    ExpressionError::Evaluation(EvaluationError::TypeError {
                        message: "Stack underflow".to_string(),
                    })
                })?;
                if is_truthy(&condition) {
                    state.pc = *target;
                }
            }

            // Function call
            Instruction::Call(name, arg_count) => {
                self.execute_call(state, name, *arg_count)?;
            }

            // Return - signal to stop execution
            Instruction::Return => {
                return Ok(false);
            }

            // Array/object operations
            Instruction::MakeArray(size) => Self::execute_make_array(state, *size)?,
            Instruction::MakeObject(size) => Self::execute_make_object(state, *size)?,
            Instruction::Index => Self::execute_index(state)?,
            Instruction::GetField(field) => Self::execute_get_field(state, field)?,
            Instruction::LoadField(var, field) => Self::execute_load_field(state, var, field)?,
        }

        Ok(true)
    }

    // Helper methods for array/object operations

    fn execute_make_array(state: &mut VMState, size: usize) -> Result<(), ExpressionError> {
        let mut elements = Vec::with_capacity(size);
        for _ in 0..size {
            elements.push(state.pop().ok_or_else(|| {
                ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Stack underflow".to_string(),
                })
            })?);
        }
        elements.reverse();
        state.push(Value::Array(elements))
    }

    fn execute_make_object(state: &mut VMState, size: usize) -> Result<(), ExpressionError> {
        let mut obj = serde_json::Map::new();
        for _ in 0..size {
            let value = state.pop().ok_or_else(|| {
                ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Stack underflow".to_string(),
                })
            })?;
            let key = state.pop().ok_or_else(|| {
                ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Stack underflow".to_string(),
                })
            })?;

            if let Value::String(k) = key {
                obj.insert(k, value);
            } else {
                return Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Object key must be string".to_string(),
                }));
            }
        }
        state.push(Value::Object(obj))
    }

    fn execute_index(state: &mut VMState) -> Result<(), ExpressionError> {
        let index = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let container = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&container, &index) {
            (Value::Array(arr), Value::Number(n)) => {
                // Safe cast: array indices are expected to be within usize range
                #[allow(clippy::cast_possible_truncation)]
                let idx = n.as_u64().unwrap_or(0) as usize;
                arr.get(idx).cloned().unwrap_or(Value::Null)
            }
            (Value::Object(obj), Value::String(key)) => {
                obj.get(key).cloned().unwrap_or(Value::Null)
            }
            _ => Value::Null,
        };

        state.push(result)
    }

    fn execute_get_field(state: &mut VMState, field: &str) -> Result<(), ExpressionError> {
        let obj = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match obj {
            Value::Object(map) => map.get(field).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };

        state.push(result)
    }

    fn execute_load_field(
        state: &mut VMState,
        var: &str,
        field: &str,
    ) -> Result<(), ExpressionError> {
        // Load the variable value
        let obj = state.context.get(var).cloned().unwrap_or(Value::Null);

        // Get the field from the object
        let result = match obj {
            Value::Object(map) => map.get(field).cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };

        state.push(result)
    }

    // Binary operation implementations

    fn execute_add(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                Value::Number(f64_to_number(v1 + v2)?)
            }
            (Value::String(s1), Value::String(s2)) => Value::String(format!("{s1}{s2}")),
            _ => {
                return Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Invalid operands for addition".to_string(),
                }));
            }
        };

        state.push(result)
    }

    fn execute_subtract(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                state.push(Value::Number(f64_to_number(v1 - v2)?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Invalid operands for subtraction".to_string(),
            })),
        }
    }

    fn execute_multiply(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                state.push(Value::Number(f64_to_number(v1 * v2)?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Invalid operands for multiplication".to_string(),
            })),
        }
    }

    fn execute_divide(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);

                if v2 == 0.0 {
                    return Err(ExpressionError::Evaluation(EvaluationError::DivisionByZero));
                }

                state.push(Value::Number(f64_to_number(v1 / v2)?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Invalid operands for division".to_string(),
            })),
        }
    }

    fn execute_modulo(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);

                if v2 == 0.0 {
                    return Err(ExpressionError::Evaluation(EvaluationError::DivisionByZero));
                }

                state.push(Value::Number(f64_to_number(v1 % v2)?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Invalid operands for modulo".to_string(),
            })),
        }
    }

    fn execute_power(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                let v1 = n1.as_f64().unwrap_or(0.0);
                let v2 = n2.as_f64().unwrap_or(0.0);
                state.push(Value::Number(f64_to_number(v1.powf(v2))?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Invalid operands for power".to_string(),
            })),
        }
    }

    // Comparison operations

    fn execute_equal(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        state.push(Value::Bool(values_equal(&a, &b)))
    }

    fn execute_not_equal(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        state.push(Value::Bool(!values_equal(&a, &b)))
    }

    fn execute_less(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                n1.as_f64().unwrap_or(0.0) < n2.as_f64().unwrap_or(0.0)
            }
            (Value::String(s1), Value::String(s2)) => s1 < s2,
            _ => false,
        };

        state.push(Value::Bool(result))
    }

    fn execute_less_equal(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                n1.as_f64().unwrap_or(0.0) <= n2.as_f64().unwrap_or(0.0)
            }
            (Value::String(s1), Value::String(s2)) => s1 <= s2,
            _ => false,
        };

        state.push(Value::Bool(result))
    }

    fn execute_greater(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                n1.as_f64().unwrap_or(0.0) > n2.as_f64().unwrap_or(0.0)
            }
            (Value::String(s1), Value::String(s2)) => s1 > s2,
            _ => false,
        };

        state.push(Value::Bool(result))
    }

    fn execute_greater_equal(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        let result = match (&a, &b) {
            (Value::Number(n1), Value::Number(n2)) => {
                n1.as_f64().unwrap_or(0.0) >= n2.as_f64().unwrap_or(0.0)
            }
            (Value::String(s1), Value::String(s2)) => s1 >= s2,
            _ => false,
        };

        state.push(Value::Bool(result))
    }

    // Logical operations

    fn execute_and(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        state.push(Value::Bool(is_truthy(&a) && is_truthy(&b)))
    }

    fn execute_or(state: &mut VMState) -> Result<(), ExpressionError> {
        let b = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;
        let a = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        state.push(Value::Bool(is_truthy(&a) || is_truthy(&b)))
    }

    // Unary operations

    fn execute_not(state: &mut VMState) -> Result<(), ExpressionError> {
        let val = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        state.push(Value::Bool(!is_truthy(&val)))
    }

    fn execute_negate(state: &mut VMState) -> Result<(), ExpressionError> {
        let val = state.pop().ok_or_else(|| {
            ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Stack underflow".to_string(),
            })
        })?;

        match val {
            Value::Number(n) => {
                let v = n.as_f64().unwrap_or(0.0);
                state.push(Value::Number(f64_to_number(-v)?))
            }
            _ => Err(ExpressionError::Evaluation(EvaluationError::TypeError {
                message: "Cannot negate non-numeric value".to_string(),
            })),
        }
    }

    // Function call - This needs &self for function_registry access

    fn execute_call(
        &self,
        state: &mut VMState,
        name: &str,
        arg_count: usize,
    ) -> Result<(), ExpressionError> {
        // Collect arguments
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(state.pop().ok_or_else(|| {
                ExpressionError::Evaluation(EvaluationError::TypeError {
                    message: "Stack underflow".to_string(),
                })
            })?);
        }
        args.reverse();

        // Call function
        let result = self.function_registry.call(name, args).map_err(|e| {
            ExpressionError::Evaluation(EvaluationError::FunctionError {
                name: name.to_string(),
                message: e.to_string(),
            })
        })?;

        state.push(result)
    }
}

/// VM execution state
struct VMState<'a> {
    /// Value stack
    stack: Vec<Value>,
    /// Program counter
    pc: usize,
    /// Variable context
    context: &'a HashMap<String, Value>,
    /// Maximum stack depth
    max_stack_depth: usize,
}

impl<'a> VMState<'a> {
    fn new(context: &'a HashMap<String, Value>, max_stack_depth: usize) -> Self {
        Self {
            stack: Vec::with_capacity(32),
            pc: 0,
            context,
            max_stack_depth,
        }
    }

    fn push(&mut self, val: Value) -> Result<(), ExpressionError> {
        if self.stack.len() >= self.max_stack_depth {
            return Err(ExpressionError::Evaluation(
                EvaluationError::CallStackTooDeep {
                    max: self.max_stack_depth,
                },
            ));
        }
        self.stack.push(val);
        Ok(())
    }

    fn pop(&mut self) -> Option<Value> {
        self.stack.pop()
    }

    fn peek(&self) -> Option<&Value> {
        self.stack.last()
    }
}

// Helper functions

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(o) => !o.is_empty(),
    }
}

fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Array(a), Value::Array(b)) => a == b,
        (Value::Object(a), Value::Object(b)) => a == b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{FunctionRegistry, Parser, compiler::Compiler};
    use serde_json::json;

    #[test]
    fn test_vm_arithmetic() -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(Arc::clone(&registry));
        let vm = VirtualMachine::new(registry);
        let parser = Parser::new();

        // Test basic arithmetic
        let expr = parser.parse("2 + 3 * 4")?;
        let bytecode = compiler.compile(&expr, "2 + 3 * 4")?;
        let result = vm.execute(&bytecode, &HashMap::new())?;

        assert!(matches!(result, Value::Number(n) if n.as_f64() == Some(14.0)));
        Ok(())
    }

    #[test]
    fn test_vm_variables() -> Result<(), Box<dyn std::error::Error>> {
        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(Arc::clone(&registry));
        let vm = VirtualMachine::new(registry);
        let parser = Parser::new();

        let mut context = HashMap::new();
        context.insert("x".to_string(), json!(10));
        context.insert("y".to_string(), json!(20));

        let expr = parser.parse("x + y")?;
        let bytecode = compiler.compile(&expr, "x + y")?;
        let result = vm.execute(&bytecode, &context)?;

        assert!(matches!(result, Value::Number(n) if n.as_f64() == Some(30.0)));
        Ok(())
    }

    #[test]
    fn test_vm_functions() -> Result<(), ExpressionError> {
        use crate::expression::compiler::Compiler;
        use crate::expression::functions::FunctionRegistry;
        use crate::expression::parser::Parser;

        let registry = Arc::new(FunctionRegistry::new());
        let compiler = Compiler::new(Arc::clone(&registry));
        let vm = VirtualMachine::new(registry);
        let parser = Parser::new();

        let mut context = HashMap::new();
        context.insert("text".to_string(), Value::String("hello".to_string()));

        // Test len function
        let expr = parser.parse("len(text)")?;
        let bytecode = compiler.compile(&expr, "len(text)")?;
        let result = vm.execute(&bytecode, &context)?;
        assert_eq!(
            result,
            Value::Number(
                serde_json::Number::from_f64(5.0).expect("LinkML operation should succeed")
            )
        );

        // Test max function
        let expr = parser.parse("max(1, 5, 3)")?;
        let bytecode = compiler.compile(&expr, "max(1, 5, 3)")?;
        let result = vm.execute(&bytecode, &context)?;
        assert_eq!(
            result,
            Value::Number(
                serde_json::Number::from_f64(5.0).expect("LinkML operation should succeed")
            )
        );

        Ok(())
    }
}
