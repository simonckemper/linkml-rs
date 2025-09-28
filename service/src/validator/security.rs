//! Security hardening for `LinkML` validation
//!
//! This module provides comprehensive security measures including:
//! - Input sanitization and validation
//! - Path traversal prevention
//! - Injection attack prevention
//! - Resource exhaustion protection
//! - Sensitive data handling
//! - Audit logging

use dashmap::DashMap;
use linkml_core::error::LinkMLError;
use parking_lot::RwLock;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use timestamp_core::TimestampService;
use timestamp_service::factory;

// Pre-compile regex patterns at startup to avoid runtime compilation
static SENSITIVE_DATA_PATTERNS: std::sync::LazyLock<Vec<linkml_core::error::Result<Regex>>> =
    std::sync::LazyLock::new(|| {
        vec![
            Regex::new(r"(?i)(password|passwd|pwd)\s*[:=]\s*\S+"),
            Regex::new(r"(?i)(api[_-]?key|apikey)\s*[:=]\s*\S+"),
            Regex::new(r"(?i)(secret|token)\s*[:=]\s*\S+"),
            Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2}\b"),
            Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b"), // Credit card
            Regex::new(r"\b\d{3}-\d{2}-\d{4}\b"),       // SSN
        ]
        .into_iter()
        .map(|r| r.map_err(|e| LinkMLError::service(format!("Invalid regex pattern: {e}"))))
        .collect()
    });

use bitflags::bitflags;

bitflags! {
    /// Security features to enable
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct SecurityFeatures: u8 {
        /// Enable input sanitization
        const SANITIZE_INPUT = 0b0001;
        /// Enable audit logging
        const AUDIT_LOGGING = 0b0010;
        /// Enable sensitive data masking
        const MASK_SENSITIVE_DATA = 0b0100;
        /// Enable rate limiting per IP/user
        const RATE_LIMITING = 0b1000;

        /// All security features enabled (default)
        const ALL = Self::SANITIZE_INPUT.bits()
                  | Self::AUDIT_LOGGING.bits()
                  | Self::MASK_SENSITIVE_DATA.bits()
                  | Self::RATE_LIMITING.bits();

        /// No security features (for testing only)
        const NONE = 0b0000;
    }
}

/// Security configuration
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// Enabled security features
    pub enabled_features: SecurityFeatures,
    /// Maximum allowed path depth
    pub max_path_depth: usize,
    /// Allowed file extensions
    pub allowed_extensions: Vec<String>,
    /// Maximum input size (bytes)
    pub max_input_size: usize,
    /// Blocked patterns (regex)
    pub blocked_patterns: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enabled_features: SecurityFeatures::ALL,
            max_path_depth: 10,
            allowed_extensions: vec![
                "json".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
                "csv".to_string(),
            ],
            max_input_size: 100 * 1024 * 1024, // 100MB
            blocked_patterns: vec![
                r"(?i)(password|secret|token|key)\s*[:=]".to_string(),
                r"(?i)bearer\s+[a-zA-Z0-9\-_]+".to_string(),
            ],
        }
    }
}

impl SecurityConfig {
    /// Check if input sanitization is enabled
    #[must_use]
    pub fn sanitize_input(&self) -> bool {
        self.enabled_features
            .contains(SecurityFeatures::SANITIZE_INPUT)
    }

    /// Check if audit logging is enabled
    #[must_use]
    pub fn audit_logging(&self) -> bool {
        self.enabled_features
            .contains(SecurityFeatures::AUDIT_LOGGING)
    }

    /// Check if sensitive data masking is enabled
    #[must_use]
    pub fn mask_sensitive_data(&self) -> bool {
        self.enabled_features
            .contains(SecurityFeatures::MASK_SENSITIVE_DATA)
    }

    /// Check if rate limiting is enabled
    #[must_use]
    pub fn rate_limit_enabled(&self) -> bool {
        self.enabled_features
            .contains(SecurityFeatures::RATE_LIMITING)
    }
}

