//! CSV Introspection Demo
//!
//! This example demonstrates how to use the CSV introspector to analyze
//! CSV files and generate LinkML schemas automatically.
//!
//! # Usage
//! ```bash
//! cargo run --example csv_introspection_demo
//! ```

use linkml_service::inference::{CsvIntrospector, DataIntrospector};
use logger_service::create_logger_service;
use timestamp_service::create_timestamp_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("CSV Introspection Demo");
    println!("======================\n");

    // Create required services
    let logger = create_logger_service()?;
    let timestamp = create_timestamp_service()?;

    // Create CSV introspector
    let introspector = CsvIntrospector::new(logger, timestamp);

    // Example 1: CSV with headers
    println!("Example 1: CSV with headers");
    println!("---------------------------");
    let csv_with_headers = b"name,age,email,active
John Doe,25,john@example.com,true
Jane Smith,30,jane@example.com,true
Bob Johnson,35,bob@example.com,false
Alice Williams,28,alice@example.com,true";

    let stats = introspector.analyze_bytes(csv_with_headers).await?;
    println!("Document ID: {}", stats.document_id);
    println!("Format: {}", stats.format);
    println!("Columns detected: {}", stats.elements.len());
    println!("Column names:");
    for (name, _) in &stats.elements {
        println!("  - {}", name);
    }
    println!();

    // Generate schema
    let schema = introspector.generate_schema(&stats, "people_schema").await?;
    println!("Generated schema ID: {}", schema.id);
    println!("Schema name: {}", schema.name);
    println!("Classes: {}", schema.classes.len());
    println!();

    // Example 2: CSV without headers (numeric data)
    println!("Example 2: CSV without headers");
    println!("-------------------------------");
    let csv_without_headers = b"25,30,35
40,45,50
55,60,65";

    let stats2 = introspector.analyze_bytes(csv_without_headers).await?;
    println!("Document ID: {}", stats2.document_id);
    println!("Columns detected: {}", stats2.elements.len());
    println!("Auto-generated column names:");
    for (name, _) in &stats2.elements {
        println!("  - {}", name);
    }
    println!();

    // Example 3: Tab-delimited data
    println!("Example 3: Tab-delimited CSV");
    println!("----------------------------");
    let tab_csv = b"product\tprice\tstock\ncategory
Laptop\t999.99\t15\tElectronics
Mouse\t29.99\t100\tElectronics
Keyboard\t79.99\t50\tElectronics";

    let stats3 = introspector.analyze_bytes(tab_csv).await?;
    println!("Document ID: {}", stats3.document_id);
    println!("Columns detected: {}", stats3.elements.len());
    println!("Column names:");
    for (name, _) in &stats3.elements {
        println!("  - {}", name);
    }
    println!();

    // Example 4: CSV with missing values
    println!("Example 4: CSV with missing values");
    println!("-----------------------------------");
    let csv_with_missing = b"name,age,city
John,25,
Jane,,Los Angeles
Bob,35,New York
Alice,28,";

    let stats4 = introspector.analyze_bytes(csv_with_missing).await?;
    println!("Document ID: {}", stats4.document_id);
    println!("Columns: {}", stats4.elements.len());
    println!("Column details:");
    for (name, element_stats) in &stats4.elements {
        println!("  - {}: {} samples", name, element_stats.text_samples.len());
    }
    println!();

    // Example 5: Semicolon-delimited CSV (European format)
    println!("Example 5: Semicolon-delimited CSV");
    println!("-----------------------------------");
    let semicolon_csv = b"name;age;salary
John Doe;25;50000
Jane Smith;30;65000
Bob Johnson;35;75000";

    let stats5 = introspector.analyze_bytes(semicolon_csv).await?;
    println!("Document ID: {}", stats5.document_id);
    println!("Columns detected: {}", stats5.elements.len());
    println!("Column names:");
    for (name, _) in &stats5.elements {
        println!("  - {}", name);
    }
    println!();

    // Example 6: CSV with quoted values containing delimiters
    println!("Example 6: CSV with quoted values");
    println!("----------------------------------");
    let quoted_csv = br#"name,description,price
"Product A","A product with a comma, in the description",29.99
"Product B","Another product with ""quotes""",39.99
"Product C","Simple description",19.99"#;

    let stats6 = introspector.analyze_bytes(quoted_csv).await?;
    println!("Document ID: {}", stats6.document_id);
    println!("Columns detected: {}", stats6.elements.len());
    println!("Successfully handled quoted values with special characters");
    println!();

    println!("Demo completed successfully!");
    println!("============================");
    println!();
    println!("Key Features Demonstrated:");
    println!("- Automatic header detection");
    println!("- Delimiter detection (comma, tab, semicolon)");
    println!("- Column type inference");
    println!("- Missing value handling");
    println!("- Quoted value support");
    println!("- LinkML schema generation");

    Ok(())
}
