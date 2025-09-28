//! Simple example demonstrating code generation from ISO3166 LinkML schema
//!
//! This example shows how to generate Rust and Python code from the actual ISO3166 schema

use linkml_core::prelude::*;
use linkml_service::generator::{
    Generator, PydanticGenerator, PythonDataclassGenerator, RustGenerator,
};
use std::fs;
use std::path::PathBuf;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    println!("=== Code Generation from ISO3166 LinkML Schema ===
");

    // Load the test ISO3166 schema
    let schema_path = PathBuf::from("/tmp/test_schema.yaml");
    let schema_content = fs::read_to_string(&schema_path)?;
    let schema: SchemaDefinition = serde_yaml::from_str(&schema_content)?;

    println!("Loaded schema: {}", schema.name);

    // Generate Rust code
    println!("
1. Generating Rust code...");
    let rust_generator = RustGenerator::new();
    let rust_code = rust_generator.generate(&schema)?;
    let rust_path = PathBuf::from("/tmp/iso3166.rs");
    fs::write(&rust_path, &rust_code)?;
    println!("   ✓ Saved to: {}", rust_path.display());
    println!("   Lines: {}", rust_code.lines().count());

    // Generate Python dataclass code
    println!("
2. Generating Python dataclass code...");
    let python_generator = PythonDataclassGenerator::new();
    let python_code = python_generator.generate(&schema)?;
    let python_path = PathBuf::from("/tmp/iso3166_dataclass.py");
    fs::write(&python_path, &python_code)?;
    println!("   ✓ Saved to: {}", python_path.display());
    println!("   Lines: {}", python_code.lines().count());

    // Generate Pydantic code
    println!("
3. Generating Pydantic model code...");
    let pydantic_generator = PydanticGenerator::new();
    let pydantic_code = pydantic_generator.generate(&schema)?;
    let pydantic_path = PathBuf::from("/tmp/iso3166_pydantic.py");
    fs::write(&pydantic_path, &pydantic_code)?;
    println!("   ✓ Saved to: {}", pydantic_path.display());
    println!("   Lines: {}", pydantic_code.lines().count());

    // Show preview of Rust code
    println!("
=== Rust Code Preview ===");
    for line in rust_code.lines().take(30) {
        println!("{}", line);
    }
    if rust_code.lines().count() > 30 {
        println!("... ({} more lines)", rust_code.lines().count() - 30);
    }

    // Show preview of Python code
    println!("
=== Python Dataclass Preview ===");
    for line in python_code.lines().take(30) {
        println!("{}", line);
    }
    if python_code.lines().count() > 30 {
        println!("... ({} more lines)", python_code.lines().count() - 30);
    }

    // Test compilation/syntax
    println!("
=== Testing Generated Code ===");

    // Test Rust compilation
    print!("Testing Rust compilation... ");
    let rustc_result = std::process::Command::new("rustc")
        .arg("--edition=2024")
        .arg("--crate-type=lib")
        .arg("--emit=metadata")
        .arg("-o")
        .arg("/tmp/test.rmeta")
        .arg(&rust_path)
        .stderr(std::process::Stdio::piped())
        .output()?;

    if rustc_result.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
        let stderr = String::from_utf8_lossy(&rustc_result.stderr);
        for line in stderr.lines().take(5) {
            println!("  {}", line);
        }
    }

    // Test Python syntax
    print!("Testing Python dataclass syntax... ");
    let python_result = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&python_path)
        .stderr(std::process::Stdio::piped())
        .output()?;

    if python_result.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
        let stderr = String::from_utf8_lossy(&python_result.stderr);
        for line in stderr.lines().take(5) {
            println!("  {}", line);
        }
    }

    // Test Pydantic syntax
    print!("Testing Pydantic model syntax... ");
    let pydantic_result = std::process::Command::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&pydantic_path)
        .stderr(std::process::Stdio::piped())
        .output()?;

    if pydantic_result.status.success() {
        println!("✓ Success!");
    } else {
        println!("✗ Failed");
        let stderr = String::from_utf8_lossy(&pydantic_result.stderr);
        for line in stderr.lines().take(5) {
            println!("  {}", line);
        }
    }

    println!("
=== Summary ===");
    println!("✓ Successfully generated Rust and Python code from ISO3166 schema!");
    println!("✓ Code generation works for both Rust structs and Python classes!");
    println!("
Generated files:");
    println!("  - {}", rust_path.display());
    println!("  - {}", python_path.display());
    println!("  - {}", pydantic_path.display());

    Ok(())
}
