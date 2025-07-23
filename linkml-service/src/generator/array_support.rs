//! Array support for code generators
//!
//! This module provides utilities for generating array-aware code
//! in various programming languages.

use crate::array::{ArraySpec, ArrayDimension};
use linkml_core::prelude::*;
use std::fmt::Write;

/// Language-specific array code generation
pub trait ArrayCodeGenerator {
    /// Generate array type declaration
    fn generate_array_type(&self, spec: &ArraySpec, type_name: &str) -> String;
    
    /// Generate array initialization code
    fn generate_array_init(&self, spec: &ArraySpec, var_name: &str) -> String;
    
    /// Generate array validation code
    fn generate_array_validation(&self, spec: &ArraySpec, var_name: &str) -> String;
    
    /// Generate array accessor method
    fn generate_array_accessor(&self, spec: &ArraySpec, method_name: &str) -> String;
}

/// Python/NumPy array code generator
pub struct PythonArrayGenerator;

impl ArrayCodeGenerator for PythonArrayGenerator {
    fn generate_array_type(&self, spec: &ArraySpec, type_name: &str) -> String {
        let dtype = match spec.element_type.as_str() {
            "integer" => "np.int64",
            "float" | "double" => "np.float64",
            "boolean" => "np.bool_",
            _ => "np.object_",
        };
        
        if spec.is_fixed_shape() {
            let shape = spec.fixed_shape().unwrap();
            format!("NDArray[Literal[{}], {}]", 
                shape.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
                dtype)
        } else {
            format!("NDArray[Any, {}]", dtype)
        }
    }
    
    fn generate_array_init(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        let dtype = match spec.element_type.as_str() {
            "integer" => "np.int64",
            "float" | "double" => "np.float64",
            "boolean" => "bool",
            _ => "object",
        };
        
        if let Some(shape) = spec.fixed_shape() {
            let shape_str = shape.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            
            if spec.allow_missing && spec.missing_value.is_some() {
                writeln!(&mut code, "{} = np.full(({},), {}, dtype={})",
                    var_name, shape_str, 
                    spec.missing_value.as_ref().unwrap(),
                    dtype).unwrap();
            } else {
                writeln!(&mut code, "{} = np.zeros(({},), dtype={})",
                    var_name, shape_str, dtype).unwrap();
            }
        } else {
            writeln!(&mut code, "{} = np.array([], dtype={})", 
                var_name, dtype).unwrap();
        }
        
        code
    }
    
    fn generate_array_validation(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        
        writeln!(&mut code, "def validate_{}_array(arr):", var_name).unwrap();
        writeln!(&mut code, "    \"\"\"Validate array shape and type\"\"\"").unwrap();
        
        // Check number of dimensions
        writeln!(&mut code, "    if arr.ndim != {}:", spec.ndim()).unwrap();
        writeln!(&mut code, "        raise ValueError(f\"Expected {} dimensions, got {{arr.ndim}}\")", 
            spec.ndim()).unwrap();
        
        // Check each dimension
        for (i, dim) in spec.dimensions.iter().enumerate() {
            writeln!(&mut code, "    # Validate dimension '{}' (axis {})", dim.name, i).unwrap();
            
            if let Some(size) = dim.size {
                writeln!(&mut code, "    if arr.shape[{}] != {}:", i, size).unwrap();
                writeln!(&mut code, "        raise ValueError(f\"Dimension '{}' expected size {}, got {{arr.shape[{}]}}\")", 
                    dim.name, size, i).unwrap();
            }
            
            if let Some(min) = dim.min_size {
                writeln!(&mut code, "    if arr.shape[{}] < {}:", i, min).unwrap();
                writeln!(&mut code, "        raise ValueError(f\"Dimension '{}' minimum size is {}, got {{arr.shape[{}]}}\")", 
                    dim.name, min, i).unwrap();
            }
            
            if let Some(max) = dim.max_size {
                writeln!(&mut code, "    if arr.shape[{}] > {}:", i, max).unwrap();
                writeln!(&mut code, "        raise ValueError(f\"Dimension '{}' maximum size is {}, got {{arr.shape[{}]}}\")", 
                    dim.name, max, i).unwrap();
            }
        }
        
        writeln!(&mut code, "    return True").unwrap();
        code
    }
    
