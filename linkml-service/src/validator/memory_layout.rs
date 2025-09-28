//! Memory layout optimization for validation structures
//!
//! This module provides optimized memory layouts for frequently used
//! validation structures to improve cache performance and reduce memory usage.

use super::compiled::{CompiledType, ValidationInstruction};
use super::report::{Severity, ValidationIssue};
use std::mem;

/// Optimized validation issue with better field ordering
///
/// Original `ValidationIssue` layout may have padding between fields.
/// This version reorders fields to minimize padding and improve cache line usage.
#[derive(Debug, Clone)]
pub struct OptimizedValidationIssue {
    /// Path string (typically 24 bytes on 64-bit)
    pub path: String,
    /// Message string (24 bytes)
    pub message: String,
    /// Validator name (24 bytes)
    pub validator: String,
    /// Error code if any (24 bytes for Option<String>)
    pub code: Option<String>,
    /// Severity (1 byte + padding)
    pub severity: Severity,
    /// Context stored separately to avoid inline `HashMap` overhead
    pub context_id: Option<u32>,
}

impl OptimizedValidationIssue {
    /// Convert from standard `ValidationIssue`
    #[must_use]
    pub fn from_standard(issue: ValidationIssue, context_id: Option<u32>) -> Self {
        Self {
            path: issue.path,
            message: issue.message,
            validator: issue.validator,
            code: issue.code,
            severity: issue.severity,
            context_id,
        }
    }

    /// Convert back to standard `ValidationIssue`
    #[must_use]
    pub fn to_standard(
        self,
        context: std::collections::HashMap<String, serde_json::Value>,
    ) -> ValidationIssue {
        ValidationIssue {
            severity: self.severity,
            message: self.message,
            path: self.path,
            validator: self.validator,
            code: self.code,
            context,
        }
    }
}

/// Compact validation instruction using tagged union
///
/// Reduces memory usage by using a more compact representation
/// with shared fields and a discriminant.
#[derive(Debug, Clone)]
pub struct CompactInstruction {
    /// Common path field used by most instructions
    pub path: String,
    /// Instruction variant data
    pub variant: InstructionVariant,
}

/// Instruction variant data
#[derive(Debug, Clone)]
pub enum InstructionVariant {
    /// Check required field
    CheckRequired {
        /// Name of the field that must be present
        field: String,
    },
    /// Pattern validation
    ValidatePattern {
        /// ID of the compiled regex pattern to validate against
        pattern_id: u32,
    },
    /// Range validation
    ValidateRange {
        /// Minimum value (inclusive or exclusive based on inclusive flag)
        min: Option<f64>,
        /// Maximum value (inclusive or exclusive based on inclusive flag)
        max: Option<f64>,
        /// Whether the range is inclusive on both ends
        inclusive: bool,
    },
    /// Length validation
    ValidateLength {
        /// Minimum length
        min: Option<u32>,
        /// Maximum length
        max: Option<u32>,
    },
    /// Enum validation
    ValidateEnum {
        /// ID of the enum definition to validate against
        enum_id: u32,
    },
    /// Type validation
    ValidateType {
        /// The compiled type that the value must match
        expected: CompiledType,
    },
    /// Array validation
    ValidateArray {
        /// Instructions to validate each element
        element_instructions: Box<Vec<CompactInstruction>>,
    },
    /// Object validation
    ValidateObject {
        /// Instructions for each field (field name, instructions)
        field_instructions: Box<Vec<(String, Vec<CompactInstruction>)>>,
    },
    /// No operation (placeholder for unimplemented features)
    NoOp,
}

