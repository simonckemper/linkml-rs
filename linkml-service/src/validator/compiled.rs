//! Compiled validator for optimized validation performance

use super::context::ValidationContext;
use super::report::{Severity, ValidationIssue};
use super::validators::Validator;
use linkml_core::error::{LinkMLError, Result as LinkMLResult};
use linkml_core::prelude::*;
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use bitflags::bitflags;

bitflags! {
    /// Optimization flags for validator compilation
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
    pub struct CompilationOptions: u8 {
        /// Enable regex compilation and caching
        const COMPILE_PATTERNS = 0b00001;
        /// Enable value range optimization
        const OPTIMIZE_RANGES = 0b00010;
        /// Enable type checking optimization
        const OPTIMIZE_TYPES = 0b00100;
        /// Pre-compute inheritance chains
        const PRECOMPUTE_INHERITANCE = 0b01000;
        /// Cache permissible values as hash sets
        const CACHE_PERMISSIBLE_VALUES = 0b10000;

        /// All optimizations enabled (default)
        const ALL = Self::COMPILE_PATTERNS.bits()
                  | Self::OPTIMIZE_RANGES.bits()
                  | Self::OPTIMIZE_TYPES.bits()
                  | Self::PRECOMPUTE_INHERITANCE.bits()
                  | Self::CACHE_PERMISSIBLE_VALUES.bits();

        /// No optimizations (for debugging)
        const NONE = 0b00000;
    }
}

impl Default for CompilationOptions {
    fn default() -> Self {
        Self::ALL
    }
}

/// Compiled validation instruction
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ValidationInstruction {
    /// Check if a field exists
    CheckRequired {
        /// `JSON` path to the value
        path: String,
        /// Field name to check
        field: String,
    },

    /// Validate against a compiled regex
    ValidatePattern {
        /// `JSON` path to the value
        path: String,
        /// Index into compiled patterns array
        pattern_id: usize,
    },

    /// Check numeric range
    ValidateRange {
        /// `JSON` path to the value
        path: String,
        /// Minimum value (inclusive/exclusive based on inclusive flag)
        min: Option<f64>,
        /// Maximum value (inclusive/exclusive based on inclusive flag)
        max: Option<f64>,
        /// Whether bounds are inclusive
        inclusive: bool,
    },

    /// Check string length
    ValidateLength {
        /// `JSON` path to the value
        path: String,
        /// Minimum length
        min: Option<usize>,
        /// Maximum length
        max: Option<usize>,
    },

    /// Validate against permissible values
    ValidateEnum {
        /// `JSON` path to the value
        path: String,
        /// Index into cached enums array
        enum_id: usize,
    },

    /// Type validation
    ValidateType {
        /// `JSON` path to the value
        path: String,
        /// Expected type
        expected_type: CompiledType,
    },

    /// Validate array elements
    ValidateArray {
        /// `JSON` path to the array
        path: String,
        /// Instructions to apply to each element
        element_instructions: Vec<ValidationInstruction>,
    },

    /// Conditional validation
    ConditionalValidation {
        /// Condition to evaluate
        condition: Box<ValidationInstruction>,
        /// Instructions to execute if condition passes
        then_instructions: Vec<ValidationInstruction>,
        /// Instructions to execute if condition fails
        else_instructions: Option<Vec<ValidationInstruction>>,
    },

    /// Nested object validation
    ValidateObject {
        /// `JSON` path to the object
        path: String,
        /// Instructions for each field
        field_instructions: HashMap<String, Vec<ValidationInstruction>>,
    },
}

/// Compiled type representation for fast checking
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum CompiledType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Float type
    Float,
    /// Boolean type
    Boolean,
    /// Date type
    Date,
    /// `DateTime` type
    DateTime,
    /// URI type
    Uri,
    /// Object type
    Object,
    /// Array type
    Array,
    /// Any type (no validation)
    Any,
}

