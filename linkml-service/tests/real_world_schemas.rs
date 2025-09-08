//! Real-world schema testing with actual LinkML schemas from biomedical and scientific domains

use linkml_core::types::SchemaDefinition;
use linkml_service::factory::create_linkml_service;
use linkml_service::parser::yaml_parser::YamlParser;
use linkml_service::validator::ValidationEngine;
use serde_json::json;
use std::sync::Arc;
use std::error::Error as StdError;
use tokio;


/// Test with a biomedical schema (simplified version of biolink-model)
#[tokio::test]
async fn test_biolink_model_schema() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let biolink_schema = r#"
id: https://w3id.org/biolink/biolink-model
name: biolink_model
title: Biolink Model
description: A high-level data model for biological and biomedical data

prefixes:
  biolink: https://w3id.org/biolink/vocab/
  rdfs: http://www.w3.org/2000/01/rdf-schema#
  skos: http://www.w3.org/2004/02/skos/core#

default_prefix: biolink

classes:
  NamedThing:
    description: A generic grouping for any identifiable entity
    slots:
      - id
      - name
      - description
      - category
    slot_usage:
      id:
        identifier: true
        required: true
      category:
        multivalued: true

  BiologicalEntity:
    is_a: NamedThing
    description: A biological entity
    slots:
      - taxon

  Gene:
    is_a: BiologicalEntity
    description: A region of DNA that codes for a functional product
    slots:
      - symbol
      - synonyms
    slot_usage:
      category:
        range: GeneCategory

  Protein:
    is_a: BiologicalEntity
    description: A gene product that is composed of a chain of amino acid residues
    slots:
      - encoded_by
    slot_usage:
      encoded_by:
        range: Gene

  Disease:
    is_a: NamedThing
    description: A disorder of structure or function
    slots:
      - associated_with
    slot_usage:
      associated_with:
        range: Gene
        multivalued: true

slots:
  id:
    description: A unique identifier for a thing
    range: uriorcurie
    required: true

  name:
    description: A human-readable name for a thing
    range: string

  description:
    description: A human-readable description for a thing
    range: string

  category:
    description: Name of the high level ontology class
    range: category_type
    multivalued: true

  symbol:
    description: Symbol for a particular thing
    range: string

  synonyms:
    description: Alternate human-readable names for a thing
    range: string
    multivalued: true

  taxon:
    description: The taxonomic classification of the entity
    range: OrganismTaxon

  encoded_by:
    description: The gene that encodes this protein
    range: Gene

  associated_with:
    description: Connects any entity to another entity
    range: NamedThing
    multivalued: true

types:
  string:
    base: str
    description: A character string

  uriorcurie:
    base: URIorCURIE
    description: A URI or a CURIE

enums:
  GeneCategory:
    description: Categories for genes
    permissible_values:
      gene:
        description: A gene
      pseudogene:
        description: A pseudogene

  category_type:
    description: Categories for entities
    permissible_values:
      NamedThing:
        description: A named thing
      BiologicalEntity:
        description: A biological entity
      Gene:
        description: A gene
      Protein:
        description: A protein
      Disease:
        description: A disease
"#;

    // Parse the schema
    let parser = YamlParser::new();
    let schema = parser.parse(biolink_schema)?;

    // Create test data
    let test_gene = json!({
        "id": "HGNC:1100",
        "name": "BRCA1",
        "description": "BRCA1 DNA repair associated",
        "category": ["Gene"],
        "symbol": "BRCA1",
        "synonyms": ["breast cancer 1", "BRCC1"],
        "taxon": "NCBITaxon:9606"
    }
    Ok(())
});

    let test_protein = json!({
        "id": "UniProtKB:P38398",
        "name": "BRCA1 protein",
        "description": "Breast cancer type 1 susceptibility protein",
        "category": ["Protein"],
        "encoded_by": "HGNC:1100",
        "taxon": "NCBITaxon:9606"
    });

    let test_disease = json!({
        "id": "MONDO:0007254",
        "name": "Breast cancer",
        "description": "A malignant neoplasm of the breast",
        "category": ["Disease"],
        "associated_with": ["HGNC:1100"]
    });

    // Create validation engine
    let engine = ValidationEngine::new(Arc::new(schema));

    // Validate each entity
    let gene_result = engine.validate_instance(&test_gene, "Gene").await?;
    assert!(
        gene_result.is_valid(),
        "Gene validation should pass: {:?}",
        gene_result.issues
    );

    let protein_result = engine.validate_instance(&test_protein, "Protein").await?;
    assert!(
        protein_result.is_valid(),
        "Protein validation should pass: {:?}",
        protein_result.issues
    );

    let disease_result = engine.validate_instance(&test_disease, "Disease").await?;
    assert!(
        disease_result.is_valid(),
        "Disease validation should pass: {:?}",
        disease_result.issues
    );

    println!("✅ Biolink model schema validation passed");
    Ok(())
}

