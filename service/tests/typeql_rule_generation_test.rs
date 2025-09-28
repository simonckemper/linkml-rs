//! Integration tests for TypeQL rule generation

use linkml_core::prelude::*;
use linkml_service::generator::options::GeneratorOptions;
use linkml_service::generator::traits::Generator;
use linkml_service::generator::typeql_generator_enhanced::EnhancedTypeQLGenerator;
use std::collections::HashMap;

/// Create a test schema with various rule types
fn create_test_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = "test_rule_schema".to_string();

    // Person class with various constraints and rules
    let mut person = ClassDefinition::default();
    person.description = Some("A person with validation rules".to_string());

    // Add slots
    person.slots.push("name".to_string());
    person.slots.push("age".to_string());
    person.slots.push("employment_status".to_string());
    person.slots.push("employer".to_string());
    person.slots.push("age_category".to_string());
    person.slots.push("is_adult".to_string());

    // Add slot usage with constraints
    let mut name_slot = SlotDefinition::default();
    name_slot.required = Some(true);
    name_slot.pattern = Some(r"^[A-Za-z ]+$".to_string());
    person.slot_usage.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::default();
    age_slot.minimum_value = Some(Value::Integer(0));
    age_slot.maximum_value = Some(Value::Integer(150));
    person.slot_usage.insert("age".to_string(), age_slot);

    // Add computed slot
    let mut age_category_slot = SlotDefinition::default();
    age_category_slot.equals_expression =
        Some(r#"case({age} < 18, "minor", {age} < 65, "adult", "senior")"#.to_string());
    person
        .slot_usage
        .insert("age_category".to_string(), age_category_slot);

    // Add derived boolean
    let mut is_adult_slot = SlotDefinition::default();
    is_adult_slot.equals_expression = Some("{age} >= 18".to_string());
    person
        .slot_usage
        .insert("is_adult".to_string(), is_adult_slot);

    // Add conditional requirement
    let mut conditional = ConditionalRequirement::default();
    conditional.if_required.slot = Some("employment_status".to_string());
    conditional.if_required.equals_string = Some("employed".to_string());
    conditional.then_required = vec!["employer".to_string()];
    person.conditional_requirements.push(conditional);

    // Add custom rule
    let mut custom_rule = Rule::default();
    custom_rule.name = "valid_age_for_employment".to_string();
    custom_rule.description = Some("Must be at least 16 to be employed".to_string());
    person.rules.push(custom_rule);

    schema.classes.insert("Person".to_string(), person);

    // Define slots in schema
    let mut name_def = SlotDefinition::default();
    name_def.description = Some("Person's name".to_string());
    name_def.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_def);

    let mut age_def = SlotDefinition::default();
    age_def.description = Some("Person's age".to_string());
    age_def.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_def);

    let mut employment_status_def = SlotDefinition::default();
    employment_status_def.description = Some("Employment status".to_string());
    employment_status_def.range = Some("string".to_string());
    schema
        .slots
        .insert("employment_status".to_string(), employment_status_def);

    let mut employer_def = SlotDefinition::default();
    employer_def.description = Some("Employer name".to_string());
    employer_def.range = Some("string".to_string());
    schema.slots.insert("employer".to_string(), employer_def);

    let mut age_category_def = SlotDefinition::default();
    age_category_def.description = Some("Age category".to_string());
    age_category_def.range = Some("string".to_string());
    schema
        .slots
        .insert("age_category".to_string(), age_category_def);

    let mut is_adult_def = SlotDefinition::default();
    is_adult_def.description = Some("Is adult".to_string());
    is_adult_def.range = Some("boolean".to_string());
    schema.slots.insert("is_adult".to_string(), is_adult_def);

    schema
}

