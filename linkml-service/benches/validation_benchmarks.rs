//! Performance benchmarks for LinkML validation

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
};
use linkml_service::validator::{ValidationEngine, ValidationOptions};
use serde_json::json;
use std::collections::HashMap;

fn create_simple_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("benchmark_schema");

    // Add slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(0.0.into());
    age_slot.maximum_value = Some(150.0.into());
    schema.slots.insert("age".to_string(), age_slot);

    // Add class
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    schema
}

fn create_complex_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::new("complex_schema");

    // Add enum
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

    // Add many slots
    for i in 0..20 {
        let mut slot = SlotDefinition::new(&format!("field_{}", i));
        slot.range = Some(
            if i % 3 == 0 {
                "string"
            } else if i % 3 == 1 {
                "integer"
            } else {
                "boolean"
            }
            .to_string(),
        );
        if i % 4 == 0 {
            slot.required = Some(true);
        }
        if i % 5 == 0 {
            slot.pattern = Some(r"^\w+$".to_string());
        }
        schema.slots.insert(format!("field_{}", i), slot);
    }

    // Add classes with inheritance
    let mut base_class = ClassDefinition::new("BaseEntity");
    base_class.slots = vec!["field_0".to_string(), "field_1".to_string()];
    schema.classes.insert("BaseEntity".to_string(), base_class);

    for i in 0..5 {
        let mut class = ClassDefinition::new(&format!("Entity{}", i));
        class.is_a = Some("BaseEntity".to_string());
        class.slots = (2..6)
            .map(|j| format!("field_{}", (i * 4 + j) % 20))
            .collect();
        schema.classes.insert(format!("Entity{}", i), class);
    }

    schema
}

fn bench_simple_validation(c: &mut Criterion) {
    let schema = create_simple_schema();
    let engine = ValidationEngine::new(schema.clone());
    let options = ValidationOptions::default();

    let valid_data = json!({
        "name": "John Doe",
        "age": 30
    });

    let invalid_data = json!({
        "age": 200  // Missing required name, age out of range
    });

    c.bench_function("simple_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&valid_data), "Person", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("simple_invalid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&invalid_data), "Person", &options);
            assert!(result.is_err());
        })
    });
}

fn bench_complex_validation(c: &mut Criterion) {
    let schema = create_complex_schema();
    let engine = ValidationEngine::new(schema.clone());
    let options = ValidationOptions::default();

    // Create test data
    let mut valid_data = serde_json::Map::new();
    for i in 0..20 {
        let value = if i % 3 == 0 {
            json!("valid_string")
        } else if i % 3 == 1 {
            json!(42)
        } else {
            json!(true)
        };
        valid_data.insert(format!("field_{}", i), value);
    }

    c.bench_function("complex_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&json!(valid_data)), "Entity0", &options);
            assert!(result.is_ok());
        })
    });
}

fn bench_batch_validation(c: &mut Criterion) {
    let schema = create_simple_schema();
    let engine = ValidationEngine::new(schema.clone());
    let options = ValidationOptions::default();

    let mut group = c.benchmark_group("batch_validation");

    for size in [10, 100, 1000].iter() {
        let instances: Vec<_> = (0..*size)
            .map(|i| {
                json!({
                    "name": format!("Person {}", i),
                    "age": i % 100
                })
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                for instance in &instances {
                    let _ = engine.validate(black_box(instance), "Person", &options);
                }
            })
        });
    }

    group.finish();
}

fn bench_pattern_validation(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("pattern_schema");

    // Add slot with complex pattern
    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut class = ClassDefinition::new("Contact");
    class.slots = vec!["email".to_string()];
    schema.classes.insert("Contact".to_string(), class);

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    let valid_email = json!({
        "email": "user@example.com"
    });

    let invalid_email = json!({
        "email": "not-an-email"
    });

    c.bench_function("pattern_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&valid_email), "Contact", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("pattern_invalid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&invalid_email), "Contact", &options);
            assert!(result.is_err());
        })
    });
}

