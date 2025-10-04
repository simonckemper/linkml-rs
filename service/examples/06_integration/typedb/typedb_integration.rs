//! Example demonstrating TypeDB integration for LinkML
//!
//! This example shows how to:
//! 1. Load data from TypeDB into LinkML instances
//! 2. Map TypeDB types to LinkML classes
//! 3. Dump LinkML instances back to TypeDB
//! 4. Use both direct TypeDB connection and DBMS service

use linkml_core::prelude::*;
use linkml_service::loader::{
    DBMSServiceExecutor, DataDumper, DataLoader, TypeDBIntegrationDumper, TypeDBIntegrationLoader,
    TypeDBIntegrationOptions,
};
use std::collections::HashMap;
use tracing::info;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!(
        "=== LinkML TypeDB Integration Example ===
"
    );

    // Create a sample schema representing a knowledge graph
    let schema = create_knowledge_graph_schema();

    // Example 1: Direct TypeDB Connection
    println!("1. Direct TypeDB Connection");
    println!(
        "==========================
"
    );

    let mut direct_options = TypeDBIntegrationOptions {
        database_name: "knowledge-graph".to_string(),
        batch_size: 500,
        infer_types: true,
        include_inferred: false,
        ..Default::default()
    };

    // Map TypeDB types to LinkML classes
    direct_options
        .type_mapping
        .insert("person".to_string(), "Person".to_string());
    direct_options
        .type_mapping
        .insert("organization".to_string(), "Organization".to_string());
    direct_options
        .type_mapping
        .insert("employment".to_string(), "Employment".to_string());
    direct_options
        .type_mapping
        .insert("friendship".to_string(), "Friendship".to_string());

    // Map TypeDB attributes to LinkML slots
    let mut person_attrs = HashMap::new();
    person_attrs.insert("full-name".to_string(), "full_name".to_string());
    person_attrs.insert("birth-date".to_string(), "birth_date".to_string());
    direct_options
        .attribute_mapping
        .insert("person".to_string(), person_attrs);

    println!("Configuration:");
    println!("  Database: {}", direct_options.database_name);
    println!(
        "  Type mappings: {} types",
        direct_options.type_mapping.len()
    );
    println!(
        "  Include inferred facts: {}",
        direct_options.include_inferred
    );
    println!();

    // Create executor and loader
    let executor = DirectTypeDBExecutor::new("localhost:1729");
    let mut loader = TypeDBIntegrationLoader::new(direct_options.clone(), executor);

    // Note: In a real application, you would load from an actual TypeDB instance
    // let instances = loader.load(&schema).await?;
    // println!("Loaded {} instances from TypeDB", instances.len());

    // Example 2: Using DBMS Service
    println!("2. Using DBMS Service");
    println!(
        "====================
"
    );

    println!("DBMS Service provides:");
    println!("  - Connection pooling");
    println!("  - Health monitoring");
    println!("  - Recovery mechanisms");
    println!("  - Schema versioning");
    println!("  - Performance optimization");
    println!();

    // In a real application, you would:
    // 1. Get the DBMS service from dependency injection
    // 2. Create a DBMSServiceExecutor
    // 3. Use it with the TypeDBIntegrationLoader

    /*
    let dbms_service = get_dbms_service(); // From DI container
    let executor = DBMSServiceExecutor::new(dbms_service);
    let mut loader = TypeDBIntegrationLoader::new(options, executor);
    let instances = loader.load(&schema).await?;
    */

    // Example 3: Complex Type Mappings
    println!("3. Complex Type Mappings");
    println!(
        "=======================
"
    );

    let mut complex_options = TypeDBIntegrationOptions {
        database_name: "social-network".to_string(),
        batch_size: 1000,
        ..Default::default()
    };

    // Hierarchical type mappings
    complex_options.type_mapping.extend([
        ("user".to_string(), "User".to_string()),
        ("premium-user".to_string(), "PremiumUser".to_string()),
        ("admin-user".to_string(), "AdminUser".to_string()),
        ("post".to_string(), "Post".to_string()),
        ("comment".to_string(), "Comment".to_string()),
        ("like".to_string(), "Like".to_string()),
        ("authorship".to_string(), "Authorship".to_string()),
    ]);

    // Complex attribute mappings
    let mut post_attrs = HashMap::new();
    post_attrs.insert("content-text".to_string(), "content".to_string());
    post_attrs.insert("created-timestamp".to_string(), "created_at".to_string());
    post_attrs.insert("last-edited".to_string(), "updated_at".to_string());
    post_attrs.insert("view-count".to_string(), "views".to_string());
    complex_options
        .attribute_mapping
        .insert("post".to_string(), post_attrs);

    println!("Social network schema mapping:");
    for (typedb_type, linkml_class) in &complex_options.type_mapping {
        println!("  {} -> {}", typedb_type, linkml_class);
    }
    println!();

    // Example 4: Dumping Data to TypeDB
    println!("4. Dumping Data to TypeDB");
    println!(
        "========================
"
    );

    // Create sample instances
    let instances = create_sample_instances()?;

    let dump_options = TypeDBIntegrationOptions {
        database_name: "test-dump".to_string(),
        batch_size: 100,
        ..Default::default()
    };

    println!("Dump configuration:");
    println!("  Target database: {}", dump_options.database_name);
    println!("  Batch size: {}", dump_options.batch_size);
    println!("  Sample instances to dump: {}", instances.len());
    println!();

    // Create dumper
    let executor = DirectTypeDBExecutor::new("localhost:1729");
    let mut dumper = TypeDBIntegrationDumper::new(dump_options, executor);

    // Note: In a real application, you would dump to an actual TypeDB instance
    // let result = dumper.dump(&instances, &schema).await?;
    // println!("Successfully dumped data: {} bytes", result.len());

    // Example 5: Query Patterns
    println!("5. Query Patterns");
    println!(
        "================
"
    );

    println!("Common TypeQL patterns used by the loader:");
    println!();

    println!("Get all entity types:");
    println!("  match $x sub entity; get $x;");
    println!();

    println!("Get attributes for a type:");
    println!("  match $type type person; $type owns $attr; get $attr;");
    println!();

    println!("Load instances with attributes:");
    println!("  match $x isa person;");
    println!("    $x has full-name $name;");
    println!("    $x has age $age;");
    println!("  get $x, $name, $age;");
    println!();

    println!("Load relations with role players:");
    println!("  match $emp (employee: $person, employer: $org) isa employment;");
    println!("    $emp has start-date $date;");
    println!("  get $emp, $person, $org, $date;");
    println!();

    println!(
        "
✅ TypeDB integration examples complete!"
    );
    println!(
        "
Key features demonstrated:"
    );
    println!("- Direct TypeDB connection");
    println!("- DBMS service integration");
    println!("- Type and attribute mapping");
    println!("- Bidirectional data transformation");
    println!("- Complex schema handling");
    println!("- Query pattern examples");

    Ok(())
}

