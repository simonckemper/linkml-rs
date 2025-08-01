//! Pattern matching implementation for LinkML
//!
//! This module provides pattern matching functionality for LinkML schemas,
//! supporting regular expressions, structured patterns, and interpolation.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Error type for pattern matching operations
#[derive(Debug, Error)]
pub enum PatternError {
    /// Invalid regular expression
    #[error("Invalid regex pattern: {0}")]
    InvalidRegex(#[from] regex::Error),
    
    /// Pattern compilation failed
    #[error("Failed to compile pattern: {0}")]
    CompilationError(String),
    
    /// Interpolation error
    #[error("Interpolation error: {0}")]
    InterpolationError(String),
    
    /// Pattern not found
    #[error("Pattern not found: {0}")]
    PatternNotFound(String),
}

/// Result type for pattern operations
pub type PatternResult<T> = Result<T, PatternError>;

/// A compiled pattern ready for matching
#[derive(Debug, Clone)]
pub struct CompiledPattern {
    /// The original pattern string
    pub pattern: String,
    
    /// The compiled regex
    pub regex: Arc<Regex>,
    
    /// Named capture groups
    pub capture_groups: Vec<String>,
    
    /// Pattern metadata
    pub metadata: PatternMetadata,
}

/// Metadata about a pattern
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PatternMetadata {
    /// Whether the pattern is case-sensitive
    pub case_sensitive: bool,
    
    /// Whether to match multiline
    pub multiline: bool,
    
    /// Whether to enable dot-all mode
    pub dot_all: bool,
    
    /// Custom flags
    pub flags: HashMap<String, String>,
}

/// Pattern matcher for LinkML schemas
pub struct PatternMatcher {
    /// Compiled patterns cache
    patterns: HashMap<String, CompiledPattern>,
    
    /// Default metadata for patterns
    default_metadata: PatternMetadata,
}

impl PatternMatcher {
    /// Create a new pattern matcher
    pub fn new() -> Self {
        Self {
            patterns: HashMap::new(),
            default_metadata: PatternMetadata::default(),
        }
    }
    
    /// Create with default metadata
    pub fn with_defaults(metadata: PatternMetadata) -> Self {
        Self {
            patterns: HashMap::new(),
            default_metadata: metadata,
        }
    }
    
    /// Compile a pattern
    pub fn compile(
        &mut self,
        name: &str,
        pattern: &str,
        metadata: Option<PatternMetadata>,
    ) -> PatternResult<()> {
        let metadata = metadata.unwrap_or_else(|| self.default_metadata.clone());
        
        // Build regex with flags
        let mut regex_builder = regex::RegexBuilder::new(pattern);
        regex_builder.case_insensitive(!metadata.case_sensitive);
        regex_builder.multi_line(metadata.multiline);
        regex_builder.dot_matches_new_line(metadata.dot_all);
        
        let regex = regex_builder.build()?;
        
        // Extract capture group names
        let capture_groups: Vec<String> = regex
            .capture_names()
            .flatten()
            .map(|s| s.to_string())
            .collect();
        
        let compiled = CompiledPattern {
            pattern: pattern.to_string(),
            regex: Arc::new(regex),
            capture_groups,
            metadata,
        };
        
        self.patterns.insert(name.to_string(), compiled);
        Ok(())
    }
    
    /// Compile a structured pattern with interpolation
    pub fn compile_structured(
        &mut self,
        name: &str,
        pattern: &str,
        variables: &HashMap<String, String>,
        metadata: Option<PatternMetadata>,
    ) -> PatternResult<()> {
        let interpolated = self.interpolate_pattern(pattern, variables)?;
        self.compile(name, &interpolated, metadata)
    }
    
    /// Interpolate variables in a pattern
    fn interpolate_pattern(
        &self,
        pattern: &str,
        variables: &HashMap<String, String>,
    ) -> PatternResult<String> {
        let mut result = pattern.to_string();
        
        // Replace {var} with variable values
        for (key, value) in variables {
            let placeholder = format!("{{{}}}", key);
            let escaped_value = regex::escape(value);
            result = result.replace(&placeholder, &escaped_value);
        }
        
        // Check for unresolved placeholders
        if result.contains('{') && result.contains('}') {
            return Err(PatternError::InterpolationError(
                "Unresolved placeholders in pattern".to_string()
            ));
        }
        
        Ok(result)
    }
    
