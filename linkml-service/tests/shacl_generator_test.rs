//! Tests for SHACL generation

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
    TypeDefinition,
};
use linkml_service::generator::{Generator, GeneratorOptions, ShaclGenerator};
#[tokio::test]
async fn test_basic_shacl_generation() {
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

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Check content
    assert!(shacl.contains("# SHACL Shapes generated from LinkML schema"));
    assert!(shacl.contains("@prefix sh: <http://www.w3.org/ns/shacl#>"));
    assert!(shacl.contains("@prefix person_schema: <https://example.org/person#>"));

    // Check PersonShape
    assert!(shacl.contains("person_schema:PersonShape"));
    assert!(shacl.contains("a sh:NodeShape"));
    assert!(shacl.contains("sh:targetClass person_schema:Person"));
    assert!(shacl.contains("rdfs:comment \"A human being\""));

    // Check property shapes
    assert!(shacl.contains("sh:path person_schema:name"));
    assert!(shacl.contains("sh:datatype xsd:string"));
    assert!(shacl.contains("sh:minCount 1")); // required
    assert!(shacl.contains("sh:maxCount 1")); // not multivalued

    assert!(shacl.contains("sh:path person_schema:age"));
    assert!(shacl.contains("sh:datatype xsd:integer"));
    assert!(shacl.contains("sh:minInclusive 0"));
    assert!(shacl.contains("sh:maxInclusive 150"));

    assert!(shacl.contains("sh:path person_schema:email"));
    assert!(shacl.contains("sh:pattern \"^[^@]+@[^@]+\\.[^@]+$\""));
}

#[tokio::test]
async fn test_enum_shacl() {
    let mut schema = SchemaDefinition::new("status_schema");

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

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Check enum constraint using sh:in
    assert!(shacl.contains("sh:path status_schema:status"));
    assert!(shacl.contains("sh:in (\"pending\" \"processing\" \"shipped\" \"delivered\")"));
}

#[tokio::test]
async fn test_inheritance_shacl() {
    let mut schema = SchemaDefinition::new("entity_schema");

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

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Check PersonShape has all properties (inherited + own)
    assert!(shacl.contains("entity_schema:PersonShape"));
    assert!(shacl.contains("entity_schema:PersonShape-id"));
    assert!(shacl.contains("entity_schema:PersonShape-created_at"));
    assert!(shacl.contains("entity_schema:PersonShape-name"));

    // Check datetime mapping
    assert!(shacl.contains("sh:datatype xsd:dateTime"));
}

#[tokio::test]
async fn test_multivalued_shacl() {
    let mut schema = SchemaDefinition::new("team_schema");

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

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Debug print to see the actual content
    println!("Generated SHACL:
{}", shacl.content);

    // Check that multivalued fields don't have maxCount
    assert!(shacl.contains("sh:path team_schema:team_name"));
    assert!(shacl.contains("sh:maxCount 1")); // single valued

    assert!(shacl.contains("sh:path team_schema:members"));
    // Check that members property shape doesn't have maxCount (since it's multivalued)
    let members_section = shacl
        .split("team_schema:TeamShape-members")
        .nth(1)
        .unwrap_or("");
    let next_section_start = members_section
        .find("team_schema:")
        .unwrap_or(members_section.len());
    let members_shape_content = &members_section[..next_section_start];
    assert!(
        !members_shape_content.contains("sh:maxCount"),
        "Multivalued field should not have maxCount"
    );
}

#[tokio::test]
async fn test_object_references_shacl() {
    let mut schema = SchemaDefinition::new("org_schema");

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

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Check object references use sh:class
    assert!(shacl.contains("sh:path org_schema:ceo"));
    assert!(shacl.contains("sh:class org_schema:Person"));

    assert!(shacl.contains("sh:path org_schema:employees"));
    assert!(
        shacl
            .split("org_schema:OrganizationShape-employees")
            .nth(1)
            .expect("Test operation failed")
            .contains("sh:class org_schema:Person")
    );
}

#[tokio::test]
async fn test_custom_types_shacl() {
    let mut schema = SchemaDefinition::new("product_schema");

    // Add custom types
    let url_type = TypeDefinition {
        name: "URL".to_string(),
        description: Some("URL type".to_string()),
        base_type: Some("string".to_string()),
        pattern: Some(r"^https?://".to_string()),
        ..Default::default()
    };
    schema.types.insert("URL".to_string(), url_type);

    let price_type = TypeDefinition {
        name: "Price".to_string(),
        description: Some("Positive decimal price".to_string()),
        base_type: Some("float".to_string()),
        minimum_value: Some(0.0.into()),
        ..Default::default()
    };
    schema.types.insert("Price".to_string(), price_type);

    // Add slots using custom types
    let mut website_slot = SlotDefinition::new("website");
    website_slot.range = Some("URL".to_string());
    schema.slots.insert("website".to_string(), website_slot);

    let mut price_slot = SlotDefinition::new("price");
    price_slot.range = Some("Price".to_string());
    schema.slots.insert("price".to_string(), price_slot);

    // Add class
    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec!["website".to_string(), "price".to_string()];
    schema.classes.insert("Product".to_string(), product_class);

    // Generate SHACL
    let generator = ShaclGenerator::new();
    let options = GeneratorOptions::default();
    let shacl = generator.generate(&schema).expect("Test operation failed");

    // Debug print
    println!("Generated SHACL for custom types:
{}", shacl.content);

    // Check custom types are resolved to base types with constraints
    assert!(shacl.contains("sh:path product_schema:website"));
    assert!(shacl.contains("sh:datatype xsd:string"));
    assert!(shacl.contains("sh:pattern \"^https?://\""));

    assert!(shacl.contains("sh:path product_schema:price"));
    assert!(shacl.contains("sh:datatype xsd:double"));
    assert!(shacl.contains("sh:minInclusive 0"));
}
