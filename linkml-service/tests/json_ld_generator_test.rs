//! Tests for JSON-LD generation

use linkml_core::types::SchemaDefinition;
use linkml_core::types::{
    ClassDefinition, Definition, EnumDefinition, PermissibleValue, SlotDefinition, TypeDefinition,
};
use linkml_core::types::{
    ClassDefinition, Element, EnumDefinition, SchemaDefinition, SlotDefinition, SubsetDefinition,
    TypeDefinition,
};
use linkml_service::generator::{Generator, GeneratorOptions, JsonLdGenerator};
use serde_json::{Value, json};
#[tokio::test]
async fn test_basic_json_ld_context() {
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
    schema.slots.insert("age".to_string(), age_slot);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    email_slot.multivalued = Some(true);
    schema.slots.insert("email".to_string(), email_slot);

    // Add a class
    let mut person_class = ClassDefinition::new("Person");
    person_class.description = Some("A human being".to_string());
    person_class.slots = vec!["name".to_string(), "age".to_string(), "email".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default().set_custom("pretty_print", "true");
    let output = generator.generate(&schema).expect("Test operation failed");

    let context: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check basic context structure
    assert_eq!(context["@vocab"], "https://example.org/person#");
    assert_eq!(context["xsd"], "http://www.w3.org/2001/XMLSchema#");
    assert_eq!(context["person_schema"], "https://example.org/person#");

    // Check class mapping
    assert_eq!(context["Person"], "person_schema:Person");

    // Check property mappings
    assert!(context["name"].is_object());
    assert_eq!(context["name"]["@id"], "person_schema:name");
    assert_eq!(context["name"]["@type"], "xsd:string");

    assert!(context["age"].is_object());
    assert_eq!(context["age"]["@id"], "person_schema:age");
    assert_eq!(context["age"]["@type"], "xsd:integer");

    // Check multivalued field
    assert!(context["email"].is_object());
    assert_eq!(context["email"]["@id"], "person_schema:email");
    assert_eq!(context["email"]["@container"], "@set");
}

#[tokio::test]
async fn test_json_ld_schema_document() {
    let mut schema = SchemaDefinition::new("org_schema");
    schema.id = "https://example.org/organization".to_string();

    // Add classes
    let person_class = ClassDefinition::new("Person");
    schema.classes.insert("Person".to_string(), person_class);

    let org_class = ClassDefinition::new("Organization");
    schema.classes.insert("Organization".to_string(), org_class);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let schema_doc: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check graph structure
    assert!(schema_doc["@context"].is_object());
    assert!(schema_doc["@graph"].is_array());

    let graph = schema_doc["@graph"]
        .as_array()
        .expect("Test operation failed");

    // Find schema metadata
    let schema_meta = graph
        .iter()
        .find(|item| item["@type"] == "owl:Ontology")
        .expect("Schema metadata not found");

    assert_eq!(schema_meta["@id"], "https://example.org/organization");
    assert_eq!(schema_meta["rdfs:label"], "org_schema");
}

#[tokio::test]
async fn test_enum_json_ld() {
    let mut schema = SchemaDefinition::new("status_schema");
    schema.id = "https://example.org/status".to_string();

    // Add enum
    let status_enum = EnumDefinition {
        name: "OrderStatus".to_string(),
        description: Some("Status of an order".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("pending".to_string()),
            PermissibleValue::Simple("shipped".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("OrderStatus".to_string(), status_enum);

    // Add slot using enum
    let mut status_slot = SlotDefinition::new("status");
    status_slot.range = Some("OrderStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let context: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check enum mapping
    assert_eq!(context["OrderStatus"], "status_schema:OrderStatus");

    // Check that enum range uses @id type
    assert!(context["status"].is_object());
    assert_eq!(context["status"]["@type"], "@id");
}

#[tokio::test]
async fn test_inheritance_json_ld() {
    let mut schema = SchemaDefinition::new("entity_schema");
    schema.id = "https://example.org/entity".to_string();

    // Base class
    let mut entity_class = ClassDefinition::new("Entity");
    entity_class.slots = vec!["id".to_string()];
    schema.classes.insert("Entity".to_string(), entity_class);

    // Derived class
    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("Entity".to_string());
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Slots
    let id_slot = SlotDefinition::new("id");
    schema.slots.insert("id".to_string(), id_slot);

    let name_slot = SlotDefinition::new("name");
    schema.slots.insert("name".to_string(), name_slot);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let schema_doc: Value = serde_json::from_str(&output).expect("Test operation failed");
    let graph = schema_doc["@graph"]
        .as_array()
        .expect("Test operation failed");

    // Find Person class
    let person = graph
        .iter()
        .find(|item| item["@id"] == "entity_schema:Person")
        .expect("Test operation failed");

    assert_eq!(person["rdfs:subClassOf"], "entity_schema:Entity");
}

#[tokio::test]
async fn test_object_references_json_ld() {
    let mut schema = SchemaDefinition::new("org_schema");
    schema.id = "https://example.org/org".to_string();

    // Classes
    let person_class = ClassDefinition::new("Person");
    schema.classes.insert("Person".to_string(), person_class);

    let mut org_class = ClassDefinition::new("Organization");
    org_class.slots = vec!["ceo".to_string()];
    schema.classes.insert("Organization".to_string(), org_class);

    // Object reference slot
    let mut ceo_slot = SlotDefinition::new("ceo");
    ceo_slot.range = Some("Person".to_string());
    schema.slots.insert("ceo".to_string(), ceo_slot);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let context: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check object reference uses @id type
    assert!(context["ceo"].is_object());
    assert_eq!(context["ceo"]["@type"], "@id");
}

#[tokio::test]
async fn test_json_ld_frames() {
    let mut schema = SchemaDefinition::new("person_schema");
    schema.id = "https://example.org/person".to_string();

    // Add slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let age_slot = SlotDefinition::new("age");
    schema.slots.insert("age".to_string(), age_slot);

    // Add class
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let frame: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check frame structure
    assert_eq!(frame["@type"], "person_schema:Person");

    // Required fields should have @default
    assert!(frame["name"].is_object());
    assert_eq!(frame["name"]["@default"], json!(null));
}

#[tokio::test]
async fn test_json_ld_examples() {
    let mut schema = SchemaDefinition::new("person_schema");
    schema.id = "https://example.org/person".to_string();

    // Add slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut age_slot = SlotDefinition::new("age");
    age_slot.range = Some("integer".to_string());
    schema.slots.insert("age".to_string(), age_slot);

    // Add class
    let mut person_class = ClassDefinition::new("Person");
    person_class.slots = vec!["name".to_string(), "age".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Generate JSON-LD with examples
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default().set_custom("generate_examples", "true");
    let output = generator.generate(&schema).expect("Test operation failed");

    let example: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check example structure
    assert_eq!(example["@context"], "person_schema.context.jsonld");
    assert_eq!(example["@type"], "Person");
    assert!(
        example["@id"]
            .as_str()
            .expect("Test operation failed")
            .contains("example-person")
    );
    assert_eq!(example["name"], "example string");
}

#[tokio::test]
async fn test_custom_types_json_ld() {
    let mut schema = SchemaDefinition::new("product_schema");
    schema.id = "https://example.org/product".to_string();

    // Add custom type
    let url_type = TypeDefinition {
        name: "URL".to_string(),
        base_type: Some("string".to_string()),
        pattern: Some(r"^https?://".to_string()),
        ..Default::default()
    };
    schema.types.insert("URL".to_string(), url_type);

    // Add slot using custom type
    let mut website_slot = SlotDefinition::new("website");
    website_slot.range = Some("URL".to_string());
    schema.slots.insert("website".to_string(), website_slot);

    // Add class
    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec!["website".to_string()];
    schema.classes.insert("Product".to_string(), product_class);

    // Generate JSON-LD
    let generator = JsonLdGenerator::new();
    let options = GeneratorOptions::default();
    let output = generator.generate(&schema).expect("Test operation failed");

    let context: Value = serde_json::from_str(&output).expect("Test operation failed");

    // Check custom type resolves to base type
    assert!(context["website"].is_object());
    assert_eq!(context["website"]["@type"], "xsd:string");
}
