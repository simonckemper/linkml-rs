//! Custom validator support for user-defined validation logic
//!
//! This module provides the infrastructure for creating custom validators
//! that can be registered with the validation engine.

use linkml_core::{
    error::{LinkMLError, Result},
    types::SlotDefinition,
    Value,
};
use std::sync::Arc;

use crate::validator::{
    context::ValidationContext,
    report::ValidationIssue,
};

use super::Validator;

/// Function signature for custom validation logic
pub type ValidationFunction = Arc<
    dyn Fn(&Value, &SlotDefinition, &mut ValidationContext) -> Vec<ValidationIssue>
        + Send
        + Sync,
>;

/// A custom validator that wraps user-provided validation logic
pub struct CustomValidator {
    /// Name of the validator
    name: String,
    /// Description of what this validator does
    description: Option<String>,
    /// The validation function
    validation_fn: ValidationFunction,
    /// Whether this validator applies to all slots or specific ones
    applies_to: AppliesTo,
}

/// Defines which slots a custom validator applies to
#[derive(Clone)]
pub enum AppliesTo {
    /// Applies to all slots
    All,
    /// Applies to slots with specific names
    SlotNames(Vec<String>),
    /// Applies to slots with specific ranges
    SlotRanges(Vec<String>),
    /// Applies based on a predicate function
    Predicate(Arc<dyn Fn(&SlotDefinition) -> bool + Send + Sync>),
}

impl std::fmt::Debug for AppliesTo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppliesTo::All => write!(f, "All"),
            AppliesTo::SlotNames(names) => f.debug_tuple("SlotNames").field(names).finish(),
            AppliesTo::SlotRanges(ranges) => f.debug_tuple("SlotRanges").field(ranges).finish(),
            AppliesTo::Predicate(_) => write!(f, "Predicate(<function>)"),
        }
    }
}

impl CustomValidator {
    /// Create a new custom validator
    pub fn new(
        name: impl Into<String>,
        validation_fn: ValidationFunction,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            validation_fn,
            applies_to: AppliesTo::All,
        }
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set which slots this validator applies to
    pub fn with_applies_to(mut self, applies_to: AppliesTo) -> Self {
        self.applies_to = applies_to;
        self
    }

    /// Check if this validator applies to a given slot
    fn applies_to_slot(&self, slot: &SlotDefinition) -> bool {
        match &self.applies_to {
            AppliesTo::All => true,
            AppliesTo::SlotNames(names) => names.contains(&slot.name),
            AppliesTo::SlotRanges(ranges) => {
                if let Some(range) = &slot.range {
                    ranges.contains(range)
                } else {
                    false
                }
            }
            AppliesTo::Predicate(pred) => pred(slot),
        }
    }
}

impl Validator for CustomValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        if !self.applies_to_slot(slot) {
            return Vec::new();
        }

        (self.validation_fn)(value, slot, context)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Builder for creating custom validators with a fluent API
pub struct CustomValidatorBuilder {
    name: String,
    description: Option<String>,
    applies_to: AppliesTo,
    validation_fn: Option<ValidationFunction>,
}

impl CustomValidatorBuilder {
    /// Create a new builder with a validator name
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            applies_to: AppliesTo::All,
            validation_fn: None,
        }
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Apply to specific slot names
    pub fn for_slots(mut self, slot_names: Vec<String>) -> Self {
        self.applies_to = AppliesTo::SlotNames(slot_names);
        self
    }

    /// Apply to specific slot ranges
    pub fn for_ranges(mut self, ranges: Vec<String>) -> Self {
        self.applies_to = AppliesTo::SlotRanges(ranges);
        self
    }

    /// Apply based on a predicate
    pub fn when<F>(mut self, predicate: F) -> Self
    where
        F: Fn(&SlotDefinition) -> bool + Send + Sync + 'static,
    {
        self.applies_to = AppliesTo::Predicate(Arc::new(predicate));
        self
    }

    /// Set the validation function
    pub fn validate_with<F>(mut self, f: F) -> Self
    where
        F: Fn(&Value, &SlotDefinition, &mut ValidationContext) -> Vec<ValidationIssue>
            + Send
            + Sync
            + 'static,
    {
        self.validation_fn = Some(Arc::new(f));
        self
    }

    /// Build the custom validator
    ///
    /// # Errors
    ///
    /// Returns an error if the validation function is not set
    pub fn build(self) -> Result<CustomValidator> {
        let validation_fn = self.validation_fn
            .ok_or_else(|| LinkMLError::other("Validation function not set"))?;

        Ok(CustomValidator {
            name: self.name,
            description: self.description,
            validation_fn,
            applies_to: self.applies_to,
        })
    }
}

