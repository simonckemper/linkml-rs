//! Validation report structures using string interning for memory optimization
//!
//! This module provides validation report structures that use string interning
//! to reduce memory consumption, especially for frequently occurring strings
//! like field names, error codes, and validator names.

use std::collections::HashMap;
use std::fmt;

use super::report::{Severity, ValidationStats};
use super::string_interner::{InternedString, StringInterner, global_interner};

/// A single validation issue using interned strings for memory efficiency
#[derive(Debug, Clone)]
pub struct InternedValidationIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Human-readable message (interned)
    pub message: InternedString,
    /// `JSON` path to the problematic value (interned)
    pub path: InternedString,
    /// Name of the validator that detected this issue (interned for memory efficiency)
    pub validator: InternedString,
    /// Optional error code for programmatic handling (interned)
    pub code: Option<InternedString>,
    /// Additional context information (keys are interned)
    pub context: HashMap<InternedString, serde_json::Value>,
}

impl InternedValidationIssue {
    /// Create a new validation issue using the global interner
    pub fn new(
        severity: Severity,
        message: impl AsRef<str>,
        path: impl AsRef<str>,
        validator: impl AsRef<str>,
    ) -> Self {
        let interner = global_interner();
        Self {
            severity,
            message: interner.intern(message.as_ref()),
            path: interner.intern(path.as_ref()),
            validator: interner.intern(validator.as_ref()),
            code: None,
            context: HashMap::new(),
        }
    }

    /// Create an error issue
    pub fn error(
        message: impl AsRef<str>,
        path: impl AsRef<str>,
        validator: impl AsRef<str>,
    ) -> Self {
        Self::new(Severity::Error, message, path, validator)
    }

    /// Create a warning issue
    pub fn warning(
        message: impl AsRef<str>,
        path: impl AsRef<str>,
        validator: impl AsRef<str>,
    ) -> Self {
        Self::new(Severity::Warning, message, path, validator)
    }

    /// Create an info issue
    pub fn info(
        message: impl AsRef<str>,
        path: impl AsRef<str>,
        validator: impl AsRef<str>,
    ) -> Self {
        Self::new(Severity::Info, message, path, validator)
    }

    /// Set the error code using interning
    #[must_use]
    pub fn with_code(mut self, code: impl AsRef<str>) -> Self {
        let interner = global_interner();
        self.code = Some(interner.intern(code.as_ref()));
        self
    }

    /// Add context information with interned key
    #[must_use]
    pub fn with_context(mut self, key: impl AsRef<str>, value: serde_json::Value) -> Self {
        let interner = global_interner();
        self.context.insert(interner.intern(key.as_ref()), value);
        self
    }

    /// Convert to a regular `ValidationIssue` for serialization
    #[must_use]
    pub fn to_regular(&self) -> super::report::ValidationIssue {
        let interner = global_interner();
        let mut regular = super::report::ValidationIssue::new(
            self.severity,
            interner.get(self.message).unwrap_or_default(),
            interner.get(self.path).unwrap_or_default(),
            interner.get(self.validator).unwrap_or_default(),
        );

        if let Some(code) = self.code {
            regular.code = Some(interner.get(code).unwrap_or_default());
        }

        for (key, value) in &self.context {
            regular
                .context
                .insert(interner.get(*key).unwrap_or_default(), value.clone());
        }

        regular
    }

    /// Get message as string
    #[must_use]
    pub fn message_str(&self) -> String {
        global_interner().get(self.message).unwrap_or_default()
    }

    /// Get path as string
    #[must_use]
    pub fn path_str(&self) -> String {
        global_interner().get(self.path).unwrap_or_default()
    }

    /// Get validator as string
    #[must_use]
    pub fn validator_str(&self) -> String {
        global_interner().get(self.validator).unwrap_or_default()
    }

    /// Get code as string
    #[must_use]
    pub fn code_str(&self) -> Option<String> {
        self.code
            .map(|c| global_interner().get(c).unwrap_or_default())
    }
}

impl fmt::Display for InternedValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {}: {}",
            self.severity,
            self.path_str(),
            self.message_str()
        )
    }
}