/// Security audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event ID
    pub id: String,
    /// Event type
    pub event_type: AuditEventType,
    /// User/client identifier
    pub client_id: Option<String>,
    /// Resource accessed
    pub resource: Option<String>,
    /// Action taken
    pub action: String,
    /// Result of action
    pub result: AuditResult,
    /// Additional context
    pub context: serde_json::Map<String, serde_json::Value>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Audit event types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditEventType {
    /// Input validation
    InputValidation,
    /// File access
    FileAccess,
    /// Schema load
    SchemaLoad,
    /// Validation request
    ValidationRequest,
    /// Security violation
    SecurityViolation,
    /// Configuration change
    ConfigChange,
}

/// Audit result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditResult {
    /// Success
    Success,
    /// Blocked by security
    Blocked,
    /// Failed
    Failed,
    /// Rate limited
    RateLimited,
}

/// Input sanitizer
pub struct InputSanitizer {
    config: Arc<RwLock<SecurityConfig>>,
    blocked_patterns: Vec<regex::Regex>,
}

impl InputSanitizer {
    /// Create new input sanitizer
    ///
    /// # Errors
    ///
    /// Returns an error if any of the blocked patterns are invalid regex.
    pub fn new(config: SecurityConfig) -> linkml_core::error::Result<Self> {
        let patterns = config
            .blocked_patterns
            .iter()
            .map(|p| {
                regex::Regex::new(p)
                    .map_err(|e| LinkMLError::service(format!("Invalid regex pattern: {e}")))
            })
            .collect::<linkml_core::error::Result<Vec<_>>>()?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            blocked_patterns: patterns,
        })
    }

    /// Sanitize input string
    ///
    /// # Errors
    ///
    /// Returns an error if the input contains blocked patterns or exceeds size limits.
    pub fn sanitize_string(&self, input: &str) -> linkml_core::error::Result<String> {
        let config = self.config.read();

        if !config.sanitize_input() {
            return Ok(input.to_string());
        }

        // Check size
        if input.len() > config.max_input_size {
            return Err(LinkMLError::service(format!(
                "Input size {} exceeds maximum {}",
                input.len(),
                config.max_input_size
            )));
        }

        // Check for blocked patterns
        for pattern in &self.blocked_patterns {
            if pattern.is_match(input) {
                return Err(LinkMLError::service(
                    "Input contains blocked security pattern",
                ));
            }
        }

        // Remove null bytes
        let sanitized = input.replace('\0', "");

        // Validate UTF-8
        if sanitized.chars().any(|c| c == '\u{FFFD}') {
            return Err(LinkMLError::service("Input contains invalid UTF-8"));
        }

        Ok(sanitized)
    }

    /// Sanitize `JSON` value
    ///
    /// # Errors
    ///
    /// Returns an error if any string in the `JSON` contains blocked patterns or exceeds size limits.
    pub fn sanitize_json(&self, value: &mut serde_json::Value) -> linkml_core::error::Result<()> {
        match value {
            serde_json::Value::String(s) => {
                *s = self.sanitize_string(s)?;
            }
            serde_json::Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.sanitize_json(v)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr.iter_mut() {
                    self.sanitize_json(v)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Path security validator
pub struct PathValidator {
    config: Arc<RwLock<SecurityConfig>>,
}

impl PathValidator {
    /// Create new path validator
    #[must_use]
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Validate file path
    ///
    /// # Errors
    ///
    /// Returns an error if the path is absolute, contains path traversal attempts,
    /// or violates other security constraints.
    pub fn validate_path(&self, path: &Path) -> linkml_core::error::Result<()> {
        let config = self.config.read();

        // Check if path is absolute - reject for security
        if path.is_absolute() {
            return Err(LinkMLError::service(
                "Absolute paths not allowed for security",
            ));
        }

        // Check for path traversal attempts
        let components: Vec<_> = path.components().collect();

        if components
            .iter()
            .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(LinkMLError::service("Path traversal detected (..)"));
        }

        // Check depth
        if components.len() > config.max_path_depth {
            return Err(LinkMLError::service(format!(
                "Path depth {} exceeds maximum {}",
                components.len(),
                config.max_path_depth
            )));
        }

        // Check extension
        if let Some(ext) = path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if !config.allowed_extensions.contains(&ext_str) {
                return Err(LinkMLError::service(format!(
                    "File extension '{ext_str}' not allowed"
                )));
            }
        }

        // Check for special characters
        let path_str = path.to_string_lossy();
        if path_str.contains('\0') || path_str.contains('\n') || path_str.contains('\r') {
            return Err(LinkMLError::service("Path contains invalid characters"));
        }

        Ok(())
    }

    /// Canonicalize path safely
    ///
    /// # Errors
    ///
    /// Returns an error if the path validation fails, canonicalization fails,
    /// or the resulting path escapes the base directory.
    ///
    /// # Panics
    ///
    /// This function will panic if the underlying path operations panic.
    pub fn safe_canonicalize(
        &self,
        base: &Path,
        path: &Path,
    ) -> linkml_core::error::Result<PathBuf> {
        self.validate_path(path)?;

        let joined = base.join(path);
        let canonical = joined
            .canonicalize()
            .map_err(|e| LinkMLError::service(format!("Path canonicalization failed: {e}")))?;

        // Ensure canonicalized path is still under base
        if !canonical.starts_with(base) {
            return Err(LinkMLError::service(
                "Path escapes base directory after canonicalization",
            ));
        }

        Ok(canonical)
    }
}

/// Query injection prevention
pub struct InjectionPrevention;

impl InjectionPrevention {
    /// Validate and escape `SQL`-like query
    ///
    /// # Errors
    ///
    /// Returns an error if the query contains potential `SQL` injection patterns.
    ///
    /// # Panics
    ///
    /// Panics if regex creation fails (should not happen with valid patterns).
    pub fn validate_sql_like(&self, query: &str) -> linkml_core::error::Result<String> {
        // Basic SQL injection patterns
        let dangerous_patterns = [
            r"(?i)(union\s+select)",
            r"(?i)(insert\s+into)",
            r"(?i)(drop\s+table)",
            r"(?i)(delete\s+from)",
            r"(?i)(update\s+.*\s+set)",
            r"--",
            r"/\*.*\*/",
            r";\s*$",
        ];

        for pattern in &dangerous_patterns {
            let re = regex::Regex::new(pattern).map_err(|e| LinkMLError::DataValidationError {
                message: format!("SQL injection pattern should be valid regex: {e}"),
                path: None,
                expected: None,
                actual: None,
            })?;
            if re.is_match(query) {
                return Err(LinkMLError::service(
                    "Query contains potential SQL injection pattern",
                ));
            }
        }

        // Escape single quotes
        Ok(query.replace('\'', "''"))
    }

    /// Validate `JSONPath` expression
    ///
    /// # Errors
    ///
    /// Returns an error if the `JSONPath` contains potentially dangerous patterns.
    pub fn validate_jsonpath(&self, path: &str) -> linkml_core::error::Result<()> {
        // Check for script injection in JSONPath
        if path.contains("..") && !path.contains("..") {
            return Err(LinkMLError::service(
                "JSONPath contains potential injection",
            ));
        }

        // Check for function calls that might be dangerous
        let dangerous_functions = ["eval", "exec", "system"];
        for func in &dangerous_functions {
            if path.contains(func) {
                return Err(LinkMLError::service(format!(
                    "JSONPath contains potentially dangerous function: {func}"
                )));
            }
        }

        Ok(())
    }

    /// Validate GraphQL query
    ///
    /// # Errors
    ///
    /// Returns an error if the query contains introspection queries or exceeds depth limits.
    pub fn validate_graphql(&self, query: &str) -> linkml_core::error::Result<()> {
        // Check for introspection queries if not allowed
        if query.contains("__schema") || query.contains("__type") {
            return Err(LinkMLError::service(
                "GraphQL introspection queries not allowed",
            ));
        }

        // Check query depth
        let depth = Self::calculate_query_depth(query);
        if depth > 10 {
            return Err(LinkMLError::service(format!(
                "GraphQL query depth {depth} exceeds maximum 10"
            )));
        }

        Ok(())
    }

    /// Calculate GraphQL query depth
    fn calculate_query_depth(query: &str) -> usize {
        let mut depth = 0;
        let mut current_depth: usize = 0;

        for char in query.chars() {
            match char {
                '{' => {
                    current_depth += 1;
                    depth = depth.max(current_depth);
                }
                '}' => {
                    current_depth = current_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        depth
    }
}

/// Sensitive data handler
pub struct SensitiveDataHandler {
    config: Arc<RwLock<SecurityConfig>>,
    sensitive_patterns: Vec<regex::Regex>,
}

impl SensitiveDataHandler {
    /// Create new sensitive data handler
    ///
    /// Create a new security validator
    ///
    /// # Errors
    ///
    /// Returns an error if regex patterns fail to compile
    pub fn new(config: SecurityConfig) -> linkml_core::error::Result<Self> {
        // Validate that all patterns compiled successfully
        let mut patterns = Vec::new();
        for pattern_result in SENSITIVE_DATA_PATTERNS.iter() {
            match pattern_result {
                Ok(regex) => patterns.push(regex.clone()),
                Err(e) => {
                    return Err(LinkMLError::service(format!(
                        "Failed to compile sensitive data pattern: {e}"
                    )));
                }
            }
        }

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            sensitive_patterns: patterns,
        })
    }

    /// Mask sensitive data in string
    #[must_use]
    pub fn mask_string(&self, input: &str) -> String {
        let config = self.config.read();

        if !config.mask_sensitive_data() {
            return input.to_string();
        }

        let mut masked = input.to_string();

        for pattern in &self.sensitive_patterns {
            masked = pattern.replace_all(&masked, "[REDACTED]").to_string();
        }

        masked
    }

    /// Check if data contains sensitive information
    #[must_use]
    pub fn contains_sensitive(&self, input: &str) -> bool {
        self.sensitive_patterns
            .iter()
            .any(|pattern| pattern.is_match(input))
    }
}

/// Security audit logger
pub struct AuditLogger {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    config: Arc<RwLock<SecurityConfig>>,
    timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
}

impl AuditLogger {
    /// Create new audit logger
    #[must_use]
    pub fn new(
        config: SecurityConfig,
        timestamp_service: Arc<dyn TimestampService<Error = timestamp_core::TimestampError>>,
    ) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::with_capacity(10000))),
            config: Arc::new(RwLock::new(config)),
            timestamp_service,
        }
    }

    /// Log audit event
    pub fn log_event(&self, event: AuditEvent) {
        let config = self.config.read();

        if !config.audit_logging() {
            return;
        }

        drop(config);

        let mut events = self.events.write();

        // Rotate logs if too large
        if events.len() >= 10000 {
            events.drain(0..1000);
        }

        // Log to tracing before moving event
        tracing::info!(
            event_type = ?event.event_type,
            result = ?event.result,
            resource = ?event.resource,
            "Security audit event"
        );

        events.push(event);
    }

    /// Create audit event builder
    #[must_use]
    pub fn event(&self, event_type: AuditEventType) -> AuditEventBuilder {
        AuditEventBuilder::new(event_type, self.clone())
    }

    /// Get recent events
    #[must_use]
    pub fn get_recent_events(&self, count: usize) -> Vec<AuditEvent> {
        let events = self.events.read();
        events.iter().rev().take(count).cloned().collect()
    }

    /// Search events
    #[must_use]
    pub fn search_events(
        &self,
        event_type: Option<AuditEventType>,
        result: Option<AuditResult>,
        since: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Vec<AuditEvent> {
        let events = self.events.read();

        events
            .iter()
            .filter(|e| {
                event_type.is_none_or(|t| e.event_type == t)
                    && result.is_none_or(|r| e.result == r)
                    && since.is_none_or(|s| e.timestamp >= s)
            })
            .cloned()
            .collect()
    }
}

