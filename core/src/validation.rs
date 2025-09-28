//! Validation types and utilities for LinkML schemas
//!
//! This module provides comprehensive validation functionality for LinkML schemas,
//! including severity levels, validation results, and validation context.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Severity level for validation messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Informational message
    Info,
    /// Warning that doesn't prevent processing
    Warning,
    /// Error that prevents successful processing
    Error,
    /// Critical error that indicates severe problems
    Critical,
}

impl fmt::Display for ValidationSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A single validation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Severity level of the message
    pub severity: ValidationSeverity,
    /// Human-readable message
    pub message: String,
    /// Path to the element that caused the validation issue
    pub path: Option<String>,
    /// Line number in the source (if applicable)
    pub line: Option<usize>,
    /// Column number in the source (if applicable)
    pub column: Option<usize>,
    /// Rule or constraint that was violated
    pub rule: Option<String>,
    /// Additional context information
    pub context: HashMap<String, String>,
}

impl ValidationMessage {
    /// Create a new validation message
    pub fn new(severity: ValidationSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            path: None,
            line: None,
            column: None,
            rule: None,
            context: HashMap::new(),
        }
    }

    /// Set the path for this validation message
    #[must_use]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set the line number for this validation message
    #[must_use]
    pub fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the column number for this validation message
    #[must_use]
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }

    /// Set the rule that was violated
    #[must_use]
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.rule = Some(rule.into());
        self
    }

    /// Add context information
    #[must_use]
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for ValidationMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.severity, self.message)?;

        if let Some(path) = &self.path {
            write!(f, " at {path}")?;
        }

        if let Some(line) = self.line {
            write!(f, " (line {line})")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }

        if let Some(rule) = &self.rule {
            write!(f, " [rule: {rule}]")?;
        }

        Ok(())
    }
}

/// Result of a validation operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the validation passed (no errors or critical issues)
    pub is_valid: bool,
    /// All validation messages
    pub messages: Vec<ValidationMessage>,
    /// Summary statistics
    pub summary: ValidationSummary,
}

/// Summary statistics for validation results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationSummary {
    /// Number of info messages
    pub info_count: usize,
    /// Number of warning messages
    pub warning_count: usize,
    /// Number of error messages
    pub error_count: usize,
    /// Number of critical messages
    pub critical_count: usize,
}

impl ValidationResult {
    /// Create a new validation result
    #[must_use]
    pub fn new() -> Self {
        Self {
            is_valid: true,
            messages: Vec::new(),
            summary: ValidationSummary {
                info_count: 0,
                warning_count: 0,
                error_count: 0,
                critical_count: 0,
            },
        }
    }

    /// Add a validation message
    pub fn add_message(&mut self, message: ValidationMessage) {
        // Update validity based on severity
        if matches!(
            message.severity,
            ValidationSeverity::Error | ValidationSeverity::Critical
        ) {
            self.is_valid = false;
        }

        // Update summary counts
        match message.severity {
            ValidationSeverity::Info => self.summary.info_count += 1,
            ValidationSeverity::Warning => self.summary.warning_count += 1,
            ValidationSeverity::Error => self.summary.error_count += 1,
            ValidationSeverity::Critical => self.summary.critical_count += 1,
        }

        self.messages.push(message);
    }

    /// Add an info message
    pub fn add_info(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage::new(ValidationSeverity::Info, message));
    }

    /// Add a warning message
    pub fn add_warning(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage::new(ValidationSeverity::Warning, message));
    }

    /// Add an error message
    pub fn add_error(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage::new(ValidationSeverity::Error, message));
    }

    /// Add a critical message
    pub fn add_critical(&mut self, message: impl Into<String>) {
        self.add_message(ValidationMessage::new(
            ValidationSeverity::Critical,
            message,
        ));
    }

    /// Get messages of a specific severity
    #[must_use]
    pub fn messages_by_severity(&self, severity: ValidationSeverity) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|msg| msg.severity == severity)
            .collect()
    }

    /// Check if there are any errors or critical issues
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.is_valid
    }

    /// Get the highest severity level present
    #[must_use]
    pub fn max_severity(&self) -> Option<ValidationSeverity> {
        self.messages.iter().map(|msg| msg.severity).max()
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Validation Result: {}",
            if self.is_valid { "VALID" } else { "INVALID" }
        )?;
        writeln!(
            f,
            "Summary: {} info, {} warnings, {} errors, {} critical",
            self.summary.info_count,
            self.summary.warning_count,
            self.summary.error_count,
            self.summary.critical_count
        )?;

        if !self.messages.is_empty() {
            writeln!(f, "Messages:")?;
            for message in &self.messages {
                writeln!(f, "  {message}")?;
            }
        }

        Ok(())
    }
}

/// Context for validation operations
#[derive(Debug, Clone)]
pub struct ValidationContext {
    /// Current path being validated
    pub current_path: Vec<String>,
    /// Additional context data
    pub context_data: HashMap<String, String>,
    /// Whether to stop on first error
    pub fail_fast: bool,
    /// Maximum number of messages to collect
    pub max_messages: Option<usize>,
}

impl ValidationContext {
    /// Create a new validation context
    #[must_use]
    pub fn new() -> Self {
        Self {
            current_path: Vec::new(),
            context_data: HashMap::new(),
            fail_fast: false,
            max_messages: None,
        }
    }

    /// Push a path component
    pub fn push_path(&mut self, component: impl Into<String>) {
        self.current_path.push(component.into());
    }

    /// Pop a path component
    pub fn pop_path(&mut self) -> Option<String> {
        self.current_path.pop()
    }

    /// Get the current path as a string
    #[must_use]
    pub fn current_path_string(&self) -> String {
        self.current_path.join(".")
    }

    /// Set context data
    pub fn set_context(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.context_data.insert(key.into(), value.into());
    }

    /// Get context data
    #[must_use]
    pub fn get_context(&self, key: &str) -> Option<&String> {
        self.context_data.get(key)
    }
}

impl Default for ValidationContext {
    fn default() -> Self {
        Self::new()
    }
}