/// Compiled validator with optimized validation logic
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledValidator {
    /// Validator name
    pub name: String,

    /// Compiled validation instructions
    pub instructions: Vec<ValidationInstruction>,

    /// Compiled regex patterns (stored as strings for serialization)
    #[serde(skip)]
    pub compiled_patterns: Vec<regex::Regex>,

    /// Pattern strings for serialization
    pub pattern_strings: Vec<String>,

    /// Cached permissible value sets
    pub cached_enums: Vec<std::collections::HashSet<String>>,

    /// Schema metadata
    pub schema_id: String,

    /// Class or slot being validated
    pub target_name: String,
}

impl Default for CompiledValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CompiledValidator {
    /// Create a new empty compiled validator (for testing)
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "test".to_string(),
            instructions: Vec::new(),
            compiled_patterns: Vec::new(),
            pattern_strings: Vec::new(),
            cached_enums: Vec::new(),
            schema_id: "test-schema".to_string(),
            target_name: "TestClass".to_string(),
        }
    }

    /// Compile a validator from a schema and class/slot definition
    ///
    /// # Errors
    ///
    /// Returns an error if the operation fails.
    pub fn compile_class(
        schema: &SchemaDefinition,
        class_name: &str,
        class: &ClassDefinition,
        options: CompilationOptions,
    ) -> LinkMLResult<Self> {
        let mut compiler = ValidatorCompiler::new(schema, &options);
        compiler.compile_class(class_name, class)
    }

    /// Execute compiled validation instructions
    pub fn execute(
        &self,
        value: &JsonValue,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        for instruction in &self.instructions {
            issues.extend(self.execute_instruction(instruction, value, context));
        }

        issues
    }

    /// Create a validation issue for range violations
    fn create_range_violation_issue(
        &self,
        path: &str,
        num: f64,
        min: Option<&f64>,
        max: Option<&f64>,
    ) -> ValidationIssue {
        let mut context = HashMap::new();
        if let Some(json_num) = serde_json::Number::from_f64(num) {
            context.insert("value".to_string(), serde_json::Value::Number(json_num));
        }
        if let Some(min_val) = min
            && let Some(json_min) = serde_json::Number::from_f64(*min_val)
        {
            context.insert("min".to_string(), serde_json::Value::Number(json_min));
        }
        if let Some(max_val) = max
            && let Some(json_max) = serde_json::Number::from_f64(*max_val)
        {
            context.insert("max".to_string(), serde_json::Value::Number(json_max));
        }
        ValidationIssue {
            severity: Severity::Error,
            path: path.to_string(),
            message: format!("Value {num} is out of range"),
            validator: self.name.clone(),
            code: Some("range_violation".to_string()),
            context,
        }
    }

    /// Execute a single validation instruction
    #[allow(clippy::only_used_in_recursion)]
    fn execute_instruction(
        &self,
        instruction: &ValidationInstruction,
        value: &JsonValue,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        match instruction {
            ValidationInstruction::CheckRequired { path, field } => {
                self.validate_required_field(value, path, field)
            }

            ValidationInstruction::ValidatePattern { path, pattern_id } => {
                self.validate_pattern(value, path, *pattern_id)
            }

            ValidationInstruction::ValidateRange {
                path,
                min,
                max,
                inclusive,
            } => self.validate_range(value, path, min.as_ref(), max.as_ref(), *inclusive),

            ValidationInstruction::ValidateEnum { path, enum_id } => {
                self.execute_enum_validation(value, path, *enum_id)
            }

            ValidationInstruction::ValidateType {
                path,
                expected_type,
            } => self.execute_type_validation(value, path, expected_type),

            ValidationInstruction::ValidateArray {
                path,
                element_instructions,
            } => self.execute_array_validation(value, path, element_instructions, context),

            ValidationInstruction::ValidateObject {
                path,
                field_instructions,
            } => self.execute_object_validation(value, path, field_instructions, context),

            ValidationInstruction::ConditionalValidation {
                condition,
                then_instructions,
                else_instructions,
            } => self.execute_conditional_validation(
                value,
                condition,
                then_instructions,
                else_instructions.as_ref(),
                context,
            ),

            ValidationInstruction::ValidateLength { path, min, max } => {
                self.execute_length_validation(value, path, min.as_ref(), max.as_ref())
            }
        }
    }

    /// Execute enum validation instruction
    fn execute_enum_validation(
        &self,
        value: &JsonValue,
        path: &str,
        enum_id: usize,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(field_value) = Self::extract_value_at_path(value, path)
            && let Some(s) = field_value.as_str()
            && let Some(enum_set) = self.cached_enums.get(enum_id)
            && !enum_set.contains(s)
        {
            let mut context = HashMap::new();
            context.insert(
                "value".to_string(),
                serde_json::Value::String(s.to_string()),
            );
            issues.push(ValidationIssue {
                severity: Severity::Error,
                path: path.to_string(),
                message: format!("Value '{s}' is not a permissible value"),
                validator: self.name.clone(),
                code: Some("enum_violation".to_string()),
                context,
            });
        }

        issues
    }

    /// Execute type validation instruction
    fn execute_type_validation(
        &self,
        value: &JsonValue,
        path: &str,
        expected_type: &CompiledType,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(field_value) = Self::extract_value_at_path(value, path) {
            let actual_type = Self::get_json_type(field_value);
            // Special handling for Date and DateTime types - they are strings in JSON
            let type_mismatch = match expected_type {
                CompiledType::Date | CompiledType::DateTime => actual_type != CompiledType::String,
                _ => actual_type != *expected_type && *expected_type != CompiledType::Any,
            };

            if type_mismatch {
                let mut context = HashMap::new();
                context.insert(
                    "expected_type".to_string(),
                    serde_json::Value::String(format!("{expected_type:?}")),
                );
                context.insert(
                    "actual_type".to_string(),
                    serde_json::Value::String(format!("{actual_type:?}")),
                );
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: path.to_string(),
                    message: format!("Expected type {expected_type:?}, got {actual_type:?}"),
                    validator: self.name.clone(),
                    code: Some("type_mismatch".to_string()),
                    context,
                });
            }
        }

        issues
    }

    /// Execute array validation instruction
    fn execute_array_validation(
        &self,
        value: &JsonValue,
        path: &str,
        element_instructions: &[ValidationInstruction],
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(arr) = value.as_array() {
            for (i, elem) in arr.iter().enumerate() {
                let elem_path = format!("{path}[{i}]");
                for inst in element_instructions {
                    let mut elem_inst = inst.clone();
                    self.update_instruction_path(&mut elem_inst, &elem_path);
                    issues.extend(self.execute_instruction(&elem_inst, elem, context));
                }
            }
        }

        issues
    }

    /// Execute object validation instruction
    fn execute_object_validation(
        &self,
        value: &JsonValue,
        path: &str,
        field_instructions: &HashMap<String, Vec<ValidationInstruction>>,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(obj) = value.as_object() {
            for (field, instructions) in field_instructions {
                if let Some(field_value) = obj.get(field) {
                    let field_path = format!("{path}.{field}");
                    for inst in instructions {
                        let mut field_inst = inst.clone();
                        self.update_instruction_path(&mut field_inst, &field_path);
                        issues.extend(self.execute_instruction(&field_inst, field_value, context));
                    }
                }
            }
        }

        issues
    }

    /// Execute conditional validation instruction
    fn execute_conditional_validation(
        &self,
        value: &JsonValue,
        condition: &ValidationInstruction,
        then_instructions: &[ValidationInstruction],
        else_instructions: Option<&Vec<ValidationInstruction>>,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Evaluate condition
        let condition_result = self.execute_instruction(condition, value, context);

        // If condition passes (no issues), execute then branch
        if condition_result.is_empty() {
            for inst in then_instructions {
                issues.extend(self.execute_instruction(inst, value, context));
            }
        } else if let Some(else_insts) = else_instructions {
            // Otherwise execute else branch if present
            for inst in else_insts {
                issues.extend(self.execute_instruction(inst, value, context));
            }
        }

        issues
    }

    /// Execute length validation instruction
    fn execute_length_validation(
        &self,
        value: &JsonValue,
        path: &str,
        min: Option<&usize>,
        max: Option<&usize>,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        if let Some(field_value) = Self::extract_value_at_path(value, path)
            && let Some(s) = field_value.as_str()
        {
            let len = s.chars().count();
            let valid = match (min, max) {
                (Some(min_len), Some(max_len)) => len >= *min_len && len <= *max_len,
                (Some(min_len), None) => len >= *min_len,
                (None, Some(max_len)) => len <= *max_len,
                (None, None) => true,
            };

            if !valid {
                let mut context_map = HashMap::new();
                context_map.insert("length".to_string(), serde_json::Value::Number(len.into()));
                if let Some(min_len) = min {
                    context_map.insert(
                        "min".to_string(),
                        serde_json::Value::Number((*min_len).into()),
                    );
                }
                if let Some(max_len) = max {
                    context_map.insert(
                        "max".to_string(),
                        serde_json::Value::Number((*max_len).into()),
                    );
                }
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    path: path.to_string(),
                    message: format!("String length {len} is out of range"),
                    validator: self.name.clone(),
                    code: Some("length_violation".to_string()),
                    context: context_map,
                });
            }
        }

        issues
    }

    /// Validate required field presence
    fn validate_required_field(
        &self,
        value: &JsonValue,
        path: &str,
        field: &str,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        if let Some(obj) = value.as_object()
            && !obj.contains_key(field)
        {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                path: path.to_string(),
                message: format!("Required field '{field}' is missing"),
                validator: self.name.clone(),
                code: Some("required_field_missing".to_string()),
                context: HashMap::new(),
            });
        }
        issues
    }

    /// Validate pattern matching
    fn validate_pattern(
        &self,
        value: &JsonValue,
        path: &str,
        pattern_id: usize,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        if let Some(field_value) = Self::extract_value_at_path(value, path)
            && let Some(s) = field_value.as_str()
            && let Some(pattern) = self.compiled_patterns.get(pattern_id)
            && !pattern.is_match(s)
        {
            let mut context = HashMap::new();
            context.insert(
                "value".to_string(),
                serde_json::Value::String(s.to_string()),
            );
            context.insert(
                "pattern".to_string(),
                serde_json::Value::String(pattern.as_str().to_string()),
            );
            issues.push(ValidationIssue {
                severity: Severity::Error,
                path: path.to_string(),
                message: format!("Value does not match pattern: {}", pattern.as_str()),
                validator: self.name.clone(),
                code: Some("pattern_mismatch".to_string()),
                context,
            });
        }
        issues
    }

    /// Validate numeric range
    fn validate_range(
        &self,
        value: &JsonValue,
        path: &str,
        min: Option<&f64>,
        max: Option<&f64>,
        inclusive: bool,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        if let Some(field_value) = Self::extract_value_at_path(value, path)
            && let Some(num) = field_value.as_f64()
        {
            let valid = match (min, max) {
                (Some(min_val), Some(max_val)) => {
                    if inclusive {
                        num >= *min_val && num <= *max_val
                    } else {
                        num > *min_val && num < *max_val
                    }
                }
                (Some(min_val), None) => {
                    if inclusive {
                        num >= *min_val
                    } else {
                        num > *min_val
                    }
                }
                (None, Some(max_val)) => {
                    if inclusive {
                        num <= *max_val
                    } else {
                        num < *max_val
                    }
                }
                (None, None) => true,
            };

            if !valid {
                issues.push(self.create_range_violation_issue(path, num, min, max));
            }
        }
        issues
    }

    /// Extract value at a `JSON` path
    /// Path format: $ (root), $.field, $.field.subfield
    fn extract_value_at_path<'a>(root: &'a JsonValue, path: &str) -> Option<&'a JsonValue> {
        if path == "$" {
            return Some(root);
        }

        let mut current = root;
        let parts: Vec<&str> = path.trim_start_matches("$.").split('.').collect();

        for part in parts {
            match current {
                JsonValue::Object(obj) => {
                    current = obj.get(part)?;
                }
                JsonValue::Array(arr) => {
                    // Handle array index
                    if let Ok(index) = part.parse::<usize>() {
                        current = arr.get(index)?;
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }

        Some(current)
    }

    /// Get the compiled type of a `JSON` value
    fn get_json_type(value: &JsonValue) -> CompiledType {
        match value {
            JsonValue::String(_) => CompiledType::String,
            JsonValue::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    CompiledType::Integer
                } else {
                    CompiledType::Float
                }
            }
            JsonValue::Bool(_) => CompiledType::Boolean,
            JsonValue::Array(_) => CompiledType::Array,
            JsonValue::Object(_) => CompiledType::Object,
            JsonValue::Null => CompiledType::Any,
        }
    }

    /// Update instruction path for nested validation
    fn update_instruction_path(&self, instruction: &mut ValidationInstruction, new_path: &str) {
        match instruction {
            ValidationInstruction::CheckRequired { path, .. }
            | ValidationInstruction::ValidatePattern { path, .. }
            | ValidationInstruction::ValidateRange { path, .. }
            | ValidationInstruction::ValidateLength { path, .. }
            | ValidationInstruction::ValidateEnum { path, .. }
            | ValidationInstruction::ValidateType { path, .. }
            | ValidationInstruction::ValidateArray { path, .. }
            | ValidationInstruction::ValidateObject { path, .. } => {
                *path = new_path.to_string();
            }
            ValidationInstruction::ConditionalValidation {
                condition,
                then_instructions,
                else_instructions,
            } => {
                // Update paths in condition and branches
                self.update_instruction_path(condition, new_path);
                for inst in then_instructions {
                    self.update_instruction_path(inst, new_path);
                }
                if let Some(else_insts) = else_instructions {
                    for inst in else_insts {
                        self.update_instruction_path(inst, new_path);
                    }
                }
            }
        }
    }
}

