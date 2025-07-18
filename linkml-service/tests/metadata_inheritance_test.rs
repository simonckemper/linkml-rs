//! Comprehensive tests for metadata inheritance and merging

use linkml_core::{
    metadata::{Contributor, ElementMetadata, Example, SchemaMetadata},
    types::{ClassDefinition, SchemaDefinition, SlotDefinition},
};
use linkml_service::transform::InheritanceResolver;
use linkml_service::parser::{SchemaParser, YamlParser};

#[test]
fn test_slot_metadata_inheritance() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    // Base slot with metadata
    let mut base_slot = SlotDefinition::new("base_slot");
    base_slot.metadata = ElementMetadata {
        aliases: vec!["base_alias1".to_string(), "base_alias2".to_string()],
        description: Some("Base slot description".to_string()),
        see_also: vec!["https://example.org/base".to_string()],
        examples: vec![
            Example {
                value: "base_example".to_string(),
                description: Some("Base example".to_string()),
            }
        ],
        deprecated: Some("Use new_slot instead".to_string()),
        todos: vec!["Base TODO".to_string()],
        notes: vec!["Base note".to_string()],
        comments: vec!["Base comment".to_string()],
        rank: Some(10),
    };
    schema.slots.insert("base_slot".to_string(), base_slot);
    
    // Derived slot that overrides some metadata
    let mut derived_slot = SlotDefinition::new("derived_slot");
    derived_slot.is_a = Some("base_slot".to_string());
    derived_slot.metadata = ElementMetadata {
        aliases: vec!["derived_alias".to_string()], // Override
        description: Some("Derived slot description".to_string()), // Override
        see_also: vec!["https://example.org/derived".to_string()], // Additional
        examples: vec![
            Example {
                value: "derived_example".to_string(),
                description: Some("Derived example".to_string()),
            }
        ],
        deprecated: None, // Clear deprecation
        todos: vec!["Derived TODO".to_string()], // Additional
        notes: vec![], // Clear notes
        comments: vec!["Derived comment".to_string()], // Override
        rank: Some(5), // Override
    };
    schema.slots.insert("derived_slot".to_string(), derived_slot);
    
    // Resolve inheritance
    let mut resolver = InheritanceResolver::new(&schema);
    let resolved = resolver.resolve().unwrap();
    
    let resolved_slot = resolved.slots.get("derived_slot").unwrap();
    let metadata = &resolved_slot.metadata;
    
    // Check overrides
    assert_eq!(metadata.description.as_deref(), Some("Derived slot description"));
    assert_eq!(metadata.aliases, vec!["derived_alias".to_string()]);
    assert_eq!(metadata.rank, Some(5));
    
    // Check additions (should merge)
    assert_eq!(metadata.see_also.len(), 2);
    assert!(metadata.see_also.contains(&"https://example.org/base".to_string()));
    assert!(metadata.see_also.contains(&"https://example.org/derived".to_string()));
    
    assert_eq!(metadata.todos.len(), 2);
    assert!(metadata.todos.contains(&"Base TODO".to_string()));
    assert!(metadata.todos.contains(&"Derived TODO".to_string()));
    
    // Check cleared fields
    assert_eq!(metadata.deprecated, None);
    assert_eq!(metadata.notes.len(), 0);
}

#[test]
fn test_schema_metadata_features() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    // Add comprehensive schema metadata
    schema.schema_metadata = SchemaMetadata {
        contributors: vec![
            Contributor {
                name: "John Doe".to_string(),
                email: Some("john@example.org".to_string()),
                github: Some("johndoe".to_string()),
                orcid: Some("0000-0001-2345-6789".to_string()),
                role: Some("lead".to_string()),
            },
            Contributor {
                name: "Jane Smith".to_string(),
                email: Some("jane@example.org".to_string()),
                github: None,
                orcid: None,
                role: Some("reviewer".to_string()),
            },
        ],
        status: Some("draft".to_string()),
        license: Some("CC-BY-4.0".to_string()),
        version: Some("1.0.0".to_string()),
        categories: vec!["biomedical".to_string(), "genomics".to_string()],
        keywords: vec!["gene".to_string(), "protein".to_string(), "expression".to_string()],
        see_also: vec!["https://example.org/related".to_string()],
        created_on: Some("2024-01-01".to_string()),
        modified_on: Some("2024-01-15".to_string()),
        source: Some("https://github.com/example/schema".to_string()),
    };
    
    // Verify all fields
    assert_eq!(schema.schema_metadata.contributors.len(), 2);
    assert_eq!(schema.schema_metadata.contributors[0].name, "John Doe");
    assert_eq!(schema.schema_metadata.contributors[0].role.as_deref(), Some("lead"));
    assert_eq!(schema.schema_metadata.status.as_deref(), Some("draft"));
    assert_eq!(schema.schema_metadata.categories.len(), 2);
    assert!(schema.schema_metadata.keywords.contains(&"gene".to_string()));
}