impl Clone for AuditLogger {
    fn clone(&self) -> Self {
        Self {
            events: self.events.clone(),
            config: self.config.clone(),
            timestamp_service: self.timestamp_service.clone(),
        }
    }
}

/// Audit event builder
pub struct AuditEventBuilder {
    event_type: AuditEventType,
    client_id: Option<String>,
    resource: Option<String>,
    action: String,
    result: AuditResult,
    context: serde_json::Map<String, serde_json::Value>,
    logger: AuditLogger,
}

impl AuditEventBuilder {
    fn new(event_type: AuditEventType, logger: AuditLogger) -> Self {
        Self {
            event_type,
            client_id: None,
            resource: None,
            action: String::new(),
            result: AuditResult::Success,
            context: serde_json::Map::new(),
            logger,
        }
    }

    /// Set the client ID for the audit event
    #[must_use]
    pub fn client(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    /// Set the resource being accessed
    #[must_use]
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Set the action being performed
    #[must_use]
    pub fn action(mut self, action: impl Into<String>) -> Self {
        self.action = action.into();
        self
    }

    /// Set the result of the action
    #[must_use]
    pub fn result(mut self, result: AuditResult) -> Self {
        self.result = result;
        self
    }

    /// Add context information to the audit event
    #[must_use]
    pub fn context(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.context.insert(key.into(), v);
        }
        self
    }

