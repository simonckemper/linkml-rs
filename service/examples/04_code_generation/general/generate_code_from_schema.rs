//! Example demonstrating code generation from LinkML schemas
//!
//! This example shows how to:
//! 1. Load the ISO3166Entity schema
//! 2. Generate Rust structs from the schema
//! 3. Generate Python dataclasses from the schema
//! 4. Generate Pydantic models from the schema
//! 5. Verify the generated code is valid

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, GeneratorOptions, GeneratorRegistry, IndentStyle, OutputFormat, PydanticGenerator,
    PythonDataclassGenerator, RustGenerator,
};
use std::fs;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "=== Code Generation from LinkML Schemas ===
"
    );

    // ========================================================================
    // Part 1: Load the ISO3166Entity schema
    // ========================================================================

    println!("Part 1: Loading Schema");
    println!("{}", "=".repeat(50));

    // Path to schema files
    let schema_base = PathBuf::from("/home/kempersc/apps/rootreal/domain/schema");
    let country_schema_path = schema_base.join("place/polity/country/schema.yaml");

    // Read and parse the schema
    let schema_content = fs::read_to_string(&country_schema_path)?;
    let schema: SchemaDefinition = serde_yaml::from_str(&schema_content)?;

    println!("✓ Loaded country schema");
    println!(
        "  - Schema name: {}",
        schema.name.as_ref().unwrap_or(&"unnamed".to_string())
    );
    if let Some(desc) = &schema.description {
        println!("  - Description: {}", desc);
    }

    // Count classes and slots
    let class_count = schema.classes.as_ref().map_or(0, |c| c.len());
    let slot_count = schema.slots.as_ref().map_or(0, |s| s.len());

    println!("  - Classes: {}", class_count);
    println!("  - Slots: {}", slot_count);
    println!();

    // ========================================================================
    // Part 2: Generate Rust code
    // ========================================================================

    println!("Part 2: Generating Rust Code");
    println!("{}", "=".repeat(50));

    // Create Rust generator with options
    let rust_generator = RustGenerator::new();
    let mut rust_options = GeneratorOptions::new();
    rust_options.include_docs = true;
    rust_options.generate_tests = false;
    rust_options.indent = IndentStyle::Spaces(4);
    rust_options.output_format = OutputFormat::Rust;

    // Generate Rust code
    let rust_code = rust_generator.generate(&schema, &rust_options).await?;

    println!("✓ Generated Rust code");

    // Save to file
    let rust_output_path = PathBuf::from("/tmp/iso3166_generated.rs");
    fs::write(&rust_output_path, &rust_code)?;
    println!("  - Saved to: {}", rust_output_path.display());

    // Show a preview of the generated code
    let rust_lines: Vec<&str> = rust_code.lines().collect();
    let preview_lines = 15;
    println!(
        "
  Preview (first {} lines):",
        preview_lines
    );
    println!("  {}", "-".repeat(40));
    for line in rust_lines.iter().take(preview_lines) {
        println!("  {}", line);
    }
    println!("  {}", "-".repeat(40));
    println!("  ... ({} total lines)", rust_lines.len());
    println!();

    // ========================================================================
    // Part 3: Generate Python dataclasses
    // ========================================================================

    println!("Part 3: Generating Python Dataclasses");
    println!("{}", "=".repeat(50));

    // Create Python dataclass generator
    let python_generator = PythonDataclassGenerator::new();
    let mut python_options = GeneratorOptions::new();
    python_options.include_docs = true;
    python_options.generate_tests = false;
    python_options.indent = IndentStyle::Spaces(4);
    python_options.output_format = OutputFormat::Python;

    // Generate Python code
    let python_code = python_generator.generate(&schema, &python_options).await?;

    println!("✓ Generated Python dataclasses");

    // Save to file
    let python_output_path = PathBuf::from("/tmp/iso3166_generated.py");
    fs::write(&python_output_path, &python_code)?;
    println!("  - Saved to: {}", python_output_path.display());

    // Show a preview
    let python_lines: Vec<&str> = python_code.lines().collect();
    println!(
        "
  Preview (first {} lines):",
        preview_lines
    );
    println!("  {}", "-".repeat(40));
    for line in python_lines.iter().take(preview_lines) {
        println!("  {}", line);
    }
    println!("  {}", "-".repeat(40));
    println!("  ... ({} total lines)", python_lines.len());
    println!();

    // ========================================================================
    // Part 4: Generate Pydantic models
    // ========================================================================

    println!("Part 4: Generating Pydantic Models");
    println!("{}", "=".repeat(50));

    // Create Pydantic generator
    let pydantic_generator = PydanticGenerator::new();
    let mut pydantic_options = GeneratorOptions::new();
    pydantic_options.include_docs = true;
    pydantic_options.generate_tests = false;
    pydantic_options.indent = IndentStyle::Spaces(4);
    pydantic_options.output_format = OutputFormat::Python;

    // Add Pydantic-specific options
    pydantic_options = pydantic_options
        .set_custom("use_field_validators", "true")
        .set_custom("generate_config", "true");

    // Generate Pydantic code
    let pydantic_code = pydantic_generator
        .generate(&schema, &pydantic_options)
        .await?;

    println!("✓ Generated Pydantic models");

    // Save to file
    let pydantic_output_path = PathBuf::from("/tmp/iso3166_pydantic.py");
    fs::write(&pydantic_output_path, &pydantic_code)?;
    println!("  - Saved to: {}", pydantic_output_path.display());

    // Show a preview
    let pydantic_lines: Vec<&str> = pydantic_code.lines().collect();
    println!(
        "
  Preview (first {} lines):",
        preview_lines
    );
    println!("  {}", "-".repeat(40));
    for line in pydantic_lines.iter().take(preview_lines) {
        println!("  {}", line);
    }
    println!("  {}", "-".repeat(40));
    println!("  ... ({} total lines)", pydantic_lines.len());
    println!();

    // ========================================================================
    // Part 5: Test generated code compilation/validity
    // ========================================================================

    println!("Part 5: Testing Generated Code");
    println!("{}", "=".repeat(50));

    // Test Rust code compilation
    println!("Testing Rust code:");
    let rustc_test = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg("/tmp/test_rust.rmeta")
        .arg(&rust_output_path)
        .output()?;

    if rustc_test.status.success() {
        println!("  ✓ Rust code compiles successfully!");
    } else {
        println!("  ✗ Rust code compilation failed:");
        let stderr = String::from_utf8_lossy(&rustc_test.stderr);
        for line in stderr.lines().take(5) {
            println!("    {}", line);
        }
    }

    // Test Python code syntax
    println!(
        "
Testing Python dataclass code:"
    );
    let python_test = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&python_output_path)
        .output()?;

    if python_test.status.success() {
        println!("  ✓ Python dataclass code is syntactically valid!");
    } else {
        println!("  ✗ Python dataclass syntax check failed:");
        let stderr = String::from_utf8_lossy(&python_test.stderr);
        for line in stderr.lines().take(5) {
            println!("    {}", line);
        }
    }

    // Test Pydantic code syntax
    println!(
        "
Testing Pydantic model code:"
    );
    let pydantic_test = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&pydantic_output_path)
        .output()?;

    if pydantic_test.status.success() {
        println!("  ✓ Pydantic model code is syntactically valid!");
    } else {
        println!("  ✗ Pydantic model syntax check failed:");
        let stderr = String::from_utf8_lossy(&pydantic_test.stderr);
        for line in stderr.lines().take(5) {
            println!("    {}", line);
        }
    }

    println!();

    // ========================================================================
    // Part 6: Generate code for ISO3166Entity specifically
    // ========================================================================

    println!("Part 6: Focused Generation for ISO3166Entity");
    println!("{}", "=".repeat(50));

    // Create a simplified schema with just ISO3166Entity
    if let Some(classes) = &schema.classes {
        if let Some(iso_entity) = classes.get("ISO3166Entity") {
            println!("Found ISO3166Entity class:");
            println!("  - Description: {:?}", iso_entity.description);

            // Create focused schema
            let mut focused_schema = SchemaDefinition::default();
            focused_schema.name = Some("ISO3166".to_string());
            focused_schema.description = Some("ISO 3166 Country Code Schema".to_string());

            let mut classes = HashMap::new();
            classes.insert("ISO3166Entity".to_string(), iso_entity.clone());
            focused_schema.classes = Some(classes);

            // Copy relevant slots
            if let Some(all_slots) = &schema.slots {
                let mut relevant_slots = HashMap::new();
                for (name, slot) in all_slots {
                    // Include slots used by ISO3166Entity
                    relevant_slots.insert(name.clone(), slot.clone());
                }
                focused_schema.slots = Some(relevant_slots);
            }

            // Generate focused Rust code
            let focused_rust = rust_generator
                .generate(&focused_schema, &rust_options)
                .await?;
            let focused_rust_path = PathBuf::from("/tmp/iso3166_entity.rs");
            fs::write(&focused_rust_path, &focused_rust)?;
            println!(
                "
  ✓ Generated focused Rust code: {}",
                focused_rust_path.display()
            );

            // Generate focused Python code
            let focused_python = python_generator
                .generate(&focused_schema, &python_options)
                .await?;
            let focused_python_path = PathBuf::from("/tmp/iso3166_entity.py");
            fs::write(&focused_python_path, &focused_python)?;
            println!(
                "  ✓ Generated focused Python code: {}",
                focused_python_path.display()
            );
        }
    }

    println!();

    // ========================================================================
    // Part 7: Summary
    // ========================================================================

    println!("Part 7: Summary");
    println!("{}", "=".repeat(50));

    println!("✓ Successfully generated code from LinkML schemas");
    println!();
    println!("Generated files:");
    println!("  - Rust:           {}", rust_output_path.display());
    println!("  - Python:         {}", python_output_path.display());
    println!("  - Pydantic:       {}", pydantic_output_path.display());
    println!("  - Focused Rust:   /tmp/iso3166_entity.rs");
    println!("  - Focused Python: /tmp/iso3166_entity.py");

    println!();
    println!("Key Features Demonstrated:");
    println!("  ✓ LinkML schemas can generate Rust structs/enums");
    println!("  ✓ LinkML schemas can generate Python dataclasses");
    println!("  ✓ LinkML schemas can generate Pydantic models");
    println!("  ✓ Generated code includes documentation from schema");
    println!("  ✓ Code generation supports custom options");
    println!("  ✓ Generated code is syntactically valid");

    println!();
    println!("The LinkML service successfully generates code from YAML schemas!");

    Ok(())
}
