# LinkML Service Migration Guide

## Overview

This guide covers migration scenarios for the RootReal LinkML Service, including migrating from Python LinkML, handling schema versions, and data migration strategies.

## Table of Contents

1. [Migrating from Python LinkML](#migrating-from-python-linkml)
2. [Schema Version Migration](#schema-version-migration)
3. [Data Migration](#data-migration)
4. [Breaking Changes](#breaking-changes)
5. [Migration Tools](#migration-tools)
6. [Best Practices](#best-practices)

## Migrating from Python LinkML

### Feature Compatibility

Before migrating, review the [Parity Evaluation](PARITY_EVALUATION.md) to understand feature differences.

#### Supported Features (Direct Migration)

```python
# Python LinkML
from linkml_runtime.loaders import yaml_loader
from linkml_runtime.validators import validate

schema = yaml_loader.load("schema.yaml")
errors = validate(data, schema, "Person")
```

```rust
// RootReal LinkML
let service = create_linkml_service().await?;
let schema = service.load_schema("schema.yaml").await?;
let report = service.validate(&data, &schema, "Person").await?;
```

#### Features Requiring Adaptation

1. **Boolean Constraints**
   ```yaml
   # Python: any_of constraint
   slot:
     any_of:
       - range: string
       - range: integer
   
   # RootReal: Use union type (workaround)
   slot:
     range: string  # Document both types accepted
   ```

2. **Expression Language**
   ```yaml
   # Python: equals_expression
   slot:
     equals_expression: "other_slot + 1"
   
   # RootReal: Use custom validator
   # Implement validation logic in code
   ```

### API Migration

#### Schema Loading

```python
# Python
from linkml_runtime.loaders import yaml_loader
schema = yaml_loader.load("schema.yaml", base_dir="schemas/")
```

```rust
// RootReal
let mut config = LinkMLServiceConfig::default();
config.import_paths.push(PathBuf::from("schemas/"));
let service = create_linkml_service_with_config(config).await?;
let schema = service.load_schema("schema.yaml").await?;
```

#### Validation

```python
# Python
from linkml_runtime.validators import validate
errors = validate(instance, schema, target_class="Person")
for error in errors:
    print(f"{error.message} at {error.path}")
```

```rust
// RootReal
let report = service.validate(&data, &schema, "Person").await?;
if !report.valid {
    for error in &report.errors {
        println!("{} at {}", error.message, error.path.as_deref().unwrap_or("root"));
    }
}
```

#### Code Generation

```python
# Python
from linkml.generators.sqlgen import SQLGenerator
gen = SQLGenerator(schema_path)
sql = gen.serialize()
```

```rust
// RootReal
let sql = service.generate_sql(&schema, SqlDialect::PostgreSQL).await?;
```

### Performance Considerations

When migrating from Python LinkML:

1. **Async Operations**: RootReal uses async/await
   ```rust
   // Wrap in async runtime if needed
   let runtime = tokio::runtime::Runtime::new()?;
   let result = runtime.block_on(async_operation())?;
   ```

2. **Batch Processing**: Use RootReal's optimized batch APIs
   ```rust
   // Process multiple validations efficiently
   let reports = service.validate_batch(&records, &schema, "Person").await?;
   ```

3. **Caching**: Enable caching for better performance
   ```rust
   let config = LinkMLServiceConfig {
       enable_caching: true,
       cache_size: 10_000,
       ..Default::default()
   };
   ```

## Schema Version Migration

### Detecting Breaking Changes

```bash
# Check for breaking changes between versions
linkml migrate check \
  --old v1/schema.yaml \
  --new v2/schema.yaml

# Output
Breaking Changes Detected:
1. Class 'Person' removed slot 'phone'
2. Slot 'age' changed from string to integer
3. Enum 'Status' removed value 'pending'
```

### Migration Strategies

#### 1. Automatic Migration

For simple changes, use automatic migration:

```rust
let migration_plan = service.analyze_changes(&old_schema, &new_schema).await?;

for change in &migration_plan.changes {
    match &change.migration_strategy {
        MigrationStrategy::Automatic { transform } => {
            println!("Auto-migration available: {}", transform);
        }
        MigrationStrategy::Manual { instructions } => {
            println!("Manual migration needed: {}", instructions);
        }
    }
}
```

#### 2. Custom Migration Rules

```rust
// Define custom migration logic
pub struct PhoneToContactMigration;

impl DataMigration for PhoneToContactMigration {
    fn migrate(&self, data: &mut Value) -> Result<(), MigrationError> {
        if let Some(phone) = data.get("phone") {
            let contact = json!({
                "type": "phone",
                "value": phone
            });
            data["contacts"] = json!([contact]);
            data.as_object_mut().unwrap().remove("phone");
        }
        Ok(())
    }
}
```

#### 3. Version Tracking

```yaml
# Add version to schema
id: https://example.org/schemas/person
version: 2.0.0
migration_from: 1.0.0

# Track changes
changes:
  - removed: phone
    replacement: contacts
  - modified: age
    from_type: string
    to_type: integer
```

### Migration Workflow

```bash
# 1. Generate migration plan
linkml migrate plan \
  --old v1/schema.yaml \
  --new v2/schema.yaml \
  --output migration-plan.yaml

# 2. Review and customize plan
cat migration-plan.yaml

# 3. Test migration on sample data
linkml migrate test \
  --plan migration-plan.yaml \
  --sample sample-data.json

# 4. Apply migration to production data
linkml migrate apply \
  --plan migration-plan.yaml \
  --input old-data.json \
  --output new-data.json \
  --backup backup-data.json
```

## Data Migration

### Batch Data Migration

```rust
use futures::stream::{self, StreamExt};

async fn migrate_large_dataset(
    input_path: &Path,
    output_path: &Path,
    migration: &DataMigration,
) -> Result<(), Error> {
    let input = File::open(input_path)?;
    let output = File::create(output_path)?;
    let mut writer = BufWriter::new(output);
    
    // Stream processing for large files
    let reader = BufReader::new(input);
    let lines = reader.lines();
    
    let migrated = stream::iter(lines)
        .map(|line| async move {
            let mut data: Value = serde_json::from_str(&line?)?;
            migration.migrate(&mut data)?;
            Ok(serde_json::to_string(&data)?)
        })
        .buffer_unordered(100)
        .collect::<Vec<Result<String, Error>>>()
        .await;
    
    for result in migrated {
        writeln!(writer, "{}", result?)?;
    }
    
    Ok(())
}
```

### Rollback Support

```rust
// Create reversible migrations
pub struct ReversibleMigration {
    forward: Box<dyn DataMigration>,
    backward: Box<dyn DataMigration>,
}

impl ReversibleMigration {
    pub async fn apply(&self, data: &mut Value) -> Result<(), Error> {
        // Save original state
        let backup = data.clone();
        
        match self.forward.migrate(data) {
            Ok(_) => Ok(()),
            Err(e) => {
                // Rollback on error
                *data = backup;
                Err(e)
            }
        }
    }
    
    pub async fn rollback(&self, data: &mut Value) -> Result<(), Error> {
        self.backward.migrate(data)
    }
}
```

## Breaking Changes

### Version 1.0 → 2.0

Key breaking changes to be aware of:

1. **Async API**: All operations now async
   ```rust
   // Before
   let schema = service.load_schema("schema.yaml")?;
   
   // After
   let schema = service.load_schema("schema.yaml").await?;
   ```

2. **Service Architecture**: Now requires dependency injection
   ```rust
   // Before
   let service = LinkMLService::new();
   
   // After
   let service = create_linkml_service().await?;
   ```

3. **Error Types**: Unified error handling
   ```rust
   // Before
   ValidationError, ParseError, IOError
   
   // After
   LinkMLError with variants
   ```

### Deprecation Notices

Features scheduled for removal:

- `validate_sync()` - Use async version
- `load_schema_string()` - Use `load_schema_str()`
- `generate_code()` - Use specific generators

## Migration Tools

### CLI Migration Commands

```bash
# Interactive migration wizard
linkml migrate wizard

# Batch migration with progress
linkml migrate batch \
  --input-dir ./v1-data/ \
  --output-dir ./v2-data/ \
  --schema-old v1/schema.yaml \
  --schema-new v2/schema.yaml \
  --parallel 8 \
  --progress

# Verify migrated data
linkml migrate verify \
  --data ./v2-data/ \
  --schema v2/schema.yaml
```

### Programmatic Migration

```rust
use linkml_service::migration::{MigrationEngine, MigrationOptions};

let engine = MigrationEngine::new();

// Configure migration
let options = MigrationOptions {
    strict_mode: false,
    create_backups: true,
    validate_output: true,
    ..Default::default()
};

// Execute migration
let results = engine.migrate_directory(
    "v1-data/",
    "v2-data/",
    &old_schema,
    &new_schema,
    options
).await?;

println!("Migrated {} files", results.success_count);
println!("Failed {} files", results.failure_count);
```

## Best Practices

### 1. Test Migrations Thoroughly

```rust
#[cfg(test)]
mod migration_tests {
    #[test]
    fn test_phone_to_contact_migration() {
        let input = json!({
            "name": "John",
            "phone": "555-1234"
        });
        
        let expected = json!({
            "name": "John",
            "contacts": [{
                "type": "phone",
                "value": "555-1234"
            }]
        });
        
        let mut data = input.clone();
        migration.migrate(&mut data).unwrap();
        assert_eq!(data, expected);
    }
}
```

### 2. Version Your Schemas

```yaml
# schema-v1.0.0.yaml
version: 1.0.0

# schema-v2.0.0.yaml
version: 2.0.0
migration_from: 1.0.0
```

### 3. Document Changes

```markdown
# MIGRATION.md
## Version 2.0.0

### Breaking Changes
- Removed `phone` field from Person class
- Changed `age` from string to integer

### Migration Instructions
1. Run migration tool: `linkml migrate ...`
2. Update API clients to use `contacts` array
3. Ensure age values are numeric
```

### 4. Gradual Migration

```rust
// Support both old and new formats during transition
pub struct DualFormatValidator {
    old_schema: SchemaDefinition,
    new_schema: SchemaDefinition,
}

impl DualFormatValidator {
    pub async fn validate(&self, data: &Value) -> Result<ValidationReport> {
        // Try new format first
        let report = self.service.validate(data, &self.new_schema, "Person").await?;
        
        if !report.valid {
            // Fall back to old format
            return self.service.validate(data, &self.old_schema, "Person").await;
        }
        
        Ok(report)
    }
}
```

### 5. Monitor Migration Progress

```rust
// Track migration metrics
let metrics = MigrationMetrics {
    total_records: 1_000_000,
    migrated_records: 0,
    failed_records: 0,
    start_time: Instant::now(),
};

// Update progress
metrics.migrated_records += 1;
let progress = metrics.migrated_records as f64 / metrics.total_records as f64;
println!("Migration progress: {:.1}%", progress * 100.0);
```

## Troubleshooting

### Common Migration Issues

1. **Schema Import Errors**
   - Ensure import paths are configured
   - Check for circular dependencies
   - Verify file permissions

2. **Data Type Mismatches**
   - Use type coercion where appropriate
   - Implement custom converters
   - Validate before and after migration

3. **Performance Problems**
   - Enable streaming for large files
   - Use parallel processing
   - Monitor memory usage

4. **Validation Failures**
   - Review validation reports
   - Check for schema differences
   - Implement gradual migration

## Conclusion

Migration to RootReal LinkML Service provides significant performance benefits while maintaining compatibility with most LinkML features. Plan migrations carefully, test thoroughly, and use the provided tools to ensure smooth transitions.
