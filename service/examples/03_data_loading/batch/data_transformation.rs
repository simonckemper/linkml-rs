//! Example demonstrating data transformation between formats
//!
//! This example shows how to:
//! 1. Load data from CSV
//! 2. Transform it to RDF
//! 3. Query and manipulate the RDF data
//! 4. Convert between different RDF formats
//! 5. Export back to CSV

use linkml_core::prelude::*;
use linkml_service::loader::{
    CsvDumper, CsvLoader, DumpOptions, LoadOptions, RdfDumper, RdfLoader, RdfOptions,
    RdfSerializationFormat,
};

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "=== Data Transformation Example ===
"
    );

    // Create a knowledge graph schema
    let schema = create_knowledge_graph_schema();

    // Example 1: CSV to RDF transformation
    println!("1. CSV to RDF Transformation");
    println!(
        "===========================
"
    );

    // Sample CSV data about people and organizations
    let people_csv = r#"id,name,email,affiliation,expertise,collaborators
alice,Alice Johnson,alice@research.org,MIT,"machine learning;data science","bob;charlie"
bob,Bob Smith,bob@university.edu,Stanford,"quantum computing;algorithms",alice
charlie,Charlie Brown,charlie@lab.com,Berkeley,"bioinformatics;genomics","alice;david"
david,David Lee,david@institute.org,MIT,"robotics;AI","charlie;alice"
"#;

    // Load from CSV
    let csv_loader = CsvLoader::new();
    let load_options = LoadOptions {
        target_class: Some("Researcher".to_string()),
        validate: true,
        ..Default::default()
    };

    let researchers = csv_loader
        .load_string(people_csv, &schema, &load_options)
        .await?;
    println!("Loaded {} researchers from CSV", researchers.len());

    // Convert to RDF Turtle
    let mut rdf_options = RdfOptions::default();
    rdf_options.default_namespace = "http://research.example.org/".to_string();
    rdf_options
        .prefixes
        .insert("foaf".to_string(), "http://xmlns.com/foaf/0.1/".to_string());
    rdf_options
        .prefixes
        .insert("schema".to_string(), "http://schema.org/".to_string());
    rdf_options
        .prefixes
        .insert("ex".to_string(), "http://research.example.org/".to_string());

    let rdf_dumper = RdfDumper::with_options(rdf_options.clone());
    let turtle = rdf_dumper
        .dump_string(&researchers, &schema, &DumpOptions::default())
        .await?;

    println!("Generated Turtle RDF:");
    println!("{}", &turtle[..500.min(turtle.len())]); // First 500 chars
    println!(
        "...
"
    );

    // Save to file
    std::fs::write("researchers.ttl", &turtle)?;

    // Example 2: RDF format conversions
    println!("2. RDF Format Conversions");
    println!(
        "========================
"
    );

    // Convert to N-Triples
    let ntriples_dumper = RdfDumper::with_format(RdfSerializationFormat::NTriples);
    let ntriples = ntriples_dumper
        .dump_string(&researchers, &schema, &DumpOptions::default())
        .await?;

    println!("N-Triples format (first 5 lines):");
    for line in ntriples.lines().take(5) {
        println!("{}", line);
    }
    println!(
        "...
"
    );

    // Convert to RDF/XML
    let rdfxml_dumper = RdfDumper::with_format(RdfSerializationFormat::RdfXml);
    let rdfxml = rdfxml_dumper
        .dump_string(&researchers, &schema, &DumpOptions::default())
        .await?;

    println!("RDF/XML format (first 300 chars):");
    println!("{}", &rdfxml[..300.min(rdfxml.len())]);
    println!(
        "...
"
    );

    // Example 3: Load organizations data and merge
    println!("3. Loading and Merging Data");
    println!(
        "==========================
"
    );

    let orgs_csv = r#"id,name,type,location,founded
