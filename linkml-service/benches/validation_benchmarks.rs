//! Performance benchmarks for `LinkML` validation.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use indexmap::IndexMap;
use linkml_core::types::{
    AnonymousSlotExpression, ClassDefinition, ConditionalRequirement, EnumDefinition,
    PermissibleValue, SchemaDefinition, SlotCondition, SlotDefinition,
};
use linkml_service::validator::{ValidationEngine, ValidationOptions, ValidationReport};
use serde_json::{Value, json};
use std::fmt::Display;
use std::sync::Arc;
use tokio::runtime::Runtime;

/// Helper that panics with context when a benchmark setup step fails.
fn require_ok<T, E>(result: Result<T, E>, context: &str) -> T
where
    E: Display,
{
    match result {
        Ok(value) => value,
        Err(err) => panic!("{context}: {err}"),
    }
}

fn create_runtime() -> Runtime {
    require_ok(Runtime::new(), "Failed to create Tokio runtime")
}

fn create_engine(schema: &SchemaDefinition) -> Arc<ValidationEngine> {
    Arc::new(require_ok(
        ValidationEngine::new(schema),
        "Failed to construct validation engine",
    ))
}

fn run_validation(
    runtime: &Runtime,
    engine: Arc<ValidationEngine>,
    data: Value,
    class_name: &str,
    options: ValidationOptions,
) -> ValidationReport {
    let class = class_name.to_string();
    runtime.block_on(async move {
        require_ok(
            engine.validate_as_class(&data, &class, Some(options)).await,
            "Validation execution failed",
        )
    })
}

fn create_simple_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("benchmark_schema");

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(0.0.into());
    age_slot.maximum_value = Some(150.0.into());
    schema.slots.insert("age".to_string(), age_slot);

    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    schema
}

fn create_complex_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("complex_schema");

    let status_enum = EnumDefinition {
        name: "Status".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("active".to_string()),
            PermissibleValue::Simple("inactive".to_string()),
            PermissibleValue::Simple("pending".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("Status".to_string(), status_enum);

    for i in 0..20 {
        let mut slot = SlotDefinition::new(&format!("field_{i}"));
        slot.range = Some(
            match i % 3 {
                0 => "string",
                1 => "integer",
                _ => "boolean",
            }
            .to_string(),
        );
        if i % 4 == 0 {
            slot.required = Some(true);
        }
        if i % 5 == 0 {
            slot.pattern = Some(r"^\w+$".to_string());
        }
        schema.slots.insert(format!("field_{i}"), slot);
    }

    let mut base_class = ClassDefinition::new("BaseEntity");
    base_class.slots = vec!["field_0".to_string(), "field_1".to_string()];
    schema.classes.insert("BaseEntity".to_string(), base_class);

    for i in 0..5 {
        let mut class = ClassDefinition::new(&format!("Entity{i}"));
        class.is_a = Some("BaseEntity".to_string());
        class.slots = (2..6)
            .map(|j| format!("field_{}", (i * 4 + j) % 20))
            .collect();
        schema.classes.insert(format!("Entity{i}"), class);
    }

    schema
}

fn create_conditional_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("conditional_schema");

    let mut type_slot = SlotDefinition::new("type");
    type_slot.range = Some("string".to_string());
    schema.slots.insert("type".to_string(), type_slot);

    let mut value_slot = SlotDefinition::new("value");
    value_slot.range = Some("string".to_string());
    schema.slots.insert("value".to_string(), value_slot);

    let mut class = ClassDefinition::new("ConditionalEntity");
    class.slots = vec!["type".to_string(), "value".to_string()];

    let mut slot_condition = SlotCondition::default();
    slot_condition.equals_string = Some("special".to_string());

    let mut requirement = ConditionalRequirement::default();
    requirement.condition = Some(slot_condition);
    requirement.then_required = Some(vec!["value".to_string()]);

    let mut condition_map = IndexMap::new();
    condition_map.insert("type".to_string(), requirement);
    class.if_required = Some(condition_map);

    schema
        .classes
        .insert("ConditionalEntity".to_string(), class);

    schema
}

/// Benchmark validating simple instances against a compact schema.
fn bench_simple_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let schema = create_simple_schema();
    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let valid_data = json!({
        "name": "John Doe",
        "age": 30
    });

    let invalid_data = json!({
        "age": 200
    });

    c.bench_function("simple_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = valid_data.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Person",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("simple_invalid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = invalid_data.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Person",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });
}