    fn generate_array_accessor(&self, spec: &ArraySpec, method_name: &str) -> String {
        let mut code = String::new();
        
        let indices = spec.dimensions.iter()
            .map(|d| format!("{}: int", d.name))
            .collect::<Vec<_>>()
            .join(", ");
        
        let index_args = spec.dimensions.iter()
            .map(|d| &d.name)
            .collect::<Vec<_>>()
            .join(", ");
        
        writeln!(&mut code, "def {}(self, {}) -> {}:", 
            method_name, indices, 
            self.python_type(&spec.element_type)).unwrap();
        writeln!(&mut code, "    \"\"\"Get array element\"\"\"").unwrap();
        writeln!(&mut code, "    return self._array[{}]", index_args).unwrap();
        
        writeln!(&mut code).unwrap();
        writeln!(&mut code, "def set_{}(self, {}, value: {}):", 
            method_name, indices,
            self.python_type(&spec.element_type)).unwrap();
        writeln!(&mut code, "    \"\"\"Set array element\"\"\"").unwrap();
        writeln!(&mut code, "    self._array[{}] = value", index_args).unwrap();
        
        code
    }
}

impl PythonArrayGenerator {
    fn python_type(&self, linkml_type: &str) -> &'static str {
        match linkml_type {
            "string" => "str",
            "integer" => "int",
            "float" | "double" | "decimal" => "float",
            "boolean" => "bool",
            "date" => "date",
            "datetime" => "datetime",
            _ => "Any",
        }
    }
}

/// TypeScript array code generator
pub struct TypeScriptArrayGenerator;

impl ArrayCodeGenerator for TypeScriptArrayGenerator {
    fn generate_array_type(&self, spec: &ArraySpec, _type_name: &str) -> String {
        let base_type = match spec.element_type.as_str() {
            "integer" | "float" | "double" => "number",
            "string" => "string",
            "boolean" => "boolean",
            _ => "any",
        };
        
        // Generate nested array type based on dimensions
        let mut array_type = base_type.to_string();
        for _ in 0..spec.ndim() {
            array_type = format!("{}[]", array_type);
        }
        
        array_type
    }
    
    fn generate_array_init(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        
        if let Some(shape) = spec.fixed_shape() {
            // Generate nested array initialization
            writeln!(&mut code, "const {} = createArray({}, {});",
                var_name,
                shape.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", "),
                self.default_value(&spec.element_type)).unwrap();
        } else {
            writeln!(&mut code, "const {}: {} = [];", 
                var_name, 
                self.generate_array_type(spec, "")).unwrap();
        }
        
        code
    }
    
    fn generate_array_validation(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        
        writeln!(&mut code, "function validate{}Array(arr: any): boolean {{", 
            capitalize_first(var_name)).unwrap();
        writeln!(&mut code, "  if (!Array.isArray(arr)) return false;").unwrap();
        
        // For fixed dimensions, generate recursive validation
        if spec.is_fixed_shape() {
            let shape = spec.fixed_shape().unwrap();
            self.generate_shape_validation(&mut code, &shape, 0, "arr");
        }
        
        writeln!(&mut code, "  return true;").unwrap();
        writeln!(&mut code, "}}").unwrap();
        
        code
    }
    
    fn generate_array_accessor(&self, spec: &ArraySpec, method_name: &str) -> String {
        let mut code = String::new();
        
        let params = spec.dimensions.iter()
            .map(|d| format!("{}: number", d.name))
            .collect::<Vec<_>>()
            .join(", ");
        
        let indices = spec.dimensions.iter()
            .map(|d| format!("[{}]", d.name))
            .collect::<Vec<_>>()
            .join("");
        
        writeln!(&mut code, "{}<T>({}): T {{", method_name, params).unwrap();
        writeln!(&mut code, "  return this.array{};", indices).unwrap();
        writeln!(&mut code, "}}").unwrap();
        
        code
    }
}

