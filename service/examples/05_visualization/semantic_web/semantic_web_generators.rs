//! Example demonstrating semantic web generators: SPARQL, ShEx, and PlantUML
//!
//! This example shows how to generate various semantic web and diagram formats
//! from LinkML schemas.

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, , PlantUmlGenerator, ShExGenerator, SparqlGenerator,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Create a sample schema for a research data model
    let mut schema = SchemaDefinition::default();
    schema.name = Some("ResearchDataSchema".to_string());
    schema.id = Some("http://example.org/research".to_string());
    schema.description = Some("A schema for research data management".to_string());

    // Base class for all entities
    let mut entity = ClassDefinition::default();
    entity.abstract_ = Some(true);
    entity.description = Some("Base class for all research entities".to_string());
    entity.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "created_date".to_string(),
    ];
    schema.classes.insert("Entity".to_string(), entity);

    // Researcher class
    let mut researcher = ClassDefinition::default();
    researcher.description = Some("A researcher or scientist".to_string());
    researcher.is_a = Some("Entity".to_string());
    researcher.slots = vec![
        "orcid".to_string(),
        "affiliation".to_string(),
        "publications".to_string(),
    ];
    schema.classes.insert("Researcher".to_string(), researcher);

    // Publication class
    let mut publication = ClassDefinition::default();
    publication.description = Some("A research publication".to_string());
    publication.is_a = Some("Entity".to_string());
    publication.slots = vec![
        "doi".to_string(),
        "title".to_string(),
        "authors".to_string(),
        "publication_date".to_string(),
        "journal".to_string(),
        "status".to_string(),
    ];
    schema
        .classes
        .insert("Publication".to_string(), publication);

    // Dataset class
    let mut dataset = ClassDefinition::default();
    dataset.description = Some("A research dataset".to_string());
    dataset.is_a = Some("Entity".to_string());
    dataset.slots = vec![
        "description".to_string(),
        "size_mb".to_string(),
        "format".to_string(),
        "license".to_string(),
        "publications".to_string(),
    ];
    schema.classes.insert("Dataset".to_string(), dataset);

    // Organization class
    let mut organization = ClassDefinition::default();
    organization.description = Some("A research organization or institution".to_string());
    organization.slots = vec![
        "name".to_string(),
        "country".to_string(),
        "type".to_string(),
    ];
    schema
        .classes
        .insert("Organization".to_string(), organization);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    id_slot.required = Some(true);
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut created_date_slot = SlotDefinition::default();
    created_date_slot.range = Some("date".to_string());
    schema
        .slots
        .insert("created_date".to_string(), created_date_slot);

    let mut orcid_slot = SlotDefinition::default();
    orcid_slot.range = Some("string".to_string());
    orcid_slot.pattern = Some(r"^\d{4}-\d{4}-\d{4}-\d{4}$".to_string());
    orcid_slot.description = Some("ORCID identifier".to_string());
    schema.slots.insert("orcid".to_string(), orcid_slot);

    let mut doi_slot = SlotDefinition::default();
    doi_slot.range = Some("string".to_string());
    doi_slot.pattern = Some(r"^10\.\d{4,}/[-._;()/:a-zA-Z0-9]+$".to_string());
    doi_slot.description = Some("Digital Object Identifier".to_string());
    schema.slots.insert("doi".to_string(), doi_slot);

    let mut title_slot = SlotDefinition::default();
    title_slot.range = Some("string".to_string());
    title_slot.required = Some(true);
    schema.slots.insert("title".to_string(), title_slot);

    let mut authors_slot = SlotDefinition::default();
    authors_slot.range = Some("Researcher".to_string());
    authors_slot.multivalued = Some(true);
    authors_slot.required = Some(true);
    schema.slots.insert("authors".to_string(), authors_slot);

    let mut publications_slot = SlotDefinition::default();
    publications_slot.range = Some("Publication".to_string());
    publications_slot.multivalued = Some(true);
    schema
        .slots
        .insert("publications".to_string(), publications_slot);

    let mut affiliation_slot = SlotDefinition::default();
    affiliation_slot.range = Some("Organization".to_string());
    schema
        .slots
        .insert("affiliation".to_string(), affiliation_slot);

    let mut size_mb_slot = SlotDefinition::default();
    size_mb_slot.range = Some("float".to_string());
    size_mb_slot.minimum_value = Some(serde_json::json!(0));
    size_mb_slot.unit = Some("megabytes".to_string());
    schema.slots.insert("size_mb".to_string(), size_mb_slot);

    let mut status_slot = SlotDefinition::default();
    status_slot.range = Some("PublicationStatus".to_string());
    schema.slots.insert("status".to_string(), status_slot);

    let mut license_slot = SlotDefinition::default();
    license_slot.range = Some("LicenseType".to_string());
    schema.slots.insert("license".to_string(), license_slot);

    // Add remaining string slots
    for slot_name in &[
        "description",
        "publication_date",
        "journal",
        "format",
        "country",
        "type",
    ] {
        let mut slot = SlotDefinition::default();
        slot.range = Some("string".to_string());
        schema.slots.insert(slot_name.to_string(), slot);
    }

    // Define enumerations
    let mut pub_status = EnumDefinition::default();
    pub_status.description = Some("Publication status".to_string());
    pub_status.permissible_values = vec![
        PermissibleValue::Simple("DRAFT".to_string()),
        PermissibleValue::Simple("SUBMITTED".to_string()),
        PermissibleValue::Simple("IN_REVIEW".to_string()),
        PermissibleValue::Simple("ACCEPTED".to_string()),
        PermissibleValue::Simple("PUBLISHED".to_string()),
        PermissibleValue::Simple("RETRACTED".to_string()),
    ];
    schema
        .enums
        .insert("PublicationStatus".to_string(), pub_status);

    let mut license_type = EnumDefinition::default();
    license_type.description = Some("Data license types".to_string());
    license_type.permissible_values = vec![
        PermissibleValue::Simple("CC0".to_string()),
        PermissibleValue::Simple("CC_BY".to_string()),
        PermissibleValue::Simple("CC_BY_SA".to_string()),
        PermissibleValue::Simple("CC_BY_NC".to_string()),
        PermissibleValue::Simple("MIT".to_string()),
        PermissibleValue::Simple("GPL".to_string()),
        PermissibleValue::Simple("PROPRIETARY".to_string()),
    ];
    schema.enums.insert("LicenseType".to_string(), license_type);

    println!(
        "=== Semantic Web Generators Example ===
"
    );

    // SPARQL Generation Examples
    println!("1. SPARQL Query Generation");
    println!(
        "==========================
"
    );

    // SELECT queries
    println!("1.1 SELECT Queries:");
    let select_generator = SparqlGenerator::new();
    let result = select_generator.generate(&schema)?;
    std::fs::write("research_select.sparql", &result)?;
    println!("Generated: research_select.sparql");
    println!(
        "Sample query:
{}
...
",
        result.lines().take(15).collect::<Vec<_>>().join(
            "
"
        )
    );

    // CONSTRUCT queries
    println!("1.2 CONSTRUCT Queries:");
    let construct_generator = SparqlGenerator::new().with_query_type(SparqlQueryType::Construct);
    let result = construct_generator.generate(&schema)?;
    std::fs::write("research_construct.sparql", &result)?;
    println!(
        "Generated: research_construct.sparql
"
    );

    // ASK queries
    println!("1.3 ASK Queries (Validation):");
    let ask_generator = SparqlGenerator::new().with_query_type(SparqlQueryType::Ask);
    let result = ask_generator.generate(&schema)?;
    std::fs::write("research_ask.sparql", &result)?;
    println!(
        "Generated: research_ask.sparql
"
    );

    // ShEx Generation Examples
    println!("2. ShEx (Shape Expressions) Generation");
    println!(
        "======================================
"
    );

    // ShEx Compact syntax
    println!("2.1 ShEx Compact Syntax:");
    let shexc_generator = ShExGenerator::new();
    let result = shexc_generator.generate(&schema)?;
    std::fs::write("research_shapes.shex", &result)?;
    println!("Generated: research_shapes.shex");
    println!(
        "Sample shape:
{}
...
",
        result[0].content.lines().take(20).collect::<Vec<_>>().join(
            "
"
        )
    );

    // ShEx JSON
    println!("2.2 ShEx JSON Format:");
    let shexj_generator = ShExGenerator::new().with_style(ShExStyle::Json);
    let result = shexj_generator.generate(&schema)?;
    std::fs::write("research_shapes.shexj", &result)?;
    println!(
        "Generated: research_shapes.shexj
"
    );

    // PlantUML Generation Examples
    println!("3. PlantUML Diagram Generation");
    println!(
        "==============================
"
    );

    // Class diagram
    println!("3.1 Class Diagram:");
    let class_generator = PlantUmlGenerator::new();
    let result = class_generator.generate(&schema)?;
    std::fs::write("research_class.puml", &result)?;
    println!("Generated: research_class.puml");

    // ER diagram
    println!("3.2 Entity-Relationship Diagram:");
    let er_generator =
        PlantUmlGenerator::new().with_diagram_type(PlantUmlDiagramType::EntityRelationship);
    let result = er_generator.generate(&schema)?;
    std::fs::write("research_er.puml", &result)?;
    println!("Generated: research_er.puml");

    // State diagram
    println!("3.3 State Diagram (Publication Status):");
    let state_generator = PlantUmlGenerator::new().with_diagram_type(PlantUmlDiagramType::State);
    let result = state_generator.generate(&schema)?;
    std::fs::write("research_states.puml", &result)?;
    println!("Generated: research_states.puml");

    // Mind map
    println!("3.4 Mind Map:");
    let mindmap_generator =
        PlantUmlGenerator::new().with_diagram_type(PlantUmlDiagramType::MindMap);
    let result = mindmap_generator.generate(&schema)?;
    std::fs::write("research_mindmap.puml", &result)?;
    println!(
        "Generated: research_mindmap.puml
"
    );

    // Create an integrated example showing how these work together
    let integrated_example = r#"