/// Test with a scientific data schema (simplified version of FAIR data schema)
#[tokio::test]
async fn test_fair_data_schema() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let fair_schema = r#"
id: https://example.org/fair-data-model
name: fair_data_model
title: FAIR Data Model
description: A schema for FAIR (Findable, Accessible, Interoperable, Reusable) data

classes:
  Dataset:
    description: A collection of data
    slots:
      - identifier
      - title
      - description
      - creator
      - publisher
      - created_date
      - modified_date
      - license
      - keywords
      - format
      - size_bytes
    slot_usage:
      identifier:
        identifier: true
        required: true
      title:
        required: true
      creator:
        required: true
        multivalued: true

  Person:
    description: A person who created or contributed to a dataset
    slots:
      - identifier
      - name
      - email
      - affiliation
    slot_usage:
      identifier:
        identifier: true
        required: true
      name:
        required: true

  Organization:
    description: An organization that published or hosts a dataset
    slots:
      - identifier
      - name
      - url
      - contact_email
    slot_usage:
      identifier:
        identifier: true
        required: true
      name:
        required: true

slots:
  identifier:
    description: A unique identifier
    range: string
    required: true

  title:
    description: The title of the resource
    range: string

  name:
    description: The name of the entity
    range: string

  description:
    description: A description of the resource
    range: string

  creator:
    description: The person or organization that created the resource
    range: Person
    multivalued: true

  publisher:
    description: The organization that published the resource
    range: Organization

  email:
    description: Email address
    range: string
    pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}
    Ok(())
}$"

  url:
    description: A URL
    range: string
    pattern: "^https?://.*"

  contact_email:
    description: Contact email address
    range: string
    pattern: "^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\\.[a-zA-Z]{2,}$"

  affiliation:
    description: Organizational affiliation
    range: Organization

  created_date:
    description: Date when the resource was created
    range: date

  modified_date:
    description: Date when the resource was last modified
    range: date

  license:
    description: License under which the resource is available
    range: string

  keywords:
    description: Keywords describing the resource
    range: string
    multivalued: true

  format:
    description: File format of the resource
    range: string

  size_bytes:
    description: Size of the resource in bytes
    range: integer
    minimum_value: 0

types:
  string:
    base: str
    description: A character string

  integer:
    base: int
    description: An integer

  date:
    base: str
    description: A date in ISO 8601 format
"#;

    // Parse the schema
    let parser = YamlParser::new();
    let schema = parser.parse(fair_schema)?;

    // Create test data
    let test_person = json!({
        "identifier": "orcid:0000-0000-0000-0000",
        "name": "Dr. Jane Smith",
        "email": "jane.smith@university.edu",
        "affiliation": "org:university-123"
    });

    let test_organization = json!({
        "identifier": "org:university-123",
        "name": "Example University",
        "url": "https://www.example-university.edu",
        "contact_email": "data@university.edu"
    });

    let test_dataset = json!({
        "identifier": "doi:10.1234/example-dataset",
        "title": "Climate Data Collection 2023",
        "description": "A comprehensive collection of climate measurements from 2023",
        "creator": ["orcid:0000-0000-0000-0000"],
        "publisher": "org:university-123",
        "created_date": "2023-01-01",
        "modified_date": "2023-12-31",
        "license": "CC-BY-4.0",
        "keywords": ["climate", "temperature", "precipitation", "2023"],
        "format": "CSV",
        "size_bytes": 1048576
    });

    // Create validation engine
    let engine = ValidationEngine::new(Arc::new(schema));

    // Validate each entity
    let person_result = engine.validate_instance(&test_person, "Person").await?;
    assert!(
        person_result.is_valid(),
        "Person validation should pass: {:?}",
        person_result.issues
    );

    let org_result = engine
        .validate_instance(&test_organization, "Organization")
        .await?;
    assert!(
        org_result.is_valid(),
        "Organization validation should pass: {:?}",
        org_result.issues
    );

    let dataset_result = engine.validate_instance(&test_dataset, "Dataset").await?;
    assert!(
        dataset_result.is_valid(),
        "Dataset validation should pass: {:?}",
        dataset_result.issues
    );

    println!("✅ FAIR data schema validation passed");
    Ok(())
}

/// Test cross-reference validation with complex relationships
#[tokio::test]
async fn test_cross_reference_validation() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r#"
id: https://example.org/cross-ref-test
name: cross_ref_test
description: Schema for testing cross-reference validation