impl TypeScriptArrayGenerator {
    fn default_value(&self, element_type: &str) -> &'static str {
        match element_type {
            "integer" | "float" | "double" => "0",
            "string" => "''",
            "boolean" => "false",
            _ => "null",
        }
    }
    
    fn generate_shape_validation(&self, code: &mut String, shape: &[usize], depth: usize, var: &str) {
        if depth < shape.len() {
            writeln!(code, "{}if ({}.length !== {}) return false;",
                "  ".repeat(depth + 1), var, shape[depth]).unwrap();
            
            if depth < shape.len() - 1 {
                writeln!(code, "{}for (let i = 0; i < {}.length; i++) {{",
                    "  ".repeat(depth + 1), var).unwrap();
                writeln!(code, "{}if (!Array.isArray({}[i])) return false;",
                    "  ".repeat(depth + 2), var).unwrap();
                
                let next_var = format!("{}[i]", var);
                self.generate_shape_validation(code, shape, depth + 1, &next_var);
                
                writeln!(code, "{}}}", "  ".repeat(depth + 1)).unwrap();
            }
        }
    }
}

/// Rust array code generator
pub struct RustArrayGenerator;

impl ArrayCodeGenerator for RustArrayGenerator {
    fn generate_array_type(&self, spec: &ArraySpec, _type_name: &str) -> String {
        let element_type = match spec.element_type.as_str() {
            "integer" => "i64",
            "float" | "double" => "f64",
            "boolean" => "bool",
            "string" => "String",
            _ => "serde_json::Value",
        };
        
        if let Some(shape) = spec.fixed_shape() {
            // Use fixed-size arrays for small dimensions
            if shape.iter().all(|&s| s <= 32) {
                let mut array_type = element_type.to_string();
                for &size in shape.iter().rev() {
                    array_type = format!("[{}; {}]", array_type, size);
                }
                array_type
            } else {
                // Use ndarray for larger arrays
                format!("ndarray::Array{}<{}>", shape.len(), element_type)
            }
        } else {
            format!("ndarray::ArrayD<{}>", element_type)
        }
    }
    
    fn generate_array_init(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        
        if let Some(shape) = spec.fixed_shape() {
            let shape_str = shape.iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            
            writeln!(&mut code, "let {}: {} = ndarray::Array::zeros([{}]);",
                var_name,
                self.generate_array_type(spec, ""),
                shape_str).unwrap();
        } else {
            writeln!(&mut code, "let {}: {} = ndarray::ArrayD::zeros(vec![]);",
                var_name,
                self.generate_array_type(spec, "")).unwrap();
        }
        
        code
    }
    
    fn generate_array_validation(&self, spec: &ArraySpec, var_name: &str) -> String {
        let mut code = String::new();
        
        writeln!(&mut code, "impl {} {{", capitalize_first(var_name)).unwrap();
        writeln!(&mut code, "    /// Validate array dimensions").unwrap();
        writeln!(&mut code, "    pub fn validate_array(&self) -> Result<(), ArrayError> {{").unwrap();
        
        writeln!(&mut code, "        if self.array.ndim() != {} {{", spec.ndim()).unwrap();
        writeln!(&mut code, "            return Err(ArrayError::InvalidShape(").unwrap();
        writeln!(&mut code, "                format!(\"Expected {} dimensions, got {{}}\", self.array.ndim())", 
            spec.ndim()).unwrap();
        writeln!(&mut code, "            ));").unwrap();
        writeln!(&mut code, "        }}").unwrap();
        
        for (i, dim) in spec.dimensions.iter().enumerate() {
            if let Some(size) = dim.size {
                writeln!(&mut code, "        if self.array.shape()[{}] != {} {{", i, size).unwrap();
                writeln!(&mut code, "            return Err(ArrayError::ShapeMismatch {{").unwrap();
                writeln!(&mut code, "                expected: vec![{}],", size).unwrap();
                writeln!(&mut code, "                actual: vec![self.array.shape()[{}]],", i).unwrap();
                writeln!(&mut code, "            }});").unwrap();
                writeln!(&mut code, "        }}").unwrap();
            }
        }
        
        writeln!(&mut code, "        Ok(())").unwrap();
        writeln!(&mut code, "    }}").unwrap();
        writeln!(&mut code, "}}").unwrap();
        
        code
    }
    