impl Validator for CompiledValidator {
    fn name(&self) -> &str {
        &self.name
    }

    fn validate(
        &self,
        value: &JsonValue,
        _slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        self.execute(value, context)
    }
}

/// Compiler for creating optimized validators
struct ValidatorCompiler<'a> {
    schema: &'a SchemaDefinition,
    options: &'a CompilationOptions,
    compiled_patterns: Vec<regex::Regex>,
    pattern_strings: Vec<String>,
    cached_enums: Vec<std::collections::HashSet<String>>,
    pattern_map: HashMap<String, usize>,
    enum_map: HashMap<String, usize>,
}

impl<'a> ValidatorCompiler<'a> {
    fn new(schema: &'a SchemaDefinition, options: &'a CompilationOptions) -> Self {
        Self {
            schema,
            options,
            compiled_patterns: Vec::new(),
            pattern_strings: Vec::new(),
            cached_enums: Vec::new(),
            pattern_map: HashMap::new(),
            enum_map: HashMap::new(),
        }
    }

    fn compile_class(
        &mut self,
        class_name: &str,
        class: &ClassDefinition,
    ) -> LinkMLResult<CompiledValidator> {
        let mut instructions = Vec::new();

        // Compile slot validations
        for slot_name in &class.slots {
            if let Some(slot) = self.schema.slots.get(slot_name) {
                let slot_instructions = self.compile_slot(slot_name, slot)?;
                instructions.extend(slot_instructions);
            }
        }

        // Handle inheritance if precompute_inheritance is enabled
        if self
            .options
            .contains(CompilationOptions::PRECOMPUTE_INHERITANCE)
            && let Some(parent_name) = &class.is_a
            && let Some(parent_class) = self.schema.classes.get(parent_name)
        {
            let parent_instructions = self.compile_inherited_slots(parent_class)?;
            instructions.extend(parent_instructions);
        }

        Ok(CompiledValidator {
            name: format!("compiled_validator_{class_name}"),
            instructions,
            compiled_patterns: self.compiled_patterns.clone(),
            pattern_strings: self.pattern_strings.clone(),
            cached_enums: self.cached_enums.clone(),
            schema_id: self.schema.id.clone(),
            target_name: class_name.to_string(),
        })
    }

