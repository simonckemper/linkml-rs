//! Array support for code generators
//!
//! This module provides utilities for generating array-aware code
//! in various programming languages.

use crate::array::ArraySpec;
use std::fmt::Write;

/// Language-specific array code generation
pub trait ArrayCodeGenerator {
    /// Generate array type declaration
    fn generate_array_type(&self, spec: &ArraySpec, type_name: &str) -> String;

    /// Generate array initialization code
    ///
    /// # Errors
    /// Returns an error if array initialization code generation fails
    fn generate_array_init(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Generate array validation code
    ///
    /// # Errors
    /// Returns an error if array validation code generation fails
    fn generate_array_validation(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;

    /// Generate array accessor method
    ///
    /// # Errors
    /// Returns an error if array accessor code generation fails
    fn generate_array_accessor(
        &self,
        spec: &ArraySpec,
        method_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>>;
}

/// Python/NumPy array code generator
pub struct PythonArrayGenerator;

impl PythonArrayGenerator {
    /// Helper to handle write errors gracefully
    /// In practice, writing to a String should never fail, but we handle it properly
    fn write_or_default(dest: &mut String, args: std::fmt::Arguments) {
        if let Err(e) = dest.write_fmt(args) {
            // Log the error in production (would use proper logging)
            eprintln!("Warning: Failed to format string in array generator: {e}");
            // Continue with what we have
        }
    }
}

impl ArrayCodeGenerator for PythonArrayGenerator {
    fn generate_array_type(&self, spec: &ArraySpec, _type_name: &str) -> String {
        let dtype = match spec.element_type.as_str() {
            "integer" => "np.int64",
            "float" | "double" => "np.float64",
            "boolean" => "np.bool_",
            _ => "np.object_",
        };

        if spec.is_fixed_shape() {
            // We checked is_fixed_shape() so this should always succeed
            // But we handle the None case gracefully
            let shape = spec.fixed_shape().unwrap_or_else(|| {
                eprintln!("Warning: fixed_shape() returned None after is_fixed_shape() check");
                vec![] // Fallback to empty shape
            });
            format!(
                "NDArray[Literal[{}], {}]",
                shape
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                dtype
            )
        } else {
            format!("NDArray[Any, {dtype}]")
        }
    }

    fn generate_array_init(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();
        let dtype = match spec.element_type.as_str() {
            "integer" => "np.int64",
            "float" | "double" => "np.float64",
            "boolean" => "bool",
            _ => "object",
        };

        if let Some(shape) = spec.fixed_shape() {
            let shape_str = shape
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");

            if spec.allow_missing && spec.missing_value.is_some() {
                // We checked is_some() so use unwrap() since we know it's Some
                let missing_val = spec
                    .missing_value
                    .as_ref()
                    .expect("LinkML operation should succeed");
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "{var_name} = np.full(({shape_str},), {missing_val}, dtype={dtype})
"
                    ),
                );
            } else {
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "{var_name} = np.zeros(({shape_str}), dtype={dtype})
"
                    ),
                );
            }
        } else {
            Self::write_or_default(
                &mut code,
                format_args!(
                    "{var_name} = np.array([], dtype={dtype})
"
                ),
            );
        }

        Ok(code)
    }

    fn generate_array_validation(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        Self::write_or_default(
            &mut code,
            format_args!(
                "def validate_{var_name}_array(arr):
"
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "    \"\"\"Validate array shape and type\"\"\"
"
            ),
        );

        // Check number of dimensions
        Self::write_or_default(
            &mut code,
            format_args!(
                "    if arr.ndim != {}:
",
                spec.ndim()
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "        raise ValueError(f\"Expected {} dimensions, got {{arr.ndim}}\")
",
                spec.ndim()
            ),
        );

        // Check each dimension
        for (i, dim) in spec.dimensions.iter().enumerate() {
            Self::write_or_default(
                &mut code,
                format_args!(
                    "    # Validate dimension '{}' (axis {})
",
                    dim.name, i
                ),
            );

            if let Some(size) = dim.size {
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "    if arr.shape[{i}] != {size}:
"
                    ),
                );
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "        raise ValueError(f\"Dimension '{}' expected size {}, got {{arr.shape[{}]}}\")
",
                        dim.name, size, i
                    ),
                );
            }

            if let Some(min) = dim.min_size {
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "    if arr.shape[{i}] < {min}:
"
                    ),
                );
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "        raise ValueError(f\"Dimension '{}' minimum size is {}, got {{arr.shape[{}]}}\")
",
                        dim.name, min, i
                    ),
                );
            }

            if let Some(max) = dim.max_size {
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "    if arr.shape[{i}] > {max}:
"
                    ),
                );
                Self::write_or_default(
                    &mut code,
                    format_args!(
                        "        raise ValueError(f\"Dimension '{}' maximum size is {}, got {{arr.shape[{}]}}\")
",
                        dim.name, max, i
                    ),
                );
            }
        }

        Self::write_or_default(
            &mut code,
            format_args!(
                "    return True
"
            ),
        );
        Ok(code)
    }

    fn generate_array_accessor(
        &self,
        spec: &ArraySpec,
        method_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        let indices = spec
            .dimensions
            .iter()
            .map(|d| format!("{}: int", d.name))
            .collect::<Vec<_>>()
            .join(", ");

        let index_args = spec
            .dimensions
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        Self::write_or_default(
            &mut code,
            format_args!(
                "def {}(self, {}) -> {}:
",
                method_name,
                indices,
                Self::python_type(&spec.element_type)
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "    \"\"\"Get array element\"\"\"
"
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "    return self._array[{index_args}]
"
            ),
        );

        Self::write_or_default(
            &mut code,
            format_args!(
                "
"
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "def set_{}(self, {}, value: {}):
",
                method_name,
                indices,
                Self::python_type(&spec.element_type)
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "    \"\"\"Set array element\"\"\"
"
            ),
        );
        Self::write_or_default(
            &mut code,
            format_args!(
                "    self._array[{index_args}] = value
"
            ),
        );

        Ok(code)
    }
}

