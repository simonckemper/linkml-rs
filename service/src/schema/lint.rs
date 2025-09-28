//! Schema linting functionality for `LinkML`
//!
//! This module provides tools to check schema quality and compliance.

use linkml_core::prelude::*;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Lint severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Error - must be fixed
    Error,
    /// Warning - should be fixed
    Warning,
    /// Info - suggestion for improvement
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Info => write!(f, "info"),
        }
    }
}

/// Lint issue found in schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintIssue {
    /// Rule that triggered the issue
    pub rule: String,

    /// Severity of the issue
    pub severity: Severity,

    /// Issue message
    pub message: String,

    /// Element type (class, slot, etc.)
    pub element_type: Option<String>,

    /// Element name
    pub element_name: Option<String>,

    /// Line number in source
    pub line: Option<usize>,

    /// Column number in source
    pub column: Option<usize>,

    /// Suggestion for fixing
    pub suggestion: Option<String>,

    /// Whether this can be auto-fixed
    pub fixable: bool,
}

/// Lint rule definition
pub trait LintRule: Send + Sync {
    /// Rule name
    fn name(&self) -> &str;

    /// Rule description
    fn description(&self) -> &str;

    /// Default severity
    fn severity(&self) -> Severity;

    /// Check the schema
    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue>;

    /// Fix issues if possible
    ///
    /// # Errors
    /// Returns error if fixing issues fails or encounters invalid schema state.
    fn fix(&self, schema: &mut SchemaDefinition, issues: &[LintIssue]) -> Result<usize>;
}

/// Options for linting
pub struct LintOptions {
    /// Rules to apply
    pub rules: Vec<Box<dyn LintRule>>,

    /// Rule configuration
    pub rule_config: HashMap<String, HashMap<String, serde_json::Value>>,

    /// Ignore patterns
    pub ignore_patterns: Vec<Regex>,
}

impl Default for LintOptions {
    fn default() -> Self {
        Self {
            rules: vec![
                Box::new(NamingConventionRule),
                Box::new(MissingDocumentationRule),
                Box::new(UnusedDefinitionsRule),
                Box::new(SlotConsistencyRule),
                Box::new(TypeSafetyRule),
                Box::new(SchemaMetadataRule),
            ],
            rule_config: HashMap::new(),
            ignore_patterns: Vec::new(),
        }
    }
}

impl LintOptions {
    /// Apply configuration from a map
    pub fn apply_config(&mut self, config: HashMap<String, serde_json::Value>) {
        // Store rule configurations
        for (key, value) in config {
            if key.starts_with("rule.") {
                let rule_name = key.strip_prefix("rule.").expect("just checked starts_with");
                if let Some(rule_config) = value.as_object() {
                    let mut config_map = HashMap::new();
                    for (k, v) in rule_config {
                        config_map.insert(k.clone(), v.clone());
                    }
                    self.rule_config.insert(rule_name.to_string(), config_map);
                }
            }
        }
    }

    /// Filter rules by name
    pub fn filter_rules(&mut self, rule_names: &[String]) {
        self.rules
            .retain(|rule| rule_names.contains(&rule.name().to_string()));
    }
}

/// Result of linting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintResult {
    /// Issues found
    pub issues: Vec<LintIssue>,

    /// Issues that can be auto-fixed
    pub fixable_issues: Vec<LintIssue>,
}

impl LintResult {
    /// Count errors
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }

    /// Count warnings
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }

    /// Count info messages
    #[must_use]
    pub fn info_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Info)
            .count()
    }

    /// Convert to `JUnit` `XML` format
    #[must_use]
    pub fn to_junit_xml(&self, test_name: &str) -> String {
        let mut xml = String::new();

        xml.push_str(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
",
        );
        xml.push_str("<testsuite name=\"LinkML Lint\" tests=\"1\"");
        write!(xml, " errors=\"{}\"", self.error_count())
            .expect("write! to String should never fail");
        write!(xml, " failures=\"{}\"", self.warning_count())
            .expect("write! to String should never fail");
        xml.push_str(
            ">
",
        );

        writeln!(xml, "  <testcase name=\"{test_name}\">")
            .expect("writeln! to String should never fail");

        for issue in &self.issues {
            match issue.severity {
                Severity::Error => {
                    writeln!(xml, "    <error message=\"{}\"/>", issue.message)
                        .expect("writeln! to String should never fail");
                }
                Severity::Warning => {
                    writeln!(xml, "    <failure message=\"{}\"/>", issue.message)
                        .expect("writeln! to String should never fail");
                }
                Severity::Info => {
                    // Info messages are not included in JUnit
                }
            }
        }

        xml.push_str(
            "  </testcase>
",
        );
        xml.push_str(
            "</testsuite>
",
        );

        xml
    }
}