    fn compile_slot(
        &mut self,
        slot_name: &str,
        slot: &SlotDefinition,
    ) -> LinkMLResult<Vec<ValidationInstruction>> {
        let mut instructions = Vec::new();
        let path = format!("$.{slot_name}");

        // Required field check
        if slot.required == Some(true) {
            instructions.push(ValidationInstruction::CheckRequired {
                path: "$".to_string(),
                field: slot_name.to_string(),
            });
        }

        // Pattern validation
        if let Some(pattern) = &slot.pattern
            && self.options.contains(CompilationOptions::COMPILE_PATTERNS)
        {
            let pattern_id = self.compile_pattern(pattern)?;
            instructions.push(ValidationInstruction::ValidatePattern {
                path: path.clone(),
                pattern_id,
            });
        }

        // Range validation
        if self.options.contains(CompilationOptions::OPTIMIZE_RANGES)
            && (slot.minimum_value.is_some() || slot.maximum_value.is_some())
        {
            instructions.push(ValidationInstruction::ValidateRange {
                path: path.clone(),
                min: slot
                    .minimum_value
                    .as_ref()
                    .and_then(linkml_core::Value::as_f64),
                max: slot
                    .maximum_value
                    .as_ref()
                    .and_then(linkml_core::Value::as_f64),
                inclusive: true,
            });
        }

        // Type validation
        if self.options.contains(CompilationOptions::OPTIMIZE_TYPES)
            && let Some(range) = &slot.range
        {
            let compiled_type = self.compile_type(range);
            instructions.push(ValidationInstruction::ValidateType {
                path: path.clone(),
                expected_type: compiled_type,
            });
        }

        // Enum validation
        if let Some(range) = &slot.range
            && let Some(enum_def) = self.schema.enums.get(range)
            && self
                .options
                .contains(CompilationOptions::CACHE_PERMISSIBLE_VALUES)
        {
            let enum_id = self.cache_enum(range, enum_def);
            instructions.push(ValidationInstruction::ValidateEnum {
                path: path.clone(),
                enum_id,
            });
        }

        // Array validation
        if slot.multivalued == Some(true) {
            let element_instructions = if let Some(range) = &slot.range {
                vec![ValidationInstruction::ValidateType {
                    path: "$".to_string(),
                    expected_type: self.compile_type(range),
                }]
            } else {
                vec![]
            };

            instructions = vec![ValidationInstruction::ValidateArray {
                path,
                element_instructions,
            }];
        }

        Ok(instructions)
    }

