# LinkML Service Examples

This directory contains examples demonstrating the LinkML validation service, organized in a 3-level nested structure for optimal discoverability.

## Core Concept

The LinkML service in RootReal supports a powerful pattern where:
1. **Instance files** (like `ISO3166Entity.yaml`) define actual data records
2. **Schema classes** can reference these instances as their `range`
3. **Values are validated** against both patterns AND permissible values from instances

## Quick Start

### For Beginners
Start with the basic examples:
- [`01_basic/api/basic_usage_api.rs`](01_basic/api/basic_usage_api.rs) - Start here for core API usage
- [`01_basic/api/standalone_usage.rs`](01_basic/api/standalone_usage.rs) - Minimal dependencies example

### Run Any Example
```bash
# From repository root
cargo run --example <example_name>

# Example:
cargo run --example basic_usage_api
cargo run --example advanced_validation_api
```

## Directory Structure (3-Level Nesting)

```
examples/
├── 01_basic/
│   ├── api/              (2 files) - Core API examples
│   └── cli/              (2 files) - Command-line examples
├── 02_validation/
│   ├── api/              (2 files) - Validation API
│   ├── iso3166/          (3 files) - ISO 3166 validation
│   └── patterns/         (2 files) - Validation patterns
├── 03_data_loading/
│   ├── batch/            (3 files) - Batch processing
│   ├── csv/              (2 files) - CSV data
│   ├── database/         (2 files) - Database loading
│   └── excel/            (2 files) - Excel generation
├── 04_code_generation/
│   ├── database/         (2 files) - SQL/TypeQL generation
│   ├── general/          (4 files) - Generic code gen
│   ├── iso3166/          (2 files) - ISO-specific generation
│   └── rust/             (4 files) - Rust code generation
├── 05_visualization/
│   ├── diagrams/         (2 files) - Graphviz/Mermaid
│   ├── schemas/          (2 files) - Schema visualization
│   └── semantic_web/     (3 files) - RDF/semantic web
├── 06_integration/
│   ├── schemas/          (2 files) - Schema integration
│   ├── typedb/           (2 files) - TypeDB integration
│   └── full_integration.rs         - Complete integration
├── 07_performance/
│   ├── benchmarks/       (2 files) - Performance testing
│   └── security/         (2 files) - Security & performance
└── 08_advanced/
    ├── comprehensive/    (2 files) - Full feature demos
    ├── expressions/      (2 files) - Expression language
    └── extensibility/    (3 files) - Plugins & REPL
```

## Examples by Category

### 01. Basic Usage (4 examples)

**API Examples** (`01_basic/api/`)
- [`basic_usage_api.rs`](01_basic/api/basic_usage_api.rs) - Core API usage patterns
- [`standalone_usage.rs`](01_basic/api/standalone_usage.rs) - Minimal dependencies example

**CLI Examples** (`01_basic/cli/`)
- [`cli_usage.rs`](01_basic/cli/cli_usage.rs) - Command-line interface usage
- [`cli_with_fs_adapter.rs`](01_basic/cli/cli_with_fs_adapter.rs) - CLI with filesystem adapter

### 02. Validation (7 examples)

**API Validation** (`02_validation/api/`)
- [`advanced_validation_api.rs`](02_validation/api/advanced_validation_api.rs) - Advanced validation patterns
- [`test_instance_validation.rs`](02_validation/api/test_instance_validation.rs) - Instance-based validation

**ISO 3166 Validation** (`02_validation/iso3166/`)
- [`parse_and_validate_iso3166.rs`](02_validation/iso3166/parse_and_validate_iso3166.rs) - Parse and validate ISO codes
- [`validate_country_schema.rs`](02_validation/iso3166/validate_country_schema.rs) - Country schema validation
- [`validate_iso3166_standalone.rs`](02_validation/iso3166/validate_iso3166_standalone.rs) - Standalone ISO validation

**Validation Patterns** (`02_validation/patterns/`)
- [`validation_patterns.rs`](02_validation/patterns/validation_patterns.rs) - Common validation patterns
- [`validate_with_parse_service.rs`](02_validation/patterns/validate_with_parse_service.rs) - Integrated validation

### 03. Data Loading (9 examples)

**Batch Processing** (`03_data_loading/batch/`)
- [`array_support.rs`](03_data_loading/batch/array_support.rs) - Array data handling
- [`batch_processing.rs`](03_data_loading/batch/batch_processing.rs) - Batch data processing
- [`data_transformation.rs`](03_data_loading/batch/data_transformation.rs) - Data transformation pipelines

**CSV Loading** (`03_data_loading/csv/`)
- [`csv_data_loading.rs`](03_data_loading/csv/csv_data_loading.rs) - CSV file loading
- [`csv_introspection_demo.rs`](03_data_loading/csv/csv_introspection_demo.rs) - CSV schema inference