#[tokio::test]
async fn test_metadata_yaml_parsing() {
    let yaml = r#"
id: https://example.org/test
name: test_schema

# Schema-level metadata
contributors:
  - name: Alice Johnson
    email: alice@example.org
    github: alicej
    orcid: 0000-0002-1234-5678
    role: author
  - name: Bob Williams
    email: bob@example.org
    role: reviewer

status: released
license: MIT
version: 2.0.0
categories:
  - clinical
  - research
keywords:
  - patient
  - medical
  - records

classes:
  Patient:
    description: A patient in a medical system
    aliases:
      - Subject
      - MedicalPatient
    see_also:
      - https://hl7.org/fhir/patient.html
    examples:
      - value: '{"id": "P123", "name": "John Doe"}'
        description: Example patient record
    deprecated: Use Person with patient_role instead
    todos:
      - Add insurance information
      - Link to provider records
    notes:
      - This class is being phased out
    comments:
      - Consider privacy implications
    rank: 1

slots:
  patient_id:
    description: Unique identifier for a patient
    aliases:
      - medical_record_number
      - mrn
    examples:
      - value: P123456
        description: Standard patient ID format
      - value: MRN-2024-0001
        description: Alternative MRN format
    see_also:
      - https://www.hl7.org/fhir/datatypes.html#Identifier
    rank: 1
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(yaml).unwrap();
    
    // Check schema metadata
    assert_eq!(schema.schema_metadata.contributors.len(), 2);
    assert_eq!(schema.schema_metadata.contributors[0].name, "Alice Johnson");
    assert_eq!(schema.schema_metadata.contributors[0].orcid.as_deref(), Some("0000-0002-1234-5678"));
    assert_eq!(schema.schema_metadata.status.as_deref(), Some("released"));
    assert_eq!(schema.schema_metadata.version.as_deref(), Some("2.0.0"));
    assert!(schema.schema_metadata.categories.contains(&"clinical".to_string()));
    
    // Check class metadata
    let patient_class = schema.classes.get("Patient").unwrap();
    assert_eq!(patient_class.metadata.aliases.len(), 2);
    assert!(patient_class.metadata.aliases.contains(&"Subject".to_string()));
    assert_eq!(patient_class.metadata.deprecated.as_deref(), Some("Use Person with patient_role instead"));
    assert_eq!(patient_class.metadata.todos.len(), 2);
    assert_eq!(patient_class.metadata.rank, Some(1));
    
    // Check slot metadata
    let patient_id_slot = schema.slots.get("patient_id").unwrap();
    assert_eq!(patient_id_slot.metadata.aliases.len(), 2);
    assert_eq!(patient_id_slot.metadata.examples.len(), 2);
    assert_eq!(patient_id_slot.metadata.examples[0].value, "P123456");
    assert_eq!(patient_id_slot.metadata.see_also.len(), 1);
}

#[test]
fn test_metadata_class_inheritance() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    // Base class with metadata
    let mut base_class = ClassDefinition::new("BaseEntity");
    base_class.metadata = ElementMetadata {
        aliases: vec!["Entity".to_string()],
        description: Some("Base entity class".to_string()),
        see_also: vec!["https://schema.org/Thing".to_string()],
        examples: vec![
            Example {
                value: "{}".to_string(),
                description: Some("Empty entity".to_string()),
            }
        ],
        deprecated: None,
        todos: vec!["Add validation".to_string()],
        notes: vec!["Foundation class".to_string()],
        comments: vec![],
        rank: Some(100),
    };
    schema.classes.insert("BaseEntity".to_string(), base_class);
    
    // Derived class
    let mut person_class = ClassDefinition::new("Person");
    person_class.is_a = Some("BaseEntity".to_string());
    person_class.metadata = ElementMetadata {
        aliases: vec!["Individual".to_string()],
        description: Some("A person entity".to_string()), // Override
        see_also: vec!["https://schema.org/Person".to_string()], // Additional
        examples: vec![
            Example {
                value: r#"{"name": "John Doe"}"#.to_string(),
                description: Some("Person example".to_string()),
            }
        ],
        deprecated: None,
        todos: vec!["Add address".to_string()], // Additional
        notes: vec![], // Clear
        comments: vec!["GDPR compliant".to_string()],
        rank: Some(50), // Override
    };
    schema.classes.insert("Person".to_string(), person_class);
    
    // Resolve inheritance
    let mut resolver = InheritanceResolver::new(&schema);
    let resolved = resolver.resolve().unwrap();
    
    let resolved_person = resolved.classes.get("Person").unwrap();
    let metadata = &resolved_person.metadata;
    
    // Verify inheritance behavior
    assert_eq!(metadata.description.as_deref(), Some("A person entity"));
    assert_eq!(metadata.aliases, vec!["Individual".to_string()]);
    assert_eq!(metadata.rank, Some(50));
    
    // Merged fields
    assert_eq!(metadata.see_also.len(), 2);
    assert_eq!(metadata.todos.len(), 2);
    assert!(metadata.todos.contains(&"Add validation".to_string()));
    assert!(metadata.todos.contains(&"Add address".to_string()));
}