    fn compile_inherited_slots(
        &mut self,
        parent_class: &ClassDefinition,
    ) -> LinkMLResult<Vec<ValidationInstruction>> {
        let mut instructions = Vec::new();

        for slot_name in &parent_class.slots {
            if let Some(slot) = self.schema.slots.get(slot_name) {
                let slot_instructions = self.compile_slot(slot_name, slot)?;
                instructions.extend(slot_instructions);
            }
        }

        // Recursively compile parent's parent
        if let Some(grandparent_name) = &parent_class.is_a
            && let Some(grandparent_class) = self.schema.classes.get(grandparent_name)
        {
            let grandparent_instructions = self.compile_inherited_slots(grandparent_class)?;
            instructions.extend(grandparent_instructions);
        }

        Ok(instructions)
    }

    /// Compile a regex pattern and return its ID for reuse
    ///
    /// # Errors
    /// Returns `LinkMLError::SchemaValidation` if the regex pattern is invalid
    fn compile_pattern(&mut self, pattern: &str) -> LinkMLResult<usize> {
        if let Some(&id) = self.pattern_map.get(pattern) {
            return Ok(id);
        }

        let regex = regex::Regex::new(pattern)
            .map_err(|e| LinkMLError::schema_validation(format!("Invalid regex pattern: {e}")))?;

        let id = self.compiled_patterns.len();
        self.compiled_patterns.push(regex);
        self.pattern_strings.push(pattern.to_string());
        self.pattern_map.insert(pattern.to_string(), id);

        Ok(id)
    }