MIT,Massachusetts Institute of Technology,University,"Cambridge, MA",1861
Stanford,Stanford University,University,"Stanford, CA",1885
Berkeley,University of California Berkeley,University,"Berkeley, CA",1868
"#;

    let org_loader = CsvLoader::new();
    let org_options = LoadOptions {
        target_class: Some("Organization".to_string()),
        ..Default::default()
    };

    let organizations = org_loader
        .load_string(orgs_csv, &schema, &org_options)
        .await?;
    println!("Loaded {} organizations", organizations.len());

    // Combine all data
    let mut all_instances = researchers.clone();
    all_instances.extend(organizations);

    // Dump combined data as Turtle
    let combined_turtle = rdf_dumper
        .dump_string(&all_instances, &schema, &DumpOptions::default())
        .await?;
    std::fs::write("knowledge_graph.ttl", &combined_turtle)?;
    println!(
        "Created combined knowledge graph with {} instances",
        all_instances.len()
    );

    // Example 4: Round-trip through RDF
    println!(
        "
4. Round-trip Testing"
    );
    println!(
        "====================
"
    );

    // Load the Turtle back
    let rdf_loader = RdfLoader::with_options(rdf_options);
    let reloaded = rdf_loader
        .load_string(
            &combined_turtle,
            &schema,
            &LoadOptions {
                infer_types: true,
                ..Default::default()
            },
        )
        .await?;

    println!("Reloaded {} instances from Turtle", reloaded.len());

    // Export back to CSV (separate files per class)
    let csv_dumper = CsvDumper::new();

    // Separate by class
    let researchers_only: Vec<_> = reloaded
        .iter()
        .filter(|i| i.class_name == "Researcher")
        .cloned()
        .collect();

    let orgs_only: Vec<_> = reloaded
        .iter()
        .filter(|i| i.class_name == "Organization")
        .cloned()
        .collect();

    let researchers_csv = csv_dumper
        .dump_string(&researchers_only, &schema, &DumpOptions::default())
        .await?;
    let orgs_csv = csv_dumper
        .dump_string(&orgs_only, &schema, &DumpOptions::default())
        .await?;

    println!("Exported researchers CSV:");
    for line in researchers_csv.lines().take(3) {
        println!("{}", line);
    }
    println!(
        "...
"
    );

    // Example 5: Advanced RDF features
    println!("5. Advanced RDF Features");
    println!(
        "=======================
"
    );

    // Create more complex RDF with blank nodes and references
    let project_turtle = r#"
@prefix ex: <http://research.example.org/> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .

ex:project1 rdf:type ex:Project ;
    ex:name "AI for Climate Change" ;
    ex:lead ex:alice ;
    ex:members ex:bob, ex:charlie ;
    ex:funding [
        ex:amount 1000000 ;
        ex:currency "USD" ;
        ex:source "NSF"
    ] .

ex:project2 rdf:type ex:Project ;
    ex:name "Quantum Biology" ;
    ex:lead ex:charlie ;
    ex:members ex:david ;
    ex:relatedTo ex:project1 .
"#;

    // Add project to schema
    let mut extended_schema = schema.clone();
    let mut project_class = ClassDefinition::default();
    project_class.slots = vec![
        "name".to_string(),
        "lead".to_string(),
        "members".to_string(),
        "funding".to_string(),
        "relatedTo".to_string(),
    ];
    extended_schema
        .classes
        .insert("Project".to_string(), project_class);

    // Load projects
    let projects = rdf_loader
        .load_string(
            project_turtle,
            &extended_schema,
            &LoadOptions {
                infer_types: true,
                ..Default::default()
            },
        )
        .await?;

    println!("Loaded {} projects", projects.len());
    for project in &projects {
        if let Some(name) = project.data.get("name") {
            println!("Project: {}", name);
            if let Some(lead) = project.data.get("lead") {
                println!("  Lead: {}", lead);
            }
            if let Some(members) = project.data.get("members") {
                println!("  Members: {}", members);
            }
        }
    }

    // Example 6: Data statistics
    println!(
        "
6. Data Statistics"
    );
    println!(
        "=================
"
    );

    let total_instances = all_instances.len() + projects.len();
    let classes: std::collections::HashSet<_> = all_instances
        .iter()
        .chain(&projects)
        .map(|i| &i.class_name)
        .collect();

    println!("Total instances: {}", total_instances);
    println!("Classes: {:?}", classes);

    // Count relationships
    let mut relationship_count = 0;
    for instance in &all_instances {
        for (_, value) in &instance.data {
            if let serde_json::Value::String(s) = value {
                if s.starts_with("http://") {
                    relationship_count += 1;
                }
            } else if let serde_json::Value::Array(arr) = value {
                for item in arr {
                    if let serde_json::Value::String(s) = item {
                        if s.starts_with("http://") {
                            relationship_count += 1;
                        }
                    }
                }
            }
        }
    }

    println!("Relationships: {}", relationship_count);

    // Clean up
    std::fs::remove_file("researchers.ttl").ok();
    std::fs::remove_file("knowledge_graph.ttl").ok();

    println!(
        "
✅ Data transformation examples complete!"
    );
    println!(
        "
Key takeaways:"
    );
    println!("- Easy conversion between CSV and RDF formats");
    println!("- Support for multiple RDF serializations");
    println!("- Preservation of relationships and complex data");
    println!("- Round-trip data integrity");

    Ok(())
}

