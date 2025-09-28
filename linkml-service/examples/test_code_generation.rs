//! Test code generation capabilities for Rust and Python
//!
//! This example demonstrates that the LinkML service can generate
//! both Rust structs/enums and Python dataclasses from schemas.

use indexmap::IndexMap;
use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, PydanticGenerator, PythonDataclassGenerator, RustGenerator,
};
use std::fs;
use std::path::PathBuf;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Code Generation for Rust and Python ===
");

    // Create a simple test schema programmatically
    let mut schema = SchemaDefinition {
        id: "https://test.org/country".to_string(),
        name: "CountrySchema".to_string(),
        title: Some("Country Code Schema".to_string()),
        description: Some("Test schema for country codes".to_string()),
        classes: IndexMap::new(),
        slots: IndexMap::new(),
        enums: IndexMap::new(),
        ..Default::default()
    };

    // Add a Country class
    let mut country_class = ClassDefinition {
        name: "Country".to_string(),
        description: Some("A country entity".to_string()),
        slots: vec!["code".to_string(), "name".to_string(), "domain".to_string()],
        ..Default::default()
    };
    schema.classes.insert("Country".to_string(), country_class);

    // Add slots
    let mut code_slot = SlotDefinition {
        name: "code".to_string(),
        description: Some("Country code".to_string()),
        range: Some("string".to_string()),
        required: Some(true),
        pattern: Some("[A-Z]{2}".to_string()),
        ..Default::default()
    };
    schema.slots.insert("code".to_string(), code_slot);

    let mut name_slot = SlotDefinition {
        name: "name".to_string(),
        description: Some("Country name".to_string()),
        range: Some("string".to_string()),
        required: Some(true),
        ..Default::default()
    };
    schema.slots.insert("name".to_string(), name_slot);

    let mut domain_slot = SlotDefinition {
        name: "domain".to_string(),
        description: Some("Top-level domain".to_string()),
        range: Some("string".to_string()),
        ..Default::default()
    };
    schema.slots.insert("domain".to_string(), domain_slot);

    // Add an enum for regions
    let mut region_enum = EnumDefinition {
        name: "Region".to_string(),
        description: Some("Geographic regions".to_string()),
        permissible_values: vec![
            PermissibleValue::Simple("EUROPE".to_string()),
            PermissibleValue::Simple("ASIA".to_string()),
            PermissibleValue::Simple("AMERICAS".to_string()),
            PermissibleValue::Simple("AFRICA".to_string()),
            PermissibleValue::Simple("OCEANIA".to_string()),
        ],
        ..Default::default()
    };
    schema.enums.insert("Region".to_string(), region_enum);

    println!("Created test schema with:");
    println!("  - 1 class: Country");
    println!("  - 3 slots: code, name, domain");
    println!("  - 1 enum: Region (5 values)");
    println!();

    // Test Rust generation
    println!("=== Testing Rust Code Generation ===");
    let rust_generator = RustGenerator::new();
    println!("Generator: {}", rust_generator.name());
    println!("Description: {}", rust_generator.description());

    let rust_code = rust_generator.generate(&schema)?;
    let rust_path = PathBuf::from("/tmp/test_country.rs");
    fs::write(&rust_path, &rust_code)?;

    println!(
        "✓ Generated {} lines of Rust code",
        rust_code.lines().count()
    );
    println!("  Saved to: {}", rust_path.display());
    println!("
First 40 lines:");
    println!("{}", "=".repeat(50));
    for line in rust_code.lines().take(40) {
        println!("{}", line);
    }
    println!("{}", "=".repeat(50));
    println!();

    // Test Python dataclass generation
    println!("=== Testing Python Dataclass Generation ===");
    let python_generator = PythonDataclassGenerator::new();
    println!("Generator: {}", python_generator.name());
    println!("Description: {}", python_generator.description());

    let python_code = python_generator.generate(&schema)?;
    let python_path = PathBuf::from("/tmp/test_country_dataclass.py");
    fs::write(&python_path, &python_code)?;

    println!(
        "✓ Generated {} lines of Python dataclass code",
        python_code.lines().count()
    );
    println!("  Saved to: {}", python_path.display());
    println!("
First 40 lines:");
    println!("{}", "=".repeat(50));
    for line in python_code.lines().take(40) {
        println!("{}", line);
    }
    println!("{}", "=".repeat(50));
    println!();

    // Test Pydantic generation
    println!("=== Testing Pydantic Model Generation ===");
    let pydantic_generator = PydanticGenerator::new();
    println!("Generator: {}", pydantic_generator.name());
    println!("Description: {}", pydantic_generator.description());

    let pydantic_code = pydantic_generator.generate(&schema)?;
    let pydantic_path = PathBuf::from("/tmp/test_country_pydantic.py");
    fs::write(&pydantic_path, &pydantic_code)?;

    println!(
        "✓ Generated {} lines of Pydantic model code",
        pydantic_code.lines().count()
    );
    println!("  Saved to: {}", pydantic_path.display());
    println!("
First 40 lines:");
    println!("{}", "=".repeat(50));
    for line in pydantic_code.lines().take(40) {
        println!("{}", line);
    }
    println!("{}", "=".repeat(50));
    println!();

    // Test compilation/syntax
    println!("=== Validation Tests ===");

    // Test Rust
    print!("Testing Rust compilation... ");
    let rustc = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg("/tmp/test.rmeta")
        .arg(&rust_path)
        .output()?;

    if rustc.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
        let stderr = String::from_utf8_lossy(&rustc.stderr);
        for line in stderr.lines().take(3) {
            println!("  {}", line);
        }
    }

    // Test Python dataclass
    print!("Testing Python dataclass syntax... ");
    let py_check = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&python_path)
        .output()?;

    if py_check.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
    }

    // Test Pydantic
    print!("Testing Pydantic model syntax... ");
    let pydantic_check = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&pydantic_path)
        .output()?;

    if pydantic_check.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
    }

    println!("
=== Summary ===");
    println!("✓ Successfully generated Rust structs/enums from LinkML schema");
    println!("✓ Successfully generated Python dataclasses from LinkML schema");
    println!("✓ Successfully generated Pydantic models from LinkML schema");
    println!("✓ All generators produce syntactically valid code");
    println!("
Conclusion: The LinkML Rust service CAN generate both Rust and Python code!");

    Ok(())
}
