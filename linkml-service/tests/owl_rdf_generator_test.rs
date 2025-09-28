//! Tests for OWL/RDF generation

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
    SubsetDefinition, TypeDefinition,
};
use linkml_service::generator::{Generator, GeneratorOptions, OwlRdfGenerator};
#[tokio::test]
async fn test_basic_owl_generation() {
    let mut schema = SchemaDefinition::new("person_schema");
    schema.id = "https://example.org/person".to_string();
    schema.version = Some("1.0.0".to_string());
    schema.description = Some("Schema for person data".to_string());

    // Add basic slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    name_slot.description = Some("Person's full name".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    age_slot.minimum_value = Some(0.0.into());
    age_slot.maximum_value = Some(150.0.into());
    schema.slots.insert("age".to_string(), age_slot);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[^@]+@[^@]+\.[^@]+$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    // Add a class
    let mut person_class = ClassDefinition::new("Person");
    person_class.description = Some("A human being".to_string());
    person_class.slots = vec!["name".to_string(), "age".to_string(), "email".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check filename
    assert_eq!(owl.filename, "person_schema.owl");

    // Check content
    assert!(owl.contains("# OWL Ontology generated from LinkML schema"));
    assert!(owl.contains("@prefix owl: <http://www.w3.org/2002/07/owl#>"));
    assert!(owl.contains("<https://example.org/person>"));
    assert!(owl.contains("a owl:Ontology"));
    assert!(owl.contains("owl:versionInfo \"1.0.0\""));

    // Check Person class
    assert!(owl.contains("person_schema:Person"));
    assert!(owl.contains("a owl:Class"));
    assert!(owl.contains("rdfs:label \"Person\""));
    assert!(owl.contains("skos:definition \"A human being\""));

    // Check properties
    assert!(owl.contains("person_schema:name"));
    assert!(owl.contains("a owl:DatatypeProperty"));
    assert!(owl.contains("rdfs:range xsd:string"));

    assert!(owl.contains("person_schema:age"));
    assert!(owl.contains("rdfs:range xsd:integer"));

    assert!(owl.contains("person_schema:email"));
    assert!(owl.contains("xsd:pattern \"^[^@]+@[^@]+\\.[^@]+$\""));

    // Check property restrictions
    assert!(owl.contains("owl:Restriction"));
    assert!(owl.contains("owl:onProperty person_schema:name"));
    assert!(owl.contains("owl:cardinality 1")); // required + not multivalued
}

#[tokio::test]
async fn test_enum_owl() {
    let mut schema = SchemaDefinition::new("status_schema");
    schema.id = "https://example.org/status".to_string();

    // Add enum
    let status_enum = EnumDefinition {
        name: "OrderStatus".to_string(),
        description: Some("Status of an order".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("pending".to_string()),
            PermissibleValue::Simple("processing".to_string()),
            PermissibleValue::Simple("shipped".to_string()),
            PermissibleValue::Simple("delivered".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("OrderStatus".to_string(), status_enum);

    // Add slot using enum
    let mut status_slot = SlotDefinition::new("status");
    status_slot.range = Some("OrderStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    // Add class
    let mut order_class = ClassDefinition::new("Order");
    order_class.slots = vec!["status".to_string()];
    schema.classes.insert("Order".to_string(), order_class);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check enum class
    assert!(owl.contains("status_schema:OrderStatus"));
    assert!(owl.contains("a owl:Class"));
    assert!(owl.contains("owl:equivalentClass"));
    assert!(owl.contains("owl:oneOf"));

    // Check individuals
    assert!(owl.contains("status_schema:OrderStatus_Pending"));
    assert!(owl.contains("status_schema:OrderStatus_Processing"));
    assert!(owl.contains("status_schema:OrderStatus_Shipped"));
    assert!(owl.contains("status_schema:OrderStatus_Delivered"));
    assert!(owl.contains("a status_schema:OrderStatus"));
}

#[tokio::test]
async fn test_inheritance_owl() {
    let mut schema = SchemaDefinition::new("entity_schema");
    schema.id = "https://example.org/entity".to_string();

    // Base entity slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    id_slot.required = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut created_slot = SlotDefinition::new("created_at");
    created_slot.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_slot);

    // Person-specific slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    // Base entity class
    let mut entity_class = ClassDefinition::new("Entity");
    entity_class.abstract_ = Some(true);
    entity_class.slots = vec!["id".to_string(), "created_at".to_string()];
    schema.classes.insert("Entity".to_string(), entity_class);

    // Person extends Entity
    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("Entity".to_string());
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check inheritance
    assert!(owl.contains("entity_schema:Person"));
    assert!(owl.contains("rdfs:subClassOf entity_schema:Entity"));

    // Check Person has restrictions for all properties (inherited + own)
    assert!(owl.contains("owl:onProperty entity_schema:id"));
    assert!(owl.contains("owl:onProperty entity_schema:created_at"));
    assert!(owl.contains("owl:onProperty entity_schema:name"));

    // Check datetime mapping
    assert!(owl.contains("rdfs:range xsd:dateTime"));
}

#[tokio::test]
async fn test_multivalued_owl() {
    let mut schema = SchemaDefinition::new("team_schema");
    schema.id = "https://example.org/team".to_string();

    // Add slots
    let mut name_slot = SlotDefinition::new("team_name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("team_name".to_string(), name_slot);

    let mut members_slot = SlotDefinition::new("members");
    members_slot.range = Some("string".to_string());
    members_slot.multivalued = Some(true);
    schema.slots.insert("members".to_string(), members_slot);

    // Add class
    let mut team_class = ClassDefinition::new("Team");
    team_class.slots = vec!["team_name".to_string(), "members".to_string()];
    schema.classes.insert("Team".to_string(), team_class);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check that single-valued properties are functional
    assert!(owl.contains("team_schema:team_name"));
    assert!(owl.contains("a owl:FunctionalProperty"));

    // Check that multivalued properties are not functional
    assert!(owl.contains("team_schema:members"));
    assert!(owl.contains("a owl:DatatypeProperty"));
    // Should NOT have FunctionalProperty for members
    let members_section = owl
        .split("team_schema:members")
        .nth(1)
        .expect("Test operation failed");
    let next_property = members_section
        .find("# Property:")
        .unwrap_or(members_section.len());
    let members_def = &members_section[..next_property];
    assert!(!members_def.contains("owl:FunctionalProperty"));
}

#[tokio::test]
async fn test_object_references_owl() {
    let mut schema = SchemaDefinition::new("org_schema");
    schema.id = "https://example.org/org".to_string();

    // Define Person class
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Define Organization class with reference to Person
    let mut org_class = ClassDefinition::new("Organization");
    org_class.slots = vec![
        "name".to_string(),
        "ceo".to_string(),
        "employees".to_string(),
    ];
    schema.classes.insert("Organization".to_string(), org_class);

    // Slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut ceo_slot = SlotDefinition::new("ceo");
    ceo_slot.range = Some("Person".to_string());
    schema.slots.insert("ceo".to_string(), ceo_slot);

    let mut employees_slot = SlotDefinition::new("employees");
    employees_slot.range = Some("Person".to_string());
    employees_slot.multivalued = Some(true);
    schema.slots.insert("employees".to_string(), employees_slot);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check object properties
    assert!(owl.contains("org_schema:ceo"));
    assert!(owl.contains("a owl:ObjectProperty"));
    assert!(owl.contains("rdfs:range org_schema:Person"));

    assert!(owl.contains("org_schema:employees"));
    let employees_section = owl
        .split("org_schema:employees")
        .nth(1)
        .expect("Test operation failed");
    assert!(employees_section.contains("a owl:ObjectProperty"));
    assert!(employees_section.contains("rdfs:range org_schema:Person"));
}

#[tokio::test]
async fn test_property_domains_owl() {
    let mut schema = SchemaDefinition::new("domain_schema");
    schema.id = "https://example.org/domain".to_string();

    // A shared slot used by multiple classes
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    // Classes that use the name slot
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    let mut org_class = ClassDefinition::new("Organization");
    org_class.slots = vec!["name".to_string()];
    schema.classes.insert("Organization".to_string(), org_class);

    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec!["name".to_string()];
    schema.classes.insert("Product".to_string(), product_class);

    // Generate OWL
    let generator = OwlRdfGenerator::new();
    let options = GeneratorOptions::default();
    let owl = generator.generate(&schema).expect("Test operation failed");

    // Check that name property has a union domain
    assert!(owl.contains("domain_schema:name"));
    assert!(owl.contains("rdfs:domain"));
    assert!(owl.contains("owl:unionOf"));
    assert!(owl.contains("domain_schema:Person domain_schema:Organization domain_schema:Product"));
}
