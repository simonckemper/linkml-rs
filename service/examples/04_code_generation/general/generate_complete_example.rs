//! Generate code from a complete ISO3166 example schema
//!
//! This example generates more complete Rust and Python code
//! showing all the features of the code generators.

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, PydanticGenerator, PythonDataclassGenerator, RustGenerator,
};
use std::fs;
use std::path::PathBuf;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!(
        "=== Complete ISO3166 Code Generation Example ===
"
    );

    // Load the simple schema
    let schema_path = PathBuf::from(
        "/home/kempersc/apps/rootreal/docs/linkml/generated-examples/simple_schema.yaml",
    );
    let schema_content = fs::read_to_string(&schema_path)?;
    let schema: SchemaDefinition = serde_yaml::from_str(&schema_content)?;

    println!("Loaded schema: {}", schema.name);
    println!(
        "  Title: {}",
        schema.title.as_ref().unwrap_or(&"".to_string())
    );
    println!("  Classes: {}", schema.classes.len());
    println!("  Slots: {}", schema.slots.len());
    println!("  Enums: {}", schema.enums.len());
    println!();

    // Generate Rust code
    println!("=== Generating Rust Code ===");
    let rust_generator = RustGenerator::new();
    let rust_code = rust_generator.generate(&schema)?;
    let rust_path = PathBuf::from(
        "/home/kempersc/apps/rootreal/docs/linkml/generated-examples/iso3166_complete.rs",
    );
    fs::write(&rust_path, &rust_code)?;
    println!(
        "✓ Generated {} lines of Rust code",
        rust_code.lines().count()
    );
    println!("  Saved to: {}", rust_path.display());

    // Show Rust preview
    println!(
        "
Rust Code Preview:"
    );
    println!("{}", "=".repeat(60));
    for (i, line) in rust_code.lines().enumerate() {
        if i >= 50 {
            println!("... ({} more lines)", rust_code.lines().count() - 50);
            break;
        }
        println!("{}", line);
    }
    println!("{}", "=".repeat(60));
    println!();

    // Generate Python dataclass code
    println!("=== Generating Python Dataclass Code ===");
    let python_generator = PythonDataclassGenerator::new();
    let python_code = python_generator.generate(&schema)?;
    let python_path = PathBuf::from(
        "/home/kempersc/apps/rootreal/docs/linkml/generated-examples/iso3166_dataclass.py",
    );
    fs::write(&python_path, &python_code)?;
    println!(
        "✓ Generated {} lines of Python dataclass code",
        python_code.lines().count()
    );
    println!("  Saved to: {}", python_path.display());

    // Show Python preview
    println!(
        "
Python Dataclass Preview:"
    );
    println!("{}", "=".repeat(60));
    for (i, line) in python_code.lines().enumerate() {
        if i >= 40 {
            println!("... ({} more lines)", python_code.lines().count() - 40);
            break;
        }
        println!("{}", line);
    }
    println!("{}", "=".repeat(60));
    println!();

    // Generate Pydantic code
    println!("=== Generating Pydantic Model Code ===");
    let pydantic_generator = PydanticGenerator::new();
    let pydantic_code = pydantic_generator.generate(&schema)?;
    let pydantic_path = PathBuf::from(
        "/home/kempersc/apps/rootreal/docs/linkml/generated-examples/iso3166_pydantic.py",
    );
    fs::write(&pydantic_path, &pydantic_code)?;
    println!(
        "✓ Generated {} lines of Pydantic model code",
        pydantic_code.lines().count()
    );
    println!("  Saved to: {}", pydantic_path.display());

    // Show Pydantic preview
    println!(
        "
Pydantic Model Preview:"
    );
    println!("{}", "=".repeat(60));
    for (i, line) in pydantic_code.lines().enumerate() {
        if i >= 50 {
            println!("... ({} more lines)", pydantic_code.lines().count() - 50);
            break;
        }
        println!("{}", line);
    }
    println!("{}", "=".repeat(60));
    println!();

    // Test syntax
    println!("=== Testing Generated Code ===");

    // Test Python dataclass
    print!("Testing Python dataclass syntax... ");
    let py_test = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&python_path)
        .output()?;
    if py_test.status.success() {
        println!("✓ Valid!");
    } else {
        println!("✗ Invalid");
    }

    // Test Pydantic
    print!("Testing Pydantic model syntax... ");
    let pydantic_test = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&pydantic_path)
        .output()?;
    if pydantic_test.status.success() {
        println!("✓ Valid!");
    } else {
        println!("✗ Invalid");
    }

    println!(
        "
=== Summary ==="
    );
    println!("Successfully generated code from complete ISO3166 schema:");
    println!("  - Rust: {}", rust_path.display());
    println!("  - Python Dataclass: {}", python_path.display());
    println!("  - Pydantic Model: {}", pydantic_path.display());
    println!(
        "
All generated files are saved for your study!"
    );

    Ok(())
}