impl PythonArrayGenerator {
    fn python_type(linkml_type: &str) -> &'static str {
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
            array_type = format!("{array_type}[]");
        }

        array_type
    }

    fn generate_array_init(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        if let Some(shape) = spec.fixed_shape() {
            // Generate nested array initialization
            writeln!(
                &mut code,
                "const {} = createArray({}, {});",
                var_name,
                shape
                    .iter()
                    .map(std::string::ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                Self::default_value(&spec.element_type)
            )
            .expect("failed to format string: {}");
        } else {
            writeln!(
                &mut code,
                "const {}: {} = [];",
                var_name,
                self.generate_array_type(spec, "")
            )
            .expect("failed to format string: {}");
        }

        Ok(code)
    }

    fn generate_array_validation(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        writeln!(
            &mut code,
            "function validate{}Array(arr: any): boolean {{",
            capitalize_first(var_name)
        )
        .expect("failed to format string: {}");
        writeln!(&mut code, "  if (!Array.isArray(arr)) return false;")
            .expect("failed to format string: {}");

        // For fixed dimensions, generate recursive validation
        if spec.is_fixed_shape() {
            let shape = spec
                .fixed_shape()
                .ok_or_else(|| anyhow::anyhow!("Fixed shape not available"))?;
            // shape should return Some after is_fixed_shape() check
            let _ = self.generate_shape_validation(&mut code, &shape, 0, "arr");
        }

        writeln!(&mut code, "  return true;").expect("failed to format string: {}");
        writeln!(&mut code, "}}").expect("failed to format string: {}");

        Ok(code)
    }

    fn generate_array_accessor(
        &self,
        spec: &ArraySpec,
        method_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        let params = spec
            .dimensions
            .iter()
            .map(|d| format!("{}: number", d.name))
            .collect::<Vec<_>>()
            .join(", ");

        let mut indices = String::new();
        for d in &spec.dimensions {
            indices.push('[');
            indices.push_str(&d.name);
            indices.push(']');
        }

        writeln!(&mut code, "{method_name}<T>({params}): T {{")
            .expect("failed to format string: {}");
        writeln!(&mut code, "  return this.array{indices};").expect("failed to format string: {}");
        writeln!(&mut code, "}}").expect("failed to format string: {}");

        Ok(code)
    }
}