# Integrated Semantic Web Example

## Use Case: Research Data Management

The generated files demonstrate a complete semantic web solution:

### 1. Data Validation (ShEx)
Use the ShEx shapes to validate RDF data:
```bash
# Validate RDF data against shapes
shex-validator research_data.ttl research_shapes.shex
```

### 2. Data Querying (SPARQL)
Query research data using generated SPARQL:
```bash
# Find all publications by a researcher
sparql --data research_data.ttl --query research_select.sparql

# Transform data format
sparql --data research_data.ttl --query research_construct.sparql > transformed.ttl

# Validate data completeness
sparql --data research_data.ttl --query research_ask.sparql
```

### 3. Documentation (PlantUML)
Generate visual documentation:
```bash
# Generate PNG diagrams
plantuml research_class.puml
plantuml research_er.puml
plantuml research_states.puml
plantuml research_mindmap.puml
```

## Workflow Integration

1. **Schema Design**: Define your LinkML schema
2. **Validation Rules**: Generate ShEx shapes for data validation
3. **Query Templates**: Generate SPARQL queries for data access
4. **Documentation**: Generate PlantUML diagrams for communication
5. **Implementation**: Use generated artifacts in your semantic web application

## Example RDF Data

```turtle
@prefix ex: <http://example.org/research#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .

ex:researcher123 a ex:Researcher ;
    ex:id "researcher123" ;
    ex:name "Dr. Jane Smith" ;
    ex:orcid "0000-0002-1234-5678" ;
    ex:affiliation ex:org456 ;
    ex:publications ex:pub789 .

ex:pub789 a ex:Publication ;
    ex:id "pub789" ;
    ex:doi "10.1234/example.2024" ;
    ex:title "Advanced Research in LinkML" ;
    ex:authors ex:researcher123 ;
    ex:status "PUBLISHED" ;
    ex:publication_date "2024-01-15" .
```

## Benefits

1. **Consistency**: All artifacts generated from single schema
2. **Validation**: ShEx ensures data quality
3. **Queryability**: SPARQL enables complex data retrieval
4. **Documentation**: PlantUML provides clear visualizations
5. **Interoperability**: Standards-based semantic web technologies
"#;

    std::fs::write("semantic_web_integration.md", integrated_example)?;

    println!("✅ Semantic Web generators complete!");
    println!(
        "
Generated files:"
    );
    println!("- SPARQL queries: *.sparql");
    println!("- ShEx shapes: *.shex, *.shexj");
    println!("- PlantUML diagrams: *.puml");
    println!("- Integration guide: semantic_web_integration.md");
    println!(
        "
These tools enable:"
    );
    println!("1. RDF data validation with ShEx");
    println!("2. Semantic querying with SPARQL");
    println!("3. Visual documentation with PlantUML");
    println!("4. Complete semantic web workflows");

    Ok(())
}
