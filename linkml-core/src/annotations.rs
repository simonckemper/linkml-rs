//! Annotation support for LinkML schemas
//!
//! Annotations are arbitrary key-value pairs that can be attached to any
//! LinkML schema element. They're used for metadata, tooling hints, and
//! custom extensions.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// An annotation on a schema element
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Annotation {
    /// Simple string annotation
    Simple(String),
    /// Complex annotation with tag and value
    Complex {
        /// The annotation tag (key)
        tag: String,
        /// The annotation value
        value: AnnotationValue,
    },
}

/// Value types for annotations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AnnotationValue {
    /// String value
    String(String),
    /// Boolean value
    Bool(bool),
    /// Numeric value
    Number(serde_json::Number),
    /// Array of values
    Array(Vec<AnnotationValue>),
    /// Object/map of values
    Object(IndexMap<String, AnnotationValue>),
    /// Null value
    Null,
}

/// A collection of annotations
pub type Annotations = IndexMap<String, AnnotationValue>;

/// Trait for elements that can have annotations
pub trait Annotatable {
    /// Get the annotations for this element
    fn annotations(&self) -> Option<&Annotations>;
    
    /// Get a mutable reference to annotations
    fn annotations_mut(&mut self) -> Option<&mut Annotations>;
    
    /// Get a specific annotation value
    fn get_annotation(&self, key: &str) -> Option<&AnnotationValue> {
        self.annotations()?.get(key)
    }
    
    /// Set an annotation
    fn set_annotation(&mut self, key: impl Into<String>, value: AnnotationValue) {
        if let Some(annotations) = self.annotations_mut() {
            annotations.insert(key.into(), value);
        }
    }
    
    /// Remove an annotation
    fn remove_annotation(&mut self, key: &str) -> Option<AnnotationValue> {
        self.annotations_mut()?.shift_remove(key)
    }
    
    /// Check if an annotation exists
    fn has_annotation(&self, key: &str) -> bool {
        self.annotations().map_or(false, |a| a.contains_key(key))
    }
}

impl From<String> for AnnotationValue {
    fn from(s: String) -> Self {
        AnnotationValue::String(s)
    }
}

impl From<&str> for AnnotationValue {
    fn from(s: &str) -> Self {
        AnnotationValue::String(s.to_string())
    }
}

impl From<bool> for AnnotationValue {
    fn from(b: bool) -> Self {
        AnnotationValue::Bool(b)
    }
}

impl From<i32> for AnnotationValue {
    fn from(n: i32) -> Self {
        AnnotationValue::Number(n.into())
    }
}

impl From<f64> for AnnotationValue {
    fn from(n: f64) -> Self {
        if let Some(num) = serde_json::Number::from_f64(n) {
            AnnotationValue::Number(num)
        } else {
            AnnotationValue::Null
        }
    }
}

impl From<Value> for AnnotationValue {
    fn from(value: Value) -> Self {
        match value {
            Value::String(s) => AnnotationValue::String(s),
            Value::Bool(b) => AnnotationValue::Bool(b),
            Value::Number(n) => AnnotationValue::Number(n),
            Value::Array(arr) => AnnotationValue::Array(
                arr.into_iter().map(AnnotationValue::from).collect()
            ),
            Value::Object(obj) => AnnotationValue::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, AnnotationValue::from(v)))
                    .collect()
            ),
            Value::Null => AnnotationValue::Null,
        }
    }
}

impl From<AnnotationValue> for Value {
    fn from(value: AnnotationValue) -> Self {
        match value {
            AnnotationValue::String(s) => Value::String(s),
            AnnotationValue::Bool(b) => Value::Bool(b),
            AnnotationValue::Number(n) => Value::Number(n),
            AnnotationValue::Array(arr) => Value::Array(
                arr.into_iter().map(Value::from).collect()
            ),
            AnnotationValue::Object(obj) => Value::Object(
                obj.into_iter()
                    .map(|(k, v)| (k, Value::from(v)))
                    .collect()
            ),
            AnnotationValue::Null => Value::Null,
        }
    }
}

/// Helper to merge annotations from multiple sources
pub fn merge_annotations(
    base: Option<&Annotations>,
    override_annotations: Option<&Annotations>,
) -> Option<Annotations> {
    match (base, override_annotations) {
        (None, None) => None,
        (Some(b), None) => Some(b.clone()),
        (None, Some(o)) => Some(o.clone()),
        (Some(b), Some(o)) => {
            let mut merged = b.clone();
            for (key, value) in o {
                merged.insert(key.clone(), value.clone());
            }
            Some(merged)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_annotation_value_conversions() {
        // String conversion
        let av: AnnotationValue = "test".into();
        assert_eq!(av, AnnotationValue::String("test".to_string()));
        
        // Bool conversion
        let av: AnnotationValue = true.into();
        assert_eq!(av, AnnotationValue::Bool(true));
        
        // Number conversion
        let av: AnnotationValue = 42.into();
        if let AnnotationValue::Number(n) = av {
            assert_eq!(n.as_i64(), Some(42));
        } else {
            panic!("Expected Number");
        }
    }
    
    #[test]
    fn test_json_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let mut annotations = Annotations::new();
        annotations.insert("author".to_string(), "John Doe".into());
        annotations.insert("version".to_string(), 2.into());
        annotations.insert("deprecated".to_string(), true.into());
        
        let json = serde_json::to_string(&annotations)?;
        let parsed: Annotations = serde_json::from_str(&json)?;
        
        assert_eq!(annotations, parsed);
        Ok(())
    }
    
    #[test]
    fn test_merge_annotations() -> Result<(), Box<dyn std::error::Error>> {
        let mut base = Annotations::new();
        base.insert("key1".to_string(), "value1".into());
        base.insert("key2".to_string(), "value2".into());
        
        let mut override_ann = Annotations::new();
        override_ann.insert("key2".to_string(), "new_value2".into());
        override_ann.insert("key3".to_string(), "value3".into());
        
        let merged = merge_annotations(Some(&base), Some(&override_ann))
            .ok_or("Failed to merge annotations")?;
        
        assert_eq!(
            merged.get("key1").ok_or("key1 not found")?, 
            &AnnotationValue::String("value1".to_string())
        );
        assert_eq!(
            merged.get("key2").ok_or("key2 not found")?, 
            &AnnotationValue::String("new_value2".to_string())
        );
        assert_eq!(
            merged.get("key3").ok_or("key3 not found")?, 
            &AnnotationValue::String("value3".to_string())
        );
        Ok(())
    }
}