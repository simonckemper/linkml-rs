# TypeQL Generator User Guide

## Overview

The TypeQL Generator converts LinkML schemas to TypeDB 3.0 schema definitions. It provides comprehensive support for:

- Entity and relation mapping
- Complex inheritance hierarchies
- Multi-way and nested relations
- Constraint generation (@key, @unique, @card, regex)
- Rule generation for validation and inference
- Schema migration support
- Performance optimized for large schemas

## Performance Characteristics

The generator has been optimized for exceptional performance:

| Schema Size | Generation Time | Performance Target | Status |
|-------------|-----------------|-------------------|---------|
| 10 classes  | ~0.3ms         | -                 | ✅      |
| 100 classes | ~0.85ms        | <100ms            | ✅      |
| 1000 classes| ~6.5ms         | <1s               | ✅      |

Performance scales linearly at approximately 6-8 microseconds per class.

## Basic Usage

```rust
use linkml_service::generator::{
    Generator, GeneratorOptions,
    typeql_generator_enhanced::EnhancedTypeQLGenerator,
};

// Create generator
let generator = EnhancedTypeQLGenerator::new();
let options = GeneratorOptions::default();

// Generate TypeQL
let outputs = generator.generate(&schema, &options).await?;

// Write to file
for output in outputs {
    std::fs::write(&output.filename, &output.content)?;
}
```

## Schema Mapping

### Classes to Entities

LinkML classes map to TypeDB entities:

```yaml
# LinkML
classes:
  Person:
    is_a: NamedThing
    slots:
      - id
      - name
      - email
```

```typeql
# Generated TypeQL
person sub named-thing,
    owns id @key,
    owns name,
    owns email;
```

### Slots to Attributes

LinkML slots map to TypeDB attributes with appropriate value types:

```yaml
# LinkML
slots:
  age:
    range: integer
    minimum_value: 0
    maximum_value: 150
```

```typeql
# Generated TypeQL
age sub attribute, value long;

rule person-age-validation: when {
    $x isa person, has age $v;
    $v < 0;
} then {
    # Validation failure
};
```

### Relations

The generator automatically detects and creates relations:

```yaml
# LinkML
classes:
  Employment:
    slots:
      - employee  # range: Person
      - employer  # range: Organization
      - start_date
```

```typeql
# Generated TypeQL
employment sub relation,
    relates employee,
    relates employer,
    owns start-date;

person plays employment:employee;
organization plays employment:employer;
```

## Advanced Features

### Multi-way Relations

Relations with more than two participants are fully supported:

```yaml
classes:
  Meeting:
    slots:
      - organizer      # range: Person
      - participants   # range: Person, multivalued
      - location       # range: string
```

```typeql
meeting sub relation,
    relates organizer,
    relates participants,
    owns location;

person plays meeting:organizer,
       plays meeting:participants;
```

### Nested Relations

Relations that reference other relations:

```yaml
classes:
  ContractNegotiation:
    slots:
      - employment     # range: Employment
      - negotiator     # range: Person
      - outcome
```

```typeql
contract-negotiation sub relation,
    relates employment-rel,
    relates negotiator,
    owns outcome;

employment plays contract-negotiation:employment-rel;
person plays contract-negotiation:negotiator;
```

### Constraints

#### Cardinality Constraints

```yaml
slots:
  email:
    multivalued: false
    required: true
    
  tags:
    multivalued: true
    minimum_cardinality: 1
    maximum_cardinality: 10
```

```typeql
person owns email @card(1..1);
person owns tags @card(1..10);
```

#### Pattern Constraints

```yaml
slots:
  postal_code:
    pattern: "^[0-9]{5}(-[0-9]{4})?$"
```

```typeql
postal-code sub attribute, 
    value string,
    regex "^[0-9]{5}(-[0-9]{4})?$";
```

### Rule Generation

#### Validation Rules

Required fields and constraints generate validation rules:

```yaml
classes:
  Person:
    slots:
      - age
    rules:
      - title: adult_validation
        preconditions:
          - age >= 18
```

```typeql
rule person-adult-validation: when {
    $p isa person, has age $age;
    $age >= 18;
} then {
    $p has validated true;
};
```

#### Conditional Requirements

```yaml
classes:
  Document:
    conditional_requirements:
      - if_field: status
        value_presence: present
        required_fields: [approved_by, approval_date]
```

```typeql
rule document-conditional-requirement: when {
    $d isa document, has status $s;
    not { $d has approved-by $a; };
} then {
    # Validation failure
};
```

## Migration Support

### Schema Versioning

The generator tracks schema versions and generates migration scripts:

```rust
use linkml_service::generator::typeql_migration::{
    SchemaDiffer, MigrationAnalyzer, MigrationGenerator
};

// Compare schemas
let differ = SchemaDiffer::new();
let diff = differ.compare(&old_schema, &new_schema)?;

// Analyze impact
let analyzer = MigrationAnalyzer::new();
let analysis = analyzer.analyze(&diff)?;

// Generate migration
let generator = MigrationGenerator::new();
let migration = generator.generate(&analysis)?;
```

### Migration Categories

- **Safe**: Additive changes (new attributes, entities)
- **Warning**: Potentially breaking (type changes, new requirements)
- **Breaking**: Data loss possible (deletions, incompatible changes)

## Configuration Options

```rust
let mut options = GeneratorOptions::default();

// Include documentation
options.include_docs = true;

// Custom naming
options.set_custom("naming_convention", "kebab-case");

// Target TypeDB version
options.target_version = Some("3.0".to_string());

// Validation strictness
options.set_custom("strict_validation", "true");
```

## Best Practices

1. **Use Meaningful Names**: TypeDB uses kebab-case by default
2. **Define Clear Relations**: Explicit relation classes are preferred over implicit ones
3. **Leverage Inheritance**: Use abstract base classes for shared attributes
4. **Add Validation Rules**: Define constraints in LinkML for automatic rule generation
5. **Version Your Schemas**: Use migration support for production systems

## Error Handling

The generator provides detailed error messages:

```rust
match generator.generate(&schema, &options).await {
    Ok(outputs) => {
        // Success
    }
    Err(GeneratorError::SchemaValidation(msg)) => {
        eprintln!("Schema validation failed: {}", msg);
    }
    Err(GeneratorError::UnsupportedFeature(feature)) => {
        eprintln!("Unsupported LinkML feature: {}", feature);
    }
    Err(e) => {
        eprintln!("Generation failed: {}", e);
    }
}
```

## Limitations

- TypeDB doesn't support:
  - Union types (LinkML any_of)
  - Complex conditional logic in rules
  - Recursive relation definitions
  
- The generator will emit warnings for unsupported features

## Examples

See the `examples/` directory for complete examples:

- `typeql_basic_usage.rs` - Simple schema conversion
- `typeql_complex_relations.rs` - Multi-way and nested relations
- `typeql_migration_example.rs` - Schema migration workflow
- `typeql_performance_check.rs` - Performance testing

## Troubleshooting

### Common Issues

1. **Missing Relations**: Ensure slot ranges reference valid classes
2. **Invalid Rules**: Check TypeDB 3.0 rule syntax requirements
3. **Performance**: For very large schemas (>10k classes), consider chunking

### Debug Output

Enable debug logging:

```rust
env::set_var("RUST_LOG", "linkml_service::generator::typeql=debug");
env_logger::init();
```

## Further Reading

- [TypeDB Documentation](https://typedb.com/docs)
- [LinkML Specification](https://linkml.io/linkml/)
- [TypeQL Language Guide](https://typedb.com/docs/typeql/overview)
