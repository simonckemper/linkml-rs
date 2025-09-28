//! Tests for Protocol Buffers code generation

use linkml_core::types::{
    ClassDefinition, EnumDefinition, PermissibleValue, SchemaDefinition, SlotDefinition,
    SubsetDefinition, TypeDefinition,
};
use linkml_service::generator::{Generator, GeneratorOptions, ProtobufGenerator};
#[tokio::test]
async fn test_basic_protobuf_generation() {
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

    // Generate protobuf
    let generator = ProtobufGenerator::new();
    let options = GeneratorOptions::default();
    let proto = generator.generate(&schema).expect("Test operation failed");

    // Check content
    assert!(proto.contains("syntax = \"proto3\""));
    assert!(proto.contains("package person_schema"));
    assert!(proto.contains("// A human being"));
    assert!(proto.contains("message Person {"));
    assert!(proto.contains("// Person's full name"));
    assert!(proto.contains("string name = 1"));
    assert!(proto.contains("int64 age = 2"));
    assert!(proto.contains("string email = 3"));
    assert!(proto.contains("}"));
}

#[tokio::test]
async fn test_enums_and_repeated_fields() {
    let mut schema = SchemaDefinition::new("employee_schema");
    schema.id = "https://example.org/employee".to_string();

    // Add enum for department
    let department_enum = EnumDefinition {
        name: "Department".to_string(),
        description: Some("Company departments".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("engineering".to_string()),
            PermissibleValue::Simple("marketing".to_string()),
            PermissibleValue::Simple("sales".to_string()),
            PermissibleValue::Complex {
                text: "hr".to_string(),
                description: Some("Human Resources".to_string()),
                meaning: None,
            },
        ],
        ..Default::default()
    };
    schema
        .enums
        .insert("Department".to_string(), department_enum);

    // Add slots
    let mut id_slot = SlotDefinition::new("employee_id");
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("employee_id".to_string(), id_slot);

    let mut dept_slot = SlotDefinition::new("department");
    dept_slot.range = Some("Department".to_string());
    schema.slots.insert("department".to_string(), dept_slot);

    let mut skills_slot = SlotDefinition::new("skills");
    skills_slot.range = Some("string".to_string());
    skills_slot.multivalued = Some(true);
    schema.slots.insert("skills".to_string(), skills_slot);

    let mut projects_slot = SlotDefinition::new("project_ids");
    projects_slot.range = Some("integer".to_string());
    projects_slot.multivalued = Some(true);
    schema
        .slots
        .insert("project_ids".to_string(), projects_slot);

    // Add employee class
    let mut employee_class = ClassDefinition::new("Employee");
    employee_class.slots = vec![
        "employee_id".to_string(),
        "department".to_string(),
        "skills".to_string(),
        "project_ids".to_string(),
    ];
    schema
        .classes
        .insert("Employee".to_string(), employee_class);

    // Generate protobuf
    let generator = ProtobufGenerator::new();
    let options = GeneratorOptions::default();
    let proto = generator.generate(&schema).expect("Test operation failed");

    // Check enum generation
    assert!(proto.contains("enum Department {"));
    assert!(proto.contains("DEPARTMENT_UNSPECIFIED = 0"));
    assert!(proto.contains("ENGINEERING = 1"));
    assert!(proto.contains("MARKETING = 2"));
    assert!(proto.contains("SALES = 3"));
    assert!(proto.contains("HR = 4"));

    // Check message with repeated fields
    assert!(proto.contains("message Employee {"));
    assert!(proto.contains("string employee_id = 1"));
    assert!(proto.contains("Department department = 2"));
    assert!(proto.contains("repeated string skills = 3"));
    assert!(proto.contains("repeated int64 project_ids = 4"));
}

#[tokio::test]
async fn test_inheritance_and_composition() {
    let mut schema = SchemaDefinition::new("org_schema");

    // Base entity class
    let mut entity_class = ClassDefinition::new("Entity");
    entity_class.abstract_ = Some(true);
    entity_class.slots = vec!["id".to_string(), "created_at".to_string()];
    schema.classes.insert("Entity".to_string(), entity_class);

    // Person extends Entity
    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("Entity".to_string());
    person_class.slots = vec!["name".to_string(), "email".to_string()];
    schema.classes.insert("Person".to_string(), person_class);

    // Organization extends Entity
    let mut org_class = ClassDefinition::new("Organization");
    org_class.is_a = Some("Entity".to_string());
    org_class.slots = vec!["name".to_string(), "members".to_string()];
    schema.classes.insert("Organization".to_string(), org_class);

    // Add slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut created_slot = SlotDefinition::new("created_at");
    created_slot.range = Some("datetime".to_string());
    schema.slots.insert("created_at".to_string(), created_slot);

    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut members_slot = SlotDefinition::new("members");
    members_slot.range = Some("Person".to_string());
    members_slot.multivalued = Some(true);
    schema.slots.insert("members".to_string(), members_slot);

    // Generate protobuf
    let generator = ProtobufGenerator::new();
    let options = GeneratorOptions::default();
    let proto = generator.generate(&schema).expect("Test operation failed");

    // Check that timestamp import is added for datetime
    assert!(proto.contains("import \"google/protobuf/timestamp.proto\""));

    // Check Person has inherited fields
    assert!(proto.contains("message Person {"));
    assert!(proto.contains("string id = 1"));
    assert!(proto.contains("string created_at = 2")); // datetime becomes string
    assert!(proto.contains("string name = 3"));
    assert!(proto.contains("string email = 4"));

    // Check Organization
    assert!(proto.contains("message Organization {"));
    assert!(proto.contains("repeated Person members = 4"));
}

