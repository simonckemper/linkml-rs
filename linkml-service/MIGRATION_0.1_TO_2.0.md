# Migration Guide: LinkML Service 0.1.0 to 2.0.0

This guide helps you migrate from LinkML Service 0.1.0 (70% parity) to 2.0.0 (100% parity).

## Overview of Changes

### Major Additions
- Boolean constraints (exactly_one_of, any_of, all_of, none_of)
- Expression language for computed fields
- Rules engine for complex validation
- Enhanced TypeQL generator
- 10+ code generators
- Schema migration support
- Batch processing capabilities

### Breaking Changes
1. Service initialization requires more dependencies
2. Many methods are now async
3. Error types have been restructured
4. Some API methods have new signatures

## Step-by-Step Migration

### 1. Update Dependencies

```toml
# Cargo.toml
[dependencies]
linkml-service = "2.0.0"  # was 0.1.0
```

### 2. Update Service Initialization

**Before (0.1.0):**
```rust
let service = LinkMLService::new(
    logger.clone(),
    timestamp.clone(),
)?;
```

**After (2.0.0):**
```rust
let service = LinkMLService::new(
    logger.clone(),
    timestamp.clone(),
    task_manager.clone(),  // NEW: Required
    configuration.clone(), // NEW: Required
    cache.clone(),        // NEW: Required
    telemetry.clone(),    // NEW: Required
)?;
```

### 3. Update Async Methods

Many methods are now async for better performance:

**Before (0.1.0):**
```rust
let schema = service.parse_schema(content)?;
let report = service.validate(&data, &schema, "MyClass")?;
```

**After (2.0.0):**
```rust
let schema = service.parse_schema(content).await?;
let report = service.validate(&data, &schema, "MyClass").await?;
```

### 4. Update Error Handling

**Before (0.1.0):**
```rust
use linkml_service::error::LinkMLError;

match result {
    Err(LinkMLError::ValidationError(msg)) => { /* ... */ }
    // ...
}
```

**After (2.0.0):**
```rust
use linkml_service::error::{LinkMLError, ErrorKind};

match result {
    Err(LinkMLError { kind: ErrorKind::Validation(details), .. }) => {
        println!("Validation failed: {}", details.message);
    }
    // ...
}
```

### 5. New Features Usage

#### Boolean Constraints
```rust
// New in 2.0.0
let schema = r#"
classes:
  Config:
    exactly_one_of:
      - slot_group: database
        required: true
      - slot_group: file_storage
        required: true
"#;
```

#### Expression Language
```rust
// New in 2.0.0
let schema = r#"
slots:
  total_price:
    equals_expression: "base_price * (1 + tax_rate)"
"#;
```

#### Rules Engine
```rust
// New in 2.0.0
let schema = r#"
rules:
  - title: age_validation
    preconditions:
      expression: "age < 18"
    postconditions:
      expression: "guardian_required"
"#;
```

#### Code Generation
```rust
// New in 2.0.0
let typeql = service.generate_code(&schema, "typeql", &options).await?;
let sql = service.generate_code(&schema, "sql", &options).await?;
```

### 6. Performance Optimizations

Take advantage of new performance features:

```rust
// Batch validation (new in 2.0.0)
let reports = service.validate_batch(&documents, &schema, "MyClass").await?;

// Compiled validators (new in 2.0.0)
let compiled = service.compile_schema(&schema).await?;
let report = compiled.validate(&data, "MyClass")?;

// Streaming validation (new in 2.0.0)
let stream = service.validate_stream(data_stream, &schema, "MyClass");
```

## Common Migration Patterns

### Pattern 1: Simple Validation

**Before:**
```rust
fn validate_data(data: &Value) -> Result<bool> {
    let schema = load_schema()?;
    let report = service.validate(data, &schema, "Person")?;
    Ok(report.valid)
}
```

**After:**
```rust
async fn validate_data(data: &Value) -> Result<bool> {
    let schema = load_schema().await?;
    let report = service.validate(data, &schema, "Person").await?;
    Ok(report.valid)
}
```

### Pattern 2: Schema Loading

**Before:**
```rust
let yaml_parser = YamlParser::new();
let schema = yaml_parser.parse(content)?;
```

**After:**
```rust
let yaml_parser = YamlParser::new();
let schema = yaml_parser.parse_str(content)?; // Method renamed
```

### Pattern 3: Error Context

**Before:**
```rust
if !report.valid {
    for error in report.errors {
        println!("Error: {}", error);
    }
}
```

**After:**
```rust
if !report.valid {
    for error in &report.errors {
        println!("Error at {}: {}", 
                 error.field.as_ref().unwrap_or(&"root".to_string()),
                 error.message);
        if let Some(context) = &error.context {
            println!("  Context: {:?}", context);
        }
    }
}
```

## Deprecated Features

The following features from 0.1.0 are deprecated:

1. `validate_simple()` - Use `validate()` instead
2. `parse_schema_yaml()` - Use `YamlParser::parse_str()`
3. `generate_typeql_basic()` - Use full TypeQL generator

## New Best Practices

1. **Use batch operations** for multiple documents
2. **Enable schema compilation** for repeated validations
3. **Configure appropriate timeouts** for async operations
4. **Use streaming** for large datasets
5. **Enable parallel processing** where appropriate

## Troubleshooting

### Issue: Compilation errors after upgrade

**Solution**: Update all async method calls with `.await`

### Issue: Missing dependencies at runtime

**Solution**: Ensure all required services are initialized and passed to LinkMLService

### Issue: Performance regression

**Solution**: Enable compiled validators and batch processing

### Issue: Memory usage increased

**Solution**: Use streaming APIs for large datasets

## Getting Help

- Check the [examples](examples/) directory for usage patterns
- Read the [API documentation](https://docs.rs/linkml-service/2.0.0)
- File issues at [GitHub](https://github.com/simonckemper/rootreal/issues)

## Summary

The migration from 0.1.0 to 2.0.0 brings significant improvements:
- 100% Python LinkML feature parity
- 10x+ performance improvements
- Enhanced security
- Better error messages
- More code generation targets

While there are breaking changes, the migration effort is justified by the substantial improvements in functionality and performance.