#[tokio::test]
async fn test_rule_generation() {
    let schema = create_test_schema();
    let generator = EnhancedTypeQLGenerator::new();

    let result = generator.generate(&schema).expect("Test operation failed");
    let typeql = result;

    println!("Generated TypeQL:
{}", typeql);

    // Check for required field validation rule
    assert!(typeql.contains("rule person-requires-name:"));
    assert!(typeql.contains("not { $x has name $v; }"));
    assert!(typeql.contains("validation-error \"Missing required field: name\""));

    // Check for pattern validation rule
    assert!(typeql.contains("rule person-name-pattern:"));
    assert!(typeql.contains("not { $v like \"^[A-Za-z ]+$\"; }"));

    // Check for range validation rule
    assert!(typeql.contains("rule person-age-range:"));
    assert!(typeql.contains("$v < 0") || typeql.contains("$v > 150"));

    // Check for conditional requirement rule
    assert!(typeql.contains("rule person-conditional-"));
    assert!(typeql.contains("$x has employment-status \"employed\""));
    assert!(typeql.contains("not { $x has employer $v; }"));

    // Check for computed attribute rule
    assert!(typeql.contains("rule person-compute-is-adult:"));
    assert!(typeql.contains("$a >= 18"));
    assert!(typeql.contains("$x has is-adult"));

    // The complex case expression should be mentioned as needing decomposition
    // (since it generates multiple rules)
    assert!(typeql.contains("TODO") || typeql.contains("case"));
}

#[tokio::test]
async fn test_multiple_classes_with_rules() {
    let mut schema = SchemaDefinition::default();
    schema.name = "multi_class_schema".to_string();

    // Create Employee class
    let mut employee = ClassDefinition::default();
    employee.is_a = Some("Person".to_string());
    employee.slots.push("employee_id".to_string());
    employee.slots.push("department".to_string());
    employee.slots.push("salary".to_string());

    // Add required employee_id
    let mut employee_id_slot = SlotDefinition::default();
    employee_id_slot.required = Some(true);
    employee_id_slot.pattern = Some(r"^EMP\d{6}$".to_string());
    employee
        .slot_usage
        .insert("employee_id".to_string(), employee_id_slot);

    // Add salary range constraint
    let mut salary_slot = SlotDefinition::default();
    salary_slot.minimum_value = Some(Value::Number(
        serde_json::Number::from_f64(0.0).expect("Test operation failed"),
    ));
    salary_slot.maximum_value = Some(Value::Number(
        serde_json::Number::from_f64(1000000.0).expect("Test operation failed"),
    ));
    employee
        .slot_usage
        .insert("salary".to_string(), salary_slot);

    schema.classes.insert("Employee".to_string(), employee);

    // Create Department class with unique constraint
    let mut department = ClassDefinition::default();
    department.slots.push("dept_code".to_string());
    department.slots.push("name".to_string());

    // Add unique key
    use indexmap::IndexMap;
    use linkml_core::types::{
        ClassDefinition, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
        TypeDefinition,
    };
    let mut unique_keys_map: IndexMap<String, linkml_core::types::UniqueKeyDefinition> =
        IndexMap::new();
    unique_keys_map.insert(
        "dept_key".to_string(),
        linkml_core::types::UniqueKeyDefinition {
            description: Some("Department code uniqueness".to_string()),
            unique_key_slots: vec!["dept_code".to_string()],
            consider_nulls_inequal: Some(true),
        },
    );
    department.unique_keys = unique_keys_map;

    schema.classes.insert("Department".to_string(), department);

    // Add slots to schema
    let mut employee_id_def = SlotDefinition::default();
    employee_id_def.range = Some("string".to_string());
    schema
        .slots
        .insert("employee_id".to_string(), employee_id_def);

    let mut department_def = SlotDefinition::default();
    department_def.range = Some("string".to_string());
    schema
        .slots
        .insert("department".to_string(), department_def);

    let mut salary_def = SlotDefinition::default();
    salary_def.range = Some("float".to_string());
    schema.slots.insert("salary".to_string(), salary_def);

    let mut dept_code_def = SlotDefinition::default();
    dept_code_def.range = Some("string".to_string());
    schema.slots.insert("dept_code".to_string(), dept_code_def);

    let mut name_def = SlotDefinition::default();
    name_def.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_def);

    let generator = EnhancedTypeQLGenerator::new();

    let result = generator.generate(&schema).expect("Test operation failed");
    let typeql = result;

    // Check Employee rules
    assert!(typeql.contains("## Validation Rules for Employee"));
    assert!(typeql.contains("rule employee-requires-employee-id:"));
    assert!(typeql.contains("rule employee-employee-id-pattern:"));
    assert!(typeql.contains("rule employee-salary-range:"));

    // Check Department doesn't generate unnecessary rules
    // (unique constraint is handled by @key, not rules)
    assert!(
        !typeql.contains("## Validation Rules for Department") || typeql.contains("dept_code @key")
    );
}

#[tokio::test]
async fn test_expression_based_rules() {
    let mut schema = SchemaDefinition::default();
    schema.name = "expression_schema".to_string();

    let mut product = ClassDefinition::default();
    product.slots.push("price".to_string());
    product.slots.push("tax_rate".to_string());
    product.slots.push("total_price".to_string());
    product.slots.push("is_luxury".to_string());

    // Add computed total price (requires arithmetic - should be marked as complex)
    let mut total_price_slot = SlotDefinition::default();
    total_price_slot.equals_expression = Some("{price} * (1 + {tax_rate})".to_string());
    product
        .slot_usage
        .insert("total_price".to_string(), total_price_slot);

    // Add simple boolean derivation
    let mut is_luxury_slot = SlotDefinition::default();
    is_luxury_slot.equals_expression = Some("{price} > 1000".to_string());
    product
        .slot_usage
        .insert("is_luxury".to_string(), is_luxury_slot);

    schema.classes.insert("Product".to_string(), product);

    // Define slots
    let mut price_def = SlotDefinition::default();
    price_def.range = Some("float".to_string());
    schema.slots.insert("price".to_string(), price_def);

    let mut tax_rate_def = SlotDefinition::default();
    tax_rate_def.range = Some("float".to_string());
    schema.slots.insert("tax_rate".to_string(), tax_rate_def);

    let mut total_price_def = SlotDefinition::default();
    total_price_def.range = Some("float".to_string());
    schema
        .slots
        .insert("total_price".to_string(), total_price_def);

    let mut is_luxury_def = SlotDefinition::default();
    is_luxury_def.range = Some("boolean".to_string());
    schema.slots.insert("is_luxury".to_string(), is_luxury_def);

    let generator = EnhancedTypeQLGenerator::new();

    let result = generator.generate(&schema).expect("Test operation failed");
    let typeql = result;

    // Check simple expression rule
    assert!(typeql.contains("rule product-compute-is-luxury:"));
    assert!(typeql.contains("$x has price $v"));
    assert!(typeql.contains("$v") && typeql.contains("> 1000"));
    assert!(typeql.contains("$x has is-luxury"));

    // Complex arithmetic expressions should not generate rules
    // (marked as requiring computed attributes)
    assert!(
        !typeql.contains("rule product-compute-total-price:")
            || typeql.contains("TODO")
            || typeql.contains("computed attribute")
    );
}