/// Complete validation report using interned strings
#[derive(Debug, Clone)]
pub struct InternedValidationReport {
    /// Whether validation passed (no errors)
    pub valid: bool,
    /// List of validation issues with interned strings
    pub issues: Vec<InternedValidationIssue>,
    /// Validation statistics
    pub stats: ValidationStats,
    /// Schema ID that was validated against (interned)
    pub schema_id: InternedString,
    /// Optional target class if specified (interned)
    pub target_class: Option<InternedString>,
}

impl InternedValidationReport {
    /// Create a new validation report
    pub fn new(schema_id: impl AsRef<str>) -> Self {
        let interner = global_interner();
        Self {
            valid: true,
            issues: Vec::new(),
            stats: ValidationStats::default(),
            schema_id: interner.intern(schema_id.as_ref()),
            target_class: None,
        }
    }

    /// Add an issue to the report
    pub fn add_issue(&mut self, issue: InternedValidationIssue) {
        match issue.severity {
            Severity::Error => {
                self.valid = false;
                self.stats.error_count += 1;
            }
            Severity::Warning => self.stats.warning_count += 1,
            Severity::Info => self.stats.info_count += 1,
        }
        self.issues.push(issue);
    }

    /// Get all errors
    pub fn errors(&self) -> impl Iterator<Item = &InternedValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    /// Get all warnings
    pub fn warnings(&self) -> impl Iterator<Item = &InternedValidationIssue> {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
    }

    /// Get a summary of the validation
    #[must_use]
    pub fn summary(&self) -> String {
        if self.valid {
            format!(
                "Validation passed with {} warnings",
                self.stats.warning_count
            )
        } else {
            format!(
                "Validation failed with {} errors and {} warnings",
                self.stats.error_count, self.stats.warning_count
            )
        }
    }

    /// Convert to regular `ValidationReport` for serialization
    #[must_use]
    pub fn to_regular(&self) -> super::report::ValidationReport {
        let interner = global_interner();
        let mut report =
            super::report::ValidationReport::new(interner.get(self.schema_id).unwrap_or_default());

        report.valid = self.valid;
        report.stats = self.stats.clone();

        if let Some(tc) = self.target_class {
            report.target_class = Some(interner.get(tc).unwrap_or_default());
        }

        for issue in &self.issues {
            report.issues.push(issue.to_regular());
        }

        report
    }

    /// Set target class with interning
    pub fn set_target_class(&mut self, class: impl AsRef<str>) {
        let interner = global_interner();
        self.target_class = Some(interner.intern(class.as_ref()));
    }
}

/// Builder for creating validation issues with commonly used strings
pub struct IssueBuilder {
    interner: &'static StringInterner,
}

impl IssueBuilder {
    /// Create a new issue builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            interner: global_interner(),
        }
    }

    /// Create a required field missing error
    #[must_use]
    pub fn required_field_missing(&self, field_name: &str, path: &str) -> InternedValidationIssue {
        let common = self.interner.common();
        InternedValidationIssue {
            severity: Severity::Error,
            message: self
                .interner
                .intern(&format!("Required field '{field_name}' is missing")),
            path: self.interner.intern(path),
            validator: self.interner.intern("RequiredValidator"),
            code: Some(common.error_required),
            context: HashMap::new(),
        }
    }

    /// Create a type mismatch error
    #[must_use]
    pub fn type_mismatch(
        &self,
        expected: &str,
        actual: &str,
        path: &str,
    ) -> InternedValidationIssue {
        let common = self.interner.common();
        InternedValidationIssue {
            severity: Severity::Error,
            message: self
                .interner
                .intern(&format!("Type mismatch: expected {expected}, got {actual}")),
            path: self.interner.intern(path),
            validator: self.interner.intern("TypeValidator"),
            code: Some(common.error_type_mismatch),
            context: HashMap::new(),
        }
    }

    /// Create a pattern mismatch error
    #[must_use]
    pub fn pattern_mismatch(
        &self,
        value: &str,
        pattern: &str,
        path: &str,
    ) -> InternedValidationIssue {
        let common = self.interner.common();
        InternedValidationIssue {
            severity: Severity::Error,
            message: self.interner.intern(&format!(
                "Value '{value}' does not match pattern '{pattern}'"
            )),
            path: self.interner.intern(path),
            validator: self.interner.intern("PatternValidator"),
            code: Some(common.error_pattern_mismatch),
            context: HashMap::new(),
        }
    }

    /// Create a range violation error
    #[must_use]
    pub fn range_violation(&self, message: &str, path: &str) -> InternedValidationIssue {
        let common = self.interner.common();
        InternedValidationIssue {
            severity: Severity::Error,
            message: self.interner.intern(message),
            path: self.interner.intern(path),
            validator: self.interner.intern("RangeValidator"),
            code: Some(common.error_range_violation),
            context: HashMap::new(),
        }
    }

    /// Create an enum violation error
    #[must_use]
    pub fn enum_violation(
        &self,
        value: &str,
        allowed: &[String],
        path: &str,
    ) -> InternedValidationIssue {
        let common = self.interner.common();
        InternedValidationIssue {
            severity: Severity::Error,
            message: self.interner.intern(&format!(
                "Value '{}' is not in allowed values: [{}]",
                value,
                allowed.join(", ")
            )),
            path: self.interner.intern(path),
            validator: self.interner.intern("EnumValidator"),
            code: Some(common.error_enum_violation),
            context: HashMap::new(),
        }
    }
}

