# LinkML Service Architecture

## Overview

The RootReal LinkML Service provides a high-performance, production-ready implementation of LinkML schema validation and code generation. This document describes the architectural design, key components, and integration patterns.

## Table of Contents

1. [Design Principles](#design-principles)
2. [Component Architecture](#component-architecture)
3. [Service Integration](#service-integration)
4. [Data Flow](#data-flow)
5. [Performance Architecture](#performance-architecture)
6. [Security Architecture](#security-architecture)
7. [Extensibility](#extensibility)

## Design Principles

### 1. Production-First Design
- **Zero Placeholders**: Every component implements real functionality
- **Resource Management**: Strict limits on memory, CPU, and concurrent operations
- **Fault Tolerance**: Circuit breakers, retries, and graceful degradation
- **Observability**: Comprehensive metrics, logging, and health checks

### 2. Performance by Design
- **Compiled Validators**: Pre-compile validation logic for 10x speedup
- **Multi-Layer Caching**: L1/L2/L3 cache hierarchy with automatic promotion
- **Zero-Copy Parsing**: Minimize allocations during schema processing
- **Async Everything**: Non-blocking operations throughout

### 3. Enterprise Integration
- **Service-Oriented**: Integrates with RootReal's 52+ service ecosystem
- **Dependency Injection**: Flexible service composition
- **Configuration Management**: Hot-reload support via ConfigurationService
- **Monitoring Ready**: Built-in telemetry and performance tracking

## Component Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                      LinkML Service API                      │
├─────────────────────────────────────────────────────────────┤
│                    Service Implementation                    │
├──────────────┬────────────────┬────────────────┬───────────┤
│    Parser    │   Validator    │   Generator    │   Cache   │
├──────────────┼────────────────┼────────────────┼───────────┤
│ YAML Parser  │ Type Validators│ TypeQL Gen     │ L1 Cache  │
│ JSON Parser  │ Constraint Val │ SQL Gen        │ L2 Cache  │
│ Import Resolver│ Pattern Val   │ GraphQL Gen    │ L3 Cache  │
│ Schema Merger │ Compiled Cache │ Rust Gen       │ Key Builder│
└──────────────┴────────────────┴────────────────┴───────────┘
```

### Module Organization

```
linkml-service/
├── src/
│   ├── lib.rs              # Public API and trait definitions
│   ├── service.rs          # Main service implementation
│   ├── parser/             # Schema parsing and loading
│   │   ├── yaml_parser.rs  # YAML schema parser
│   │   ├── json_parser.rs  # JSON schema parser
│   │   └── import_resolver.rs # Import resolution
│   ├── validator/          # Validation engine
│   │   ├── engine.rs       # Core validation logic
│   │   ├── compiled.rs     # Compiled validator cache
│   │   ├── validators/     # Individual validators
│   │   └── context.rs      # Validation context
│   ├── generator/          # Code generation
│   │   ├── typeql.rs       # TypeDB TypeQL
│   │   ├── sql.rs          # SQL DDL
│   │   ├── graphql.rs      # GraphQL schema
│   │   └── rust.rs         # Rust structs
│   ├── cache/              # Caching infrastructure
│   │   ├── multi_layer.rs  # L1/L2/L3 implementation
│   │   ├── key_builder.rs  # Cache key generation
│   │   └── warming.rs      # Cache warming strategies
│   ├── migration.rs        # Schema migration tools
│   ├── ide.rs              # IDE integration support
│   └── cli.rs              # Command-line interface
```

## Service Integration

### RootReal Service Dependencies

```
LinkMLService
    │
    ├─── LoggerService (Phase 2)
    │    └── Structured logging for all operations
    │
    ├─── TimestampService (Phase 2)
    │    └── Consistent timestamps for cache TTL
    │
    ├─── ConfigurationService (Phase 5)
    │    └── Hot-reload configuration support
    │
    ├─── CacheService (Phase 10)
    │    └── External cache backend integration
    │
    ├─── MonitoringService (Phase 11)
    │    └── Performance metrics and health checks
    │
    ├─── ErrorHandlingService (Phase 6)
    │    └── Consistent error handling and recovery
    │
    └─── TaskManagementService (Phase 3)
         └── Async task coordination
```

### Integration Patterns

#### 1. Service Injection
```rust
pub struct LinkMLServiceImpl<C, L, T, M> 
where
    C: ConfigurationService,
    L: LoggerService,
    T: TimestampService,
    M: MonitoringService,
{
    config: Arc<C>,
    logger: Arc<L>,
    timestamp: Arc<T>,
    monitoring: Arc<M>,
    // ... internal components
}
```

#### 2. Async Service Calls
```rust
// All operations are async for non-blocking integration
pub async fn validate(
    &self,
    data: &Value,
    schema: &SchemaDefinition,
    target_class: &str,
) -> Result<ValidationReport, LinkMLError> {
    // Log operation start
    self.logger.info("Starting validation").await?;
    
    // Record metrics
    let start = self.timestamp.now().await?;
    
    // Perform validation
    let result = self.engine.validate(data, schema, target_class).await?;
    
    // Record duration
    let duration = self.timestamp.elapsed(start).await?;
    self.monitoring.record_metric("validation.duration", duration).await?;
    
    Ok(result)
}
```

## Data Flow

### Validation Flow

```
1. Input Data (JSON/YAML)
     │
2. Schema Loading
     ├── Import Resolution
     ├── Schema Merging
     └── Compilation
     │
3. Validator Compilation
     ├── Type Analysis
     ├── Constraint Extraction
     └── Optimizer Pass
     │
4. Validation Execution
     ├── Type Checking
     ├── Constraint Validation
     └── Cross-Field Rules
     │
5. Report Generation
     ├── Error Collection
     ├── Path Tracking
     └── Severity Assessment
```

### Code Generation Flow

```
1. Schema Analysis
     ├── Class Hierarchy
     ├── Slot Dependencies
     └── Type Resolution
     │
2. Target Selection
     ├── TypeQL
     ├── SQL
     ├── GraphQL
     └── Rust
     │
3. Generation Strategy
     ├── Template Selection
     ├── Naming Convention
     └── Feature Flags
     │
4. Code Emission
     ├── Structure Generation
     ├── Documentation
     └── Formatting
```

## Performance Architecture

### Multi-Layer Cache Design

```
┌─────────────────────────────────────────┐
│           Request Layer                 │
├─────────────────────────────────────────┤
│      L1 Cache (In-Memory LRU)          │
│  - Size: 1000 entries                  │
│  - TTL: 5 minutes                      │
│  - Hit Rate: ~60%                      │
├─────────────────────────────────────────┤
│      L2 Cache (In-Memory)              │
│  - Size: 10,000 entries                │
│  - TTL: 1 hour                         │
│  - Hit Rate: ~30%                      │
├─────────────────────────────────────────┤
│      L3 Cache (External)               │
│  - Size: Unlimited                     │
│  - TTL: 24 hours                       │
│  - Hit Rate: ~7%                       │
├─────────────────────────────────────────┤
│         Validation Engine               │
└─────────────────────────────────────────┘
```

### Cache Key Strategy

```rust
// Hierarchical cache keys for efficient invalidation
format!(
    "linkml:v1:{}:{}:{}:{}",
    operation,      // validate, compile, generate
    schema_hash,    // xxHash3 of schema
    data_hash,      // xxHash3 of data (if applicable)
    params_hash     // xxHash3 of parameters
)
```

### Compiled Validator Cache

```
Schema Definition
     │
     ├── Analyze Structure
     ├── Extract Constraints
     └── Generate Validator Code
     │
Compiled Validator
     ├── Type Checks (inlined)
     ├── Pattern Matchers (pre-compiled)
     └── Range Validators (optimized)
     │
Cached for Reuse
```

### Performance Optimizations

1. **String Interning**: Common strings cached globally
2. **Buffer Pooling**: Reuse allocations for parsing
3. **Parallel Validation**: Multi-threaded for batch operations
4. **Zero-Copy JSON**: Use `serde_json::from_slice`
5. **Lazy Compilation**: Compile validators on-demand

## Security Architecture

### Input Validation

```rust
// All inputs sanitized before processing
pub struct SecurityValidator {
    max_schema_size: usize,      // 10MB default
    max_import_depth: usize,     // 10 levels
    max_validation_depth: usize, // 100 levels
    allowed_protocols: Vec<String>, // ["https", "file"]
}
```

### Path Traversal Prevention

```rust
// Secure path resolution
fn resolve_import(base: &Path, import: &str) -> Result<PathBuf> {
    let resolved = base.join(import).canonicalize()?;
    
    // Ensure resolved path is within allowed directories
    if !resolved.starts_with(base) {
        return Err(SecurityError::PathTraversal);
    }
    
    Ok(resolved)
}
```

### Resource Limits

```rust
pub struct ResourceLimits {
    max_memory: usize,           // 500MB
    max_cpu_time: Duration,      // 60 seconds
    max_concurrent_ops: usize,   // 100
    rate_limit: RateLimiter,     // 1000 req/min
}
```

## Extensibility

### Custom Validators

```rust
#[async_trait]
pub trait Validator: Send + Sync {
    fn name(&self) -> &str;
    
    async fn validate(
        &self,
        value: &Value,
        context: &ValidationContext,
    ) -> Result<ValidationResult, LinkMLError>;
}

// Register custom validators
service.register_validator(Box::new(MyCustomValidator))?;
```

### Code Generator Plugins

```rust
#[async_trait]
pub trait Generator: Send + Sync {
    fn name(&self) -> &str;
    fn file_extension(&self) -> &str;
    
    async fn generate(
        &self,
        schema: &SchemaDefinition,
        options: &GeneratorOptions,
    ) -> Result<String, LinkMLError>;
}

// Register custom generators
service.register_generator(Box::new(MyCustomGenerator))?;
```

### Schema Transformers

```rust
#[async_trait]
pub trait SchemaTransformer: Send + Sync {
    async fn transform(
        &self,
        schema: SchemaDefinition,
    ) -> Result<SchemaDefinition, LinkMLError>;
}

// Apply transformations
let transformed = transformer.transform(schema).await?;
```

## Error Handling

### Error Hierarchy

```rust
pub enum LinkMLError {
    Io(IoError),
    Parse(ParseError),
    Validation(ValidationError),
    Generation(GenerationError),
    Service(ServiceError),
    Security(SecurityError),
}
```

### Recovery Strategies

1. **Circuit Breaker**: Prevent cascading failures
2. **Retry with Backoff**: Transient error recovery
3. **Graceful Degradation**: Partial functionality
4. **Error Enrichment**: Add context to errors

## Monitoring and Observability

### Key Metrics

```rust
// Performance metrics
linkml.validation.duration_ms
linkml.compilation.duration_ms
linkml.generation.duration_ms

// Cache metrics
linkml.cache.hit_rate
linkml.cache.eviction_rate
linkml.cache.size_bytes

// Error metrics
linkml.errors.validation
linkml.errors.parsing
linkml.errors.generation

// Resource metrics
linkml.memory.heap_bytes
linkml.cpu.usage_percent
linkml.concurrent_operations
```

### Health Checks

```rust
pub async fn health_check(&self) -> HealthStatus {
    HealthStatus {
        cache_connected: self.cache.ping().await.is_ok(),
        memory_ok: self.memory_usage() < self.limits.max_memory,
        cpu_ok: self.cpu_usage() < 0.8,
        error_rate_ok: self.error_rate() < 0.01,
    }
}
```

## Best Practices

1. **Use Compiled Validators**: Pre-compile for production
2. **Enable Caching**: Significant performance gains
3. **Monitor Resources**: Watch memory and CPU usage
4. **Handle Errors**: Never panic, always recover
5. **Profile Performance**: Use built-in profiler
6. **Security First**: Validate all inputs
7. **Test Thoroughly**: >90% coverage minimum

## Future Considerations

### Planned Enhancements

1. **WebAssembly Support**: Browser-side validation
2. **Distributed Caching**: Redis cluster support
3. **Schema Federation**: Multi-repository schemas
4. **Real-time Validation**: WebSocket support
5. **Machine Learning**: Intelligent error suggestions

### API Stability

- Core validation API: Stable
- Code generation API: Stable
- Migration tools: Beta
- IDE integration: Beta
- Custom validators: Stable

## Conclusion

The LinkML Service architecture prioritizes performance, reliability, and extensibility while maintaining compatibility with RootReal's service ecosystem. The multi-layered design allows for flexible deployment scenarios from single-node to distributed clusters.