fn bench_enum_validation(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("enum_schema");

    // Large enum
    let large_enum = EnumDefinition {
        name: "Country".to_string(),
        permissible_values: (0..200)
            .map(|i| PermissibleValue::Simple(format!("COUNTRY_{:03}", i)))
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

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    let valid_data = json!({
        "country": "COUNTRY_050"
    });

    let invalid_data = json!({
        "country": "INVALID_COUNTRY"
    });

    c.bench_function("enum_large_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&valid_data), "Location", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("enum_large_invalid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&invalid_data), "Location", &options);
            assert!(result.is_err());
        })
    });
}

fn bench_inheritance_resolution(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("inheritance_schema");

    // Create deep inheritance hierarchy
    for i in 0..10 {
        let mut class = ClassDefinition::new(&format!("Level{}", i));
        if i > 0 {
            class.is_a = Some(format!("Level{}", i - 1));
        }

        // Add slot at each level
        let mut slot = SlotDefinition::new(&format!("field_level_{}", i));
        slot.range = Some("string".to_string());
        schema.slots.insert(format!("field_level_{}", i), slot);

        class.slots = vec![format!("field_level_{}", i)];
        schema.classes.insert(format!("Level{}", i), class);
    }

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    // Data with all inherited fields
    let mut data = serde_json::Map::new();
    for i in 0..10 {
        data.insert(format!("field_level_{}", i), json!(format!("value_{}", i)));
    }

    c.bench_function("deep_inheritance", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&json!(data)), "Level9", &options);
            assert!(result.is_ok());
        })
    });
}

fn bench_boolean_constraints(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("boolean_schema");

    // Create any_of constraint slot
    let mut any_of_slot = SlotDefinition::new("flexible_value");
    any_of_slot.any_of = Some(vec![
        linkml_core::types::AnonymousSlotExpression {
            range: Some("string".to_string()),
            pattern: Some(r"^[A-Z]{2}\d{6}$".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            range: Some("integer".to_string()),
            minimum_value: Some(json!(1000)),
            maximum_value: Some(json!(9999)),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            pattern: Some(r"^\d{3}-\d{2}-\d{4}$".to_string()),
            ..Default::default()
        },
    ]);
    schema
        .slots
        .insert("flexible_value".to_string(), any_of_slot);

    // Create all_of constraint slot (for parallel evaluation)
    let mut all_of_slot = SlotDefinition::new("strict_value");
    all_of_slot.all_of = Some(vec![
        linkml_core::types::AnonymousSlotExpression {
            range: Some("string".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            pattern: Some(r"^[A-Z]".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            pattern: Some(r"\d$".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            minimum_value: Some(json!(5)),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            maximum_value: Some(json!(20)),
            ..Default::default()
        },
    ]);
    schema.slots.insert("strict_value".to_string(), all_of_slot);

    // Create none_of constraint slot
    let mut none_of_slot = SlotDefinition::new("exclusive_value");
    none_of_slot.none_of = Some(vec![
        linkml_core::types::AnonymousSlotExpression {
            range: Some("string".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            range: Some("integer".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            range: Some("boolean".to_string()),
            ..Default::default()
        },
    ]);
    schema
        .slots
        .insert("exclusive_value".to_string(), none_of_slot);

    // Create exactly_one_of constraint slot
    let mut exactly_one_slot = SlotDefinition::new("precise_value");
    exactly_one_slot.exactly_one_of = Some(vec![
        linkml_core::types::AnonymousSlotExpression {
            range: Some("string".to_string()),
            pattern: Some(r"^test_".to_string()),
            ..Default::default()
        },
        linkml_core::types::AnonymousSlotExpression {
            range: Some("integer".to_string()),
            minimum_value: Some(json!(100)),
            ..Default::default()
        },
    ]);
    schema
        .slots
        .insert("precise_value".to_string(), exactly_one_slot);

    // Create classes
    let mut any_of_class = ClassDefinition::new("FlexibleEntity");
    any_of_class.slots = vec!["flexible_value".to_string()];
    schema
        .classes
        .insert("FlexibleEntity".to_string(), any_of_class);

    let mut all_of_class = ClassDefinition::new("StrictEntity");
    all_of_class.slots = vec!["strict_value".to_string()];
    schema
        .classes
        .insert("StrictEntity".to_string(), all_of_class);

    let mut none_of_class = ClassDefinition::new("ExclusiveEntity");
    none_of_class.slots = vec!["exclusive_value".to_string()];
    schema
        .classes
        .insert("ExclusiveEntity".to_string(), none_of_class);

    let mut exactly_one_class = ClassDefinition::new("PreciseEntity");
    exactly_one_class.slots = vec!["precise_value".to_string()];
    schema
        .classes
        .insert("PreciseEntity".to_string(), exactly_one_class);

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    // Test data
    let any_of_valid = json!({"flexible_value": "AB123456"});
    let any_of_invalid = json!({"flexible_value": "invalid"});

    let all_of_valid = json!({"strict_value": "Hello123"});
    let all_of_invalid = json!({"strict_value": "hello"});

    let none_of_valid = json!({"exclusive_value": 3.14});
    let none_of_invalid = json!({"exclusive_value": "string"});

    let exactly_one_valid = json!({"precise_value": "test_123"});
    let exactly_one_invalid = json!({"precise_value": 500}); // Satisfies integer constraint too

    // Benchmarks
    c.bench_function("any_of_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&any_of_valid), "FlexibleEntity", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("any_of_invalid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&any_of_invalid), "FlexibleEntity", &options);
            assert!(result.is_err());
        })
    });

    c.bench_function("all_of_valid_parallel", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&all_of_valid), "StrictEntity", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("all_of_invalid_parallel", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&all_of_invalid), "StrictEntity", &options);
            assert!(result.is_err());
        })
    });

    c.bench_function("none_of_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&none_of_valid), "ExclusiveEntity", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("none_of_invalid_early_exit", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&none_of_invalid), "ExclusiveEntity", &options);
            assert!(result.is_err());
        })
    });

    c.bench_function("exactly_one_of_valid", |b| {
        b.iter(|| {
            let result = engine.validate(black_box(&exactly_one_valid), "PreciseEntity", &options);
            assert!(result.is_ok());
        })
    });

    c.bench_function("exactly_one_of_invalid", |b| {
        b.iter(|| {
            let result =
                engine.validate(black_box(&exactly_one_invalid), "PreciseEntity", &options);
            assert!(result.is_err());
        })
    });
}