/// Schema linter
pub struct SchemaLinter {
    options: LintOptions,
}

impl SchemaLinter {
    /// Create new linter
    #[must_use]
    pub fn new(options: LintOptions) -> Self {
        Self { options }
    }

    /// Lint a schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn lint(&self, schema: &SchemaDefinition) -> Result<LintResult> {
        let mut all_issues = Vec::new();
        let mut fixable_issues = Vec::new();

        // Run each rule
        for rule in &self.options.rules {
            let issues = rule.check(schema);

            for issue in issues {
                if issue.fixable {
                    fixable_issues.push(issue.clone());
                }
                all_issues.push(issue);
            }
        }

        Ok(LintResult {
            issues: all_issues,
            fixable_issues,
        })
    }

    /// Fix issues in schema
    /// Returns an error if the operation fails
    ///
    /// # Errors
    ///
    pub fn fix(&self, schema: &mut SchemaDefinition, result: &mut LintResult) -> Result<usize> {
        let mut total_fixed = 0;

        // Group issues by rule
        let mut issues_by_rule: HashMap<String, Vec<&LintIssue>> = HashMap::new();
        for issue in &result.fixable_issues {
            issues_by_rule
                .entry(issue.rule.clone())
                .or_default()
                .push(issue);
        }

        // Apply fixes for each rule
        for rule in &self.options.rules {
            if let Some(rule_issues) = issues_by_rule.get(rule.name()) {
                // Convert &Vec<&LintIssue> to Vec<LintIssue>
                let issues: Vec<LintIssue> =
                    rule_issues.iter().map(|&issue| issue.clone()).collect();
                let fixed = rule.fix(schema, &issues)?;
                total_fixed += fixed;
            }
        }

        Ok(total_fixed)
    }
}

// Built-in lint rules

/// Naming convention rule
#[derive(Default)]
struct NamingConventionRule;

impl LintRule for NamingConventionRule {
    fn name(&self) -> &'static str {
        "naming-convention"
    }

    fn description(&self) -> &'static str {
        "Check naming conventions for classes, slots, and types"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Check class names (should be PascalCase)
        let pascal_case = Regex::new(r"^[A-Z][a-zA-Z0-9]*$").expect("valid regex pattern");
        for class_name in schema.classes.keys() {
            if !pascal_case.is_match(class_name) {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Class name '{class_name}' should be in PascalCase"),
                    element_type: Some("class".to_string()),
                    element_name: Some(class_name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some(format!("Rename to '{}'", to_pascal_case(class_name))),
                    fixable: false,
                });
            }
        }

        // Check slot names (should be snake_case)
        let snake_case = Regex::new(r"^[a-z][a-z0-9_]*$").expect("valid regex pattern");
        for slot_name in schema.slots.keys() {
            if !snake_case.is_match(slot_name) {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Slot name '{slot_name}' should be in snake_case"),
                    element_type: Some("slot".to_string()),
                    element_name: Some(slot_name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some(format!("Rename to '{}'", to_snake_case(slot_name))),
                    fixable: false,
                });
            }
        }

        issues
    }

    fn fix(&self, _schema: &mut SchemaDefinition, _issues: &[LintIssue]) -> Result<usize> {
        // Naming changes require manual intervention
        Ok(0)
    }
}

/// Missing documentation rule
#[derive(Default)]
struct MissingDocumentationRule;

impl LintRule for MissingDocumentationRule {
    fn name(&self) -> &'static str {
        "missing-documentation"
    }

    fn description(&self) -> &'static str {
        "Check for missing descriptions on classes, slots, and types"
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Check schema description
        if schema.description.is_none() {
            issues.push(LintIssue {
                rule: self.name().to_string(),
                severity: self.severity(),
                message: "Schema has no description".to_string(),
                element_type: Some("schema".to_string()),
                element_name: Some(schema.name.clone()),
                line: None,
                column: None,
                suggestion: Some("Add a description field to the schema".to_string()),
                fixable: false,
            });
        }

        // Check class descriptions
        for (name, class) in &schema.classes {
            if class.description.is_none() {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Class '{name}' has no description"),
                    element_type: Some("class".to_string()),
                    element_name: Some(name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some("Add a description to the class".to_string()),
                    fixable: false,
                });
            }
        }

        // Check slot descriptions
        for (name, slot) in &schema.slots {
            if slot.description.is_none() {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Slot '{name}' has no description"),
                    element_type: Some("slot".to_string()),
                    element_name: Some(name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some("Add a description to the slot".to_string()),
                    fixable: false,
                });
            }
        }

        issues
    }

    fn fix(&self, _schema: &mut SchemaDefinition, _issues: &[LintIssue]) -> Result<usize> {
        // Documentation must be added manually
        Ok(0)
    }
}