/// Create a sample knowledge graph schema
fn create_knowledge_graph_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("KnowledgeGraphSchema".to_string());
    schema.description =
        Some("Schema for a knowledge graph with people and organizations".to_string());

    // Person class
    let mut person_class = ClassDefinition::default();
    person_class.description = Some("A person in the knowledge graph".to_string());
    person_class.slots = vec![
        "id".to_string(),
        "full_name".to_string(),
        "email".to_string(),
        "birth_date".to_string(),
        "age".to_string(),
    ];
    schema.classes.insert("Person".to_string(), person_class);

    // Organization class
    let mut org_class = ClassDefinition::default();
    org_class.description = Some("An organization".to_string());
    org_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "founded_date".to_string(),
        "industry".to_string(),
    ];
    schema.classes.insert("Organization".to_string(), org_class);

    // Employment relation class
    let mut employment_class = ClassDefinition::default();
    employment_class.description = Some("Employment relationship".to_string());
    employment_class.slots = vec![
        "employee".to_string(),
        "employer".to_string(),
        "start_date".to_string(),
        "end_date".to_string(),
        "role".to_string(),
    ];
    schema
        .classes
        .insert("Employment".to_string(), employment_class);

    // Friendship relation class
    let mut friendship_class = ClassDefinition::default();
    friendship_class.description = Some("Friendship relationship".to_string());
    friendship_class.slots = vec![
        "friend1".to_string(),
        "friend2".to_string(),
        "since_date".to_string(),
    ];
    schema
        .classes
        .insert("Friendship".to_string(), friendship_class);

    // Define slots
    let slots = vec![
        ("id", "string", true, true),
        ("full_name", "string", true, false),
        ("name", "string", true, false),
        ("email", "string", false, false),
        ("birth_date", "date", false, false),
        ("age", "integer", false, false),
        ("founded_date", "date", false, false),
        ("industry", "string", false, false),
        ("employee", "Person", true, false),
        ("employer", "Organization", true, false),
        ("friend1", "Person", true, false),
        ("friend2", "Person", true, false),
        ("start_date", "date", true, false),
        ("end_date", "date", false, false),
        ("since_date", "date", false, false),
        ("role", "string", true, false),
    ];

    for (name, range, required, identifier) in slots {
        let mut slot = SlotDefinition::default();
        slot.range = Some(range.to_string());
        slot.required = Some(required);
        slot.identifier = Some(identifier);
        schema.slots.insert(name.to_string(), slot);
    }

    schema
}

/// Create sample instances for dumping
fn create_sample_instances()
-> Result<Vec<linkml_service::loader::traits::DataInstance>, serde_json::Error> {
    use linkml_service::loader::traits::DataInstance;
    use serde_json::json;

    Ok(vec![
        DataInstance {
            class_name: "Person".to_string(),
            data: serde_json::from_value(json!({
                "id": "person-001",
                "full_name": "Alice Johnson",
                "email": "alice@example.com",
                "birth_date": "1985-03-15",
                "age": 39
            }))?,
        },
        DataInstance {
            class_name: "Person".to_string(),
            data: serde_json::from_value(json!({
                "id": "person-002",
                "full_name": "Bob Smith",
                "email": "bob@example.com",
                "birth_date": "1990-07-22",
                "age": 34
            }))?,
        },
        DataInstance {
            class_name: "Organization".to_string(),
            data: serde_json::from_value(json!({
                "id": "org-001",
                "name": "TechCorp",
                "founded_date": "2010-01-01",
                "industry": "Technology"
            }))?,
        },
        DataInstance {
            class_name: "Employment".to_string(),
            data: serde_json::from_value(json!({
                "employee": {
                    "@type": "Person",
                    "id": "person-001"
                },
                "employer": {
                    "@type": "Organization",
                    "id": "org-001"
                },
                "start_date": "2020-06-01",
                "role": "Senior Engineer"
            }))?,
        },
        DataInstance {
            class_name: "Friendship".to_string(),
            data: serde_json::from_value(json!({
                "friend1": {
                    "@type": "Person",
                    "id": "person-001"
                },
                "friend2": {
                    "@type": "Person",
                    "id": "person-002"
                },
                "since_date": "2018-09-15"
            }))?,
        },
    ])
}