    /// Match a string against a compiled pattern
    pub fn matches(&self, pattern_name: &str, text: &str) -> PatternResult<bool> {
        let pattern = self.patterns.get(pattern_name)
            .ok_or_else(|| PatternError::PatternNotFound(pattern_name.to_string()))?;
        
        Ok(pattern.regex.is_match(text))
    }
    
    /// Find all matches for a pattern
    pub fn find_all<'t>(
        &self,
        pattern_name: &str,
        text: &'t str,
    ) -> PatternResult<Vec<Match<'t>>> {
        let pattern = self.patterns.get(pattern_name)
            .ok_or_else(|| PatternError::PatternNotFound(pattern_name.to_string()))?;
        
        let matches: Vec<Match<'t>> = pattern.regex
            .find_iter(text)
            .map(|m| Match {
                text: m.as_str(),
                start: m.start(),
                end: m.end(),
                captures: HashMap::new(),
            })
            .collect();
        
        Ok(matches)
    }
    
    /// Extract captures from a match
    pub fn capture<'t>(
        &self,
        pattern_name: &str,
        text: &'t str,
    ) -> PatternResult<Option<CaptureMatch<'t>>> {
        let pattern = self.patterns.get(pattern_name)
            .ok_or_else(|| PatternError::PatternNotFound(pattern_name.to_string()))?;
        
        if let Some(caps) = pattern.regex.captures(text) {
            let mut captures = HashMap::new();
            
            // Extract named captures
            for name in &pattern.capture_groups {
                if let Some(m) = caps.name(name) {
                    captures.insert(name.clone(), m.as_str());
                }
            }
            
            // Extract numbered captures
            for i in 0..caps.len() {
                if let Some(m) = caps.get(i) {
                    captures.insert(i.to_string(), m.as_str());
                }
            }
            
            let full_match = caps.get(0)
                .map(|m| Match {
                    text: m.as_str(),
                    start: m.start(),
                    end: m.end(),
                    captures: captures.clone(),
                })
                .expect("regex match should always have group 0");
            
            Ok(Some(CaptureMatch {
                full_match,
                captures,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Replace matches with a replacement string
    pub fn replace(
        &self,
        pattern_name: &str,
        text: &str,
        replacement: &str,
    ) -> PatternResult<String> {
        let pattern = self.patterns.get(pattern_name)
            .ok_or_else(|| PatternError::PatternNotFound(pattern_name.to_string()))?;
        
        Ok(pattern.regex.replace_all(text, replacement).to_string())
    }
    
    /// Replace with a closure
    pub fn replace_with<F>(
        &self,
        pattern_name: &str,
        text: &str,
        replacer: F,
    ) -> PatternResult<String>
    where
        F: FnMut(&regex::Captures) -> String,
    {
        let pattern = self.patterns.get(pattern_name)
            .ok_or_else(|| PatternError::PatternNotFound(pattern_name.to_string()))?;
        
        Ok(pattern.regex.replace_all(text, replacer).to_string())
    }
    
    /// Get a compiled pattern
    pub fn get_pattern(&self, name: &str) -> Option<&CompiledPattern> {
        self.patterns.get(name)
    }
    
    /// List all pattern names
    pub fn pattern_names(&self) -> Vec<&str> {
        self.patterns.keys().map(|s| s.as_str()).collect()
    }
    
    /// Clear pattern cache
    pub fn clear_cache(&mut self) {
        self.patterns.clear();
    }
}

/// A match result
#[derive(Debug, Clone)]
pub struct Match<'t> {
    /// The matched text
    pub text: &'t str,
    
    /// Start position in the original text
    pub start: usize,
    
    /// End position in the original text
    pub end: usize,
    
    /// Captured groups (if any)
    pub captures: HashMap<String, &'t str>,
}

/// A match with captures
#[derive(Debug, Clone)]
pub struct CaptureMatch<'t> {
    /// The full match
    pub full_match: Match<'t>,
    
    /// All captures (named and numbered)
    pub captures: HashMap<String, &'t str>,
}

/// Builder for pattern matcher
pub struct PatternMatcherBuilder {
    patterns: Vec<(String, String, PatternMetadata)>,
    default_metadata: PatternMetadata,
}

impl PatternMatcherBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
            default_metadata: PatternMetadata::default(),
        }
    }
    
    /// Set default metadata
    pub fn default_metadata(mut self, metadata: PatternMetadata) -> Self {
        self.default_metadata = metadata;
        self
    }
    
    /// Add a pattern
    pub fn add_pattern(
        mut self,
        name: impl Into<String>,
        pattern: impl Into<String>,
    ) -> Self {
        self.patterns.push((
            name.into(),
            pattern.into(),
            self.default_metadata.clone(),
        ));
        self
    }
    
    /// Add a pattern with metadata
    pub fn add_pattern_with_metadata(
        mut self,
        name: impl Into<String>,
        pattern: impl Into<String>,
        metadata: PatternMetadata,
    ) -> Self {
        self.patterns.push((name.into(), pattern.into(), metadata));
        self
    }
    
    /// Build the pattern matcher
    pub fn build(self) -> PatternResult<PatternMatcher> {
        let mut matcher = PatternMatcher::with_defaults(self.default_metadata);
        
        for (name, pattern, metadata) in self.patterns {
            matcher.compile(&name, &pattern, Some(metadata))?;
        }
        
        Ok(matcher)
    }
}

