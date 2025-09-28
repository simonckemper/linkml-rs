# LinkML Service API Documentation

## Overview

The RootReal LinkML Service provides a comprehensive API for schema validation, code generation, and data transformation. This document covers all public APIs with examples.

## Table of Contents

1. [Service Creation](#service-creation)
2. [Schema Operations](#schema-operations)
3. [Validation](#validation)
4. [Code Generation](#code-generation)
5. [Migration Tools](#migration-tools)
6. [Error Handling](#error-handling)
7. [Configuration](#configuration)

## Service Creation

### `create_linkml_service`

Creates a new LinkML service with default configuration.

```rust
use linkml_service::{create_linkml_service, LinkMLService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = create_linkml_service().await?;
    // Service is ready to use
    Ok(())
}
```

### `create_linkml_service_with_config`

Creates a new LinkML service with custom configuration.

```rust
use linkml_service::{create_linkml_service_with_config, LinkMLServiceConfig};
use std::time::Duration;

let config = LinkMLServiceConfig {
    enable_caching: true,
    cache_size: 1000,
    validation_timeout: Duration::from_secs(30),
    max_validation_depth: 10,
    enable_parallel_validation: true,
    strict_mode: false,
    ..Default::default()
};

let service = create_linkml_service_with_config(config).await?;
```

## Schema Operations

### Loading Schemas

#### `load_schema`

Load a schema from a file path.

```rust
use std::path::PathBuf;

let schema_path = PathBuf::from("schema.yaml");
let schema = service.load_schema(&schema_path).await?;

println!("Loaded schema: {}", schema.name);
println!("Version: {}", schema.version.unwrap_or_default());
```

#### `load_schema_str`

Load a schema from a string.

```rust
use linkml_core::traits::SchemaFormat;

let yaml_content = r#"
id: https://example.org/my-schema
name: MySchema
classes:
  Person:
    slots:
      - name
      - age
slots:
  name:
    range: string
    required: true
  age:
    range: integer
    minimum_value: 0
"#;

let schema = service.load_schema_str(yaml_content, SchemaFormat::Yaml).await?;
```

### Schema Introspection

#### Accessing Schema Components

```rust
// Iterate over classes
for (name, class_def) in &schema.classes {
    println!("Class: {}", name);
    if let Some(desc) = &class_def.description {
        println!("  Description: {}", desc);
    }
    println!("  Slots: {:?}", class_def.slots);
}

// Access specific slot
if let Some(slot) = schema.slots.get("name") {
    println!("Slot 'name' range: {:?}", slot.range);
    println!("Required: {:?}", slot.required.unwrap_or(false));
}

// Check enums
for (name, enum_def) in &schema.enums {
    println!("Enum {}: {} values", name, enum_def.permissible_values.len());
}
```

## Validation

### `validate`

Validate data against a schema and class.

```rust
use serde_json::json;

let data = json!({
    "name": "John Doe",
    "age": 30
});

let report = service.validate(&data, &schema, "Person").await?;

if report.valid {
    println!("✓ Validation passed");
} else {
    println!("✗ Validation failed with {} errors", report.errors.len());
    for error in &report.errors {
        println!("  - {}: {}", 
            error.path.as_deref().unwrap_or("root"),
            error.message
        );
    }
}
```

### Validation Report Structure

```rust
pub struct ValidationReport {
    /// Whether validation passed
    pub valid: bool,
    
    /// List of validation errors
    pub errors: Vec<ValidationIssue>,
    
    /// List of warnings
    pub warnings: Vec<ValidationIssue>,
    
    /// Additional information
    pub info: Vec<ValidationIssue>,
}

pub struct ValidationIssue {
    /// Error type identifier
    pub error_type: String,
    
    /// Human-readable message
    pub message: String,
    
    /// JSON path to the error
    pub path: Option<String>,
    
    /// Expected value/type
    pub expected: Option<String>,
    
    /// Actual value found
    pub actual: Option<String>,
}
```

### Batch Validation

```rust
let records = vec![
    json!({"name": "Alice", "age": 25}),
    json!({"name": "Bob", "age": 30}),
    json!({"name": "Charlie", "age": -5}), // Invalid
];

let mut results = Vec::new();
for record in &records {
    let report = service.validate(record, &schema, "Person").await?;
    results.push(report);
}

let valid_count = results.iter().filter(|r| r.valid).count();
println!("Valid records: {}/{}", valid_count, records.len());
```

### Parallel Validation

```rust
use futures::future::join_all;

let validation_futures: Vec<_> = records
    .iter()
    .map(|record| service.validate(record, &schema, "Person"))
    .collect();

let results = join_all(validation_futures).await;
```

## Code Generation

### `generate_typeql`

Generate TypeQL schema for TypeDB.

```rust
let typeql = service.generate_typeql(&schema).await?;
println!("Generated TypeQL:\n{}", typeql);

// Example output:
// define
// person sub entity,
//     owns name,
//     owns age;
// name sub attribute, value string;
// age sub attribute, value long;
```

### `generate_sql`

Generate SQL DDL with dialect options.

```rust
use linkml_core::generation::SqlDialect;

let sql = service.generate_sql(&schema, SqlDialect::PostgreSQL).await?;
println!("Generated SQL:\n{}", sql);

// Example output:
// CREATE TABLE person (
//     name TEXT NOT NULL,
//     age INTEGER CHECK (age >= 0)
// );
```

### `generate_graphql`

Generate GraphQL schema.

```rust
let graphql = service.generate_graphql(&schema).await?;
println!("Generated GraphQL:\n{}", graphql);

// Example output:
// type Person {
//     name: String!
//     age: Int
// }
```

### `generate_rust`

Generate Rust code with options.

```rust
use linkml_core::generation::RustGenerationOptions;

let options = RustGenerationOptions {
    derive_traits: vec!["Debug", "Clone", "Serialize", "Deserialize"],
    use_builders: true,
    generate_validators: true,
    ..Default::default()
};

let rust_code = service.generate_rust(&schema, options).await?;
```

### `generate_openapi`

Generate OpenAPI specification.

```rust
use linkml_core::generation::OpenApiOptions;

let options = OpenApiOptions {
    title: "My API".to_string(),
    version: "1.0.0".to_string(),
    base_path: "/api/v1".to_string(),
    include_crud_operations: true,
    ..Default::default()
};

let openapi = service.generate_openapi(&schema, options).await?;
```

## Migration Tools

### `analyze_changes`

Analyze breaking changes between schema versions.

```rust
let old_schema = service.load_schema(&old_path).await?;
let new_schema = service.load_schema(&new_path).await?;

let changes = service.analyze_changes(&old_schema, &new_schema).await?;

for change in &changes {
    println!("Breaking change: {} - {}", change.element, change.description);
    match &change.migration_strategy {
        MigrationStrategy::Automatic { transform } => {
            println!("  Auto-migration available: {}", transform);
        }
        MigrationStrategy::Manual { instructions } => {
            println!("  Manual migration needed: {}", instructions);
        }
        _ => {}
    }
}
```

### `generate_migration`

Generate migration scripts between versions.

```rust
use linkml_core::migration::{MigrationOptions, MigrationLanguage};

let options = MigrationOptions {
    target_language: MigrationLanguage::SQL,
    include_data_migration: true,
    safe_mode: true,
    ..Default::default()
};

let migration_script = service.generate_migration(
    &old_schema,
    &new_schema,
    options
).await?;
```

## Error Handling

All operations return `Result<T, LinkMLError>`.

```rust
use linkml_core::error::{LinkMLError, ErrorKind};

match service.load_schema(&path).await {
    Ok(schema) => {
        // Success
    }
    Err(LinkMLError { kind, message, source, .. }) => {
        match kind {
            ErrorKind::Io => eprintln!("IO error: {}", message),
            ErrorKind::Parse => eprintln!("Parse error: {}", message),
            ErrorKind::Validation => eprintln!("Validation error: {}", message),
            _ => eprintln!("Error: {}", message),
        }
    }
}
```

## Configuration

### LinkMLServiceConfig

```rust
pub struct LinkMLServiceConfig {
    /// Enable caching for schemas and validations
    pub enable_caching: bool,
    
    /// Maximum number of cached items
    pub cache_size: usize,
    
    /// Timeout for validation operations
    pub validation_timeout: Duration,
    
    /// Maximum recursion depth for validation
    pub max_validation_depth: usize,
    
    /// Enable parallel validation
    pub enable_parallel_validation: bool,
    
    /// Strict mode (fail on warnings)
    pub strict_mode: bool,
    
    /// Custom validators to register
    pub custom_validators: Vec<Box<dyn Validator>>,
    
    /// Import search paths
    pub import_paths: Vec<PathBuf>,
}
```

### Environment Variables

The service respects these environment variables:

- `LINKML_CACHE_ENABLED` - Enable/disable caching
- `LINKML_CACHE_SIZE` - Maximum cache entries
- `LINKML_VALIDATION_TIMEOUT` - Timeout in seconds
- `LINKML_IMPORT_PATH` - Colon-separated import paths
- `LINKML_STRICT_MODE` - Enable strict validation

## Advanced Usage

### Custom Validators

```rust
use linkml_core::validation::{Validator, ValidationContext};

struct MyCustomValidator;

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
        // Custom validation logic
        Ok(ValidationResult::valid())
    }
}

// Register with service
let mut config = LinkMLServiceConfig::default();
config.custom_validators.push(Box::new(MyCustomValidator));
```

### Performance Monitoring

```rust
use std::time::Instant;

let start = Instant::now();
let report = service.validate(&data, &schema, "Person").await?;
let duration = start.elapsed();

println!("Validation took {:?}", duration);
println!("Cache hit rate: {:.1}%", service.cache_hit_rate() * 100.0);
```

### Streaming Validation

For large datasets:

```rust
use tokio::sync::mpsc;
use futures::StreamExt;

let (tx, mut rx) = mpsc::channel(100);

// Producer task
tokio::spawn(async move {
    for record in large_dataset {
        tx.send(record).await.unwrap();
    }
});

// Consumer task
while let Some(record) = rx.recv().await {
    let report = service.validate(&record, &schema, "Person").await?;
    if !report.valid {
        eprintln!("Invalid record: {:?}", record);
    }
}
```

## Best Practices

1. **Reuse Service Instances**: Create one service instance and reuse it
2. **Enable Caching**: For repeated validations, enable caching
3. **Use Parallel Validation**: For batch processing, enable parallel mode
4. **Handle Errors Gracefully**: Always check validation reports
5. **Monitor Performance**: Track cache hits and validation times

## Examples

See the `examples/` directory for complete examples:
- `basic_usage.rs` - Simple validation example
- `advanced_validation.rs` - Complex constraints and patterns
- `schema_generation.rs` - Code generation examples
- `rootreal_integration.rs` - Integration with RootReal services
