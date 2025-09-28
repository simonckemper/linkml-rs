//! Constraint validators for `LinkML` validation rules

use super::utils::value_type;
use super::{ValidationContext, ValidationIssue, Validator};
use crate::utils::safe_cast::u64_to_f64_lossy;
use linkml_core::annotations::AnnotationValue;
use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

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
        if let Some(range) = &slot.range
            && let Some(enum_values) = self.get_enum_values(range)
        {
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

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// Advanced cross-reference validator for semantic validation
pub struct CrossReferenceValidator {
    name: String,
    /// Cache of resolved references for performance
    reference_cache: parking_lot::RwLock<HashMap<String, HashSet<String>>>,
}

impl Default for CrossReferenceValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CrossReferenceValidator {
    /// Create a new cross-reference validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: "cross_reference_validator".to_string(),
            reference_cache: parking_lot::RwLock::new(HashMap::new()),
        }
    }

    /// Get cached references for a given type
    fn get_cached_references(&self, ref_type: &str) -> Option<HashSet<String>> {
        let cache = self.reference_cache.read();
        cache.get(ref_type).cloned()
    }

    /// Cache references for a given type
    fn cache_references(&self, ref_type: String, references: HashSet<String>) {
        let mut cache = self.reference_cache.write();
        cache.insert(ref_type, references);
    }

    /// Build reference cache from all instances
    fn build_reference_cache(instances: &[Value]) -> HashMap<String, HashSet<String>> {
        let mut type_refs = HashMap::new();

        for instance in instances {
            if let Some(type_name) = instance
                .get("@type")
                .or_else(|| instance.get("type"))
                .and_then(|t| t.as_str())
                && let Some(id) = instance.get("id").and_then(|id| id.as_str())
            {
                type_refs
                    .entry(type_name.to_string())
                    .or_insert_with(HashSet::new)
                    .insert(id.to_string());
            }
        }

        type_refs
    }

    /// Validate cross-references between objects
    fn validate_cross_references(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check if this slot references another class
        if let Some(ref range) = slot.range
            && schema.classes.contains_key(range)
        {
            // This is a reference to another class
            if let Some(ref_id) = value.as_str() {
                // First check cache for performance
                let referenced_exists = if let Some(cached_refs) = self.get_cached_references(range)
                {
                    // Use cached references
                    cached_refs.contains(ref_id)
                } else if let Some(ref all_instances) = context.all_instances {
                    // Build cache if we have instances
                    let type_refs = Self::build_reference_cache(all_instances);

                    // Cache all discovered references for future use
                    for (ref_type, refs) in &type_refs {
                        self.cache_references(ref_type.clone(), refs.clone());
                    }

                    // Check if reference exists in newly built cache
                    type_refs
                        .get(range)
                        .is_some_and(|refs| refs.contains(ref_id))
                } else {
                    // No cache and no instances - can't validate
                    issues.push(ValidationIssue::warning(
                            format!("Cannot validate cross-reference to {range} with id '{ref_id}' - no instance context provided"),
                            context.path(),
                            &self.name,
                        ));
                    return issues;
                };

                if !referenced_exists {
                    issues.push(ValidationIssue::error(
                            format!("Cross-reference validation failed: Referenced {range} with id '{ref_id}' not found"),
                            context.path(),
                            &self.name,
                        ));
                }
            }
        }

        // Check for circular references
        if let Some(ref current_id) = context.current_instance_id
            && let Some(ref_id) = value.as_str()
            && current_id == ref_id
        {
            issues.push(ValidationIssue::error(
                format!("Circular reference detected: object references itself with id '{ref_id}'"),
                context.path(),
                &self.name,
            ));
        }

        issues
    }

    /// Validate semantic constraints
    fn validate_semantic_constraints(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        schema: &SchemaDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Check for semantic consistency based on slot relationships
        if let Some(slot_usage) = context.current_class()
            && let Some(class_def) = schema.classes.get(slot_usage)
        {
            // Check if this slot has any semantic rules defined
            self.validate_slot_dependencies(value, slot, class_def, context, &mut issues);
            self.validate_conditional_constraints(value, slot, class_def, context, &mut issues);
            self.validate_business_rules(value, slot, class_def, schema, context, &mut issues);
            if !class_def.rules.is_empty() {
                for rule in &class_def.rules {
                    // Apply semantic validation rules
                    if let Some(ref preconditions) = rule.preconditions {
                        // This is a simplified semantic validation
                        // In a full implementation, this would use an expression engine
                        if let Some(ref slot_conditions) = preconditions.slot_conditions
                            && slot_conditions.contains_key(&slot.name)
                        {
                            issues.push(ValidationIssue::info(
                                format!(
                                    "Semantic rule '{}' applies to field '{}'",
                                    rule.title.as_ref().unwrap_or(&"unnamed".to_string()),
                                    slot.name
                                ),
                                context.path(),
                                &self.name,
                            ));
                        }
                    }
                }
            }
        }

        issues
    }

    /// Validate slot dependencies (e.g., if slot A has value X, slot B must have value Y)
    fn validate_slot_dependencies(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        class_def: &ClassDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Check slot-level dependencies from annotations
        if let Some(annotations) = &slot.annotations
            && let Some(dependency_info) = annotations.get("depends_on")
        {
            let dependency_str = match dependency_info {
                AnnotationValue::String(s) => s.as_str(),
                AnnotationValue::Bool(b) => {
                    if *b {
                        "true"
                    } else {
                        "false"
                    }
                }
                AnnotationValue::Number(n) => &n.to_string(),
                AnnotationValue::Array(_) => "array",
                AnnotationValue::Object(_) => "object",
                AnnotationValue::Null => "null",
            };
            self.check_dependency_constraint(value, dependency_str, context, issues);
        }

        // CRITICAL: Also check class-level dependency rules
        if let Some(class_annotations) = &class_def.annotations {
            // Check for class-wide slot dependency rules
            if let Some(class_deps) = class_annotations.get("slot_dependencies")
                && let AnnotationValue::Object(deps_map) = class_deps
            {
                // If this slot is mentioned in class dependencies
                if let Some(slot_dep) = deps_map.get(&slot.name)
                    && let AnnotationValue::String(dep_rule) = slot_dep
                {
                    self.check_dependency_constraint(value, dep_rule, context, issues);
                }
            }
        }

        // Check if this slot appears in any class rules
        for rule in &class_def.rules {
            if let Some(ref preconditions) = rule.preconditions
                && let Some(ref slot_conditions) = preconditions.slot_conditions
                && slot_conditions.contains_key(&slot.name)
            {
                // Validate this slot's value against the rule
                if value.is_null() && rule.deactivated != Some(true) {
                    issues.push(ValidationIssue::error(
                        format!(
                            "Class rule '{}' requires slot '{}' to have a value",
                            rule.title.as_ref().unwrap_or(&"unnamed".to_string()),
                            slot.name
                        ),
                        context.path(),
                        &self.name,
                    ));
                }
            }
        }
    }

    /// Validate conditional constraints based on other field values
    fn validate_conditional_constraints(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        class_def: &ClassDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Use the slot parameter for slot-specific validation rules
        if let Some(slot_annotations) = &slot.annotations {
            // Check for conditional validation rules in slot annotations
            if let Some(condition) = slot_annotations.get("validate_if")
                && let AnnotationValue::String(condition_str) = condition
            {
                // Parse and apply conditional validation
                if condition_str.contains("required_when") && value.is_null() {
                    issues.push(ValidationIssue::error(
                        format!(
                            "Slot '{}' is conditionally required based on: {}",
                            slot.name, condition_str
                        ),
                        context.path(),
                        &self.name,
                    ));
                }
            }
        }

        // Use class_def to check class-level conditional constraints
        if let Some(class_annotations) = &class_def.annotations
            && let Some(constraints) = class_annotations.get("conditional_constraints")
            && let AnnotationValue::Object(constraints_map) = constraints
            && let Some(slot_constraint) = constraints_map.get(&slot.name)
            && let AnnotationValue::String(constraint_expr) = slot_constraint
        {
            // Apply class-level conditional constraint for this slot
            if constraint_expr.contains("non_empty") && value.is_null() {
                issues.push(ValidationIssue::error(
                    format!(
                        "Class '{}' requires slot '{}' to be non-empty per constraint: {}",
                        class_def.name, slot.name, constraint_expr
                    ),
                    context.path(),
                    &self.name,
                ));
            }
        }

        // Specific validation based on slot name and value
        match slot.name.as_str() {
            "status" if value.as_str() == Some("published") => {
                // Check if publication_date is set in the current object
                if !context.has_sibling_field("publication_date") {
                    issues.push(ValidationIssue::error(
                        "When status is 'published', publication_date must be set",
                        context.path(),
                        &self.name,
                    ));
                }
            }
            "end_date" if !value.is_null() => {
                // If end_date is set, it should be after start_date
                if let Some(start_date_value) = context.get_sibling_field("start_date")
                    && let (Some(start), Some(end)) = (start_date_value.as_str(), value.as_str())
                    && end < start
                {
                    issues.push(ValidationIssue::error(
                        "End date must be after start date",
                        context.path(),
                        &self.name,
                    ));
                }
            }
            _ => {
                // Check slot's range constraints if defined
                if let Some(ref range) = slot.range {
                    // Validate value matches expected range type
                    match range.as_str() {
                        "date" | "datetime" => {
                            if let Some(val_str) = value.as_str() {
                                // Basic date format validation
                                if !val_str.contains('-') && !val_str.contains('/') {
                                    issues.push(ValidationIssue::warning(
                                        format!(
                                            "Slot '{}' expects date format but got: {}",
                                            slot.name, val_str
                                        ),
                                        context.path(),
                                        &self.name,
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    /// Validate business rules specific to the domain
    fn validate_business_rules(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        class_def: &ClassDefinition,
        schema: &SchemaDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // First validate value against slot-specific business rules
        if let Some(slot_annotations) = &slot.annotations
            && let Some(business_rule) = slot_annotations.get("business_rule")
            && let AnnotationValue::String(rule_expr) = business_rule
        {
            // Apply business rule to the value
            match rule_expr.as_str() {
                "non_negative" => {
                    if let Some(num) = value.as_i64()
                        && num < 0
                    {
                        issues.push(ValidationIssue::error(
                            format!(
                                "Business rule violation: {} must be non-negative",
                                slot.name
                            ),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
                "future_date" => {
                    if let Some(date_str) = value.as_str() {
                        // Simple check - in production would use proper date parsing
                        if date_str < "2025" {
                            issues.push(ValidationIssue::warning(
                                format!("Business rule: {} should be a future date", slot.name),
                                context.path(),
                                &self.name,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        // Check schema-wide business rules that might apply
        if let Some(schema_annotations) = &schema.annotations
            && let Some(global_rules) = schema_annotations.get("business_rules")
            && let AnnotationValue::Array(rules) = global_rules
        {
            for rule in rules {
                if let AnnotationValue::String(rule_str) = rule {
                    // Apply schema-wide business rule
                    if rule_str.contains(&slot.name) || rule_str.contains(&class_def.name) {
                        issues.push(ValidationIssue::info(
                            format!(
                                "Schema business rule '{}' applies to {}.{}",
                                rule_str, class_def.name, slot.name
                            ),
                            context.path(),
                            &self.name,
                        ));

                        // Validate the value against the schema rule
                        if rule_str.contains("required") && value.is_null() {
                            issues.push(ValidationIssue::error(
                                format!(
                                    "Schema business rule violation: {} is required",
                                    slot.name
                                ),
                                context.path(),
                                &self.name,
                            ));
                        }
                    }
                }
            }
        }

        // Class-specific business rules that use the actual value
        match class_def.name.as_str() {
            "Person" => self.validate_person_rules(value, slot, context, issues),
            "Organization" => self.validate_organization_rules(value, slot, context, issues),
            "Event" => self.validate_event_rules(value, slot, context, issues),
            _ => {
                // For other classes, check if they have specific validation annotations
                if let Some(class_annotations) = &class_def.annotations
                    && let Some(validation_type) = class_annotations.get("validation_type")
                    && let AnnotationValue::String(vtype) = validation_type
                {
                    // Apply class-specific validation based on type
                    self.apply_custom_validation(value, slot, vtype.as_str(), context, issues);
                }
            }
        }
    }

    /// Apply custom validation based on validation type
    fn apply_custom_validation(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        validation_type: &str,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        match validation_type {
            "strict" => {
                if value.is_null() && slot.required != Some(false) {
                    issues.push(ValidationIssue::error(
                        format!("Strict validation: {} cannot be null", slot.name),
                        context.path(),
                        &self.name,
                    ));
                }
            }
            "lenient" => {
                // Lenient validation only warns
                if value.is_null() {
                    issues.push(ValidationIssue::warning(
                        format!("Lenient validation: {} is null", slot.name),
                        context.path(),
                        &self.name,
                    ));
                }
            }
            _ => {}
        }
    }

    /// Check a specific dependency constraint
    fn check_dependency_constraint(
        &self,
        value: &Value,
        dependency_info: &str,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Only check dependencies if the current value is not null/empty
        if value.is_null() {
            return;
        }

        // Parse dependency info (format: "field_name:required_value")
        if let Some((field_name, required_value)) = dependency_info.split_once(':')
            && let Some(dependent_value) = context.get_sibling_field(field_name)
            && dependent_value.as_str() != Some(required_value)
        {
            issues.push(ValidationIssue::error(
                        format!(
                            "Field dependency not satisfied: {field_name} must be '{required_value}' when this field has value '{value}'"
                        ),
                        context.path(),
                        &self.name,
                    ));
        }
    }

    /// Validate Person-specific business rules
    fn validate_person_rules(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Use the slot parameter directly instead of getting from context
        match slot.name.as_str() {
            "age" => {
                // Use the value parameter directly
                if let Some(age) = value.as_u64() {
                    if age > 150 {
                        issues.push(ValidationIssue::warning(
                            "Age over 150 seems unrealistic",
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Additional validation using slot metadata
                    if let Some(max_val) = &slot.maximum_value
                        && let Some(max_num) = max_val.as_u64()
                        && age > max_num
                    {
                        issues.push(ValidationIssue::error(
                            format!("Age {age} exceeds maximum allowed value {max_num}"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            "email" => {
                if let Some(email) = value.as_str() {
                    if !email.contains('@') || !email.contains('.') {
                        issues.push(ValidationIssue::error(
                            "Invalid email format",
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Check slot pattern if defined
                    if let Some(pattern) = &slot.pattern {
                        let regex = regex::Regex::new(pattern);
                        if let Ok(re) = regex
                            && !re.is_match(email)
                        {
                            issues.push(ValidationIssue::error(
                                format!("Email does not match required pattern: {pattern}"),
                                context.path(),
                                &self.name,
                            ));
                        }
                    }
                }
            }
            "birth_date" => {
                // Validate birth date using slot metadata
                if let Some(date_str) = value.as_str()
                    && let Some(annotations) = &slot.annotations
                    && let Some(date_format) = annotations.get("date_format")
                    && let AnnotationValue::String(format) = date_format
                {
                    // Validate date matches expected format
                    if !Self::validate_date_format(date_str, format.as_str()) {
                        issues.push(ValidationIssue::error(
                            format!("Birth date '{date_str}' does not match format '{format}'"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    /// Helper to validate date format
    fn validate_date_format(date_str: &str, format: &str) -> bool {
        use chrono::NaiveDate;

        match format {
            "YYYY-MM-DD" => {
                // Parse ISO 8601 date format
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok()
            }
            "MM/DD/YYYY" => {
                // Parse US date format
                NaiveDate::parse_from_str(date_str, "%m/%d/%Y").is_ok()
            }
            "DD/MM/YYYY" => {
                // Parse European date format
                NaiveDate::parse_from_str(date_str, "%d/%m/%Y").is_ok()
            }
            "YYYY-MM-DD HH:MM:SS" => {
                // Parse datetime format
                chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S").is_ok()
            }
            "ISO8601" => {
                // Parse full ISO 8601 datetime
                chrono::DateTime::parse_from_rfc3339(date_str).is_ok()
            }
            _ => {
                // Try common formats as fallback
                NaiveDate::parse_from_str(date_str, "%Y-%m-%d").is_ok()
                    || NaiveDate::parse_from_str(date_str, "%m/%d/%Y").is_ok()
                    || NaiveDate::parse_from_str(date_str, "%d/%m/%Y").is_ok()
            }
        }
    }

    /// Validate Organization-specific business rules
    fn validate_organization_rules(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Use slot parameter directly
        match slot.name.as_str() {
            "employee_count" => {
                if let Some(count) = value.as_u64() {
                    if count == 0 {
                        issues.push(ValidationIssue::warning(
                            "Organization with 0 employees is unusual",
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Use slot metadata for additional validation
                    if let Some(min_val) = &slot.minimum_value
                        && let Some(min_num) = min_val.as_f64()
                        && u64_to_f64_lossy(count) < min_num
                    {
                        issues.push(ValidationIssue::error(
                            format!("Employee count {count} is below minimum {min_num}"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            "founded_year" => {
                if let Some(year) = value.as_u64() {
                    let current_year = 2025;
                    if year > current_year {
                        issues.push(ValidationIssue::error(
                            format!("Founded year {year} cannot be in the future"),
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Check slot annotations for additional rules
                    if let Some(annotations) = &slot.annotations
                        && let Some(min_year) = annotations.get("minimum_year")
                        && let AnnotationValue::Number(min) = min_year
                        && let Some(min_val) = min.as_f64()
                        && u64_to_f64_lossy(year) < min_val
                    {
                        issues.push(ValidationIssue::warning(
                            format!("Founded year {year} seems unusually early"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            "website" => {
                if let Some(url) = value.as_str() {
                    // Validate URL format using slot pattern if available
                    if !url.starts_with("http://") && !url.starts_with("https://") {
                        issues.push(ValidationIssue::warning(
                            "Website URL should start with http:// or https://",
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    /// Validate Event-specific business rules
    fn validate_event_rules(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &ValidationContext,
        issues: &mut Vec<ValidationIssue>,
    ) {
        // Use slot parameter directly
        match slot.name.as_str() {
            "capacity" => {
                if let Some(capacity) = value.as_u64() {
                    if capacity > 1_000_000 {
                        issues.push(ValidationIssue::warning(
                            "Event capacity over 1 million seems unusually large",
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Use slot metadata for validation
                    if let Some(max_val) = &slot.maximum_value
                        && let Some(max_num) = max_val.as_u64()
                        && capacity > max_num
                    {
                        issues.push(ValidationIssue::error(
                            format!("Capacity {capacity} exceeds venue maximum {max_num}"),
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Check minimum capacity
                    if capacity == 0 {
                        issues.push(ValidationIssue::error(
                            "Event capacity cannot be zero",
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            "event_date" => {
                if let Some(date_str) = value.as_str() {
                    // Validate using slot annotations
                    if let Some(annotations) = &slot.annotations
                        && let Some(date_constraint) = annotations.get("date_constraint")
                        && let AnnotationValue::String(constraint) = date_constraint
                        && constraint == "future_only"
                        && date_str < "2025-02-01"
                    {
                        issues.push(ValidationIssue::error(
                            "Event date must be in the future",
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            "ticket_price" => {
                if let Some(price) = value.as_f64() {
                    if price < 0.0 {
                        issues.push(ValidationIssue::error(
                            "Ticket price cannot be negative",
                            context.path(),
                            &self.name,
                        ));
                    }
                    // Check slot range for currency validation
                    if let Some(ref range) = slot.range
                        && range == "currency"
                        && price > 10000.0
                    {
                        issues.push(ValidationIssue::warning(
                            format!("Ticket price ${price} seems unusually high"),
                            context.path(),
                            &self.name,
                        ));
                    }
                }
            }
            _ => {}
        }
    }
}

impl Validator for CrossReferenceValidator {
    fn validate(
        &self,
        value: &Value,
        slot: &SlotDefinition,
        context: &mut ValidationContext,
    ) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // Clone schema reference to avoid borrowing conflicts
        let schema = Arc::clone(&context.schema);

        // Perform cross-reference validation
        issues.extend(self.validate_cross_references(value, slot, &schema, context));

        // Perform semantic validation
        issues.extend(self.validate_semantic_constraints(value, slot, &schema, context));

        issues
    }

    fn name(&self) -> &str {
        &self.name
    }
}