    fn cache_enum(&mut self, name: &str, enum_def: &EnumDefinition) -> usize {
        if let Some(&id) = self.enum_map.get(name) {
            return id;
        }

        let mut enum_set = std::collections::HashSet::new();
        for value in &enum_def.permissible_values {
            match value {
                PermissibleValue::Simple(s) => {
                    enum_set.insert(s.clone());
                }
                PermissibleValue::Complex { text, .. } => {
                    enum_set.insert(text.clone());
                }
            }
        }

        let id = self.cached_enums.len();
        self.cached_enums.push(enum_set);
        self.enum_map.insert(name.to_string(), id);

        id
    }

    fn compile_type(&self, type_name: &str) -> CompiledType {
        match type_name {
            "string" | "str" => CompiledType::String,
            "integer" | "int" => CompiledType::Integer,
            "float" | "double" | "decimal" => CompiledType::Float,
            "boolean" | "bool" => CompiledType::Boolean,
            "date" => CompiledType::Date,
            "datetime" => CompiledType::DateTime,
            "uri" | "url" => CompiledType::Uri,
            _ => {
                // Check if it's a class reference
                if self.schema.classes.contains_key(type_name) {
                    CompiledType::Object
                } else if self.schema.enums.contains_key(type_name) {
                    CompiledType::String // Enums are strings
                } else {
                    CompiledType::Any
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[tokio::test]
    async fn test_compiled_validator() -> anyhow::Result<()> {
        let mut schema = SchemaDefinition::default();

        // Add a simple slot
        let mut name_slot = SlotDefinition::default();
        name_slot.name = "name".to_string();
        name_slot.range = Some("string".to_string());
        name_slot.required = Some(true);
        name_slot.pattern = Some("^[A-Za-z]+$".to_string());

        schema.slots.insert("name".to_string(), name_slot);

        // Add a class
        let mut person_class = ClassDefinition::default();
        person_class.name = "Person".to_string();
        person_class.slots = vec!["name".to_string()];

        // Compile validator
        let options = CompilationOptions::default();
        let validator = CompiledValidator::compile_class(&schema, "Person", &person_class, options)
            .expect("Failed to compile validator: {}");

        // Test valid data
        let valid_data = serde_json::json!({
            "name": "John"
        });

        let mut context = ValidationContext::new(std::sync::Arc::new(schema.clone()));
        assert!(validator.execute(&valid_data, &mut context).is_empty());

        // Test invalid data (missing required field)
        let invalid_data = serde_json::json!({});

        let mut context = ValidationContext::new(std::sync::Arc::new(schema.clone()));
        assert!(!validator.execute(&invalid_data, &mut context).is_empty());

        // Test invalid pattern
        let invalid_pattern = serde_json::json!({
            "name": "John123"
        });

        let mut context = ValidationContext::new(std::sync::Arc::new(schema.clone()));
        assert!(!validator.execute(&invalid_pattern, &mut context).is_empty());
        Ok(())
    }
}