fn bench_conditional_validation(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("conditional_schema");

    // Add slots with conditional requirements
    let mut type_slot = SlotDefinition::new("type");
    type_slot.range = Some("string".to_string());
    schema.slots.insert("type".to_string(), type_slot);

    let mut value_slot = SlotDefinition::new("value");
    value_slot.range = Some("string".to_string());

    // Add conditional requirement
    let mut condition = HashMap::new();
    condition.insert("equals_string".to_string(), json!("special"));

    value_slot.if_required = Some(json!({
        "slot": "type",
        "value": condition
    }));
    value_slot.then_required = Some(true);

    schema.slots.insert("value".to_string(), value_slot);

    let mut class = ClassDefinition::new("ConditionalEntity");
    class.slots = vec!["type".to_string(), "value".to_string()];
    schema
        .classes
        .insert("ConditionalEntity".to_string(), class);

    let engine = ValidationEngine::new(schema);
    let options = ValidationOptions::default();

    let data_requiring_value = json!({
        "type": "special",
        "value": "required_value"
    });

    let data_not_requiring_value = json!({
        "type": "normal"
    });

    c.bench_function("conditional_required", |b| {
        b.iter(|| {
            let result = engine.validate(
                black_box(&data_requiring_value),
                "ConditionalEntity",
                &options,
            );
            assert!(result.is_ok());
        })
    });

    c.bench_function("conditional_not_required", |b| {
        b.iter(|| {
            let result = engine.validate(
                black_box(&data_not_requiring_value),
                "ConditionalEntity",
                &options,
            );
            assert!(result.is_ok());
        })
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
