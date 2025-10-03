# LinkML Rust Implementation

[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-CC--BY--NC--4.0-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-2.0.0-green.svg)](CHANGELOG.md)

A high-performance, production-ready Rust implementation of [LinkML](https://linkml.io/) with 100% feature parity with the Python reference implementation. This library provides schema validation, code generation, and TypeQL generation capabilities with exceptional performance and safety guarantees.

## Features

- ✅ **100% Python LinkML Parity** - Complete feature compatibility with Python LinkML
- 🚀 **High Performance** - 126x faster TypeQL generation, 10x faster validation
- 🛡️ **Production Ready** - Comprehensive security audit, 500+ tests
- 🔧 **Multi-Language Code Generation** - Generate code for 10+ target languages
- 📊 **Advanced Validation** - Rules engine, boolean constraints, conditional requirements
- 🎯 **TypeQL Generator** - Exceptional performance (0.79ms for 100 classes)
- 💾 **Batch Processing** - Handle 100k+ records/second
- 🔒 **Secure** - Expression sandboxing, resource limits, injection protection
- 📦 **Modular Architecture** - Clean separation of concerns with core, service, and client crates

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
linkml-service = "2.0.0"
linkml-core = "2.0.0"
tokio = { version = "1.43", features = ["full"] }
serde_json = "1.0"
```

### Basic Usage

```rust
use linkml_service::{create_linkml_service, LinkMLService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the LinkML service
    let linkml = create_linkml_service().await?;

    // Load and validate a schema
    let schema = linkml.load_schema("person_schema.yaml").await?;

    // Validate data against the schema
    let data = serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    });

    let result = linkml.validate_data(&schema, &data, "Person").await?;

    if result.is_valid() {
        println!("✅ Data is valid!");
    } else {
        println!("❌ Validation errors: {:?}", result.errors());
    }

    Ok(())
}
```

### Generate TypeQL

```rust
use linkml_service::{create_linkml_service, LinkMLService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let linkml = create_linkml_service().await?;

    // Load schema
    let schema = linkml.load_schema("schema.yaml").await?;

    // Generate TypeQL
    let typeql = linkml.generate_typeql(&schema).await?;

    // Save to file
    std::fs::write("schema.tql", typeql)?;

    println!("✅ TypeQL schema generated!");

    Ok(())
}
```

## Project Structure

This project is organized into multiple crates for modularity and clarity:

```
linkml/
├── core/              # Core types, traits, and error definitions
├── service/           # Main LinkML validation and code generation service
├── client/            # Client library for interacting with LinkML services
├── build-tools/       # Build-time tools and cargo plugins
├── ide-plugins/       # IDE integration plugins
├── scripts/           # Utility scripts
├── schemas/           # Example LinkML schemas
└── docs/              # Comprehensive documentation
```

### Crates

- **linkml-core** - Core types, traits, and foundational functionality
- **linkml-service** - Main validation and code generation service
- **linkml-client** - Client library for LinkML services

## Documentation

Comprehensive documentation is available in the [`docs/`](docs/) directory:

- [**Getting Started**](docs/GETTING_STARTED.md) - Quick start guide
- [**User Guide**](docs/USER_GUIDE.md) - Comprehensive usage documentation
- [**Developer Guide**](docs/DEVELOPER_GUIDE.md) - For contributors and developers
- [**API Documentation**](docs/API.md) - Complete API reference
- [**Architecture**](docs/ARCHITECTURE.md) - System architecture and design
- [**Migration Guide**](docs/MIGRATION_FROM_PYTHON.md) - Migrating from Python LinkML
- [**Performance**](docs/PERFORMANCE.md) - Performance benchmarks and optimization
- [**Security**](docs/SECURITY.md) - Security features and best practices

## Performance

The Rust implementation offers significant performance improvements:

| Operation | Python LinkML | Rust LinkML | Speedup |
|-----------|--------------|-------------|---------|
| TypeQL Generation (100 classes) | 100ms | 0.79ms | **126x** |
| Validation (compiled) | 10ms | 1ms | **10x** |
| Batch Processing | 10k/sec | 100k+/sec | **10x** |
| Schema Loading | 50ms | 5ms | **10x** |

See [PERFORMANCE.md](docs/PERFORMANCE.md) for detailed benchmarks.

## Security

The LinkML Rust implementation includes comprehensive security features:

- Expression language sandboxing with resource limits
- Protection against ReDoS (Regular Expression Denial of Service)
- Input validation for all user data
- Secure file path handling
- No unsafe code in critical paths

See [SECURITY.md](docs/SECURITY.md) for the complete security audit.

## Feature Parity

This implementation achieves 100% feature parity with Python LinkML 1.x:

- ✅ Schema parsing (YAML/JSON)
- ✅ Complete validation (types, patterns, ranges, cardinality)
- ✅ Boolean constraints (exactly_one_of, any_of, all_of, none_of)
- ✅ Conditional requirements (if/then validation)
- ✅ Rules engine with complex validation logic
- ✅ Expression language with security sandboxing
- ✅ Pattern interpolation with named captures
- ✅ Unique key validation
- ✅ Code generation for 10+ languages
- ✅ TypeQL generation for TypeDB
- ✅ Schema migration with diff detection
- ✅ SchemaView for efficient schema navigation
- ✅ Compiled validators for performance

See [100_PERCENT_PARITY_ACHIEVED.md](docs/100_PERCENT_PARITY_ACHIEVED.md) for details.

## Examples

The [`service/examples/`](service/examples/) directory contains comprehensive examples:

- **basic_usage.rs** - Basic validation and schema loading
- **typeql_generation.rs** - TypeQL generation examples
- **code_generation.rs** - Multi-language code generation
- **batch_processing.rs** - High-throughput batch validation
- **custom_rules.rs** - Custom validation rules
- **migration.rs** - Schema migration and diffing

Run an example:

```bash
cargo run --example basic_usage
```

## Testing

The project includes extensive test coverage (500+ tests):

```bash
# Run all tests
cargo test --all-features

