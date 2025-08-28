# LinkML Service Performance Guide

## Overview

This guide covers performance characteristics, optimization techniques, and tuning recommendations for the RootReal LinkML Service.

## Performance Characteristics

### Baseline Performance

| Operation | Performance | Notes |
|-----------|------------|-------|
| Schema Loading | 50-200ms | Depends on imports |
| Schema Compilation | <100ms | Complex schemas |
| Simple Validation | 0.1-0.5ms | Single record |
| Complex Validation | 1-5ms | With patterns/constraints |
| Batch Validation | 10,000-85,000/sec | Varies by complexity |
| Code Generation | 10-500ms | Target dependent |

### Memory Usage

| Component | Memory Usage | Notes |
|-----------|-------------|-------|
| Base Service | ~15MB | Without schemas |
| Per Schema | ~5-20MB | Typical schemas |
| Compiled Validators | ~1-5MB | Per schema |
| L1 Cache | ~50MB | 1000 entries |
| L2 Cache | ~200MB | 10,000 entries |

### Scaling Characteristics

- **CPU Scaling**: Near-linear up to 8 cores
- **Memory Scaling**: O(n) with schema count
- **Cache Efficiency**: >95% hit rate after warmup
- **Concurrent Operations**: Handles 1000+ concurrent validations

## Performance Optimization

### Schema Optimization

#### 1. Minimize Import Depth

```yaml
# ❌ Deep import chains
# a.yaml imports b.yaml imports c.yaml imports d.yaml

# ✅ Flatter structure
# main.yaml imports [types.yaml, constraints.yaml, enums.yaml]
```

#### 2. Use Specific Imports

```yaml
# ❌ Import everything
imports:
  - linkml:types

# ✅ Import only needed elements
imports:
  - linkml:types.string
  - linkml:types.integer
```

#### 3. Optimize Regex Patterns

```yaml
# ❌ Complex nested patterns
pattern: "^(?:(?:(?:0?[13578]|1[02])(\/|-|\.)31)|(?:(?:0?[1,3-9]|1[0-2])(\/|-|\.)(?:29|30)))(\/|-|\.)(?:[1-9]\d{3}|\d{2})$"

# ✅ Simpler patterns or use types
range: date
```

### Validation Optimization

#### 1. Enable Compilation

```rust
// Always compile schemas for production
let config = LinkMLServiceConfig {
    compile_on_load: true,
    ..Default::default()
};
```

#### 2. Batch Processing

```rust
// ❌ Individual validations
for record in records {
    let report = service.validate(&record, &schema, "Person").await?;
}

// ✅ Batch validation
let reports = service.validate_batch(&records, &schema, "Person").await?;
```

#### 3. Parallel Validation

```rust
use futures::stream::{FuturesUnordered, StreamExt};

let mut futures = FuturesUnordered::new();

for chunk in records.chunks(1000) {
    futures.push(validate_chunk(chunk));
}

while let Some(result) = futures.next().await {
    process_result(result?);
}
```

### Caching Strategy

#### 1. Cache Configuration

```yaml
cache:
  # L1 - Hot data
  l1_size: 1000
  l1_ttl_minutes: 5
  
  # L2 - Warm data
  l2_size: 10000
  l2_ttl_minutes: 60
  
  # L3 - Cold data (external)
  l3_enabled: true
  l3_ttl_hours: 24
```

#### 2. Cache Warming

```rust
// Pre-warm cache with common schemas
let common_schemas = ["person.yaml", "organization.yaml", "product.yaml"];

for schema_path in common_schemas {
    service.load_schema(schema_path).await?;
}

// Pre-compile validators
service.compile_all_validators().await?;
```

#### 3. Cache Key Optimization

```rust
// Use efficient cache keys
let key = format!(
    "v:{}:{}:{}",
    schema_hash,    // xxHash3 of schema
    data_hash,      // xxHash3 of data
    class_name      // Target class
);
```

### Memory Optimization

#### 1. String Interning

```rust
// Intern common strings
lazy_static! {
    static ref INTERNED: StringInterner = {
        let mut interner = StringInterner::new();
        interner.intern("id");
        interner.intern("name");
        interner.intern("type");
        interner
    };
}
```

#### 2. Arena Allocation

```rust
// Use arena for temporary objects
let arena = Arena::new();
let validator = arena.alloc(create_validator());
```

#### 3. Buffer Reuse

```rust
// Reuse buffers for parsing
thread_local! {
    static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(4096));
}
```

## Profiling and Monitoring

### Built-in Profiler

```bash
# Profile specific operation
linkml profile --schema complex.yaml --operation validate --iterations 10000

# Output
Operation: validate
Iterations: 10,000
Total time: 1.234s
Avg time: 0.123ms
Min time: 0.089ms
Max time: 0.456ms
P50: 0.112ms
P95: 0.178ms
P99: 0.234ms
```

