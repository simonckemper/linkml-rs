//! Tests for parsing rules from YAML schemas

use linkml_service::parser::{SchemaParser, YamlParser};

#[test]
fn test_parse_simple_rule() {
    let yaml_content = r#"
id: https://example.org/test-schema
name: test_schema
title: Test Schema with Rules

classes:
  Person:
    name: Person
    description: A person with age-based rules
    slots:
      - age
      - guardian_name
      - guardian_phone
    rules:
      - description: Minors require guardian information
        priority: 100
        preconditions:
          slot_conditions:
            age:
              maximum_value: 17
        postconditions:
          slot_conditions:
            guardian_name:
              required: true
            guardian_phone:
              required: true

slots:
  age:
    name: age
    description: Person's age
    range: integer
  guardian_name:
    name: guardian_name
    description: Guardian's name
    range: string
  guardian_phone:
    name: guardian_phone
    description: Guardian's phone number
    range: string
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(yaml_content).expect("Failed to parse schema");

    // Verify schema basics
    assert_eq!(schema.name, "test_schema");

    // Check that Person class exists
    let person_class = schema
        .classes
        .get("Person")
        .expect("Person class not found");

    // Verify rule was parsed
    assert_eq!(person_class.rules.len(), 1);
    let rule = &person_class.rules[0];

    assert_eq!(
        rule.description.as_deref(),
        Some("Minors require guardian information")
    );
    assert_eq!(rule.priority, Some(100));

    // Check preconditions
    assert!(rule.preconditions.is_some());
    let preconditions = rule.preconditions.as_ref().expect("Test operation failed");
    assert!(preconditions.slot_conditions.is_some());
    let slot_conditions = preconditions
        .slot_conditions
        .as_ref()
        .expect("Test operation failed");

    let age_condition = slot_conditions.get("age").expect("age condition not found");
    assert_eq!(age_condition.maximum_value, Some(serde_json::json!(17)));

    // Check postconditions
    assert!(rule.postconditions.is_some());
    let postconditions = rule.postconditions.as_ref().expect("Test operation failed");
    assert!(postconditions.slot_conditions.is_some());
    let post_slot_conditions = postconditions
        .slot_conditions
        .as_ref()
        .expect("Test operation failed");

    let guardian_name_condition = post_slot_conditions
        .get("guardian_name")
        .expect("guardian_name condition not found");
    assert_eq!(guardian_name_condition.required, Some(true));
}

#[test]
fn test_parse_complex_rules() {
    let yaml_content = r#"
id: https://example.org/complex-rules
name: complex_rules_schema

classes:
  Order:
    name: Order
    slots:
      - total_amount
      - status
      - approval_required
      - approved_by
    rules:
      # Expression-based rule
      - description: High value orders need approval
        priority: 50
        preconditions:
          expression_conditions:
            - "{total_amount} > 10000"
        postconditions:
          slot_conditions:
            approval_required:
              equals_string: "yes"

      # Composite conditions
      - description: Approved orders must have approver
        priority: 40
        preconditions:
          composite_conditions:
            all_of:
              - slot_conditions:
                  status:
                    equals_string: "approved"
              - slot_conditions:
                  approval_required:
                    equals_string: "yes"
        postconditions:
          slot_conditions:
            approved_by:
              required: true

      # Else conditions
      - description: Cancelled orders cleared
        priority: 30
        preconditions:
          slot_conditions:
            status:
              equals_string: "cancelled"
        postconditions:
          slot_conditions:
            total_amount:
              equals_number: 0.0
        else_conditions:
          slot_conditions:
            total_amount:
              minimum_value: 0.01

slots:
  total_amount:
    name: total_amount
    range: float
  status:
    name: status
    range: string
  approval_required:
    name: approval_required
    range: string
  approved_by:
    name: approved_by
    range: string
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Failed to parse complex schema");

    let order_class = schema.classes.get("Order").expect("Order class not found");
    assert_eq!(order_class.rules.len(), 3);

    // Test expression-based rule
    let expr_rule = &order_class.rules[0];
    assert!(expr_rule.preconditions.is_some());
    let preconditions = expr_rule
        .preconditions
        .as_ref()
        .expect("Test operation failed");
    assert!(preconditions.expression_conditions.is_some());
    let expressions = preconditions
        .expression_conditions
        .as_ref()
        .expect("Test operation failed");
    assert_eq!(expressions.len(), 1);
    assert_eq!(expressions[0], "{total_amount} > 10000");

    // Test composite conditions
    let composite_rule = &order_class.rules[1];
    assert!(composite_rule.preconditions.is_some());
    let preconditions = composite_rule
        .preconditions
        .as_ref()
        .expect("Test operation failed");
    assert!(preconditions.composite_conditions.is_some());
    let composite = preconditions
        .composite_conditions
        .as_ref()
        .expect("Test operation failed");
    assert!(composite.all_of.is_some());

    // Test else conditions
    let else_rule = &order_class.rules[2];
    assert!(else_rule.else_conditions.is_some());
    let else_conditions = else_rule
        .else_conditions
        .as_ref()
        .expect("Test operation failed");
    assert!(else_conditions.slot_conditions.is_some());
}

#[test]
fn test_parse_conditional_requirements() {
    let yaml_content = r#"
id: https://example.org/conditional-req
name: conditional_requirements_schema

classes:
  Address:
    name: Address
    slots:
      - country
      - state
      - province
      - postal_code
    if_required:
      country_us:
        condition:
          equals_string: "USA"
        then_required:
          - state
          - postal_code
      country_ca:
        condition:
          equals_string: "Canada"
        then_required:
          - province
          - postal_code

slots:
  country:
    name: country
    range: string
  state:
    name: state
    range: string
  province:
    name: province
    range: string
  postal_code:
    name: postal_code
    range: string
"#;

    let parser = YamlParser::new();
    let schema = parser
        .parse(yaml_content)
        .expect("Failed to parse conditional requirements");

    let address_class = schema
        .classes
        .get("Address")
        .expect("Address class not found");
    assert!(address_class.if_required.is_some());

    let if_required = address_class
        .if_required
        .as_ref()
        .expect("Test operation failed");
    assert_eq!(if_required.len(), 2);

    // Check US requirements
    let us_req = if_required
        .get("country_us")
        .expect("country_us requirement not found");
    assert!(us_req.condition.is_some());
    let condition = us_req.condition.as_ref().expect("Test operation failed");
    assert_eq!(condition.equals_string.as_deref(), Some("USA"));

    assert!(us_req.then_required.is_some());
    let then_required = us_req
        .then_required
        .as_ref()
        .expect("Test operation failed");
    assert_eq!(then_required.len(), 2);
    assert!(then_required.contains(&"state".to_string()));
    assert!(then_required.contains(&"postal_code".to_string()));
}