/// Unused definitions rule
#[derive(Default)]
struct UnusedDefinitionsRule;

impl LintRule for UnusedDefinitionsRule {
    fn name(&self) -> &'static str {
        "unused-definitions"
    }

    fn description(&self) -> &'static str {
        "Check for unused slots, types, and enums"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Collect all slot references from classes
        let mut used_slots = HashSet::new();
        for class in schema.classes.values() {
            used_slots.extend(class.slots.iter().cloned());

            // Also check slot_usage
            used_slots.extend(class.slot_usage.keys().cloned());
        }

        // Find unused slots
        for slot_name in schema.slots.keys() {
            if !used_slots.contains(slot_name) {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Slot '{slot_name}' is defined but never used"),
                    element_type: Some("slot".to_string()),
                    element_name: Some(slot_name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some(
                        "Remove the unused slot or reference it in a class".to_string(),
                    ),
                    fixable: true,
                });
            }
        }

        // Collect all type references
        let mut used_types = HashSet::new();
        for slot in schema.slots.values() {
            if let Some(range) = &slot.range {
                used_types.insert(range.clone());
            }
        }

        // Find unused types
        for type_name in schema.types.keys() {
            if !used_types.contains(type_name) {
                issues.push(LintIssue {
                    rule: self.name().to_string(),
                    severity: self.severity(),
                    message: format!("Type '{type_name}' is defined but never used"),
                    element_type: Some("type".to_string()),
                    element_name: Some(type_name.clone()),
                    line: None,
                    column: None,
                    suggestion: Some(
                        "Remove the unused type or use it in a slot range".to_string(),
                    ),
                    fixable: true,
                });
            }
        }

        issues
    }

    fn fix(&self, schema: &mut SchemaDefinition, issues: &[LintIssue]) -> Result<usize> {
        let mut fixed = 0;

        for issue in issues {
            if let Some(element_name) = &issue.element_name {
                match issue.element_type.as_deref() {
                    Some("slot") => {
                        schema.slots.shift_remove(element_name);
                        fixed += 1;
                    }
                    Some("type") => {
                        schema.types.shift_remove(element_name);
                        fixed += 1;
                    }
                    _ => {}
                }
            }
        }

        Ok(fixed)
    }
}

/// Slot consistency rule
#[derive(Default)]
struct SlotConsistencyRule;

impl LintRule for SlotConsistencyRule {
    fn name(&self) -> &'static str {
        "slot-consistency"
    }

    fn description(&self) -> &'static str {
        "Check for slot definition consistency"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Check that slots referenced in classes are defined
        for (class_name, class) in &schema.classes {
            for slot_name in &class.slots {
                if !schema.slots.contains_key(slot_name) {
                    issues.push(LintIssue {
                        rule: self.name().to_string(),
                        severity: self.severity(),
                        message: format!(
                            "Class '{class_name}' references undefined slot '{slot_name}'"
                        ),
                        element_type: Some("class".to_string()),
                        element_name: Some(class_name.clone()),
                        line: None,
                        column: None,
                        suggestion: Some(format!(
                            "Define slot '{slot_name}' or remove the reference"
                        )),
                        fixable: false,
                    });
                }
            }
        }

        issues
    }

    fn fix(&self, _schema: &mut SchemaDefinition, _issues: &[LintIssue]) -> Result<usize> {
        // Cannot auto-fix undefined slots
        Ok(0)
    }
}

/// Type safety rule
#[derive(Default)]
struct TypeSafetyRule;

