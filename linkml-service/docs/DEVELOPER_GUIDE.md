# LinkML Service Developer Guide

## Table of Contents

1. [Development Setup](#development-setup)
2. [Architecture Overview](#architecture-overview)
3. [Contributing Guidelines](#contributing-guidelines)
4. [Testing Strategy](#testing-strategy)
5. [Code Style](#code-style)
6. [Building and Releasing](#building-and-releasing)
7. [Extending the Service](#extending-the-service)
8. [Performance Optimization](#performance-optimization)
9. [Debugging Tips](#debugging-tips)

## Development Setup

### Prerequisites

- Rust 1.75+ with cargo
- Git
- Python 3.8+ (for comparison testing)
- Docker (optional, for integration tests)

### Clone and Build

```bash
# Clone the repository
git clone https://github.com/simonckemper/rootreal.git
cd rootreal

# Build the LinkML service
cd crates/linkml/linkml-service
cargo build

# Run tests
cargo test

# Run with all features
cargo test --all-features
```

### Development Tools

```bash
# Install development dependencies
cargo install cargo-watch cargo-tarpaulin cargo-criterion

# Install pre-commit hooks
pre-commit install

# Setup IDE (VS Code)
code --install-extension rust-lang.rust-analyzer
code --install-extension tamasfe.even-better-toml
```

### Environment Setup

```bash
# Development environment variables
export RUST_LOG=linkml=debug
export RUST_BACKTRACE=1
export LINKML_TEST_DATA=/path/to/test/data
export LINKML_PROFILE=dev
```

## Architecture Overview

### Crate Structure

```
linkml-service/
├── Cargo.toml          # Dependencies and features
├── build.rs            # Build script
├── benches/            # Criterion benchmarks
├── examples/           # Usage examples
├── src/
│   ├── lib.rs          # Public API
│   ├── service.rs      # Core implementation
│   ├── parser/         # Schema parsing
│   ├── validator/      # Validation engine
│   ├── generator/      # Code generation
│   ├── cache/          # Caching layer
│   └── tests/          # Integration tests
└── tests/              # Additional tests
```

### Key Design Patterns

#### 1. Trait-Based Architecture

```rust
// Core service trait
#[async_trait]
pub trait LinkMLService: Send + Sync {
    async fn load_schema(&self, path: &Path) -> Result<SchemaDefinition, LinkMLError>;
    async fn validate(&self, data: &Value, schema: &SchemaDefinition, class: &str) 
        -> Result<ValidationReport, LinkMLError>;
}

// Extensible validator trait
#[async_trait]
pub trait Validator: Send + Sync {
    fn name(&self) -> &str;
    async fn validate(&self, value: &Value, context: &ValidationContext) 
        -> Result<ValidationResult, LinkMLError>;
}
```

#### 2. Dependency Injection

```rust
// Service with injected dependencies
pub struct LinkMLServiceImpl<L, T, C, M> {
    logger: Arc<L>,
    timestamp: Arc<T>,
    cache: Arc<C>,
    monitoring: Arc<M>,
}

// Builder pattern for construction
let service = LinkMLServiceBuilder::new()
    .logger(logger)
    .cache(cache)
    .monitoring(monitoring)
    .build()?;
```

#### 3. Error Handling

```rust
// Comprehensive error types
#[derive(Debug, thiserror::Error)]
pub enum LinkMLError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Parse error: {message}")]
    Parse { message: String, location: Location },
    
    #[error("Validation error: {0}")]
    Validation(ValidationError),
}

// Result type alias
pub type Result<T> = std::result::Result<T, LinkMLError>;
```

## Contributing Guidelines

### Contribution Process

1. **Fork and Branch**
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make Changes**
   - Follow RootReal coding standards
   - Add tests for new functionality
   - Update documentation

3. **Test Thoroughly**
   ```bash
   # Run all tests
   cargo test
   
   # Run specific test
   cargo test test_pattern_validation
   
   # Run with coverage
   cargo tarpaulin
   ```

4. **Submit PR**
   - Clear description
   - Link related issues
   - Pass all CI checks

### Code Standards

#### No Placeholders Policy

```rust
// ❌ NEVER do this
fn validate_something() -> Result<()> {
    todo!("Implement validation")  // FORBIDDEN
}

// ✅ Always implement fully
fn validate_something() -> Result<()> {
    // Full implementation
    Ok(())
}
```

#### Error Handling

```rust
// ❌ Never use unwrap in production code
let value = map.get("key").unwrap();

// ✅ Always handle errors properly
let value = map.get("key")
    .ok_or_else(|| LinkMLError::missing_field("key"))?;
```

#### Documentation

```rust
/// Validates data against a LinkML schema.
/// 
/// # Arguments
/// 
/// * `data` - JSON data to validate
/// * `schema` - LinkML schema definition
/// * `target_class` - Name of the target class
/// 
/// # Returns
/// 
/// A validation report containing any errors or warnings
/// 
/// # Example
/// 
/// ```rust
/// let report = service.validate(&data, &schema, "Person").await?;
/// if report.valid {
///     println!("Validation passed!");
/// }
/// ```
pub async fn validate(
    &self,
    data: &Value,
    schema: &SchemaDefinition,
    target_class: &str,
) -> Result<ValidationReport, LinkMLError> {
    // Implementation
}
```

## Testing Strategy

### Test Organization

```
tests/
├── unit/               # Unit tests for individual components
├── integration/        # Integration tests
├── performance/        # Performance benchmarks
├── regression/         # Regression test cases
└── comparison/         # Python LinkML comparison tests
```

### Unit Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_string_validation() {
        let validator = StringTypeValidator::new();
        let context = ValidationContext::new();
        
        // Test valid string
        let result = validator.validate(
            &json!("hello"),
            &context
        ).await.unwrap();
        assert!(result.is_valid());
        
        // Test invalid type
        let result = validator.validate(
            &json!(123),
            &context
        ).await.unwrap();
        assert!(!result.is_valid());
    }
}
```

### Integration Testing

```rust
#[tokio::test]
async fn test_full_validation_workflow() {
    // Setup services
    let logger = MockLoggerService::new();
    let cache = MockCacheService::new();
    let service = create_linkml_service_with_mocks(logger, cache).await.unwrap();
    
    // Load schema
    let schema = service.load_schema("test_data/person.yaml").await.unwrap();
    
    // Validate data
    let data = json!({
        "name": "John Doe",
        "age": 30
    });
    
    let report = service.validate(&data, &schema, "Person").await.unwrap();
    assert!(report.valid);
}
```

### Performance Testing

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn validation_benchmark(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let service = runtime.block_on(create_linkml_service()).unwrap();
    let schema = runtime.block_on(
        service.load_schema("benches/schema.yaml")
    ).unwrap();
    
    c.bench_function("validate_simple", |b| {
        b.iter(|| {
            runtime.block_on(service.validate(
                black_box(&json!({"name": "test"})),
                black_box(&schema),
                black_box("Person")
            ))
        })
    });
}

criterion_group!(benches, validation_benchmark);
criterion_main!(benches);
```

### Test Coverage Requirements

- Unit tests: >90% coverage
- Integration tests: All major workflows
- Performance tests: Critical paths
- No test code uses unwrap() or expect()

## Code Style

### Rust Style Guide

Follow the official Rust style guide with RootReal-specific additions:

```rust
// Imports grouped and sorted
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{LinkMLError, Result};
use crate::types::SchemaDefinition;

// Constants in UPPER_CASE
const MAX_CACHE_SIZE: usize = 10_000;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

// Error messages as constants
const ERR_SCHEMA_NOT_FOUND: &str = "Schema not found";

// Descriptive variable names
let validation_report = validator.validate(&data).await?;
let cached_schema = cache.get(&schema_key).await?;

// Early returns for error conditions
if schema.classes.is_empty() {
    return Err(LinkMLError::invalid_schema("No classes defined"));
}

// Use ? operator instead of unwrap
let class_def = schema.classes.get(class_name)
    .ok_or_else(|| LinkMLError::class_not_found(class_name))?;
```

### Async Best Practices

```rust
// Use async/await consistently
pub async fn process_batch(items: Vec<Data>) -> Result<Vec<Report>> {
    // Process in parallel with controlled concurrency
    use futures::stream::{FuturesUnordered, StreamExt};
    
    let mut futures = FuturesUnordered::new();
    
    for item in items {
        futures.push(validate_item(item));
    }
    
    let mut results = Vec::new();
    while let Some(result) = futures.next().await {
        results.push(result?);
    }
    
    Ok(results)
}

// Proper timeout handling
use tokio::time::timeout;

let result = timeout(
    Duration::from_secs(30),
    expensive_operation()
).await
.map_err(|_| LinkMLError::timeout("Operation timed out"))?;
```

## Building and Releasing

### Build Process

```bash
# Development build
cargo build

# Release build with optimizations
cargo build --release

# Build with specific features
cargo build --features "advanced-patterns,migration-tools"

# Cross-compilation
cargo build --target x86_64-unknown-linux-musl
```

### Release Checklist

1. **Version Bump**
   ```toml
   # Cargo.toml
   [package]
   version = "1.2.0"  # Follow semantic versioning
   ```

2. **Update Changelog**
   ```markdown
   # CHANGELOG.md
   ## [1.2.0] - 2024-02-01
   ### Added
   - New feature X
   ### Fixed
   - Bug Y
   ```

3. **Run Release Tests**
   ```bash
   ./scripts/release-test.sh
   ```

4. **Tag Release**
   ```bash
   git tag -a v1.2.0 -m "Release version 1.2.0"
   git push origin v1.2.0
   ```

### Continuous Integration

```yaml
# .github/workflows/linkml.yml
name: LinkML CI

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      
      - name: Run tests
        run: cargo test --all-features
        
      - name: Check formatting
        run: cargo fmt -- --check
        
      - name: Run clippy
        run: cargo clippy -- -D warnings
        
      - name: Generate coverage
        run: cargo tarpaulin --out Xml
        
      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

## Extending the Service

### Adding a New Validator

```rust
// 1. Define the validator
pub struct MyCustomValidator {
    config: ValidatorConfig,
}

#[async_trait]
impl Validator for MyCustomValidator {
    fn name(&self) -> &str {
        "my_custom_validator"
    }
    
    async fn validate(
        &self,
        value: &Value,
        context: &ValidationContext,
    ) -> Result<ValidationResult, LinkMLError> {
        // Implementation
        let mut issues = Vec::new();
        
        // Perform validation
        if !self.is_valid(value) {
            issues.push(ValidationIssue {
                error_type: "custom_error".to_string(),
                message: "Value failed custom validation".to_string(),
                severity: Severity::Error,
                path: context.path.clone(),
            });
        }
        
        Ok(ValidationResult::new(issues))
    }
}

// 2. Register with the service
service.register_validator(Box::new(MyCustomValidator::new(config)))?;
```

### Adding a New Code Generator

```rust
// 1. Define the generator
pub struct ElixirGenerator;

#[async_trait]
impl Generator for ElixirGenerator {
    fn name(&self) -> &str {
        "elixir"
    }
    
    fn file_extension(&self) -> &str {
        "ex"
    }
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> Result<String, LinkMLError> {
        let mut output = String::new();
        
        // Generate Elixir modules
        for (name, class) in &schema.classes {
            output.push_str(&format!("defmodule {} do\n", name));
            // Generate struct and functions
            output.push_str("end\n\n");
        }
        
        Ok(output)
    }
}

// 2. Register the generator
service.register_generator(Box::new(ElixirGenerator))?;
```

### Adding Cache Backends

```rust
// Implement the Cache trait for a new backend
pub struct RedisCache {
    client: redis::Client,
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Option<Vec<u8>> {
        // Redis GET implementation
    }
    
    async fn set(&self, key: &str, value: Vec<u8>, ttl: Duration) -> Result<()> {
        // Redis SETEX implementation
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        // Redis DEL implementation
    }
}
```

## Performance Optimization

### Profiling Tools

```bash
# CPU profiling with perf
cargo build --release
perf record --call-graph=dwarf target/release/linkml
perf report

# Memory profiling with valgrind
valgrind --tool=massif target/release/linkml
ms_print massif.out.*

# Rust-specific profiling
cargo install flamegraph
cargo flamegraph
```

### Optimization Techniques

#### 1. Validator Compilation

```rust
// Compile validators once and reuse
pub struct CompiledValidator {
    type_check: Box<dyn Fn(&Value) -> bool + Send + Sync>,
    constraints: Vec<Box<dyn Fn(&Value) -> Result<()> + Send + Sync>>,
}

impl CompiledValidator {
    pub fn compile(slot: &SlotDefinition) -> Self {
        // Pre-compile all checks
        let type_check = Self::compile_type_check(&slot.range);
        let constraints = Self::compile_constraints(slot);
        
        Self { type_check, constraints }
    }
}
```

#### 2. String Interning

```rust
use string_cache::DefaultAtom;

// Intern commonly used strings
lazy_static! {
    static ref COMMON_FIELDS: HashSet<DefaultAtom> = {
        let mut set = HashSet::new();
        set.insert(DefaultAtom::from("id"));
        set.insert(DefaultAtom::from("name"));
        set.insert(DefaultAtom::from("type"));
        set
    };
}
```

#### 3. Memory Pooling

```rust
use crossbeam::queue::ArrayQueue;

// Pool for reusable buffers
pub struct BufferPool {
    pool: ArrayQueue<Vec<u8>>,
}

impl BufferPool {
    pub fn acquire(&self) -> Vec<u8> {
        self.pool.pop().unwrap_or_else(|| Vec::with_capacity(4096))
    }
    
    pub fn release(&self, mut buffer: Vec<u8>) {
        buffer.clear();
        let _ = self.pool.push(buffer);
    }
}
```

## Debugging Tips

### Common Issues

#### Schema Loading Failures

```rust
// Enable debug logging for imports
std::env::set_var("RUST_LOG", "linkml::parser=debug");

// Add debug prints in import resolver
debug!("Resolving import: {} from base: {}", import, base_path.display());
```

#### Validation Performance

```rust
// Add timing to validators
let start = Instant::now();
let result = validator.validate(value, context).await?;
let duration = start.elapsed();

if duration > Duration::from_millis(100) {
    warn!("Slow validation: {} took {:?}", validator.name(), duration);
}
```

#### Memory Leaks

```rust
// Use weak references for circular dependencies
use std::sync::Weak;

struct SchemaNode {
    parent: Option<Weak<SchemaNode>>,
    children: Vec<Arc<SchemaNode>>,
}
```

### Debug Builds

```toml
# Cargo.toml - Debug features
[features]
debug-allocator = ["jemallocator"]
trace-validation = []
expensive-checks = []

[profile.dev]
debug = true
opt-level = 0

[profile.release-with-debug]
inherits = "release"
debug = true
```

### Useful Debug Commands

```bash
# Run with verbose logging
RUST_LOG=linkml=trace cargo run

# Run with backtrace
RUST_BACKTRACE=full cargo test failing_test

# Check for memory leaks
cargo test --features debug-allocator -- --nocapture

# Generate test coverage with lines
cargo tarpaulin --out Html --output-dir coverage
```

## Best Practices Summary

1. **Always implement fully** - No placeholders or TODOs
2. **Handle all errors** - No unwrap() or panic!
3. **Test thoroughly** - >90% coverage minimum
4. **Document clearly** - Examples in doc comments
5. **Profile regularly** - Monitor performance
6. **Review carefully** - Follow RootReal standards
7. **Optimize wisely** - Measure before optimizing
8. **Stay compatible** - Maintain API stability

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Async Book](https://rust-lang.github.io/async-book/)
- [LinkML Specification](https://linkml.io/linkml/)
- [RootReal Architecture](../../architecture/README.md)
- [Performance Guide](PERFORMANCE.md)
- [Security Guide](SECURITY.md)