/// Helper functions for creating common custom validators
pub mod helpers {
    use super::*;

    /// Create a validator that checks if a string matches a custom format
    pub fn format_validator(
        name: impl Into<String>,
        format_name: impl Into<String>,
        check_fn: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Result<CustomValidator> {
        let name_str = name.into();
        let format = format_name.into();
        let format_upper = format.to_uppercase();
        let format_clone = format.clone();
        let name_clone = name_str.clone();
        
        CustomValidatorBuilder::new(name_str)
            .description(format!("Validates {} format", format))
            .validate_with(move |value, _slot, context| {
                let mut issues = Vec::new();
                
                match value {
                    Value::String(s) => {
                        if !check_fn(s) {
                            let mut issue = ValidationIssue::error(
                                format!("Value '{}' is not a valid {}", s, format_clone),
                                context.path(),
                                &name_clone,
                            );
                            issue.code = Some(format!("{}_FORMAT_INVALID", format_upper));
                            issues.push(issue);
                        }
                    }
                    Value::Null => {
                        // Null is allowed unless the slot is required
                    }
                    _ => {
                        let mut issue = ValidationIssue::error(
                            format!("Expected string for {} validation", format_clone),
                            context.path(),
                            &name_clone,
                        );
                        issue.code = Some("TYPE_MISMATCH".to_string());
                        issues.push(issue);
                    }
                }
                
                issues
            })
            .build()
    }

    /// Create a validator that ensures a value is within a custom set
    pub fn custom_enum_validator(
        name: impl Into<String>,
        allowed_values: Vec<String>,
    ) -> Result<CustomValidator> {
        let name_str = name.into();
        let name_clone = name_str.clone();
        
        CustomValidatorBuilder::new(name_str)
            .description("Validates against a custom set of allowed values")
            .validate_with(move |value, _slot, context| {
                let mut issues = Vec::new();
                
                match value {
                    Value::String(s) => {
                        if !allowed_values.contains(s) {
                            let mut issue = ValidationIssue::error(
                                format!(
                                    "Value '{}' is not in allowed set: [{}]",
                                    s,
                                    allowed_values.join(", ")
                                ),
                                context.path(),
                                &name_clone,
                            );
                            issue.code = Some("CUSTOM_ENUM_VIOLATION".to_string());
                            issues.push(issue);
                        }
                    }
                    _ => {
                        // Only validate strings
                    }
                }
                
                issues
            })
            .build()
    }

    /// Create a cross-field validator
    pub fn cross_field_validator(
        name: impl Into<String>,
        check_fn: impl Fn(&Value, &SlotDefinition, &ValidationContext) -> Option<String> 
            + Send 
            + Sync 
            + 'static,
    ) -> Result<CustomValidator> {
        let name_str = name.into();
        let name_clone = name_str.clone();
        
        CustomValidatorBuilder::new(name_str)
            .description("Validates relationships between fields")
            .validate_with(move |value, slot, context| {
                let mut issues = Vec::new();
                
                if let Some(error_msg) = check_fn(value, slot, context) {
                    let mut issue = ValidationIssue::error(
                        error_msg,
                        context.path(),
                        &name_clone,
                    );
                    issue.code = Some("CROSS_FIELD_VIOLATION".to_string());
                    issues.push(issue);
                }
                
                issues
            })
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::SchemaDefinition;

    #[test]
    fn test_custom_validator_basic() {
        let validator = CustomValidatorBuilder::new("uppercase_validator")
            .description("Ensures strings are uppercase")
            .validate_with(|value, _slot, context| {
                let mut issues = Vec::new();
                
                if let Value::String(s) = value {
                    if s != &s.to_uppercase() {
                        issues.push(ValidationIssue::error(
                            format!("Value '{}' must be uppercase", s),
                            context.path(),
                            "uppercase_validator",
                        ));
                    }
                }
                
                issues
            })
            .build()
            .expect("should build custom validator");

        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);
        let slot = SlotDefinition::new("code");

        // Valid uppercase
        let value = Value::String("HELLO".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid lowercase
        let value = Value::String("hello".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_custom_validator_for_specific_slots() {
        let validator = CustomValidatorBuilder::new("email_validator")
            .for_slots(vec!["email".to_string(), "contact_email".to_string()])
            .validate_with(|value, _slot, context| {
                let mut issues = Vec::new();
                
                if let Value::String(s) = value {
                    if !s.contains('@') {
                        issues.push(ValidationIssue::error(
                            format!("'{}' is not a valid email", s),
                            context.path(),
                            "email_validator",
                        ));
                    }
                }
                
                issues
            })
            .build()
            .expect("should build custom validator");

        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        // Should validate email slot
        let email_slot = SlotDefinition::new("email");
        let value = Value::String("invalid".to_string());
        let issues = validator.validate(&value, &email_slot, &mut context);
        assert_eq!(issues.len(), 1);

        // Should not validate other slots
        let name_slot = SlotDefinition::new("name");
        let issues = validator.validate(&value, &name_slot, &mut context);
        assert!(issues.is_empty());
    }

    #[test]
    fn test_format_validator_helper() {
        let validator = helpers::format_validator(
            "phone_validator",
            "phone number",
            |s| {
                // Simple phone validation
                s.chars().filter(|c| c.is_numeric()).count() >= 10
            },
        )
        .expect("should build format validator");

        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);
        let slot = SlotDefinition::new("phone");

        // Valid phone
        let value = Value::String("123-456-7890".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid phone
        let value = Value::String("123".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_custom_enum_validator_helper() {
        let validator = helpers::custom_enum_validator(
            "priority_validator",
            vec!["low".to_string(), "medium".to_string(), "high".to_string()],
        )
        .expect("should build custom enum validator");

        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);
        let slot = SlotDefinition::new("priority");

        // Valid value
        let value = Value::String("high".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert!(issues.is_empty());

        // Invalid value
        let value = Value::String("urgent".to_string());
        let issues = validator.validate(&value, &slot, &mut context);
        assert_eq!(issues.len(), 1);
    }

    #[test]
    fn test_predicate_based_validator() {
        let validator = CustomValidatorBuilder::new("range_validator")
            .when(|slot| slot.range.as_deref() == Some("integer"))
            .validate_with(|value, _slot, context| {
                let mut issues = Vec::new();
                
                if let Value::Number(n) = value {
                    if n.as_i64().is_none() {
                        issues.push(ValidationIssue::error(
                            "Value must be an integer",
                            context.path(),
                            "range_validator",
                        ));
                    }
                }
                
                issues
            })
            .build()
            .expect("should build custom validator");

        let schema = Arc::new(SchemaDefinition::default());
        let mut context = ValidationContext::new(schema);

        // Should validate integer slots
        let mut int_slot = SlotDefinition::new("count");
        int_slot.range = Some("integer".to_string());
        
        let value = Value::Number(serde_json::Number::from_f64(3.14).expect("should create number from f64"));
        let issues = validator.validate(&value, &int_slot, &mut context);
        assert_eq!(issues.len(), 1);

        // Should not validate other slots
        let mut str_slot = SlotDefinition::new("name");
        str_slot.range = Some("string".to_string());
        let issues = validator.validate(&value, &str_slot, &mut context);
        assert!(issues.is_empty());
    }
}