impl CompactInstruction {
    /// Convert from standard `ValidationInstruction`
    pub fn from_standard(instruction: ValidationInstruction) -> Self {
        match instruction {
            ValidationInstruction::CheckRequired { path, field } => Self {
                path,
                variant: InstructionVariant::CheckRequired { field },
            },
            ValidationInstruction::ValidatePattern { path, pattern_id } => Self {
                path,
                variant: InstructionVariant::ValidatePattern {
                    pattern_id: u32::try_from(pattern_id).unwrap_or(u32::MAX),
                },
            },
            ValidationInstruction::ValidateRange {
                path,
                min,
                max,
                inclusive,
            } => Self {
                path,
                variant: InstructionVariant::ValidateRange {
                    min,
                    max,
                    inclusive,
                },
            },
            ValidationInstruction::ValidateLength { path, min, max } => Self {
                path,
                variant: InstructionVariant::ValidateLength {
                    min: min.map(|v| u32::try_from(v).unwrap_or(u32::MAX)),
                    max: max.map(|v| u32::try_from(v).unwrap_or(u32::MAX)),
                },
            },
            ValidationInstruction::ValidateEnum { path, enum_id } => Self {
                path,
                variant: InstructionVariant::ValidateEnum {
                    enum_id: u32::try_from(enum_id).unwrap_or(u32::MAX),
                },
            },
            ValidationInstruction::ValidateType {
                path,
                expected_type,
            } => Self {
                path,
                variant: InstructionVariant::ValidateType {
                    expected: expected_type,
                },
            },
            ValidationInstruction::ValidateArray {
                path,
                element_instructions,
            } => Self {
                path,
                variant: InstructionVariant::ValidateArray {
                    element_instructions: Box::new(
                        element_instructions
                            .into_iter()
                            .map(CompactInstruction::from_standard)
                            .collect(),
                    ),
                },
            },
            ValidationInstruction::ValidateObject {
                path,
                field_instructions,
            } => Self {
                path,
                variant: InstructionVariant::ValidateObject {
                    field_instructions: Box::new(
                        field_instructions
                            .into_iter()
                            .map(|(k, v)| {
                                (
                                    k,
                                    v.into_iter()
                                        .map(CompactInstruction::from_standard)
                                        .collect(),
                                )
                            })
                            .collect(),
                    ),
                },
            },
            ValidationInstruction::ConditionalValidation { .. } => {
                // Conditional validation is evaluated at runtime in the validator
                // For now, return a no-op instruction
                Self {
                    path: String::new(),
                    variant: InstructionVariant::NoOp,
                }
            }
        }
    }
}

/// Memory pool for validation contexts
///
/// Pre-allocates and reuses validation context memory to reduce
/// allocation overhead and improve cache locality.
pub struct ValidationContextPool {
    /// Pool of context maps
    contexts: parking_lot::Mutex<Vec<std::collections::HashMap<String, serde_json::Value>>>,
    /// Maximum pool size
    max_size: usize,
}

impl ValidationContextPool {
    /// Create a new context pool
    #[must_use]
    pub fn new(max_size: usize) -> Self {
        Self {
            contexts: parking_lot::Mutex::new(Vec::with_capacity(max_size)),
            max_size,
        }
    }

    /// Get a context map from the pool
    pub fn get(&self) -> std::collections::HashMap<String, serde_json::Value> {
        let mut pool = self.contexts.lock();
        pool.pop().unwrap_or_default()
    }

    /// Return a context map to the pool
    pub fn put(&self, mut context: std::collections::HashMap<String, serde_json::Value>) {
        context.clear();
        let mut pool = self.contexts.lock();
        if pool.len() < self.max_size {
            pool.push(context);
        }
    }
}

/// Size-optimized compiled type representation
///
/// Uses a single byte instead of full enum discriminant
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompactType {
    /// String type
    String = 0,
    /// Integer type
    Integer = 1,
    /// Float type
    Float = 2,
    /// Boolean type
    Boolean = 3,
    /// Date type
    Date = 4,
    /// `DateTime` type
    DateTime = 5,
    /// URI type
    Uri = 6,
    /// Object type
    Object = 7,
    /// Array type
    Array = 8,
    /// Any type
    Any = 9,
}

impl From<CompiledType> for CompactType {
    fn from(compiled: CompiledType) -> Self {
        match compiled {
            CompiledType::String => CompactType::String,
            CompiledType::Integer => CompactType::Integer,
            CompiledType::Float => CompactType::Float,
            CompiledType::Boolean => CompactType::Boolean,
            CompiledType::Date => CompactType::Date,
            CompiledType::DateTime => CompactType::DateTime,
            CompiledType::Uri => CompactType::Uri,
            CompiledType::Object => CompactType::Object,
            CompiledType::Array => CompactType::Array,
            CompiledType::Any => CompactType::Any,
        }
    }
}

