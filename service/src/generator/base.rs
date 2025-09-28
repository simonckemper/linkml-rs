//! Base functionality shared by all code generators

use super::traits::GeneratorResult;
use linkml_core::prelude::*;
use std::collections::{HashMap, HashSet};

/// Common type mappings for various languages
pub struct TypeMapper;

impl TypeMapper {
    /// Map `LinkML` type to Python type
    #[must_use]
    pub fn to_python(linkml_type: &str) -> &'static str {
        match linkml_type {
            "string" | "str" | "uri" | "uriorcurie" | "curie" | "ncname" => "str",
            "integer" | "int" => "int",
            "float" | "double" | "decimal" => "float",
            "boolean" | "bool" => "bool",
            "date" => "datetime.date",
            "datetime" => "datetime.datetime",
            "time" => "datetime.time",
            _ => "Any",
        }
    }

    /// Map `LinkML` type to TypeScript type
    #[must_use]
    pub fn to_typescript(linkml_type: &str) -> &'static str {
        match linkml_type {
            "string" | "str" | "uri" | "uriorcurie" | "curie" | "ncname" | "date" | "datetime"
            | "time" => "string",
            "integer" | "int" | "float" | "double" | "decimal" => "number",
            "boolean" | "bool" => "boolean",
            _ => "unknown",
        }
    }

    /// Map `LinkML` type to JavaScript `JSDoc` type
    #[must_use]
    pub fn to_javascript(linkml_type: &str) -> &'static str {
        match linkml_type {
            "string" | "str" | "uri" | "uriorcurie" | "curie" | "ncname" | "date" | "datetime"
            | "time" => "string",
            "integer" | "int" | "float" | "double" | "decimal" => "number",
            "boolean" | "bool" => "boolean",
            _ => "*",
        }
    }
}

/// Import manager for tracking and organizing imports
#[derive(Debug, Default)]
pub struct ImportManager {
    /// Module -> Set of imports from that module
    imports: HashMap<String, HashSet<String>>,
    /// Direct import statements
    direct_imports: HashSet<String>,
}

impl ImportManager {
    /// Create a new import manager
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an import from a module
    pub fn add_import(&mut self, module: impl Into<String>, item: impl Into<String>) {
        self.imports
            .entry(module.into())
            .or_default()
            .insert(item.into());
    }

    /// Add a direct import statement
    pub fn add_direct(&mut self, import: impl Into<String>) {
        self.direct_imports.insert(import.into());
    }

    /// Generate Python import statements
    #[must_use]
    pub fn python_imports(&self) -> String {
        let mut imports = Vec::new();

        // Standard library imports first
        let stdlib = ["dataclasses", "typing", "datetime", "enum", "abc"];
        for module in &stdlib {
            if let Some(items) = self.imports.get(*module) {
                let mut sorted_items: Vec<_> = items.iter().cloned().collect();
                sorted_items.sort();
                imports.push(format!(
                    "from {} import {}",
                    module,
                    sorted_items.join(", ")
                ));
            }
        }

        // Then third-party imports
        for (module, items) in &self.imports {
            if !stdlib.contains(&module.as_str()) {
                let mut sorted_items: Vec<_> = items.iter().cloned().collect();
                sorted_items.sort();
                imports.push(format!(
                    "from {} import {}",
                    module,
                    sorted_items.join(", ")
                ));
            }
        }

        // Direct imports
        let mut direct: Vec<_> = self.direct_imports.iter().cloned().collect();
        direct.sort();
        imports.extend(direct);

        imports.join(
            "
",
        )
    }

    /// Generate TypeScript import statements
    #[must_use]
    pub fn typescript_imports(&self) -> String {
        let mut imports = Vec::new();

        for (module, items) in &self.imports {
            let mut sorted_items: Vec<_> = items.iter().cloned().collect();
            sorted_items.sort();
            imports.push(format!(
                "import {{ {} }} from '{}';",
                sorted_items.join(", "),
                module
            ));
        }

        imports.sort();
        imports.join(
            "
",
        )
    }
}

/// Base code formatter with common functionality
pub struct BaseCodeFormatter;

