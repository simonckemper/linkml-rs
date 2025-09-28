//! Performance benchmarks for `LinkML` expression evaluation and rules engine.

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use linkml_core::types::{ClassDefinition, RuleConditions, SchemaDefinition};
use linkml_service::expression::{Evaluator, Parser};
use linkml_service::{Rule, rule_engine::RuleEngine};
use serde_json::json;
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

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

/// Benchmark parsing of representative expression fragments.
fn bench_expression_parsing(c: &mut Criterion) {
    let parser = Parser::new();

    let expressions = vec![
        ("simple_var", "{name}"),
        ("arithmetic", "{age} + 10"),
        ("comparison", "{age} >= 18 and {age} <= 65"),
        ("function_call", "len({name}) > 0"),
        ("nested", "({value} * 2) + (max({a}, {b}) - min({c}, {d}))"),
        (
            "complex",
            "case({status} == 'active', {premium} * 0.9, {status} == 'inactive', {premium} * 1.1, {premium})",
        ),
    ];

    let mut group = c.benchmark_group("expression_parsing");

    for (name, expr) in expressions {
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = parser.parse(black_box(expr));
                assert!(result.is_ok());
            })
        });
    }

    group.finish();
}

/// Benchmark evaluating expressions with the default evaluator configuration.
fn bench_expression_evaluation(c: &mut Criterion) {
    let parser = Parser::new();
    let evaluator = Evaluator::new();

    // Prepare context
    let mut context = HashMap::new();
    context.insert("name".to_string(), json!("John Doe"));
    context.insert("age".to_string(), json!(30));
    context.insert("status".to_string(), json!("active"));
    context.insert("premium".to_string(), json!(100.0));
    context.insert("value".to_string(), json!(42));
    context.insert("a".to_string(), json!(10));
    context.insert("b".to_string(), json!(20));
    context.insert("c".to_string(), json!(5));
    context.insert("d".to_string(), json!(15));
    context.insert("items".to_string(), json!(["a", "b", "c", "d", "e"]));

    let expressions = vec![
        ("variable_lookup", "{name}"),
        ("arithmetic_simple", "{age} + 10"),
        ("arithmetic_complex", "({value} * 2) + ({age} / 3) - 5"),
        ("comparison", "{age} >= 18 and {age} <= 65"),
        (
            "logical",
            "{status} == 'active' or ({age} > 25 and {premium} < 200)",
        ),
        ("function_len", "len({items})"),
        ("function_math", "max({a}, {b}) + min({c}, {d})"),
        ("function_contains", "contains({name}, 'John')"),
        (
            "case_expression",
            "case({status} == 'active', {premium} * 0.9, {premium} * 1.1)",
        ),
        (
            "nested_complex",
            "len({items}) > 3 and max({a}, {b}) > {age} / 2",
        ),
    ];

    let mut group = c.benchmark_group("expression_evaluation");

    for (name, expr) in expressions {
        let ast = require_ok(parser.parse(expr), "Failed to parse benchmark expression");
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = evaluator.evaluate(black_box(&ast), black_box(&context));
                assert!(result.is_ok());
            })
        });
    }

    group.finish();
}

/// Benchmark the rule engine against synthetic instances.
fn bench_rule_engine(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("rule_schema");

    // Create class with rules
    let mut entity_class = ClassDefinition::new("Entity");

    // Simple rules
    for i in 0..10 {
        let rule = Rule {
            title: Some(format!("Rule {}", i)),
            description: Some(format!("Test rule {}", i)),
            preconditions: Some(RuleConditions {
                expression_conditions: Some(vec![format!("{{field_{}}} > 0", i)]),
                ..Default::default()
            }),
            postconditions: Some(RuleConditions {
                expression_conditions: Some(vec![format!("{{result_{}}} == true", i)]),
                ..Default::default()
            }),
            else_conditions: None,
            deactivated: Some(false),
            priority: Some(i32::try_from(i).unwrap_or(0)),
        };
        entity_class.rules.push(rule);
    }
    schema.classes.insert("Entity".to_string(), entity_class);

    let engine = RuleEngine::new(Arc::new(schema.clone()));

    // Create test data
    let mut data = serde_json::Map::new();
    for i in 0..10 {
        data.insert(format!("field_{}", i), json!(i + 1));
        data.insert(format!("result_{}", i), json!(true));
    }
    let data = json!(data);

    let mut group = c.benchmark_group("rule_engine");

    // Sequential execution
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let mut validation_context = linkml_service::validator::context::ValidationContext::new(
                Arc::new(schema.clone()),
            );

            let issues = engine.validate(
                black_box(&data),
                black_box("Entity"),
                black_box(&mut validation_context),
            );
            // Expect no validation issues for well-formed test data
            assert!(issues.is_empty());
        })
    });
    // Parallel execution (if available)
    group.bench_function("parallel", |b| {
        b.iter(|| {
            let mut validation_context = linkml_service::validator::context::ValidationContext::new(
                Arc::new(schema.clone()),
            );

            let issues = engine.validate(
                black_box(&data),
                black_box("Entity"),
                black_box(&mut validation_context),
            );
            assert!(issues.is_empty());
        })
    });
    // Priority-based execution
    group.bench_function("priority", |b| {
        b.iter(|| {
            let mut validation_context = linkml_service::validator::context::ValidationContext::new(
                Arc::new(schema.clone()),
            );

            let issues = engine.validate(
                black_box(&data),
                black_box("Entity"),
                black_box(&mut validation_context),
            );
            assert!(issues.is_empty());
        })
    });

    group.finish();
}