/// Benchmark validating complex instances against a rich schema.
fn bench_complex_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let schema = create_complex_schema();
    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let mut valid_data = serde_json::Map::new();
    for i in 0..20 {
        let value = if i % 3 == 0 {
            json!("value")
        } else if i % 3 == 1 {
            json!(42)
        } else {
            json!(true)
        };
        valid_data.insert(format!("field_{i}"), value);
    }

    c.bench_function("complex_entity_validation", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = Value::Object(valid_data.clone());
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Entity0",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report.stats.total_validated);
        });
    });
}

/// Benchmark batch validation throughput.
fn bench_batch_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let schema = create_simple_schema();
    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let mut group = c.benchmark_group("batch_validation");

    for size in [10usize, 100, 1000] {
        let instances: Vec<Value> = (0..size)
            .map(|i| {
                json!({
                    "name": format!("Person {i}"),
                    "age": i % 100
                })
            })
            .collect();

        group.throughput(Throughput::Elements(u64::try_from(size).unwrap_or(0)));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            let rt = &runtime;
            let engine = Arc::clone(&engine);
            let opts = options.clone();
            b.iter(|| {
                let mut total = 0usize;
                for instance in instances.clone() {
                    let report =
                        run_validation(rt, Arc::clone(&engine), instance, "Person", opts.clone());
                    assert!(report.valid);
                    total += report.stats.total_validated as usize;
                }
                black_box(total);
            });
        });
    }

    group.finish();
}

/// Benchmark validation of pattern constraints.
fn bench_pattern_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let mut schema = SchemaDefinition::new("pattern_schema");

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut class = ClassDefinition::new("Contact");
    class.slots = vec!["email".to_string()];
    schema.classes.insert("Contact".to_string(), class);

    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let valid_email = json!({ "email": "user@example.com" });
    let invalid_email = json!({ "email": "invalid" });

    c.bench_function("pattern_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = valid_email.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Contact",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("pattern_invalid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = invalid_email.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Contact",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });
}

/// Benchmark validation of enumerated values.
fn bench_enum_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let mut schema = SchemaDefinition::new("enum_schema");

    let large_enum = EnumDefinition {
        name: "Country".to_string(),
        permissible_values: (0..200)
            .map(|i| PermissibleValue::Simple(format!("COUNTRY_{i:03}")))
            .collect(),
        ..Default::default()
    };
    schema.enums.insert("Country".to_string(), large_enum);

    let mut country_slot = SlotDefinition::new("country");
    country_slot.range = Some("Country".to_string());
    schema.slots.insert("country".to_string(), country_slot);

    let mut class = ClassDefinition::new("Location");
    class.slots = vec!["country".to_string()];
    schema.classes.insert("Location".to_string(), class);

    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let valid_data = json!({ "country": "COUNTRY_050" });
    let invalid_data = json!({ "country": "INVALID" });

    c.bench_function("enum_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = valid_data.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Location",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("enum_invalid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = invalid_data.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Location",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });
}

/// Benchmark validation when inheritance resolution is required.
fn bench_inheritance_resolution(c: &mut Criterion) {
    let runtime = create_runtime();
    let mut schema = SchemaDefinition::new("inheritance_schema");

    for i in 0..10 {
        let mut class = ClassDefinition::new(&format!("Level{i}"));
        if i > 0 {
            class.is_a = Some(format!("Level{}", i - 1));
        }

        let mut slot = SlotDefinition::new(&format!("field_level_{i}"));
        slot.range = Some("string".to_string());
        schema.slots.insert(format!("field_level_{i}"), slot);

        class.slots = vec![format!("field_level_{i}")];
        schema.classes.insert(format!("Level{i}"), class);
    }

    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let mut data = serde_json::Map::new();
    for i in 0..10 {
        data.insert(format!("field_level_{i}"), json!(format!("value_{i}")));
    }

    c.bench_function("deep_inheritance", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = Value::Object(data.clone());
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "Level9",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report.stats.validators_executed);
        });
    });
}

