# LinkML Service Security Guide

## Overview

This guide covers security considerations, best practices, and implementation details for the RootReal LinkML Service.

## Table of Contents

1. [Security Architecture](#security-architecture)
2. [Input Validation](#input-validation)
3. [Resource Limits](#resource-limits)
4. [Access Control](#access-control)
5. [Secure Configuration](#secure-configuration)
6. [Vulnerability Prevention](#vulnerability-prevention)
7. [Audit and Logging](#audit-and-logging)
8. [Security Checklist](#security-checklist)

## Security Architecture

### Defense in Depth

```
┌─────────────────────────────────────────┐
│         Input Validation Layer          │
├─────────────────────────────────────────┤
│        Resource Limit Layer             │
├─────────────────────────────────────────┤
│         Access Control Layer            │
├─────────────────────────────────────────┤
│      Secure Processing Layer            │
├─────────────────────────────────────────┤
│         Audit Logging Layer             │
└─────────────────────────────────────────┘
```

### Security Principles

1. **Least Privilege**: Minimal permissions required
2. **Defense in Depth**: Multiple security layers
3. **Fail Secure**: Deny by default
4. **Input Validation**: Never trust user input
5. **Output Encoding**: Prevent injection attacks
6. **Audit Everything**: Comprehensive logging

## Input Validation

### Schema File Validation

```rust
pub struct SchemaValidator {
    max_size: usize,        // 10MB default
    allowed_protocols: Vec<String>,
    max_import_depth: usize,
}

impl SchemaValidator {
    pub fn validate_path(&self, path: &Path) -> Result<(), SecurityError> {
        // Prevent path traversal
        let canonical = path.canonicalize()
            .map_err(|_| SecurityError::InvalidPath)?;
        
        // Check if path is within allowed directories
        if !self.is_allowed_path(&canonical) {
            return Err(SecurityError::PathTraversal);
        }
        
        // Check file size
        let metadata = fs::metadata(&canonical)?;
        if metadata.len() > self.max_size as u64 {
            return Err(SecurityError::FileTooLarge);
        }
        
        Ok(())
    }
    
    pub fn validate_url(&self, url: &str) -> Result<(), SecurityError> {
        let parsed = Url::parse(url)
            .map_err(|_| SecurityError::InvalidUrl)?;
        
        // Check protocol
        if !self.allowed_protocols.contains(&parsed.scheme().to_string()) {
            return Err(SecurityError::DisallowedProtocol);
        }
        
        // Prevent SSRF
        if self.is_internal_address(&parsed) {
            return Err(SecurityError::SSRFAttempt);
        }
        
        Ok(())
    }
}
```

### Data Validation

```rust
pub struct DataValidator {
    max_depth: usize,       // 100 levels
    max_string_length: usize, // 1MB
    max_array_size: usize,  // 10,000 items
}

impl DataValidator {
    pub fn validate_json(&self, data: &Value) -> Result<(), SecurityError> {
        self.validate_value(data, 0)
    }
    
    fn validate_value(&self, value: &Value, depth: usize) -> Result<(), SecurityError> {
        if depth > self.max_depth {
            return Err(SecurityError::MaxDepthExceeded);
        }
        
        match value {
            Value::String(s) => {
                if s.len() > self.max_string_length {
                    return Err(SecurityError::StringTooLong);
                }
                // Check for malicious patterns
                self.check_string_safety(s)?;
            }
            Value::Array(arr) => {
                if arr.len() > self.max_array_size {
                    return Err(SecurityError::ArrayTooLarge);
                }
                for item in arr {
                    self.validate_value(item, depth + 1)?;
                }
            }
            Value::Object(obj) => {
                for (key, val) in obj {
                    self.check_string_safety(key)?;
                    self.validate_value(val, depth + 1)?;
                }
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn check_string_safety(&self, s: &str) -> Result<(), SecurityError> {
        // Prevent various injection attacks
        const DANGEROUS_PATTERNS: &[&str] = &[
            "<script",
            "javascript:",
            "data:text/html",
            "../",
            "..\\",
            "\0",
        ];
        
        let lower = s.to_lowercase();
        for pattern in DANGEROUS_PATTERNS {
            if lower.contains(pattern) {
                return Err(SecurityError::DangerousContent);
            }
        }
        
        Ok(())
    }
}
```

### Pattern Validation Security

```rust
pub struct SecurePatternValidator {
    max_pattern_length: usize,
    timeout: Duration,
}

impl SecurePatternValidator {
    pub fn validate_pattern(&self, pattern: &str) -> Result<Regex, SecurityError> {
        // Check pattern length
        if pattern.len() > self.max_pattern_length {
            return Err(SecurityError::PatternTooLong);
        }
        
        // Detect potentially catastrophic patterns
        if self.is_catastrophic_pattern(pattern) {
            return Err(SecurityError::CatastrophicPattern);
        }
        
        // Compile with timeout
        let regex = timeout(self.timeout, async {
            Regex::new(pattern)
        }).await
        .map_err(|_| SecurityError::PatternTimeout)?
        .map_err(|_| SecurityError::InvalidPattern)?;
        
        Ok(regex)
    }
    
    fn is_catastrophic_pattern(&self, pattern: &str) -> bool {
        // Detect patterns prone to catastrophic backtracking
        pattern.contains("(.*)*") ||
        pattern.contains("(.*)+") ||
        pattern.contains("(.+)*") ||
        pattern.contains("(.+)+")
    }
}
```

## Resource Limits

### Memory Limits

```rust
pub struct MemoryLimiter {
    max_heap: usize,
    max_stack: usize,
}

impl MemoryLimiter {
    pub fn check_memory(&self) -> Result<(), ResourceError> {
        let current = self.get_current_memory();
        
        if current > self.max_heap {
            return Err(ResourceError::MemoryExceeded);
        }
        
        Ok(())
    }
    
    pub fn with_limit<F, R>(&self, f: F) -> Result<R, ResourceError>
    where
        F: FnOnce() -> R,
    {
        // Set memory limit for operation
        let _guard = MemoryGuard::new(self.max_heap);
        
        // Check periodically during operation
        let result = f();
        
        self.check_memory()?;
        Ok(result)
    }
}
```

### CPU Limits

```rust
pub struct CpuLimiter {
    max_duration: Duration,
    max_cpu_percent: f64,
}

impl CpuLimiter {
    pub async fn with_timeout<F, R>(&self, future: F) -> Result<R, ResourceError>
    where
        F: Future<Output = R>,
    {
        match timeout(self.max_duration, future).await {
            Ok(result) => Ok(result),
            Err(_) => Err(ResourceError::Timeout),
        }
    }
    
    pub fn check_cpu_usage(&self) -> Result<(), ResourceError> {
        let usage = self.get_cpu_usage();
        
        if usage > self.max_cpu_percent {
            return Err(ResourceError::CpuExceeded);
        }
        
        Ok(())
    }
}
```

### Concurrent Operation Limits

```rust
pub struct ConcurrencyLimiter {
    semaphore: Arc<Semaphore>,
    max_queue_size: usize,
}

impl ConcurrencyLimiter {
    pub async fn acquire(&self) -> Result<SemaphorePermit, ResourceError> {
        // Check queue size
        if self.semaphore.available_permits() == 0 {
            let queue_size = self.estimate_queue_size();
            if queue_size > self.max_queue_size {
                return Err(ResourceError::QueueFull);
            }
        }
        
        // Acquire with timeout
        match timeout(Duration::from_secs(5), self.semaphore.acquire()).await {
            Ok(permit) => Ok(permit?),
            Err(_) => Err(ResourceError::AcquireTimeout),
        }
    }
}
```

## Access Control

### File System Access

```rust
pub struct FileAccessControl {
    allowed_paths: Vec<PathBuf>,
    denied_extensions: Vec<String>,
}

impl FileAccessControl {
    pub fn check_access(&self, path: &Path) -> Result<(), AccessError> {
        // Canonicalize to prevent traversal
        let canonical = path.canonicalize()
            .map_err(|_| AccessError::InvalidPath)?;
        
        // Check if in allowed paths
        let allowed = self.allowed_paths.iter()
            .any(|allowed| canonical.starts_with(allowed));
        
        if !allowed {
            return Err(AccessError::Forbidden);
        }
        
        // Check extension
        if let Some(ext) = canonical.extension() {
            if self.denied_extensions.contains(&ext.to_string_lossy().to_string()) {
                return Err(AccessError::DeniedExtension);
            }
        }
        
        Ok(())
    }
}
```

### Network Access Control

```rust
pub struct NetworkAccessControl {
    allowed_hosts: Vec<String>,
    denied_ips: Vec<IpAddr>,
    allowed_ports: Vec<u16>,
}

impl NetworkAccessControl {
    pub async fn check_url(&self, url: &str) -> Result<(), AccessError> {
        let parsed = Url::parse(url)?;
        
        // Check host
        if let Some(host) = parsed.host_str() {
            if !self.is_allowed_host(host) {
                return Err(AccessError::DeniedHost);
            }
            
            // Resolve IP
            let ips = lookup_host(host).await?;
            for ip in ips {
                if self.denied_ips.contains(&ip) {
                    return Err(AccessError::DeniedIP);
                }
            }
        }
        
        // Check port
        let port = parsed.port().unwrap_or(match parsed.scheme() {
            "http" => 80,
            "https" => 443,
            _ => return Err(AccessError::UnknownScheme),
        });
        
        if !self.allowed_ports.contains(&port) {
            return Err(AccessError::DeniedPort);
        }
        
        Ok(())
    }
}
```

## Secure Configuration

### Environment Variables

```rust
pub struct SecureConfig {
    required_vars: Vec<&'static str>,
    sensitive_vars: Vec<&'static str>,
}

impl SecureConfig {
    pub fn load(&self) -> Result<Config, ConfigError> {
        // Check required variables
        for var in &self.required_vars {
            if env::var(var).is_err() {
                return Err(ConfigError::MissingRequired(var));
            }
        }
        
        // Load configuration
        let mut config = Config::new();
        
        // Mask sensitive values in logs
        for var in &self.sensitive_vars {
            if let Ok(value) = env::var(var) {
                config.set_masked(var, value);
            }
        }
        
        Ok(config)
    }
}
```

### Configuration Validation

```rust
pub fn validate_config(config: &LinkMLServiceConfig) -> Result<(), ConfigError> {
    // Validate paths
    for path in &config.import_paths {
        if !path.exists() {
            return Err(ConfigError::InvalidPath(path.clone()));
        }
    }
    
    // Validate limits
    if config.max_memory_mb > 4096 {
        return Err(ConfigError::ExcessiveMemory);
    }
    
    if config.validation_timeout > Duration::from_secs(300) {
        return Err(ConfigError::ExcessiveTimeout);
    }
    
    // Validate cache settings
    if config.cache_size > 100_000 {
        return Err(ConfigError::ExcessiveCacheSize);
    }
    
    Ok(())
}
```

## Vulnerability Prevention

### Path Traversal Prevention

```rust
pub fn safe_path_join(base: &Path, untrusted: &str) -> Result<PathBuf, SecurityError> {
    // Remove any path separators
    let cleaned = untrusted
        .replace('/', "")
        .replace('\\', "")
        .replace("..", "");
    
    // Join with base
    let joined = base.join(&cleaned);
    
    // Verify still under base
    let canonical_base = base.canonicalize()?;
    let canonical_joined = joined.canonicalize()?;
    
    if !canonical_joined.starts_with(&canonical_base) {
        return Err(SecurityError::PathTraversal);
    }
    
    Ok(canonical_joined)
}
```

### Injection Prevention

```rust
pub struct InjectionPrevention;

impl InjectionPrevention {
    pub fn escape_sql(input: &str) -> String {
        input
            .replace('\'', "''")
            .replace('\\', "\\\\")
            .replace('\0', "\\0")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\x1a', "\\Z")
    }
    
    pub fn escape_html(input: &str) -> String {
        input
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
    }
    
    pub fn sanitize_filename(input: &str) -> String {
        input
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_' || *c == '.')
            .take(255)
            .collect()
    }
}
```

### DoS Prevention

```rust
pub struct DoSPrevention {
    rate_limiter: RateLimiter,
    circuit_breaker: CircuitBreaker,
}

impl DoSPrevention {
    pub async fn check_request(&self, client_id: &str) -> Result<(), SecurityError> {
        // Check rate limit
        if !self.rate_limiter.check(client_id).await? {
            return Err(SecurityError::RateLimitExceeded);
        }
        
        // Check circuit breaker
        if self.circuit_breaker.is_open() {
            return Err(SecurityError::ServiceUnavailable);
        }
        
        Ok(())
    }
}
```

## Audit and Logging

### Security Event Logging

```rust
#[derive(Debug, Serialize)]
pub struct SecurityEvent {
    timestamp: DateTime<Utc>,
    event_type: SecurityEventType,
    user_id: Option<String>,
    ip_address: Option<IpAddr>,
    resource: String,
    outcome: Outcome,
    details: Value,
}

pub enum SecurityEventType {
    AccessDenied,
    InvalidInput,
    RateLimitExceeded,
    ResourceExhausted,
    SuspiciousPattern,
    ConfigurationChange,
}

pub struct SecurityLogger {
    logger: Arc<dyn LoggerService>,
}

impl SecurityLogger {
    pub async fn log_event(&self, event: SecurityEvent) -> Result<(), Error> {
        // Log to security audit log
        self.logger.security(&serde_json::to_string(&event)?).await?;
        
        // Alert on critical events
        match event.event_type {
            SecurityEventType::SuspiciousPattern |
            SecurityEventType::AccessDenied => {
                self.send_alert(&event).await?;
            }
            _ => {}
        }
        
        Ok(())
    }
}
```

### Audit Trail

```rust
pub struct AuditTrail {
    storage: Arc<dyn AuditStorage>,
}

impl AuditTrail {
    pub async fn record_operation(&self, operation: Operation) -> Result<(), Error> {
        let entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            operation: operation.name,
            user: operation.user,
            input_hash: self.hash_input(&operation.input),
            output_hash: self.hash_output(&operation.output),
            duration: operation.duration,
            success: operation.success,
        };
        
        self.storage.store(entry).await?;
        Ok(())
    }
    
    fn hash_input(&self, input: &Value) -> String {
        // Hash sensitive data for audit
        let serialized = serde_json::to_string(input).unwrap();
        format!("{:x}", sha256::digest(serialized))
    }
}
```

## Security Checklist

### Development Phase

- [ ] All inputs validated before processing
- [ ] Resource limits implemented and tested
- [ ] No hardcoded secrets or credentials
- [ ] Dependencies audited for vulnerabilities
- [ ] Error messages don't leak sensitive info
- [ ] All paths canonicalized before use
- [ ] Timeouts on all external operations
- [ ] Rate limiting implemented

### Code Review

- [ ] No use of `unsafe` without justification
- [ ] All unwrap() calls removed
- [ ] Input validation comprehensive
- [ ] Error handling doesn't expose internals
- [ ] Logging doesn't include sensitive data
- [ ] Resource cleanup guaranteed (RAII)
- [ ] Concurrent access properly synchronized

### Testing

- [ ] Fuzzing tests for all inputs
- [ ] Path traversal tests
- [ ] Resource exhaustion tests
- [ ] Concurrent access tests
- [ ] Error injection tests
- [ ] Performance under attack scenarios

### Deployment

- [ ] Minimal permissions configured
- [ ] Network policies restricted
- [ ] Secrets management configured
- [ ] Audit logging enabled
- [ ] Monitoring alerts configured
- [ ] Incident response plan ready
- [ ] Regular security updates scheduled

### Operations

- [ ] Security patches applied promptly
- [ ] Audit logs reviewed regularly
- [ ] Anomaly detection active
- [ ] Backup and recovery tested
- [ ] Access controls reviewed
- [ ] Security training completed

## Security Best Practices

1. **Never Trust User Input**: Validate everything
2. **Fail Securely**: Deny by default
3. **Minimize Attack Surface**: Disable unused features
4. **Defense in Depth**: Multiple security layers
5. **Least Privilege**: Minimal permissions
6. **Audit Everything**: Comprehensive logging
7. **Stay Updated**: Regular security patches
8. **Test Security**: Regular penetration testing

## Incident Response

### Security Incident Procedure

1. **Detect**: Monitoring alerts trigger
2. **Contain**: Isolate affected systems
3. **Investigate**: Analyze logs and forensics
4. **Remediate**: Fix vulnerabilities
5. **Recover**: Restore normal operations
6. **Review**: Post-incident analysis

### Contact Information

- Security Team: textpast@textpast.com
- Incident Response: textpast@textpast.com

## Conclusion

Security is a continuous process. Regular reviews, updates, and testing are essential to maintain a secure LinkML Service deployment.
