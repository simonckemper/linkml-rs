//! Example demonstrating CSV data loading and dumping
//!
//! This example shows how to:
//! 1. Load CSV data into LinkML instances
//! 2. Validate loaded data against a schema
//! 3. Transform and manipulate the data
//! 4. Dump the data back to CSV format

use linkml_core::prelude::*;
use linkml_service::loader::{CsvDumper, CsvLoader, CsvOptions, DumpOptions, LoadOptions};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== CSV Data Loading and Dumping Example ===
");

    // Create a research dataset schema
    let mut schema = create_research_schema();

    // Example 1: Basic CSV loading
    println!("1. Basic CSV Loading");
    println!("===================
");

    let csv_data = r#"experiment_id,sample_name,temperature,ph,concentration,tags,notes
EXP001,Sample A,25.5,7.2,0.5,"control;baseline",Initial measurement
EXP002,Sample B,30.0,6.8,1.2,"treatment;high-temp",Heat treatment applied
EXP003,Sample C,25.5,7.5,0.8,"control;replicate",Replicate of Sample A
EXP004,Sample D,20.0,7.0,0.6,"treatment;low-temp",Cold treatment
"#;

    // Create CSV loader
    let loader = CsvLoader::new();

    // Load with automatic type inference
    let load_options = LoadOptions {
        target_class: Some("Experiment".to_string()),
        validate: true,
        infer_types: true,
        ..Default::default()
    };

    let instances = loader.load_string(csv_data, &schema, &load_options).await?;

    println!("Loaded {} experiments", instances.len());
    for (i, instance) in instances.iter().enumerate() {
        println!("
Experiment {}:", i + 1);
        println!("  ID: {}", instance.id.as_ref()?);
        println!("  Sample: {}", instance.data.get("sample_name")?);
        println!("  Temperature: {}°C", instance.data.get("temperature")?);
        println!("  Tags: {}", instance.data.get("tags")?);
    }

    // Example 2: Loading with field mappings
    println!("

2. Loading with Field Mappings");
    println!("==============================
");

    let csv_with_different_headers = r#"ID,Name,Temp(C),pH_value,Conc_mg/ml,Keywords,Comments
EXP005,Sample E,22.0,7.1,0.9,"validation;standard",Standard conditions
EXP006,Sample F,28.0,6.9,1.5,"experimental;novel",New protocol test
"#;

    // Create field mappings
    let mut field_mappings = HashMap::new();
    field_mappings.insert("ID".to_string(), "experiment_id".to_string());
    field_mappings.insert("Name".to_string(), "sample_name".to_string());
    field_mappings.insert("Temp(C)".to_string(), "temperature".to_string());
    field_mappings.insert("pH_value".to_string(), "ph".to_string());
    field_mappings.insert("Conc_mg/ml".to_string(), "concentration".to_string());
    field_mappings.insert("Keywords".to_string(), "tags".to_string());
    field_mappings.insert("Comments".to_string(), "notes".to_string());

    let mapped_options = LoadOptions {
        target_class: Some("Experiment".to_string()),
        field_mappings,
        ..Default::default()
    };

    let mapped_instances = loader
        .load_string(csv_with_different_headers, &schema, &mapped_options)
        .await?;
    println!(
        "Loaded {} experiments with field mapping",
        mapped_instances.len()
    );

    // Example 3: TSV format
    println!("

3. TSV Format Loading");
    println!("====================
");

    let tsv_data = "experiment_id\tsample_name\ttemperature\tph\tconcentration\ttags\tnotes
EXP007\tSample G\t26.0\t7.3\t1.0\ttsv-test;format\tTSV format test
EXP008\tSample H\t24.5\t6.7\t0.7\ttsv-test;validation\tAnother TSV record
";

    let tsv_loader = CsvLoader::tsv();
    let tsv_instances = tsv_loader
        .load_string(tsv_data, &schema, &load_options)
        .await?;
    println!("Loaded {} experiments from TSV", tsv_instances.len());

    // Example 4: Handling errors and validation
    println!("

4. Error Handling and Validation");
    println!("================================
");

    let invalid_csv = r#"experiment_id,sample_name,temperature,ph,concentration,tags,notes
EXP009,Sample I,not_a_number,7.0,0.5,invalid,Temperature is invalid
EXP010,Sample J,25.0,15.0,0.5,invalid,pH out of range
EXP011,Sample K,25.0,7.0,-0.5,invalid,Negative concentration
"#;

    let strict_options = LoadOptions {
        target_class: Some("Experiment".to_string()),
        validate: true,
        skip_invalid: true, // Skip invalid records instead of failing
        ..Default::default()
    };

    let valid_instances = loader
        .load_string(invalid_csv, &schema, &strict_options)
        .await?;
    println!(
        "Loaded {} valid experiments (skipped invalid ones)",
        valid_instances.len()
    );

    // Example 5: Dumping data back to CSV
    println!("

5. Dumping Data to CSV");
    println!("=====================
");

    // Combine all loaded instances
    let mut all_instances = instances.clone();
    all_instances.extend(mapped_instances);
    all_instances.extend(tsv_instances);

    // Create dumper
    let dumper = CsvDumper::new();

    // Dump with default options
    let dump_options = DumpOptions::default();
    let csv_output = dumper
        .dump_string(&all_instances, &schema, &dump_options)
        .await?;

    println!("Dumped CSV (first 5 lines):");
    for line in csv_output.lines().take(5) {
        println!("{}", line);
    }
    println!("...");

    // Example 6: Dumping with custom options
    println!("

6. Custom Dump Options");
    println!("====================
");

    let custom_dump_options = DumpOptions {
        include_metadata: true,
        pretty_print: true,
        include_nulls: false,
        limit: Some(3), // Only dump first 3 records
        ..Default::default()
    };

    let limited_output = dumper
        .dump_string(&all_instances, &schema, &custom_dump_options)
        .await?;

    println!("Limited output (3 records):");
    println!("{}", limited_output);

    // Example 7: Round-trip test
    println!("

7. Round-trip Test");
    println!("=================
");

    // Dump and reload
    let roundtrip_csv = dumper
        .dump_string(&all_instances, &schema, &dump_options)
        .await?;
    let reloaded = loader
        .load_string(&roundtrip_csv, &schema, &load_options)
        .await?;

    println!("Original instances: {}", all_instances.len());
    println!("Reloaded instances: {}", reloaded.len());
    println!(
        "Round-trip successful: {}",
        all_instances.len() == reloaded.len()
    );

    // Example 8: Working with files
    println!("

8. File Operations");
    println!("=================
");

    // Save to file
    let output_path = std::path::Path::new("experiments.csv");
    dumper
        .dump_file(&all_instances, output_path, &schema, &dump_options)
        .await?;
    println!("Saved to: {}", output_path.display());

    // Load from file
    let file_instances = loader
        .load_file(output_path, &schema, &load_options)
        .await?;
    println!("Loaded {} instances from file", file_instances.len());

    // Clean up
    std::fs::remove_file(output_path)?;

    println!("
✅ CSV loading and dumping examples complete!");

    Ok(())
}

/// Create a research experiment schema
fn create_research_schema() -> SchemaDefinition {
    let mut schema = SchemaDefinition::default();
    schema.name = Some("ResearchDataSchema".to_string());
    schema.description = Some("Schema for research experiment data".to_string());

    // Experiment class
    let mut experiment_class = ClassDefinition::default();
    experiment_class.description = Some("A research experiment record".to_string());
    experiment_class.slots = vec![
        "experiment_id".to_string(),
        "sample_name".to_string(),
        "temperature".to_string(),
        "ph".to_string(),
        "concentration".to_string(),
        "tags".to_string(),
        "notes".to_string(),
        "timestamp".to_string(),
    ];
    schema
        .classes
        .insert("Experiment".to_string(), experiment_class);

    // Define slots
    let mut id_slot = SlotDefinition::default();
    id_slot.identifier = Some(true);
    id_slot.range = Some("string".to_string());
    id_slot.required = Some(true);
    id_slot.pattern = Some(r"^EXP\d{3,}$".to_string());
    schema.slots.insert("experiment_id".to_string(), id_slot);

    let mut name_slot = SlotDefinition::default();
    name_slot.range = Some("string".to_string());
    name_slot.required = Some(true);
    schema.slots.insert("sample_name".to_string(), name_slot);

    let mut temp_slot = SlotDefinition::default();
    temp_slot.range = Some("float".to_string());
    temp_slot.minimum_value = Some(serde_json::json!(-273.15)); // Absolute zero
    temp_slot.maximum_value = Some(serde_json::json!(1000.0));
    temp_slot.unit = Some("celsius".to_string());
    schema.slots.insert("temperature".to_string(), temp_slot);

    let mut ph_slot = SlotDefinition::default();
    ph_slot.range = Some("float".to_string());
    ph_slot.minimum_value = Some(serde_json::json!(0.0));
    ph_slot.maximum_value = Some(serde_json::json!(14.0));
    schema.slots.insert("ph".to_string(), ph_slot);

    let mut conc_slot = SlotDefinition::default();
    conc_slot.range = Some("float".to_string());
    conc_slot.minimum_value = Some(serde_json::json!(0.0));
    conc_slot.unit = Some("mg/ml".to_string());
    schema.slots.insert("concentration".to_string(), conc_slot);

    let mut tags_slot = SlotDefinition::default();
    tags_slot.range = Some("string".to_string());
    tags_slot.multivalued = Some(true);
    tags_slot.description = Some("Semicolon-separated tags".to_string());
    schema.slots.insert("tags".to_string(), tags_slot);

    let mut notes_slot = SlotDefinition::default();
    notes_slot.range = Some("string".to_string());
    schema.slots.insert("notes".to_string(), notes_slot);

    let mut timestamp_slot = SlotDefinition::default();
    timestamp_slot.range = Some("datetime".to_string());
    schema.slots.insert("timestamp".to_string(), timestamp_slot);

    schema
}