/// Benchmark validation of boolean constraint combinations.
fn bench_boolean_constraints(c: &mut Criterion) {
    let runtime = create_runtime();
    let mut schema = SchemaDefinition::new("boolean_schema");

    let mut any_of_slot = SlotDefinition::new("flexible_value");
    any_of_slot.any_of = Some(vec![
        AnonymousSlotExpression {
            range: Some("string".to_string()),
            pattern: Some(r"^[A-Z]{2}\d{6}$".to_string()),
            ..Default::default()
        },
        AnonymousSlotExpression {
            range: Some("integer".to_string()),
            minimum_value: Some(json!(1000)),
            maximum_value: Some(json!(9999)),
            ..Default::default()
        },
        AnonymousSlotExpression {
            pattern: Some(r"^\d{3}-\d{2}-\d{4}$".to_string()),
            ..Default::default()
        },
    ]);
    schema
        .slots
        .insert("flexible_value".to_string(), any_of_slot);

    let mut all_of_slot = SlotDefinition::new("strict_value");
    all_of_slot.all_of = Some(vec![
        AnonymousSlotExpression {
            range: Some("string".to_string()),
            ..Default::default()
        },
        AnonymousSlotExpression {
            pattern: Some(r"^[A-Z]".to_string()),
            ..Default::default()
        },
        AnonymousSlotExpression {
            pattern: Some(r"\d$".to_string()),
            ..Default::default()
        },
    ]);
    schema.slots.insert("strict_value".to_string(), all_of_slot);

    let mut none_of_slot = SlotDefinition::new("exclusive_value");
    none_of_slot.none_of = Some(vec![
        AnonymousSlotExpression {
            range: Some("string".to_string()),
            ..Default::default()
        },
        AnonymousSlotExpression {
            range: Some("integer".to_string()),
            ..Default::default()
        },
    ]);
    schema
        .slots
        .insert("exclusive_value".to_string(), none_of_slot);

    let mut any_class = ClassDefinition::new("FlexibleEntity");
    any_class.slots = vec!["flexible_value".to_string()];
    schema
        .classes
        .insert("FlexibleEntity".to_string(), any_class);

    let mut all_class = ClassDefinition::new("StrictEntity");
    all_class.slots = vec!["strict_value".to_string()];
    schema.classes.insert("StrictEntity".to_string(), all_class);

    let mut none_class = ClassDefinition::new("ExclusiveEntity");
    none_class.slots = vec!["exclusive_value".to_string()];
    schema
        .classes
        .insert("ExclusiveEntity".to_string(), none_class);

    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let any_valid = json!({ "flexible_value": "AB123456" });
    let any_invalid = json!({ "flexible_value": "invalid" });
    let all_valid = json!({ "strict_value": "Avalue9" });
    let none_valid = json!({ "exclusive_value": std::f64::consts::PI });
    let none_invalid = json!({ "exclusive_value": "string" });

    c.bench_function("any_of_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = any_valid.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "FlexibleEntity",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("any_of_invalid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = any_invalid.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "FlexibleEntity",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });

    c.bench_function("all_of_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = all_valid.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "StrictEntity",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("none_of_valid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = none_valid.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "ExclusiveEntity",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("none_of_invalid", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = none_invalid.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "ExclusiveEntity",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });
}

/// Benchmark validation of conditional (if/then) requirements.
fn bench_conditional_validation(c: &mut Criterion) {
    let runtime = create_runtime();
    let schema = create_conditional_schema();
    let engine = create_engine(&schema);
    let options = ValidationOptions::default();

    let requires_value = json!({
        "type": "special",
        "value": "present",
    });

    let missing_required = json!({
        "type": "special"
    });

    let not_required = json!({
        "type": "normal"
    });

    c.bench_function("conditional_required_present", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = requires_value.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "ConditionalEntity",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });

    c.bench_function("conditional_missing_value", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = missing_required.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "ConditionalEntity",
                opts.clone(),
            );
            assert!(!report.valid);
            black_box(report.summary());
        });
    });

    c.bench_function("conditional_not_required", |b| {
        let rt = &runtime;
        let engine = Arc::clone(&engine);
        let payload = not_required.clone();
        let opts = options.clone();
        b.iter(|| {
            let report = run_validation(
                rt,
                Arc::clone(&engine),
                payload.clone(),
                "ConditionalEntity",
                opts.clone(),
            );
            assert!(report.valid);
            black_box(report);
        });
    });
}

criterion_group!(
    benches,
    bench_simple_validation,
    bench_complex_validation,
    bench_batch_validation,
    bench_pattern_validation,
    bench_enum_validation,
    bench_inheritance_resolution,
    bench_boolean_constraints,
    bench_conditional_validation
);

criterion_main!(benches);
