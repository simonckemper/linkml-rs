# LinkML Service User Guide

## Table of Contents

1. [Introduction](#introduction)
2. [Installation](#installation)
3. [Command Line Interface](#command-line-interface)
4. [Configuration](#configuration)
5. [Common Tasks](#common-tasks)
6. [Troubleshooting](#troubleshooting)
7. [FAQ](#faq)

## Introduction

The RootReal LinkML Service provides schema validation and code generation for LinkML schemas. This guide covers everyday usage from a user perspective.

## Installation

### Prerequisites

- Rust 1.75+ (for building from source)
- 4GB RAM minimum (8GB recommended)
- 100MB disk space

### From Source

```bash
git clone https://github.com/simonckemper/rootreal.git
cd rootreal/crates/linkml/linkml-service
cargo build --release
```

### Adding to PATH

```bash
export PATH=$PATH:/path/to/rootreal/target/release
```

## Command Line Interface

### Basic Commands

#### Validate Data

```bash
linkml validate --schema person.yaml --data people.json
```

Options:
- `--schema, -s`: Path to LinkML schema file (YAML or JSON)
- `--data, -d`: Path to data file to validate
- `--class, -c`: Target class name (optional, auto-detected)
- `--format, -f`: Output format (text, json, junit)
- `--strict`: Fail on warnings

Example with all options:
```bash
linkml validate \
  --schema schemas/biolink.yaml \
  --data data/genes.json \
  --class Gene \
  --format junit \
  --strict > validation-report.xml
```

#### Check Schema

```bash
linkml check --schema myschema.yaml
```

Checks for:
- Syntax errors
- Import resolution
- Circular dependencies
- Undefined references
- Naming conflicts

#### Convert Schema Format

```bash
linkml convert --input schema.yaml --output schema.json
```

Supported conversions:
- YAML → JSON
- JSON → YAML

#### Generate Code

```bash
linkml generate --schema schema.yaml --target rust --output models.rs
```

Targets:
- `typeql`: TypeDB schema
- `sql`: SQL DDL (PostgreSQL)
- `graphql`: GraphQL schema
- `rust`: Rust structs
- `openapi`: OpenAPI specification
- `docs`: HTML documentation

Generator options:
```bash
# Rust with specific features
linkml generate \
  --schema person.yaml \
  --target rust \
  --output models.rs \
  --derive Debug,Clone,Serialize \
  --builders \
  --validators

# SQL with dialect
linkml generate \
  --schema database.yaml \
  --target sql \
  --output schema.sql \
  --dialect postgresql

# Documentation with template
linkml generate \
  --schema api.yaml \
  --target docs \
  --output docs/ \
  --template material
```

#### Performance Profiling

```bash
linkml profile --schema large_schema.yaml --iterations 1000
```

Output:
```
Schema Loading:     125ms (avg)
Compilation:        80ms (avg)
Validation:         0.5ms per record
Memory Usage:       45MB peak
Cache Hit Rate:     97.3%
```

#### Debug Mode

```bash
linkml debug --schema problematic.yaml --verbose
```

Shows:
- Import resolution steps
- Compilation process
- Validator selection
- Performance bottlenecks

#### Stress Testing

```bash
linkml stress \
  --schema schema.yaml \
  --records 100000 \
  --threads 8 \
  --duration 60s
```

Stress test options:
- `--records`: Number of test records
- `--threads`: Concurrent threads
- `--duration`: Test duration
- `--memory-limit`: Max memory usage
- `--report`: Output detailed report

### Interactive Mode

```bash
linkml interactive
```

Commands in interactive mode:
- `load <schema>`: Load a schema file
- `validate <data>`: Validate data
- `show classes`: List all classes
- `show slots`: List all slots
- `inspect <class>`: Show class details
- `tree <class>`: Show inheritance tree
- `search <pattern>`: Search schema elements
- `compile`: Compile validators
- `stats`: Show performance stats
- `help`: Show available commands
- `exit`: Exit interactive mode

Example session:
```
> load biolink.yaml
Schema loaded: biolink-model (3.5.0)

> show classes
Classes (150):
  - Entity
  - NamedThing
  - Gene
  - Disease
  - ...

> inspect Gene
Class: Gene
  Description: A region of DNA that encodes a functional RNA or protein
  Inherits from: GenomicEntity, BiologicalEntity
  Slots:
    - id (required)
    - name (required)
    - symbol
    - chromosome
    - start_position
    - end_position

> validate genes.json
Validating as Gene...
✓ 1000 records validated
  Valid: 998
  Invalid: 2
  
Show errors? (y/n): y
  Record 451: Missing required field 'name'
  Record 822: Invalid pattern for 'id' field
```

## Configuration

### Configuration File

Location: `~/.linkml/config.yaml`

```yaml
# Cache settings
cache:
  enabled: true
  size: 10000
  ttl_minutes: 60
  directory: ~/.linkml/cache

# Validation settings
validation:
  timeout_seconds: 30
  max_depth: 100
  strict_mode: false
  parallel: true
  batch_size: 1000

# Import paths
import_paths:
  - ./schemas
  - ~/.linkml/schemas
  - /usr/share/linkml/schemas

# Performance settings
performance:
  max_memory_mb: 500
  max_threads: 8
  compile_on_load: true

# Output settings
output:
  color: true
  format: text
  verbose: false

# Security settings
security:
  max_schema_size_mb: 10
  max_import_depth: 10
  allowed_protocols:
    - https
    - file
```

### Environment Variables

```bash
# Override config file location
export LINKML_CONFIG=/path/to/config.yaml

# Cache directory
export LINKML_CACHE_DIR=/var/cache/linkml

# Import paths (colon-separated)
export LINKML_IMPORT_PATH=/schemas:/usr/share/schemas

# Validation settings
export LINKML_VALIDATION_TIMEOUT=60
export LINKML_STRICT_MODE=true

# Performance settings
export LINKML_MAX_MEMORY=1024
export LINKML_PARALLEL=true

# Logging
export LINKML_LOG_LEVEL=debug
export LINKML_LOG_FILE=/var/log/linkml.log
```

## Common Tasks

### Validating API Input

```bash
# Validate incoming JSON against schema
linkml validate \
  --schema api/user.yaml \
  --data request.json \
  --class UserCreateRequest \
  --format json > response.json

# Check exit code
if [ $? -eq 0 ]; then
  echo "Valid request"
else
  echo "Invalid request"
  cat response.json
fi
```

### Batch Processing

```bash
# Validate all files in directory
for file in data/*.json; do
  echo "Validating $file..."
  linkml validate \
    --schema schema.yaml \
    --data "$file" \
    --format text
done

# Parallel validation
find data -name "*.json" | \
  xargs -P 8 -I {} linkml validate \
    --schema schema.yaml \
    --data {} \
    --format json > results.jsonl
```

### CI/CD Integration

```yaml
# GitHub Actions example
- name: Validate Data
  run: |
    linkml validate \
      --schema schemas/main.yaml \
      --data data/**/*.json \
      --strict \
      --format junit > test-results.xml

- name: Upload Test Results
  uses: actions/upload-artifact@v3
  with:
    name: validation-results
    path: test-results.xml
```

### Schema Migration

```bash
# Check for breaking changes
linkml migrate check \
  --old schemas/v1/schema.yaml \
  --new schemas/v2/schema.yaml

# Generate migration plan
linkml migrate plan \
  --old schemas/v1/schema.yaml \
  --new schemas/v2/schema.yaml \
  --output migration-plan.yaml

# Apply migration to data
linkml migrate apply \
  --plan migration-plan.yaml \
  --data old-data.json \
  --output new-data.json
```

### Performance Tuning

```bash
# Profile schema compilation
linkml profile \
  --schema complex-schema.yaml \
  --operation compile

# Warm cache before production
linkml cache warm --schema production.yaml

# Monitor cache effectiveness
linkml cache stats

# Clear cache if needed
linkml cache clear
```

## Troubleshooting

### Common Issues

#### Schema Not Found

```
Error: Failed to load schema: Schema not found: common/types.yaml
```

Solution:
```bash
# Add to import path
export LINKML_IMPORT_PATH=$LINKML_IMPORT_PATH:./common

# Or specify directly
linkml validate \
  --schema main.yaml \
  --import-path ./common \
  --data data.json
```

#### Validation Timeout

```
Error: Validation timeout after 30 seconds
```

Solution:
```bash
# Increase timeout
export LINKML_VALIDATION_TIMEOUT=120

# Or disable timeout
linkml validate \
  --schema schema.yaml \
  --data large-file.json \
  --no-timeout
```

#### Out of Memory

```
Error: Out of memory: heap allocation failed
```

Solution:
```bash
# Increase memory limit
export LINKML_MAX_MEMORY=2048

# Enable streaming for large files
linkml validate \
  --schema schema.yaml \
  --data huge-file.json \
  --streaming \
  --batch-size 100
```

#### Circular Import

```
Error: Circular import detected: a.yaml → b.yaml → c.yaml → a.yaml
```

Solution:
- Restructure schemas to avoid circular dependencies
- Use mixins instead of inheritance
- Extract common types to separate file

### Debug Techniques

#### Verbose Output

```bash
linkml validate \
  --schema schema.yaml \
  --data data.json \
  --verbose \
  --log-level debug 2> debug.log
```

#### Schema Inspection

```bash
# Show compiled schema
linkml debug show-compiled --schema schema.yaml

# Show inheritance tree
linkml debug tree --schema schema.yaml --class MyClass

# Find element
linkml debug find --schema schema.yaml --name "user_*"
```

#### Performance Analysis

```bash
# CPU profile
linkml profile \
  --schema schema.yaml \
  --cpu-profile profile.json

# Memory profile  
linkml profile \
  --schema schema.yaml \
  --memory-profile memory.json

# Analyze with external tools
go tool pprof -http=:8080 profile.json
```

## FAQ

### General Questions

**Q: What LinkML version is supported?**
A: The service supports LinkML schemas version 1.0 through 1.7, with 70% feature parity with Python LinkML.

**Q: Can I validate against multiple schemas?**
A: Yes, use imports in your schema or specify multiple schemas with `--schema`.

**Q: How do I validate streaming data?**
A: Use the `--streaming` flag for line-delimited JSON or CSV files.

**Q: What's the maximum schema size?**
A: Default is 10MB, configurable via `LINKML_MAX_SCHEMA_SIZE`.

### Performance Questions

**Q: How fast is validation?**
A: Typically 10,000-85,000 validations/second depending on schema complexity.

**Q: How much memory does it use?**
A: Base ~15MB + ~8MB per loaded schema + cache size.

**Q: Can I disable caching?**
A: Yes, use `--no-cache` or set `cache.enabled: false`.

**Q: How do I optimize for my use case?**
A: Run `linkml profile` to identify bottlenecks, then tune settings.

### Integration Questions

**Q: Can I use this in my Rust application?**
A: Yes, add `linkml-service` to your `Cargo.toml` and use the API.

**Q: Is there a REST API?**
A: Not built-in, but see examples for Axum/Warp integration.

**Q: Can I extend with custom validators?**
A: Yes, implement the `Validator` trait and register with the service.

**Q: How do I integrate with CI/CD?**
A: Use JUnit output format and appropriate exit codes.

### Troubleshooting Questions

**Q: Why is validation slow?**
A: Check if caching is enabled, schema is compiled, and batch size is appropriate.

**Q: Why do I get different results than Python LinkML?**
A: Check the [parity documentation](PARITY_EVALUATION.md) for known differences.

**Q: How do I report a bug?**
A: File an issue at https://github.com/simonckemper/rootreal/issues

**Q: Where can I get help?**
A: Check documentation, examples, or ask in discussions.

## Additional Resources

- [API Documentation](API.md)
- [Architecture Guide](ARCHITECTURE.md)
- [Developer Guide](DEVELOPER_GUIDE.md)
- [Migration Guide](MIGRATION.md)
- [Performance Tuning](PERFORMANCE.md)
- [Security Guide](SECURITY.md)
