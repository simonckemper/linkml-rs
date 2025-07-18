//! Performance benchmarks for LinkML expression evaluation and rules engine

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use linkml_service::expression::{Parser, Evaluator, EvaluationContext};
use linkml_service::rule_engine::{RuleEngine, Rule, RuleCondition, ExecutionStrategy};
use linkml_core::types::{SchemaDefinition, ClassDefinition};
use serde_json::json;
use std::collections::HashMap;

fn bench_expression_parsing(c: &mut Criterion) {
    let parser = Parser::new();
    
    let expressions = vec![
        ("simple_var", "{name}"),
        ("arithmetic", "{age} + 10"),
        ("comparison", "{age} >= 18 and {age} <= 65"),
        ("function_call", "len({name}) > 0"),
        ("nested", "({value} * 2) + (max({a}, {b}) - min({c}, {d}))"),
        ("complex", "case({status} == 'active', {premium} * 0.9, {status} == 'inactive', {premium} * 1.1, {premium})"),
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

fn bench_expression_evaluation(c: &mut Criterion) {
    let parser = Parser::new();
    let evaluator = Evaluator::new();
    
    // Prepare context
    let mut context = EvaluationContext::new();
    context.set_variable("name", json!("John Doe"));
    context.set_variable("age", json!(30));
    context.set_variable("status", json!("active"));
    context.set_variable("premium", json!(100.0));
    context.set_variable("value", json!(42));
    context.set_variable("a", json!(10));
    context.set_variable("b", json!(20));
    context.set_variable("c", json!(5));
    context.set_variable("d", json!(15));
    context.set_variable("items", json!(["a", "b", "c", "d", "e"]));
    
    let expressions = vec![
        ("variable_lookup", "{name}"),
        ("arithmetic_simple", "{age} + 10"),
        ("arithmetic_complex", "({value} * 2) + ({age} / 3) - 5"),
        ("comparison", "{age} >= 18 and {age} <= 65"),
        ("logical", "{status} == 'active' or ({age} > 25 and {premium} < 200)"),
        ("function_len", "len({items})"),
        ("function_math", "max({a}, {b}) + min({c}, {d})"),
        ("function_contains", "contains({name}, 'John')"),
        ("case_expression", "case({status} == 'active', {premium} * 0.9, {premium} * 1.1)"),
        ("nested_complex", "len({items}) > 3 and max({a}, {b}) > {age} / 2"),
    ];
    
    let mut group = c.benchmark_group("expression_evaluation");
    
    for (name, expr) in expressions {
        let ast = parser.parse(expr).unwrap();
        group.bench_function(name, |b| {
            b.iter(|| {
                let result = evaluator.evaluate(black_box(&ast), black_box(&context));
                assert!(result.is_ok());
            })
        });
    }
    
    group.finish();
}

fn bench_rule_engine(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("rule_schema");
    
    // Create class with rules
    let mut entity_class = ClassDefinition::new("Entity");
    
    // Simple rules
    for i in 0..10 {
        let rule = Rule {
            name: format!("rule_{}", i),
            description: Some(format!("Test rule {}", i)),
            preconditions: Some(RuleCondition::Expression(format!("{{field_{}}} > 0", i))),
            postconditions: Some(RuleCondition::Expression(format!("{{result_{}}} == true", i))),
            elseconditions: None,
            deactivated: Some(false),
            priority: Some(i as i32),
        };
        entity_class.rules.push(rule);
    }
    
    schema.classes.insert("Entity".to_string(), entity_class);
    
    let engine = RuleEngine::new(schema);
    
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
            let mut context = HashMap::new();
            context.insert("data".to_string(), data.clone());
            
            let result = engine.execute_class_rules(
                black_box("Entity"),
                black_box(&context),
                black_box(&ExecutionStrategy::Sequential)
            );
            assert!(result.is_ok());
        })
    });
    
    // Priority-based execution
    group.bench_function("priority", |b| {
        b.iter(|| {
            let mut context = HashMap::new();
            context.insert("data".to_string(), data.clone());
            
            let result = engine.execute_class_rules(
                black_box("Entity"),
                black_box(&context),
                black_box(&ExecutionStrategy::Priority)
            );
            assert!(result.is_ok());
        })
    });
    
    group.finish();
}