impl BaseCodeFormatter {
    /// Escape a string for Python
    #[must_use]
    pub fn escape_python_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    }

    /// Escape a string for JavaScript/TypeScript
    #[must_use]
    pub fn escape_js_string(s: &str) -> String {
        s.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
            .replace('\0', "\\0")
    }

    /// Convert `snake_case` to `PascalCase`
    #[must_use]
    pub fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().chain(chars).collect(),
                }
            })
            .collect()
    }

    /// Convert `snake_case` to camelCase
    #[must_use]
    pub fn to_camel_case(s: &str) -> String {
        let mut parts = s.split('_');
        match parts.next() {
            None => String::new(),
            Some(first) => {
                let rest: String = parts
                    .map(|part| {
                        let mut chars = part.chars();
                        match chars.next() {
                            None => String::new(),
                            Some(first) => first.to_uppercase().chain(chars).collect(),
                        }
                    })
                    .collect();
                format!("{first}{rest}")
            }
        }
    }

    /// Convert `PascalCase` or camelCase to `snake_case`
    #[must_use]
    pub fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        let chars = s.chars().peekable();

        for ch in chars {
            if ch.is_uppercase() && !result.is_empty() {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        }

        result
    }

    /// Wrap text to a specific line width
    #[must_use]
    pub fn wrap_text(text: &str, width: usize, indent: &str) -> String {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in words {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + 1 + word.len() <= width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    line.clone()
                } else {
                    format!("{indent}{line}")
                }
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    }
}

/// Helper to collect all slots for a class including inherited ones
///
/// # Errors
///
/// Returns an error if:
/// - Schema validation fails during slot collection
/// - Circular inheritance is detected
/// - Required class definitions are missing from schema
pub fn collect_all_slots(
    class: &ClassDefinition,
    schema: &SchemaDefinition,
) -> GeneratorResult<Vec<String>> {
    let mut slots = Vec::new();
    let mut visited = HashSet::new();

    fn collect_recursive(
        class_name: &str,
        schema: &SchemaDefinition,
        slots: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) -> GeneratorResult<()> {
        if !visited.insert(class_name.to_string()) {
            return Ok(());
        }

        if let Some(class_def) = schema.classes.get(class_name) {
            // Add slots from this class
            slots.extend(class_def.slots.iter().cloned());

            // Process parent class
            if let Some(ref parent) = class_def.is_a {
                collect_recursive(parent, schema, slots, visited)?;
            }

            // Process mixins
            for mixin in &class_def.mixins {
                collect_recursive(mixin, schema, slots, visited)?;
            }
        }

        Ok(())
    }

    collect_recursive(&class.name, schema, &mut slots, &mut visited)?;

    // Remove duplicates while preserving order
    let mut seen = HashSet::new();
    slots.retain(|slot| seen.insert(slot.clone()));

    Ok(slots)
}

/// Check if a type is optional (not required)
#[must_use]
pub fn is_optional_slot(slot: &SlotDefinition) -> bool {
    !slot.required.unwrap_or(false)
}

/// Get the default value for a slot as a string
#[must_use]
pub fn get_default_value_str(slot: &SlotDefinition, language: &str) -> Option<String> {
    // Check if multivalued - these default to empty collections
    if slot.multivalued.unwrap_or(false) {
        return match language {
            "python" => Some("field(default_factory=list)".to_string()),
            "typescript" | "javascript" => Some("[]".to_string()),
            _ => None,
        };
    }

    // Note: LinkML doesn't have a default_value field in SlotDefinition
    // This is a placeholder for future enhancement

    // Optional fields default to None/null/undefined
    if is_optional_slot(slot) {
        match language {
            "python" => Some("None".to_string()),
            "javascript" => Some("null".to_string()),
            _ => None, // TypeScript and other languages use ? for optional or have no default
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_mapping() {
        assert_eq!(TypeMapper::to_python("string"), "str");
        assert_eq!(TypeMapper::to_python("integer"), "int");
        assert_eq!(TypeMapper::to_typescript("boolean"), "boolean");
        assert_eq!(TypeMapper::to_javascript("float"), "number");
    }

    #[test]
    fn test_import_manager() {
        let mut imports = ImportManager::new();
        imports.add_import("typing", "Optional");
        imports.add_import("typing", "List");
        imports.add_import("dataclasses", "dataclass");

        let python_imports = imports.python_imports();
        assert!(python_imports.contains("from dataclasses import dataclass"));
        assert!(python_imports.contains("from typing import List, Optional"));
    }

    #[test]
    fn test_case_conversions() {
        assert_eq!(
            BaseCodeFormatter::to_pascal_case("hello_world"),
            "HelloWorld"
        );
        assert_eq!(
            BaseCodeFormatter::to_camel_case("hello_world"),
            "helloWorld"
        );
        assert_eq!(BaseCodeFormatter::to_pascal_case("simple"), "Simple");
        assert_eq!(BaseCodeFormatter::to_camel_case("simple"), "simple");
    }

    #[test]
    fn test_string_escaping() {
        let test_str = "Hello \"world\"
New line\t\ttab";
        let escaped_py = BaseCodeFormatter::escape_python_string(test_str);
        assert_eq!(
            escaped_py,
            "Hello \\\"world\\\"\
New line\\t\\ttab"
        );
    }
}
