//! Validators for different constraint types

use linkml_core::types::{SchemaDefinition, SlotDefinition};
use serde_json::Value;

use crate::validator::{context::ValidationContext, report::ValidationIssue};

pub mod boolean_constraints;
pub mod conditional_requirements;
pub mod constraint_validators;
pub mod custom_validator;
pub mod expression_validator;
pub mod instance_validator;
pub mod pattern_validator;
pub mod pattern_validator_enhanced;
pub mod range_validator;
pub mod rule_validator;
pub mod string_constraints;
pub mod type_validators;
pub mod unique_key_validator;
pub mod utils;

pub use boolean_constraints::{
    AllOfValidator, AnyOfValidator, ExactlyOneOfValidator, NoneOfValidator,
};
pub use conditional_requirements::ConditionalRequirementValidator;
pub use constraint_validators::{
    MultivaluedValidator, PermissibleValueValidator, RequiredValidator,
};
pub use custom_validator::{
    AppliesTo, CustomValidator, CustomValidatorBuilder, ValidationFunction, helpers,
};
pub use expression_validator::ExpressionValidator;
pub use instance_validator::InstanceValidator;
pub use pattern_validator::PatternValidator;
pub use pattern_validator_enhanced::{EnhancedPatternValidator, PatternMatchResult};
pub use range_validator::RangeValidator;
pub use rule_validator::{RuleValidator, RuleValidation};
pub use string_constraints::{EqualsStringInValidator, StructuredPatternValidator};
pub use type_validators::*;
pub use unique_key_validator::{UniqueKeyValidator, UniqueValueTracker};

/// Trait for all validators
pub trait Validator: Send + Sync {
    /// Validate a value against a slot definition
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue>;

    /// Get the validator name
    fn name(&self) -> &str;
}

/// Registry of validators
pub struct ValidatorRegistry {
    validators: Vec<Box<dyn Validator>>,
    rule_validator: Option<RuleValidator>,
    conditional_requirement_validator: Option<ConditionalRequirementValidator>,
    unique_key_validator: Option<UniqueKeyValidator>,
}

impl ValidatorRegistry {
    /// Create a new validator registry
    ///
    /// # Errors
    ///
    /// Returns an error if any validator fails to initialize.
    pub fn new(schema: &SchemaDefinition) -> Result<Self, linkml_core::error::LinkMLError> {
        let validators: Vec<Box<dyn Validator>> = vec![
            // Type validators
            Box::new(TypeValidator::new()),
            // Constraint validators
            Box::new(RequiredValidator::new()),
            Box::new(MultivaluedValidator::new()),
            Box::new(EnhancedPatternValidator::new()),
            Box::new(RangeValidator::new()),
            Box::new(PermissibleValueValidator::new(schema)?),
            // Boolean constraint validators
            Box::new(AnyOfValidator::new()),
            Box::new(AllOfValidator::new()),
            Box::new(ExactlyOneOfValidator::new()),
            Box::new(NoneOfValidator::new()),
            // Expression validator
            Box::new(ExpressionValidator::new()),
            // String constraint validators
            Box::new(EqualsStringInValidator::new()),
            Box::new(StructuredPatternValidator::new()),
        ];

        // Create rule validator if schema has classes with rules
        let has_rules = schema.classes.values().any(|c| !c.rules.is_empty());
        let rule_validator = if has_rules {
            Some(RuleValidator::new(std::sync::Arc::new(schema.clone())))
        } else {
            None
        };
        
        // Create conditional requirement validator if schema has classes with if_required
        let has_conditional_requirements = schema.classes.values().any(|c| c.if_required.is_some());
        let conditional_requirement_validator = if has_conditional_requirements {
            Some(ConditionalRequirementValidator::new())
        } else {
            None
        };
        
        // Create unique key validator if schema has classes with unique keys or identifier slots
        let has_unique_constraints = schema.classes.values().any(|c| !c.unique_keys.is_empty()) ||
            schema.slots.values().any(|s| s.identifier.unwrap_or(false));
        let unique_key_validator = if has_unique_constraints {
            Some(UniqueKeyValidator::new())
        } else {
            None
        };

        Ok(Self { validators, rule_validator, conditional_requirement_validator, unique_key_validator })
    }

    /// Get all validators that apply to a slot
    #[must_use]
    pub fn get_validators_for_slot(&self, _slot: &SlotDefinition) -> Vec<&dyn Validator> {
        // For now, return all validators and let them decide if they apply
        self.validators
            .iter()
            .map(std::convert::AsRef::as_ref)
            .collect()
    }

    /// Add a custom validator
    pub fn add_validator(&mut self, validator: Box<dyn Validator>) {
        self.validators.push(validator);
    }
    
    /// Get the rule validator if available
    pub fn rule_validator(&self) -> Option<&RuleValidator> {
        self.rule_validator.as_ref()
    }
    
    /// Get the conditional requirement validator if available
    pub fn conditional_requirement_validator(&self) -> Option<&ConditionalRequirementValidator> {
        self.conditional_requirement_validator.as_ref()
    }
    
    /// Get the unique key validator if available
    pub fn unique_key_validator(&self) -> Option<&UniqueKeyValidator> {
        self.unique_key_validator.as_ref()
    }
    
    /// Get a mutable reference to the unique key validator if available
    pub fn unique_key_validator_mut(&mut self) -> Option<&mut UniqueKeyValidator> {
        self.unique_key_validator.as_mut()
    }
}

/// Base implementation for validators
pub struct BaseValidator {
    _name: String,
}

impl BaseValidator {
    /// Create a new base validator
    pub fn new(name: impl Into<String>) -> Self {
        Self { _name: name.into() }
    }
}
