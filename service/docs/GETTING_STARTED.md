# Getting Started with RootReal LinkML Service

## Introduction

The RootReal LinkML Service provides a high-performance, production-ready implementation of LinkML validation and code generation for Rust applications. This guide will help you get started quickly.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Basic Concepts](#basic-concepts)
4. [Common Use Cases](#common-use-cases)
5. [Integration with RootReal](#integration-with-rootreal)
6. [Troubleshooting](#troubleshooting)

## Installation

Add the LinkML service to your `Cargo.toml`:

```toml
[dependencies]
linkml-service = { path = "../linkml/linkml-service" }
linkml-core = { path = "../linkml/linkml-core" }
tokio = { version = "1.43", features = ["full"] }
serde_json = "1.0"
```

## Quick Start

### 1. Create a Simple Schema

Create a file `person_schema.yaml`:

```yaml
id: https://example.org/person-schema
name: PersonSchema
description: A simple schema for person data

classes:
  Person:
    description: A person with basic information
    slots:
      - name
      - email
      - age

slots:
  name:
    description: Full name
    range: string
    required: true
    
  email:
    description: Email address
    range: string
    pattern: "^[\\w._%+-]+@[\\w.-]+\\.[a-zA-Z]{2,}$"
    
  age:
    description: Age in years
    range: integer
    minimum_value: 0
    maximum_value: 150
```

### 2. Validate Data

```rust
use linkml_service::{create_linkml_service, LinkMLService};
use linkml_core::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create service
    let service = create_linkml_service().await?;
    
    // Load schema
    let schema = service.load_schema_str(
        include_str!("person_schema.yaml"),
        SchemaFormat::Yaml
    ).await?;
    
    // Create data to validate
    let person_data = json!({
        "name": "Alice Smith",
        "email": "alice@example.com",
        "age": 30
    });
    
    // Validate
    let report = service.validate(&person_data, &schema, "Person").await?;
    
    if report.valid {
        println!("✓ Data is valid!");
    } else {
        println!("✗ Validation errors:");
        for error in &report.errors {
            println!("  - {}", error.message);
        }
    }
    
    Ok(())
}
```

## Basic Concepts

### Schemas

A LinkML schema defines:
- **Classes**: Types of objects (like Person, Organization)
- **Slots**: Properties/fields of classes
- **Types**: Custom data types with constraints
- **Enums**: Restricted sets of values

### Validation

Validation checks:
- Required fields are present
- Values match their declared types
- Patterns (regex) are satisfied
- Range constraints are met
- Enum values are permissible

### Code Generation

Generate code from schemas:
- TypeQL for TypeDB
- SQL DDL for databases
- GraphQL schemas
- Rust structs
- API documentation

## Common Use Cases

### 1. Validating API Input

```rust
use warp::Filter;
use serde::Deserialize;

#[derive(Deserialize)]
struct PersonInput {
    name: String,
    email: String,
    age: Option<i32>,
}

async fn validate_person(
    input: PersonInput,
    service: Arc<dyn LinkMLService>,
    schema: Arc<SchemaDefinition>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let data = serde_json::to_value(&input).unwrap();
    let report = service.validate(&data, &schema, "Person").await.unwrap();
    
    if report.valid {
        Ok(warp::reply::json(&input))
    } else {
        Ok(warp::reply::with_status(
            warp::reply::json(&report.errors),
            warp::http::StatusCode::BAD_REQUEST,
        ))
    }
}
```

### 2. Batch Data Processing

```rust
async fn process_csv_with_validation(
    csv_path: &str,
    service: Arc<dyn LinkMLService>,
    schema: Arc<SchemaDefinition>,
) -> Result<Vec<Person>, Box<dyn std::error::Error>> {
    let mut reader = csv::Reader::from_path(csv_path)?;
    let mut valid_records = Vec::new();
    let mut errors = Vec::new();
    
    for (index, result) in reader.deserialize().enumerate() {
        let record: Person = result?;
        let data = serde_json::to_value(&record)?;
        
        let report = service.validate(&data, &schema, "Person").await?;
        
        if report.valid {
            valid_records.push(record);
        } else {
            errors.push((index, report.errors));
        }
    }
    
    println!("Processed {} records, {} valid, {} invalid",
        valid_records.len() + errors.len(),
        valid_records.len(),
        errors.len()
    );
    
    Ok(valid_records)
}
```

### 3. Schema-Driven Database Tables

```rust
async fn create_database_schema(
    service: Arc<dyn LinkMLService>,
    schema: Arc<SchemaDefinition>,
    db_url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Generate SQL
    let sql = service.generate_sql(&schema, SqlDialect::PostgreSQL).await?;
    
    // Connect to database
    let pool = sqlx::PgPool::connect(db_url).await?;
    
    // Execute DDL
    sqlx::raw_sql(&sql).execute(&pool).await?;
    
    println!("Database schema created successfully");
    Ok(())
}
```

### 4. Type-Safe Code Generation

```rust
async fn generate_rust_types(
    service: Arc<dyn LinkMLService>,
    schema: Arc<SchemaDefinition>,
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let options = RustGenerationOptions {
        derive_traits: vec!["Debug", "Clone", "Serialize", "Deserialize"],
        use_builders: true,
        generate_validators: true,
        ..Default::default()
    };
    
    let rust_code = service.generate_rust(&schema, options).await?;
    
    std::fs::write(output_path, rust_code)?;
    
    println!("Generated Rust code at: {}", output_path);
    Ok(())
}
```

### 5. Dynamic Schema Loading

```rust
use std::collections::HashMap;

struct SchemaRegistry {
    service: Arc<dyn LinkMLService>,
    schemas: HashMap<String, Arc<SchemaDefinition>>,
}

impl SchemaRegistry {
    async fn load_schema(&mut self, name: &str, path: &str) -> Result<(), LinkMLError> {
        let schema = self.service.load_schema(path).await?;
        self.schemas.insert(name.to_string(), Arc::new(schema));
        Ok(())
    }
    
    async fn validate_with_schema(
        &self,
        schema_name: &str,
        data: &Value,
        class_name: &str,
    ) -> Result<ValidationReport, LinkMLError> {
        let schema = self.schemas.get(schema_name)
            .ok_or_else(|| LinkMLError::schema_not_found(schema_name))?;
        
        self.service.validate(data, schema, class_name).await
    }
}
```

## Integration with RootReal

### Using with RootReal Services

```rust
use rootreal_services::prelude::*;

async fn setup_linkml_with_rootreal() -> Result<Arc<dyn LinkMLService>, Box<dyn std::error::Error>> {
    // Get RootReal services using factory functions
    let logger = logger_service::factory::create_standard_logger().await?;
    let config = configuration_service::factory::create_standard_configuration_service().await?;
    let cache = cache_service::factory::create_valkey_cache_service().await?;
    
    // Configure LinkML
    let mut linkml_config = LinkMLServiceConfig::default();
    
    if let Some(enabled) = config.get("linkml.cache_enabled") {
        linkml_config.enable_caching = enabled == "true";
    }
    
    if let Some(size) = config.get("linkml.cache_size") {
        linkml_config.cache_size = size.parse()?;
    }
    
    // Create service
    let service = create_linkml_service_with_config(linkml_config).await?;
    
    logger.info("LinkML service initialized");
    
    Ok(Arc::new(service))
}
```

### Performance Monitoring

```rust
use rootreal_services::monitoring::MonitoringService;

async fn validate_with_monitoring(
    data: &Value,
    schema: &SchemaDefinition,
    class: &str,
    service: Arc<dyn LinkMLService>,
    monitoring: Arc<MonitoringService>,
) -> Result<ValidationReport, LinkMLError> {
    let start = std::time::Instant::now();
    
    let report = service.validate(data, schema, class).await?;
    
    let duration = start.elapsed();
    
    monitoring.record_metric("linkml.validation.duration_ms", duration.as_millis() as f64);
    monitoring.record_metric("linkml.validation.errors", report.errors.len() as f64);
    
    if report.valid {
        monitoring.increment_counter("linkml.validation.success");
    } else {
        monitoring.increment_counter("linkml.validation.failure");
    }
    
    Ok(report)
}
```

## Troubleshooting

### Common Issues

#### 1. Schema Not Found

```rust
// Problem: Import resolution fails
// Solution: Set import paths
let mut config = LinkMLServiceConfig::default();
config.import_paths.push(PathBuf::from("./schemas"));
config.import_paths.push(PathBuf::from("/usr/share/linkml"));
```

#### 2. Validation Timeout

```rust
// Problem: Complex regex causes timeout
// Solution: Increase timeout
let mut config = LinkMLServiceConfig::default();
config.validation_timeout = Duration::from_secs(60);
```

#### 3. Memory Usage

```rust
// Problem: Large schemas use too much memory
// Solution: Limit cache size
let mut config = LinkMLServiceConfig::default();
config.cache_size = 100; // Limit to 100 cached items
```

#### 4. Performance Issues

```rust
// Problem: Slow validation for large datasets
// Solution: Enable parallel validation
let mut config = LinkMLServiceConfig::default();
config.enable_parallel_validation = true;

// Also consider batching
let batch_size = 1000;
for chunk in data.chunks(batch_size) {
    // Process chunk in parallel
}
```

### Debug Output

Enable debug logging:

```rust
// Set environment variable
std::env::set_var("RUST_LOG", "linkml=debug");

// Or use tracing
tracing_subscriber::fmt()
    .with_env_filter("linkml=debug")
    .init();
```

### Validation Error Details

```rust
// Get detailed error information
let report = service.validate(&data, &schema, "Person").await?;

for error in &report.errors {
    eprintln!("Error Type: {}", error.error_type);
    eprintln!("Message: {}", error.message);
    eprintln!("Path: {}", error.path.as_deref().unwrap_or("root"));
    
    if let Some(expected) = &error.expected {
        eprintln!("Expected: {}", expected);
    }
    
    if let Some(actual) = &error.actual {
        eprintln!("Actual: {}", actual);
    }
}
```

## Next Steps

1. **Explore Examples**: Check out the `examples/` directory
2. **Read API Docs**: See the full [API documentation](API.md)
3. **Learn LinkML**: Visit [linkml.io](https://linkml.io) for schema design
4. **Join Community**: Contribute to RootReal LinkML development

## Resources

- [LinkML Documentation](https://linkml.io/linkml/)
- [RootReal Services Guide](../../README.md)
- [Performance Tuning Guide](PERFORMANCE.md)
- [Migration Guide](MIGRATION.md)