impl Default for IssueBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory statistics for interned validation reports
#[derive(Debug, Clone)]
pub struct InternedMemoryStats {
    /// Number of unique strings interned
    pub unique_strings: usize,
    /// Total bytes saved by interning
    pub bytes_saved: usize,
    /// Number of string references
    pub reference_count: usize,
    /// Average string length
    pub avg_string_length: f64,
}

impl InternedMemoryStats {
    /// Calculate memory statistics for the current interner state
    #[must_use]
    pub fn calculate() -> Self {
        let interner = global_interner();
        let stats = interner.stats();

        // Estimate bytes saved (assuming average 3 references per string)
        let estimated_references = stats.total_strings * 3;
        let bytes_without_interning = stats.total_bytes * 3;
        let bytes_with_interning = stats.total_bytes + (estimated_references * 8); // 8 bytes per reference
        let bytes_saved = bytes_without_interning.saturating_sub(bytes_with_interning);

        Self {
            unique_strings: stats.total_strings,
            bytes_saved,
            reference_count: estimated_references,
            avg_string_length: stats.average_length,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interned_validation_issue() {
        let issue =
            InternedValidationIssue::error("Test error message", "$.field.path", "TestValidator");

        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.message_str(), "Test error message");
        assert_eq!(issue.path_str(), "$.field.path");
        assert_eq!(issue.validator_str(), "TestValidator");
    }

    #[test]
    fn test_issue_builder() {
        let builder = IssueBuilder::new();

        let issue = builder.required_field_missing("name", "$.person");
        assert_eq!(issue.severity, Severity::Error);
        assert!(issue.message_str().contains("Required field"));
        assert!(issue.message_str().contains("name"));
        assert_eq!(issue.path_str(), "$.person");

        let issue2 = builder.type_mismatch("string", "number", "$.age");
        assert!(issue2.message_str().contains("Type mismatch"));
        assert_eq!(issue2.path_str(), "$.age");
    }

    #[test]
    fn test_interned_report() {
        let mut report = InternedValidationReport::new("test-schema");

        report.add_issue(InternedValidationIssue::error(
            "Error 1",
            "$.field1",
            "Validator1",
        ));

        report.add_issue(InternedValidationIssue::warning(
            "Warning 1",
            "$.field2",
            "Validator2",
        ));

        assert!(!report.valid);
        assert_eq!(report.stats.error_count, 1);
        assert_eq!(report.stats.warning_count, 1);
        assert_eq!(report.errors().count(), 1);
        assert_eq!(report.warnings().count(), 1);
    }

    #[test]
    fn test_memory_efficiency() {
        // Create many issues with the same strings
        let mut issues = Vec::new();
        for i in 0..100 {
            let issue = InternedValidationIssue::error(
                "Required field is missing",   // Same message
                &format!("$.items[{i}].name"), // Different paths
                "RequiredValidator",           // Same validator
            );
            issues.push(issue);
        }

        // The interner should have deduplicated the common strings
        let stats = InternedMemoryStats::calculate();

        // We should have fewer unique strings than total issues
        assert!(stats.unique_strings < issues.len() * 3);
    }
}