# Run unit tests only
cargo test --lib

# Run integration tests
cargo test --test integration_test

# Run with coverage
cargo tarpaulin --all-features --out Html
```

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Development Setup

1. **Clone the repository**:
   ```bash
   git clone https://github.com/simonckemper/rootreal.git
   cd rootreal/crates/model/symbolic/linkml
   ```

2. **Install Rust** (2024 edition required):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

3. **Build the project**:
   ```bash
   cargo build --all-features
   ```

4. **Run tests**:
   ```bash
   cargo test --all-features
   ```

5. **Check code quality**:
   ```bash
   cargo clippy --all-targets --all-features
   cargo fmt --all -- --check
   ```

## Versioning

This project follows [Semantic Versioning 2.0.0](https://semver.org/). See [CHANGELOG.md](CHANGELOG.md) for release history.

## License

This project is licensed under the **Creative Commons Attribution-NonCommercial 4.0 International License (CC-BY-NC-4.0)**.

You are free to:
- Share — copy and redistribute the material in any medium or format
- Adapt — remix, transform, and build upon the material

Under the following terms:
- **Attribution** — You must give appropriate credit
- **NonCommercial** — You may not use the material for commercial purposes

See [LICENSE](LICENSE) for the full license text.

## Authors

- Simon C. Kemper <textpast@textpast.com>

## Acknowledgments

- The [LinkML](https://linkml.io/) project and community
- The Rust community for excellent tools and libraries
- All contributors to this project

## Links

- **Repository**: https://github.com/simonckemper/rootreal
- **LinkML Website**: https://linkml.io/
- **LinkML Specification**: https://linkml.io/linkml-model/docs/
- **Issue Tracker**: https://github.com/simonckemper/rootreal/issues
- **RustDoc API**: [Generated on docs.rs]

## Status

**Current Version**: 2.0.0 (Production Ready)

This implementation is production-ready with comprehensive testing, security auditing, and performance optimization. It is actively maintained and used in production systems.

For questions, issues, or feature requests, please use the [GitHub issue tracker](https://github.com/simonckemper/rootreal/issues).
