//! Constraint validators for `LinkML` validation rules

use super::utils::value_type;
use super::{ValidationContext, ValidationIssue, Validator};
use linkml_core::types::{SchemaDefinition, SlotDefinition};
use serde_json::Value;
use std::collections::HashSet;

/// Validator for required fields
pub struct RequiredValidator {
    name: String,
}

impl Default for RequiredValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RequiredValidator {
    /// Create a new required validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "required_validator".to_string(),
        }
    }
}

impl Validator for RequiredValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // This validator only checks if required values are non-null
        // The engine checks if required fields are present
        if slot.required.unwrap_or(false) && value.is_null() {
            issues.push(ValidationIssue::error(
                "Required field cannot be null",
                context.path(),
                &self.name,
            ));
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Validator for multivalued slots
pub struct MultivaluedValidator {
    name: String,
}

impl Default for MultivaluedValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl MultivaluedValidator {
    /// Create a new multivalued validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "multivalued_validator".to_string(),
        }
    }
}

impl Validator for MultivaluedValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Only validate if the slot is marked as multivalued
        if slot.multivalued.unwrap_or(false) {
            // Multivalued slots must be arrays
            if !value.is_array() && !value.is_null() {
                issues.push(ValidationIssue::error(
                    format!(
                        "Multivalued slot must be an array, got {}",
                        value_type(value)
                    ),
                    context.path(),
                    &self.name,
                ));
            }
        } else {
            // Non-multivalued slots must not be arrays
            if value.is_array() {
                issues.push(ValidationIssue::error(
                    "Non-multivalued slot cannot be an array",
                    context.path(),
                    &self.name,
                ));
            }
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Validator for permissible values (enums)
pub struct PermissibleValueValidator {
    name: String,
    schema: SchemaDefinition,
}

impl PermissibleValueValidator {
    /// Create a new permissible value validator
    ///
    /// # Errors
    ///
    /// This function will return an error if the schema is invalid
    pub fn new(schema: &SchemaDefinition) -> Result<Self, linkml_core::error::LinkMLError> {
        Ok(Self {
            name: "permissible_value_validator".to_string(),
            schema: schema.clone(),
        })
    }

    fn get_enum_values(&self, enum_name: &str) -> Option<HashSet<String>> {
        self.schema.enums.get(enum_name).map(|enum_def| {
            enum_def
                .permissible_values
                .iter()
                .map(|pv| match pv {
                    linkml_core::types::PermissibleValue::Simple(s) => s.clone(),
                    linkml_core::types::PermissibleValue::Complex { text, .. } => text.clone(),
                })
                .collect()
        })
    }
}

impl Validator for PermissibleValueValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check if the slot range is an enum
        if let Some(range) = &slot.range {
            if let Some(enum_values) = self.get_enum_values(range) {
                let check_value = |v: &Value, path: &str| -> Option<ValidationIssue> {
                    if let Some(s) = v.as_str() {
                        if enum_values.contains(s) {
                            None
                        } else {
                            Some(ValidationIssue::error(
                                format!(
                                    "Value '{}' is not in permissible values: {:?}",
                                    s,
                                    enum_values.iter().take(5).cloned().collect::<Vec<_>>()
                                ),
                                path,
                                &self.name,
                            ))
                        }
                    } else if !v.is_null() {
                        Some(ValidationIssue::error(
                            "Enum value must be a string",
                            path,
                            &self.name,
                        ))
                    } else {
                        None
                    }
                };

                if slot.multivalued.unwrap_or(false) {
                    if let Some(array) = value.as_array() {
                        for (i, element) in array.iter().enumerate() {
                            if let Some(issue) =
                                check_value(element, &format!("{}[{}]", context.path(), i))
                            {
                                issues.push(issue);
                            }
                        }
                    }
                } else if let Some(issue) = check_value(value, &context.path()) {
                    issues.push(issue);
                }
            }
        }

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}