### Performance Metrics

```rust
// Key metrics to monitor
linkml.validation.duration_ms       // Validation time
linkml.compilation.duration_ms      // Compilation time
linkml.cache.hit_rate              // Cache effectiveness
linkml.memory.heap_bytes           // Memory usage
linkml.cpu.usage_percent           // CPU utilization
linkml.concurrent_operations       // Active operations
```

### Custom Profiling

```rust
use std::time::Instant;

let start = Instant::now();
let _guard = defer(|| {
    let duration = start.elapsed();
    metrics.record("custom.operation", duration.as_millis() as f64);
});

// Operation to profile
perform_operation().await?;
```

## Performance Tuning

### For High Throughput

```yaml
# Configuration for high throughput
validation:
  parallel: true
  batch_size: 1000
  timeout_seconds: 5

cache:
  l1_size: 5000
  compile_on_load: true

performance:
  max_threads: 16
  max_memory_mb: 2048
```

### For Low Latency

```yaml
# Configuration for low latency
validation:
  parallel: false  # Avoid thread overhead
  batch_size: 1    # Process immediately

cache:
  l1_size: 10000  # Larger L1 cache
  compile_on_load: true

performance:
  max_threads: 4   # Fewer threads
  cpu_affinity: true
```

### For Memory Constrained

```yaml
# Configuration for limited memory
cache:
  l1_size: 100
  l2_size: 0       # Disable L2
  l3_enabled: false # Disable L3

validation:
  streaming: true  # Stream large files
  batch_size: 10

performance:
  max_memory_mb: 256
  gc_interval: 100
```

## Benchmarking

### Micro-benchmarks

```rust
#[bench]
fn bench_validate_simple(b: &mut Bencher) {
    let service = create_service();
    let schema = load_schema("simple.yaml");
    let data = json!({"name": "test"});
    
    b.iter(|| {
        black_box(service.validate(&data, &schema, "Person"))
    });
}
```

### Load Testing

```bash
# Generate load test data
linkml generate-test-data --schema schema.yaml --count 100000 > test-data.json

# Run load test
linkml stress \
  --schema schema.yaml \
  --data test-data.json \
  --duration 60s \
  --threads 8 \
  --report load-test-report.html
```

### Comparison Benchmarks

```bash
# Compare with Python LinkML
./scripts/benchmark-comparison.sh

# Output
Operation         | Python LinkML | RootReal LinkML | Speedup
-----------------|---------------|-----------------|--------
Schema Load      | 523ms         | 87ms            | 6.0x
Simple Validate  | 2.3ms         | 0.12ms          | 19.2x
Complex Validate | 8.7ms         | 0.89ms          | 9.8x
Batch (10k)      | 18.2s         | 1.3s            | 14.0x
```

## Common Performance Issues

### Issue: Slow Schema Loading

**Symptoms**: Schema takes >1s to load

**Solutions**:
1. Check import paths - use local paths
2. Reduce import depth
3. Cache compiled schemas
4. Use schema bundles

### Issue: High Memory Usage

**Symptoms**: Memory grows unbounded

**Solutions**:
1. Limit cache sizes
2. Enable streaming for large files
3. Use weak references
4. Monitor for leaks

### Issue: CPU Spikes

**Symptoms**: 100% CPU usage

**Solutions**:
1. Enable rate limiting
2. Use async operations
3. Batch small operations
4. Profile hot paths

### Issue: Cache Misses

**Symptoms**: <80% cache hit rate

**Solutions**:
1. Increase cache size
2. Adjust TTL values
3. Implement cache warming
4. Use predictive caching

## Performance Best Practices

1. **Measure First**: Always profile before optimizing
2. **Cache Aggressively**: Use all cache layers
3. **Batch Operations**: Reduce overhead
4. **Compile Schemas**: Always for production
5. **Monitor Metrics**: Track key indicators
6. **Test Under Load**: Realistic conditions
7. **Tune for Use Case**: Different configs for different needs
8. **Update Regularly**: Performance improvements in updates

## Advanced Techniques

### SIMD Optimization

```rust
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

unsafe fn validate_batch_simd(values: &[u8]) -> bool {
    // SIMD validation for specific patterns
}
```

### Custom Memory Allocator

```rust
#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
```

### Lock-Free Data Structures

```rust
use crossbeam::queue::SegQueue;

let queue: SegQueue<ValidationTask> = SegQueue::new();
```

## Conclusion

The LinkML Service is designed for high performance with multiple optimization levels. Start with default settings and tune based on profiling results and specific requirements.
