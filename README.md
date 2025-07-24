# LinkML for Rust

[![Crates.io](https://img.shields.io/crates/v/linkml.svg)](https://crates.io/crates/linkml)
[![Documentation](https://docs.rs/linkml/badge.svg)](https://docs.rs/linkml)
[![License](https://img.shields.io/badge/license-CC--BY--NC--4.0-blue.svg)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-500%2B-brightgreen.svg)](linkml-service/tests/)

A high-performance Rust implementation of the [LinkML](https://linkml.io) (Linked Data Modeling Language) specification with full Python parity, TypeQL generation for TypeDB, and comprehensive code generation capabilities.

## ✨ Features

### Core Capabilities
- **Full LinkML Specification Support** - 100% parity with Python LinkML
- **High Performance** - 10-100x faster than Python implementation
- **Memory Efficient** - Streaming support for large datasets
- **Type Safe** - Leverages Rust's type system for correctness

### Schema Features
- **Validation** - Complete validation with detailed error reporting
- **Inheritance** - Full support for mixins, abstract classes, and inheritance
- **Expressions** - Built-in expression language for computed values
- **Constraints** - Boolean constraints, conditional requirements, patterns
- **Imports** - Schema composition and modular design

### Code Generation
- **TypeQL** - Generate TypeDB schemas with full constraint support
- **SQL** - DDL generation for PostgreSQL, MySQL, SQLite
- **GraphQL** - Schema generation with resolvers
- **Rust** - Native Rust structs with serde support
- **Python** - Dataclasses, Pydantic models, SQLAlchemy
- **SHACL** - Generate SHACL shapes for RDF validation
- **20+ More** - Java, TypeScript, Go, Protobuf, JSON Schema, etc.

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
linkml = "2.0"
```

Or use individual components:

```toml
[dependencies]
linkml-core = "2.0"     # Core types and traits
linkml-service = "2.0"  # Full service implementation
```

## 🚀 Quick Start

### Basic Usage

```rust
use linkml::prelude::*;

// Load a schema
let schema_yaml = r#"
id: https://example.org/person
name: person_schema
prefixes:
  person: https://example.org/person/

classes:
  Person:
    attributes:
      name:
        required: true
        range: string
      age:
        range: integer
        minimum_value: 0
"#;

// Parse the schema
let schema = YamlParser::new().parse_str(schema_yaml)?;

// Validate data
let data = json!({
    "name": "Alice",
    "age": 30
});

let validator = Validator::new();
let result = validator.validate(&data, &schema, "Person")?;
assert!(result.is_valid());
```

### TypeQL Generation

```rust
use linkml::generator::TypeQLGenerator;

let generator = TypeQLGenerator::new();
let typeql = generator.generate(&schema, &Default::default())?;

// Output:
// define
// person:Person sub entity,
//   owns person:name,
//   owns person:age;
// person:name sub attribute, value string;
// person:age sub attribute, value long;
```

### Advanced Features

```rust
// Expression language
let schema_with_expressions = r#"
classes:
  Person:
    attributes:
      full_name:
        expression: "{first_name} {last_name}"
"#;

// Boolean constraints
let schema_with_constraints = r#"
classes:
  Document:
    rules:
      - exactly_one_of:
          - slot: doi
          - slot: isbn
          - slot: issn
"#;

// Pattern validation
let schema_with_patterns = r#"
types:
  EmailType:
    base: string
    pattern: "^[\\w\\.-]+@[\\w\\.-]+\\.\\w+$"
"#;
```

## 📚 Documentation

- **[Getting Started Guide](linkml-service/docs/GETTING_STARTED.md)** - Installation and basic usage
- **[User Guide](linkml-service/docs/USER_GUIDE.md)** - Comprehensive feature documentation
- **[API Reference](linkml-service/docs/API.md)** - Complete API documentation
- **[Examples](linkml-service/examples/)** - Working code examples
- **[Migration Guide](linkml-service/docs/MIGRATION.md)** - Migrating from Python LinkML

## 🛠️ Advanced Usage

### Custom Validators

```rust
use linkml::validator::{Validator, ValidatorPlugin};

struct MyCustomValidator;

impl ValidatorPlugin for MyCustomValidator {
    fn validate(&self, data: &Value, schema: &SchemaDefinition) -> ValidationResult {
        // Custom validation logic
    }
}

let validator = Validator::builder()
    .add_plugin(Box::new(MyCustomValidator))
    .build();
```

### Streaming Large Datasets

```rust
use linkml::stream::StreamValidator;

let validator = StreamValidator::new(schema);
let stream = BufReader::new(File::open("large_dataset.jsonl")?);

for result in validator.validate_stream(stream) {
    match result {
        Ok(report) => println!("Valid: {}", report.instance_id),
        Err(e) => eprintln!("Invalid: {}", e),
    }
}
```

### Integration with RootReal Services

When used within the RootReal ecosystem, LinkML integrates with various services:

```rust
// Example with RootReal services (optional dependencies)
use linkml::factory::LinkMLServiceFactory;

let factory = LinkMLServiceFactory::new()
    .with_cache_service(cache_service)
    .with_monitoring_service(monitoring_service)
    .build();

let service = factory.create_service().await?;
```

## 🎯 Performance

Benchmarks on AMD Ryzen 9 5950X:

| Operation | Performance | vs Python |
|-----------|------------|-----------|
| Schema Parsing | 0.5ms for 1000 classes | 50x faster |
| Validation | 100,000 records/sec | 100x faster |
| TypeQL Generation | 0.79ms for 100 classes | 126x faster |
| Memory Usage | 10MB for 1M records | 10x less |

## 🧩 Feature Flags

```toml
[dependencies]
linkml = { version = "2.0", features = ["full"] }
```

Available features:
- `default` - Core functionality
- `excel` - Excel generation support
- `graphql` - GraphQL schema generation
- `sql` - SQL DDL generation
- `typedb` - TypeDB/TypeQL support
- `full` - All features enabled

## 🤝 Contributing

This crate is part of the [RootReal](https://github.com/simonckemper/rootreal) project but is maintained as a standalone library for the LinkML community.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/simonckemper/linkml-rs.git
cd linkml-rs

# Run tests
cargo test

# Run benchmarks
cargo bench

# Build documentation
cargo doc --open
```

## 📄 License

This project is licensed under the Creative Commons Attribution-NonCommercial 4.0 International License (CC-BY-NC-4.0).

## 🙏 Acknowledgments

- The [LinkML](https://linkml.io) team for the specification
- The Rust community for excellent tooling
- RootReal project for supporting this implementation

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/simonckemper/linkml-rs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/simonckemper/linkml-rs/discussions)
- **Email**: simon.kemper@kempertech.com