**Database Loading** (`03_data_loading/database/`)
- [`api_loading.rs`](03_data_loading/database/api_loading.rs) - Load schemas via API
- [`database_loading.rs`](03_data_loading/database/database_loading.rs) - Database integration

**Excel Generation** (`03_data_loading/excel/`)
- [`excel_generation.rs`](03_data_loading/excel/excel_generation.rs) - Basic Excel generation
- [`excel_generation_advanced.rs`](03_data_loading/excel/excel_generation_advanced.rs) - Advanced Excel features

### 04. Code Generation (12 examples)

**Database Code Generation** (`04_code_generation/database/`)
- [`sqlalchemy_generation.rs`](04_code_generation/database/sqlalchemy_generation.rs) - Generate SQLAlchemy models
- [`typeql_generation.rs`](04_code_generation/database/typeql_generation.rs) - Generate TypeQL schemas

**General Code Generation** (`04_code_generation/general/`)
- [`code_generation_showcase.rs`](04_code_generation/general/code_generation_showcase.rs) - Code generation overview
- [`generate_code_from_schema.rs`](04_code_generation/general/generate_code_from_schema.rs) - Schema-to-code generation
- [`generate_complete_example.rs`](04_code_generation/general/generate_complete_example.rs) - Complete generation example
- [`test_code_generation.rs`](04_code_generation/general/test_code_generation.rs) - Test code generation

**ISO 3166 Code Generation** (`04_code_generation/iso3166/`)
- [`generate_iso3166_code.rs`](04_code_generation/iso3166/generate_iso3166_code.rs) - ISO 3166 code generation
- [`generate_iso3166_simple.rs`](04_code_generation/iso3166/generate_iso3166_simple.rs) - Simplified ISO generation

**Rust Code Generation** (`04_code_generation/rust/`)
- [`rust_code_generation.rs`](04_code_generation/rust/rust_code_generation.rs) - Generate Rust structs
- [`test_rust_generator_comprehensive.rs`](04_code_generation/rust/test_rust_generator_comprehensive.rs) - Comprehensive Rust tests
- [`verify_rust_generator.rs`](04_code_generation/rust/verify_rust_generator.rs) - Verify Rust generator
- [`verify_rust_generator_fix.rs`](04_code_generation/rust/verify_rust_generator_fix.rs) - Rust generator fixes

### 05. Visualization (7 examples)

**Diagrams** (`05_visualization/diagrams/`)
- [`graphviz_generation.rs`](05_visualization/diagrams/graphviz_generation.rs) - Graphviz diagrams
- [`mermaid_generation.rs`](05_visualization/diagrams/mermaid_generation.rs) - Mermaid diagrams

**Schema Visualization** (`05_visualization/schemas/`)
- [`project_generation.rs`](05_visualization/schemas/project_generation.rs) - Project structure visualization
- [`schema_generation_api.rs`](05_visualization/schemas/schema_generation_api.rs) - Schema generation API

**Semantic Web** (`05_visualization/semantic_web/`)
- [`prefix_map_generation.rs`](05_visualization/semantic_web/prefix_map_generation.rs) - Prefix map visualization
- [`rdf_generation.rs`](05_visualization/semantic_web/rdf_generation.rs) - RDF output generation
- [`semantic_web_generators.rs`](05_visualization/semantic_web/semantic_web_generators.rs) - Semantic web formats

### 06. Integration (5 examples)

**Schema Integration** (`06_integration/schemas/`)
- [`page_xml_schema_inference.rs`](06_integration/schemas/page_xml_schema_inference.rs) - PAGE XML schema inference
- [`schema_migration_example.rs`](06_integration/schemas/schema_migration_example.rs) - Schema migration patterns

**TypeDB Integration** (`06_integration/typedb/`)
- [`typedb_integration.rs`](06_integration/typedb/typedb_integration.rs) - TypeDB integration
- [`typeql_performance_check.rs`](06_integration/typedb/typeql_performance_check.rs) - TypeQL performance testing

**Root Level**
- [`full_integration.rs`](06_integration/full_integration.rs) - Complete service integration

### 07. Performance (4 examples)

**Benchmarks** (`07_performance/benchmarks/`)
- [`performance_comparison.rs`](07_performance/benchmarks/performance_comparison.rs) - Performance benchmarking
- [`performance_summary.rs`](07_performance/benchmarks/performance_summary.rs) - Performance summary reports

**Security & Performance** (`07_performance/security/`)
- [`expression_performance_demo.rs`](07_performance/security/expression_performance_demo.rs) - Expression evaluation performance
- [`performance_and_security.rs`](07_performance/security/performance_and_security.rs) - Performance and security testing

### 08. Advanced (7 examples)