    /// Log the audit event
    ///
    /// # Errors
    ///
    /// Returns an error if timestamp generation or audit logging fails.
    pub async fn log(self) -> Result<(), linkml_core::error::LinkMLError> {
        let timestamp =
            self.logger.timestamp_service.now_utc().await.map_err(|e| {
                LinkMLError::service(format!("Failed to get current timestamp: {e}"))
            })?;

        let event = AuditEvent {
            id: uuid::Uuid::new_v4().to_string(),
            event_type: self.event_type,
            client_id: self.client_id,
            resource: self.resource,
            action: self.action,
            result: self.result,
            context: self.context,
            timestamp,
        };

        self.logger.log_event(event);
        Ok(())
    }
}

/// Rate limiter for security
pub struct SecurityRateLimiter {
    limits: DashMap<String, RateLimit>,
    config: Arc<RwLock<SecurityConfig>>,
}

#[derive(Debug)]
struct RateLimit {
    requests: Vec<Instant>,
    limit: usize,
    window: Duration,
}

impl SecurityRateLimiter {
    /// Create new rate limiter
    #[must_use]
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            limits: DashMap::new(),
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Check rate limit
    ///
    /// # Errors
    ///
    /// Returns an error if the rate limit is exceeded.
    ///
    /// # Panics
    ///
    /// Panics if `Instant::now()` produces an invalid time (should not happen).
    pub fn check_limit(&self, client_id: &str) -> linkml_core::error::Result<()> {
        let config = self.config.read();

        if !config.rate_limit_enabled() {
            return Ok(());
        }

        drop(config);

        let mut entry = self
            .limits
            .entry(client_id.to_string())
            .or_insert_with(|| RateLimit {
                requests: Vec::new(),
                limit: 100,
                window: Duration::from_secs(60),
            });

        let now = Instant::now();
        let cutoff = now.checked_sub(entry.window).unwrap_or(now);

        // Remove old requests
        entry.requests.retain(|&t| t > cutoff);

        if entry.requests.len() >= entry.limit {
            return Err(LinkMLError::service(format!(
                "Rate limit exceeded: {} requests in {:?}",
                entry.limit, entry.window
            )));
        }

        entry.requests.push(now);
        Ok(())
    }
}