fn bench_complex_rule_scenarios(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("complex_rule_schema");

    // Create class with complex rules
    let mut order_class = ClassDefinition::new("Order");

    let discount_rule = Rule {
        title: Some("Apply Discount".to_string()),
        description: Some("Apply discount based on order total".to_string()),
        preconditions: Some(RuleConditions {
            expression_conditions: Some(vec!["{total} > 100".to_string()]),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            expression_conditions: Some(vec!["{discount} == {total} * 0.1".to_string()]),
            ..Default::default()
        }),
        else_conditions: Some(RuleConditions {
            expression_conditions: Some(vec!["{discount} == 0".to_string()]),
            ..Default::default()
        }),
        deactivated: Some(false),
        priority: Some(1),
    };
    order_class.rules.push(discount_rule);

    let shipping_rule = Rule {
        title: Some("Calculate Shipping".to_string()),
        description: Some("Calculate shipping based on weight and destination".to_string()),
        preconditions: Some(RuleConditions {
            expression_conditions: Some(vec![
                "{weight} > 0".to_string(),
                "len({destination}) > 0".to_string(),
            ]),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            expression_conditions: Some(vec![
                "case({destination} == 'local', {weight} * 2, {destination} == 'national', {weight} * 5, {weight} * 10)".to_string()
            ]),
            ..Default::default()
        }),
        else_conditions: None,
        deactivated: Some(false),
        priority: Some(2),
    };
    order_class.rules.push(shipping_rule);

    let validation_rule = Rule {
        title: Some("Validate Order".to_string()),
        description: Some("Validate order completeness".to_string()),
        preconditions: Some(RuleConditions {
            expression_conditions: Some(vec![
                "{items} == null".to_string(),
                "len({items}) == 0".to_string(),
            ]),
            ..Default::default()
        }),
        postconditions: Some(RuleConditions {
            expression_conditions: Some(vec!["{valid} == false".to_string()]),
            ..Default::default()
        }),
        else_conditions: Some(RuleConditions {
            expression_conditions: Some(vec!["{valid} == true".to_string()]),
            ..Default::default()
        }),
        deactivated: Some(false),
        priority: Some(3),
    };
    order_class.rules.push(validation_rule);

    schema.classes.insert("Order".to_string(), order_class);

    let engine = RuleEngine::new(Arc::new(schema.clone()));

    // Test data variations
    let valid_order = json!({
        "total": 150.0,
        "weight": 5.0,
        "destination": "national",
        "items": ["item1", "item2", "item3"]
    });

    let invalid_order = json!({
        "total": 50.0,
        "weight": 0.0,
        "destination": "",
        "items": []
    });

    let mut group = c.benchmark_group("complex_rules");

    group.bench_function("valid_order", |b| {
        b.iter(|| {
            let mut validation_context = linkml_service::validator::context::ValidationContext::new(
                Arc::new(schema.clone()),
            );

            let issues = engine.validate(
                black_box(&valid_order),
                black_box("Order"),
                black_box(&mut validation_context),
            );
            assert!(issues.is_empty());
        })
    });

    group.bench_function("invalid_order", |b| {
        b.iter(|| {
            let mut validation_context = linkml_service::validator::context::ValidationContext::new(
                Arc::new(schema.clone()),
            );

            let _issues = engine.validate(
                black_box(&invalid_order),
                black_box("Order"),
                black_box(&mut validation_context),
            );
            // Invalid order may have validation issues, so we don't assert empty
        })
    });

    group.finish();
}

fn bench_expression_caching_impact(c: &mut Criterion) {
    let parser = Parser::new();
    let evaluator = Evaluator::new();

    // Complex expression that would benefit from caching
    let expr =
        "max({a}, {b}) + min({c}, {d}) * 2 + len({items}) - case({status} == 'active', 10, 20)";
    let ast = require_ok(
        parser.parse(expr),
        "Failed to parse expression for caching benchmark",
    );

    // Multiple contexts to simulate real usage
    let contexts: Vec<_> = (0..100)
        .map(|i| {
            let mut ctx = HashMap::new();
            ctx.insert("a".to_string(), json!(i));
            ctx.insert("b".to_string(), json!(i * 2));
            ctx.insert("c".to_string(), json!(i / 2));
            ctx.insert("d".to_string(), json!(i + 10));
            ctx.insert("items".to_string(), json!(vec!["a"; i % 10 + 1]));
            ctx.insert(
                "status".to_string(),
                json!(if i % 2 == 0 { "active" } else { "inactive" }),
            );
            ctx
        })
        .collect();

    c.bench_function("expression_repeated_eval", |b| {
        b.iter(|| {
            for ctx in &contexts {
                let result = evaluator.evaluate(black_box(&ast), black_box(ctx));
                assert!(result.is_ok());
            }
        })
    });
}

criterion_group!(
    benches,
    bench_expression_parsing,
    bench_expression_evaluation,
    bench_rule_engine,
    bench_complex_rule_scenarios,
    bench_expression_caching_impact
);

criterion_main!(benches);
