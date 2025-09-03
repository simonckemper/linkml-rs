# LinkML Service API Guide

## Overview

The LinkML Service provides a comprehensive Rust implementation of LinkML (Linked Data Modeling Language) with full schema validation, code generation, and advanced features for biomedical and scientific data modeling.

## Quick Start

```rust
use linkml_service::factory::create_linkml_service;
use linkml_service::parser::yaml_parser::YamlParser;
use linkml_service::validator::ValidationEngine;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse a LinkML schema
    let parser = YamlParser::new();
    let schema = parser.parse_str(schema_yaml)?;
    
    // Create validation engine
    let engine = ValidationEngine::new(Arc::new(schema));
    
    // Validate data
    let result = engine.validate_instance(&data, "ClassName").await?;
    println!("Valid: {}", result.is_valid());
    
    Ok(())
}
```

## Core Components

### 1. Schema Parsing

#### YamlParser
Parses LinkML schemas from YAML format.

```rust
use linkml_service::parser::yaml_parser::YamlParser;

let parser = YamlParser::new();
let schema = parser.parse_str(yaml_content)?;
let schema_from_file = parser.parse_file("schema.yaml").await?;
```

**Features:**
- Full LinkML specification support
- Error reporting with line numbers
- Async file loading
- Memory-efficient streaming for large schemas

### 2. Data Validation

#### ValidationEngine
Core validation engine with advanced features.

```rust
use linkml_service::validator::{ValidationEngine, ValidationContext};

let engine = ValidationEngine::new(Arc::new(schema));

// Basic validation
let result = engine.validate_instance(&data, "Person").await?;

// Advanced validation with context
let mut context = ValidationContext::new(Arc::new(schema));
context.set_all_instances(all_data); // For cross-reference validation
context.set_current_instance_id("person:001".to_string());

let result = engine.validate_with_context(&data, "Person", &context).await?;
```

**Validation Features:**
- **Pattern Validation**: Regex pattern matching
- **Range Validation**: Min/max values for numbers
- **Cross-Reference Validation**: Validate references between objects
- **Semantic Validation**: Rule-based validation
- **Circular Reference Detection**: Prevent infinite loops
- **Enum Validation**: Permissible values checking
- **Required Field Validation**: Ensure mandatory fields

#### ValidationResult
Contains validation outcome and detailed error information.

```rust
if result.is_valid() {
    println!("Validation passed!");
} else {
    for issue in &result.issues {
        println!("Error at {}: {}", issue.path, issue.message);
        println!("Severity: {:?}", issue.severity);
    }
}
```

### 3. Code Generation

#### Python Dataclass Generator
Generates Python dataclasses with validation.

```rust
use linkml_service::generator::python_dataclass::PythonDataclassGenerator;
use linkml_service::generator::traits::Generator;

let generator = PythonDataclassGenerator::new();
let python_code = generator.generate(&schema)?;
```

**Generated Features:**
- Type annotations with `typing` module
- Field validation in `__post_init__`
- Support for `Optional`, `List`, `Set`, `Sequence`
- Enum generation
- Documentation strings

#### TypeScript Generator
Generates TypeScript interfaces and validators.

```rust
use linkml_service::generator::typescript::TypeScriptGenerator;

let generator = TypeScriptGenerator::new();
let typescript_code = generator.generate(&schema)?;
```

**Generated Features:**
- Advanced type annotations (`ReadonlyArray`, `Set`)
- Type guards (`is${ClassName}`)
- Validation functions with detailed error reporting
- JSDoc documentation
- Enum support

### 4. Data Loading

#### JSON V2 Loader
Intelligent JSON data loader with class inference.

```rust
use linkml_service::loader::json_v2::JsonV2Loader;

let mut loader = JsonV2Loader::new();
let instances = loader.load_str(&json_data, &schema).await?;
```

**Features:**
- Automatic class name inference from object structure
- Support for arrays and nested objects
- Metadata preservation
- Error handling with context

#### XML Loader
XML data loader with schema validation.

```rust
use linkml_service::loader::xml::XmlLoader;

let mut loader = XmlLoader::new();
let instances = loader.load_str(&xml_data, &schema).await?;
```

## Advanced Features

### Cross-Reference Validation

Enable validation of references between objects:

```rust
let mut context = ValidationContext::new(Arc::new(schema));
context.set_all_instances(vec![
    author_data,
    publication_data,
    journal_data
]);

// Validates that author references in publications exist
let result = engine.validate_with_context(&publication, "Publication", &context).await?;
```

