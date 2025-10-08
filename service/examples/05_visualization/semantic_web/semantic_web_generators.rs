//! Generate semantic-web artefacts (PlantUML, ShEx, SPARQL) for a simple schema.

use anyhow::Result;
use linkml_core::prelude::*;
use linkml_service::generator::{PlantUmlGenerator, ShExGenerator, SparqlGenerator};
use std::fs;
use std::path::Path;

const OUTPUT_DIR: &str = "/tmp";

fn main() -> Result<()> {
    let schema = build_schema();

    let plantuml = PlantUmlGenerator::new().generate(&schema)?;
    let shex = ShExGenerator::new().generate(&schema)?;
    let sparql = SparqlGenerator::new().generate(&schema)?;

    write("research_uml.puml", &plantuml)?;
    write("research_shapes.shex", &shex)?;
    write("research_queries.sparql", &sparql)?;

    println!("Artifacts written to {OUTPUT_DIR}:");
    println!("  • research_uml.puml (PlantUML class diagram)");
    println!("  • research_shapes.shex (ShEx shapes)");
    println!("  • research_queries.sparql (sample queries)");

    Ok(())
}

fn write(name: &str, contents: &str) -> Result<()> {
    let path = Path::new(OUTPUT_DIR).join(name);
    fs::write(&path, contents)?;
    Ok(())
}

fn build_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.id = "https://example.org/research".to_string();
    schema.name = "ResearchSchema".to_string();
    schema.description = Some("Minimal research metadata model".to_string());

    let mut entity = ClassDefinition::default();
    entity.name = Some("Entity".to_string());
    entity.abstract_ = Some(true);
    entity.slots = vec!["id".to_string(), "name".to_string(), "created".to_string()];
    schema.classes.insert("Entity".to_string(), entity);

    let mut dataset = ClassDefinition::default();
    dataset.name = Some("Dataset".to_string());
    dataset.is_a = Some("Entity".to_string());
    dataset.slots = vec![
        "description".to_string(),
        "format".to_string(),
        "size_mb".to_string(),
    ];
    schema.classes.insert("Dataset".to_string(), dataset);

    let mut publication = ClassDefinition::default();
    publication.name = Some("Publication".to_string());
    publication.is_a = Some("Entity".to_string());
    publication.slots = vec![
        "doi".to_string(),
        "journal".to_string(),
        "status".to_string(),
    ];
    schema
        .classes
        .insert("Publication".to_string(), publication);

    let mut researcher = ClassDefinition::default();
    researcher.name = Some("Researcher".to_string());
    researcher.is_a = Some("Entity".to_string());
    researcher.slots = vec!["orcid".to_string(), "affiliation".to_string()];
    schema.classes.insert("Researcher".to_string(), researcher);

    schema
        .slots
        .insert("id".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("name".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("created".to_string(), make_slot("date", false));
    schema
        .slots
        .insert("description".to_string(), make_slot("string", false));
    schema
        .slots
        .insert("format".to_string(), make_slot("string", false));

    let mut size_slot = make_slot("float", false);
    size_slot.minimum_value = Some(serde_json::json!(0));
    schema.slots.insert("size_mb".to_string(), size_slot);

    schema
        .slots
        .insert("doi".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("journal".to_string(), make_slot("string", false));
    schema
        .slots
        .insert("status".to_string(), make_slot("PublicationStatus", false));
    schema
        .slots
        .insert("orcid".to_string(), make_slot("string", true));
    schema
        .slots
        .insert("affiliation".to_string(), make_slot("string", false));

    let mut status_enum = EnumDefinition::default();
    status_enum.permissible_values = vec![
        PermissibleValue::Simple("DRAFT".to_string()),
        PermissibleValue::Simple("SUBMITTED".to_string()),
        PermissibleValue::Simple("PUBLISHED".to_string()),
    ];
    schema
        .enums
        .insert("PublicationStatus".to_string(), status_enum);

    schema
}

fn make_slot(range: &str, required: bool) -> SlotDefinition {
    let mut slot = SlotDefinition::default();
    slot.range = Some(range.to_string());
    slot.required = Some(required);
    slot
}