    fn generate_array_accessor(&self, spec: &ArraySpec, method_name: &str) -> String {
        let mut code = String::new();
        
        let params = spec.dimensions.iter()
            .map(|d| format!("{}: usize", d.name))
            .collect::<Vec<_>>()
            .join(", ");
        
        let indices = spec.dimensions.iter()
            .map(|d| &d.name)
            .collect::<Vec<_>>()
            .join(", ");
        
        let return_type = match spec.element_type.as_str() {
            "string" => "&str",
            _ => self.rust_type(&spec.element_type),
        };
        
        writeln!(&mut code, "pub fn {}(&self, {}) -> {} {{", 
            method_name, params, return_type).unwrap();
        writeln!(&mut code, "    self.array[[{}]]", indices).unwrap();
        writeln!(&mut code, "}}").unwrap();
        
        code
    }
}

impl RustArrayGenerator {
    fn rust_type(&self, linkml_type: &str) -> &'static str {
        match linkml_type {
            "string" => "String",
            "integer" => "i64",
            "float" | "double" => "f64",
            "boolean" => "bool",
            _ => "serde_json::Value",
        }
    }
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Get array code generator for a language
pub fn get_array_generator(language: &str) -> Option<Box<dyn ArrayCodeGenerator>> {
    match language {
        "python" | "pydantic" => Some(Box::new(PythonArrayGenerator)),
        "typescript" | "javascript" => Some(Box::new(TypeScriptArrayGenerator)),
        "rust" => Some(Box::new(RustArrayGenerator)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::array::ArrayDimension;
    
    #[test]
    fn test_python_array_generation() {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 10))
            .with_dimension(ArrayDimension::fixed("y", 20));
        
        let gen = PythonArrayGenerator;
        
        let type_code = gen.generate_array_type(&spec, "MyArray");
        assert!(type_code.contains("NDArray"));
        assert!(type_code.contains("10, 20"));
        
        let init_code = gen.generate_array_init(&spec, "data");
        assert!(init_code.contains("np.zeros"));
        assert!(init_code.contains("(10, 20)"));
    }
    
    #[test]
    fn test_typescript_array_generation() {
        let spec = ArraySpec::new("integer")
            .with_dimension(ArrayDimension::fixed("rows", 3))
            .with_dimension(ArrayDimension::fixed("cols", 4));
        
        let gen = TypeScriptArrayGenerator;
        
        let type_code = gen.generate_array_type(&spec, "Matrix");
        assert_eq!(type_code, "number[][]");
        
        let validation = gen.generate_array_validation(&spec, "matrix");
        assert!(validation.contains("validateMatrixArray"));
        assert!(validation.contains("length !== 3"));
    }
    
    #[test]
    fn test_rust_array_generation() {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 100))
            .with_dimension(ArrayDimension::fixed("y", 200));
        
        let gen = RustArrayGenerator;
        
        let type_code = gen.generate_array_type(&spec, "Grid");
        assert!(type_code.contains("ndarray::Array2<f64>"));
        
        let init_code = gen.generate_array_init(&spec, "grid");
        assert!(init_code.contains("Array::zeros"));
        assert!(init_code.contains("[100, 200]"));
    }
}