### Performance Optimization

#### Buffer Pools
Reuse memory allocations for better performance:

```rust
use linkml_service::validator::buffer_pool::ValidationBufferPools;

let buffer_pools = Arc::new(ValidationBufferPools::new());
let context = ValidationContext::with_buffer_pools(schema, buffer_pools);
```

#### Batch Validation
Validate multiple instances efficiently:

```rust
let results = engine.validate_batch(&instances, "ClassName").await?;
```

### Custom Validators

Extend validation with custom logic:

```rust
use linkml_service::validator::validators::custom_validator::CustomValidator;

let mut validator = CustomValidator::new("my_validator");
validator.add_rule(|value, slot, schema, context| {
    // Custom validation logic
    vec![] // Return validation issues
});
```

## Configuration

### Generator Options
Customize code generation:

```rust
use linkml_service::generator::options::GeneratorOptions;

let mut options = GeneratorOptions::new();
options.include_docs = true;
options.set_custom("generate_validation", "true");
options.set_custom("generate_type_guards", "true");

let code = generator.generate_with_options(&schema, &options)?;
```

### Validation Options
Configure validation behavior:

```rust
use linkml_service::validator::ValidationOptions;

let options = ValidationOptions {
    strict_mode: true,
    enable_cross_references: true,
    max_recursion_depth: 10,
    collect_all_errors: true,
};

let result = engine.validate_with_options(&data, "Person", &options).await?;
```

## Error Handling

All operations return `Result` types with detailed error information:

```rust
match engine.validate_instance(&data, "Person").await {
    Ok(result) => {
        if result.is_valid() {
            println!("Validation successful");
        } else {
            for issue in &result.issues {
                eprintln!("Validation error: {}", issue.message);
            }
        }
    }
    Err(e) => {
        eprintln!("Engine error: {}", e);
    }
}
```

## Performance Guidelines

### Best Practices

1. **Reuse ValidationEngine**: Create once, use many times
2. **Use Buffer Pools**: For high-throughput scenarios
3. **Batch Operations**: Validate multiple instances together
4. **Schema Caching**: Cache parsed schemas
5. **Async Operations**: Use async/await for I/O operations

### Benchmarking

Run performance benchmarks:

```bash
cargo bench --bench linkml_benchmarks
```

Expected performance:
- Schema parsing: ~1000 schemas/second
- Validation: ~10,000 instances/second
- Code generation: ~100 schemas/second

## Integration Examples

### Web Service Integration

```rust
use axum::{Json, extract::State};
use linkml_service::validator::ValidationEngine;

async fn validate_data(
    State(engine): State<Arc<ValidationEngine>>,
    Json(data): Json<serde_json::Value>
) -> Result<Json<ValidationResult>, StatusCode> {
    let result = engine.validate_instance(&data, "Person").await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(result))
}
```

### Database Integration

```rust
use sqlx::PgPool;

async fn validate_and_store(
    pool: &PgPool,
    engine: &ValidationEngine,
    data: &serde_json::Value
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate first
    let result = engine.validate_instance(data, "Record").await?;
    if !result.is_valid() {
        return Err("Validation failed".into());
    }
    
    // Store in database
    sqlx::query("INSERT INTO records (data) VALUES ($1)")
        .bind(data)
        .execute(pool)
        .await?;
    
    Ok(())
}
```

## Testing

### Unit Tests
```rust
#[tokio::test]
async fn test_validation() {
    let schema = create_test_schema();
    let engine = ValidationEngine::new(Arc::new(schema));
    
    let valid_data = json!({"id": "test", "name": "Test"});
    let result = engine.validate_instance(&valid_data, "Person").await.unwrap();
    assert!(result.is_valid());
}
```

### Integration Tests
See `tests/real_world_schemas.rs` for comprehensive examples with biomedical and scientific schemas.

## Troubleshooting

### Common Issues

1. **Schema Parse Errors**: Check YAML syntax and LinkML specification compliance
2. **Validation Failures**: Review error messages and data structure
3. **Performance Issues**: Use buffer pools and batch operations
4. **Memory Usage**: Monitor for large schemas and data sets

### Debug Mode
Enable detailed logging:

```rust
env_logger::init();
// Set RUST_LOG=debug for detailed output
```

## Contributing

See the main repository for contribution guidelines. Key areas for contribution:
- Additional generators (Java, C#, etc.)
- New validation algorithms
- Performance optimizations
- Documentation improvements
