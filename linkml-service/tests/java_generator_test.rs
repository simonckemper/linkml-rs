//! Tests for Java code generation

use linkml_core::types::{
    SchemaDefinition, ClassDefinition, SlotDefinition, EnumDefinition, 
    PermissibleValue, TypeDefinition,
};
use linkml_service::generator::{JavaGenerator, Generator, GeneratorOptions};

#[tokio::test]
async fn test_basic_java_generation() {
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
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    assert_eq!(results.len(), 1);
    let java = &results[0];
    
    // Check filename
    assert_eq!(java.filename, "Person.java");
    
    // Check content
    assert!(java.content.contains("package com.example.person_schema"));
    assert!(java.content.contains("import java.util.*;"));
    assert!(java.content.contains("import javax.validation.constraints.*;"));
    assert!(java.content.contains("public class Person {"));
    
    // Check fields
    assert!(java.content.contains("@NotNull"));
    assert!(java.content.contains("private String name;"));
    assert!(java.content.contains("@Min(0)"));
    assert!(java.content.contains("@Max(150)"));
    assert!(java.content.contains("private Long age;"));
    assert!(java.content.contains("@Pattern(regexp ="));
    assert!(java.content.contains("private String email;"));
    
    // Check getters and setters
    assert!(java.content.contains("public String getName()"));
    assert!(java.content.contains("public void setName(String name)"));
    assert!(java.content.contains("public Long getAge()"));
    assert!(java.content.contains("public void setAge(Long age)"));
}

