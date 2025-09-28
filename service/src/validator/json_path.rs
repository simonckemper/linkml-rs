//! Optimized JSON path navigation for validation
//!
//! This module provides efficient JSON path navigation using a compiled
//! path representation and optimized traversal algorithms.

use linkml_core::error::{LinkMLError, Result as LinkMLResult};
use serde_json::Value;
use std::fmt;

/// Compiled `JSON` path for efficient navigation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPath {
    segments: Vec<PathSegment>,
    /// String representation for display
    string_repr: String,
}

/// A segment in a `JSON` path
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathSegment {
    /// Root marker ($)
    Root,
    /// Property access (.property)
    Property(String),
    /// Array index ([n])
    Index(usize),
    /// Wildcard array access ([*])
    Wildcard,
}

impl JsonPath {
    /// Create a new `JSON` path from a string
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The path doesn't start with '$'
    /// - The path contains invalid characters
    /// - Array indices are not valid numbers
    /// - Array indices are not properly closed with ']'
    ///
    /// # Panics
    ///
    /// This function should not panic under normal circumstances. The `unwrap()` calls
    /// are protected by `peek()` checks that ensure the values exist.
    pub fn parse(path: &str) -> LinkMLResult<Self> {
        let mut segments = Vec::new();
        let mut chars = path.chars().peekable();

        // Expect root marker
        if chars.next() != Some('$') {
            return Err(LinkMLError::parse(format!("Invalid JSON path: {path}")));
        }
        segments.push(PathSegment::Root);

        while let Some(ch) = chars.next() {
            match ch {
                '.' => {
                    // Property access
                    let mut property = String::new();
                    while let Some(&next_ch) = chars.peek() {
                        if next_ch == '.' || next_ch == '[' {
                            break;
                        }
                        property.push(ch);
                    }
                    if property.is_empty() {
                        return Err(LinkMLError::parse(format!(
                            "Empty property in path: {path}"
                        )));
                    }
                    segments.push(PathSegment::Property(property));
                }
                '[' => {
                    // Array access
                    let mut index_str = String::new();
                    let mut found_closing = false;

                    for next_ch in chars.by_ref() {
                        if next_ch == ']' {
                            found_closing = true;
                            break;
                        }
                        index_str.push(next_ch);
                    }

                    if !found_closing {
                        return Err(LinkMLError::parse(format!(
                            "Unclosed array index in path: {path}"
                        )));
                    }

                    if index_str == "*" {
                        segments.push(PathSegment::Wildcard);
                    } else {
                        match index_str.parse::<usize>() {
                            Ok(index) => segments.push(PathSegment::Index(index)),
                            Err(_) => {
                                return Err(LinkMLError::parse(format!(
                                    "Invalid array index: {index_str}"
                                )));
                            }
                        }
                    }
                }
                _ => {
                    return Err(LinkMLError::parse(format!(
                        "Unexpected character '{ch}' in path: {path}"
                    )));
                }
            }
        }

        Ok(Self {
            segments,
            string_repr: path.to_string(),
        })
    }

    /// Create a root path
    #[must_use]
    pub fn root() -> Self {
        Self {
            segments: vec![PathSegment::Root],
            string_repr: "$".to_string(),
        }
    }

    /// Append a property access
    pub fn property(&mut self, name: &str) -> &mut Self {
        self.segments.push(PathSegment::Property(name.to_string()));
        self.update_string_repr();
        self
    }

    /// Append an array index
    pub fn index(&mut self, idx: usize) -> &mut Self {
        self.segments.push(PathSegment::Index(idx));
        self.update_string_repr();
        self
    }

    /// Append a wildcard
    pub fn wildcard(&mut self) -> &mut Self {
        self.segments.push(PathSegment::Wildcard);
        self.update_string_repr();
        self
    }