#[test]
fn test_metadata_serialization_roundtrip() {
    use serde_yaml;
    
    let metadata = ElementMetadata {
        aliases: vec!["alias1".to_string(), "alias2".to_string()],
        description: Some("Test description".to_string()),
        see_also: vec!["https://example.org".to_string()],
        examples: vec![
            Example {
                value: "test".to_string(),
                description: Some("Test example".to_string()),
            }
        ],
        deprecated: Some("Use new_element".to_string()),
        todos: vec!["TODO 1".to_string(), "TODO 2".to_string()],
        notes: vec!["Note 1".to_string()],
        comments: vec!["Comment 1".to_string()],
        rank: Some(42),
    };
    
    // Serialize to YAML
    let yaml = serde_yaml::to_string(&metadata).unwrap();
    
    // Deserialize back
    let deserialized: ElementMetadata = serde_yaml::from_str(&yaml).unwrap();
    
    // Verify all fields
    assert_eq!(metadata.aliases, deserialized.aliases);
    assert_eq!(metadata.description, deserialized.description);
    assert_eq!(metadata.see_also, deserialized.see_also);
    assert_eq!(metadata.examples.len(), deserialized.examples.len());
    assert_eq!(metadata.deprecated, deserialized.deprecated);
    assert_eq!(metadata.todos, deserialized.todos);
    assert_eq!(metadata.notes, deserialized.notes);
    assert_eq!(metadata.comments, deserialized.comments);
    assert_eq!(metadata.rank, deserialized.rank);
}

#[test]
fn test_contributor_validation() {
    let contributor = Contributor {
        name: "Test User".to_string(),
        email: Some("test@example.org".to_string()),
        github: Some("testuser".to_string()),
        orcid: Some("0000-0001-2345-6789".to_string()),
        role: Some("developer".to_string()),
    };
    
    // Test ORCID format (basic check)
    assert!(contributor.orcid.as_ref().unwrap().len() == 19);
    assert!(contributor.orcid.as_ref().unwrap().chars().filter(|c| *c == '-').count() == 3);
    
    // Test email contains @
    assert!(contributor.email.as_ref().unwrap().contains('@'));
}

#[test]
fn test_example_with_multiline_values() {
    let example = Example {
        value: r#"{
  "id": "123",
  "name": "Test",
  "data": {
    "nested": true
  }
}"#.to_string(),
        description: Some("Multi-line JSON example".to_string()),
    };
    
    // Verify multiline value is preserved
    assert!(example.value.contains('\n'));
    assert!(example.value.contains("nested"));
}

#[test]
fn test_metadata_empty_collections() {
    let metadata = ElementMetadata {
        aliases: vec![],
        description: None,
        see_also: vec![],
        examples: vec![],
        deprecated: None,
        todos: vec![],
        notes: vec![],
        comments: vec![],
        rank: None,
    };
    
    // All collections should be empty but not cause issues
    assert_eq!(metadata.aliases.len(), 0);
    assert_eq!(metadata.see_also.len(), 0);
    assert_eq!(metadata.examples.len(), 0);
    assert_eq!(metadata.todos.len(), 0);
    assert_eq!(metadata.notes.len(), 0);
    assert_eq!(metadata.comments.len(), 0);
}

#[test]
fn test_metadata_with_special_characters() {
    let mut schema = SchemaDefinition::new("test_schema");
    
    schema.schema_metadata.contributors.push(Contributor {
        name: "José García-López".to_string(),
        email: Some("josé@español.es".to_string()),
        github: Some("josé-garcía".to_string()),
        orcid: None,
        role: Some("científico".to_string()),
    });
    
    let mut slot = SlotDefinition::new("测试槽");
    slot.metadata.description = Some("描述 with émojis 🚀".to_string());
    slot.metadata.aliases = vec!["別名".to_string(), "псевдоним".to_string()];
    schema.slots.insert("test_slot".to_string(), slot);
    
    // Verify special characters are preserved
    assert_eq!(schema.schema_metadata.contributors[0].name, "José García-López");
    let slot = schema.slots.get("test_slot").unwrap();
    assert!(slot.metadata.description.as_ref().unwrap().contains("🚀"));
    assert!(slot.metadata.aliases.contains(&"別名".to_string()));
}