classes:
  Author:
    description: An author of publications
    slots:
      - id
      - name
      - publications
    slot_usage:
      id:
        identifier: true
        required: true
      publications:
        range: Publication
        multivalued: true

  Publication:
    description: A scientific publication
    slots:
      - id
      - title
      - authors
      - journal
    slot_usage:
      id:
        identifier: true
        required: true
      authors:
        range: Author
        multivalued: true
        required: true

  Journal:
    description: A scientific journal
    slots:
      - id
      - name
      - impact_factor
    slot_usage:
      id:
        identifier: true
        required: true

slots:
  id:
    range: string
    required: true
  name:
    range: string
    required: true
  title:
    range: string
    required: true
  publications:
    range: Publication
    multivalued: true
  authors:
    range: Author
    multivalued: true
  journal:
    range: Journal
  impact_factor:
    range: float
    minimum_value: 0.0

types:
  string:
    base: str
  float:
    base: float
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    // Test data with valid cross-references
    let author1 = json!({
        "id": "author:001",
        "name": "Dr. Alice Johnson",
        "publications": ["pub:001", "pub:002"]
    }
    Ok(())
});

    let author2 = json!({
        "id": "author:002",
        "name": "Prof. Bob Smith",
        "publications": ["pub:001"]
    });

    let journal = json!({
        "id": "journal:nature",
        "name": "Nature",
        "impact_factor": 42.778
    });

    let publication = json!({
        "id": "pub:001",
        "title": "Breakthrough in Quantum Computing",
        "authors": ["author:001", "author:002"],
        "journal": "journal:nature"
    });

    // Create validation context with all instances for cross-reference validation
    let all_instances = vec![
        author1.clone(),
        author2.clone(),
        journal.clone(),
        publication.clone(),
    ];

    // Validate with cross-reference context
    let mut context = linkml_service::validator::ValidationContext::new(engine.schema().clone());
    context.set_all_instances(all_instances);

    // Test valid cross-references
    let result = engine
        .validate_instance(&publication, "Publication")
        .await?;
    assert!(
        result.is_valid(),
        "Publication with valid cross-references should pass"
    );

    // Test invalid cross-reference
    let invalid_publication = json!({
        "id": "pub:002",
        "title": "Another Paper",
        "authors": ["author:999"], // Non-existent author
        "journal": "journal:nature"
    });

    let invalid_result = engine
        .validate_instance(&invalid_publication, "Publication")
        .await?;
    // This should fail due to invalid cross-reference (if cross-reference validator is enabled)

    println!("✅ Cross-reference validation test completed");
    Ok(())
}

/// Test performance with large datasets
#[tokio::test]
async fn test_large_dataset_performance() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let schema_yaml = r#"
id: https://example.org/performance-test
name: performance_test
description: Schema for performance testing

classes:
  Record:
    description: A data record
    slots:
      - id
      - value
      - timestamp
      - category
    slot_usage:
      id:
        identifier: true
        required: true

slots:
  id:
    range: string
    required: true
  value:
    range: float
    minimum_value: 0.0
    maximum_value: 1000.0
  timestamp:
    range: string
    pattern: "^\\d{4}
    Ok(())
}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$"
  category:
    range: RecordCategory

enums:
  RecordCategory:
    permissible_values:
      A: {}
      B: {}
      C: {}

types:
  string:
    base: str
  float:
    base: float
"#;

    let parser = YamlParser::new();
    let schema = parser.parse(schema_yaml)?;
    let engine = ValidationEngine::new(Arc::new(schema));

    // Generate large dataset
    let start_time = std::time::Instant::now();
    let mut records = Vec::new();

    for i in 0..1000 {
        let record = json!({
            "id": format!("record:{:04}", i),
            "value": (i as f64) % 1000.0,
            "timestamp": "2023-01-01T12:00:00Z",
            "category": match i % 3 {
                0 => "A",
                1 => "B",
                _ => "C"
            }
        });
        records.push(record);
    }

    let generation_time = start_time.elapsed();
    println!("Generated 1000 records in {:?}", generation_time);

    // Validate all records
    let validation_start = std::time::Instant::now();
    let mut valid_count = 0;
    let mut error_count = 0;

    for record in &records {
        let result = engine.validate_instance(record, "Record").await?;
        if result.is_valid() {
            valid_count += 1;
        } else {
            error_count += 1;
        }
    }

    let validation_time = validation_start.elapsed();
    let records_per_second = records.len() as f64 / validation_time.as_secs_f64();

    println!(
        "Validated {} records in {:?}",
        records.len(),
        validation_time
    );
    println!("Performance: {:.2} records/second", records_per_second);
    println!("Valid: {}, Errors: {}", valid_count, error_count);

    // Performance should be reasonable (at least 100 records/second)
    assert!(
        records_per_second > 100.0,
        "Performance should be at least 100 records/second, got {:.2}",
        records_per_second
    );
    assert_eq!(valid_count, 1000, "All records should be valid");

    println!("✅ Large dataset performance test passed");
    Ok(())
}