fn bench_complex_rule_scenarios(c: &mut Criterion) {
    let mut schema = SchemaDefinition::new("complex_rule_schema");
    
    // Create class with complex rules
    let mut order_class = ClassDefinition::new("Order");
    
    // Discount rules
    let discount_rule = Rule {
        name: "apply_discount".to_string(),
        description: Some("Apply discount based on order total".to_string()),
        preconditions: Some(RuleCondition::Expression("{total} > 100".to_string())),
        postconditions: Some(RuleCondition::Expression("{discount} == {total} * 0.1".to_string())),
        elseconditions: Some(RuleCondition::Expression("{discount} == 0".to_string())),
        deactivated: Some(false),
        priority: Some(1),
    };
    order_class.rules.push(discount_rule);
    
    // Shipping rules
    let shipping_rule = Rule {
        name: "calculate_shipping".to_string(),
        description: Some("Calculate shipping based on weight and destination".to_string()),
        preconditions: Some(RuleCondition::And(vec![
            Box::new(RuleCondition::Expression("{weight} > 0".to_string())),
            Box::new(RuleCondition::Expression("len({destination}) > 0".to_string())),
        ])),
        postconditions: Some(RuleCondition::Expression(
            "case({destination} == 'local', {weight} * 2, {destination} == 'national', {weight} * 5, {weight} * 10)".to_string()
        )),
        elseconditions: None,
        deactivated: Some(false),
        priority: Some(2),
    };
    order_class.rules.push(shipping_rule);
    
    // Validation rules
    let validation_rule = Rule {
        name: "validate_order".to_string(),
        description: Some("Validate order completeness".to_string()),
        preconditions: Some(RuleCondition::Or(vec![
            Box::new(RuleCondition::Expression("{items} == null".to_string())),
            Box::new(RuleCondition::Expression("len({items}) == 0".to_string())),
        ])),
        postconditions: Some(RuleCondition::Expression("{valid} == false".to_string())),
        elseconditions: Some(RuleCondition::Expression("{valid} == true".to_string())),
        deactivated: Some(false),
        priority: Some(0),
    };
    order_class.rules.push(validation_rule);
    
    schema.classes.insert("Order".to_string(), order_class);
    
    let engine = RuleEngine::new(schema);
    
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
            let mut context = HashMap::new();
            context.insert("data".to_string(), valid_order.clone());
            
            let result = engine.execute_class_rules(
                black_box("Order"),
                black_box(&context),
                black_box(&ExecutionStrategy::Sequential)
            );
            assert!(result.is_ok());
        })
    });
    
    group.bench_function("invalid_order", |b| {
        b.iter(|| {
            let mut context = HashMap::new();
            context.insert("data".to_string(), invalid_order.clone());
            
            let result = engine.execute_class_rules(
                black_box("Order"),
                black_box(&context),
                black_box(&ExecutionStrategy::Sequential)
            );
            assert!(result.is_ok());
        })
    });
    
    group.finish();
}

fn bench_expression_caching_impact(c: &mut Criterion) {
    let parser = Parser::new();
    let evaluator = Evaluator::new();
    
    // Complex expression that would benefit from caching
    let expr = "max({a}, {b}) + min({c}, {d}) * 2 + len({items}) - case({status} == 'active', 10, 20)";
    let ast = parser.parse(expr).unwrap();
    
    // Multiple contexts to simulate real usage
    let contexts: Vec<_> = (0..100).map(|i| {
        let mut ctx = EvaluationContext::new();
        ctx.set_variable("a", json!(i));
        ctx.set_variable("b", json!(i * 2));
        ctx.set_variable("c", json!(i / 2));
        ctx.set_variable("d", json!(i + 10));
        ctx.set_variable("items", json!(vec!["a"; i % 10 + 1]));
        ctx.set_variable("status", json!(if i % 2 == 0 { "active" } else { "inactive" }));
        ctx
    }).collect();
    
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