impl Default for PatternMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_pattern_matching() {
        let mut matcher = PatternMatcher::new();
        
        matcher.compile("email", r"[\w.+-]+@[\w.-]+\.[\w.-]+", None)
            .expect("should compile email pattern");
        
        assert!(matcher.matches("email", "test@example.com")
            .expect("matching should succeed"));
        assert!(!matcher.matches("email", "not-an-email")
            .expect("matching should succeed"));
    }
    
    #[test]
    fn test_capture_groups() {
        let mut matcher = PatternMatcher::new();
        
        matcher.compile(
            "version",
            r"v(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)",
            None,
        ).expect("should compile version pattern");
        
        let capture = matcher.capture("version", "v1.2.3")
            .expect("capture should succeed")
            .expect("should find match");
        
        assert_eq!(capture.captures.get("major"), Some(&"1"));
        assert_eq!(capture.captures.get("minor"), Some(&"2"));
        assert_eq!(capture.captures.get("patch"), Some(&"3"));
    }
    
    #[test]
    fn test_pattern_interpolation() {
        let mut matcher = PatternMatcher::new();
        
        let mut vars = HashMap::new();
        vars.insert("prefix".to_string(), "test".to_string());
        vars.insert("suffix".to_string(), "example".to_string());
        
        matcher.compile_structured(
            "custom",
            r"{prefix}_\w+_{suffix}",
            &vars,
            None,
        ).expect("should compile structured pattern");
        
        assert!(matcher.matches("custom", "test_hello_example")
            .expect("matching should succeed"));
        assert!(!matcher.matches("custom", "prod_hello_example")
            .expect("matching should succeed"));
    }
    
    #[test]
    fn test_case_insensitive() {
        let mut matcher = PatternMatcher::new();
        
        let metadata = PatternMetadata {
            case_sensitive: false,
            ..Default::default()
        };
        
        matcher.compile("word", r"hello", Some(metadata))
            .expect("should compile pattern");
        
        assert!(matcher.matches("word", "HELLO")
            .expect("matching should succeed"));
        assert!(matcher.matches("word", "Hello")
            .expect("matching should succeed"));
    }
    
    #[test]
    fn test_builder_pattern() {
        let matcher = PatternMatcherBuilder::new()
            .add_pattern("email", r"[\w.+-]+@[\w.-]+\.[\w.-]+")
            .add_pattern("url", r"https?://[\w.-]+(?:\.[\w.-]+)+[\w/]")
            .build()
            .expect("builder should succeed");
        
        assert!(matcher.matches("email", "test@example.com")
            .expect("matching should succeed"));
        assert!(matcher.matches("url", "https://example.com")
            .expect("matching should succeed"));
    }
}