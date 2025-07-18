//! Benchmarks for TypeQL generation performance

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, GeneratorOptions, IndentStyle,
    typeql_generator_enhanced::EnhancedTypeQLGenerator,
};
use std::collections::HashMap;

/// Create a simple schema with the specified number of classes
fn create_schema(num_classes: usize) -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = format!("BenchmarkSchema{}", num_classes);
    schema.id = Some(format!("https://example.org/schemas/benchmark{}", num_classes));
    
    // Add common slots
    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);
    
    let mut id_slot = SlotDefinition::default();
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("id".to_string(), id_slot);
    
    let mut created_at = SlotDefinition::default();
    created_at.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_at);
    
    // Create classes
    for i in 0..num_classes {
        let mut class = ClassDefinition::default();
        class.description = Some(format!("Test class {}", i));
        
        // Add slots
        class.slots.extend(vec![
            "id".to_string(),
            "name".to_string(),
            "created_at".to_string(),
        ]);
        
        // Add some classes with inheritance
        if i > 0 && i % 3 == 0 {
            class.is_a = Some(format!("Class{}", i - 1));
        }
        
        // Add some relations
        if i % 5 == 0 && i > 0 {
            class.slots.push(format!("related_to_{}", i - 1));
            
            let mut rel_slot = SlotDefinition::default();
            rel_slot.range = Some(format!("Class{}", i - 1));
            rel_slot.multivalued = Some(true);
            schema.slots.insert(format!("related_to_{}", i - 1), rel_slot);
        }
        
        schema.classes.insert(format!("Class{}", i), class);
    }
    
    schema
}

/// Create a schema with complex relationships
fn create_complex_schema(num_relations: usize) -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "ComplexRelationshipSchema".to_string();
    
    // Create base entity classes
    for i in 0..5 {
        let mut class = ClassDefinition::default();
        class.slots.push("id".to_string());
        schema.classes.insert(format!("Entity{}", i), class);
    }
    
    // Create relation classes
    for i in 0..num_relations {
        let mut rel_class = ClassDefinition::default();
        rel_class.description = Some("Complex multi-way relation".to_string());
        
        // Add 3-5 role players
        let num_roles = 3 + (i % 3);
        for j in 0..num_roles {
            rel_class.slots.push(format!("role_{}", j));
            
            let mut role_slot = SlotDefinition::default();
            role_slot.range = Some(format!("Entity{}", j % 5));
            schema.slots.insert(format!("relation_{}_role_{}", i, j), role_slot);
        }
        
        schema.classes.insert(format!("Relation{}", i), rel_class);
    }
    
    schema
}

/// Create a schema with many rules
fn create_rule_heavy_schema(num_rules: usize) -> SchemaDefinition {
    let mut schema = create_schema(10); // Base schema with 10 classes
    
    // Add rules to classes
    for (i, (name, class)) in schema.classes.iter_mut().enumerate() {
        if i >= num_rules {
            break;
        }
        
        // Add validation rules
        let mut rule = Rule::default();
        rule.name = Some(format!("validation_rule_{}", i));
        rule.description = Some("Validation rule".to_string());
        rule.preconditions = Some(vec![
            Expression::Comparison {
                left: Box::new(Expression::Variable("name".to_string())),
                operator: ComparisonOperator::NotEqual,
                right: Box::new(Expression::Literal(Value::String("".to_string()))),
            }
        ]);
        
        class.rules.push(rule);
        
        // Add conditional requirements
        let mut cond_req = SlotCondition::default();
        cond_req.value_presence = Some(PresenceEnum::Present);
        
        let mut if_req = IfRequiredCondition::default();
        if_req.field = "name".to_string();
        if_req.condition = Some(cond_req);
        if_req.required_fields = vec!["created_at".to_string()];
        
        class.conditional_requirements.push(if_req);
    }
    
    schema
}

fn bench_simple_schemas(c: &mut Criterion) {
    let mut group = c.benchmark_group("typeql_simple_schemas");
    
    for size in [10, 50, 100, 500, 1000].iter() {
        let schema = create_schema(*size);
        let generator = EnhancedTypeQLGenerator::new();
        let options = GeneratorOptions {
            indent_style: IndentStyle::Spaces(2),
            ..Default::default()
        };
        
        group.bench_with_input(
            BenchmarkId::from_parameter(size),
            size,
            |b, _| {
                b.iter(|| {
                    let result = generator.generate(black_box(&schema), black_box(&options));
                    assert!(result.is_ok());
                });
            },
        );
    }
    
    group.finish();
}

fn bench_complex_relations(c: &mut Criterion) {
    let mut group = c.benchmark_group("typeql_complex_relations");
    
    for num_relations in [10, 25, 50, 100].iter() {
        let schema = create_complex_schema(*num_relations);
        let generator = EnhancedTypeQLGenerator::new();
        let options = GeneratorOptions::default();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(num_relations),
            num_relations,
            |b, _| {
                b.iter(|| {
                    let result = generator.generate(black_box(&schema), black_box(&options));
                    assert!(result.is_ok());
                });
            },
        );
    }
    
    group.finish();
}

fn bench_rule_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("typeql_rule_generation");
    
    for num_rules in [10, 50, 100, 200].iter() {
        let schema = create_rule_heavy_schema(*num_rules);
        let generator = EnhancedTypeQLGenerator::new();
        let options = GeneratorOptions::default();
        
        group.bench_with_input(
            BenchmarkId::from_parameter(num_rules),
            num_rules,
            |b, _| {
                b.iter(|| {
                    let result = generator.generate(black_box(&schema), black_box(&options));
                    assert!(result.is_ok());
                });
            },
        );
    }
    
    group.finish();
}

fn bench_memory_allocation(c: &mut Criterion) {
    c.bench_function("typeql_string_allocation_1000_classes", |b| {
        let schema = create_schema(1000);
        let generator = EnhancedTypeQLGenerator::new();
        let options = GeneratorOptions::default();
        
        b.iter(|| {
            // This measures the full generation including string allocation
            let result = generator.generate(black_box(&schema), black_box(&options));
            let output = result.unwrap();
            // Ensure the string is actually used
            assert!(output.content.len() > 0);
        });
    });
}

criterion_group!(
    benches,
    bench_simple_schemas,
    bench_complex_relations,
    bench_rule_generation,
    bench_memory_allocation
);
criterion_main!(benches);