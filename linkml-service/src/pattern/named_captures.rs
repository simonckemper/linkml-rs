//! Named captures support for pattern matching
//!
//! This module provides advanced named capture functionality for pattern matching,
//! including type conversion, validation, and extraction utilities.

use crate::utils::safe_cast::i64_to_f64_lossy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;
use thiserror::Error;

/// Error type for named capture operations
#[derive(Debug, Error)]
pub enum CaptureError {
    /// Capture not found
    #[error("Capture not found: {0}")]
    CaptureNotFound(String),

    /// Type conversion failed
    #[error("Failed to convert capture '{name}' to {target_type}: {error}")]
    ConversionError {
        /// Name of the capture that failed conversion
        name: String,
        /// Target type for conversion
        target_type: String,
        /// Error message from conversion attempt
        error: String,
    },

    /// Validation failed
    #[error("Validation failed for capture '{name}': {reason}")]
    ValidationError {
        /// Name of the capture that failed validation
        name: String,
        /// Reason for validation failure
        reason: String,
    },

    /// Pattern error
    #[error("Pattern error: {0}")]
    PatternError(String),
}

/// Result type for capture operations
pub type CaptureResult<T> = Result<T, CaptureError>;

/// A named capture definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureDefinition {
    /// Name of the capture
    pub name: String,

    /// Expected type
    pub capture_type: CaptureType,

    /// Whether the capture is required
    pub required: bool,

    /// Default value if not captured
    pub default: Option<String>,

    /// Validation rules
    pub validators: Vec<CaptureValidator>,
}

/// Type of a capture
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureType {
    /// String value
    String,
    /// Integer value
    Integer,
    /// Float value
    Float,
    /// Boolean value
    Boolean,
    /// Enumeration value
    Enum(Vec<String>),
    /// Custom type with converter
    Custom(String),
}

/// Validation rule for captures
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureValidator {
    /// Minimum length
    MinLength(usize),
    /// Maximum length
    MaxLength(usize),
    /// Minimum value (numeric)
    MinValue(f64),
    /// Maximum value (numeric)
    MaxValue(f64),
    /// Pattern match
    Pattern(String),
    /// Custom validator name
    Custom(String),
}

/// Type alias for converter functions
type ConverterFn = Box<dyn Fn(&str) -> CaptureResult<CaptureValue>>;

/// Type alias for validator functions
type ValidatorFn = Box<dyn Fn(&str) -> CaptureResult<()>>;

/// Named capture extractor
pub struct CaptureExtractor {
    /// Capture definitions
    definitions: HashMap<String, CaptureDefinition>,

    /// Custom converters
    converters: HashMap<String, ConverterFn>,

    /// Custom validators
    validators: HashMap<String, ValidatorFn>,
}

/// Extracted capture value
#[derive(Debug, Clone, PartialEq)]
pub enum CaptureValue {
    /// String value
    String(String),
    /// Integer value
    Integer(i64),
    /// Float value
    Float(f64),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
}