**Comprehensive Demos** (`08_advanced/comprehensive/`)
- [`comprehensive_demo.rs`](08_advanced/comprehensive/comprehensive_demo.rs) - Comprehensive feature demo
- [`comprehensive_usage.rs`](08_advanced/comprehensive/comprehensive_usage.rs) - Advanced usage patterns

**Expression Language** (`08_advanced/expressions/`)
- [`expression_functions_demo.rs`](08_advanced/expressions/expression_functions_demo.rs) - Expression language functions
- [`expression_language.rs`](08_advanced/expressions/expression_language.rs) - Expression language features

**Extensibility** (`08_advanced/extensibility/`)
- [`interactive_repl_demo.rs`](08_advanced/extensibility/interactive_repl_demo.rs) - Interactive REPL
- [`plugin_system_demo.rs`](08_advanced/extensibility/plugin_system_demo.rs) - Plugin architecture
- [`schema_builder_demo.rs`](08_advanced/extensibility/schema_builder_demo.rs) - Programmatic schema building

## The ISO 3166 Validation Example

### Instance File: ISO3166Entity.yaml

```yaml
- id: "US"
  label: "United States of America"
  tld: ".us"

- id: "GB"
  label: "United Kingdom"
  tld: ".gb"
```

### Schema: CountryCodeAlpha2Identifier

```yaml
CountryCodeAlpha2Identifier:
  slot_usage:
    identifier:
      range: ISO3166Entity  # References instance file
      structured_pattern:
        syntax: '(?P<CountryCodeAlpha2Identifier>[A-Z]{2})'
```

### Validation Rules

A value is valid if it:
1. **Matches the pattern**: Exactly 2 uppercase letters
2. **Exists in instances**: Must be an `id` from ISO3166Entity.yaml

### Examples

| Value | Pattern Match | Instance Match | Valid |
|-------|--------------|----------------|-------|
| "US"  | ✓            | ✓              | ✓     |
| "GB"  | ✓            | ✓              | ✓     |
| "XX"  | ✓            | ✗              | ✗     |
| "us"  | ✗            | -              | ✗     |

## Key Insights

1. **Separation of Concerns**: Patterns validate structure, instances validate semantics
2. **Reusability**: Instance files can be referenced by multiple schemas
3. **Maintainability**: Update instance files without changing schemas
4. **Type Safety**: Compile-time validation in Rust implementation

## Implementation Details

The LinkML service uses:
- **InstanceLoader**: Loads and caches instance data
- **InstanceValidator**: Validates against permissible values
- **PatternValidator**: Validates regex patterns
- **ValidationEngine**: Orchestrates all validators

## Customization

### Configuring Permissible Value Keys

By default, the service looks for permissible values in these fields:
1. `id` (highest priority)
2. `identifier`
3. `label`

Configure in `domain/schema/digital/linkml/instance/PermissibleValueKeys.yaml`

## Shared Utilities

The `common/` directory contains shared utilities used across examples:
- [`common/mod.rs`](common/mod.rs) - Common module exports
- [`common/service_init.rs`](common/service_init.rs) - Service initialization helpers

## Schemas

The `schema/` directory contains example schemas and configuration:
- [`schema/linkml-config-schema.yaml`](schema/linkml-config-schema.yaml) - LinkML configuration schema
- [`schema/default.yaml`](schema/default.yaml) - Default configuration
- [`schema/production.yaml`](schema/production.yaml) - Production configuration

## Navigation Tips

### By Use Case
- **Getting Started**: `01_basic/api/`
- **Validating Data**: `02_validation/`
- **Loading External Data**: `03_data_loading/`
- **Generating Code**: `04_code_generation/`
- **Visual Outputs**: `05_visualization/`
- **Complex Integrations**: `06_integration/`
- **Performance Tuning**: `07_performance/`
- **Advanced Features**: `08_advanced/`

### By Technology
- **ISO 3166**: `02_validation/iso3166/`, `04_code_generation/iso3166/`
- **TypeDB**: `06_integration/typedb/`
- **Rust Code Gen**: `04_code_generation/rust/`
- **CSV Processing**: `03_data_loading/csv/`
- **Excel**: `03_data_loading/excel/`
- **Semantic Web**: `05_visualization/semantic_web/`

## Troubleshooting

### Build Lock Issues

If you encounter "Blocking waiting for file lock":
```bash
# Kill any hanging cargo processes
pkill cargo

# Try again
cargo run --example <example_name>
```

### Missing Dependencies

The full examples require RootReal services. Use the standalone examples (`01_basic/api/standalone_usage.rs`) for testing without dependencies.

## Further Reading

- [Validation Architecture](../../../../domain/schema/docs/VALIDATION_ARCHITECTURE.md)
- [Legacy Python Docs](../../../../domain/schema/docs/legacy/)
- [LinkML Core Types](../../linkml-core/src/types.rs)