#[tokio::test]
async fn test_complex_types() {
    let mut schema = SchemaDefinition::new("complex_schema");

    // Add custom types
    let url_type = TypeDefinition {
        name: "URL".to_string(),
        description: Some("URL type".to_string()),
        base_type: Some("string".to_string()),
        pattern: Some(r"^https?://".to_string()),
        ..Default::default()
    };
    schema.types.insert("URL".to_string(), url_type);

    let positive_int = TypeDefinition {
        name: "PositiveInt".to_string(),
        description: Some("Positive integer".to_string()),
        base_type: Some("integer".to_string()),
        minimum_value: Some(1.0.into()),
        ..Default::default()
    };
    schema.types.insert("PositiveInt".to_string(), positive_int);

    // Add slots using custom types
    let mut website_slot = SlotDefinition::new("website");
    website_slot.range = Some("URL".to_string());
    schema.slots.insert("website".to_string(), website_slot);

    let mut count_slot = SlotDefinition::new("count");
    count_slot.range = Some("PositiveInt".to_string());
    schema.slots.insert("count".to_string(), count_slot);

    // Various built-in types
    let mut active_slot = SlotDefinition::new("is_active");
    active_slot.range = Some("boolean".to_string());
    schema.slots.insert("is_active".to_string(), active_slot);

    let mut price_slot = SlotDefinition::new("price");
    price_slot.range = Some("float".to_string());
    schema.slots.insert("price".to_string(), price_slot);

    let mut data_slot = SlotDefinition::new("metadata");
    data_slot.range = Some("string".to_string());
    data_slot.description = Some("JSON metadata".to_string());
    schema.slots.insert("metadata".to_string(), data_slot);

    // Add class using all types
    let mut product_class = ClassDefinition::new("Product");
    product_class.slots = vec![
        "website".to_string(),
        "count".to_string(),
        "is_active".to_string(),
        "price".to_string(),
        "metadata".to_string(),
    ];
    schema.classes.insert("Product".to_string(), product_class);

    // Generate protobuf
    let generator = ProtobufGenerator::new();
    let options = GeneratorOptions::default();
    let proto = generator.generate(&schema).expect("Test operation failed");

    // Check type mappings
    assert!(proto.contains("string website = 1")); // Custom types map to base type
    assert!(proto.contains("int64 count = 2"));
    assert!(proto.contains("bool is_active = 3"));
    assert!(proto.contains("double price = 4"));
    assert!(proto.contains("// JSON metadata"));
    assert!(proto.contains("string metadata = 5"));
}

#[tokio::test]
async fn test_case_handling() {
    let mut schema = SchemaDefinition::new("CaseTestSchema");

    // Test various naming styles
    let enum_def = EnumDefinition {
        name: "HTTPMethod".to_string(),
        permissible_values: vec![
            PermissibleValue::Simple("GET".to_string()),
            PermissibleValue::Simple("POST".to_string()),
            PermissibleValue::Simple("put".to_string()),
            PermissibleValue::Simple("delete".to_string()),
            PermissibleValue::Simple("patch-request".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("HTTPMethod".to_string(), enum_def);

    let mut class_def = ClassDefinition::new("APIRequest");
    class_def.slots = vec!["requestMethod".to_string(), "responseCode".to_string()];
    schema.classes.insert("APIRequest".to_string(), class_def);

    let mut method_slot = SlotDefinition::new("requestMethod");
    method_slot.range = Some("HTTPMethod".to_string());
    schema
        .slots
        .insert("requestMethod".to_string(), method_slot);

    let mut code_slot = SlotDefinition::new("responseCode");
    code_slot.range = Some("integer".to_string());
    schema.slots.insert("responseCode".to_string(), code_slot);

    // Generate protobuf
    let generator = ProtobufGenerator::new();
    let options = GeneratorOptions::default();
    let proto = generator.generate(&schema).expect("Test operation failed");

    // Check package name conversion
    assert!(proto.contains("package case_test_schema"));

    // Check enum name stays PascalCase
    assert!(proto.contains("enum HTTPMethod"));
    assert!(proto.contains("HTTP_METHOD_UNSPECIFIED = 0"));
    assert!(proto.contains("GET = 1"));
    assert!(proto.contains("POST = 2"));
    assert!(proto.contains("PUT = 3"));
    assert!(proto.contains("DELETE = 4"));
    assert!(proto.contains("PATCH_REQUEST = 5"));

    // Check message name stays PascalCase
    assert!(proto.contains("message APIRequest"));

    // Check field names become snake_case
    assert!(proto.contains("HTTPMethod request_method = 1"));
    assert!(proto.contains("int64 response_code = 2"));
}