#[tokio::test]
async fn test_enum_generation() {
    let mut schema = SchemaDefinition::new("status_schema");
    
    // Add enum
    let status_enum = EnumDefinition {
        name: "OrderStatus".to_string(),
        description: Some("Status of an order".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("pending".to_string()),
            PermissibleValue::Simple("processing".to_string()),
            PermissibleValue::Complex {
                text: "shipped".to_string(),
                description: Some("Order has been shipped".to_string()),
                meaning: None,
            },
            PermissibleValue::Simple("delivered".to_string()),
            PermissibleValue::Simple("cancelled".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("OrderStatus".to_string(), status_enum);
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    assert_eq!(results.len(), 1);
    let java = &results[0];
    
    assert_eq!(java.filename, "OrderStatus.java");
    assert!(java.content.contains("public enum OrderStatus {"));
    assert!(java.content.contains("PENDING,"));
    assert!(java.content.contains("PROCESSING,"));
    assert!(java.content.contains("/**\n     * Order has been shipped\n     */"));
    assert!(java.content.contains("SHIPPED,"));
    assert!(java.content.contains("DELIVERED,"));
    assert!(java.content.contains("CANCELLED;"));
}

#[tokio::test]
async fn test_inheritance() {
    let mut schema = SchemaDefinition::new("entity_schema");
    
    // Base entity slots
    let mut id_slot = SlotDefinition::new("id");
    id_slot.range = Some("string".to_string());
    id_slot.identifier = Some(true);
    schema.slots.insert("id".to_string(), id_slot);
    
    let mut created_slot = SlotDefinition::new("createdAt");
    created_slot.range = Some("datetime".to_string());
    schema.slots.insert("createdAt".to_string(), created_slot);
    
    // Person-specific slots
    let mut name_slot = SlotDefinition::new("name");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("name".to_string(), name_slot);
    
    // Base entity class
    let mut entity_class = ClassDefinition::new("Entity");
    entity_class.abstract_ = Some(true);
    entity_class.slots = vec!["id".to_string(), "createdAt".to_string()];
    schema.classes.insert("Entity".to_string(), entity_class);
    
    // Person extends Entity
    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("Entity".to_string());
    person_class.slots = vec!["name".to_string()];
    schema.classes.insert("Person".to_string(), person_class);
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    assert_eq!(results.len(), 2);
    
    // Check Entity class
    let entity_java = results.iter().find(|r| r.filename == "Entity.java").unwrap();
    assert!(entity_java.content.contains("public class Entity {"));
    assert!(entity_java.content.contains("private String id;"));
    assert!(entity_java.content.contains("private LocalDateTime createdAt;"));
    
    // Check Person class
    let person_java = results.iter().find(|r| r.filename == "Person.java").unwrap();
    assert!(person_java.content.contains("public class Person extends Entity {"));
    assert!(person_java.content.contains("private String name;"));
    // Should not redeclare inherited fields
    assert!(!person_java.content.contains("private String id;"));
}

#[tokio::test]
async fn test_collections() {
    let mut schema = SchemaDefinition::new("team_schema");
    
    // Add slots
    let mut name_slot = SlotDefinition::new("teamName");
    name_slot.range = Some("string".to_string());
    schema.slots.insert("teamName".to_string(), name_slot);
    
    let mut members_slot = SlotDefinition::new("members");
    members_slot.range = Some("string".to_string());
    members_slot.multivalued = Some(true);
    schema.slots.insert("members".to_string(), members_slot);
    
    let mut scores_slot = SlotDefinition::new("scores");
    scores_slot.range = Some("integer".to_string());
    scores_slot.multivalued = Some(true);
    schema.slots.insert("scores".to_string(), scores_slot);
    
    // Add class
    let mut team_class = ClassDefinition::new("Team");
    team_class.slots = vec!["teamName".to_string(), "members".to_string(), "scores".to_string()];
    schema.classes.insert("Team".to_string(), team_class);
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    let java = &results[0];
    
    // Check list fields
    assert!(java.content.contains("private String teamName;"));
    assert!(java.content.contains("private List<String> members;"));
    assert!(java.content.contains("private List<Long> scores;"));
    
    // Check constructor initializes lists
    assert!(java.content.contains("this.members = new ArrayList<>();"));
    assert!(java.content.contains("this.scores = new ArrayList<>();"));
    
    // Check getter returns List
    assert!(java.content.contains("public List<String> getMembers()"));
    assert!(java.content.contains("public List<Long> getScores()"));
}

#[tokio::test]
async fn test_custom_types() {
    let mut schema = SchemaDefinition::new("product_schema");
    
    // Add custom types
    let url_type = TypeDefinition {
        name: "URL".to_string(),
        description: Some("URL type".to_string()),
        base_type: Some("uri".to_string()),
        pattern: Some(r"^https?://".to_string()),
        ..Default::default()
    };
    schema.types.insert("URL".to_string(), url_type);
    
    let price_type = TypeDefinition {
        name: "Price".to_string(),
        description: Some("Positive decimal price".to_string()),
        base_type: Some("decimal".to_string()),
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
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    let java = &results[0];
    
    // Check type mappings
    assert!(java.content.contains("private URI website;")); // URL -> uri -> URI
    assert!(java.content.contains("private BigDecimal price;")); // Price -> decimal -> BigDecimal
}

#[tokio::test]
async fn test_builder_pattern() {
    let mut schema = SchemaDefinition::new("user_schema");
    
    // Add slots
    let mut username_slot = SlotDefinition::new("username");
    username_slot.range = Some("string".to_string());
    username_slot.required = Some(true);
    schema.slots.insert("username".to_string(), username_slot);
    
    let mut email_slot = SlotDefinition::new("email");
    email_slot.range = Some("string".to_string());
    schema.slots.insert("email".to_string(), email_slot);
    
    // Add class
    let mut user_class = ClassDefinition::new("User");
    user_class.slots = vec!["username".to_string(), "email".to_string()];
    schema.classes.insert("User".to_string(), user_class);
    
    // Generate Java with builder
    let generator = JavaGenerator::new();
    let mut options = GeneratorOptions::default();
    options = options.set_custom("generate_builder", "true");
    let results = generator.generate(&schema, &options).await.unwrap();
    
    let java = &results[0];
    
    // Check builder class
    assert!(java.content.contains("public static class Builder {"));
    assert!(java.content.contains("private final User instance = new User();"));
    assert!(java.content.contains("public Builder withUsername(String username)"));
    assert!(java.content.contains("public Builder withEmail(String email)"));
    assert!(java.content.contains("public User build()"));
    assert!(java.content.contains("public static Builder builder()"));
}

#[tokio::test]
async fn test_case_handling() {
    let mut schema = SchemaDefinition::new("CaseTestSchema");
    
    // Test various naming styles
    let mut class_def = ClassDefinition::new("APIResponse");
    class_def.slots = vec!["responseCode".to_string(), "isActive".to_string()];
    schema.classes.insert("APIResponse".to_string(), class_def);
    
    let mut code_slot = SlotDefinition::new("responseCode");
    code_slot.range = Some("integer".to_string());
    schema.slots.insert("responseCode".to_string(), code_slot);
    
    let mut active_slot = SlotDefinition::new("isActive");
    active_slot.range = Some("boolean".to_string());
    schema.slots.insert("isActive".to_string(), active_slot);
    
    // Generate Java
    let generator = JavaGenerator::new();
    let options = GeneratorOptions::default();
    let results = generator.generate(&schema, &options).await.unwrap();
    
    let java = &results[0];
    
    // Check class name stays PascalCase
    assert!(java.content.contains("public class APIResponse"));
    
    // Check field names are camelCase
    assert!(java.content.contains("private Long responseCode;"));
    assert!(java.content.contains("private Boolean isActive;"));
    
    // Check getter/setter names
    assert!(java.content.contains("public Long getResponseCode()"));
    assert!(java.content.contains("public void setResponseCode(Long responseCode)"));
    assert!(java.content.contains("public Boolean getIsActive()"));
    assert!(java.content.contains("public void setIsActive(Boolean isActive)"));
}