    /// Navigate to a value in `JSON` data
    #[must_use]
    pub fn navigate<'a>(&self, value: &'a Value) -> Vec<(&'a Value, String)> {
        let mut results = Vec::new();
        self.navigate_recursive(value, 0, "$", &mut results);
        results
    }

    /// Recursive navigation helper
    fn navigate_recursive<'a>(
        &self,
        value: &'a Value,
        segment_idx: usize,
        current_path: &str,
        results: &mut Vec<(&'a Value, String)>,
    ) {
        if segment_idx >= self.segments.len() {
            results.push((value, current_path.to_string()));
            return;
        }

        match &self.segments[segment_idx] {
            PathSegment::Root => {
                self.navigate_recursive(value, segment_idx + 1, current_path, results);
            }
            PathSegment::Property(name) => {
                if let Some(obj) = value.as_object()
                    && let Some(field_value) = obj.get(name)
                {
                    let new_path = format!("{current_path}.{name}");
                    self.navigate_recursive(field_value, segment_idx + 1, &new_path, results);
                }
            }
            PathSegment::Index(idx) => {
                if let Some(arr) = value.as_array()
                    && let Some(elem) = arr.get(*idx)
                {
                    let new_path = format!("{current_path}[{idx}]");
                    self.navigate_recursive(elem, segment_idx + 1, &new_path, results);
                }
            }
            PathSegment::Wildcard => {
                if let Some(arr) = value.as_array() {
                    for (i, elem) in arr.iter().enumerate() {
                        let new_path = format!("{current_path}[{i}]");
                        self.navigate_recursive(elem, segment_idx + 1, &new_path, results);
                    }
                }
            }
        }
    }

    /// Update string representation after modification
    fn update_string_repr(&mut self) {
        let mut repr = String::new();

        for (i, segment) in self.segments.iter().enumerate() {
            match segment {
                PathSegment::Root => {
                    if i == 0 {
                        repr.push('$');
                    }
                }
                PathSegment::Property(name) => {
                    if i > 0 {
                        repr.push('.');
                    }
                    repr.push_str(name);
                }
                PathSegment::Index(idx) => {
                    repr.push('[');
                    repr.push_str(&idx.to_string());
                    repr.push(']');
                }
                PathSegment::Wildcard => {
                    repr.push_str("[*]");
                }
            }
        }

        self.string_repr = repr;
    }

    /// Check if this path is a prefix of another
    #[must_use]
    pub fn is_prefix_of(&self, other: &Self) -> bool {
        if self.segments.len() > other.segments.len() {
            return false;
        }

        for (i, segment) in self.segments.iter().enumerate() {
            if segment != &other.segments[i] {
                return false;
            }
        }

        true
    }

    /// Get the parent path
    #[must_use]
    pub fn parent(&self) -> Option<Self> {
        if self.segments.len() <= 1 {
            return None;
        }

        let mut parent = self.clone();
        parent.segments.pop();
        parent.update_string_repr();
        Some(parent)
    }

    /// Get the depth of the path
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.segments.len().saturating_sub(1)
    }
}

impl fmt::Display for JsonPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.string_repr)
    }
}

/// Optimized `JSON` path navigator with caching
pub struct JsonNavigator {
    /// Cache of compiled paths
    path_cache: std::collections::HashMap<String, JsonPath>,
}

impl JsonNavigator {
    /// Create a new navigator
    #[must_use]
    pub fn new() -> Self {
        Self {
            path_cache: std::collections::HashMap::new(),
        }
    }

    /// Navigate to a value using a path string
    ///
    /// # Errors
    ///
    /// Returns an error if the path string cannot be parsed.
    ///
    /// # Panics
    ///
    /// This function should not panic under normal circumstances.
    pub fn navigate<'a>(
        &mut self,
        value: &'a Value,
        path: &str,
    ) -> LinkMLResult<Vec<(&'a Value, String)>> {
        let json_path = if let Some(cached) = self.path_cache.get(path) {
            cached.clone()
        } else {
            let parsed = JsonPath::parse(path)?;
            self.path_cache.insert(path.to_string(), parsed.clone());
            parsed
        };

        Ok(json_path.navigate(value))
    }

    /// Clear the path cache
    pub fn clear_cache(&mut self) {
        self.path_cache.clear();
    }
}

impl Default for JsonNavigator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_path_parsing() {
        assert!(JsonPath::parse("$").is_ok());
        assert!(JsonPath::parse("$.name").is_ok());
        assert!(JsonPath::parse("$.items[0]").is_ok());
        assert!(JsonPath::parse("$.items[*]").is_ok());
        assert!(JsonPath::parse("$.items[0].name").is_ok());

        assert!(JsonPath::parse("").is_err());
        assert!(JsonPath::parse("name").is_err());
        assert!(JsonPath::parse("$[").is_err());
        assert!(JsonPath::parse("$.").is_err());
    }

    #[test]
    fn test_navigation() -> Result<(), Box<dyn std::error::Error>> {
        let data = json!({
            "name": "John",
            "age": 30,
            "items": [
                {"id": 1, "name": "Item 1"},
                {"id": 2, "name": "Item 2"}
            ]
        });

        let path = JsonPath::parse("$.name").expect("should parse valid path: {}");
        let results = path.navigate(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, &json!("John"));

        let path =
            JsonPath::parse("$.items[0].name").expect("should parse valid path with index: {}");
        let results = path.navigate(&data);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, &json!("Item 1"));

        let path =
            JsonPath::parse("$.items[*].name").expect("should parse valid path with wildcard: {}");
        let results = path.navigate(&data);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, &json!("Item 1"));
        assert_eq!(results[1].0, &json!("Item 2"));
        Ok(())
    }

    #[test]
    fn test_path_builder() {
        let mut path = JsonPath::root();
        path.property("items").index(0).property("name");
        assert_eq!(path.to_string(), "$.items[0].name");

        let mut path = JsonPath::root();
        path.property("items").wildcard().property("id");
        assert_eq!(path.to_string(), "$.items[*].id");
    }

    #[test]
    fn test_navigator_caching() -> Result<(), Box<dyn std::error::Error>> {
        let data = json!({"name": "test"});
        let mut navigator = JsonNavigator::new();

        // First call parses the path
        let result1 = navigator
            .navigate(&data, "$.name")
            .expect("should navigate to name: {}");
        assert_eq!(result1.len(), 1);

        // Second call uses cached path
        let result2 = navigator.navigate(&data, "$.name").expect("Error: {}");
        assert_eq!(result2.len(), 1);

        // Verify cache contains the path
        assert_eq!(navigator.path_cache.len(), 1);
        Ok(())
    }
}
