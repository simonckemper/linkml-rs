//! Validation report structures

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Severity level for validation issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational message
    Info,
    /// Warning that doesn't prevent validation
    Warning,
    /// Error that causes validation to fail
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
        }
    }
}

/// A single validation issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Severity of the issue
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// `JSON` path to the problematic value
    pub path: String,
    /// Name of the validator that detected this issue
    pub validator: String,
    /// Optional error code for programmatic handling
    pub code: Option<String>,
    /// Additional context information
    pub context: HashMap<String, serde_json::Value>,
}

impl ValidationIssue {
    /// Create a new validation issue
    pub fn new(
        severity: Severity,
        message: impl Into<String>,
        path: impl Into<String>,
        validator: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            message: message.into(),
            path: path.into(),
            validator: validator.into(),
            code: None,
            context: HashMap::new(),
        }
    }

    /// Create an error issue
    pub fn error(
        message: impl Into<String>,
        path: impl Into<String>,
        validator: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, message, path, validator)
    }

    /// Create a warning issue
    pub fn warning(
        message: impl Into<String>,
        path: impl Into<String>,
        validator: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, message, path, validator)
    }

    /// Create an info issue
    pub fn info(
        message: impl Into<String>,
        path: impl Into<String>,
        validator: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Info, message, path, validator)
    }

    /// Set the error code
    #[must_use]
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Add context information
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.context.insert(key.into(), value);
        self
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.severity, self.path, self.message)
    }
}

/// Validation statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationStats {
    /// Total number of values validated
    pub total_validated: usize,
    /// Number of errors
    pub error_count: usize,
    /// Number of warnings
    pub warning_count: usize,
    /// Number of info messages
    pub info_count: usize,
    /// Validation duration in milliseconds
    pub duration_ms: u64,
    /// Number of validators executed
    pub validators_executed: usize,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
}

/// Complete validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether validation passed (no errors)
    pub valid: bool,
    /// List of validation issues
    pub issues: Vec<ValidationIssue>,
    /// Validation statistics
    pub stats: ValidationStats,
    /// Schema ID that was validated against
    pub schema_id: String,
    /// Optional target class if specified
    pub target_class: Option<String>,
}

impl ValidationReport {
    /// Create a new validation report
    pub fn new(schema_id: impl Into<String>) -> Self {
        Self {
            valid: true,
            issues: Vec::new(),
            stats: ValidationStats::default(),
            schema_id: schema_id.into(),
            target_class: None,
        }
    }

    /// Add an issue to the report
    pub fn add_issue(&mut self, issue: ValidationIssue) {
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
    pub fn errors(&self) -> impl Iterator<Item = &ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error)
    }

    /// Get all warnings
    pub fn warnings(&self) -> impl Iterator<Item = &ValidationIssue> {
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

    /// Sort issues by severity and path
    pub fn sort_issues(&mut self) {
        self.issues.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .reverse()
                .then_with(|| a.path.cmp(&b.path))
        });
    }
}

impl fmt::Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.summary())?;
        if !self.issues.is_empty() {
            writeln!(
                f,
                "
Issues:"
            )?;
            for issue in &self.issues {
                writeln!(f, "  {issue}")?;
            }
        }
        Ok(())
    }
}