impl From<CompactType> for CompiledType {
    fn from(compact: CompactType) -> Self {
        match compact {
            CompactType::String => CompiledType::String,
            CompactType::Integer => CompiledType::Integer,
            CompactType::Float => CompiledType::Float,
            CompactType::Boolean => CompiledType::Boolean,
            CompactType::Date => CompiledType::Date,
            CompactType::DateTime => CompiledType::DateTime,
            CompactType::Uri => CompiledType::Uri,
            CompactType::Object => CompiledType::Object,
            CompactType::Array => CompiledType::Array,
            CompactType::Any => CompiledType::Any,
        }
    }
}

/// Memory statistics for validation structures
#[derive(Debug)]
pub struct MemoryStats {
    /// Size of `ValidationIssue`
    pub validation_issue_size: usize,
    /// Size of `OptimizedValidationIssue`
    pub optimized_issue_size: usize,
    /// Size of `ValidationInstruction`
    pub instruction_size: usize,
    /// Size of `CompactInstruction`
    pub compact_instruction_size: usize,
    /// Size of `CompiledType`
    pub compiled_type_size: usize,
    /// Size of `CompactType`
    pub compact_type_size: usize,
}

impl MemoryStats {
    /// Calculate memory statistics
    #[must_use]
    pub fn calculate() -> Self {
        Self {
            validation_issue_size: mem::size_of::<ValidationIssue>(),
            optimized_issue_size: mem::size_of::<OptimizedValidationIssue>(),
            instruction_size: mem::size_of::<ValidationInstruction>(),
            compact_instruction_size: mem::size_of::<CompactInstruction>(),
            compiled_type_size: mem::size_of::<CompiledType>(),
            compact_type_size: mem::size_of::<CompactType>(),
        }
    }

    /// Print memory statistics
    pub fn print(&self) {
        println!("Memory Layout Statistics:");
        println!("  ValidationIssue: {} bytes", self.validation_issue_size);
        println!(
            "  OptimizedValidationIssue: {} bytes ({}% reduction)",
            self.optimized_issue_size,
            100 - (self.optimized_issue_size * 100 / self.validation_issue_size)
        );
        println!("  ValidationInstruction: {} bytes", self.instruction_size);
        println!(
            "  CompactInstruction: {} bytes ({}% reduction)",
            self.compact_instruction_size,
            100 - (self.compact_instruction_size * 100 / self.instruction_size)
        );
        println!("  CompiledType: {} bytes", self.compiled_type_size);
        println!(
            "  CompactType: {} bytes ({}% reduction)",
            self.compact_type_size,
            100 - (self.compact_type_size * 100 / self.compiled_type_size)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_sizes() {
        let stats = MemoryStats::calculate();

        // Ensure our optimized structures are actually smaller
        assert!(stats.optimized_issue_size <= stats.validation_issue_size);
        assert!(stats.compact_instruction_size <= stats.instruction_size);
        assert!(stats.compact_type_size <= stats.compiled_type_size);

        // CompactType should be exactly 1 byte
        assert_eq!(stats.compact_type_size, 1);
    }

    #[test]
    fn test_compact_type_conversion() {
        let types = vec![
            CompiledType::String,
            CompiledType::Integer,
            CompiledType::Float,
            CompiledType::Boolean,
            CompiledType::Date,
            CompiledType::DateTime,
            CompiledType::Uri,
            CompiledType::Object,
            CompiledType::Array,
            CompiledType::Any,
        ];

        for compiled in types {
            let compact = CompactType::from(compiled.clone());
            let back = CompiledType::from(compact);
            assert_eq!(compiled, back);
        }
    }

    #[test]
    fn test_context_pool() {
        let pool = ValidationContextPool::new(2);

        // Get a context
        let mut ctx1 = pool.get();
        ctx1.insert("key".to_string(), serde_json::json!("value"));

        // Return it
        pool.put(ctx1);

        // Get it again - should be empty
        let ctx2 = pool.get();
        assert!(ctx2.is_empty());
    }
}
