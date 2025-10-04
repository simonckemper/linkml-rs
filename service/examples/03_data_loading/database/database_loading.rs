//! Example demonstrating database loading and dumping in LinkML
//!
//! This example shows how to:
//! 1. Connect to different database types
//! 2. Load data from database tables into LinkML instances
//! 3. Map database schemas to LinkML schemas
//! 4. Dump LinkML instances back to databases

use linkml_core::prelude::*;
use linkml_service::loader::{
    DataDumper, DataLoader, DatabaseDumper, DatabaseLoader, DatabaseOptions, ForeignKeyRelation,
};
use std::collections::HashMap;
use tracing::info;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!(
        "=== LinkML Database Loading Example ===
"
    );

    // Create a sample schema representing a typical e-commerce database
    let schema = create_ecommerce_schema();

    // Example 1: Load from PostgreSQL
    println!("1. PostgreSQL Example");
    println!(
        "====================
"
    );

    let mut pg_options = DatabaseOptions {
        connection_string: "postgresql://user:password@localhost/ecommerce".to_string(),
        schema_name: Some("public".to_string()),
        batch_size: 1000,
        ..Default::default()
    };

    // Map table names to LinkML class names
    pg_options
        .table_mapping
        .insert("users".to_string(), "User".to_string());
    pg_options
        .table_mapping
        .insert("products".to_string(), "Product".to_string());
    pg_options
        .table_mapping
        .insert("orders".to_string(), "Order".to_string());
    pg_options
        .table_mapping
        .insert("order_items".to_string(), "OrderItem".to_string());

    // Define foreign key relationships
    pg_options.foreign_keys.insert(
        "orders".to_string(),
        vec![ForeignKeyRelation {
            column: "user_id".to_string(),
            referenced_table: "users".to_string(),
            referenced_column: "id".to_string(),
            slot_name: "customer".to_string(),
        }],
    );

    pg_options.foreign_keys.insert(
        "order_items".to_string(),
        vec![
            ForeignKeyRelation {
                column: "order_id".to_string(),
                referenced_table: "orders".to_string(),
                referenced_column: "id".to_string(),
                slot_name: "order".to_string(),
            },
            ForeignKeyRelation {
                column: "product_id".to_string(),
                referenced_table: "products".to_string(),
                referenced_column: "id".to_string(),
                slot_name: "product".to_string(),
            },
        ],
    );

    println!(
        "PostgreSQL connection string: {}",
        pg_options.connection_string
    );
    println!("Table mappings:");
    for (table, class) in &pg_options.table_mapping {
        println!("  {} -> {}", table, class);
    }
    println!();

    // Note: In a real application, you would load from an actual database
    // let mut loader = DatabaseLoader::new(pg_options);
    // let instances = loader.load(&schema).await?;
    // println!("Loaded {} instances from PostgreSQL", instances.len());

    // Example 2: Load from MySQL
    println!("2. MySQL Example");
    println!(
        "===============
"
    );

    let mut mysql_options = DatabaseOptions {
        connection_string: "mysql://user:password@localhost/ecommerce".to_string(),
        batch_size: 500,
        ..Default::default()
    };

    // Column name mappings for specific tables
    let mut user_columns = HashMap::new();
    user_columns.insert("fname".to_string(), "first_name".to_string());
    user_columns.insert("lname".to_string(), "last_name".to_string());
    mysql_options
        .column_mapping
        .insert("users".to_string(), user_columns);

    println!(
        "MySQL connection string: {}",
        mysql_options.connection_string
    );
    println!("Custom column mappings for 'users' table:");
    if let Some(mappings) = mysql_options.column_mapping.get("users") {
        for (db_col, slot) in mappings {
            println!("  {} -> {}", db_col, slot);
        }
    }
    println!();

    // Example 3: Load from SQLite
    println!("3. SQLite Example");
    println!(
        "================
"
    );

    let sqlite_options = DatabaseOptions {
        connection_string: "sqlite://./ecommerce.db".to_string(),
        infer_types: true,
        include_tables: Some(
            ["users", "products", "orders"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        ),
        ..Default::default()
    };

    println!("SQLite file: ./ecommerce.db");
    println!("Include only tables: {:?}", sqlite_options.include_tables);
    println!("Type inference: {}", sqlite_options.infer_types);
    println!();

    // Example 4: Complex loading scenario
    println!("4. Complex Loading Scenario");
    println!(
        "==========================
"
    );

    let mut complex_options = DatabaseOptions {
        connection_string: "postgresql://user:password@localhost/analytics".to_string(),
        schema_name: Some("reporting".to_string()),
        exclude_tables: ["audit_log", "temp_data", "migrations"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        batch_size: 5000,
        max_connections: 10,
        ..Default::default()
    };

    // Custom primary key columns
    complex_options
        .primary_key_columns
        .insert("user_sessions".to_string(), "session_id".to_string());

    println!("Complex scenario configuration:");
    println!("  Schema: reporting");
    println!("  Excluded tables: {:?}", complex_options.exclude_tables);
    println!("  Batch size: {}", complex_options.batch_size);
    println!("  Max connections: {}", complex_options.max_connections);
    println!("  Custom primary keys:");
    for (table, pk) in &complex_options.primary_key_columns {
        println!("    {} -> {}", table, pk);
    }
    println!();

    // Example 5: Dumping data back to database
    println!("5. Dumping Data to Database");
    println!(
        "==========================
"
    );

    // Create sample instances
    let instances = create_sample_instances()?;

    let mut dump_options = DatabaseOptions {
        connection_string: "sqlite://./output.db".to_string(),
        create_if_not_exists: true,
        use_transactions: true,
        batch_size: 100,
        ..Default::default()
    };

    // Map LinkML classes back to table names
    dump_options
        .table_mapping
        .insert("users".to_string(), "User".to_string());
    dump_options
        .table_mapping
        .insert("products".to_string(), "Product".to_string());

    println!("Dump configuration:");
    println!("  Target database: SQLite (./output.db)");
    println!("  Create tables: {}", dump_options.create_if_not_exists);
    println!("  Use transactions: {}", dump_options.use_transactions);
    println!("  Sample instances to dump: {}", instances.len());

    // Note: In a real application, you would dump to an actual database
    // let mut dumper = DatabaseDumper::new(dump_options);
    // let result = dumper.dump(&instances, &schema).await?;
    // println!("Successfully dumped {} bytes", result.len());

    println!(
        "
✅ Database loading examples complete!"
    );
    println!(
        "
Key features demonstrated:"
    );
    println!("- Connection to PostgreSQL, MySQL, and SQLite");
    println!("- Table and column name mapping");
    println!("- Foreign key relationship handling");
    println!("- Batch processing for performance");
    println!("- Type inference from database schemas");
    println!("- Selective table loading/exclusion");
    println!("- Custom primary key handling");
    println!("- Automatic table creation on dump");
    println!("- Transaction support");

    Ok(())
}

/// Create a sample e-commerce schema
fn create_ecommerce_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("EcommerceSchema".to_string());
    schema.description = Some("Schema for e-commerce database".to_string());

    // User class
    let mut user_class = ClassDefinition::default();
    user_class.description = Some("A user of the e-commerce platform".to_string());
    user_class.slots = vec![
        "id".to_string(),
        "email".to_string(),
        "first_name".to_string(),
        "last_name".to_string(),
        "created_at".to_string(),
    ];
    schema.classes.insert("User".to_string(), user_class);

    // Product class
    let mut product_class = ClassDefinition::default();
    product_class.description = Some("A product available for purchase".to_string());
    product_class.slots = vec![
        "id".to_string(),
        "name".to_string(),
        "description".to_string(),
        "price".to_string(),
        "stock_quantity".to_string(),
    ];
    schema.classes.insert("Product".to_string(), product_class);

    // Order class
    let mut order_class = ClassDefinition::default();
    order_class.description = Some("A customer order".to_string());
    order_class.slots = vec![
        "id".to_string(),
        "customer".to_string(),
        "order_date".to_string(),
        "total_amount".to_string(),
        "status".to_string(),
    ];
    schema.classes.insert("Order".to_string(), order_class);

    // OrderItem class
    let mut order_item_class = ClassDefinition::default();
    order_item_class.description = Some("An item within an order".to_string());
    order_item_class.slots = vec![
        "id".to_string(),
        "order".to_string(),
        "product".to_string(),
        "quantity".to_string(),
        "unit_price".to_string(),
    ];
    schema
        .classes
        .insert("OrderItem".to_string(), order_item_class);

    // Define slots
    let slots = vec![
        ("id", "string", true, true),
        ("email", "string", true, false),
        ("first_name", "string", false, false),
        ("last_name", "string", false, false),
        ("name", "string", true, false),
        ("description", "string", false, false),
        ("price", "float", true, false),
        ("stock_quantity", "integer", true, false),
        ("customer", "User", true, false),
        ("order_date", "datetime", true, false),
        ("total_amount", "float", true, false),
        ("status", "string", true, false),
        ("order", "Order", true, false),
        ("product", "Product", true, false),
        ("quantity", "integer", true, false),
        ("unit_price", "float", true, false),
        ("created_at", "datetime", true, false),
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
-> std::result::Result<Vec<linkml_service::loader::traits::DataInstance>, serde_json::Error> {
    use linkml_service::loader::traits::DataInstance;
    use serde_json::json;

    Ok(vec![
        DataInstance {
            class_name: "User".to_string(),
            data: serde_json::from_value(json!({
                "id": "user1",
                "email": "alice@example.com",
                "first_name": "Alice",
                "last_name": "Smith",
                "created_at": "2024-01-15T10:30:00Z"
            }))?,
        },
        DataInstance {
            class_name: "User".to_string(),
            data: serde_json::from_value(json!({
                "id": "user2",
                "email": "bob@example.com",
                "first_name": "Bob",
                "last_name": "Johnson",
                "created_at": "2024-02-20T14:45:00Z"
            }))?,
        },
        DataInstance {
            class_name: "Product".to_string(),
            data: serde_json::from_value(json!({
                "id": "prod1",
                "name": "Laptop",
                "description": "High-performance laptop",
                "price": 1299.99,
                "stock_quantity": 50
            }))?,
        },
        DataInstance {
            class_name: "Product".to_string(),
            data: serde_json::from_value(json!({
                "id": "prod2",
                "name": "Mouse",
                "description": "Wireless mouse",
                "price": 29.99,
                "stock_quantity": 200
            }))?,
        },
    ])
}