impl LintRule for TypeSafetyRule {
    fn name(&self) -> &'static str {
        "type-safety"
    }

    fn description(&self) -> &'static str {
        "Check type safety and range consistency"
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Valid built-in types
        let builtin_types = [
            "string",
            "integer",
            "float",
            "double",
            "boolean",
            "date",
            "datetime",
            "time",
            "uri",
            "uriorcurie",
        ];

        // Check slot ranges
        for (slot_name, slot) in &schema.slots {
            if let Some(range) = &slot.range {
                // Check if range is valid
                if !builtin_types.contains(&range.as_str())
                    && !schema.classes.contains_key(range)
                    && !schema.types.contains_key(range)
                    && !schema.enums.contains_key(range)
                {
                    issues.push(LintIssue {
                        rule: self.name().to_string(),
                        severity: self.severity(),
                        message: format!("Slot '{slot_name}' has invalid range '{range}'"),
                        element_type: Some("slot".to_string()),
                        element_name: Some(slot_name.clone()),
                        line: None,
                        column: None,
                        suggestion: Some(
                            "Use a valid built-in type or define the type".to_string(),
                        ),
                        fixable: false,
                    });
                }
            }
        }

        issues
    }

    fn fix(&self, _schema: &mut SchemaDefinition, _issues: &[LintIssue]) -> Result<usize> {
        // Cannot auto-fix type issues
        Ok(0)
    }
}

/// Schema metadata rule
#[derive(Default)]
struct SchemaMetadataRule;

impl LintRule for SchemaMetadataRule {
    fn name(&self) -> &'static str {
        "schema-metadata"
    }

    fn description(&self) -> &'static str {
        "Check for required schema metadata"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, schema: &SchemaDefinition) -> Vec<LintIssue> {
        let mut issues = Vec::new();

        // Check for schema name
        if schema.name.is_empty() {
            issues.push(LintIssue {
                rule: self.name().to_string(),
                severity: Severity::Error,
                message: "Schema has no name".to_string(),
                element_type: Some("schema".to_string()),
                element_name: None,
                line: None,
                column: None,
                suggestion: Some("Add a name field to the schema".to_string()),
                fixable: false,
            });
        }

        // Check for version
        if schema.version.is_none() {
            issues.push(LintIssue {
                rule: self.name().to_string(),
                severity: self.severity(),
                message: "Schema has no version".to_string(),
                element_type: Some("schema".to_string()),
                element_name: Some(schema.name.clone()),
                line: None,
                column: None,
                suggestion: Some("Add a version field (e.g., '1.0.0')".to_string()),
                fixable: false,
            });
        }

        // Check for license
        if schema.license.is_none() {
            issues.push(LintIssue {
                rule: self.name().to_string(),
                severity: self.severity(),
                message: "Schema has no license".to_string(),
                element_type: Some("schema".to_string()),
                element_name: Some(schema.name.clone()),
                line: None,
                column: None,
                suggestion: Some("Add a license field (e.g., 'CC0', 'MIT')".to_string()),
                fixable: false,
            });
        }

        issues
    }

    fn fix(&self, _schema: &mut SchemaDefinition, _issues: &[LintIssue]) -> Result<usize> {
        // Metadata must be added manually
        Ok(0)
    }
}

// Helper functions

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 && !prev_upper {
            result.push('_');
        }
        result.push(
            ch.to_lowercase()
                .next()
                .expect("to_lowercase() always produces at least one char"),
        );
        prev_upper = ch.is_uppercase();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use linkml_core::types::{ClassDefinition, SchemaDefinition, SlotDefinition};

    #[test]
    fn test_naming_convention_rule() {
        let mut schema = SchemaDefinition::default();

        // Add badly named class
        let class = ClassDefinition::default();
        schema.classes.insert("bad_class_name".to_string(), class);

        // Add badly named slot
        let slot = SlotDefinition::default();
        schema.slots.insert("BadSlotName".to_string(), slot);

        let rule = NamingConventionRule::default();
        let issues = rule.check(&schema);

        assert_eq!(issues.len(), 2);
        assert!(issues[0].message.contains("PascalCase"));
        assert!(issues[1].message.contains("snake_case"));
    }

    #[test]
    fn test_unused_definitions_rule() {
        let mut schema = SchemaDefinition::default();

        // Add unused slot
        let slot = SlotDefinition::default();
        schema.slots.insert("unused_slot".to_string(), slot);

        // Add class that doesn't use the slot
        let class = ClassDefinition::default();
        schema.classes.insert("MyClass".to_string(), class);

        let rule = UnusedDefinitionsRule::default();
        let issues = rule.check(&schema);

        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("never used"));
        assert!(issues[0].fixable);
    }
}