impl CaptureExtractor {
    /// Create a new capture extractor
    #[must_use]
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
            converters: HashMap::new(),
            validators: HashMap::new(),
        }
    }

    /// Add a capture definition
    pub fn add_definition(&mut self, definition: CaptureDefinition) {
        self.definitions.insert(definition.name.clone(), definition);
    }

    /// Add a custom converter
    pub fn add_converter<F>(&mut self, name: &str, converter: F)
    where
        F: Fn(&str) -> CaptureResult<CaptureValue> + 'static,
    {
        self.converters
            .insert(name.to_string(), Box::new(converter));
    }

    /// Add a custom validator
    pub fn add_validator<F>(&mut self, name: &str, validator: F)
    where
        F: Fn(&str) -> CaptureResult<()> + 'static,
    {
        self.validators
            .insert(name.to_string(), Box::new(validator));
    }

    /// Extract captures from regex captures
    ///
    /// # Errors
    /// Returns `CaptureError::CaptureNotFound` if a required capture is missing, or validation/processing errors from capture definitions
    pub fn extract(
        &self,
        captures: &regex::Captures,
    ) -> CaptureResult<HashMap<String, CaptureValue>> {
        let mut result = HashMap::new();

        for (name, definition) in &self.definitions {
            let value = match captures.name(name) {
                Some(m) => {
                    let text = m.as_str();
                    self.process_capture(text, definition)?
                }
                None => {
                    if definition.required {
                        return Err(CaptureError::CaptureNotFound(name.clone()));
                    } else if let Some(default) = &definition.default {
                        self.process_capture(default, definition)?
                    } else {
                        CaptureValue::Null
                    }
                }
            };

            result.insert(name.clone(), value);
        }

        Ok(result)
    }

    /// Process a single capture
    fn process_capture(
        &self,
        text: &str,
        definition: &CaptureDefinition,
    ) -> CaptureResult<CaptureValue> {
        // Validate first
        self.validate_capture(text, definition)?;

        // Convert to appropriate type
        match &definition.capture_type {
            CaptureType::String => Ok(CaptureValue::String(text.to_string())),

            CaptureType::Integer => i64::from_str(text).map(CaptureValue::Integer).map_err(|e| {
                CaptureError::ConversionError {
                    name: definition.name.clone(),
                    target_type: "integer".to_string(),
                    error: e.to_string(),
                }
            }),

            CaptureType::Float => f64::from_str(text).map(CaptureValue::Float).map_err(|e| {
                CaptureError::ConversionError {
                    name: definition.name.clone(),
                    target_type: "float".to_string(),
                    error: e.to_string(),
                }
            }),

            CaptureType::Boolean => match text.to_lowercase().as_str() {
                "true" | "yes" | "1" => Ok(CaptureValue::Boolean(true)),
                "false" | "no" | "0" => Ok(CaptureValue::Boolean(false)),
                _ => Err(CaptureError::ConversionError {
                    name: definition.name.clone(),
                    target_type: "boolean".to_string(),
                    error: format!("Invalid boolean value: {text}"),
                }),
            },

            CaptureType::Enum(values) => {
                if values.contains(&text.to_string()) {
                    Ok(CaptureValue::String(text.to_string()))
                } else {
                    Err(CaptureError::ValidationError {
                        name: definition.name.clone(),
                        reason: format!("Value '{text}' not in allowed values: {values:?}"),
                    })
                }
            }

            CaptureType::Custom(converter_name) => {
                if let Some(converter) = self.converters.get(converter_name) {
                    converter(text)
                } else {
                    Err(CaptureError::ConversionError {
                        name: definition.name.clone(),
                        target_type: converter_name.clone(),
                        error: "Converter not found".to_string(),
                    })
                }
            }
        }
    }

    /// Validate a capture against its rules
    fn validate_capture(&self, text: &str, definition: &CaptureDefinition) -> CaptureResult<()> {
        for validator in &definition.validators {
            match validator {
                CaptureValidator::MinLength(min) => {
                    if text.len() < *min {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Length {} is less than minimum {}", text.len(), min),
                        });
                    }
                }

                CaptureValidator::MaxLength(max) => {
                    if text.len() > *max {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Length {} exceeds maximum {}", text.len(), max),
                        });
                    }
                }

                CaptureValidator::MinValue(min) => {
                    if let Ok(value) = f64::from_str(text)
                        && value < *min
                    {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Value {value} is less than minimum {min}"),
                        });
                    }
                }

                CaptureValidator::MaxValue(max) => {
                    if let Ok(value) = f64::from_str(text)
                        && value > *max
                    {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Value {value} exceeds maximum {max}"),
                        });
                    }
                }

                CaptureValidator::Pattern(pattern) => {
                    let regex = Regex::new(pattern)
                        .map_err(|_| CaptureError::PatternError(pattern.clone()))?;

                    if !regex.is_match(text) {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Value '{text}' doesn't match pattern '{pattern}'"),
                        });
                    }
                }

                CaptureValidator::Custom(validator_name) => {
                    if let Some(validator) = self.validators.get(validator_name) {
                        validator(text)?;
                    } else {
                        return Err(CaptureError::ValidationError {
                            name: definition.name.clone(),
                            reason: format!("Custom validator '{validator_name}' not found"),
                        });
                    }
                }
            }
        }

        Ok(())
    }
}

/// Builder for capture definitions
pub struct CaptureDefinitionBuilder {
    name: String,
    capture_type: CaptureType,
    required: bool,
    default: Option<String>,
    validators: Vec<CaptureValidator>,
}

impl CaptureDefinitionBuilder {
    /// Create a new builder
    pub fn new(name: impl Into<String>, capture_type: CaptureType) -> Self {
        Self {
            name: name.into(),
            capture_type,
            required: true,
            default: None,
            validators: Vec::new(),
        }
    }

    /// Set whether the capture is required
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Set default value
    #[must_use]
    pub fn default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Add a validator
    #[must_use]
    pub fn validator(mut self, validator: CaptureValidator) -> Self {
        self.validators.push(validator);
        self
    }