impl TypeScriptArrayGenerator {
    fn default_value(element_type: &str) -> &'static str {
        match element_type {
            "integer" | "float" | "double" => "0",
            "string" => r"''",
            "boolean" => "false",
            _ => "null",
        }
    }

    fn generate_shape_validation(
        &self,
        code: &mut String,
        shape: &[usize],
        depth: usize,
        var: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if depth < shape.len() {
            writeln!(
                code,
                "{}if ({}.length !== {}) return false;",
                "  ".repeat(depth + 1),
                var,
                shape[depth]
            )
            .expect("failed to format string: {}");

            if depth < shape.len() - 1 {
                writeln!(
                    code,
                    "{}for (let i = 0; i < {}.length; i++) {{",
                    "  ".repeat(depth + 1),
                    var
                )
                .expect("failed to format string: {}");
                writeln!(
                    code,
                    "{}if (!Array.isArray({}[i])) return false;",
                    "  ".repeat(depth + 2),
                    var
                )
                .expect("failed to format string: {}");

                let next_var = format!("{var}[i]");
                self.generate_shape_validation(code, shape, depth + 1, &next_var)?;

                writeln!(code, "{}}}", "  ".repeat(depth + 1))
                    .expect("failed to format string: {}");
            }
        }
        Ok(())
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
                    array_type = format!("[{array_type}; {size}]");
                }
                array_type
            } else {
                // Use ndarray for larger arrays
                format!("ndarray::Array{}<{}>", shape.len(), element_type)
            }
        } else {
            format!("ndarray::ArrayD<{element_type}>")
        }
    }

    fn generate_array_init(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        if let Some(shape) = spec.fixed_shape() {
            let shape_str = shape
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(
                &mut code,
                "let {}: {} = ndarray::Array::zeros([{}]);",
                var_name,
                self.generate_array_type(spec, ""),
                shape_str
            )
            .expect("failed to format string: {}");
        } else {
            writeln!(
                &mut code,
                "let {}: {} = ndarray::ArrayD::zeros(vec![]);",
                var_name,
                self.generate_array_type(spec, "")
            )
            .expect("failed to format string: {}");
        }

        Ok(code)
    }

    fn generate_array_validation(
        &self,
        spec: &ArraySpec,
        var_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        writeln!(&mut code, "impl {} {{", capitalize_first(var_name))
            .expect("failed to format string: {}");
        writeln!(&mut code, "    /// Validate array dimensions")
            .expect("failed to format string: {}");
        writeln!(
            &mut code,
            "    pub fn validate_array(&self) -> Result<(), ArrayError> {{"
        )
        .expect("failed to format string: {}");

        writeln!(
            &mut code,
            "        if self.array.ndim() != {} {{",
            spec.ndim()
        )
        .expect("failed to format string: {}");
        writeln!(
            &mut code,
            "            return Err(ArrayError::InvalidShape("
        )
        .expect("failed to format string: {}");
        writeln!(
            &mut code,
            "                format!(\"Expected {} dimensions, got {{}}\", self.array.ndim())",
            spec.ndim()
        )
        .expect("failed to format string: {}");
        writeln!(&mut code, "            ));").expect("failed to format string: {}");
        writeln!(&mut code, "        }}").expect("failed to format string: {}");

        for (i, dim) in spec.dimensions.iter().enumerate() {
            if let Some(size) = dim.size {
                writeln!(&mut code, "        if self.array.shape()[{i}] != {size} {{")
                    .expect("failed to format string: {}");
                writeln!(
                    &mut code,
                    "            return Err(ArrayError::ShapeMismatch {{"
                )
                .expect("failed to format string: {}");
                writeln!(&mut code, "                expected: vec![{size}],")
                    .expect("failed to format string: {}");
                writeln!(
                    &mut code,
                    "                actual: vec![self.array.shape()[{i}]],"
                )
                .expect("failed to format string: {}");
                writeln!(&mut code, "            }});").expect("failed to format string: {}");
                writeln!(&mut code, "        }}").expect("failed to format string: {}");
            }
        }

        writeln!(&mut code, "        Ok(())").expect("failed to format string: {}");
        writeln!(&mut code, "    }}").expect("failed to format string: {}");
        writeln!(&mut code, "}}").expect("failed to format string: {}");

        Ok(code)
    }

    fn generate_array_accessor(
        &self,
        spec: &ArraySpec,
        method_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let mut code = String::new();

        let params = spec
            .dimensions
            .iter()
            .map(|d| format!("{}: usize", d.name))
            .collect::<Vec<_>>()
            .join(", ");

        let indices = spec
            .dimensions
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");

        let return_type = match spec.element_type.as_str() {
            "string" => "&str",
            _ => Self::rust_type(&spec.element_type),
        };

        writeln!(
            &mut code,
            "pub fn {method_name}(&self, {params}) -> {return_type} {{"
        )
        .expect("failed to format string: {}");
        writeln!(&mut code, "    self.array[[{indices}]]").expect("failed to format string: {}");
        writeln!(&mut code, "}}").expect("failed to format string: {}");

        Ok(code)
    }
}

impl RustArrayGenerator {
    fn rust_type(linkml_type: &str) -> &'static str {
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
#[must_use]
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
    fn test_python_array_generation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 10))
            .with_dimension(ArrayDimension::fixed("y", 20));

        let generator = PythonArrayGenerator;

        let type_code = generator.generate_array_type(&spec, "MyArray");
        assert!(type_code.contains("NDArray"));
        assert!(type_code.contains("10, 20"));

        let init_code = generator.generate_array_init(&spec, "data")?;
        assert!(init_code.contains("np.zeros"));
        assert!(init_code.contains("(10, 20)"));
        Ok(())
    }

    #[test]
    fn test_typescript_array_generation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("integer")
            .with_dimension(ArrayDimension::fixed("rows", 3))
            .with_dimension(ArrayDimension::fixed("cols", 4));

        let generator = TypeScriptArrayGenerator;

        let type_code = generator.generate_array_type(&spec, "Matrix");
        assert_eq!(type_code, "number[][]");

        let validation = generator.generate_array_validation(&spec, "matrix")?;
        assert!(validation.contains("validateMatrixArray"));
        assert!(validation.contains("length !== 3"));
        Ok(())
    }

    #[test]
    fn test_rust_array_generation() -> Result<(), Box<dyn std::error::Error>> {
        let spec = ArraySpec::new("float")
            .with_dimension(ArrayDimension::fixed("x", 100))
            .with_dimension(ArrayDimension::fixed("y", 200));

        let generator = RustArrayGenerator;

        let type_code = generator.generate_array_type(&spec, "Grid");
        assert!(type_code.contains("ndarray::Array2<f64>"));

        let init_code = generator.generate_array_init(&spec, "grid")?;
        assert!(init_code.contains("Array::zeros"));
        assert!(init_code.contains("[100, 200]"));
        Ok(())
    }
}
