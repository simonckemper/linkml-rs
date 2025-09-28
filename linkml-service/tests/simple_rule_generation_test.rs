//! Simple test for TypeQL rule generation

use linkml_core::prelude::*;
use linkml_service::generator::typeql_rule_generator::{RuleGenerator, RuleType};
use serde_json::Value;

#[test]
fn test_basic_rule_generation() {
    let mut rule_generator = RuleGenerator::new();

    // Create a simple schema
    let mut schema = SchemaDefinition::default();

    // Create a Person class with slots
    let mut person = ClassDefinition::default();
    person.description = Some("A person".to_string());

    // Add a required name slot
    person.slots.push("name".to_string());
    let mut name_slot = SlotDefinition::default();
    name_slot.required = Some(true);
    person.slot_usage.insert("name".to_string(), name_slot);

    // Add an age slot with range constraint
    person.slots.push("age".to_string());
    let mut age_slot = SlotDefinition::default();
    age_slot.minimum_value = Some(Value::Number(serde_json::Number::from(0)));
    age_slot.maximum_value = Some(Value::Number(serde_json::Number::from(150)));
    person.slot_usage.insert("age".to_string(), age_slot);

    // Generate rules for the class
    let rules = rule_generator.generate_class_rules("Person", &person, &schema);

    // Check that we got some rules
    assert!(!rules.is_empty());

    // Check for required field rule
    let required_rule = rules
        .iter()
        .find(|r| r.rule_type == RuleType::Validation && r.name.contains("requires-name"))
        .expect("Should have generated a required field rule");

    println!("Required rule: {}", required_rule.to_typeql());
    assert!(
        required_rule
            .to_typeql()
            .contains("not { $x has name $v; }")
    );
    assert!(required_rule.to_typeql().contains("validation-error"));

    // Check for range rule
    let range_rule = rules
        .iter()
        .find(|r| r.rule_type == RuleType::Validation && r.name.contains("age-range"))
        .expect("Should have generated a range rule");

    println!("Range rule: {}", range_rule.to_typeql());
    assert!(
        range_rule.to_typeql().contains("$v < 0") || range_rule.to_typeql().contains("$v > 150")
    );
}

#[test]
fn test_conditional_requirement_rule() {
    let mut rule_generator = RuleGenerator::new();
    let schema = SchemaDefinition::default();

    // Create a Person class with conditional requirements
    let mut person = ClassDefinition::default();
    person.slots.push("employment_status".to_string());
    person.slots.push("employer".to_string());

    // Add conditional requirement: if employment_status = "employed", then employer is required
    let mut conditional = ConditionalRequirement::default();
    conditional.condition = Some(SlotCondition {
        equals_string: Some("employed".to_string()),
        ..Default::default()
    });
    conditional.then_required = Some(vec!["employer".to_string()]);

    person.if_required = Some(indexmap::indexmap! {
        "employment_status".to_string() => conditional
    });

    // Generate rules
    let rules = rule_generator.generate_class_rules("Person", &person, &schema);

    // Check for conditional rule
    let conditional_rule = rules
        .iter()
        .find(|r| r.name.contains("conditional"))
        .expect("Should have generated a conditional rule");

    println!("Conditional rule: {}", conditional_rule.to_typeql());
    assert!(
        conditional_rule
            .to_typeql()
            .contains("$x has employment_status $cond")
    );
    assert!(
        conditional_rule
            .to_typeql()
            .contains("$cond = \"employed\"")
    );
    assert!(
        conditional_rule
            .to_typeql()
            .contains("not { $x has employer $v; }")
    );
}

#[test]
fn test_rule_typeql_output() {
    use linkml_core::types::{
        ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition,
        SubsetDefinition, TypeDefinition,
    };
    use linkml_service::generator::typeql_rule_generator::TypeQLRule;

    let rule = TypeQLRule {
        name: "person-requires-name".to_string(),
        rule_type: RuleType::Validation,
        when_patterns: vec![
            "$x isa person".to_string(),
            "not { $x has name $v; }".to_string(),
        ],
        then_patterns: vec!["$x has validation-error \"Missing required field: name\"".to_string()],
        description: Some("Validates that name is required".to_string()),
        dependencies: vec![],
    };

    let typeql = rule.to_typeql();
    println!("Generated TypeQL:
{}", typeql);

    // Check structure
    assert!(typeql.contains("# Validates that name is required"));
    assert!(typeql.contains("rule person-requires-name:"));
    assert!(typeql.contains("when {"));
    assert!(typeql.contains("    $x isa person;"));
    assert!(typeql.contains("    not { $x has name $v; }"));
    assert!(typeql.contains("} then {"));
    assert!(typeql.contains("    $x has validation-error \"Missing required field: name\""));
    assert!(typeql.contains("};"));
}