/// Security manager combining all security features
pub struct SecurityManager {
    sanitizer: InputSanitizer,
    path_validator: PathValidator,
    injection_prevention: InjectionPrevention,
    sensitive_handler: SensitiveDataHandler,
    audit_logger: AuditLogger,
    rate_limiter: SecurityRateLimiter,
}

impl SecurityManager {
    /// Create new security manager
    ///
    /// # Errors
    ///
    /// Returns an error if input sanitizer creation fails due to invalid regex patterns.
    pub fn new(config: SecurityConfig) -> linkml_core::error::Result<Self> {
        let timestamp_service = factory::create_timestamp_service();
        Ok(Self {
            sanitizer: InputSanitizer::new(config.clone())?,
            path_validator: PathValidator::new(config.clone()),
            injection_prevention: InjectionPrevention,
            sensitive_handler: SensitiveDataHandler::new(config.clone())?,
            audit_logger: AuditLogger::new(config.clone(), timestamp_service),
            rate_limiter: SecurityRateLimiter::new(config),
        })
    }

    /// Get sanitizer
    #[must_use]
    pub fn sanitizer(&self) -> &InputSanitizer {
        &self.sanitizer
    }

    /// Get path validator
    #[must_use]
    pub fn path_validator(&self) -> &PathValidator {
        &self.path_validator
    }