    /// Build the definition
    #[must_use]
    pub fn build(self) -> CaptureDefinition {
        CaptureDefinition {
            name: self.name,
            capture_type: self.capture_type,
            required: self.required,
            default: self.default,
            validators: self.validators,
        }
    }
}

impl CaptureValue {
    /// Try to get as string
    #[must_use]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            CaptureValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Try to get as integer
    #[must_use]
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            CaptureValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Try to get as float
    #[must_use]
    pub fn as_float(&self) -> Option<f64> {
        match self {
            CaptureValue::Float(f) => Some(*f),
            CaptureValue::Integer(i) => Some(i64_to_f64_lossy(*i)),
            _ => None,
        }
    }

    /// Try to get as boolean
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            CaptureValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Check if null
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, CaptureValue::Null)
    }
}

impl Default for CaptureExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_extraction() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let pattern = Regex::new(r"(?P<name>\w+) v(?P<version>\d+\.\d+)")
            .expect("pattern should compile: {}");

        let mut extractor = CaptureExtractor::new();

        extractor.add_definition(
            CaptureDefinitionBuilder::new("name", CaptureType::String)
                .validator(CaptureValidator::MinLength(2))
                .build(),
        );

        extractor.add_definition(
            CaptureDefinitionBuilder::new("version", CaptureType::String)
                .validator(CaptureValidator::Pattern(r"^\d+\.\d+$".to_string()))
                .build(),
        );

        let captures = pattern.captures("project v1.0").expect("should match: {}");

        let extracted = extractor
            .extract(&captures)
            .expect("extraction should succeed: {}");

        assert_eq!(
            extracted.get("name"),
            Some(&CaptureValue::String("project".to_string()))
        );
        assert_eq!(
            extracted.get("version"),
            Some(&CaptureValue::String("1.0".to_string()))
        );
        Ok(())
    }

    #[test]
    fn test_type_conversion() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let pattern = Regex::new(r"(?P<count>\d+) (?P<enabled>true|false)")
            .expect("pattern should compile: {}");

        let mut extractor = CaptureExtractor::new();

        extractor.add_definition(
            CaptureDefinitionBuilder::new("count", CaptureType::Integer)
                .validator(CaptureValidator::MinValue(0.0))
                .build(),
        );

        extractor
            .add_definition(CaptureDefinitionBuilder::new("enabled", CaptureType::Boolean).build());

        let captures = pattern.captures("42 true").expect("should match: {}");

        let extracted = extractor
            .extract(&captures)
            .expect("extraction should succeed: {}");

        assert_eq!(extracted.get("count"), Some(&CaptureValue::Integer(42)));
        assert_eq!(extracted.get("enabled"), Some(&CaptureValue::Boolean(true)));
        Ok(())
    }

    #[test]
    fn test_enum_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let pattern = Regex::new(r"(?P<level>\w+)").expect("pattern should compile: {}");

        let mut extractor = CaptureExtractor::new();

        extractor.add_definition(
            CaptureDefinitionBuilder::new(
                "level",
                CaptureType::Enum(vec![
                    "debug".to_string(),
                    "info".to_string(),
                    "warn".to_string(),
                    "error".to_string(),
                ]),
            )
            .build(),
        );

        let captures = pattern.captures("info").expect("should match: {}");

        let extracted = extractor
            .extract(&captures)
            .expect("extraction should succeed: {}");

        assert_eq!(
            extracted.get("level"),
            Some(&CaptureValue::String("info".to_string()))
        );

        // Test invalid enum value
        let invalid_captures = pattern.captures("critical").expect("should match: {}");

        assert!(extractor.extract(&invalid_captures).is_err());
        Ok(())
    }

    #[test]
    fn test_optional_with_default() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let pattern = Regex::new(r"name=(?P<name>\w+)(?:\s+port=(?P<port>\d+))?")
            .expect("pattern should compile: {}");

        let mut extractor = CaptureExtractor::new();

        extractor
            .add_definition(CaptureDefinitionBuilder::new("name", CaptureType::String).build());

        extractor.add_definition(
            CaptureDefinitionBuilder::new("port", CaptureType::Integer)
                .required(false)
                .default("8080")
                .build(),
        );

        let captures = pattern.captures("name=server").expect("should match: {}");

        let extracted = extractor
            .extract(&captures)
            .expect("extraction should succeed: {}");

        assert_eq!(
            extracted.get("name"),
            Some(&CaptureValue::String("server".to_string()))
        );
        assert_eq!(extracted.get("port"), Some(&CaptureValue::Integer(8080)));
        Ok(())
    }
}
