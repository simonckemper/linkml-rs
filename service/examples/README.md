# LinkML Service Examples

This directory contains examples demonstrating the LinkML validation service, with a focus on instance-based permissible value validation.

## Core Concept

The LinkML service in RootReal supports a powerful pattern where:
1. **Instance files** (like `ISO3166Entity.yaml`) define actual data records
2. **Schema classes** can reference these instances as their `range`
3. **Values are validated** against both patterns AND permissible values from instances

## Examples

### 1. validate_iso3166_standalone.rs

**Purpose**: Demonstrates the validation principle without requiring all RootReal dependencies.

**Key Features**:
- Loads ISO3166Entity instances manually
- Shows pattern validation vs. permissible value validation
- Explains how the two work together

**Run**: `cargo run --example validate_iso3166_standalone -p linkml-service`

### 2. validate_country_schema.rs

**Purpose**: Comprehensive example showing full LinkML validation workflow.

**Key Features**:
- Validates ISO3166Entity instances against their schema
- Tests CountryCodeAlpha2Identifier validation
- Demonstrates pattern matching with named capture groups
- Shows compound identifier validation

**Run**: Requires full RootReal service dependencies

### 3. test_instance_validation.rs

**Purpose**: Focused test of the instance-based validation mechanism.

**Key Features**:
- Loads schemas and instances
- Validates test values
- Shows both valid and invalid cases
- Explains the validation rules

**Run**: Requires full RootReal service dependencies

## The ISO 3166 Example

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

## Running the Examples

### Prerequisites

1. RootReal project cloned
2. Rust toolchain installed
3. Domain schema files present at `/home/kempersc/apps/rootreal/domain/schema/`

### Quick Test

```bash
# Run the standalone example (no dependencies)
cargo run --example validate_iso3166_standalone -p linkml-service
```

### Full Integration Test

```bash
# Requires all RootReal services
cargo run --example validate_country_schema -p linkml-service
```

## Troubleshooting

### Build Lock Issues

If you encounter "Blocking waiting for file lock":
```bash
# Kill any hanging cargo processes
pkill cargo

# Try again
cargo run --example validate_iso3166_standalone -p linkml-service
```

### Missing Dependencies

The full examples require RootReal services. Use the standalone example for testing without dependencies.

## Further Reading

- [Validation Architecture](../../../../domain/schema/docs/VALIDATION_ARCHITECTURE.md)
- [Legacy Python Docs](../../../../domain/schema/docs/legacy/)
- [LinkML Core Types](../../linkml-core/src/types.rs)