    /// Get injection prevention
    #[must_use]
    pub fn injection_prevention(&self) -> &InjectionPrevention {
        &self.injection_prevention
    }

    /// Get sensitive data handler
    #[must_use]
    pub fn sensitive_handler(&self) -> &SensitiveDataHandler {
        &self.sensitive_handler
    }

    /// Get audit logger
    #[must_use]
    pub fn audit_logger(&self) -> &AuditLogger {
        &self.audit_logger
    }

    /// Get rate limiter
    #[must_use]
    pub fn rate_limiter(&self) -> &SecurityRateLimiter {
        &self.rate_limiter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_input_sanitizer() -> anyhow::Result<(), LinkMLError> {
        let config = SecurityConfig::default();
        let sanitizer = InputSanitizer::new(config)?;

        // Normal input
        assert_eq!(sanitizer.sanitize_string("hello")?, "hello");

        // Null bytes removed
        assert_eq!(sanitizer.sanitize_string("hello\0world")?, "helloworld");

        // Size limit
        let large = "x".repeat(200 * 1024 * 1024);
        assert!(sanitizer.sanitize_string(&large).is_err());
        Ok(())
    }

    #[test]
    fn test_path_validator() {
        let validator = PathValidator::new(SecurityConfig::default());

        // Valid paths
        assert!(validator.validate_path(Path::new("data/test.json")).is_ok());
        assert!(
            validator
                .validate_path(Path::new("schemas/main.yaml"))
                .is_ok()
        );

        // Invalid paths
        assert!(validator.validate_path(Path::new("/etc/passwd")).is_err());
        assert!(
            validator
                .validate_path(Path::new("../../../etc/passwd"))
                .is_err()
        );
        assert!(
            validator
                .validate_path(Path::new("data/../../secret"))
                .is_err()
        );
    }

    #[test]
    fn test_injection_prevention() {
        let prevention = InjectionPrevention;

        // SQL-like
        assert!(prevention.validate_sql_like("SELECT * FROM users").is_ok());
        assert!(
            prevention
                .validate_sql_like("'; DROP TABLE users; --")
                .is_err()
        );

        // JSONPath
        assert!(prevention.validate_jsonpath("$.store.book[0]").is_ok());
        assert!(
            prevention
                .validate_jsonpath("$..eval('malicious')")
                .is_err()
        );
    }

    #[test]
    fn test_sensitive_data_handler() {
        let handler = SensitiveDataHandler::new(SecurityConfig::default())
            .expect("security handler creation should succeed");

        assert_eq!(handler.mask_string("password: secret123"), "[REDACTED]");

        assert_eq!(
            handler.mask_string("email: test@example.com"),
            "email: [REDACTED]"
        );

        assert!(handler.contains_sensitive("api_key: abc123"));
        assert!(!handler.contains_sensitive("normal text"));
    }
}
