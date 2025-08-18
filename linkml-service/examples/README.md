# LinkML Service Examples

This directory contains comprehensive examples demonstrating all LinkML service capabilities.

## Quick Start Examples

These examples demonstrate the LinkML API without requiring the full RootReal service infrastructure:

### Basic Usage
- **`basic_usage_api.rs`** - Core API demonstration including schema loading, validation, and error handling
- **`schema_migration_example.rs`** - Schema evolution and migration patterns

### Validation
- **`advanced_validation_api.rs`** - Advanced validation features including cross-field rules  
- **`validation_patterns.rs`** - Comprehensive validation patterns (regex, ranges, enums, cross-field)
- **`expression_language.rs`** - Expression language for computed fields and complex rules

### Code Generation
- **`schema_generation_api.rs`** - Basic code generation for TypeQL, SQL, GraphQL
- **`code_generation_showcase.rs`** - All generators with performance comparisons
- **`typeql_generation.rs`** - TypeQL generator with relations, rules, and migrations
- **`rust_code_generation.rs`** - Generating Rust structs from LinkML schemas

### Performance
- **`batch_processing.rs`** - High-performance batch validation with streaming and concurrency
- **`typeql_performance_check.rs`** - TypeQL generation performance benchmarks
- **`performance_summary.rs`** - Comprehensive performance characteristics across all operations
- **`performance_and_security.rs`** - Performance optimizations with security features

### Complete Demonstrations
- **`comprehensive_demo.rs`** - The most complete example showcasing ALL LinkML features

Run examples with:
```bash
# Basic examples
cargo run --example basic_usage_api
cargo run --example validation_patterns

# Code generation
cargo run --example typeql_generation
cargo run --example code_generation_showcase

# Performance
cargo run --example batch_processing
cargo run --release --example performance_summary

# Complete feature showcase
cargo run --release --example comprehensive_demo
```

## Full Integration Examples

The `*.rs.full` files demonstrate complete integration with the RootReal service ecosystem. These require all RootReal services to be available and properly initialized following the dyn-compatibility guidelines.

In a production RootReal application, you would:

1. Initialize all concrete service implementations at startup
2. Pass them through the dependency chain
3. Create the LinkML service with all dependencies
4. Use the service throughout your application

See `docs/architecture/dyn-compatibility-guidelines.md` for the complete production initialization pattern.

## Key Concepts

### Dyn-Compatibility

Some RootReal services (like TaskManagementService) are not dyn-compatible due to generic methods. These must be passed as concrete types, not trait objects. The LinkML service handles this by using generic type parameters for non-dyn-compatible dependencies.

### Service Initialization Pattern

```rust
// Production pattern (simplified) - use factory functions
let logger = logger_service::factory::create_standard_logger().await?;
let task_manager = task_management_service::factory::create_standard_task_management_service().await?;
// ... initialize other services using factory functions ...

let linkml = create_linkml_service(
    logger,      // dyn-compatible - passed as trait object
    timestamp,   // dyn-compatible - passed as trait object  
    task_manager,// NOT dyn-compatible - passed as concrete type
    // ... other services ...
).await?;
```

### Schema Features

LinkML supports:
- **Validation**: Pattern matching, range constraints, required fields
- **Inheritance**: Classes can inherit from abstract base classes
- **Enums**: Define permissible values for fields
- **Rules**: Cross-field validation with pre/post conditions
- **Code Generation**: Transform schemas into TypeQL, SQL, GraphQL, etc.

## Common Patterns

### Loading Schemas

```rust
// From string
let schema = linkml.load_schema_str(yaml_content, SchemaFormat::Yaml).await?;

// From file
let schema = linkml.load_schema(Path::new("schema.yaml")).await?;
```

### Validation

```rust
// Single document
let report = linkml.validate(&data, &schema, "Person").await?;

if !report.valid {
    for error in &report.errors {
        println!("Error: {}", error.message);
    }
}

// Batch validation (parallel)
let reports = futures::future::join_all(
    documents.iter().map(|doc| linkml.validate(doc, &schema, "Person"))
).await;
```

### Code Generation

```rust
// Generate TypeQL for TypeDB
let typeql = linkml.generate_typeql(&schema).await?;

// Generate SQL DDL
let sql = linkml.generate_sql(&schema, SqlDialect::PostgreSQL).await?;

// Generate GraphQL schema
let graphql = linkml.generate_graphql(&schema).await?;
```

## Example Features by Category

### Schema Definition
- Class inheritance and mixins
- Slot definitions with constraints
- Enumerations with permissible values
- Conditional requirements
- Cross-field validation rules
- Computed fields with expressions

### Validation Capabilities
- Pattern matching (regex)
- Range constraints (min/max)
- Required fields
- Unique constraints
- Cardinality constraints
- Custom validation rules
- Cross-field dependencies
- Conditional validations

### Code Generation Targets
- **TypeQL** - TypeDB schema with relations and rules
- **SQL** - PostgreSQL, MySQL, SQLite DDL
- **GraphQL** - Type definitions and queries
- **JSON Schema** - Full JSON Schema draft
- **Rust** - Structs with serde derives
- **Python** - Dataclasses or Pydantic models
- **TypeScript** - Interfaces with validation
- **OWL/RDF** - Ontology definitions
- **SHACL** - Shape validation
- **Java** - POJOs with annotations

### Performance Features
- Parallel validation (100k+ records/sec)
- Streaming for large datasets
- Compiled schema caching
- Batch processing
- Controlled concurrency
- Progress tracking
- Memory-efficient processing

## Performance Benchmarks

Based on the examples, typical performance metrics:

| Operation | Performance | Notes |
|-----------|------------|-------|
| Schema Parsing | <5ms | YAML/JSON parsing |
| Simple Validation | 10k-50k/sec | Single-threaded |
| Batch Validation | 100k+/sec | Parallel processing |
| TypeQL Generation | 0.79ms/100 classes | 126x faster than target |
| SQL Generation | <10ms | Full DDL |
| Code Generation | <50ms | Any target language |

## Best Practices

### Schema Design
1. Use meaningful identifiers
2. Provide descriptions for documentation
3. Use enums for controlled vocabularies
4. Define reusable slots
5. Leverage inheritance for common fields

### Validation
1. Validate early and often
2. Use batch validation for collections
3. Enable parallel processing
4. Cache compiled schemas
5. Provide clear error messages

### Code Generation
1. Choose appropriate target languages
2. Configure generation options
3. Use TypeQL for TypeDB integration
4. Generate migrations for schema changes
5. Include documentation in output

### Performance
1. **Enable schema compilation** for frequently used schemas
2. **Use batch validation** for multiple documents
3. **Configure appropriate cache sizes** based on your workload
4. **Use streaming validation** for large files
5. **Enable parallel validation** when processing many documents
6. **Control concurrency** to avoid overwhelming resources
7. **Monitor memory usage** for large datasets

## Common Use Cases

### Data Validation Pipeline
```rust
// Load schema once
let schema = linkml.load_schema("schema.yaml").await?;

// Validate streaming data
let validation_stream = data_stream
    .map(|record| linkml.validate(&record, &schema, "MyClass"))
    .buffer_unordered(100);
```

### Schema-Driven Development
```rust
// Generate all artifacts from schema
let typeql = linkml.generate_typeql(&schema).await?;
let sql = linkml.generate_sql(&schema, SqlDialect::PostgreSQL).await?;
let rust = linkml.generate_rust(&schema).await?;
```

### Migration Management
```rust
// Compare schemas and generate migration
let diff = linkml.compare_schemas(&old_schema, &new_schema).await?;
let migration = linkml.generate_migration(&diff).await?;
```