/// Create a knowledge graph schema
fn create_knowledge_graph_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("KnowledgeGraphSchema".to_string());
    schema.description = Some("Schema for research knowledge graph".to_string());

    // Base entity
    let mut entity = ClassDefinition::default();
    entity.abstract_ = Some(true);
    entity.slots = vec!["id".to_string(), "name".to_string()];
    schema.classes.insert("Entity".to_string(), entity);

    // Researcher class
    let mut researcher = ClassDefinition::default();
    researcher.is_a = Some("Entity".to_string());
    researcher.description = Some("A research scientist".to_string());
    researcher.slots = vec![
        "email".to_string(),
        "affiliation".to_string(),
        "expertise".to_string(),
        "collaborators".to_string(),
    ];
    schema.classes.insert("Researcher".to_string(), researcher);

    // Organization class
    let mut organization = ClassDefinition::default();
    organization.is_a = Some("Entity".to_string());
    organization.description = Some("A research organization".to_string());
    organization.slots = vec![
        "type".to_string(),
        "location".to_string(),
        "founded".to_string(),
    ];
    schema
        .classes
        .insert("Organization".to_string(), organization);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    schema.slots.insert("id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("name".to_string(), name_slot);

    let mut email_slot = SlotDefinition::default();
    email_slot.range = Some("string".to_string());
    email_slot.pattern = Some(r"^[^@]+@[^@]+\.[^@]+$".to_string());
    schema.slots.insert("email".to_string(), email_slot);

    let mut affiliation_slot = SlotDefinition::default();
    affiliation_slot.range = Some("Organization".to_string());
    schema
        .slots
        .insert("affiliation".to_string(), affiliation_slot);

    let mut expertise_slot = SlotDefinition::default();
    expertise_slot.range = Some("string".to_string());
    expertise_slot.multivalued = Some(true);
    schema.slots.insert("expertise".to_string(), expertise_slot);

    let mut collaborators_slot = SlotDefinition::default();
    collaborators_slot.range = Some("Researcher".to_string());
    collaborators_slot.multivalued = Some(true);
    schema
        .slots
        .insert("collaborators".to_string(), collaborators_slot);

    let mut type_slot = SlotDefinition::default();
    type_slot.range = Some("string".to_string());
    schema.slots.insert("type".to_string(), type_slot);

    let mut location_slot = SlotDefinition::default();
    location_slot.range = Some("string".to_string());
    schema.slots.insert("location".to_string(), location_slot);

    let mut founded_slot = SlotDefinition::default();
    founded_slot.range = Some("integer".to_string());
    schema.slots.insert("founded".to_string(), founded_slot);

    // Additional slots for extended example
    let mut lead_slot = SlotDefinition::default();
    lead_slot.range = Some("Researcher".to_string());
    schema.slots.insert("lead".to_string(), lead_slot);

    let mut members_slot = SlotDefinition::default();
    members_slot.range = Some("Researcher".to_string());
    members_slot.multivalued = Some(true);
    schema.slots.insert("members".to_string(), members_slot);

    let mut funding_slot = SlotDefinition::default();
    funding_slot.range = Some("string".to_string());
    schema.slots.insert("funding".to_string(), funding_slot);

    let mut related_slot = SlotDefinition::default();
    related_slot.range = Some("Project".to_string());
    schema.slots.insert("relatedTo".to_string(), related_slot);

    let mut amount_slot = SlotDefinition::default();
    amount_slot.range = Some("integer".to_string());
    schema.slots.insert("amount".to_string(), amount_slot);

    let mut currency_slot = SlotDefinition::default();
    currency_slot.range = Some("string".to_string());
    schema.slots.insert("currency".to_string(), currency_slot);

    let mut source_slot = SlotDefinition::default();
    source_slot.range = Some("string".to_string());
    schema.slots.insert("source".to_string(), source_slot);

    schema
}
